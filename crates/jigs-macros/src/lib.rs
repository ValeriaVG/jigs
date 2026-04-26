//! Procedural macros for the `jigs` framework.
//!
//! `#[jig]` instruments a function so that each invocation records its name
//! and wall-clock duration into the thread-local trace buffer in `jigs-core`.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn jig(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let name_str = input.sig.ident.to_string();
    let vis = &input.vis;
    let sig = &input.sig;
    let block = &input.block;

    let expanded = quote! {
        #vis #sig {
            let __jig_idx = ::jigs::trace::enter(#name_str);
            let __jig_start = ::std::time::Instant::now();
            let __jig_result = (move || #block)();
            let __jig_ok = ::jigs::Status::ok(&__jig_result);
            let __jig_err = ::jigs::Status::error(&__jig_result);
            ::jigs::trace::exit(__jig_idx, __jig_start.elapsed(), __jig_ok, __jig_err);
            __jig_result
        }
    };

    expanded.into()
}
