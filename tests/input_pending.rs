//! Original regressions for the scalar TLS input handoff. The oracle is the
//! prior two-Cell representation, not another scalar-slot implementation.
#[path = "../src/model/input_pending.rs"]
mod input_pending;
#[path = "../src/model/input_delay.rs"]
#[allow(dead_code)]
mod input_delay;
#[path = "../src/model/input_adapter.rs"]
#[allow(dead_code)]
mod input_adapter;

use std::cell::{Cell, RefCell};
use input_pending::{Pending, Tls};

thread_local! {
    static WORDS: Cell<[usize; 16]> = const { Cell::new([0; 16]) };
    static METER: Cell<bool> = const { Cell::new(false) };
}
static ALLOCS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
struct Allocator;
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
unsafe impl std::alloc::GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        if METER.try_with(Cell::get).unwrap_or(false) {
            ALLOCS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        std::alloc::System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        std::alloc::System.dealloc(ptr, layout);
    }
}

const KEYS: [u32; 4] = [2, 9, 0, 15]; // deliberately noncontiguous; key 0 is valid
struct Api {
    calls: Cell<usize>,
    fail: usize,
    freed: RefCell<Vec<u32>>,
}
impl Api {
    fn new(fail: usize) -> Self {
        WORDS.set([0; 16]);
        Self { calls: Cell::new(0), fail, freed: RefCell::new(Vec::new()) }
    }
}
impl Tls for Api {
    type Error = u32;
    fn allocate(&self) -> Result<u32, u32> {
        let n = self.calls.replace(self.calls.get() + 1);
        if n == self.fail { Err(0x1203) } else { Ok(KEYS[n]) }
    }
    fn free(&self, key: u32) { self.freed.borrow_mut().push(key); }
    fn get(&self, key: u32) -> usize { WORDS.get()[key as usize] }
    fn set(&self, key: u32, value: usize) {
        let mut words = WORDS.get();
        words[key as usize] = value;
        WORDS.set(words);
    }
}

#[test]
fn reservation_failure_rolls_back_each_partial_prefix_before_publication() {
    for fail in 0..4 {
        let api = Api::new(fail);
        assert!(matches!(Pending::reserve(&api), Err(0x1203)));
        assert_eq!(api.calls.get(), fail + 1);
        assert_eq!(*api.freed.borrow(), KEYS[..fail].iter().rev().copied().collect::<Vec<_>>());
    }
    let api = Api::new(4);
    let pending = Pending::reserve(&api).unwrap();
    assert!(api.freed.borrow().is_empty());
    pending.release(&api);
    assert_eq!(*api.freed.borrow(), [15, 0, 9, 2]);
}

#[test]
fn sequences_match_original_cells_including_nested_overwrite_and_null_output() {
    let api = Api::new(4);
    let pending = Pending::reserve(&api).unwrap();
    let original = Cell::new((0usize, 0i64, 0usize));
    let controller = Cell::new(0usize);
    let note = |out, slot, holder, pad| {
        original.set((out, slot, holder));
        pending.note(&api, out, slot, holder);
        controller.set(pad);
        pending.set_controller(&api, pad);
    };
    let take = || assert_eq!(pending.take(&api), original.replace((0, 0, 0)));
    take(); // caller A can pass the apply site without having called mapping
    note(0x1000, -1, 0x2000, 0x3000);
    take();
    assert_eq!(pending.controller(&api), controller.get()); // non-P1 early return retains pad
    note(0, 0, 0, 0x4000);
    take(); // null output is consumed too
    assert_eq!(pending.controller(&api), controller.get());
    note(0x1000, 0, 0x2000, 0x3000);
    note(0x5000, 1, 0x6000, 0x7000); // current semantics: inner note replaces outer
    take();
    take(); // no invented restoration of an outer tuple
    note(0x8000, 0, 0x9000, 0xa000);
    take();
    note(0xb000, 0, 0xc000, 0xd000); // a nested helper before the outer controller take
    assert_eq!(pending.take_controller(&api), controller.replace(0));
    assert_eq!(pending.controller(&api), controller.get());
    take();
}

#[test]
fn interleaved_threads_and_a_later_thread_have_independent_zero_initial_state() {
    let api = Api::new(4);
    let pending = Pending::reserve(&api).unwrap();
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        for n in 1..=2 {
            let pending = &pending;
            let barrier = &barrier;
            scope.spawn(move || {
                let api = Api::new(4);
                assert_eq!(pending.take(&api), (0, 0, 0));
                assert_eq!(pending.controller(&api), 0);
                pending.note(&api, n * 100, n as i64, n * 200);
                pending.set_controller(&api, n * 300);
                barrier.wait();
                assert_eq!(pending.take(&api), (n * 100, n as i64, n * 200));
                assert_eq!(pending.take_controller(&api), n * 300);
            });
        }
    });
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let api = Api::new(4);
            assert_eq!(pending.take(&api), (0, 0, 0));
            assert_eq!(pending.controller(&api), 0);
        });
    });
}

#[test]
fn first_and_repeated_pending_operations_allocate_nothing() {
    let api = Api::new(4);
    let pending = Pending::reserve(&api).unwrap();
    ALLOCS.store(0, std::sync::atomic::Ordering::Relaxed);
    METER.set(true);
    for n in 0..100 {
        pending.note(&api, n, -1, n + 1);
        pending.set_controller(&api, n + 2);
        std::hint::black_box(pending.take(&api));
        std::hint::black_box(pending.controller(&api));
        std::hint::black_box(pending.take_controller(&api));
    }
    METER.set(false);
    assert_eq!(ALLOCS.load(std::sync::atomic::Ordering::Relaxed), 0);
}

#[test]
fn pending_identity_survives_queue_reset_and_every_input_owner() {
    use input_adapter::InputAdapter;
    use input_delay::{Clear, MappedInputs, Owners};
    use std::sync::atomic::{AtomicU8, Ordering};
    let api = Api::new(4);
    let pending = Pending::reserve(&api).unwrap();
    let mut adapter = InputAdapter::new();
    let mask = AtomicU8::new(0);
    let actual = MappedInputs { buttons: 0x20, ..MappedInputs::NEUTRAL };
    for why in [Clear::Session, Clear::SaveStateLoad, Clear::TrainingReset, Clear::NativePause] {
        pending.note(&api, 0x1000, 0, 0x2000);
        pending.set_controller(&api, 0x3000);
        adapter.poll(&mask, 2, actual);
        adapter.clear(why);
        assert_eq!(pending.take(&api), (0x1000, 0, 0x2000));
        assert_eq!(pending.take_controller(&api), 0x3000);
        assert_eq!(adapter.poll(&mask, 2, actual).0, MappedInputs::NEUTRAL);
    }
    for owner in [Owners::MENU, Owners::STOP_CHORD, Owners::NATIVE_UI_BUSY, Owners::REPLAY_EDITOR, Owners::VANILLA_PAUSE] {
        pending.note(&api, 0x4000, 0, 0x5000);
        mask.store(owner.bits(), Ordering::Release);
        assert_eq!(pending.take(&api), (0x4000, 0, 0x5000));
        assert_eq!(adapter.poll(&mask, 2, actual).0, MappedInputs::NEUTRAL);
        assert_eq!(adapter.queued(), 0);
        mask.store(0, Ordering::Release);
        assert_eq!(adapter.poll(&mask, 2, actual).0, MappedInputs::NEUTRAL);
    }
}
