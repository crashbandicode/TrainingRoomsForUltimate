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

use policy::{PoolBudget, PoolStatus, POOL_LIMIT};
const MANAGER: usize = 0x80000;
fn pool_entered() -> Capture {
    let c = entered();
    c.observe_pool(THREAD, 1);
    c
}
fn pool_sample(c: &Capture, total: u32, free: u32) -> usize {
    let mut reads = 0;
    c.pool_after(THREAD, FRAME - 0x40, HELPER_RETURN, 1, MANAGER, |address| {
        reads += 1;
        match address {
            x if x == MANAGER + 0x70 => total,
            x if x == MANAGER + 0x80 => free,
            _ => panic!("unexpected native field"),
        }
    });
    reads
}
#[test]
fn pool_actual_after_helper_counts_and_early_resource_paths() {
    let c = pool_entered();
    assert_eq!(pool_sample(&c, 800, 8), 2);
    c.helper_return(THREAD, FRAME - 0x40, HELPER_RETURN, 1);
    c.constructor_return(THREAD, FRAME, 1);
    let s = c.finish(THREAD).unwrap();
    assert!(
        !c.owns(THREAD),
        "logging must occur after ownership release"
    );
    let p = s.pool.unwrap();
    assert_eq!(
        (p.ordinal, p.manager, p.total, p.available),
        (1, MANAGER, 800, 8)
    );
    assert_eq!(p.status, PoolStatus::AfterHelper);
    for result in [0, 1] {
        let c = pool_entered();
        c.pool_after(THREAD, FRAME - 0x40, HELPER_RETURN, result, OBJECT, |_| {
            panic!("early resource path must not read the outer object")
        });
        assert_eq!(
            c.finish(THREAD).unwrap().pool.unwrap().status,
            PoolStatus::Denied
        );
    }
    let c = pool_entered();
    c.pool_after(THREAD, FRAME - 0x40, HELPER_RETURN, 0, MANAGER, |_| 800);
    assert_eq!(
        c.finish(THREAD).unwrap().pool.unwrap().status,
        PoolStatus::AfterHelper
    );
}
#[test]
fn pool_foreign_registers_bounds_and_missing_scope_never_read() {
    for (thread, sp, lr, result, manager) in [
        (THREAD + 1, FRAME - 0x40, HELPER_RETURN, 1, MANAGER),
        (THREAD, FRAME - 0x50, HELPER_RETURN, 1, MANAGER),
        (THREAD, FRAME - 0x40, HELPER_RETURN + 4, 1, MANAGER),
        (THREAD, FRAME - 0x40, HELPER_RETURN, 2, MANAGER),
        (THREAD, FRAME - 0x40, HELPER_RETURN, 1, 0),
        (THREAD, FRAME - 0x40, HELPER_RETURN, 1, MANAGER + 1),
        (THREAD, FRAME - 0x40, HELPER_RETURN, 1, usize::MAX - 3),
        (THREAD, usize::MAX - 1, HELPER_RETURN, 1, MANAGER),
    ] {
        let c = pool_entered();
        c.pool_after(thread, sp, lr, result, manager, |_| panic!("denied read"));
        let p = c.finish(THREAD).unwrap().pool.unwrap();
        assert_eq!(
            p.status,
            if thread == THREAD {
                PoolStatus::Denied
            } else {
                PoolStatus::MissingHelper
            }
        );
        assert_eq!(p.manager, 0);
    }
    let c = Capture::new();
    c.observe_pool(THREAD, 1);
    assert_eq!(pool_sample(&c, 800, 8), 0);
    assert!(c.begin(THREAD));
    c.observe_pool(THREAD, 1);
    assert_eq!(pool_sample(&c, 800, 8), 0, "no constructor");
    assert_eq!(
        c.finish(THREAD).unwrap().pool.unwrap().status,
        PoolStatus::Denied
    );
}
#[test]
fn pool_nesting_ambiguity_duplicate_and_late_reads_are_denied() {
    for mode in 0..5 {
        let c = pool_entered();
        match mode {
            0 => {
                assert!(!c.begin(THREAD));
            }
            1 => c.constructor(THREAD, ENTRY_SP, OBJECT, CONSTRUCTOR_RETURN),
            2 => c.helper_return(THREAD, FRAME - 0x40, HELPER_RETURN, 1),
            3 => c.constructor_return(THREAD, FRAME, 1),
            _ => c.constructor(THREAD, ENTRY_SP, OBJECT, CONSTRUCTOR_RETURN + 4),
        }
        assert_eq!(pool_sample(&c, 800, 8), 0);
        assert_ne!(
            c.finish(THREAD).unwrap().pool.unwrap().status,
            PoolStatus::AfterHelper
        );
    }
    let c = pool_entered();
    assert_eq!(pool_sample(&c, 800, 789), 2);
    assert_eq!(pool_sample(&c, 800, 778), 0, "duplicate never rereads");
    assert_eq!(
        c.finish(THREAD).unwrap().pool.unwrap().status,
        PoolStatus::Ambiguous
    );
    let c = pool_entered();
    assert_eq!(pool_sample(&c, 800, 789), 2);
    assert!(!c.begin(THREAD));
    assert_eq!(
        c.finish(THREAD).unwrap().pool.unwrap().status,
        PoolStatus::Ambiguous
    );
}
#[test]
fn pool_raw_anomalies_are_preserved_without_clamping() {
    for (total, free) in [(800, u32::MAX), (u32::MAX, 8), (800, 801)] {
        let c = pool_entered();
        assert_eq!(pool_sample(&c, total, free), 2);
        let p = c.finish(THREAD).unwrap().pool.unwrap();
        assert_eq!(p.status, PoolStatus::Inconsistent);
        assert_eq!((p.total, p.available), (total as i32, free as i32));
    }
    let c = pool_entered();
    assert_eq!(pool_sample(&c, 0, 0), 2);
    assert_eq!(
        c.finish(THREAD).unwrap().pool.unwrap().status,
        PoolStatus::AfterHelper
    );
}
#[test]
fn pool_window_reserves_missing_requests_and_preserves_failure_budget() {
    let budget = PoolBudget::new();
    let failed = FailureBudget::new();
    let c = Capture::new();
    for ordinal in 1..=POOL_LIMIT {
        assert!(budget.available());
        assert!(c.begin(THREAD));
        c.observe_pool(THREAD, budget.reserve().unwrap());
        let p = c.finish(THREAD).unwrap().pool.unwrap();
        assert_eq!(p.ordinal, ordinal);
        assert_eq!(p.status, PoolStatus::MissingHelper);
    }
    assert!(!budget.available());
    assert_eq!(budget.reserve(), None);
    assert_eq!(failed.record(1), None);
    for n in 1..=8 {
        assert_eq!(failed.record(0), Some(n));
    }
    assert_eq!(failed.record(0), None);
    assert!(c.begin(THREAD));
    c.constructor(THREAD, ENTRY_SP, OBJECT, CONSTRUCTOR_RETURN);
    assert_eq!(pool_sample(&c, 800, 8), 0, "no reserved ordinal");
    assert!(
        c.finish(THREAD).unwrap().pool.is_none(),
        "previous sample must be reset"
    );
}
#[test]
fn pool_record_bound_includes_max_fields_and_sink_prefix() {
    for outcome in [
        Outcome::Unobserved,
        Outcome::Ambiguous,
        Outcome::NotReturned,
        Outcome::ConstructorSucceeded,
        Outcome::HelperRejected,
        Outcome::HandleOrEpochRejected,
        Outcome::FinalPayloadRejected,
        Outcome::BeforeHelper,
    ] {
        let line = format!(
            "[HitboxPool] n={} stage=AfterHelper status={} mgr={:#x} total={} free={} handle={:?} outcome={:?}",
            POOL_LIMIT, PoolStatus::Inconsistent as usize, usize::MAX,
            i32::MIN, i32::MIN, Some(u32::MAX), outcome
        );
        assert!(
            line.len() + 53 <= 256,
            "{} bytes including prefix/reserve",
            line.len() + 53
        );
    }
}
