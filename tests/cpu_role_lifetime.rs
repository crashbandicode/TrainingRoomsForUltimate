//! R3-1: the exact production retirement/consumer adapter with native-call spies.
//! This is not a claim to execute Skyline hooks or reproduce a hardware fault.
#[path = "../src/runtime/features/cpu_role_adapter.rs"]
mod adapter;
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;
#[path = "../src/engine/mod.rs"]
pub mod engine;

use model::cpu::{
    role::{FighterKey, FrameReceipt, Scene},
    state::Cpu,
};

const TRAINING: Option<Scene> = Some(Scene::Training);
const P1: usize = 0x1000;
const A: usize = 0xa000;
const B: usize = 0xb000;

fn cpu() -> Cpu {
    let mut cpu = Cpu::factory(0x703);
    cpu.settings.frame_advantage = true;
    cpu
}

fn register(
    cpu: &mut Cpu,
    scene: Option<Scene>,
    owner: i32,
    partner: bool,
    accessor: usize,
) -> FrameReceipt {
    let receipt = adapter::before_frame(cpu, scene, owner).unwrap();
    assert!(cpu
        .roles
        .set(receipt, accessor, FighterKey::new(owner, partner).unwrap()));
    receipt
}

#[derive(Default)]
struct NativeSpy {
    reads: Vec<usize>,
    writes: Vec<usize>,
}

impl NativeSpy {
    fn measure(&mut self, cpu: &mut Cpu, scene: Option<Scene>, fresh: Option<usize>) {
        // Same production adapter plan; native queries happen only after admission
        // and outside its CPU-state mutation (the runtime drops its lock first).
        if let Some((player, other)) = adapter::measurement(cpu, scene, P1, fresh) {
            self.reads.extend([player, other]);
        }
    }

    fn command(
        &mut self,
        cpu: &mut Cpu,
        scene: Option<Scene>,
        accessor: usize,
        original: u32,
    ) -> u32 {
        let Some(_) = adapter::role(cpu, scene, accessor) else {
            return original;
        };
        self.writes.push(accessor);
        original | 0x10
    }
}

/// SPEC-input-06 §5.1: the frame strip's CPU is the live primary role only, whatever Frame Advantage says, and never
/// a departed entry or P1 itself.
#[test]
fn the_frame_strip_reads_only_the_live_primary_cpu() {
    let mut cpu = cpu();
    cpu.settings.frame_advantage = false;
    register(&mut cpu, TRAINING, 1, false, A);
    assert_eq!(adapter::primary_cpu(&cpu, Scene::Training, P1, A), Some(A), "Frame Advantage off: still read");
    assert_eq!(adapter::primary_cpu(&cpu, Scene::Training, P1, B), None, "not the registered accessor");
    assert_eq!(adapter::primary_cpu(&cpu, Scene::Training, A, A), None, "never P1's own accessor");
    adapter::entry_departed(&mut cpu, 1);
    assert_eq!(adapter::primary_cpu(&cpu, Scene::Training, P1, A), None, "a departed entry is never read");
}

#[test]
fn r3_predestruction_then_p1_before_replacement_never_queries_a() {
    let mut cpu = cpu();
    register(&mut cpu, TRAINING, 1, false, A);
    let mut spy = NativeSpy::default();
    spy.measure(&mut cpu, TRAINING, Some(A));
    assert_eq!(spy.reads, [P1, A]);
    spy.reads.clear();
    // This is the same adapter the native on_destroy calls before body-bank/free.
    adapter::entry_departed(&mut cpu, 1);
    spy.measure(&mut cpu, TRAINING, Some(B));
    spy.measure(&mut cpu, TRAINING, Some(A));
    assert!(
        spy.reads.is_empty(),
        "P1 before B cannot query either stale A or unregistered B"
    );
    register(&mut cpu, TRAINING, 1, false, B);
    spy.measure(&mut cpu, TRAINING, Some(B));
    assert_eq!(
        spy.reads,
        [P1, B],
        "valid replacement restores normal measurement"
    );
}

#[test]
fn r3_scene_exit_reentry_without_cpu_frame_or_operation_lease() {
    let mut cpu = cpu();
    let old = register(&mut cpu, TRAINING, 1, false, A);
    adapter::scene_departed(&mut cpu);
    assert!(!adapter::current_frame(&mut cpu, TRAINING, old));
    let mut spy = NativeSpy::default();
    spy.measure(&mut cpu, TRAINING, Some(A));
    assert_eq!(spy.command(&mut cpu, TRAINING, A, 0xa5), 0xa5);
    assert!(spy.reads.is_empty() && spy.writes.is_empty());
}

#[test]
fn r3_same_address_reuse_does_not_revalidate_an_old_frame_receipt() {
    let mut cpu = cpu();
    let old = register(&mut cpu, TRAINING, 1, false, A);
    adapter::entry_departed(&mut cpu, 1);
    let fresh = register(&mut cpu, TRAINING, 1, false, A);
    assert_ne!(old, fresh);
    let key = FighterKey::new(1, false).unwrap();
    assert!(
        !cpu.roles.set(old, B, key),
        "late old frame cannot replace reused-address registration"
    );
    assert_eq!(adapter::role(&mut cpu, TRAINING, A), Some(key));
    assert_eq!(adapter::role(&mut cpu, TRAINING, B), None);
}

#[test]
fn r3_original_frame_destruction_invalidates_post_original_registration() {
    let mut cpu = cpu();
    let receipt = adapter::before_frame(&mut cpu, TRAINING, 1).unwrap();
    adapter::entry_departed(&mut cpu, 1); // nested native original callback
    assert!(!adapter::current_frame(&mut cpu, TRAINING, receipt));
    assert!(!cpu
        .roles
        .set(receipt, A, FighterKey::new(1, false).unwrap()));
}

#[test]
fn r3_paired_owner_retirement_revokes_both_bodies_only() {
    let mut cpu = cpu();
    register(&mut cpu, TRAINING, 1, false, A);
    register(&mut cpu, TRAINING, 1, true, B);
    register(&mut cpu, TRAINING, 2, false, 0xc000);
    adapter::entry_departed(&mut cpu, 1);
    assert_eq!(adapter::role(&mut cpu, TRAINING, A), None);
    assert_eq!(adapter::role(&mut cpu, TRAINING, B), None);
    assert_eq!(
        adapter::role(&mut cpu, TRAINING, 0xc000),
        FighterKey::new(2, false)
    );
}

#[test]
fn r3_outside_practice_hook_before_any_cpu_frame_preserves_native_result() {
    let mut cpu = cpu();
    let receipt = register(&mut cpu, TRAINING, 1, false, A);
    let mut spy = NativeSpy::default();
    for original in [0, 0xa5, u32::MAX] {
        assert_eq!(spy.command(&mut cpu, None, A, original), original);
        spy.measure(&mut cpu, None, Some(A));
    }
    assert!(spy.reads.is_empty() && spy.writes.is_empty());
    assert!(!adapter::current_frame(&mut cpu, TRAINING, receipt));
    assert_eq!(
        adapter::role(&mut cpu, TRAINING, A),
        None,
        "no CPU frame required to retire scope"
    );
}

#[test]
fn r3_busy_swap_scope_and_waiting_training_change_require_fresh_registration() {
    let mut cpu = cpu();
    let old = register(&mut cpu, TRAINING, 1, false, A);
    assert!(adapter::before_frame(&mut cpu, None, 0).is_none());
    assert!(!adapter::current_frame(&mut cpu, TRAINING, old));
    let waiting = Some(Scene::Lobby);
    register(&mut cpu, waiting, 4, false, B);
    assert_eq!(adapter::role(&mut cpu, TRAINING, B), None);
    assert_eq!(
        adapter::role(&mut cpu, waiting, B),
        None,
        "return is a new scope too"
    );
    register(&mut cpu, waiting, 4, false, B);
    let mut spy = NativeSpy::default();
    spy.measure(&mut cpu, waiting, Some(B));
    assert_eq!(spy.reads, [P1, B]);
}

#[test]
fn r3_p1_and_nonwaiting_entries_do_not_register_as_waiting_cpu() {
    let mut cpu = cpu();
    let scene = Some(Scene::Lobby);
    for owner in [0, 1, 2, 7] {
        let receipt = adapter::before_frame(&mut cpu, scene, owner).unwrap();
        assert!(!cpu
            .roles
            .set(receipt, A, FighterKey::new(owner, false).unwrap()));
    }
    assert!(adapter::before_frame(&mut cpu, scene, -1).is_none());
    assert!(adapter::before_frame(&mut cpu, scene, 8).is_none());
    assert_eq!(adapter::role(&mut cpu, scene, A), None);
}

#[test]
fn r3_p1_entry_departure_retires_cross_body_measurement_and_late_receipts() {
    let mut cpu = cpu();
    let old = register(&mut cpu, TRAINING, 1, false, A);
    adapter::entry_departed(&mut cpu, 0);
    let mut spy = NativeSpy::default();
    spy.measure(&mut cpu, TRAINING, Some(A));
    assert!(spy.reads.is_empty());
    assert!(!cpu.roles.set(old, A, FighterKey::new(1, false).unwrap()));
}

#[test]
fn r3_current_manager_body_must_match_primary_not_partner_or_other_cpu() {
    let mut cpu = cpu();
    register(&mut cpu, TRAINING, 1, false, A);
    register(&mut cpu, TRAINING, 1, true, B);
    register(&mut cpu, TRAINING, 2, false, 0xc000);
    let mut spy = NativeSpy::default();
    for fresh in [None, Some(B), Some(0xc000), Some(0xd000)] {
        spy.measure(&mut cpu, TRAINING, fresh);
    }
    assert!(spy.reads.is_empty());
    spy.measure(&mut cpu, TRAINING, Some(A));
    assert_eq!(spy.reads, [P1, A]);
}

#[test]
fn r3_invalid_owner_retirement_is_not_a_global_reset() {
    let mut cpu = cpu();
    register(&mut cpu, TRAINING, 1, false, A);
    adapter::entry_departed(&mut cpu, u32::MAX);
    assert_eq!(
        adapter::role(&mut cpu, TRAINING, A),
        FighterKey::new(1, false)
    );
}

#[test]
fn r3_native_callback_wiring_uses_real_adapter_at_authoritative_boundaries() {
    // Supplemental wiring check: adapter/spies above execute behavior. This
    // prevents moving the real revocation into service or behind lease checks.
    let roster = include_str!("../src/runtime/roster/character_roster.rs");
    let destroy = roster
        .split("pub unsafe fn on_destroy(entry: *const u8)")
        .nth(1)
        .unwrap();
    let revoke = destroy.find("practice::entry_departed(entry as usize, owner)").unwrap();
    assert!(revoke < destroy.find("body_bank::on_destroy(entry)").unwrap());
    let service = roster
        .split("unsafe fn service(ctx:")
        .nth(1)
        .unwrap()
        .split("unsafe fn service_body")
        .next()
        .unwrap();
    assert!(!service.contains("practice::entry_departed(entry as usize, owner)"));
    let practice = include_str!("../src/runtime/lobby/training/practice.rs");
    let scene = practice.split("pub fn scene_departed()").nth(1).unwrap();
    assert!(scene.contains("depart_session()"));
    assert!(!scene.split("fn depart_session()").next().unwrap().contains("if "));
    assert!(scene.contains("practice_policy::depart_session("));
    let entry = practice.split("pub fn entry_departed(").nth(1).unwrap().split("pub fn scene_departed()").next().unwrap();
    assert!(entry.contains("cpu::entry_departed(owner)"));
    assert!(entry.contains("owned || owner == 0"));
    let training = include_str!("../src/runtime/lobby/training/mod.rs");
    let clear = training.split("pub(crate) fn clear_session_state()").nth(1).unwrap().split('}').next().unwrap();
    assert!(clear.find("cpu::scene_departed()").unwrap() < clear.find("recorder::session_clear()").unwrap());
    let runtime = include_str!("../src/runtime/features/cpu.rs");
    let frame = runtime.split("extern \"C\" fn on_frame(").nth(1).unwrap();
    assert!(
        frame.find("role_adapter::before_frame").unwrap()
            < frame.find("let result = original(").unwrap()
    );
    assert!(runtime.contains("role_adapter::role(cpu, scene, accessor)"));
    assert!(runtime.contains("role_adapter::measurement(cpu, scene, player, fresh_cpu)"));
    assert!(runtime.contains("game::natives::module_accessor("));
    assert!(!runtime.contains("c.roles.accessor_of(key)"));
}

#[test]
fn r3_actual_session_departure_protocol_clears_consumers_without_a_lease() {
    use std::cell::RefCell;
    use engine::lobby::practice_policy::{depart_session, EntryLease};
    let mut cpu = cpu();
    let old = register(&mut cpu, TRAINING, 1, false, A);
    let mut lease = EntryLease::new();
    let order = RefCell::new(Vec::new());
    // The production practice::depart_session passes these two callbacks to
    // this exact protocol. An empty lease must NOT suppress any consumer.
    depart_session(
        || { assert!(!lease.retire()); order.borrow_mut().push("lease"); },
        || { adapter::scene_departed(&mut cpu); order.borrow_mut().push("cpu/ui/recorder"); },
    );
    assert_eq!(*order.borrow(), ["lease", "cpu/ui/recorder"]);
    let mut spy = NativeSpy::default();
    spy.measure(&mut cpu, TRAINING, Some(A));
    assert!(spy.reads.is_empty());
    assert!(!adapter::current_frame(&mut cpu, TRAINING, old));
    register(&mut cpu, TRAINING, 1, false, A);
    spy.measure(&mut cpu, TRAINING, Some(A));
    assert_eq!(spy.reads, [P1, A], "new lifetime at reused address recovers normally");
}
