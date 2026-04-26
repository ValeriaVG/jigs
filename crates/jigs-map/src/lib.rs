#![warn(missing_docs)]
//! HTML and Mermaid map generators for `jigs` pipelines.
//!
//! Both renderers read the global `JigMeta` inventory populated by the
//! `#[jig]` macro. Call from any binary in a crate that defines (or imports)
//! the jigs you want mapped — the linker pulls them in and the inventory
//! iteration finds them.
//!
//! ```ignore
//! fn main() -> std::io::Result<()> {
//!     let dir = env!("CARGO_MANIFEST_DIR");
//!     std::fs::write(format!("{dir}/map.html"),
//!         jigs_map::to_html(Some("handle"), "my service", None))?;
//!     std::fs::write(format!("{dir}/map.md"),
//!         jigs_map::to_markdown(Some("handle"), "my service"))?;
//!     Ok(())
//! }
//! ```

pub mod html;
pub mod mermaid;

pub use html::to_html;
pub use mermaid::{to_markdown, to_mermaid};
