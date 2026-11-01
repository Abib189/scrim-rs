//! One budget for the isolated layers a render holds at once (upstream FigCraft 96eed159, audit
//! #1061 RND-2): each nested layer with opacity, effects, a mask or a clip keeps its pixmaps while
//! the layers inside it draw, so a crafted nest of large ones could ask for more memory than there
//! is. A layer past the budget is left out (with a warning, once) instead.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

/// Bytes the isolated layers being drawn on one thread may hold at once: two of the largest layer
/// there is (16,384 px square).
pub const BUDGET: u64 = 2 << 30;

thread_local! {
    /// Bytes held now on this thread, the most held while a test measures, and the budget (tests
    /// lower it).
    static HELD: Cell<u64> = const { Cell::new(0) };
    static PEAK: Cell<u64> = const { Cell::new(0) };
    static LIMIT: Cell<u64> = const { Cell::new(BUDGET) };
}

static WARNED: AtomicBool = AtomicBool::new(false);

/// A share of the budget, given back when dropped (on unwinding too).
pub(crate) struct Held(u64);

impl Held {
    /// Room for `count` RGBA pixmaps of `w` × `h` pixels, if the budget has it.
    pub(crate) fn take(w: u32, h: u32, count: u64) -> Option<Held> {
        let bytes = u64::from(w).saturating_mul(u64::from(h)).saturating_mul(4).saturating_mul(count);
        let total = HELD.with(Cell::get).saturating_add(bytes);
        let limit = LIMIT.with(Cell::get);
        if total > limit {
            if !WARNED.swap(true, Ordering::Relaxed) {
                log::warn!("render: nested layers need more than {} MB at once; the innermost are left out", limit >> 20);
            }
            return None;
        }
        HELD.with(|c| c.set(total));
        PEAK.with(|c| c.set(c.get().max(total)));
        Some(Held(bytes))
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        HELD.with(|c| c.set(c.get().saturating_sub(self.0)));
    }
}

/// Tests: run `f` with a budget of `limit` bytes on this thread; returns what it gave and the most
/// the layers held at once.
#[cfg(test)]
pub(crate) fn within<T>(limit: u64, f: impl FnOnce() -> T) -> (T, u64) {
    let was = LIMIT.with(|c| c.replace(limit));
    PEAK.with(|c| c.set(HELD.with(Cell::get)));
    let out = f();
    LIMIT.with(|c| c.set(was));
    (out, PEAK.with(Cell::get))
}
