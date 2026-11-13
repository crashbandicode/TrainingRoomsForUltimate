//! The counting global allocator (playbook: "No heap in hooks"; the install line's
//! `heap_allocations=`). The allocator, the `HookScope` that marks hook code and the per-hook
//! invocation counters are `model::hook_stats` (host-tested, with an intentionally allocating stub);
//! this file only installs the allocator for the plugin. The boot thread allocates freely (settings,
//! JSON) and is not counted.
//!
//! One global allocator per plugin: the diag heap features read `hook_stats`' byte meter (`plugin-heap-meter`:
//! `roster::heap_metrics`; `console-practice-trace`: its recorder's live/peak fields) instead of declaring allocators of their own.
//! `console-practice-trace` additionally records every allocation call as a trace event: the global allocator is then the
//! recorder's `TraceAllocator`, which forwards to this plugin's `CountingAllocator` (so the hook counter and the byte meter keep
//! counting).
//!
//! Part of the Switch build.

pub use crate::model::hook_stats::{hook_allocations, HookId, HookScope};

#[cfg(not(feature = "console-practice-trace"))]
#[global_allocator]
static ALLOCATOR: crate::model::hook_stats::CountingAllocator = crate::model::hook_stats::CountingAllocator;

#[cfg(feature = "console-practice-trace")]
#[global_allocator]
static ALLOCATOR: crate::runtime::roster::practice_trace::TraceAllocator = crate::runtime::roster::practice_trace::TraceAllocator;
