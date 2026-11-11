//! Exercises the exact Switch counter storage adapter with a checked native
//! boundary stub. Counter attribution itself is exercised by hook_alloc.
// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;
#[path = "../src/runtime/hook_tls.rs"]
mod adapter;
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use model::scalar_tls::{Bank, Tls};
thread_local! {
    static WORDS: Cell<[usize; 16]> = const { Cell::new([0; 16]) };
    static METER: Cell<bool> = const { Cell::new(false) };
}
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
struct Allocator;
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
unsafe impl std::alloc::GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        if METER.try_with(Cell::get).unwrap_or(false) { ALLOCS.fetch_add(1, Relaxed); }
        std::alloc::System.alloc(layout)
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: std::alloc::Layout) { std::alloc::System.dealloc(p, layout); }
}
const KEYS: [u32; 4] = [9, 0, 15, 2];
static RESERVED: AtomicUsize = AtomicUsize::new(0);
mod native_tls {
    use super::*;
    pub(crate) struct NativeTls;
    impl Tls for NativeTls {
        type Error = u32;
        fn allocate(&self) -> Result<u32, u32> {
            // Simulate allocator reentry during native reservation. It must
            // see no published cell, so cannot recurse into get/set.
            assert!(adapter::HOOK_DEPTH.try_with(|v| v.get()).is_err());
            let index = RESERVED.fetch_add(1, Relaxed);
            Ok(KEYS[index])
        }
        fn free(&self, _: u32) { panic!("successful published keys must stay reserved"); }
        fn get(&self, key: u32) -> usize {
            assert_eq!(RESERVED.load(Relaxed), 4);
            assert!(KEYS.contains(&key));
            WORDS.get()[key as usize]
        }
        fn set(&self, key: u32, value: usize) {
            assert_eq!(RESERVED.load(Relaxed), 4);
            assert!(KEYS.contains(&key));
            let mut words = WORDS.get(); words[key as usize] = value; WORDS.set(words);
        }
    }
}

#[test]
fn actual_counter_adapter_bootstrap_first_use_nested_values_and_threads() {
    use adapter::{CURRENT, HOOK_DEPTH, LOAD_DEPTH, UNCOUNTED_DEPTH};
    let other = model::hook_stats::HookId::Other as u8;
    assert!(HOOK_DEPTH.try_with(|v| v.get()).is_err());
    METER.set(true);
    adapter::init().unwrap();
    adapter::init().unwrap(); // idempotent, no second reservation
    for _ in 0..100 {
        assert_eq!(HOOK_DEPTH.try_with(|v| v.get()), Ok(0));
        assert_eq!(CURRENT.try_with(|v| v.get()), Ok(other));
        let outer = CURRENT.try_with(|v| v.replace(3)).unwrap();
        HOOK_DEPTH.try_with(|v| v.set(v.get() + 1)).unwrap();
        let inner = CURRENT.try_with(|v| v.replace(17)).unwrap();
        HOOK_DEPTH.try_with(|v| v.set(v.get() + 1)).unwrap();
        LOAD_DEPTH.try_with(|v| v.set(2)).unwrap();
        UNCOUNTED_DEPTH.try_with(|v| v.set(1)).unwrap();
        assert_eq!(CURRENT.try_with(|v| v.get()), Ok(17));
        assert_eq!(HOOK_DEPTH.try_with(|v| v.get()), Ok(2));
        CURRENT.try_with(|v| v.set(inner)).unwrap();
        HOOK_DEPTH.try_with(|v| v.set(v.get().saturating_sub(1))).unwrap();
        assert_eq!(CURRENT.try_with(|v| v.get()), Ok(3));
        assert_eq!(LOAD_DEPTH.try_with(|v| v.replace(0)), Ok(2));
        assert_eq!(UNCOUNTED_DEPTH.try_with(|v| v.replace(0)), Ok(1));
        CURRENT.try_with(|v| v.set(outer)).unwrap();
        HOOK_DEPTH.try_with(|v| v.set(v.get().saturating_sub(1))).unwrap();
    }
    METER.set(false);
    assert_eq!(ALLOCS.load(Relaxed), 0, "boot and first/repeated adapter access");
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        for value in [6, 23] {
            let barrier = &barrier;
            scope.spawn(move || {
                METER.set(true);
                assert_eq!(CURRENT.try_with(|v| v.get()), Ok(other));
                assert_eq!(HOOK_DEPTH.try_with(|v| v.get()), Ok(0));
                CURRENT.try_with(|v| v.set(value)).unwrap();
                METER.set(false);
                barrier.wait();
                METER.set(true);
                assert_eq!(CURRENT.try_with(|v| v.get()), Ok(value));
                METER.set(false);
            });
        }
    });
    assert_eq!(CURRENT.try_with(|v| v.get()), Ok(other));
    assert_eq!(ALLOCS.load(Relaxed), 0, "first access on fresh threads");
}

#[test]
fn every_bank_failure_leaves_bootstrap_unavailable_and_rolls_back() {
    struct Api<'a> { bank: &'a Bank<4>, fail: usize, calls: Cell<usize>, frees: Cell<usize> }
    impl Tls for Api<'_> {
        type Error = u32;
        fn allocate(&self) -> Result<u32, u32> {
            assert!(self.bank.cell(self, 0, 0).is_none());
            let n = self.calls.replace(self.calls.get() + 1);
            if n == self.fail { Err(0x1203) } else { Ok(KEYS[n]) }
        }
        fn free(&self, key: u32) {
            assert!(self.bank.cell(self, 0, 0).is_none());
            let n = self.frees.replace(self.frees.get() + 1);
            assert_eq!(key, KEYS[self.fail - n - 1]);
        }
        fn get(&self, _: u32) -> usize { panic!("unpublished key read"); }
        fn set(&self, _: u32, _: usize) { panic!("unpublished key write"); }
    }
    for fail in 0..4 {
        let bank = Bank::new();
        let api = Api { bank: &bank, fail, calls: Cell::new(0), frees: Cell::new(0) };
        assert_eq!(bank.init(&api), Err(0x1203));
        assert!(bank.cell(&api, 3, 39).is_none());
        assert_eq!(api.frees.get(), fail);
    }
}
