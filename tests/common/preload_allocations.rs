//! Library-test-only regression for allocations observed on s6-09 planning ticks.
use crate::engine::swap::mii_cycle::compact_regressions::AllocationCount;

fn rc_freeze_removes_native_count_mutators_and_fixed_seat_authority() {
    let roster = include_str!("../../src/runtime/roster/character_roster.rs");
    let sources = include_str!("../../src/runtime/roster/character_sources.rs");
    let hats = include_str!("../../src/runtime/roster/kirby_hats.rs");
    let seams = include_str!("../../src/model/seams/native_seams/mod.rs");
    for absent in ["training_count_readiness", "count_diagnostics", "publish_promoted_presentation"] {
        assert!(!roster.contains(absent), "RC must not register {absent}");
    }
    for absent in ["reserve_count_promotion", "cpu_seats_known", "fixed_source_changed",
        "active_reload", "cutover_items", "begin_promoted_reload"] {
        assert!(!sources.contains(absent), "RC retains F9 path {absent}");
    }
    assert!(!seams.contains("rows_training_count"));
    assert!(!hats.contains("hat_source_cutover"));
    // Removing fixed-seat publication must not strand the ordinary request walk.
    assert!(sources.contains("pending_request_for(state,Some(slot),menu_open)"));
    assert!(sources.contains("state.book.can_replace(index) == Ok(true)"));
    let owned = sources.find("if service_owned(ctx, entry, slot)").unwrap();
    let new_admission = sources[owned..].find("let Ok(backend) = qualified_backend()").unwrap();
    assert!(new_admission > 0, "owned completion must remain ahead of new admission");
}

fn rc_native_mii_scope_keeps_one_cpu_and_waiting_role_without_promoted_owners() {
    use crate::engine::mii::backend::Layout;
    for (quantity, expected) in [(None, false), (Some(0), false), (Some(1), true),
        (Some(2), true), (Some(3), true), (Some(u32::MAX), false)] {
        assert_eq!(Layout::Training.quantity(quantity), expected);
    }
    assert_eq!(Layout::Training.logical(0), Some(0));
    assert_eq!(Layout::Training.logical(1), Some(1));
    for owner in [2, 3, 4, 7] { assert_eq!(Layout::Training.logical(owner), None); }
    assert_eq!(Layout::Ldn.logical(4), Some(1));
    assert!(Layout::Ldn.quantity(None));
    assert!(crate::engine::mii::ordinary_cpu::supported_kind(0x4a));
    let backend = include_str!("../../src/runtime/mii/mii_backend.rs");
    assert!(backend.contains("if !layout.quantity(super::native_source::training_cpu_quantity())"));
    let lifecycle = include_str!("../../src/runtime/roster/character_lifecycle.rs");
    assert!(lifecycle.contains("if entry_id > 1 || !ordinary_training()"));
}

fn rc_exact_mii_uses_native_helpers_and_ordinary_created_capacity_stays_schedulable() {
    use crate::engine::preload::cycle_preload_policy as policy;
    let targets = [Some(0x00u32), Some(0x4a), Some(0x02)];
    let exact = |kind| policy::exact_mii(kind, false);
    let measured = AllocationCount::start();
    for _ in 0..128 {
        assert!(!policy::created_mii_sources());
        // The native pair still prioritizes saved Gunner; an ordinary extra cannot.
        let native = policy::schedulable(targets, true, exact);
        let created = policy::schedulable(targets, policy::created_mii_sources(), exact);
        assert_eq!(native, [Some(0x4a), Some(0x00), Some(0x02)]);
        assert_eq!(created, [Some(0x00), Some(0x02), None]);
        assert_eq!(policy::missing(native, [None; 8], 2, true, false), Some(0x4a));
        assert_eq!(policy::missing(native, [None; 8], 3, false, false), None);
        assert_eq!(policy::missing(created, [None; 8], 7, true, false), Some(0x00));
        assert_eq!(policy::missing(created, [None; 8], 7, true, true), None);
        assert_eq!(policy::missing(created, [None; 8], 8, true, false), None);
    }
    assert_eq!(measured.finish(), 0);
}

fn rc_portraits_can_reuse_native_pair_but_never_foreign_bound_row() {
    use crate::engine::mii_root::mii_hud_portrait_policy as portrait;
    let mut ownership = portrait::Ownership::new();
    for training in [true, false] {
        for owner in 0..2 {
            for helper in [2, 3] {
                assert!(portrait::slot_may_install(ownership.source_exclusions(owner, training), helper));
            }
        }
    }
    ownership.bound[0] = Some(portrait::Binding {
        request: portrait::Request { row: 2, sub: 7 }, descriptor: 0x1000,
    });
    assert!(!portrait::slot_may_install(ownership.source_exclusions(1, true), 2));
    assert!(portrait::slot_may_install(ownership.source_exclusions(1, true), 3));
    assert!(portrait::slot_may_install(ownership.source_exclusions(0, true), 2));
    let runtime = include_str!("../../src/runtime/mii/mii_hud_portrait.rs");
    assert!(!runtime.contains("training_renderer_row"));
    assert!(runtime.contains("!l.helpers().contains(&h)"));
}

fn kirby_shortcut_selection_preserves_order_without_allocating() {
    use crate::engine::compat::menu::BASE_MENU;
    use crate::engine::swap::roster_grid::Role;
    let menu = BASE_MENU;
    for role in [Role::Player, Role::Cpu] {
        let control = AllocationCount::start();
        let old = std::hint::black_box(menu.ordered_kirby_hat_kinds(role));
        assert!(control.finish() > 0, "original shortcut allocated its choice Vec");
        let mut old_cycle = crate::engine::kirby::kirby_hat_cycle::HatCycle::new();
        let mut fixed_cycle = old_cycle;
        let mut equipped = None;
        let measured = AllocationCount::start();
        for _ in 0..128 {
            let choices = std::hint::black_box(menu.ordered_kirby_hat_kinds_fixed(role));
            assert_eq!(&*choices, old.as_slice());
            let target = fixed_cycle.target(&choices, equipped);
            assert_eq!(target, old_cycle.target(&old, equipped));
            if let Some(kind) = target {
                old_cycle.commit(kind);
                fixed_cycle.commit(kind);
            }
            equipped = target;
        }
        assert_eq!(measured.finish(), 0);
    }
}

fn roster_input_membership_preserves_order_without_first_or_repeat_allocations() {
    use crate::engine::mii::inventory::MiiClass;
    use crate::engine::swap::{ordered_roster::OrderedFighterRoster, trio::TrioFighter};
    let fighter = |kind| TrioFighter::from_kind(kind).unwrap();
    let ordinary = OrderedFighterRoster::from_fighters(&[fighter(7), fighter(0), fighter(2)]).unwrap();
    let mixed = OrderedFighterRoster::from_fighters(&[fighter(7), fighter(0x49), fighter(0), fighter(0x48)]).unwrap();
    for roster in [ordinary, mixed, OrderedFighterRoster::all()] {
        // Positive control measures the original hook expression, including its
        // temporary vector. Expected order is retained outside the measurement.
        let control = AllocationCount::start();
        let original = std::hint::black_box(roster.fighters());
        assert!(control.finish() > 0);
        let expected = original.iter().any(|f| (0x48..=0x4a).contains(&f.kind()));
        let measured = AllocationCount::start();
        for _ in 0..128 {
            let fixed = std::hint::black_box(roster.fighters_excluding_fixed(None));
            assert!(fixed.iter().eq(original.iter()));
            assert_eq!(fixed.iter().any(|f| MiiClass::from_kind(f.kind()).is_some()), expected);
        }
        assert_eq!(measured.finish(), 0);
    }
}

#[path = "../../src/runtime/roster/observe.rs"]
mod observed_map;

// Native adapter methods are Switch-only; exercise its value projection here.
#[allow(dead_code)]
#[path = "../../src/runtime/mii/mii_source_provenance.rs"]
mod mii_source_provenance;

fn created_destination_first_and_repeated_capture_allocate_nothing() {
    use crate::engine::mii::{ordinary_cpu::CreatedDestination, ordinary_player::Owner,
        backend::Layout, capacity::{Extras, Roots, ExtraFacts}};
    let owners = core::array::from_fn(|id| ([0,1,2,3,7].contains(&id)).then_some(Owner {
        entry:0x1000+id*0x100, row:0x2000+id*0x100, information:0x3000+id*0x100,
        object:0x4000+id*0x100, index:0, selector:id as u8, alias:0 }));
    let kinds = core::array::from_fn(|id| owners[id].map(|_|9));
    let extras = Extras::registered([Some(Roots {entry:0x1700,resource:0x7000,
        alias:0x4700,selection:2,descriptor_selector:0}),None,None]).unwrap();
    let facts = ExtraFacts {entry_owner:7,row_owner:7,descriptor_selector:0,
        entry_index:0,information_index:0,object_index:0,entry_kind:9,object_kind:9,
        resource:0x7000,alias_root:0x4700,selection:2};
    let measured=AllocationCount::start();
    for _ in 0..128 {
        let destination=CreatedDestination::capture(Layout::Training,extras,7,&owners,&kinds,facts,8).unwrap();
        assert_eq!(destination.record(),7);
        assert_eq!(destination.protected(),[Some(0),Some(1),Some(2),Some(3),None,None,None,None]);
    }
    assert_eq!(measured.finish(),0);
}

fn retained_source_promotion_and_readoption_allocate_nothing_on_first_or_repeated_use() {
    use crate::engine::swap::source_lifetime::{SourceBook, SourceKey, NativeEntry, Holder};
    std::thread::spawn(|| {
        let measurement = AllocationCount::start();
        let mut book = SourceBook::<2, 4>::new();
        let first = book.register_ready(0, book.epoch(), NativeEntry(17),
            SourceKey { kind: 7, costume: 2 }).unwrap();
        let second = book.register_ready(1, book.epoch(), NativeEntry(18),
            SourceKey { kind: 9, costume: 0 }).unwrap();
        let holder = Holder { category: 1, id: 42 };
        let lease = book.acquire(first, holder).unwrap();
        for _ in 0..128 {
            let seats = [Some(first.source()), Some(second.source())];
            let reserved = book.reserve_promotions(seats).unwrap();
            assert_eq!(book.reserve_promotions(seats), Ok(reserved));
            let [Some(a), Some(b)] = reserved else { panic!("both seats reserved"); };
            assert_eq!(book.published_promotion(a), Ok(None));
            let first_publication = book.publish_promotion(a).unwrap();
            let second_publication = book.publish_promotion(b).unwrap();
            assert_ne!(first_publication, second_publication);
            assert_eq!(book.publish_promotion(a), Ok(first_publication));
            assert_eq!(book.published_promotion(b), Ok(Some(second_publication)));
            assert!(book.validate_lease(lease, holder));
            assert!(book.acquire(first, holder).is_err());
            assert_eq!(book.readopt(b), Ok(second));
            assert_eq!(book.readopt(a), Ok(first));
        }
        assert!(book.release(lease));
        let allocations = measurement.finish();
        assert_eq!(allocations, 0);
    }).join().unwrap();
}

fn const_owner_lock_first_use_and_refusal_do_not_allocate() {
    // Exercise the exact public primitive selected by the native owner states.
    // Host futex contention has a different wait backend; native SVC proof is
    // separate. No setup lock or warm-up before the measured first acquisition.
    static OWNER: hos_lock::Mutex<Option<[u64; 4]>> = hos_lock::Mutex::new(None);
    std::thread::spawn(|| {
        let measurement = AllocationCount::start();
        let mut owner = OWNER.lock();
        let first = *owner;
        *owner = Some([17, 23, 42, 99]);
        let refused = OWNER.try_lock().is_none();
        drop(owner);
        let settled = *OWNER.try_lock().unwrap();
        for _ in 0..128 { assert_eq!(*OWNER.lock(), settled); }
        let allocations = measurement.finish();
        assert_eq!(allocations, 0);
        assert_eq!(first, None);
        assert_eq!(settled, Some([17, 23, 42, 99]));
        assert!(refused);
    }).join().unwrap();
}

fn native_release_recipes_use_key_mask_bound_without_allocations() {
    use crate::engine::swap::swap_model::ledger::*;
    for kind in (0..128).chain(std::iter::once(NO_KIND)) {
        for r48 in 0..3 {
            for (r7d, r7f) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let flags = RFlags { r48, r7d, r7f };
                let mut ledger = Ledger::new(flags);
                let batch = ledger.new_batch();
                let (token, _) = ledger.acquire(kind, Holder::Prefetch(GroupId(0)),
                    ReleaseRecipe::OwnedPerKind, batch, &NoConsumers, Role::Player).unwrap();
                let expected: Vec<_> = orphan_keys(kind, flags).iter(kind)
                    .map(NativeCall::ReleaseKey).chain(std::iter::once(NativeCall::ReleaseKind { kind })).collect();
                let measurement = AllocationCount::start();
                let calls = ledger.release_fixed(token, &NoConsumers, Role::Player).unwrap();
                let allocations = measurement.finish();
                assert_eq!(allocations, 0);
                assert_eq!(&*calls, expected.as_slice());
                assert!(ledger.tokens().is_empty());
            }
        }
    }
}

fn native_map_walk_preserves_bounds_and_uses_no_heap() {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Node { left: usize, right: usize, parent: usize, black: u64, key: u64, refs: u32, handle: u32 }
    let mut nodes = [Node { left: 0, right: 0, parent: 0, black: 0, key: 0, refs: 1, handle: 0 }; 64];
    let base = nodes.as_mut_ptr() as usize;
    for (i, node) in nodes.iter_mut().enumerate() {
        node.key = i as u64;
        if i != 0 { node.left = base + (i - 1) * std::mem::size_of::<Node>(); }
    }
    let mut owner = [0usize; 16];
    owner[2] = base + 63 * std::mem::size_of::<Node>();
    owner[3] = 64;
    let r = owner.as_mut_ptr() as usize;
    let measurement = AllocationCount::start();
    let walked = unsafe { observed_map::walk_map(r) }.expect("64 native nodes");
    let flags = unsafe { observed_map::owner_flags(r) };
    let allocations = measurement.finish();
    assert_eq!(allocations, 0);
    assert_eq!(flags, crate::engine::swap::swap_model::ledger::RFlags::TRAINING);
    assert_eq!(walked.len(), 64);
    assert!(walked.iter().enumerate().all(|(i, node)| node.key == i as u32 && node.refs == 1));
    unsafe { ((r + 0x18) as *mut u64).write(65); }
    assert!(unsafe { observed_map::walk_map(r) }.is_none());
    unsafe {
        ((r + 0x18) as *mut u64).write(64);
        (base as *mut Node).as_mut().unwrap().left = base;
    }
    assert!(unsafe { observed_map::walk_map(r) }.is_none(), "cycle exhausts the existing walk bound");
}

fn reconciliation_streams_counts_and_preserves_duplicate_and_missing_order() {
    use crate::engine::swap::swap_model::ledger::*;
    use std::collections::BTreeMap;
    let mut ledger = Ledger::new(RFlags::TRAINING);
    let batch = ledger.new_batch();
    let mut predicted = BTreeMap::<u32, u32>::new();
    for kind in [0, 7, 0x48, 7, 0x75] {
        let keys = registration_set(kind, RFlags::TRAINING);
        for key in keys.iter(kind) { *predicted.entry(key.value()).or_default() += 1; }
        let token = ledger.record_native(NativeRequest { kind, keys, nro: true }, batch);
        let _ = std::hint::black_box(token);
    }
    for seed in 0..64u32 {
        let actual = [(7 * KEY_STRIDE, seed % 4), (0, seed % 2), (8000, seed),
            (7 * KEY_STRIDE, (seed + 1) % 4), (0x48 * KEY_STRIDE, 0)];
        let map: BTreeMap<_, _> = actual.into_iter().collect();
        let mut expected = Reconciliation::default();
        for (key, refs) in &map {
            match predicted.get(key) {
                Some(p) if p == refs => expected.matched += 1,
                Some(p) if p < refs => expected.residual.push((*key, refs - p)),
                Some(p) => expected.missing.push((*key, p - refs)),
                None => expected.residual.push((*key, *refs)),
            }
        }
        for (key, refs) in &predicted {
            if !map.contains_key(key) { expected.missing.push((*key, *refs)); }
        }
        assert_eq!(ledger.reconcile(&actual), expected);
        let measurement = AllocationCount::start();
        assert!(ledger.predicted_iter().eq(predicted.iter().map(|(k, v)| (*k, *v))));
        assert!(ledger.residual_iter(actual.into_iter()).eq(expected.residual.iter().copied()));
        assert!(ledger.missing_iter(actual.into_iter()).eq(expected.missing.iter().copied()));
        assert_eq!(ledger.predicted_map_nodes(), predicted.len());
        let allocations = measurement.finish();
        assert_eq!(allocations, 0);
    }
}

fn native_request_receipts_and_diagnostic_iterators_allocate_nothing() {
    use crate::engine::compat::fixed::DebugIter;
    use crate::engine::swap::swap_model::ledger::{native_window_requests_fixed, RFlags};
    use std::fmt::Write;
    struct Sink(usize);
    impl std::fmt::Write for Sink {
        fn write_str(&mut self, text: &str) -> std::fmt::Result { self.0 += text.len(); Ok(()) }
    }
    let mut sink = Sink(0);
    let measurement = AllocationCount::start();
    let mut formatted = true;
    for primary in 0..=0x75 {
        let requests = native_window_requests_fixed([primary, 0x25, 0x26], RFlags::TRAINING);
        formatted &= write!(sink, "{:x?}", DebugIter(requests.iter().map(|r| r.kind))).is_ok();
        std::hint::black_box(requests);
    }
    let allocations = measurement.finish();
    assert!(formatted && sink.0 > 0);
    assert_eq!(allocations, 0);
    assert_eq!(native_window_requests_fixed([0x24, 0x25, 0x26], RFlags::TRAINING).iter()
        .map(|r| r.kind).collect::<Vec<_>>(), [0x24, 0x72, 0x25, 0x26]);
    assert_eq!(format!("{:x?}", DebugIter([10, 15].into_iter())), "[a, f]");
}

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
        let measurement = AllocationCount::start();
        for step in borrowed.steps() { std::hint::black_box(step); }
        for key in borrowed.keys() { std::hint::black_box(key); }
        for skipped in borrowed.skipped() { std::hint::black_box(skipped); }
        std::hint::black_box(borrowed.kirby_counts());
        let allocations = measurement.finish();
        assert_eq!(allocations, 0, "mask {mask}");
    }
}

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
    let measurement = AllocationCount::start();
    let mut plan = attached_plan(0, &cells, &contexts, &consumers);
    for _ in 0..32 { plan = std::hint::black_box(attached_plan(0, &cells, &contexts, &consumers)); }
    let allocations = measurement.finish();
    assert_eq!(allocations, 0);
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
    let measurement = AllocationCount::start();
    let mut changed = false;
    for _ in 0..32 {
        let plan = packages::sync_iter(held.iter().copied(), wanted());
        changed |= !plan.acquire.is_empty() || !plan.release.is_empty();
        for key in packages::referenced_keys(held.iter().copied()) { std::hint::black_box(key); }
        std::hint::black_box(plan);
    }
    let allocations = measurement.finish();
    assert!(!changed);
    assert_eq!(allocations, 0);
    // Removing one duplicate must release exactly its occurrences, keeping the
    // earlier identical holdings and descending removal order.
    let one = packages::acquisitions(&roster, 0, sets[0], 2, 1);
    let duplicates: Vec<_> = one.iter().chain(&one).copied().collect();
    let plan = packages::sync(&duplicates, &one);
    assert!(plan.acquire.is_empty());
    assert_eq!(plan.release, (one.len()..duplicates.len()).rev().collect::<Vec<_>>());
}

fn changed_copy_holdings_preserve_reference_order_without_allocations() {
    use crate::engine::swap::swap_model::{copy_packages as packages, copy_registry::*};
    let mut held = packages::empty_holdings();
    for generation in 0..128 {
        let mut wanted = packages::empty_holdings();
        let count = if generation % 7 == 0 { 0 } else { packages::MAX_HOLDINGS };
        for i in 0..count {
            let mut roster = [Record::DEFAULT; ENTRIES];
            roster[0] = Record { entry: 0, kinds: [KIRBY, 0, 1], costume: (generation % 8) as u32 };
            roster[1] = Record { entry: 1, kinds: [0x31, NO_KIND, NO_KIND], costume: 3 };
            // Duplicate occurrences and surviving old values exercise matching,
            // while repeated full replacement checks that history cannot grow.
            assert!(wanted.push(packages::Acquisition { roster, set: [KIRBY, 0, 1],
                member: ((i + generation) % 3) as u32, costume: 2 }));
        }
        let expected = packages::sync(&held, &wanted);
        let measurement = AllocationCount::start();
        let plan = packages::sync_bounded(&held, &wanted);
        let mut released = packages::empty_holdings();
        for index in plan.release.iter() { assert!(released.push(held.remove(*index))); }
        for acquisition in plan.acquire.iter() { assert!(held.push(*acquisition)); }
        for acquisition in released.iter().chain(plan.acquire.iter()) {
            std::hint::black_box(packages::roster_bytes(&acquisition.roster));
        }
        let allocations = measurement.finish();
        assert_eq!(allocations, 0);
        assert_eq!(&*plan.acquire, expected.acquire.as_slice());
        assert_eq!(&*plan.release, expected.release.as_slice());
        let settled = packages::sync(&held, &wanted);
        assert!(settled.acquire.is_empty() && settled.release.is_empty());
        assert_eq!(held.len(), wanted.len());
    }
}

fn activation_history_tracks_replacement_without_growing() {
    let mut history = crate::model::body_activation::History::<(usize, u32), 12>::new();
    let measurement = AllocationCount::start();
    for generation in 1..1000 {
        let live = std::array::from_fn(|slot| (generation * 16 + slot, slot as u32));
        history.observe(live, live[generation % live.len()]);
        std::hint::black_box(&history);
    }
    let allocations = measurement.finish();
    assert_eq!(allocations, 0);
    assert!(history.contains(&(999 * 16 + 3, 3)));
    assert!(!history.contains(&(998 * 16 + 2, 2)));
}

fn first_and_repeated_hat_and_group_plans_allocate_nothing() {
    use crate::engine::preload::cycle_preload_policy::merge_player_hats_first;
    use crate::engine::swap::body_bank::*;
    let window = Members::new(&[0, 1, 2]).unwrap();
    // Duplicate forms, a second Mii and more ordinary candidates than native
    // output cells exercise uncapped input traversal as well as whole groups.
    let lane = [0x24, 0x25, 0x5a, 0x5b, 0x48, 0x49, 3, 4, 5, 7, 8, 9, 10, 11, 12, 13];
    let budget = GroupBudget { ready_fighters: 20, pool_free: 20, special: SpecialPolicy::MII };
    let hats = [[Some(10), None, Some(11), None, None], [Some(20), None, None, None, None],
        [Some(30), None, None, None, None], [Some(40), Some(41), None, None, None]];
    let measurement = AllocationCount::start();
    let mut last = [[None; MAX_EXTRA_GROUPS]; 2];
    let mut hat_plan = [None; 5];
    for _ in 0..32 {
        hat_plan = std::hint::black_box(merge_player_hats_first(hats, 5));
        last = std::hint::black_box(plan_extra_groups([Some(window); 2], [&[]; 2], [&lane; 2], [None; 2], budget));
        std::hint::black_box(count_extra_groups_iter(last[0].iter().flatten().copied().map(|group| (group, GroupState::Planned))));
    }
    let allocations = measurement.finish();
    assert_eq!(allocations, 0);
    assert_eq!(hat_plan, [Some(10), Some(20), Some(30), Some(11), Some(40)]);
    for groups in last {
        assert_eq!(groups, [Members::native_group(0x24), Members::native_group(0x5a), Members::mii_group(0x48)]);
    }
}

fn status_raise_and_delivery_formatting_use_no_heap_on_first_or_repeated_use() {
    use crate::engine::preload::cycle_preload_policy::{CacheStatus, BuiltFighters, Readiness, Progress};
    use std::fmt::Write;
    let assets=Readiness {progress:Progress::Loading,ready:0,planned:5,configured:5,limit:5,failed:0};
    let waiting=CacheStatus::new(assets,BuiltFighters {planned:2,swap:2,..BuiltFighters::default()},false);
    let measured=AllocationCount::start();
    for _ in 0..128 {
        // ui-16 observed +3 on raise and +1 on delivery: these are the actual
        // three status/log/notice formatting uses and the Option delivery log.
        let mut changed=crate::model::storage::FixedPath::<96>::new();
        write!(changed,"{:?}",waiting.text()).unwrap();
        let mut raised=crate::model::storage::FixedPath::<96>::new();
        write!(raised,"{:?}",waiting.text()).unwrap();
        let mut notice=crate::model::storage::FixedPath::<96>::new();
        write!(notice,"{}",waiting.text()).unwrap();
        let mut delivered=crate::model::storage::FixedPath::<96>::new();
        write!(delivered,"{:?}",Some(waiting).map(|s|s.text())).unwrap();
        assert_eq!(changed.as_str(),"\"Swap the CPU once to load\"");
        assert_eq!(raised.as_str(),changed.as_str());
        assert_eq!(notice.as_str(),"Swap the CPU once to load");
        assert_eq!(delivered.as_str(),"Some(\"Swap the CPU once to load\")");
    }
    assert_eq!(measured.finish(),0);
    // Largest numeric values plus the longest waiting suffix fit the existing
    // notification_args buffer; do not silently truncate this status text.
    for pending in [0,1,255] { for ready in [0,1,127,255] { for failed in [0,1,127,255] {
        let status=CacheStatus::new(assets,BuiltFighters {planned:255,ready,failed,pending,swap:3,
            ..BuiltFighters::default()},false);
        let mut notice=crate::model::storage::FixedPath::<96>::new();
        write!(notice,"{}",status.text()).unwrap();
    } } }
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("rc_freeze_removes_native_count_mutators_and_fixed_seat_authority", rc_freeze_removes_native_count_mutators_and_fixed_seat_authority as fn()),
        ("rc_native_mii_scope_keeps_one_cpu_and_waiting_role_without_promoted_owners", rc_native_mii_scope_keeps_one_cpu_and_waiting_role_without_promoted_owners as fn()),
        ("rc_exact_mii_uses_native_helpers_and_ordinary_created_capacity_stays_schedulable", rc_exact_mii_uses_native_helpers_and_ordinary_created_capacity_stays_schedulable as fn()),
        ("rc_portraits_can_reuse_native_pair_but_never_foreign_bound_row", rc_portraits_can_reuse_native_pair_but_never_foreign_bound_row as fn()),
        ("kirby_shortcut_selection_preserves_order_without_allocating", kirby_shortcut_selection_preserves_order_without_allocating as fn()),
        ("roster_input_membership_preserves_order_without_first_or_repeat_allocations", roster_input_membership_preserves_order_without_first_or_repeat_allocations as fn()),
        ("created_destination_first_and_repeated_capture_allocate_nothing", created_destination_first_and_repeated_capture_allocate_nothing as fn()),
        ("retained_source_promotion_and_readoption_allocate_nothing_on_first_or_repeated_use", retained_source_promotion_and_readoption_allocate_nothing_on_first_or_repeated_use as fn()),
        ("const_owner_lock_first_use_and_refusal_do_not_allocate", const_owner_lock_first_use_and_refusal_do_not_allocate as fn()),
        ("native_release_recipes_use_key_mask_bound_without_allocations", native_release_recipes_use_key_mask_bound_without_allocations as fn()),
        ("native_map_walk_preserves_bounds_and_uses_no_heap", native_map_walk_preserves_bounds_and_uses_no_heap as fn()),
        ("reconciliation_streams_counts_and_preserves_duplicate_and_missing_order", reconciliation_streams_counts_and_preserves_duplicate_and_missing_order as fn()),
        ("native_request_receipts_and_diagnostic_iterators_allocate_nothing", native_request_receipts_and_diagnostic_iterators_allocate_nothing as fn()),
        ("detached_teardown_streams_all_phases_and_refusals_without_allocations", detached_teardown_streams_all_phases_and_refusals_without_allocations as fn()),
        ("attached_teardown_keeps_alias_validation_and_route_order_without_allocations", attached_teardown_keeps_alias_validation_and_route_order_without_allocations as fn()),
        ("copy_package_matching_streams_duplicate_sets_without_tick_allocations", copy_package_matching_streams_duplicate_sets_without_tick_allocations as fn()),
        ("changed_copy_holdings_preserve_reference_order_without_allocations", changed_copy_holdings_preserve_reference_order_without_allocations as fn()),
        ("activation_history_tracks_replacement_without_growing", activation_history_tracks_replacement_without_growing as fn()),
        ("first_and_repeated_hat_and_group_plans_allocate_nothing", first_and_repeated_hat_and_group_plans_allocate_nothing as fn()),
        ("status_raise_and_delivery_formatting_use_no_heap_on_first_or_repeated_use", status_raise_and_delivery_formatting_use_no_heap_on_first_or_repeated_use as fn()),
    ]);
}
