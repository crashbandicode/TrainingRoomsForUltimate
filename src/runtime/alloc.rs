//! The counting global allocator (playbook: "No heap in hooks"; the install line's
//! `heap_allocations=`). It forwards to the system allocator and counts the allocations a thread
//! makes while it is inside a hook (`HookScope`), so the count is the number of heap allocations
//! that hook code made, which must stay 0. The boot thread allocates freely (settings, JSON) and is
//! not counted.
//!
//! NOT YET SWITCH-COMPILED.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct CountingAllocator;

thread_local! {
    /// Nesting depth of hooks on this thread. A const-initialised `Cell` without a destructor needs
    /// no allocation to exist.
    static HOOK_DEPTH: Cell<u32> = const { Cell::new(0) };
}

static HOOK_ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

fn in_hook() -> bool {
    HOOK_DEPTH.try_with(|depth| depth.get() > 0).unwrap_or(false)
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if in_hook() {
            HOOK_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if in_hook() {
            HOOK_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        System.alloc_zeroed(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if in_hook() {
            HOOK_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Marks the current thread as running hook code until dropped.
pub struct HookScope;

impl HookScope {
    pub fn enter() -> Self {
        let _ = HOOK_DEPTH.try_with(|depth| depth.set(depth.get() + 1));
        HookScope
    }
}

impl Drop for HookScope {
    fn drop(&mut self) {
        let _ = HOOK_DEPTH.try_with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

/// Allocations made inside hooks since boot.
pub fn hook_allocations() -> u64 {
    HOOK_ALLOCATIONS.load(Ordering::Relaxed)
}
