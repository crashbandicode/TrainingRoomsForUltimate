#[path = "../src/model/hitbox_observation.rs"]
mod policy;
use policy::{Capture, Outcome, CONSTRUCTOR_RETURN, HELPER_RETURN};
use policy::{FailureBudget, HelperReason};
const THREAD: usize = 0x1230;
const ENTRY_SP: usize = 0x10000;
const FRAME: usize = ENTRY_SP - 0xf0;
const OBJECT: usize = 0x4560;
fn entered() -> Capture {
    let c = Capture::default();
    assert!(c.begin(THREAD));
    c.constructor(THREAD, ENTRY_SP, OBJECT, CONSTRUCTOR_RETURN);
    c
}
#[test]
fn actual_branch_outcomes_remain_distinct() {
    for (helper, final_phase, result, expected) in [
        (Some(0), false, 0, Outcome::HelperRejected),
        (Some(1), false, 0, Outcome::HandleOrEpochRejected),
        (Some(1), true, 0, Outcome::FinalPayloadRejected),
        (None, true, 0, Outcome::FinalPayloadRejected),
        (None, false, 0, Outcome::BeforeHelper),
        (Some(1), true, 1, Outcome::ConstructorSucceeded),
    ] {
        let c = entered();
        if let Some(value) = helper {
            c.helper_return(THREAD, FRAME - 0x40, HELPER_RETURN, value);
        }
        if final_phase {
            c.final_phase(THREAD, FRAME, OBJECT);
        }
        c.constructor_return(THREAD, FRAME, result);
        let s = c.finish(THREAD).unwrap();
        assert_eq!(s.outcome, expected);
        assert_eq!(
            (s.object, s.frame, s.helper, s.returned),
            (OBJECT, FRAME, helper, Some(result))
        );
    }
}
#[test]
fn foreign_threads_frames_objects_and_helper_callers_cannot_supply_evidence() {
    let c = entered();
    assert!(!c.begin(THREAD + 1));
    c.constructor(THREAD + 1, ENTRY_SP, OBJECT, CONSTRUCTOR_RETURN);
    c.helper_return(THREAD + 1, FRAME - 0x40, HELPER_RETURN, 1);
    c.helper_return(THREAD, FRAME - 0x50, HELPER_RETURN, 1);
    c.helper_return(THREAD, FRAME - 0x40, HELPER_RETURN + 4, 1);
    c.final_phase(THREAD, FRAME, OBJECT + 1);
    c.final_phase(THREAD, FRAME - 16, OBJECT);
    c.final_phase(THREAD + 1, FRAME, OBJECT);
    c.constructor_return(THREAD + 1, FRAME, 1);
    c.constructor_return(THREAD, FRAME - 16, 1);
    assert!(c.finish(THREAD + 1).is_none());
    assert_eq!(c.finish(THREAD).unwrap().outcome, Outcome::NotReturned);
}
#[test]
fn nested_request_and_multiple_constructors_never_guess() {
    let c = entered();
    assert!(!c.begin(THREAD));
    c.constructor_return(THREAD, FRAME, 0);
    assert_eq!(c.finish(THREAD).unwrap().outcome, Outcome::Ambiguous);
    assert!(c.begin(THREAD));
    c.constructor(THREAD, ENTRY_SP, OBJECT, CONSTRUCTOR_RETURN);
    c.constructor(THREAD, ENTRY_SP, OBJECT, CONSTRUCTOR_RETURN);
    c.constructor_return(THREAD, FRAME, 0);
    assert_eq!(c.finish(THREAD).unwrap().outcome, Outcome::Ambiguous);
}
#[test]
fn duplicate_and_contradictory_samples_remain_unattributed() {
    for mode in 0..3 {
        let c = entered();
        c.helper_return(THREAD, FRAME - 0x40, HELPER_RETURN, 0);
        if mode == 0 {
            c.helper_return(THREAD, FRAME - 0x40, HELPER_RETURN, 0);
        }
        if mode == 1 {
            c.final_phase(THREAD, FRAME, OBJECT);
        }
        c.constructor_return(THREAD, FRAME, 0);
        if mode == 2 {
            c.constructor_return(THREAD, FRAME, 0);
        }
        assert_eq!(c.finish(THREAD).unwrap().outcome, Outcome::Ambiguous);
    }
}
#[test]
fn missing_invalid_and_reused_requests_are_explicit() {
    let c = Capture::new();
    assert!(!c.begin(0));
    assert!(c.finish(0).is_none());
    assert!(c.begin(THREAD));
    assert_eq!(c.finish(THREAD).unwrap().outcome, Outcome::Unobserved);
    for (sp, object, caller) in [
        (0, OBJECT, CONSTRUCTOR_RETURN),
        (ENTRY_SP + 1, OBJECT, CONSTRUCTOR_RETURN),
        (ENTRY_SP, 0, CONSTRUCTOR_RETURN),
        (ENTRY_SP, OBJECT, CONSTRUCTOR_RETURN + 4),
    ] {
        assert!(c.begin(THREAD));
        c.constructor(THREAD, sp, object, caller);
        assert_eq!(c.finish(THREAD).unwrap().outcome, Outcome::Ambiguous);
    }
    assert!(c.begin(THREAD));
    c.constructor(THREAD, ENTRY_SP, OBJECT, CONSTRUCTOR_RETURN);
    c.helper_return(THREAD, FRAME - 0x40, HELPER_RETURN, 0);
    c.constructor_return(THREAD, FRAME, 0);
    assert_eq!(c.finish(THREAD).unwrap().outcome, Outcome::HelperRejected);
    assert!(c.begin(THREAD + 1));
    let s = c.finish(THREAD + 1).unwrap();
    assert_eq!(
        (s.object, s.frame, s.helper, s.returned),
        (0, 0, None, None)
    );
    assert_eq!(s.outcome, Outcome::Unobserved);
}

fn rejected(c: &Capture, regs: [usize; 3]) -> policy::Snapshot {
    c.helper_return(THREAD, FRAME - 0x40, HELPER_RETURN, 0);
    c.helper_registers(THREAD, FRAME - 0x40, HELPER_RETURN, regs);
    c.constructor_return(THREAD, FRAME, 0);
    c.finish(THREAD).unwrap()
}

fn sample(requested: i32, available: i32) -> policy::Demand {
    policy::Demand {
        requested,
        available,
        output: OBJECT + 0x1d0,
        manager: 0x9000,
        sp: FRAME - 0x40,
        caller: HELPER_RETURN,
    }
}

#[test]
fn positive_signed_demand_rejection_preserves_original_registers() {
    for (requested, available) in [(5, 4), (0, -1), (-1, -2), (i32::MAX, i32::MIN)] {
        let c = entered();
        let evidence = sample(requested, available);
        c.demand_rejected(THREAD, evidence);
        let s = rejected(&c, [u32::MAX as usize, available as u32 as usize, 0x9000]);
        assert_eq!(s.outcome, Outcome::HelperRejected);
        assert_eq!(s.helper_reason, HelperReason::EmitterDemandRejected);
        assert_eq!(s.demand, Some(evidence));
        assert_eq!(s.helper_registers.unwrap()[2], evidence.manager);
    }
}

#[test]
fn unsigned_order_equal_and_nonrejecting_counts_never_pass() {
    for (requested, available) in [(-1, 0), (0, 0), (1, 2), (i32::MIN, i32::MAX)] {
        let c = entered();
        c.demand_rejected(THREAD, sample(requested, available));
        assert_eq!(
            rejected(&c, [0, 0, 0x9000]).helper_reason,
            HelperReason::Ambiguous
        );
    }
}

#[test]
fn early_resource_rejections_require_unrestored_constructor_identity() {
    for (regs, expected) in [
        ([0, 87, OBJECT], HelperReason::ResourceNull),
        ([0x8880, 0, OBJECT], HelperReason::ResourceDisabled),
        ([0x8880, 1, OBJECT], HelperReason::Ambiguous),
        ([u32::MAX as usize, 0, 0x9000], HelperReason::Unattributed),
        ([0, 0, 0], HelperReason::Ambiguous),
    ] {
        assert_eq!(rejected(&entered(), regs).helper_reason, expected);
    }
}

#[test]
fn foreign_threads_and_inactive_requests_cannot_supply_demand_evidence() {
    let c = Capture::new();
    c.demand_rejected(THREAD, sample(2, 1));
    assert!(c.begin(THREAD));
    c.constructor(THREAD, ENTRY_SP, OBJECT, CONSTRUCTOR_RETURN);
    c.demand_rejected(THREAD + 1, sample(2, 1));
    let s = rejected(&c, [u32::MAX as usize, 1, 0x9000]);
    assert_eq!(s.helper_reason, HelperReason::Unattributed);
    assert_eq!(s.demand, None);
}

#[test]
fn caller_object_manager_and_checked_frame_mismatches_are_ambiguous() {
    for evidence in [
        policy::Demand {
            output: OBJECT + 0x1d8,
            ..sample(2, 1)
        },
        policy::Demand {
            manager: 0,
            ..sample(2, 1)
        },
        policy::Demand {
            manager: OBJECT,
            ..sample(2, 1)
        },
        policy::Demand {
            caller: HELPER_RETURN + 4,
            ..sample(2, 1)
        },
        policy::Demand {
            sp: FRAME - 0x50,
            ..sample(2, 1)
        },
        policy::Demand {
            sp: usize::MAX,
            ..sample(2, 1)
        },
    ] {
        let c = entered();
        c.demand_rejected(THREAD, evidence);
        assert_eq!(
            rejected(&c, [0, 0, 0x9000]).helper_reason,
            HelperReason::Ambiguous
        );
    }
    let c = Capture::new();
    assert!(c.begin(THREAD));
    c.constructor(THREAD, ENTRY_SP, usize::MAX - 4, CONSTRUCTOR_RETURN);
    c.demand_rejected(THREAD, sample(2, 1));
    assert_eq!(
        rejected(&c, [0, 0, 0x9000]).helper_reason,
        HelperReason::Ambiguous
    );
}

#[test]
fn duplicate_late_and_contradictory_marker_or_manager_never_guesses() {
    for mode in 0..5 {
        let c = entered();
        c.demand_rejected(THREAD, sample(2, 1));
        if mode == 0 {
            c.demand_rejected(THREAD, sample(2, 1));
        }
        if mode == 1 {
            c.demand_rejected(THREAD, sample(3, 1));
        }
        if mode == 2 {
            c.helper_return(THREAD, FRAME - 0x40, HELPER_RETURN, 0);
            c.demand_rejected(THREAD, sample(2, 1));
        }
        let manager = match mode {
            3 => OBJECT,
            4 => 0x9010,
            _ => 0x9000,
        };
        assert_eq!(
            rejected(&c, [0, 0, manager]).helper_reason,
            HelperReason::Ambiguous
        );
    }
}

#[test]
fn missing_registers_success_and_reuse_cannot_supply_false_reason() {
    let c = entered();
    c.demand_rejected(THREAD, sample(2, 1));
    c.helper_return(THREAD, FRAME - 0x40, HELPER_RETURN, 0);
    c.helper_registers(THREAD + 1, FRAME - 0x40, HELPER_RETURN, [0, 0, OBJECT]);
    c.helper_registers(THREAD, FRAME - 0x50, HELPER_RETURN, [0, 0, OBJECT]);
    c.helper_registers(THREAD, FRAME - 0x40, HELPER_RETURN + 4, [0, 0, OBJECT]);
    c.constructor_return(THREAD, FRAME, 0);
    assert_eq!(
        c.finish(THREAD).unwrap().helper_reason,
        HelperReason::Unattributed
    );
    assert!(c.begin(THREAD));
    c.constructor(THREAD, ENTRY_SP, OBJECT, CONSTRUCTOR_RETURN);
    let s = rejected(&c, [u32::MAX as usize, 0, 0x9000]);
    assert_eq!(s.helper_reason, HelperReason::Unattributed);
    assert_eq!(s.demand, None);
    let c = entered();
    c.demand_rejected(THREAD, sample(2, 1));
    c.helper_return(THREAD, FRAME - 0x40, HELPER_RETURN, 1);
    c.constructor_return(THREAD, FRAME, 1);
    assert_eq!(
        c.finish(THREAD).unwrap().helper_reason,
        HelperReason::Ambiguous
    );
}

#[test]
fn successful_handles_do_not_spend_or_emit_and_failure_budget_is_exactly_eight() {
    let budget = FailureBudget::default();
    for n in 1..=8 {
        assert!(budget.available());
        assert_eq!(budget.record(1), None);
        assert_eq!(budget.record(u32::MAX), None);
        assert_eq!(budget.record(0), Some(n));
    }
    assert!(!budget.available());
    assert_eq!(budget.record(0), None);
    assert_eq!(budget.record(1), None);
}
