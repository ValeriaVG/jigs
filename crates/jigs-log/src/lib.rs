#![warn(missing_docs)]
//! Renderers for [`jigs_trace`] entries.
//!
//! Two output modes are provided:
//!
//! - [`render_tree`] — human-readable, indented by depth, like the README.
//! - [`render_ndjson`] — one JSON object per entry per line, suitable for
//!   automated log ingestion.

pub use jigs_trace::Entry;

mod json;
mod tree;

pub use json::render_ndjson;
pub use tree::render_tree;
