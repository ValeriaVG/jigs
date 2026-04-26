//! Procedural macros for the `jigs` framework.
//!
//! Currently `#[jig]` is a pass-through placeholder. It will eventually
//! instrument the annotated item and emit graph metadata at compile time.

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn jig(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
