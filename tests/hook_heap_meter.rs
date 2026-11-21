//! Exact process-wide byte deltas for the allocator used by the plugin.
//!
//! This suite uses the System allocator for unrelated allocations, leaving the
//! production meter free to record only the direct calls below. hook_alloc.rs
//! checks the shared global allocator with real hook scopes.


use crate::model::hook_stats::{heap_bytes, CountingAllocator};

static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn the_byte_meter_counts_exact_allocator_deltas() {
    use std::alloc::{GlobalAlloc, Layout};
    let before = heap_bytes();
    let first = Layout::from_size_align(1 << 20, 8).unwrap();
    // SAFETY: the layouts are valid and every block is freed with the layout it was allocated or reallocated with.
    unsafe {
        let pointer = ALLOCATOR.alloc(first);
        assert!(!pointer.is_null());
        let held = heap_bytes();
        assert_eq!(held.live, before.live + (1 << 20), "live holds the block");
        assert!(held.peak >= held.live);
        assert_eq!((held.total, held.calls), (before.total + (1 << 20), before.calls + 1));
        assert!(held.largest >= 1 << 20);
        let moved = ALLOCATOR.realloc(pointer, first, 1 << 10);
        assert!(!moved.is_null());
        let shrunk = heap_bytes();
        assert_eq!(shrunk.live, before.live + (1 << 10), "the old block left `live` at the realloc, the new size is live");
        assert_eq!(shrunk.total, held.total + (1 << 10), "the new size counts as requested");
        assert_eq!(shrunk.peak, held.peak, "peak never falls");
        ALLOCATOR.dealloc(moved, Layout::from_size_align(1 << 10, 8).unwrap());
    }
    let after = heap_bytes();
    assert_eq!(after.live, before.live, "the freed block is not live any more");
}
