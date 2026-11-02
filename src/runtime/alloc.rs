//! The counting global allocator (playbook: "No heap in hooks"; the install line's
//! `heap_allocations=`). The allocator, the `HookScope` that marks hook code and the per-hook
//! invocation counters are `model::hook_stats` (host-tested, with an intentionally allocating stub);
//! this file only installs the allocator for the plugin. The boot thread allocates freely (settings,
//! JSON) and is not counted.
//!
//! NOT YET SWITCH-COMPILED.

pub use crate::model::hook_stats::{hook_allocations, HookId, HookScope};

#[global_allocator]
static ALLOCATOR: crate::model::hook_stats::CountingAllocator = crate::model::hook_stats::CountingAllocator;
