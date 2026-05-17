//! Procedural macros for the `jigs` framework.
//!
//! `#[jig]` marks a function as a pipeline step. It emits a zero-sized
//! marker struct implementing `JigDef` alongside the (possibly
//! transformed) function body. The marker struct is named
//! `__Jig_<fn_name>` to avoid namespace collisions with the function
//! itself. With the `trace` feature it additionally wraps the body in a
//! thread-local trace recorder.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{
    parse_macro_input, parse_quote, Data, DeriveInput, Expr, ExprMethodCall, Field, Fields,
    FieldsNamed, FieldsUnnamed, Ident, ItemFn, ReturnType, Type,
};

fn marker_ident(fn_name: &str) -> syn::Ident {
    syn::parse_str(&format!("__Jig_{fn_name}")).unwrap()
}

fn marker_path_for(name: &str) -> TokenStream2 {
    let segs: Vec<&str> = name.split("::").collect();
    let last_idx = segs.len() - 1;
    let path_segs: Vec<TokenStream2> = segs
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if i == last_idx {
                let mi = marker_ident(s);
                quote!(#mi)
            } else if *s == "crate" {
                quote!(crate)
            } else if *s == "super" {
                quote!(super)
            } else if *s == "self" {
                quote!(self)
            } else {
                let id: syn::Ident = syn::parse_str(s).unwrap();
                quote!(#id)
            }
        })
        .collect();
    quote!(#(#path_segs)::*)
}

#[proc_macro_attribute]
pub fn jig(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let vis = &input.vis;
    let attrs = &input.attrs;
    let block = &input.block;
    let name_str = input.sig.ident.to_string();
    let marker = marker_ident(&name_str);
    let kind_str = return_kind(&input.sig.output);
    let input_str = input_kind(&input.sig);
    let input_type_str = first_arg_payload(&input.sig);
    let output_type_str = return_payload(&input.sig.output);
    let is_async = input.sig.asyncness.is_some();

    let chain = collect_chain(&input.block);

    let chain_tokens: Vec<TokenStream2> = chain
        .iter()
        .map(|(name, kind)| {
            let kind_ident = match kind {
                ChainKindTok::Then => quote!(::jigs::ChainKind::Then),
                ChainKindTok::Fork => quote!(::jigs::ChainKind::Fork),
            };
            quote! { ::jigs::ChainStep { name: #name, kind: #kind_ident } }
        })
        .collect();

    let chain_collect: Vec<TokenStream2> = chain
        .iter()
        .map(|(name, _kind)| {
            let path = marker_path_for(name);
            quote! { <#path as ::jigs::JigDef>::collect(out); }
        })
        .collect();

    let marker_def = quote! {
        #[allow(non_camel_case_types)]
        #[doc(hidden)]
        pub struct #marker;

        impl ::jigs::JigDef for #marker {
            const META: ::jigs::JigMeta = ::jigs::JigMeta {
                name: #name_str,
                file: file!(),
                line: line!(),
                kind: #kind_str,
                input: #input_str,
                input_type: #input_type_str,
                output_type: #output_type_str,
                is_async: #is_async,
                module: module_path!(),
                chain: &[#(#chain_tokens),*],
            };

            fn collect(out: &mut Vec<&'static ::jigs::JigMeta>) {
                let meta = &<Self as ::jigs::JigDef>::META;
                if out.iter().any(|m| ::std::ptr::eq(*m, meta)) {
                    return;
                }
                out.push(meta);
                #(#chain_collect)*
            }
        }
    };

    let input_ident = first_arg_ident(&input.sig);

    if input.sig.asyncness.is_some() {
        let mut sig = input.sig.clone();
        sig.asyncness = None;
        let ret_ty = match &input.sig.output {
            ReturnType::Default => quote!(()),
            ReturnType::Type(_, ty) => quote!(#ty),
        };
        sig.output = parse_quote! {
            -> ::jigs::Pending<impl ::core::future::Future<Output = #ret_ty>>
        };

        let body = async_body(block, &name_str, input_ident.as_ref());
        return quote! { #marker_def #(#attrs)* #vis #sig { #body } }.into();
    }

    let sig = &input.sig;
    let body = sync_body(block, &name_str, input_ident.as_ref());
    quote! { #marker_def #(#attrs)* #vis #sig { #body } }.into()
}

#[proc_macro_derive(Request, attributes(req))]
pub fn derive_request(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as DeriveInput);
    generate_req(&parsed).unwrap_or_else(|e| e.to_compile_error().into())
}

fn generate_req(input: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "Request can only be derived for structs",
        ));
    };

    let mut explicit_field: Option<Ident> = None;

    for attr in &input.attrs {
        if attr.path().is_ident("req") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("field") {
                    let val = meta.value()?;
                    let lit: syn::LitStr = val.parse()?;
                    explicit_field = Some(syn::Ident::new(&lit.value(), lit.span()));
                    return Ok(());
                }
                Err(meta.error("unrecognized req attribute"))
            })?;
        }
    }

    let (payload_decl, payload_ref_expr, into_expr, from_expr) =
        derive_req_field_info(data, explicit_field, input)?;

    let mut merge_generics = input.generics.clone();
    merge_generics
        .params
        .push(syn::GenericParam::Type(syn::TypeParam {
            attrs: Vec::new(),
            ident: parse_quote!(__R),
            colon_token: Some(syn::Token![:](input.generics.span())),
            bounds: parse_quote!(::jigs::Response),
            eq_token: None,
            default: None,
        }));
    let (merge_impl_generics, _, merge_where_clause) = merge_generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::jigs::Request for #name #type_generics #where_clause {
            #payload_decl
            fn payload(&self) -> &Self::Payload {
                #payload_ref_expr
            }
            fn into_payload(self) -> Self::Payload {
                #into_expr
            }
            fn from_payload(payload: Self::Payload) -> Self {
                #from_expr
            }
        }
        impl #merge_impl_generics ::jigs::Merge<__R> for #name #type_generics #merge_where_clause {
            type Merged = ::jigs::Branch<#name #type_generics, __R>;
            fn into_continue(self) -> Self::Merged {
                ::jigs::Branch::Continue(self)
            }
            fn from_done(resp: __R) -> Self::Merged {
                ::jigs::Branch::Done(resp)
            }
        }
        impl #impl_generics ::jigs::Step for #name #type_generics #where_clause {
            type Out = #name #type_generics;
            type Fut = ::core::future::Ready<#name #type_generics>;
            fn into_step(self) -> Self::Fut {
                ::core::future::ready(self)
            }
        }
        impl #impl_generics ::jigs::Status for #name #type_generics #where_clause {
            fn succeeded(&self) -> bool {
                true
            }
            fn error(&self) -> Option<String> {
                None
            }
        }
    }
    .into())
}

fn derive_req_field_info(
    data: &syn::DataStruct,
    explicit_field: Option<Ident>,
    input: &DeriveInput,
) -> Result<(TokenStream2, TokenStream2, TokenStream2, TokenStream2), syn::Error> {
    if let Some(field_ident) = explicit_field {
        let field = find_field(data, &field_ident)?;
        let payload_ty = &field.ty;
        let payload_decl = quote! { type Payload = #payload_ty; };
        let payload_ref = quote! { &self.#field_ident };
        let into_expr = quote! {
            let Self { #field_ident, .. } = self;
            #field_ident
        };
        let from_expr = quote! { Self { #field_ident: payload, ..Default::default() } };
        return Ok((payload_decl, payload_ref, into_expr, from_expr));
    }

    match &data.fields {
        Fields::Unnamed(FieldsUnnamed { unnamed, .. }) if unnamed.len() == 1 => {
            let field = unnamed.first().unwrap();
            let payload_ty = &field.ty;
            let payload_decl = quote! { type Payload = #payload_ty; };
            let payload_ref = quote! { &self.0 };
            let into_expr = quote! { self.0 };
            let from_expr = quote! { Self(payload) };
            Ok((payload_decl, payload_ref, into_expr, from_expr))
        }
        Fields::Named(FieldsNamed { named, .. }) if named.len() == 1 => {
            let field = named.first().unwrap();
            let field_ident = field.ident.as_ref().unwrap();
            let payload_ty = &field.ty;
            let payload_decl = quote! { type Payload = #payload_ty; };
            let payload_ref = quote! { &self.#field_ident };
            let into_expr = quote! { self.#field_ident };
            let from_expr = quote! { Self { #field_ident: payload } };
            Ok((payload_decl, payload_ref, into_expr, from_expr))
        }
        _ => Err(syn::Error::new_spanned(
            input,
            "Request derive requires either: one field, or #[req(field = \"name\")]",
        )),
    }
}

fn find_field<'a>(data: &'a syn::DataStruct, ident: &Ident) -> Result<&'a Field, syn::Error> {
    for f in &data.fields {
        if f.ident.as_ref() == Some(ident) {
            return Ok(f);
        }
    }
    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        format!("no field named `{ident}`"),
    ))
}

#[proc_macro_derive(Response, attributes(resp))]
pub fn derive_response(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as DeriveInput);
    generate_response(&parsed).unwrap_or_else(|e| e.to_compile_error().into())
}

fn generate_response(input: &DeriveInput) -> Result<TokenStream, syn::Error> {
    match &input.data {
        Data::Struct(data) => generate_response_struct(input, data),
        Data::Enum(data) => generate_response_enum(input, data),
        Data::Union(_u) => Err(syn::Error::new_spanned(
            input,
            "Response cannot be derived for unions",
        )),
    }
}

fn generate_response_struct(
    input: &DeriveInput,
    data: &syn::DataStruct,
) -> Result<TokenStream, syn::Error> {
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    match &data.fields {
        Fields::Unnamed(FieldsUnnamed { unnamed, .. }) if unnamed.len() == 1 => {
            let f = unnamed.first().unwrap();
            let ok_expr = quote! { Self(Ok(payload)) };
            let err_expr = quote! { Self(Err(msg.into())) };
            let is_ok_expr = quote! { self.0.is_ok() };
            let into_result_expr = quote! { self.0 };
            let error_msg_expr = quote! { self.0.as_ref().err().cloned() };
            let payload_ty = extract_result_payload(&f.ty,
                "Response derive on single-field structs expects `Result<Payload, String>`",
            )?;
            Ok(generate_response_impls(ResponseImplParts {
                name,
                impl_generics,
                type_generics,
                where_clause,
                payload_ty: &payload_ty,
                ok_expr,
                err_expr,
                is_ok_expr,
                into_result_expr,
                error_msg_expr,
            }))
        }
        Fields::Named(FieldsNamed { named, .. }) if named.len() == 1 => {
            let f = named.first().unwrap();
            let field_ident = f.ident.as_ref().unwrap();
            let payload_ty = extract_result_payload(
                &f.ty,
                "Response derive on single-field structs expects `Result<Payload, String>`",
            )?;
            let ok_expr = quote! { Self { #field_ident: Ok(payload) } };
            let err_expr = quote! { Self { #field_ident: Err(msg.into()) } };
            let is_ok_expr = quote! { self.#field_ident.is_ok() };
            let into_result_expr = quote! { self.#field_ident };
            let error_msg_expr = quote! { self.#field_ident.as_ref().err().cloned() };
            Ok(generate_response_impls(ResponseImplParts {
                name,
                impl_generics,
                type_generics,
                where_clause,
                payload_ty: &payload_ty,
                ok_expr,
                err_expr,
                is_ok_expr,
                into_result_expr,
                error_msg_expr,
            }))
        }
        Fields::Named(FieldsNamed { named, .. }) if named.len() == 2 => {
            generate_response_two_fields(input, data, named, name, impl_generics, type_generics, where_clause)
        }
        _ => Err(syn::Error::new_spanned(
            input,
            "Response derive requires either: a single `Result<Payload, String>` field, or two fields",
        )),
    }
}

fn generate_response_two_fields(
    input: &DeriveInput,
    _data: &syn::DataStruct,
    named: &syn::punctuated::Punctuated<Field, syn::token::Comma>,
    name: &Ident,
    impl_generics: syn::ImplGenerics,
    type_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> Result<TokenStream, syn::Error> {
    let mut ok_field_idx: Option<usize> = None;
    let mut err_field_idx: Option<usize> = None;

    for (i, f) in named.iter().enumerate() {
        for attr in &f.attrs {
            if attr.path().is_ident("resp") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("ok") {
                        ok_field_idx = Some(i);
                        return Ok(());
                    }
                    if meta.path.is_ident("err") {
                        err_field_idx = Some(i);
                        return Ok(());
                    }
                    Err(meta.error("unrecognized resp attribute"))
                })?;
            }
        }
    }

    let ok_idx = match ok_field_idx {
        Some(i) => i,
        None => err_field_idx.map_or(0, |e| 1 - e),
    };
    let err_idx = match err_field_idx {
        Some(i) => i,
        None => ok_field_idx.map_or(1, |o| 1 - o),
    };

    if ok_idx == err_idx {
        return Err(syn::Error::new_spanned(
            input,
            "ok and err fields cannot be the same",
        ));
    }

    let ok_field = &named[ok_idx];
    let err_field = &named[err_idx];

    let ok_ident = ok_field.ident.as_ref().unwrap();
    let err_ident = err_field.ident.as_ref().unwrap();

    let is_err_string = matches!(
        syn_type_as_string(&err_field.ty).as_deref(),
        Some(s) if s == "String",
    );

    if !is_err_string {
        return Err(syn::Error::new_spanned(
            input,
            "Response derive with two fields requires the error field to be `String`",
        ));
    }

    let payload_ty = extract_option_inner(
        &ok_field.ty,
        "Response derive with two fields expects the ok field to be `Option<Payload>`",
    )?;
    let ok_expr = quote! { Self { #ok_ident: Some(payload), #err_ident: "".to_string() } };
    let err_expr = quote! { Self { #ok_ident: None, #err_ident: msg.into() } };
    let is_ok_expr = quote! { self.#ok_ident.is_some() };
    let into_result_expr = quote! {
        match self.#ok_ident {
            Some(v) => Ok(v),
            None => Err(self.#err_ident),
        }
    };
    let error_msg_expr = quote! {
        if self.#ok_ident.is_some() { None } else { Some(self.#err_ident.clone()) }
    };

    Ok(generate_response_impls(ResponseImplParts {
        name,
        impl_generics,
        type_generics,
        where_clause,
        payload_ty: &payload_ty,
        ok_expr,
        err_expr,
        is_ok_expr,
        into_result_expr,
        error_msg_expr,
    }))
}

struct ClassifiedVariant<'a> {
    variant: &'a syn::Variant,
    ident: syn::Ident,
    fields: &'a syn::Fields,
}

fn classify_enum_variants<'a>(
    data: &'a syn::DataEnum,
    input: &'a DeriveInput,
) -> Result<(ClassifiedVariant<'a>, ClassifiedVariant<'a>), syn::Error> {
    if data.variants.len() != 2 {
        return Err(syn::Error::new_spanned(
            input,
            "Response derive on enums requires exactly 2 variants",
        ));
    }

    let mut ok_variant: Option<ClassifiedVariant<'_>> = None;
    let mut err_variant: Option<ClassifiedVariant<'_>> = None;

    for v in &data.variants {
        let mut is_ok = false;
        let mut is_err = false;
        for attr in &v.attrs {
            if attr.path().is_ident("resp") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("ok") {
                        is_ok = true;
                        return Ok(());
                    }
                    if meta.path.is_ident("err") {
                        is_err = true;
                        return Ok(());
                    }
                    Err(meta.error("unrecognized resp attribute"))
                })?;
            }
        }

        if is_ok && is_err {
            return Err(syn::Error::new_spanned(
                v,
                "variant cannot be both #[resp(ok)] and #[resp(err)]",
            ));
        }

        let cv = ClassifiedVariant {
            variant: v,
            ident: v.ident.clone(),
            fields: &v.fields,
        };

        if is_ok {
            if ok_variant.is_some() {
                return Err(syn::Error::new_spanned(
                    v,
                    "only one variant can be #[resp(ok)]",
                ));
            }
            if v.fields.len() != 1 {
                return Err(syn::Error::new_spanned(
                    v,
                    "ok variant must have exactly one field (the payload)",
                ));
            }
            ok_variant = Some(cv);
        } else if is_err {
            if err_variant.is_some() {
                return Err(syn::Error::new_spanned(
                    v,
                    "only one variant can be #[resp(err)]",
                ));
            }
            if v.fields.len() > 1 {
                return Err(syn::Error::new_spanned(
                    v,
                    "err variant must have 0 or 1 fields",
                ));
            }
            err_variant = Some(cv);
        } else if ok_variant.is_none() {
            if v.fields.len() != 1 {
                return Err(syn::Error::new_spanned(
                    v,
                    "ok variant must have exactly one field (the payload)",
                ));
            }
            ok_variant = Some(cv);
        } else if err_variant.is_none() {
            if v.fields.len() > 1 {
                return Err(syn::Error::new_spanned(
                    v,
                    "err variant must have 0 or 1 fields",
                ));
            }
            err_variant = Some(cv);
        }
    }

    let ok = ok_variant.ok_or_else(|| {
        syn::Error::new_spanned(input, "Could not identify ok variant. Use #[resp(ok)]")
    })?;
    let err = err_variant.ok_or_else(|| {
        syn::Error::new_spanned(input, "Could not identify err variant. Use #[resp(err)]")
    })?;
    Ok((ok, err))
}

struct VariantCodegen {
    constructor: TokenStream2,
    wild: TokenStream2,
    pattern: TokenStream2,
}

fn variant_codegen(
    name: &syn::Ident,
    ident: &syn::Ident,
    fields: &syn::Fields,
    binding_name: &str,
) -> VariantCodegen {
    let b = syn::Ident::new(binding_name, name.span());
    if fields.is_empty() {
        let constructor = quote!(#name::#ident);
        let wild = quote!(#name::#ident);
        let pattern = quote!(#name::#ident);
        VariantCodegen {
            constructor,
            wild,
            pattern,
        }
    } else {
        let unnamed = fields.iter().next().unwrap().ident.is_none();
        let constructor = if unnamed {
            quote!(#name::#ident(#b))
        } else {
            let f = fields.iter().next().unwrap().ident.as_ref().unwrap();
            quote!(#name::#ident { #f: #b })
        };
        let wild = if unnamed {
            quote! { #name::#ident(..) }
        } else {
            quote! { #name::#ident { .. } }
        };
        let pattern = if unnamed {
            let b = syn::Ident::new(binding_name, name.span());
            quote! { #name::#ident(#b) }
        } else {
            let f = fields.iter().next().unwrap().ident.as_ref().unwrap();
            let b = syn::Ident::new(binding_name, name.span());
            quote! { #name::#ident { #f: #b } }
        };
        VariantCodegen {
            constructor,
            wild,
            pattern,
        }
    }
}

fn generate_response_enum(
    input: &DeriveInput,
    data: &syn::DataEnum,
) -> Result<TokenStream, syn::Error> {
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let (ok, err) = classify_enum_variants(data, input)?;

    let ok_ident = &ok.ident;
    let err_ident = &err.ident;
    let payload_ty = &ok.variant.fields.iter().next().unwrap().ty;

    let ok_cg = variant_codegen(name, ok_ident, ok.fields, "__p");
    let err_has_field = err.fields.len() == 1;
    let err_cg = variant_codegen(name, err_ident, err.fields, "__e");
    let VariantCodegen {
        constructor: ok_constr,
        wild: ok_wild,
        pattern: ok_pattern,
    } = ok_cg;
    let VariantCodegen {
        constructor: err_constr,
        wild: err_wild,
        pattern: err_pattern,
    } = err_cg;

    let ok_expr = quote! {
        {
            let __p = payload;
            #ok_constr
        }
    };
    let err_expr = if err_has_field {
        quote! {
            {
                let __e = msg.into();
                #err_constr
            }
        }
    } else {
        quote! { #name::#err_ident }
    };

    let is_ok_expr = quote! {
        match self {
            #ok_wild => true,
            #err_wild => false,
        }
    };
    let into_result_expr = if err_has_field {
        quote! {
            match self {
                #ok_pattern => Ok(__p),
                #err_pattern => Err(__e),
            }
        }
    } else {
        quote! {
            match self {
                #ok_pattern => Ok(__p),
                #err_wild => Err("unknown error".to_string()),
            }
        }
    };
    let error_msg_expr = if err_has_field {
        quote! {
            match self {
                #ok_wild => None,
                #err_pattern => Some(__e.to_string()),
            }
        }
    } else {
        quote! {
            match self {
                #ok_wild => None,
                #err_wild => Some("unknown error".to_string()),
            }
        }
    };

    Ok(generate_response_impls(ResponseImplParts {
        name,
        impl_generics,
        type_generics,
        where_clause,
        payload_ty,
        ok_expr,
        err_expr,
        is_ok_expr,
        into_result_expr,
        error_msg_expr,
    }))
}

struct ResponseImplParts<'a> {
    name: &'a syn::Ident,
    impl_generics: syn::ImplGenerics<'a>,
    type_generics: syn::TypeGenerics<'a>,
    where_clause: Option<&'a syn::WhereClause>,
    payload_ty: &'a Type,
    ok_expr: TokenStream2,
    err_expr: TokenStream2,
    is_ok_expr: TokenStream2,
    into_result_expr: TokenStream2,
    error_msg_expr: TokenStream2,
}

fn generate_response_impls(parts: ResponseImplParts<'_>) -> proc_macro::TokenStream {
    let ResponseImplParts {
        name,
        impl_generics,
        type_generics,
        where_clause,
        payload_ty,
        ok_expr,
        err_expr,
        is_ok_expr,
        into_result_expr,
        error_msg_expr,
    } = parts;
    quote! {
        impl #impl_generics ::jigs::Response for #name #type_generics #where_clause {
            type Payload = #payload_ty;
            fn ok(payload: Self::Payload) -> Self {
                #ok_expr
            }
            fn err(msg: impl Into<String>) -> Self {
                #err_expr
            }
            fn is_ok(&self) -> bool {
                #is_ok_expr
            }
            fn into_result(self) -> Result<Self::Payload, String> {
                #into_result_expr
            }
            fn error_msg(&self) -> Option<String> {
                #error_msg_expr
            }
        }
        impl #impl_generics ::jigs::Merge<#name #type_generics> for #name #type_generics #where_clause {
            type Merged = #name #type_generics;
            fn into_continue(self) -> Self::Merged {
                self
            }
            fn from_done(resp: #name #type_generics) -> Self::Merged {
                resp
            }
        }
        impl #impl_generics ::jigs::Step for #name #type_generics #where_clause {
            type Out = #name #type_generics;
            type Fut = ::core::future::Ready<#name #type_generics>;
            fn into_step(self) -> Self::Fut {
                ::core::future::ready(self)
            }
        }
        impl #impl_generics ::jigs::Status for #name #type_generics #where_clause {
            fn succeeded(&self) -> bool {
                ::jigs::Response::is_ok(self)
            }
            fn error(&self) -> Option<String> {
                ::jigs::Response::error_msg(self)
            }
        }
    }
    .into()
}

fn extract_result_payload(ty: &Type, msg: &str) -> Result<Type, syn::Error> {
    if let Type::Path(p) = ty {
        if let Some(seg) = p.path.segments.last() {
            if seg.ident == "Result" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if args.args.len() == 2 {
                        if let syn::GenericArgument::Type(t) = &args.args[0] {
                            if let syn::GenericArgument::Type(t2) = &args.args[1] {
                                let s = type_to_string(t2);
                                if s == "String" {
                                    return Ok(t.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Err(syn::Error::new_spanned(ty, msg))
}

fn extract_option_inner(ty: &Type, msg: &str) -> Result<Type, syn::Error> {
    if let Type::Path(p) = ty {
        if let Some(seg) = p.path.segments.last() {
            if seg.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(t)) = args.args.first() {
                        return Ok(t.clone());
                    }
                }
            }
        }
    }
    Err(syn::Error::new_spanned(ty, msg))
}

fn syn_type_as_string(ty: &Type) -> Option<String> {
    if let Type::Path(p) = ty {
        Some(
            p.path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
        )
    } else {
        None
    }
}

#[proc_macro]
pub fn jigs(input: TokenStream) -> TokenStream {
    let entry: syn::Ident = parse_macro_input!(input);
    let entry_marker = marker_ident(&entry.to_string());
    quote! {
        mod __jigs_registry {
            pub fn all_jigs() -> impl Iterator<Item = &'static ::jigs::JigMeta> {
                static CACHE: std::sync::OnceLock<Vec<&'static ::jigs::JigMeta>> = std::sync::OnceLock::new();
                CACHE.get_or_init(|| {
                    let mut v = Vec::new();
                    <super::#entry_marker as ::jigs::JigDef>::collect(&mut v);
                    v
                }).iter().copied()
            }

            pub fn find_jig(name: &str) -> Option<&'static ::jigs::JigMeta> {
                all_jigs().find(|m| m.name == name)
            }
        }
        pub use __jigs_registry::{all_jigs, find_jig};
    }
    .into()
}

fn first_arg_ident(sig: &syn::Signature) -> Option<syn::Ident> {
    if let Some(syn::FnArg::Typed(pt)) = sig.inputs.first() {
        if let syn::Pat::Ident(pi) = &*pt.pat {
            return Some(pi.ident.clone());
        }
    }
    None
}

#[cfg(feature = "trace")]
struct TraceParts {
    pre: TokenStream2,
    post: TokenStream2,
}

#[cfg(feature = "trace")]
fn trace_instrument(name_str: &str, input_ident: Option<&syn::Ident>) -> TraceParts {
    let marker = marker_ident(name_str);
    let snapshot = if let Some(id) = input_ident {
        quote! { let __jig_input_ok = ::jigs::Status::succeeded(&#id); }
    } else {
        quote! { let __jig_input_ok = true; }
    };
    let pre = quote! {
        #snapshot
        let __jig_idx = ::jigs::trace::enter(&<#marker as ::jigs::JigDef>::META);
        let __jig_start = ::std::time::Instant::now();
    };
    let post = quote! {
        let mut __jig_ok = ::jigs::Status::succeeded(&__jig_result);
        let mut __jig_err = ::jigs::Status::error(&__jig_result);
        if !__jig_input_ok && !__jig_ok {
            __jig_ok = true;
            __jig_err = None;
        }
        ::jigs::trace::exit(__jig_idx, __jig_start.elapsed(), __jig_ok, __jig_err);
        __jig_result
    };
    TraceParts { pre, post }
}

#[cfg(feature = "trace")]
fn sync_body(block: &syn::Block, name_str: &str, input_ident: Option<&syn::Ident>) -> TokenStream2 {
    let TraceParts { pre, post } = trace_instrument(name_str, input_ident);
    quote! {
        #pre
        let __jig_result = (move || #block)();
        #post
    }
}

#[cfg(not(feature = "trace"))]
fn sync_body(
    _block: &syn::Block,
    _name_str: &str,
    _input_ident: Option<&syn::Ident>,
) -> TokenStream2 {
    quote! { #block }
}

#[cfg(feature = "trace")]
fn async_body(
    block: &syn::Block,
    name_str: &str,
    input_ident: Option<&syn::Ident>,
) -> TokenStream2 {
    let TraceParts { pre, post } = trace_instrument(name_str, input_ident);
    quote! {
        ::jigs::Pending(async move {
            #pre
            let __jig_result = (async move #block).await;
            #post
        })
    }
}

#[cfg(not(feature = "trace"))]
fn async_body(
    _block: &syn::Block,
    _name_str: &str,
    _input_ident: Option<&syn::Ident>,
) -> TokenStream2 {
    quote! { ::jigs::Pending(async move #block) }
}

fn return_kind(ret: &ReturnType) -> &'static str {
    let ty = match ret {
        ReturnType::Default => return "Other",
        ReturnType::Type(_, t) => t,
    };
    match last_type_ident(ty).as_deref() {
        Some("Request") => "Request",
        Some("Response") => "Response",
        Some("Branch") => "Branch",
        Some("Pending") => "Pending",
        _ => "Other",
    }
}

fn input_kind(sig: &syn::Signature) -> &'static str {
    let ty = match sig.inputs.first() {
        Some(syn::FnArg::Typed(pt)) => &*pt.ty,
        _ => return "Other",
    };
    match last_type_ident(ty).as_deref() {
        Some("Request") => "Request",
        Some("Response") => "Response",
        _ => "Other",
    }
}

fn first_arg_payload(sig: &syn::Signature) -> String {
    let ty = match sig.inputs.first() {
        Some(syn::FnArg::Typed(pt)) => &*pt.ty,
        _ => return "?".into(),
    };
    payload_type(ty)
}

fn return_payload(ret: &ReturnType) -> String {
    let ty = match ret {
        ReturnType::Default => return "?".into(),
        ReturnType::Type(_, t) => t,
    };
    payload_type(ty)
}

fn payload_type(ty: &Type) -> String {
    if let Type::Path(p) = ty {
        if let Some(seg) = p.path.segments.last() {
            let name = seg.ident.to_string();
            match name.as_str() {
                "Request" | "Response" | "Pending" => {
                    if let syn::PathArguments::AngleBracketed(ref ab) = seg.arguments {
                        return generic_args_string(ab);
                    }
                }
                "Branch" => {
                    if let syn::PathArguments::AngleBracketed(ref ab) = seg.arguments {
                        return format!("Branch<{}>", generic_args_string(ab));
                    }
                }
                _ => {}
            }
        }
    }
    type_to_string(ty)
}

fn type_to_string(ty: &Type) -> String {
    quote::quote!(#ty).to_string().replace(' ', "")
}

fn generic_args_string(args: &syn::AngleBracketedGenericArguments) -> String {
    let mut out = String::new();
    for (i, arg) in args.args.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        match arg {
            syn::GenericArgument::Type(t) => out.push_str(&type_to_string(t)),
            syn::GenericArgument::Lifetime(l) => out.push_str(&l.ident.to_string()),
            other => out.push_str(&quote::quote!(#other).to_string().replace(' ', "")),
        }
    }
    out
}

fn last_type_ident(ty: &Type) -> Option<String> {
    if let Type::Path(p) = ty {
        return Some(p.path.segments.last()?.ident.to_string());
    }
    None
}

#[derive(Clone, Copy)]
enum ChainKindTok {
    Then,
    Fork,
}

fn collect_chain(block: &syn::Block) -> Vec<(String, ChainKindTok)> {
    struct V(Vec<(String, ChainKindTok)>);
    impl V {
        fn push_unique(&mut self, name: String, kind: ChainKindTok) {
            if !self.0.iter().any(|(n, _)| n == &name) {
                self.0.push((name, kind));
            }
        }
        fn push_path(&mut self, p: &syn::Path, kind: ChainKindTok) {
            let name = p
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            self.push_unique(name, kind);
        }
    }
    impl<'ast> Visit<'ast> for V {
        fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
            syn::visit::visit_expr(self, &m.receiver);
            if m.method == "then" {
                if let Some(Expr::Path(p)) = m.args.first() {
                    self.push_path(&p.path, ChainKindTok::Then);
                }
            }
            for a in &m.args {
                syn::visit::visit_expr(self, a);
            }
        }
        fn visit_macro(&mut self, mac: &'ast syn::Macro) {
            let last = mac
                .path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default();
            if last == "fork" {
                if let Ok(args) = syn::parse2::<ForkArgs>(mac.tokens.clone()) {
                    for j in &args.arms {
                        if let syn::Expr::Path(p) = j {
                            self.push_path(&p.path, ChainKindTok::Fork);
                        }
                    }
                    if let syn::Expr::Path(p) = &args.default {
                        self.push_path(&p.path, ChainKindTok::Fork);
                    }
                }
            }
        }
    }
    let mut v = V(Vec::new());
    v.visit_block(block);
    v.0
}

struct ForkArgs {
    arms: Vec<syn::Expr>,
    default: syn::Expr,
}

impl syn::parse::Parse for ForkArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _req: syn::Expr = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let mut arms = Vec::new();
        loop {
            if input.peek(syn::Token![_]) {
                input.parse::<syn::Token![_]>()?;
                input.parse::<syn::Token![=>]>()?;
                let default: syn::Expr = input.parse()?;
                let _: Option<syn::Token![,]> = input.parse().ok();
                return Ok(ForkArgs { arms, default });
            }
            let _pred: syn::Expr = input.parse()?;
            input.parse::<syn::Token![=>]>()?;
            let jig: syn::Expr = input.parse()?;
            input.parse::<syn::Token![,]>()?;
            arms.push(jig);
        }
    }
}
