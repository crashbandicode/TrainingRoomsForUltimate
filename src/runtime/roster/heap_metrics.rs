//! Opt-in profiling of this NRO's requested Rust allocation sizes only.
//! Native game heaps, allocator metadata and other plugins are excluded.
//! Counters are relaxed and snapshots are not atomic across fields. The peak
//! covers the process lifetime; never reset it while other threads allocate.
//! Reallocation's internal temporary copy is outside this meter. Release builds
//! omit the module; this feature (part of `diag`) never enables the broad SD trace writer.
//!
//! The playbook's whole-plugin `heap_allocations=0` does not apply to this product (menus,
//! saved files, the Lua VM and Mii data allocate by design); the install line reports the
//! call count (`heap_allocations=<calls>`) so one build can be compared with the one before
//! it. WP-H's per-hook accounting (`in_hook=`) was never wired to a callback and the operator
//! ruled the no-heap rule N/A for this plugin (2026-10-31), so the meter keeps no hook depth.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering::Relaxed};

pub struct Meter;

static LIVE: AtomicIsize = AtomicIsize::new(0);
static PEAK: AtomicIsize = AtomicIsize::new(0);
static TOTAL: AtomicUsize = AtomicUsize::new(0);
static CALLS: AtomicUsize = AtomicUsize::new(0);
static LARGEST: AtomicUsize = AtomicUsize::new(0);

#[inline]
fn grow(size: usize) {
    let live = LIVE.fetch_add(size as isize, Relaxed) + size as isize;
    PEAK.fetch_max(live, Relaxed);
    TOTAL.fetch_add(size, Relaxed);
    CALLS.fetch_add(1, Relaxed);
    LARGEST.fetch_max(size, Relaxed);
}

unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = System.alloc(layout);
        if !pointer.is_null() { grow(layout.size()); }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = System.alloc_zeroed(layout);
        if !pointer.is_null() { grow(layout.size()); }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        System.dealloc(pointer, layout);
        LIVE.fetch_sub(layout.size() as isize, Relaxed);
    }

    // A moved realloc briefly holds both blocks; counting only the new size understates
    // that copy's instantaneous peak by the old size. Acceptable for sparse profiling.
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let moved = System.realloc(pointer, layout, size);
        if !moved.is_null() {
            LIVE.fetch_sub(layout.size() as isize, Relaxed);
            grow(size);
        }
        moved
    }
}

#[cfg(not(test))]
#[global_allocator]
static METER: Meter = Meter;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Heap {
    pub live: isize,
    pub peak: isize,
    pub total: usize,
    pub calls: usize,
    pub largest: usize,
}

pub fn snapshot() -> Heap {
    Heap {
        live: LIVE.load(Relaxed),
        peak: PEAK.load(Relaxed),
        total: TOTAL.load(Relaxed),
        calls: CALLS.load(Relaxed),
        largest: LARGEST.load(Relaxed),
    }
}

/// The install line's field: `heap_allocations=<calls>`, every allocation this NRO requested so far.
impl core::fmt::Display for Heap {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "heap_allocations={}", self.calls)
    }
}

// Host tests run from `training_mod_mii/tests/heap_metrics.rs` (the module is included by path).
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// The meter's counters are process-wide: run these tests one at a time.
    static SERIAL: Mutex<()> = Mutex::new(());

    // Calls the meter directly; the test binary keeps its default global allocator.
    #[test]
    fn alloc_realloc_and_free_return_live_to_its_start() {
        let _serial = SERIAL.lock().unwrap();
        let before = snapshot();
        unsafe {
            let first = Layout::from_size_align(4096, 8).unwrap();
            let pointer = Meter.alloc(first);
            assert!(!pointer.is_null());
            let moved = Meter.realloc(pointer, first, 1024);
            assert!(!moved.is_null());
            Meter.dealloc(moved, Layout::from_size_align(1024, 8).unwrap());
        }
        let after = snapshot();
        assert_eq!(after.live, before.live);
        assert!(after.peak >= before.live + 4096);
        assert_eq!(after.total, before.total + 4096 + 1024);
        assert_eq!(after.calls, before.calls + 2);
        assert!(after.largest >= 4096);
    }

    #[test]
    fn the_install_line_field_is_the_call_count() {
        let _serial = SERIAL.lock().unwrap();
        let before = snapshot();
        let layout = Layout::from_size_align(64, 8).unwrap();
        unsafe {
            let pointer = Meter.alloc(layout);
            assert!(!pointer.is_null());
            Meter.dealloc(pointer, layout);
        }
        let after = snapshot();
        assert_eq!(after.calls, before.calls + 1);
        assert_eq!(after.to_string(), format!("heap_allocations={}", after.calls));
    }
}
