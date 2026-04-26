#![warn(missing_docs)]
//! Per-jig execution tracing.
//!
//! Wires into `#[jig]` when the `trace` feature is enabled on the `jigs`
//! umbrella crate (or on `jigs-macros` directly). Each instrumented jig
//! records its name, depth, wall-clock duration and outcome into a
//! thread-local buffer that callers can drain with [`take`].

use std::cell::{Cell, RefCell};
use std::time::Duration;

pub use jigs_core::Status;

/// One recorded jig invocation.
pub struct Entry {
    /// Function name of the jig.
    pub name: &'static str,
    /// Nesting depth at the time of entry (top-level jigs are depth 0).
    pub depth: usize,
    /// Wall-clock time spent inside the jig.
    pub duration: Duration,
    /// `true` if the jig produced a successful outcome.
    pub ok: bool,
    /// Error message captured from the jig's output, if any.
    pub error: Option<String>,
}

thread_local! {
    static DEPTH: Cell<usize> = const { Cell::new(0) };
    static BUFFER: RefCell<Vec<Entry>> = const { RefCell::new(Vec::new()) };
}

/// Record the start of a jig invocation. Returns an index used by [`exit`]
/// to close the same entry. Called by code generated from `#[jig]`.
pub fn enter(name: &'static str) -> usize {
    let depth = DEPTH.with(|d| {
        let v = d.get();
        d.set(v + 1);
        v
    });
    BUFFER.with(|b| {
        let mut buf = b.borrow_mut();
        let idx = buf.len();
        buf.push(Entry {
            name,
            depth,
            duration: Duration::ZERO,
            ok: true,
            error: None,
        });
        idx
    })
}

/// Close the entry at `idx` with its measured duration and outcome.
/// Called by code generated from `#[jig]`.
pub fn exit(idx: usize, duration: Duration, ok: bool, error: Option<String>) {
    DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    BUFFER.with(|b| {
        let mut buf = b.borrow_mut();
        buf[idx].duration = duration;
        buf[idx].ok = ok;
        buf[idx].error = error;
    });
}

/// Drain the current thread's trace buffer and reset depth tracking.
/// Call once per request after the pipeline finishes.
pub fn take() -> Vec<Entry> {
    DEPTH.with(|d| d.set(0));
    BUFFER.with(|b| std::mem::take(&mut *b.borrow_mut()))
}
