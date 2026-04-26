//! `jigs` — explicit, composable, traceable processing pipelines.
//!
//! This crate is a thin facade that re-exports the runtime from
//! [`jigs_core`] and the procedural macros from [`jigs_macros`]. When the
//! `trace` feature is enabled, [`jigs_trace`] is re-exported as `trace` and
//! `#[jig]` instruments each call site to record into its buffer.

pub use jigs_core::*;
pub use jigs_macros::jig;

#[cfg(feature = "trace")]
pub use jigs_trace as trace;
