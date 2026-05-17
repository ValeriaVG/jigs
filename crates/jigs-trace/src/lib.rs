#![warn(missing_docs)]
//! Per-jig execution tracing.
//!
//! Wires into `#[jig]` when the `trace` feature is enabled on the `jigs`
//! umbrella crate (or on `jigs-macros` directly). Each instrumented jig
//! records its metadata, depth, wall-clock duration and outcome into a
//! thread-local buffer that callers can drain with [`take`].

use std::cell::{Cell, RefCell};
use std::time::Duration;

pub use jigs_core::JigMeta;
pub use jigs_core::Status;

/// One recorded jig invocation.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Entry {
    /// Compile-time metadata for the jig that ran.
    pub meta: &'static JigMeta,
    /// Nesting depth at the time of entry (top-level jigs are depth 0).
    pub depth: usize,
    /// Wall-clock time spent inside the jig.
    pub duration: Duration,
    /// `true` if the jig produced a successful outcome.
    pub ok: bool,
    /// Error message captured from the jig's output, if any.
    pub error: Option<String>,
}

impl Entry {
    /// Function name of the jig.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.meta.name
    }

    /// Construct an `Entry` with the given fields.
    #[must_use]
    pub fn new(
        meta: &'static JigMeta,
        depth: usize,
        duration: Duration,
        ok: bool,
        error: Option<String>,
    ) -> Self {
        Self {
            meta,
            depth,
            duration,
            ok,
            error,
        }
    }
}

thread_local! {
    static DEPTH: Cell<usize> = const { Cell::new(0) };
    static BUFFER: RefCell<Vec<Entry>> = const { RefCell::new(Vec::new()) };
}

/// Record the start of a jig invocation. Returns an index used by [`exit`]
/// to close the same entry. Called by code generated from `#[jig]`.
#[must_use]
pub fn enter(meta: &'static JigMeta) -> usize {
    let depth = DEPTH.with(|d| {
        let v = d.get();
        d.set(v + 1);
        v
    });
    BUFFER.with(|b| {
        let mut buf = b.borrow_mut();
        let idx = buf.len();
        buf.push(Entry {
            meta,
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
#[must_use]
pub fn take() -> Vec<Entry> {
    DEPTH.with(|d| d.set(0));
    BUFFER.with(|b| std::mem::take(&mut *b.borrow_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(name: &'static str) -> &'static JigMeta {
        Box::leak(Box::new(JigMeta {
            name,
            file: "",
            line: 0,
            kind: "Response",
            input: "Request",
            input_type: "",
            output_type: "",
            is_async: false,
            module: "",
            chain: &[],
        }))
    }

    #[test]
    fn enter_exit_records_one_entry() {
        let m = meta("step");
        let idx = enter(m);
        exit(idx, Duration::from_micros(42), true, None);
        let entries = take();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name(), "step");
        assert_eq!(entries[0].depth, 0);
        assert!(entries[0].ok);
        assert_eq!(entries[0].duration, Duration::from_micros(42));
        assert!(entries[0].error.is_none());
    }

    #[test]
    fn depth_increases_for_nested_calls() {
        let outer = meta("outer");
        let inner = meta("inner");
        let i0 = enter(outer);
        let i1 = enter(inner);
        exit(i1, Duration::ZERO, true, None);
        exit(i0, Duration::ZERO, true, None);
        let entries = take();
        assert_eq!(entries[0].depth, 0);
        assert_eq!(entries[1].depth, 1);
    }

    #[test]
    fn error_is_recorded() {
        let m = meta("fail");
        let idx = enter(m);
        exit(idx, Duration::ZERO, false, Some("boom".into()));
        let entries = take();
        assert!(!entries[0].ok);
        assert_eq!(entries[0].error.as_deref(), Some("boom"));
    }

    #[test]
    fn take_drains_buffer() {
        let m = meta("x");
        let idx = enter(m);
        exit(idx, Duration::ZERO, true, None);
        let first = take();
        let second = take();
        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
    }

    #[test]
    fn take_resets_depth() {
        let m = meta("a");
        let idx = enter(m);
        exit(idx, Duration::ZERO, true, None);
        let _ = take();
        let idx2 = enter(m);
        exit(idx2, Duration::ZERO, true, None);
        let entries = take();
        assert_eq!(entries[0].depth, 0);
    }

    #[test]
    fn entry_exposes_full_meta() {
        let m = meta("with_meta");
        let idx = enter(m);
        exit(idx, Duration::ZERO, true, None);
        let entries = take();
        assert!(std::ptr::eq(entries[0].meta, m));
    }
}
