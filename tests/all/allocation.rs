//! Route each allocation to the existing test probes and the production hook meter.

use crate::model::hook_stats::CountingAllocator;
use std::alloc::{GlobalAlloc, Layout};
use std::sync::{Mutex, MutexGuard};

#[global_allocator]
static ALLOCATOR: IntegrationAllocator = IntegrationAllocator;

// The production meter's hook counters are process-wide. Serialize only probes
// that assert on its snapshots. The other probes use thread-local measurements.
pub(super) fn hook_counter_turn() -> MutexGuard<'static, ()> {
    static TURN: Mutex<()> = Mutex::new(());
    TURN.lock().unwrap_or_else(|error| error.into_inner())
}

fn count_allocation() {
    crate::catalog_reply::count_allocation();
    crate::cpu_core::count_allocation();
    crate::hook_tls::count_allocation();
    crate::input04::count_allocation();
    crate::input05::count_allocation();
    crate::input_delay::count_allocation();
    crate::input_pending::count_allocation();
    crate::ui_chord_capture::count_allocation();
}

struct IntegrationAllocator;

unsafe impl GlobalAlloc for IntegrationAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        CountingAllocator.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        CountingAllocator.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count_allocation();
        CountingAllocator.realloc(ptr, layout, size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        CountingAllocator.dealloc(ptr, layout);
    }
}
