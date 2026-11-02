//! SPEC-input-03 (Input Delay). Test paths are the spec's §9 rows (`delay::…`).

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

/// Counts the allocations the current thread makes while `COUNTING` is set.
struct Counting;

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.with(Cell::get) {
            ALLOCATIONS.with(|n| n.set(n.get() + 1));
        }
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if COUNTING.with(Cell::get) {
            ALLOCATIONS.with(|n| n.set(n.get() + 1));
        }
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

mod delay {
    use super::model::input_delay::{applies, Clear, DelayQueue, MappedInputs, Owners, PollStage, MAX_DELAY, POLL_ORDER};
    use super::model::settings::Snapshot;
    use super::{ALLOCATIONS, COUNTING};

    /// A record whose buttons carry the poll number, so each output names the poll it came from.
    fn rec(n: u32) -> MappedInputs {
        MappedInputs { buttons: n, lstick_x: (n % 100) as i8, lstick_y: 0, rstick_x: 0, rstick_y: 0 }
    }

    fn run(queue: &mut DelayQueue, delay: u8, from: u32, polls: u32) -> Vec<MappedInputs> {
        (from..from + polls).map(|n| queue.step(delay, rec(n))).collect()
    }

    #[test]
    fn menu_default_zero() {
        assert_eq!(Snapshot::factory().input_delay(), 0);
        assert_eq!(MAX_DELAY, 30);
        // The queue adds nothing at the default.
        let mut q = DelayQueue::new();
        for n in 1..50 {
            assert_eq!(q.step(0, rec(n)), rec(n));
            assert!(q.is_empty());
        }
    }

    #[test]
    fn steady_state_offsets() {
        for d in [0u8, 1, 15, 30] {
            let mut q = DelayQueue::new();
            let out = run(&mut q, d, 1, 100);
            for (i, got) in out.iter().enumerate() {
                let poll = i as u32 + 1;
                let want = if poll > u32::from(d) { rec(poll - u32::from(d)) } else { MappedInputs::NEUTRAL };
                assert_eq!(*got, want, "d={d} poll {poll}");
            }
            assert_eq!(q.len(), usize::from(d));
        }
        // Past the maximum behaves as the maximum.
        let mut q = DelayQueue::new();
        let out = run(&mut q, 200, 1, 40);
        assert_eq!(out[39], rec(10));
    }

    #[test]
    fn initial_fill_neutral() {
        let mut q = DelayQueue::new();
        let out = run(&mut q, 5, 1, 5);
        assert!(out.iter().all(|r| *r == MappedInputs::NEUTRAL));
        assert_eq!(q.step(5, rec(6)), rec(1));
    }

    #[test]
    fn change_up_refills() {
        let mut q = DelayQueue::new();
        run(&mut q, 2, 1, 10);
        // Raise 2 -> 5: the next 3 polls are neutral (the queue refills), then delayed input resumes.
        let out = run(&mut q, 5, 11, 8);
        assert!(out[..3].iter().all(|r| *r == MappedInputs::NEUTRAL), "{out:?}");
        // Nothing was lost: the 2 queued polls (9 and 10) come out first.
        assert_eq!(out[3], rec(9));
        assert_eq!(out[4], rec(10));
        assert_eq!(out[5], rec(11));
    }

    #[test]
    fn change_down_drops_oldest() {
        let mut q = DelayQueue::new();
        run(&mut q, 5, 1, 20);
        // Lower 5 -> 2: this poll still plays the oldest record (poll 16), the rest of the extra
        // records are discarded, and the output then runs 3 polls ahead of where it was (17..19 are
        // never played).
        let first = q.step(2, rec(21));
        assert_eq!(first, rec(16));
        assert_eq!(q.len(), 2);
        assert_eq!(q.step(2, rec(22)), rec(20));
        assert_eq!(q.step(2, rec(23)), rec(21));
    }

    #[test]
    fn delay_zero_empties_the_queue_and_passes_through() {
        let mut q = DelayQueue::new();
        run(&mut q, 4, 1, 10);
        assert_eq!(q.step(0, rec(11)), rec(7));
        assert!(q.is_empty());
        assert_eq!(q.step(0, rec(12)), rec(12));
    }

    #[test]
    fn clear_on_input_owner_change() {
        let mut q = DelayQueue::new();
        run(&mut q, 3, 1, 10);
        for owner in [Owners::MENU, Owners::STOP_CHORD, Owners::NATIVE_UI_BUSY, Owners::REPLAY_EDITOR, Owners::VANILLA_PAUSE] {
            let mut q = q;
            assert!(q.len() == 3);
            assert_eq!(q.poll(3, owner, rec(11)), MappedInputs::NEUTRAL);
            assert!(q.is_empty(), "queue emptied");
            // Nothing queued before the owner took over comes out after it.
            let after = run(&mut q, 3, 12, 4);
            assert!(after[..3].iter().all(|r| *r == MappedInputs::NEUTRAL));
            assert_eq!(after[3], rec(12));
        }
        assert!(Owners::NONE.with(Owners::MENU).contains(Owners::MENU));
        assert!(!Owners::NONE.any());
        assert_eq!(q.poll(3, Owners::NONE, rec(11)), rec(8));
        // Session clear, a save-state load and a Training reset (decision B8) all empty it.
        for why in [Clear::Session, Clear::SaveStateLoad, Clear::TrainingReset] {
            let mut q = q;
            q.clear(why);
            assert!(q.is_empty());
            assert_eq!(q.step(3, rec(50)), MappedInputs::NEUTRAL);
        }
    }

    #[test]
    fn ordering_with_chords_display_recorder() {
        let at = |s: PollStage| POLL_ORDER.iter().position(|p| *p == s).unwrap();
        assert!(at(PollStage::MenuInput) < at(PollStage::InputDelay));
        assert!(at(PollStage::ChordDetection) < at(PollStage::InputDelay));
        assert!(at(PollStage::InputDelay) < at(PollStage::InputDisplay));
        assert!(at(PollStage::InputDelay) < at(PollStage::Recorder));
        // Only P1 while Training is live.
        assert!(applies(0, true));
        assert!(!applies(1, true));
        assert!(!applies(0, false));
    }

    #[test]
    fn ring_allocates_nothing() {
        let mut q = DelayQueue::new();
        COUNTING.with(|c| c.set(true));
        let before = ALLOCATIONS.with(|n| n.get());
        let mut sink = 0u32;
        for n in 0..10_000u32 {
            let delay = (n / 300 % 31) as u8;
            let owners = if n % 997 == 0 { Owners::MENU } else { Owners::NONE };
            sink = sink.wrapping_add(q.poll(delay, owners, rec(n)).buttons);
            if n % 1500 == 0 {
                q.clear(Clear::SaveStateLoad);
            }
        }
        let after = ALLOCATIONS.with(|n| n.get());
        COUNTING.with(|c| c.set(false));
        assert_eq!(after - before, 0, "sink {sink}");
    }
}
