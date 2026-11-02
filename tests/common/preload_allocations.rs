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
fn native_request_receipts_and_diagnostic_iterators_allocate_nothing() {
    use crate::engine::compat::fixed::DebugIter;
    use crate::engine::swap::swap_model::ledger::{native_window_requests_fixed, RFlags};
    use std::fmt::Write;
    struct Sink(usize);
    impl std::fmt::Write for Sink {
        fn write_str(&mut self, text: &str) -> std::fmt::Result { self.0 += text.len(); Ok(()) }
    }
    let mut sink = Sink(0);
    COUNT.with(|count| count.set(0));
    TRACK.with(|track| track.set(true));
    let mut formatted = true;
    for primary in 0..=0x75 {
        let requests = native_window_requests_fixed([primary, 0x25, 0x26], RFlags::TRAINING);
        formatted &= write!(sink, "{:x?}", DebugIter(requests.iter().map(|r| r.kind))).is_ok();
        std::hint::black_box(requests);
    }
    TRACK.with(|track| track.set(false));
    assert!(formatted && sink.0 > 0);
    assert_eq!(COUNT.with(Cell::get), 0);
    assert_eq!(native_window_requests_fixed([0x24, 0x25, 0x26], RFlags::TRAINING).iter()
        .map(|r| r.kind).collect::<Vec<_>>(), [0x24, 0x72, 0x25, 0x26]);
    assert_eq!(format!("{:x?}", DebugIter([10, 15].into_iter())), "[a, f]");
}

#[test]
fn detached_teardown_streams_all_phases_and_refusals_without_allocations() {
    use crate::engine::swap::swap_model::lifecycle::*;
    for mask in 0..64u32 {
        let bodies = [
            BodyRef { id: 10, kind: 6, cell: 0, activated: false, valid: mask & 1 != 0, registered: Some(0) },
            BodyRef { id: 11, kind: 0, cell: 1, activated: true, valid: mask & 2 != 0, registered: Some(6) },
        ];
        let contexts = [ContextRef { ctx: 1, controller: 22, valid: mask & 4 != 0 },
            ContextRef { ctx: 1, controller: 22, valid: true },
            ContextRef { ctx: 2, controller: 33, valid: mask & 8 != 0 }];
        let groups = [DetachedView { owner: 0, bodies: &bodies, contexts: &contexts,
            dispatch: Some(DispatcherRef { arrays: [100, 101], valid: mask & 16 != 0 }),
            records: Some(RecordsRef { records: [200, 201, 202], valid: mask & 32 != 0 }), held_kinds: &[7, 8, 7] },
            DetachedView { owner: 1, bodies: &bodies[1..], contexts: &contexts[1..], dispatch: None,
                records: None, held_kinds: &[8, 9] }];
        let consumers = NativeConsumers { pad_routes: std::array::from_fn(|slot|
            Some(RouteRef { slot: slot as u8, controller: if slot % 2 == 0 { 22 } else { 33 } })),
            nana_owner_plus_8: Some(22) };
        let borrowed = DetachedPlan { groups: &groups, consumers: &consumers };
        let owned: Vec<_> = groups.iter().map(|g| Bundle::Detached(DetachedGroup { owner: g.owner,
            bodies: g.bodies.to_vec(), contexts: g.contexts.to_vec(), dispatch: g.dispatch,
            records: g.records, held_kinds: g.held_kinds.to_vec() })).collect();
        let expected = destroy_plan_many(&owned, &consumers);
        let mut steps: Vec<_> = borrowed.steps().collect();
        steps.push(DestroyStep::ReleaseKeys { kinds: borrowed.keys().collect() });
        assert_eq!(steps, expected.steps, "mask {mask}");
        assert_eq!(borrowed.skipped().collect::<Vec<_>>(), expected.skipped, "mask {mask}");
        assert_eq!(borrowed.kirby_counts(), [expected.kirby_cells, expected.kirby_misplaced, expected.kirby_branch_mismatch]);
        assert_eq!(borrowed.keys().collect::<Vec<_>>(), [7, 8, 9]);
        COUNT.with(|count| count.set(0));
        TRACK.with(|track| track.set(true));
        for step in borrowed.steps() { std::hint::black_box(step); }
        for key in borrowed.keys() { std::hint::black_box(key); }
        for skipped in borrowed.skipped() { std::hint::black_box(skipped); }
        std::hint::black_box(borrowed.kirby_counts());
        TRACK.with(|track| track.set(false));
        assert_eq!(COUNT.with(Cell::get), 0, "mask {mask}");
    }
}

#[test]
fn attached_teardown_keeps_alias_validation_and_route_order_without_allocations() {
    use crate::engine::swap::swap_model::lifecycle::*;
    let contexts = [
        ContextRef { ctx: 1, controller: 11, valid: false },
        ContextRef { ctx: 1, controller: 11, valid: true },
        ContextRef { ctx: 2, controller: 22, valid: true },
        ContextRef { ctx: 0, controller: 33, valid: true },
    ];
    let cells = [
        BodyRef { id: 1, kind: 6, cell: 1, activated: false, valid: true, registered: Some(0) },
        BodyRef { id: 2, kind: 6, cell: 2, activated: true, valid: false, registered: Some(0) },
    ];
    let consumers = NativeConsumers {
        pad_routes: std::array::from_fn(|i| Some(RouteRef { slot: i as u8,
            controller: if i == 0 { 11 } else if i == 1 { 33 } else { 22 } })),
        nana_owner_plus_8: Some(22),
    };
    COUNT.with(|count| count.set(0));
    TRACK.with(|track| track.set(true));
    let mut plan = attached_plan(0, &cells, &contexts, &consumers);
    for _ in 0..32 { plan = std::hint::black_box(attached_plan(0, &cells, &contexts, &consumers)); }
    TRACK.with(|track| track.set(false));
    assert_eq!(COUNT.with(Cell::get), 0);
    assert_eq!(plan.routes.iter().map(|r| r.slot).collect::<Vec<_>>(), (2..16).collect::<Vec<u8>>());
    assert_eq!((plan.kirby_cells, plan.kirby_misplaced, plan.kirby_branch_mismatch), (1, 1, 1));
    let generic = destroy_plan_many(&[Bundle::Attached(AttachedGroup { owner: 0,
        cells: cells.to_vec(), contexts: contexts.to_vec() })], &consumers);
    assert_eq!(generic.steps, plan.routes.iter().map(|r| DestroyStep::ReleasePadRoute {
        slot: r.slot, controller: r.controller }).collect::<Vec<_>>());
    assert!(generic.skipped.is_empty());
    // The model accepts value records independently of native probing. Preserve
    // all sixteen table records plus a distinct separately probed Nana slot.
    let distinct = NativeConsumers { pad_routes: std::array::from_fn(|i|
        Some(RouteRef { slot: (i + PAD_SLOTS) as u8, controller: 22 })), ..consumers };
    let plan = attached_plan(0, &cells, &contexts, &distinct);
    assert_eq!(plan.routes.len(), PAD_SLOTS + 1);
    assert_eq!(plan.routes.last().unwrap().slot, 8);
}

#[test]
fn copy_package_matching_streams_duplicate_sets_without_tick_allocations() {
    use crate::engine::swap::swap_model::{copy_packages as packages, copy_registry::*};
    let mut roster = [Record::DEFAULT; ENTRIES];
    roster[0] = Record { entry: 0, kinds: [KIRBY, 0, 1], costume: 2 };
    roster[1] = Record { entry: 1, kinds: [0x31, NO_KIND, NO_KIND], costume: 3 };
    let sets = [[KIRBY, 0, 1], [KIRBY, 0, 1], [0x48, 0x4b, 0x4c], [NO_KIND; SUBS]];
    let wanted = || sets.iter().flat_map(|set| packages::acquisitions_iter(&roster, 0, *set, 2, 1));
    let held: Vec<_> = wanted().collect();
    assert!(!held.is_empty());
    let expected_keys: Vec<_> = packages::referenced(&held).into_keys().collect();
    assert_eq!(packages::referenced_keys(held.iter().copied()).collect::<Vec<_>>(), expected_keys);
    COUNT.with(|count| count.set(0));
    TRACK.with(|track| track.set(true));
    let mut changed = false;
    for _ in 0..32 {
        let plan = packages::sync_iter(held.iter().copied(), wanted());
        changed |= !plan.acquire.is_empty() || !plan.release.is_empty();
        for key in packages::referenced_keys(held.iter().copied()) { std::hint::black_box(key); }
        std::hint::black_box(plan);
    }
    TRACK.with(|track| track.set(false));
    assert!(!changed);
    assert_eq!(COUNT.with(Cell::get), 0);
    // Removing one duplicate must release exactly its occurrences, keeping the
    // earlier identical holdings and descending removal order.
    let one = packages::acquisitions(&roster, 0, sets[0], 2, 1);
    let duplicates: Vec<_> = one.iter().chain(&one).copied().collect();
    let plan = packages::sync(&duplicates, &one);
    assert!(plan.acquire.is_empty());
    assert_eq!(plan.release, (one.len()..duplicates.len()).rev().collect::<Vec<_>>());
}

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
