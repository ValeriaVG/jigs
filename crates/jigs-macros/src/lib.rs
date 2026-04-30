//! Procedural macros for the `jigs` framework.
//!
//! `#[jig]` marks a function as a pipeline step. It always registers a
//! `JigMeta` entry in the global inventory (name, source location, return
//! kind, and the names of jigs called via `.then(...)` inside the body) so
//! the map generator and other tools can introspect the pipeline at runtime
//! without re-parsing source. With the `trace` feature it additionally wraps
//! the body in a thread-local trace recorder.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::visit::Visit;
use syn::{parse_macro_input, parse_quote, Expr, ExprMethodCall, ItemFn, ReturnType, Type};

#[proc_macro_attribute]
pub fn jig(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let vis = &input.vis;
    let block = &input.block;
    let name_str = input.sig.ident.to_string();
    let kind_str = return_kind(&input.sig.output);
    let input_str = input_kind(&input.sig);
    let input_type_str = first_arg_payload(&input.sig);
    let output_type_str = return_payload(&input.sig.output);
    let is_async = input.sig.asyncness.is_some();
    let chain_steps: Vec<TokenStream2> = collect_chain(&input.block)
        .into_iter()
        .map(|(name, kind)| {
            let kind_ident = match kind {
                ChainKindTok::Then => quote!(::jigs::ChainKind::Then),
                ChainKindTok::Fork => quote!(::jigs::ChainKind::Fork),
            };
            quote! { ::jigs::ChainStep { name: #name, kind: #kind_ident } }
        })
        .collect();
    let meta = quote! {
        ::jigs::inventory::submit! {
            ::jigs::JigMeta {
                name: #name_str,
                file: file!(),
                line: line!(),
                kind: #kind_str,
                input: #input_str,
                input_type: #input_type_str,
                output_type: #output_type_str,
                is_async: #is_async,
                module: module_path!(),
                chain: &[#(#chain_steps),*],
            }
        }
    };

    let response_input_ident = if input_str == "Response" {
        first_arg_ident(&input.sig)
    } else {
        None
    };

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

        let body = async_body(block, &name_str, response_input_ident.as_ref());
        return quote! { #meta #vis #sig { #body } }.into();
    }

    let sig = &input.sig;
    let body = sync_body(block, &name_str, response_input_ident.as_ref());
    quote! { #meta #vis #sig { #body } }.into()
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
fn sync_body(
    block: &syn::Block,
    name_str: &str,
    response_input: Option<&syn::Ident>,
) -> TokenStream2 {
    let snapshot = match response_input {
        Some(id) => quote! { let __jig_input_ok = ::jigs::Status::ok(&#id); },
        None => quote! { let __jig_input_ok = true; },
    };
    quote! {
        #snapshot
        let __jig_idx = ::jigs::trace::enter(#name_str);
        let __jig_start = ::std::time::Instant::now();
        let __jig_result = (move || #block)();
        let mut __jig_ok = ::jigs::Status::ok(&__jig_result);
        let mut __jig_err = ::jigs::Status::error(&__jig_result);
        if !__jig_input_ok && !__jig_ok {
            __jig_ok = true;
            __jig_err = None;
        }
        ::jigs::trace::exit(__jig_idx, __jig_start.elapsed(), __jig_ok, __jig_err);
        __jig_result
    }
}

#[cfg(not(feature = "trace"))]
fn sync_body(
    block: &syn::Block,
    _name_str: &str,
    _response_input: Option<&syn::Ident>,
) -> TokenStream2 {
    quote! { #block }
}

#[cfg(feature = "trace")]
fn async_body(
    block: &syn::Block,
    name_str: &str,
    response_input: Option<&syn::Ident>,
) -> TokenStream2 {
    let snapshot = match response_input {
        Some(id) => quote! { let __jig_input_ok = ::jigs::Status::ok(&#id); },
        None => quote! { let __jig_input_ok = true; },
    };
    quote! {
        ::jigs::Pending(async move {
            #snapshot
            let __jig_idx = ::jigs::trace::enter(#name_str);
            let __jig_start = ::std::time::Instant::now();
            let __jig_result = (async move #block).await;
            let mut __jig_ok = ::jigs::Status::ok(&__jig_result);
            let mut __jig_err = ::jigs::Status::error(&__jig_result);
            if !__jig_input_ok && !__jig_ok {
                __jig_ok = true;
                __jig_err = None;
            }
            ::jigs::trace::exit(__jig_idx, __jig_start.elapsed(), __jig_ok, __jig_err);
            __jig_result
        })
    }
}

#[cfg(not(feature = "trace"))]
fn async_body(
    block: &syn::Block,
    _name_str: &str,
    _response_input: Option<&syn::Ident>,
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
                "Request" | "Response" => {
                    if let syn::PathArguments::AngleBracketed(ref ab) = seg.arguments {
                        return generic_args_string(ab);
                    }
                }
                "Branch" => {
                    if let syn::PathArguments::AngleBracketed(ref ab) = seg.arguments {
                        return format!("Branch<{}>", generic_args_string(ab));
                    }
                }
                "Pending" => {
                    if let syn::PathArguments::AngleBracketed(ref ab) = seg.arguments {
                        return generic_args_string(ab);
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
