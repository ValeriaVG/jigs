//! `jigs` — explicit, composable, traceable processing pipelines.
//!
//! This crate is a thin facade that re-exports the runtime from
//! [`jigs_core`] and the procedural macros from [`jigs_macros`].

pub use jigs_core::*;
pub use jigs_macros::jig;
