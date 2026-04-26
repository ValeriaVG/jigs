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
    let is_async = input.sig.asyncness.is_some();
    let chain = collect_chain(&input.block);
    let meta = quote! {
        ::jigs::inventory::submit! {
            ::jigs::JigMeta {
                name: #name_str,
                file: file!(),
                line: line!(),
                kind: #kind_str,
                input: #input_str,
                is_async: #is_async,
                chain: &[#(#chain),*],
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

fn last_type_ident(ty: &Type) -> Option<String> {
    if let Type::Path(p) = ty {
        return Some(p.path.segments.last()?.ident.to_string());
    }
    None
}

fn collect_chain(block: &syn::Block) -> Vec<String> {
    struct V(Vec<String>);
    impl<'ast> Visit<'ast> for V {
        fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
            syn::visit::visit_expr(self, &m.receiver);
            if m.method == "then" {
                if let Some(Expr::Path(p)) = m.args.first() {
                    if let Some(seg) = p.path.segments.last() {
                        let name = seg.ident.to_string();
                        if !self.0.iter().any(|n| n == &name) {
                            self.0.push(name);
                        }
                    }
                }
            }
            for a in &m.args {
                syn::visit::visit_expr(self, a);
            }
        }
    }
    let mut v = V(Vec::new());
    v.visit_block(block);
    v.0
}
