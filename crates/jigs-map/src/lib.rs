#![warn(missing_docs)]
//! HTML and Mermaid map generators for `jigs` pipelines.
//!
//! Both renderers accept an iterator of [`JigMeta`] references, typically
//! produced by the `jigs!` macro's generated `all_jigs()` function. The
//! entry point is inferred from the first jig in the iterator. Call from
//! any binary in a crate that defines (or imports) the jigs you want
//! mapped.
//!
//! ```ignore
//! fn main() -> std::io::Result<()> {
//!     let dir = env!("CARGO_MANIFEST_DIR");
//!     std::fs::write(format!("{dir}/map.html"),
//!         jigs_map::to_html(jigs::all_jigs(), "my service", None))?;
//!     std::fs::write(format!("{dir}/map.md"),
//!         jigs_map::to_markdown(jigs::all_jigs(), "my service"))?;
//!     Ok(())
//! }
//! ```

pub mod html;
pub mod mermaid;

pub use html::to_html;
pub use mermaid::{to_markdown, to_mermaid};
