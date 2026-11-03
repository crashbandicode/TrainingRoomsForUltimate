#[path = "../src/model/hitbox_observation.rs"]
mod policy;
use policy::{Capture, Outcome, CONSTRUCTOR_RETURN, HELPER_RETURN};
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
