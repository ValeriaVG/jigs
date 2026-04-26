//! Per-jig execution tracing.
//!
//! Wires into `#[jig]` when the `trace` feature is enabled on the `jigs`
//! umbrella crate (or on `jigs-macros` directly). Each instrumented jig
//! records its name, depth, wall-clock duration and outcome into a
//! thread-local buffer that callers can drain with [`take`].

use std::cell::{Cell, RefCell};
use std::time::Duration;

pub use jigs_core::Status;

pub struct Entry {
    pub name: &'static str,
    pub depth: usize,
    pub duration: Duration,
    pub ok: bool,
    pub error: Option<String>,
}

thread_local! {
    static DEPTH: Cell<usize> = const { Cell::new(0) };
    static BUFFER: RefCell<Vec<Entry>> = const { RefCell::new(Vec::new()) };
}

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

pub fn exit(idx: usize, duration: Duration, ok: bool, error: Option<String>) {
    DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    BUFFER.with(|b| {
        let mut buf = b.borrow_mut();
        buf[idx].duration = duration;
        buf[idx].ok = ok;
        buf[idx].error = error;
    });
}

pub fn take() -> Vec<Entry> {
    DEPTH.with(|d| d.set(0));
    BUFFER.with(|b| std::mem::take(&mut *b.borrow_mut()))
}
