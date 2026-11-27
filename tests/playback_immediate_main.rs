macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}

#[path = "support/scratch.rs"]
mod test_scratch;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static DROPPED_BYTES: Cell<Option<usize>> = const { Cell::new(None) };
}

struct CacheAllocator;

#[global_allocator]
static ALLOCATOR: CacheAllocator = CacheAllocator;

unsafe impl GlobalAlloc for CacheAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _ = DROPPED_BYTES.try_with(|bytes| {
            if let Some(total) = bytes.get() { bytes.set(Some(total + layout.size())); }
        });
        System.dealloc(ptr, layout);
    }
}

fn cache_drop_bytes(drop_cache: impl FnOnce() -> Option<usize>) -> Option<(usize, usize)> {
    DROPPED_BYTES.with(|bytes| bytes.set(Some(0)));
    let inline = drop_cache();
    let heap = DROPPED_BYTES.with(|bytes| bytes.replace(None).unwrap());
    inline.map(|inline| (heap, inline))
}

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;
#[path = "support/playback_engine.rs"]
#[allow(dead_code, unused_imports)]
pub mod engine;
#[path = "playback_immediate.rs"]
mod playback_immediate;
