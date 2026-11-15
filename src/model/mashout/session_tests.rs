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
    let mut early = Session::<u32>::new();
    early.begin(1, 7, seed(10.0)).unwrap();
    early.advance(10);
    assert_eq!(early.run().bounds(), Some(Bounds { fastest: 1, no_input: Some(10) }));
    assert_eq!(early.run().result(), None);
    early.observe(1, 3, true);

    let mut late = Session::<u32>::new();
    late.begin(1, 7, seed(10.0)).unwrap();
    late.observe(1, 3, true);
    assert!(late.run().escaped());
    assert_eq!(late.run().result(), None);
    late.observe(1, 99, true);
    late.advance(10);
    assert_eq!(late.run().result(), early.run().result());
    assert_eq!(late.run().elapsed(), 3);
    assert_eq!(late.run().result().unwrap().outcome, Outcome::Graded(Grade::Good));
}

#[test]
fn mashout_session_rejects_stale_generations_and_progress() {
    let mut session = Session::<u32>::new();
    assert_eq!(session.begin(0, 0, seed(10.0)), Err(Error::InvalidSeed));
    assert_eq!(session.run().phase(), Phase::Off);
    session.begin(1, 1, seed(10.0)).unwrap();
    session.advance(9);
    session.observe(1, 2, false);
    assert_eq!(session.begin(1, 2, seed(1.0)), Err(Error::InvalidSeed));
    assert_eq!(session.begin(0, 2, seed(1.0)), Err(Error::InvalidSeed));
    assert_eq!(session.run().key(), Some(1));
    assert_eq!(session.run().elapsed(), 2);
    assert_eq!(session.error(), None);
    // Replay did not reset the nine ticks of valid work.
    session.advance(1);
    assert_eq!(session.run().bounds().unwrap().no_input, Some(10));

    session.begin(2, 2, seed(90.0)).unwrap();
    session.observe(1, 500, true);
    assert!(!session.pass(1, true));
    session.advance(1);
    assert_eq!(session.run().bounds(), None, "old completed bounds cannot publish");
    assert_eq!(session.run().elapsed(), 0);
    session.advance(88);
    session.begin(3, 3, seed(20.0)).unwrap();
    session.advance(1);
    assert_eq!(session.run().bounds(), None, "old partial search cannot finish the new seed");
    session.advance(19);
    assert_eq!(session.run().bounds().unwrap().no_input, Some(20));
    assert_eq!(session.run().result(), None);
    session.observe(2, 1, true);
    assert_eq!(session.run().result(), None);
    session.observe(3, 5, true);
    let result = session.run().result();
    session.advance(u32::MAX);
    session.observe(2, 900, true);
    assert_eq!(session.run().result(), result);
}

#[test]
fn mashout_session_cancel_preserves_bests_and_watermark() {
    let mut session = Session::<u32>::new();
    session.begin(1, 4, seed(10.0)).unwrap();
    session.advance(10);
    session.observe(1, 1, true);
    assert_eq!(session.run().best(4), Some(1));
    session.begin(2, 5, seed(90.0)).unwrap();
    session.advance(5);
    session.cancel();
    session.advance(100);
    session.observe(2, 1, true);
    assert!(!session.pass(2, true));
    assert_eq!(session.run().phase(), Phase::Off);
    assert_eq!(session.run().bounds(), None);
    assert_eq!(session.run().result(), None);
    assert_eq!(session.run().best(4), Some(1));
    assert_eq!(session.error(), None);
    assert_eq!(session.begin(2, 5, seed(90.0)), Err(Error::InvalidSeed));
    session.begin(3, 4, seed(10.0)).unwrap();
    assert!(session.run().bounds().is_some(), "verified cache survives cancellation");
    assert!(session.solver.is_none());
}

#[test]
fn mashout_session_cache_binds_key_and_entire_seed() {
    let mut session = Session::<u32>::new();
    let mut original = seed(5.0);
    original.record.button_mul = 0.0;
    session.begin(1, 1, original).unwrap();
    session.advance(5);
    assert_eq!(session.run().bounds().unwrap().fastest, 1);
    let mut changed = original;
    changed.record.threshold = 1.0;
    session.begin(2, 1, changed).unwrap();
    assert_eq!(session.run().bounds(), None);
    assert!(session.solver.is_some());
    session.advance(5);
    assert_eq!(session.run().bounds(), Some(Bounds { fastest: 5, no_input: Some(5) }));
    session.begin(3, 1, original).unwrap();
    assert_eq!(session.run().bounds().unwrap().fastest, 1);
    session.begin(4, 2, original).unwrap();
    assert_eq!(session.run().bounds(), None, "another key never borrows bounds");
    changed = original;
    changed.held = 4;
    session.begin(5, 1, changed).unwrap();
    assert_eq!(session.run().bounds(), None);
    changed = original;
    changed.deadline = Some(4);
    session.begin(6, 1, changed).unwrap();
    assert_eq!(session.run().bounds(), None);
}

#[test]
fn mashout_session_cache_has_eight_verified_entries() {
    let mut session = Session::<u32>::new();
    for key in 0..9 {
        session.begin(u64::from(key) + 1, key, seed(10.0)).unwrap();
        session.advance(10);
        assert!(session.run().bounds().is_some());
    }
    session.begin(10, 0, seed(10.0)).unwrap();
    assert_eq!(session.run().bounds(), None, "ninth verified result evicts oldest");
    session.begin(11, 1, seed(10.0)).unwrap();
    assert!(session.run().bounds().is_some(), "unsolved replacement did not enter cache");
    assert!(session.solver.is_none());
}

#[test]
fn mashout_session_errors_are_explicit_and_unscored() {
    let mut session = Session::<u32>::new();
    let mut invalid = seed(10.0);
    invalid.record.threshold = f32::NAN;
    assert_eq!(session.begin(1, 1, invalid), Err(Error::InvalidSeed));
    session.observe(1, 1, true);
    session.advance(100);
    assert_eq!(session.error(), Some(Error::InvalidSeed));
    assert_eq!(session.run().bounds(), None);
    assert_eq!(session.run().result(), None);
    assert_eq!(session.run().best(1), None);
    assert_eq!(session.begin(1, 8, seed(10.0)), Err(Error::InvalidSeed));
    assert_eq!(session.error(), Some(Error::InvalidSeed));
    assert_eq!(session.run().key(), Some(1));

    let mut never = seed(10.0);
    never.record.automatic = 0.0;
    never.record.manual = 0.0;
    session.begin(2, 2, never).unwrap();
    assert_eq!(session.error(), None);
    session.advance(1);
    session.observe(2, 10, true);
    assert_eq!(session.error(), Some(Error::NeverEscapes));
    assert_eq!(session.run().result(), None);
    assert_eq!(session.run().best(2), None);
    assert!(session.cache.iter().all(Option::is_none));
    session.begin(3, 2, never).unwrap();
    assert!(session.solver.is_some(), "solver errors are never cached");
    session.cancel();
    assert_eq!(session.error(), None);
    session.begin(4, 1, seed(10.0)).unwrap();
    session.advance(10);
    session.observe(4, 1, true);
    assert!(session.run().result().is_some());
}

#[test]
fn mashout_session_retry_is_one_shot_and_generation_bound() {
    let mut session = Session::<u32>::new();
    session.begin(1, 1, seed(10.0)).unwrap();
    session.advance(10);
    session.observe(1, 1, true);
    let count = session.run().remaining();
    for _ in 0..count {
        assert!(!session.pass(0, true));
        assert!(!session.pass(1, false));
    }
    assert_eq!(session.run().remaining(), count);
    for _ in 1..count {
        assert!(!session.pass(1, true));
    }
    assert!(session.pass(1, true));
    assert_eq!(session.run().phase(), Phase::Setup);
    for _ in 0..count {
        assert!(!session.pass(1, true));
    }
    session.observe(1, 99, true);
    assert_eq!(session.run().result(), None);
    assert_eq!(session.begin(1, 1, seed(10.0)), Err(Error::InvalidSeed));
    session.begin(2, 1, seed(10.0)).unwrap();
    assert_eq!(session.run().phase(), Phase::Mashing);
}

#[test]
fn mashout_session_zero_budget_and_incremental_work() {
    let mut session = Session::<u32>::new();
    session.begin(1, 1, seed(10.0)).unwrap();
    session.advance(0);
    assert_eq!(session.run().bounds(), None);
    for _ in 0..9 {
        session.advance(1);
        assert_eq!(session.run().bounds(), None);
    }
    session.advance(1);
    assert_eq!(session.run().bounds(), Some(Bounds { fastest: 1, no_input: Some(10) }));
}
}
