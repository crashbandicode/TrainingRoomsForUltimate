//! Review R5 of package 1: the allocation detector (`model::hook_stats`) is proven with stubs. The
//! test crate installs the plugin's own `CountingAllocator` as its global allocator, so the stubs'
//! allocations are counted exactly as the Switch runtime's are.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::hook_stats::{hook_allocations, snapshot, CountingAllocator, HookId, HookScope, Observer, HOOK_COUNT};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// A stub hook that follows the rule: it works on the stack only.
fn well_behaved_hook(id: HookId, n: u32) -> u32 {
    let _scope = HookScope::enter_hook(id);
    let mut ring = [0u32; 8];
    for (i, slot) in ring.iter_mut().enumerate() {
        *slot = n.wrapping_mul(i as u32 + 1);
    }
    ring.iter().fold(0, |a, b| a ^ b)
}

/// A stub hook that breaks the rule on purpose: it allocates inside its scope.
fn allocating_hook(id: HookId, bytes: usize) -> usize {
    let _scope = HookScope::enter_hook(id);
    let v = vec![1u8; bytes];
    std::hint::black_box(&v);
    v.len()
}

/// The counters are process-wide and the harness runs tests on several threads: the tests take turns.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn preview_checkpoint_first_use_restart_and_retirement_allocate_nothing() {
    use model::state::{preview::Checkpoint, record::SavedState};
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let before = hook_allocations();
    {
        let _scope = HookScope::enter_hook(HookId::StateFrame);
        let mut checkpoint = Checkpoint::new();
        for generation in 0..20 {
            let pair = std::array::from_fn(|role| SavedState {
                x: 40.0 * (1.0 - 2.0 * role as f32),
                percent: 15.0,
                ..SavedState::never_saved()
            });
            checkpoint.capture(generation, pair).unwrap();
            for _ in 0..10 { std::hint::black_box(checkpoint.restore(&generation).unwrap()); }
            checkpoint.clear();
        }
    }
    assert_eq!(hook_allocations(), before);
}

#[test]
fn setup_game_transfer_validation_and_atomic_pair_commit_allocate_nothing() {
    use model::state::{machine::{Machine, SituationIds}, record::SavedState,
        settings::StateSettings, setup_transaction::{self, Exchange, Facts, Prepared, Target}};
    use model::cpu::rng::Rng;
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let exchange = Exchange::new();
    let pair = std::array::from_fn(|i| SavedState { fighter_kind: i as i32, lr: 1.0,
        saved_stage: Some(17), situation_kind: 0, ..SavedState::never_saved() });
    let record = setup_transaction::export("Setup", &pair).unwrap();
    let target = Target { session: 9, slot: 3, enabled: true };
    let facts = Facts { session: 9, owned: true, busy: false, enabled: true, stage: 17, kinds: [0, 1],
        miis: [None::<u64>; 2], situations: SituationIds { ground: 0, air: 2, cliff: 5 } };
    let mut machine = Machine::default();
    let settings = StateSettings::default();
    let mut rng = Rng::new(19);
    let before = hook_allocations();
    for iteration in 0..20 {
        // Allocation and final destruction are worker work, outside HookScope.
        let ticket = exchange.submit(Box::new(Prepared::new(&record, [None::<u64>; 2]).unwrap()))
            .unwrap_or_else(|_| panic!("idle exchange"));
        {
            let _scope = HookScope::enter_hook(HookId::StateFrame);
            let (id, prepared) = exchange.take_game().unwrap();
            assert_eq!(id, ticket);
            prepared.validate(target, &facts, true).unwrap();
            assert!(machine.bind_setup(target.slot, prepared.pair(), iteration % 2 == 0, &settings, &mut rng));
            exchange.finish_game(id, prepared, true);
            machine.cancel_all();
        }
        let (id, prepared) = exchange.take_writer().unwrap();
        exchange.finish_writer(id, prepared);
        drop(exchange.poll(ticket).unwrap());
    }
    assert_eq!(hook_allocations(), before);
}

#[test]
fn imported_mii_json_stays_with_writer_when_game_records_are_copied() {
    use model::state::file::SaveFile;
    use model::state::machine::{Machine, Role};
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    // An opaque future field must survive storage even though gameplay never
    // interprets it. Include both roles and a non-default slot.
    let mut writer = SaveFile::default();
    writer.player[3].fighter_kind = 0x48;
    writer.player[3].percent = 47.0;
    writer.player[3].mii = Some(serde_json::json!({"future": ["player", 7]}));
    writer.cpu[3].fighter_kind = 0x4a;
    writer.cpu[3].x = -19.0;
    writer.cpu[3].mii = Some(serde_json::json!({"future": ["cpu", 9]}));
    let bytes = writer.to_pretty();
    let old_before = hook_allocations();
    {
        let _scope = HookScope::enter_hook(HookId::StateFrame);
        std::hint::black_box(writer.player[3].clone());
    }
    assert!(hook_allocations() > old_before, "the imported JSON copy must be observable");
    let mut machine = Machine::new(writer.clone().into_game_records());
    let before = hook_allocations();
    {
        let _scope = HookScope::enter_hook(HookId::StateFrame);
        for _ in 0..20 {
            for role in [Role::Player, Role::Cpu] {
                let record = machine.record(role, 3).clone();
                assert!(record.mii.is_none());
                assert!(machine.capture(role, 3, record));
            }
        }
    }
    assert_eq!(hook_allocations(), before);
    assert_eq!(machine.record(Role::Player, 3).percent, 47.0);
    assert_eq!(machine.record(Role::Cpu, 3).x, -19.0);
    assert_eq!(writer.to_pretty(), bytes);
    assert_eq!(SaveFile::parse(bytes.as_bytes()).unwrap(), writer);
}

#[test]
fn quick_reset_observation_and_temporary_restore_allocate_nothing() {
    use model::state::machine::{FrameIn, Machine, SituationIds, StatusNow};
    use model::state::quick_reset::{Pose, QuickReset};
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let mut quick = QuickReset::new();
    let mut machine = Machine::default();
    let mut rng = model::cpu::rng::Rng::new(8);
    let settings = model::state::settings::StateSettings::default();
    let pose = Pose {
        x: 3.0,
        y: 4.0,
        lr: 1.0,
        percent: 23.0,
        kind: 7,
        situation: 0,
        stage: 8,
    };
    let before = hook_allocations();
    {
        let _scope = HookScope::enter_hook(HookId::StateFrame);
        for _ in 0..20 {
            quick.clear();
            quick.observe([8, 16], 0, true, pose);
            quick.observe([8, 16], 1, true, pose);
            let pair = quick.pair([8, 16]).unwrap();
            assert!(machine.begin_temporary(pair.map(Pose::record)));
            for _ in 0..2 {
                for entry in [0, 1] {
                    std::hint::black_box(machine.frame(
                        &FrameIn {
                            entry,
                            nana: false,
                            popo: false,
                            kind: 7,
                            dead: false,
                            status: StatusNow::Wait,
                            prev_rebirth: false,
                            standby: false,
                            stage_id: 8,
                            situation: SituationIds {
                                ground: 0,
                                air: 2,
                                cliff: 1,
                            },
                            save_chord: false,
                            load_chord: false,
                            kirby: false,
                            pokemon: false,
                            buffable: false,
                        },
                        &settings,
                        &mut rng,
                    ));
                }
            }
            assert!(!machine.is_loading());
        }
    }
    assert_eq!(hook_allocations(), before);
}

#[test]
fn the_detector_reports_a_stub_that_allocates_and_passes_one_that_does_not() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let mut observer = Observer::new();
    // Whatever the other test already did is the baseline; only what follows is measured.
    let _ = observer.observe(snapshot());

    // Hook coverage without allocation: nonzero counts, delta 0.
    let mut sink = 0;
    for n in 0..50 {
        sink ^= well_behaved_hook(HookId::InputCallerAApply, n);
        sink ^= well_behaved_hook(HookId::ResetCheck, n);
    }
    std::hint::black_box(sink);
    let clean = observer.observe(snapshot()).expect("hooks ran, so a line is due");
    assert_eq!(clean.delta, 0, "a stack-only hook must not move the allocation counter: {clean}");
    assert!(clean.covered(HookId::InputCallerAApply) && clean.covered(HookId::ResetCheck));
    assert!(!clean.covered(HookId::HazardRule), "a hook that never ran is not covered");
    assert_eq!(clean.hook_calls, 100);
    assert!(clean.to_string().contains("delta=0") && clean.to_string().contains("input_caller_a_apply:50"));

    // The intentionally allocating stub: the same detector reports a nonzero delta.
    let before = hook_allocations();
    assert_eq!(allocating_hook(HookId::HazardRule, 4096), 4096);
    assert!(hook_allocations() > before, "the stub's allocation is counted");
    let dirty = observer.observe(snapshot()).expect("an allocation is reported even with one hook call");
    assert!(dirty.delta >= 1, "{dirty}");
    assert!(dirty.to_string().contains(&format!("delta={}", dirty.delta)));
    assert!(dirty.covered(HookId::HazardRule));

    // Allocation outside any hook is not counted (the boot thread allocates freely).
    let outside = hook_allocations();
    let v = vec![0u8; 1 << 16];
    std::hint::black_box(&v);
    assert_eq!(hook_allocations(), outside);
    assert_eq!(snapshot().entries.len(), HOOK_COUNT);

    // A second quiet sample reports nothing.
    assert!(observer.observe(snapshot()).is_none());
}

/// The byte meter of the diag heap features (`plugin-heap-meter`, `console-practice-trace`) lives in the same allocator, so those
/// features read this plugin's one global allocator instead of declaring their own. A block's size shows in `live` and `total`
/// while it is held, `live` returns when it is freed, `peak` keeps the high-water mark, and a realloc counts the new size.
///
/// The meter is process-wide by design (it measures the whole NRO), and this binary also runs the model's unit tests (`#[path] mod
/// model` brings ~110 of them) on other harness threads: their frees moved `live` between two readings, so the earlier in-process
/// version failed intermittently ("live holds the block"). The test therefore runs its measurement in a child process of this same
/// binary that runs this one test alone on one thread (`--exact`, `--test-threads=1`): every delta there is the measurement's own,
/// so the checks are exact.
#[test]
fn the_byte_meter_counts_what_the_one_global_allocator_serves() {
    const CHILD: &str = "TRAININGROOMS_HEAP_METER_CHILD";
    if std::env::var_os(CHILD).is_some() {
        measure_the_byte_meter_alone();
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args(["--exact", "the_byte_meter_counts_what_the_one_global_allocator_serves", "--test-threads=1", "--nocapture"])
        .env(CHILD, "1")
        .output()
        .expect("the child test process starts");
    let text = String::from_utf8_lossy(&output.stdout).into_owned() + &String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success() && text.contains("heap_meter exact"), "{text}");
}

/// The child's measurement: the only thread allocating is this one.
fn measure_the_byte_meter_alone() {
    use std::alloc::{GlobalAlloc, Layout};
    let before = model::hook_stats::heap_bytes();
    let first = Layout::from_size_align(1 << 20, 8).unwrap();
    // SAFETY: the layouts are valid and every block is freed with the layout it was allocated or reallocated with.
    unsafe {
        let pointer = ALLOCATOR.alloc(first);
        assert!(!pointer.is_null());
        let held = model::hook_stats::heap_bytes();
        assert_eq!(held.live, before.live + (1 << 20), "live holds the block");
        assert!(held.peak >= held.live);
        assert_eq!((held.total, held.calls), (before.total + (1 << 20), before.calls + 1));
        assert!(held.largest >= 1 << 20);
        let moved = ALLOCATOR.realloc(pointer, first, 1 << 10);
        assert!(!moved.is_null());
        let shrunk = model::hook_stats::heap_bytes();
        assert_eq!(shrunk.live, before.live + (1 << 10), "the old block left `live` at the realloc, the new size is live");
        assert_eq!(shrunk.total, held.total + (1 << 10), "the new size counts as requested");
        assert_eq!(shrunk.peak, held.peak, "peak never falls");
        ALLOCATOR.dealloc(moved, Layout::from_size_align(1 << 10, 8).unwrap());
    }
    let after = model::hook_stats::heap_bytes();
    assert_eq!(after.live, before.live, "the freed block is not live any more");
    println!("heap_meter exact");
}

/// Attribution (R5): a steady-state allocation is charged to the innermost open scope, and the enclosing scope is current again
/// when the inner one drops; the per-scope counts sum to the total and the report names them (`allocs=`).
#[test]
fn an_allocation_is_charged_to_the_innermost_scope() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let before = snapshot();
    {
        let _outer = HookScope::enter_hook(HookId::CpuFrame);
        {
            let _inner = HookScope::enter_hook(HookId::RosterKirby);
            std::hint::black_box(vec![0u8; 32]);
        }
        std::hint::black_box(vec![0u8; 32]);
    }
    let after = snapshot();
    let charged = |id: HookId| after.allocs[id as usize] - before.allocs[id as usize];
    assert!(charged(HookId::RosterKirby) >= 1, "the inner scope made one");
    assert!(charged(HookId::CpuFrame) >= 1, "the outer scope is current again after the inner drops");
    assert_eq!(after.allocs.iter().sum::<u64>(), after.allocations, "per-scope counts sum to the total");
    let mut observer = Observer::new();
    let line = observer.observe(after).expect("activity").to_string();
    assert!(line.contains(" allocs=") && line.contains("roster_kirby:"), "{line}");
}

/// Nesting: a hook that calls another hook's code stays inside a scope until the outer one drops.
#[test]
fn nested_scopes_count_until_the_outermost_ends() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let before = hook_allocations();
    {
        let _outer = HookScope::enter_hook(HookId::CpuFrame);
        {
            let _inner = HookScope::enter_hook(HookId::CpuSdi);
        }
        let v = vec![0u8; 64];
        std::hint::black_box(&v);
    }
    assert!(hook_allocations() > before, "the allocation after the inner scope ended is still inside the outer");
    let after = hook_allocations();
    let v = vec![0u8; 64];
    std::hint::black_box(&v);
    assert_eq!(hook_allocations(), after);
}

/// The allocations are charged to the innermost hook (`allocs=`), so a nonzero delta names its owner: a nested hook's
/// allocation goes to the nested hook, the outer one's to the outer one, and the line says which.
#[test]
fn an_allocation_is_charged_to_the_innermost_hook() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let before = snapshot();
    {
        let _outer = HookScope::enter_hook(HookId::CpuFrame);
        assert_eq!(allocating_hook(HookId::HitboxFrame, 64), 64);
        let v = vec![0u8; 32];
        std::hint::black_box(&v);
    }
    let after = snapshot();
    let by = |id: HookId| after.allocs[id as usize] - before.allocs[id as usize];
    assert!(by(HookId::HitboxFrame) >= 1, "the nested hook's allocation");
    assert!(by(HookId::CpuFrame) >= 1, "the outer hook's own allocation after the nested one returned");
    let mut observer = Observer::new();
    let line = observer.observe(after).expect("a line").to_string();
    assert!(line.contains(" allocs=") && line.contains("hitbox_frame:") && line.contains("cpu_frame:"), "{line}");
}
