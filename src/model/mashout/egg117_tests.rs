lib_unit_tests! {
use super::*;
use crate::model::mashout::grade::Grade;
use crate::model::mashout::native::{Input, Record, BUTTONS};
use std::collections::HashMap;

fn record(remaining: f32) -> Record {
    Record { remaining, automatic: 1.0, manual: 8.0, button_mul: 1.8, threshold: 0.5, sign: [0, 0],
        source: -1, stick_block: 0, button_block: 0, accepted: 0, other_block: 0,
        cross_reload: 4, other_reload: 3 }
}

fn seed(remaining: f32) -> Seed {
    Seed { record: record(remaining), held: 0, deadline: None }
}

fn finish(mut solver: Solver, work: u16) -> Result<Bounds, Error> {
    loop {
        if let Some(result) = solver.advance(work) { return result; }
    }
}

fn applied(boundary: u32, old: f32, operand: f32) -> Applied {
    Applied { boundary, old: old.to_bits(), operand: operand.to_bits(), new: add(old, operand).to_bits() }
}

/// eb755 generation 23: the printed Add operand -17.639999389648438 is the binary32 nearest -17.64 (bits 0xc18d1eb8).
const EB755_OPERAND: f32 = -17.64;

fn at(boundary: u32, operand: f32) -> Scheduled {
    Scheduled { boundary, operand }
}

/// Unreduced reference: every five-category held mask and X direction, with each write applied to every live state
/// after `boundary` ticks (the same search as native_tests' oracle, plus the schedule).
fn oracle(seed: Seed, writes: &[Scheduled], limit: u32) -> Option<u32> {
    let mut layer = vec![(seed.record, seed.held)];
    for tick in 1..=limit {
        for w in writes.iter().filter(|w| w.boundary == tick - 1) {
            for (r, _) in layer.iter_mut() { r.remaining = add(r.remaining, w.operand); }
        }
        let mut next = HashMap::new();
        for (before, held) in layer {
            for encoded in 0_u8..32 {
                let buttons = (encoded & 15) | ((encoded & 16) << 1);
                let pressed = (buttons ^ held) & buttons;
                let trigger = u32::from(pressed & 0x2e) | (u32::from(pressed & 1) << 9);
                for direction in [-1.0, 0.0, 1.0] {
                    let mut after = before;
                    after.step(Input { axes: [direction, 0.0], trigger });
                    if after.remaining <= 0.0 { return Some(tick); }
                    let key = (after.sign, after.stick_block, after.button_block,
                        after.other_block, after.accepted, buttons);
                    let entry = next.entry(key).or_insert((after, buttons));
                    if after.remaining < entry.0.remaining { *entry = (after, buttons); }
                }
            }
        }
        layer = next.into_values().collect();
    }
    None
}

/// No input, write by write: the native step loop itself.
fn no_input(seed: Seed, writes: &[Scheduled]) -> u32 {
    let mut r = seed.record;
    for tick in 1.. {
        for w in writes.iter().filter(|w| w.boundary == tick - 1) { r.remaining = add(r.remaining, w.operand); }
        r.step(Input::default());
        if r.remaining <= 0.0 { return tick; }
    }
    unreachable!()
}

#[test]
fn egg117_eb755_callback_is_reproduced_bit_exact() {
    // 113-117-eb755-ally-canonical generation 23: Add -17.639999389648438 at index 0 turns 100 into 82.36000061035156,
    // and the next eligible tick gives 81.36000061035156.
    let operand = EB755_OPERAND;
    assert_eq!(operand.to_bits(), 0xc18d_1eb8);
    let new = add(100.0, operand);
    assert_eq!(new.to_bits(), 82.36_f32.to_bits());
    let mut r = record(new);
    r.step(Input::default());
    assert_eq!(r.remaining.to_bits(), 81.36_f32.to_bits());
    let solver = replay(seed(100.0), &[applied(0, 100.0, operand)]).unwrap();
    let bounds = finish(solver, 7).unwrap();
    assert_eq!(bounds.no_input, Some(no_input(seed(100.0), &[at(0, operand)])));
    assert_eq!(bounds.no_input, Some(83));
    assert_eq!(Some(bounds.fastest), oracle(seed(100.0), &[at(0, operand)], bounds.fastest));
}

#[test]
fn egg117_empty_schedule_is_the_unscheduled_solver() {
    let mut cases = vec![seed(1.0), seed(14.5), seed(27.2), seed(90.0), seed(180.1)];
    let mut latched = seed(27.2);
    latched.held = BUTTONS;
    latched.record.stick_block = 2;
    latched.record.other_block = 3;
    latched.record.accepted = 0x25;
    cases.push(latched);
    let mut deadline = seed(90.0);
    deadline.record.automatic = 0.0;
    deadline.deadline = Some(80);
    cases.push(deadline);
    for case in cases {
        let plain = finish(Solver::new(case).unwrap(), 3);
        assert_eq!(finish(Solver::scheduled(case, &[]).unwrap(), 3), plain, "{case:?}");
        assert_eq!(finish(replay(case, &[]).unwrap(), 5), plain, "{case:?}");
    }
}

#[test]
fn egg117_scheduled_bounds_match_the_oracle_and_the_step_loop() {
    // A negative write after tick 0 would end some near-zero fastest cadence (ScheduleEscape, its own test below), so
    // the oracle cases lower the start only and raise later.
    let schedules: [&[Scheduled]; 4] = [
        &[at(0, -5.0)],
        &[at(0, 10.0), at(2, 3.5)],
        &[at(1, 4.25), at(1, 2.0), at(3, 7.0)],
        &[at(0, -5.0), at(1, 1.0), at(2, 1.0), at(2, 1.0), at(4, 2.5), at(5, 0.75), at(5, 0.5), at(6, 3.0)],
    ];
    let mut latched = seed(34.4);
    latched.held = 0x21;
    latched.record.button_block = 2;
    latched.record.accepted = 0x25;
    for case in [seed(14.5), seed(27.2), seed(34.4), latched] {
        for writes in schedules {
            let expected = finish(Solver::scheduled(case, writes).unwrap(), 64).unwrap();
            for work in [1, 2, 7] {
                assert_eq!(finish(Solver::scheduled(case, writes).unwrap(), work).unwrap(), expected);
            }
            assert_eq!(oracle(case, writes, expected.fastest), Some(expected.fastest), "{case:?} {writes:?}");
            assert_eq!(expected.no_input, Some(no_input(case, writes)), "{case:?} {writes:?}");
        }
    }
}

#[test]
fn egg117_a_write_after_the_fast_escape_moves_only_no_input() {
    let plain = finish(Solver::new(seed(90.0)).unwrap(), 9).unwrap();
    assert_eq!(plain, Bounds { fastest: 10, no_input: Some(90) });
    let later = finish(Solver::scheduled(seed(90.0), &[at(20, 30.0)]).unwrap(), 9).unwrap();
    assert_eq!(later, Bounds { fastest: 10, no_input: Some(120) }, "an escaped cadence is not extended");
    let past = finish(Solver::scheduled(seed(90.0), &[at(95, 30.0)]).unwrap(), 9).unwrap();
    assert_eq!(past, plain, "a write after every cadence ended changes nothing");
}

#[test]
fn egg117_a_write_that_ends_a_cadence_is_unscored() {
    assert_eq!(finish(replay(seed(90.0), &[applied(0, 90.0, -90.0)]).unwrap(), 3), Err(Error::ScheduleEscape));
    // The fastest cadence escapes at tick 10, so after 9 ticks it holds at most one tick's best (8 × 1.8 + 1 = 15.4);
    // a -16 write then ends it by itself.
    assert_eq!(finish(Solver::scheduled(seed(90.0), &[at(9, -16.0)]).unwrap(), 3), Err(Error::ScheduleEscape));
    assert_eq!(judge(Err(Error::ScheduleEscape), 40, true), Err(Unscored::Bounds(Error::ScheduleEscape)));
}

#[test]
fn egg117_s3_refuses_writes_the_callee_cannot_have_stored() {
    let ok = applied(0, 100.0, EB755_OPERAND);
    let nine = [ok; SCHEDULE + 1];
    assert!(matches!(replay(seed(100.0), &nine), Err(Unscored::Writes)));
    assert!(matches!(replay(seed(100.0), &[applied(3, 100.0, -1.0), applied(2, 99.0, -1.0)]), Err(Unscored::Writes)));
    let mut off = ok;
    off.new += 1;
    assert!(matches!(replay(seed(100.0), &[off]), Err(Unscored::Writes)));
    let mut clamped = applied(0, 3.0, -5.0);
    assert_eq!(clamped.new, 0.0_f32.to_bits(), "FMAX(-2, +0) stores +0");
    clamped.new = (-0.0_f32).to_bits();
    assert!(matches!(replay(seed(100.0), &[clamped]), Err(Unscored::Writes)));
    for bad in [f32::NAN, f32::INFINITY] {
        assert!(matches!(replay(seed(100.0), &[applied(0, 100.0, bad)]), Err(Unscored::Writes)), "{bad}");
        assert!(matches!(replay(seed(100.0), &[applied(0, bad, -1.0)]), Err(Unscored::Writes)), "{bad}");
    }
    let mut constant = seed(90.0);
    constant.record.automatic = 0.0;
    constant.record.manual = 0.0;
    constant.deadline = Some(3);
    assert!(matches!(replay(constant, &[applied(0, 90.0, -1.0)]), Err(Unscored::Writes)), "answered without ticking");
}

/// A native no-input attempt with Adds before or after a frame's tick, observed under clatter.rs's G1 rule (a write
/// applies before the frame's tick when its old is bit-equal to the model, else after it): the boundaries G1 records
/// replay the actual run exactly, so the no-input bound equals the actual escape tick.
#[test]
fn egg117_g1_bit_equal_boundaries_replay_the_actual_run_exactly() {
    // (frame, before the frame's tick, operand)
    let natives = [(0, true, EB755_OPERAND), (5, false, 3.25), (9, true, -1.5), (20, false, 0.125)];
    let (mut live, mut model) = (record(100.0), record(100.0));
    let (mut elapsed, mut writes) = (0_u32, Vec::new());
    while live.remaining > 0.0 {
        let mut published = Vec::new();
        for before in [true, false] {
            if !before { live.step(Input::default()); }
            for &(_, _, operand) in natives.iter().filter(|n| n.0 == elapsed && n.1 == before) {
                published.push((live.remaining, operand));
                live.remaining = add(live.remaining, operand);
            }
        }
        let mut ticked = false;
        for (old, operand) in published {
            if model.remaining.to_bits() != old.to_bits() && !ticked {
                model.step(Input::default());
                ticked = true;
            }
            assert_eq!(model.remaining.to_bits(), old.to_bits(), "G1 finds the bit-equal boundary");
            writes.push(applied(elapsed + u32::from(ticked), old, operand));
            model.remaining = add(model.remaining, operand);
        }
        if !ticked { model.step(Input::default()); }
        assert_eq!(model, live);
        elapsed += 1;
    }
    assert_eq!(writes.iter().map(|w| w.boundary).collect::<Vec<_>>(), [0, 6, 9, 21]);
    let bounds = finish(replay(seed(100.0), &writes).unwrap(), 5).unwrap();
    assert_eq!(bounds.no_input, Some(elapsed));
    assert_eq!(judge(Ok(bounds), elapsed, true), Ok(Outcome::Graded(Grade::Poor)));
}

#[test]
fn egg117_grades_only_with_the_provider_closed() {
    let operand = EB755_OPERAND;
    let bounds = finish(replay(seed(100.0), &[applied(0, 100.0, operand)]).unwrap(), 11);
    let b = bounds.unwrap();
    assert_eq!(judge(bounds, b.fastest, false), Err(Unscored::ProviderOpen));
    assert_eq!(judge(bounds, b.fastest, true), Ok(Outcome::Graded(Grade::Optimal)));
    assert_eq!(judge(bounds, 83, true), Ok(Outcome::Graded(Grade::Poor)));
    assert_eq!(TAG, 117);
}
}
