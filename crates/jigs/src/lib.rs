#![warn(missing_docs)]
//! `jigs` — explicit, composable, traceable processing pipelines.
//!
//! A *jig* is one step in a request-to-response pipeline. Steps are chained
//! with [`Request::then`], [`Response::then`] and [`Branch::then`], and the
//! type system enforces ordering: once a [`Response`] exists you cannot fall
//! back to chaining a [`Request`]-shaped step.
//!
//! This crate is a thin facade. The runtime types live in [`jigs_core`] and
//! the [`jig`] attribute macro lives in [`jigs_macros`]. With the optional
//! `trace` feature, every `#[jig]` call site records its name, depth, outcome
//! and wall-clock duration into a per-thread buffer exposed under [`trace`].
//!
//! # Example
//!
//! ```
//! use jigs::{jig, Request, Response};
//!
//! #[jig]
//! fn validate(r: Request<u32>) -> Request<u32> { r }
//!
//! #[jig]
//! fn handle(r: Request<u32>) -> Response<String> {
//!     Response::ok(format!("got {}", r.0))
//! }
//!
//! let response = Request(42u32).then(validate).then(handle);
//! assert_eq!(response.inner.unwrap(), "got 42");
//! ```
//!
//! # Features
//!
//! - `trace` — pulls in [`jigs_trace`] (data) and [`jigs_log`] (renderers),
//!   and instruments every `#[jig]` call.

pub use jigs_core::*;
pub use jigs_core::{__fork_chain, fork};
pub use jigs_macros::jig;

#[cfg(feature = "trace")]
pub use jigs_trace as trace;

#[cfg(feature = "trace")]
pub use jigs_log as log;

pub use jigs_map as map;
