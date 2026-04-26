//! Procedural macros for the `jigs` framework.
//!
//! `#[jig]` marks a function as a pipeline step. With the `trace` feature
//! enabled, each invocation also records its name and wall-clock duration
//! into the thread-local trace buffer in `jigs-trace`; without it, the
//! attribute is a zero-cost marker that leaves the body untouched.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemFn, ReturnType};

#[proc_macro_attribute]
pub fn jig(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let vis = &input.vis;
    let block = &input.block;

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

        #[cfg(feature = "trace")]
        let body = {
            let name_str = input.sig.ident.to_string();
            quote! {
                ::jigs::Pending(async move {
                    let __jig_idx = ::jigs::trace::enter(#name_str);
                    let __jig_start = ::std::time::Instant::now();
                    let __jig_result = (async move #block).await;
                    let __jig_ok = ::jigs::Status::ok(&__jig_result);
                    let __jig_err = ::jigs::Status::error(&__jig_result);
                    ::jigs::trace::exit(__jig_idx, __jig_start.elapsed(), __jig_ok, __jig_err);
                    __jig_result
                })
            }
        };
        #[cfg(not(feature = "trace"))]
        let body = quote! {
            ::jigs::Pending(async move #block)
        };

        return quote! { #vis #sig { #body } }.into();
    }

    let sig = &input.sig;

    #[cfg(feature = "trace")]
    let body = {
        let name_str = input.sig.ident.to_string();
        quote! {
            let __jig_idx = ::jigs::trace::enter(#name_str);
            let __jig_start = ::std::time::Instant::now();
            let __jig_result = (move || #block)();
            let __jig_ok = ::jigs::Status::ok(&__jig_result);
            let __jig_err = ::jigs::Status::error(&__jig_result);
            ::jigs::trace::exit(__jig_idx, __jig_start.elapsed(), __jig_ok, __jig_err);
            __jig_result
        }
    };
    #[cfg(not(feature = "trace"))]
    let body = quote! { #block };

    quote! { #vis #sig { #body } }.into()
}
