lib_unit_tests! {
use super::*;
use super::super::grade::{Grade, Outcome};
use super::super::native::Record;

fn seed(remaining: f32) -> Seed {
    Seed {
        record: Record {
            remaining, automatic: 1.0, manual: 8.0, button_mul: 1.8,
            threshold: 0.5, sign: [0, 0], source: -1, stick_block: 0,
            button_block: 0, accepted: 0, other_block: 0, cross_reload: 4,
            other_reload: 3,
        },
        held: 0,
        deadline: None,
    }
}

#[test]
fn mashout_session_completion_order() {
    let mut early = Run::<u32>::new();
    early.capture(1, 7, seed(10.0)).unwrap();
    early.advance(10);
    assert_eq!(early.bounds(), Some(Bounds { fastest: 1, no_input: Some(10) }));
    assert_eq!(early.result(), None);
    early.observe_generation(1, 3, true);
    early.finish();

    let mut late = Run::<u32>::new();
    late.capture(1, 7, seed(10.0)).unwrap();
    late.observe_generation(1, 3, true);
    assert!(late.escaped());
    assert_eq!(late.result(), None);
    late.observe_generation(1, 99, true);
    late.advance(10);
    late.finish();
    assert_eq!(late.result(), early.result());
    assert_eq!(late.elapsed(), 3);
    assert_eq!(late.result().unwrap().outcome, Outcome::Graded(Grade::Good));
}

#[test]
fn mashout_session_rejects_stale_generations_and_progress() {
    let mut session = Run::<u32>::new();
    assert_eq!(session.capture(0, 0, seed(10.0)), Err(Error::InvalidSeed));
    assert_eq!(session.phase(), Phase::Off);
    session.capture(1, 1, seed(10.0)).unwrap();
    session.advance(9);
    session.observe_generation(1, 2, false);
    assert_eq!(session.capture(1, 2, seed(1.0)), Err(Error::InvalidSeed));
    assert_eq!(session.capture(0, 2, seed(1.0)), Err(Error::InvalidSeed));
    assert_eq!(session.key(), Some(1));
    assert_eq!(session.elapsed(), 2);
    assert_eq!(session.error(), None);
    // Replay did not reset the nine ticks of valid work.
    session.advance(1);
    assert_eq!(session.bounds().unwrap().no_input, Some(10));

    session.capture(2, 2, seed(90.0)).unwrap();
    session.observe_generation(1, 500, true);
    assert!(!session.pass_generation(1, true));
    session.advance(1);
    assert_eq!(session.bounds(), None, "old completed bounds cannot publish");
    assert_eq!(session.elapsed(), 0);
    session.advance(88);
    session.capture(3, 3, seed(20.0)).unwrap();
    session.advance(1);
    assert_eq!(session.bounds(), None, "old partial search cannot finish the new seed");
    session.advance(19);
    assert_eq!(session.bounds().unwrap().no_input, Some(20));
    assert_eq!(session.result(), None);
    session.observe_generation(2, 1, true);
    assert_eq!(session.result(), None);
    session.observe_generation(3, 5, true);
    session.finish();
    let result = session.result();
    session.advance(u32::MAX);
    session.observe_generation(2, 900, true);
    assert_eq!(session.result(), result);
}

#[test]
fn mashout_session_cancel_preserves_bests_and_watermark() {
    let mut session = Run::<u32>::new();
    session.capture(1, 4, seed(10.0)).unwrap();
    session.advance(10);
    session.observe_generation(1, 1, true);
    session.finish();
    assert_eq!(session.best(4), Some(1));
    session.capture(2, 5, seed(90.0)).unwrap();
    session.advance(5);
    session.cancel();
    session.advance(100);
    session.observe_generation(2, 1, true);
    assert!(!session.pass_generation(2, true));
    assert_eq!(session.phase(), Phase::Off);
    assert_eq!(session.bounds(), None);
    assert_eq!(session.result(), None);
    assert_eq!(session.best(4), Some(1));
    assert_eq!(session.error(), None);
    assert_eq!(session.capture(2, 5, seed(90.0)), Err(Error::InvalidSeed));
    session.capture(3, 4, seed(10.0)).unwrap();
    assert_eq!(session.bounds(), None, "cancellation retains bests, not solver results");
    session.advance(10);
    assert!(session.bounds().is_some());
}

#[test]
fn mashout_session_solver_binds_key_and_entire_seed() {
    let mut session = Run::<u32>::new();
    let mut original = seed(5.0);
    original.record.button_mul = 0.0;
    session.capture(1, 1, original).unwrap();
    session.advance(5);
    assert_eq!(session.bounds().unwrap().fastest, 1);
    let mut changed = original;
    changed.record.threshold = 1.0;
    session.capture(2, 1, changed).unwrap();
    assert_eq!(session.bounds(), None);
    assert!(session.solver.is_some());
    session.advance(5);
    assert_eq!(session.bounds(), Some(Bounds { fastest: 5, no_input: Some(5) }));
    session.capture(3, 1, original).unwrap();
    assert_eq!(session.bounds(), None);
    session.advance(5);
    assert_eq!(session.bounds().unwrap().fastest, 1);
    session.capture(4, 2, original).unwrap();
    assert_eq!(session.bounds(), None, "another key never borrows bounds");
    changed = original;
    changed.held = 4;
    session.capture(5, 1, changed).unwrap();
    assert_eq!(session.bounds(), None);
    changed = original;
    changed.deadline = Some(4);
    session.capture(6, 1, changed).unwrap();
    assert_eq!(session.bounds(), None);
}

#[test]
fn mashout_session_repeated_seed_recomputes_same_bounds() {
    let mut session = Run::<u32>::new();
    let mut expected = None;
    for generation in 1..12 {
        session.capture(generation, 0, seed(10.0)).unwrap();
        assert_eq!(session.bounds(), None);
        session.advance(10);
        if let Some(bounds) = expected { assert_eq!(session.bounds(), Some(bounds)); }
        expected = session.bounds();
    }
}

#[test]
fn mashout_session_errors_are_explicit_and_unscored() {
    let mut session = Run::<u32>::new();
    let mut invalid = seed(10.0);
    invalid.record.threshold = f32::NAN;
    assert_eq!(session.capture(1, 1, invalid), Err(Error::InvalidSeed));
    session.observe_generation(1, 1, true);
    session.advance(100);
    assert_eq!(session.error(), Some(Error::InvalidSeed));
    assert_eq!(session.bounds(), None);
    assert_eq!(session.result(), None);
    assert_eq!(session.best(1), None);
    assert_eq!(session.capture(1, 8, seed(10.0)), Err(Error::InvalidSeed));
    assert_eq!(session.error(), Some(Error::InvalidSeed));
    assert_eq!(session.key(), Some(1));

    let mut never = seed(10.0);
    never.record.automatic = 0.0;
    never.record.manual = 0.0;
    session.capture(2, 2, never).unwrap();
    assert_eq!(session.error(), None);
    session.advance(1);
    session.observe_generation(2, 10, true);
    assert_eq!(session.error(), Some(Error::NeverEscapes));
    assert_eq!(session.result(), None);
    assert_eq!(session.best(2), None);
    session.capture(3, 2, never).unwrap();
    assert!(session.solver.is_some(), "an errored attempt cannot supply the next bounds");
    session.cancel();
    assert_eq!(session.error(), None);
    session.capture(4, 1, seed(10.0)).unwrap();
    session.advance(10);
    session.observe_generation(4, 1, true);
    session.finish();
    assert!(session.result().is_some());
}

#[test]
fn mashout_session_retry_is_one_shot_and_generation_bound() {
    let mut session = Run::<u32>::new();
    session.capture(1, 1, seed(10.0)).unwrap();
    session.advance(10);
    session.observe_generation(1, 1, true);
    session.finish();
    let count = session.remaining();
    for _ in 0..count {
        assert!(!session.pass_generation(0, true));
        assert!(!session.pass_generation(1, false));
    }
    assert_eq!(session.remaining(), count);
    for _ in 1..count {
        assert!(!session.pass_generation(1, true));
    }
    assert!(session.pass_generation(1, true));
    assert_eq!(session.phase(), Phase::Setup);
    for _ in 0..count {
        assert!(!session.pass_generation(1, true));
    }
    session.observe_generation(1, 99, true);
    assert_eq!(session.result(), None);
    assert_eq!(session.capture(1, 1, seed(10.0)), Err(Error::InvalidSeed));
    session.capture(2, 1, seed(10.0)).unwrap();
    assert_eq!(session.phase(), Phase::Mashing);
}

#[test]
fn mashout_session_zero_budget_and_incremental_work() {
    let mut session = Run::<u32>::new();
    session.capture(1, 1, seed(10.0)).unwrap();
    session.advance(0);
    assert_eq!(session.bounds(), None);
    for _ in 0..9 {
        session.advance(1);
        assert_eq!(session.bounds(), None);
    }
    session.advance(1);
    assert_eq!(session.bounds(), Some(Bounds { fastest: 1, no_input: Some(10) }));
}

#[test]
fn mashout_completion_fault_after_solver_cannot_commit_a_best() {
    use super::super::admission::faulted;
    let mut session = Run::<u32>::new();
    session.capture(1, 1, seed(10.0)).unwrap();
    session.observe_generation(1, 1, true);
    session.advance(10);
    assert_eq!(session.result(), None, "solver readiness is not completion admission");
    assert_eq!(session.best(1), None);
    // A dropped event becomes visible while the worker solves, before its completion check.
    let fault_through = 1;
    if faulted(1, fault_through) { session.cancel(); } else { session.finish(); }
    assert_eq!(session.result(), None);
    assert_eq!(session.best(1), None);
    session.capture(2, 1, seed(10.0)).unwrap();
    session.observe_generation(2, 1, true);
    session.advance(10);
    if faulted(2, fault_through) { session.cancel(); } else { session.finish(); }
    assert!(session.result().is_some());
    assert_eq!(session.best(1), Some(1));
}
}
