//! Opt-in profiling of this NRO's requested Rust allocation sizes only.
//! Native game heaps, allocator metadata and other plugins are excluded.
//! Counters are relaxed and snapshots are not atomic across fields. The peak
//! covers the process lifetime; never reset it while other threads allocate.
//! Reallocation's internal temporary copy is outside this meter. Release builds
//! omit the module; this feature (part of `diag`) never enables the broad SD trace writer.
//!
//! The meter is the byte meter of the plugin's one counting allocator (`model::hook_stats`, package 1): the diag build reports
//! it in a `[HeapMeter]` line at every `heap_profile` mark of the preload metrics and, sparsely, next to the `hook_stats` line
//! (`runtime::diag_log`). The install line's `heap_allocations=` is package 1's hook-allocation count, not this meter's call count
//! (the fork's `HeapFields` wrote the call count under that name; it is not carried).

/// What the meter reads: package 1's counting allocator's byte meter (`model::hook_stats`, enabled by this feature): live, peak and
/// total requested bytes, the call count and the largest request.
pub use crate::model::hook_stats::HeapBytes as Heap;

/// The counters now.
pub fn snapshot() -> Heap {
    crate::model::hook_stats::heap_bytes()
}


