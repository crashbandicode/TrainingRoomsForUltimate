//! Library-test-only regression for allocations observed on s6-09 planning ticks.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}
struct CountAlloc;
fn allocation() {
    let _ = TRACK.try_with(|track| {
        if track.get() { let _ = COUNT.try_with(|count| count.set(count.get() + 1)); }
    });
}
unsafe impl GlobalAlloc for CountAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        allocation();
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        allocation();
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: CountAlloc = CountAlloc;

#[test]
fn activation_history_tracks_replacement_without_growing() {
    let mut history = crate::model::body_activation::History::<(usize, u32), 12>::new();
    COUNT.with(|count| count.set(0));
    TRACK.with(|track| track.set(true));
    for generation in 1..1000 {
        let live = std::array::from_fn(|slot| (generation * 16 + slot, slot as u32));
        history.observe(live, live[generation % live.len()]);
        std::hint::black_box(&history);
    }
    TRACK.with(|track| track.set(false));
    assert_eq!(COUNT.with(Cell::get), 0);
    assert!(history.contains(&(999 * 16 + 3, 3)));
    assert!(!history.contains(&(998 * 16 + 2, 2)));
}

#[test]
fn first_and_repeated_hat_and_group_plans_allocate_nothing() {
    use crate::engine::preload::cycle_preload_policy::merge_player_hats_first;
    use crate::engine::swap::body_bank::*;
    let window = Members::new(&[0, 1, 2]).unwrap();
    // Duplicate forms, a second Mii and more ordinary candidates than native
    // output cells exercise uncapped input traversal as well as whole groups.
    let lane = [0x24, 0x25, 0x5a, 0x5b, 0x48, 0x49, 3, 4, 5, 7, 8, 9, 10, 11, 12, 13];
    let budget = GroupBudget { ready_fighters: 20, hats_wanted: 2, pool_free: 20, special: SpecialPolicy::MII };
    let hats = [[Some(10), None, Some(11), None, None], [Some(20), None, None, None, None],
        [Some(30), None, None, None, None], [Some(40), Some(41), None, None, None]];
    COUNT.with(|count| count.set(0));
    TRACK.with(|track| track.set(true));
    let mut last = [[None; MAX_EXTRA_GROUPS]; 2];
    let mut hat_plan = [None; 5];
    for _ in 0..32 {
        hat_plan = std::hint::black_box(merge_player_hats_first(hats, 5));
        last = std::hint::black_box(plan_extra_groups([Some(window); 2], [&[]; 2], [&lane; 2], [None; 2], budget));
        std::hint::black_box(count_extra_groups_iter(last[0].iter().flatten().copied().map(|group| (group, GroupState::Planned))));
    }
    TRACK.with(|track| track.set(false));
    assert_eq!(COUNT.with(Cell::get), 0);
    assert_eq!(hat_plan, [Some(10), Some(20), Some(30), Some(11), Some(40)]);
    for groups in last {
        assert_eq!(groups, [Members::native_group(0x24), Members::native_group(0x5a), Members::mii_group(0x48)]);
    }
}
