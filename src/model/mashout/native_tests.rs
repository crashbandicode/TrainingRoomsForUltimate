lib_unit_tests! {
use super::*;
use std::collections::HashMap;

fn record(remaining: f32) -> Record {
    Record { remaining, automatic: 1.0, manual: 8.0, button_mul: 1.8, threshold: 0.5, sign: [0, 0],
        source: -1, stick_block: 0, button_block: 0, accepted: 0, other_block: 0,
        cross_reload: 4, other_reload: 3 }
}

fn seed(remaining: f32) -> Seed {
    Seed { record: record(remaining), held: 0, deadline: None }
}

fn solve(seed: Seed) -> Result<Bounds, Error> {
    let mut solver = Solver::new(seed)?;
    loop {
        if let Some(result) = solver.advance(3) { return result; }
    }
}

#[test]
fn native_subtraction_order_and_latches() {
    // Independent native instruction fixtures: multiply, manual subtraction, automatic subtraction.
    let mut r = record(90.0);
    r.step(Input { axes: [0.0, 0.0], trigger: 4 });
    assert_eq!(r.remaining.to_bits(), 0x4295_3333); // 74.5999984741211
    assert_eq!((r.stick_block, r.other_block, r.accepted), (4, 3, 4));
    r.step(Input { axes: [1.0, 0.0], trigger: 0 });
    assert_eq!(r.sign, [0, 0], "blocked stick never overwrites the accepted signs");
    r.step(Input { axes: [0.0, 0.0], trigger: 4 });
    assert_eq!(r.remaining.to_bits(), 0x4268_cccc); // 58.19999694824219
    let mut stick = record(90.0);
    stick.step(Input { axes: [1.0, 1.0], trigger: 0x22e });
    assert_eq!(stick.remaining, 81.0, "diagonal once; stick precedes all buttons");
    assert_eq!(stick.button_block, 4);
    stick.step(Input { axes: [0.5, -0.5], trigger: 0 });
    assert_eq!(stick.remaining, 80.0);
    assert_eq!(stick.sign, [1, 1], "threshold endpoints retain the last signs");
}

#[test]
fn native_threshold_boundaries_are_strict() {
    for threshold in [0.0_f32, 0.5, 0.9] {
        let mut r = record(90.0);
        r.threshold = threshold;
        r.sign = [-1, 1];
        r.step(Input { axes: [threshold, -threshold], trigger: 0 });
        assert_eq!(r.sign, [-1, 1], "inclusive middle interval at {threshold}");
        assert_eq!(r.remaining, 89.0, "endpoints subtract automatic only");
        let outside = threshold.next_up();
        r.step(Input { axes: [outside, -outside], trigger: 0 });
        assert_eq!(r.sign, [1, -1], "one ULP crosses both strict boundaries");
        assert_eq!(r.remaining, 80.0, "diagonal crossing subtracts manual once");
    }
    for threshold in [1.0, 1.25, f32::MAX] {
        let mut r = record(90.0);
        r.threshold = threshold;
        r.sign = [-1, 1];
        r.step(Input { axes: [1.0, -1.0], trigger: 0 });
        assert_eq!((r.sign, r.remaining), ([-1, 1], 89.0));
    }
}

#[test]
fn native_threshold_invalid_values_are_rejected() {
    for threshold in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.1] {
        let mut s = seed(90.0);
        s.record.threshold = threshold;
        assert!(!s.record.valid());
        assert!(matches!(Solver::new(s), Err(Error::InvalidSeed)));
        assert!(matches!(Clock::new(s.record), Err(ClockError::InvalidRecord)));
    }
}

#[test]
fn native_no_input_uses_float_steps() {
    let mut s = seed(0.3);
    s.record.automatic = 0.1;
    s.record.manual = 0.0;
    assert_eq!((0.3_f32 / 0.1_f32).ceil(), 3.0);
    assert_eq!(solve(s), Ok(Bounds { fastest: 4, no_input: Some(4) }));
}

#[test]
fn native_solver_preserves_seed_latches() {
    assert_eq!(solve(seed(10.0)).unwrap().fastest, 1);
    let mut held = seed(10.0);
    held.held = BUTTONS;
    assert_eq!(solve(held).unwrap().fastest, 2);
    let mut blocked = seed(10.0);
    blocked.record.stick_block = 4;
    blocked.record.button_block = 4;
    blocked.record.other_block = 3;
    blocked.record.accepted = 3;
    assert_eq!(solve(blocked).unwrap().fastest, 4);
    let ordinary = solve(seed(90.0)).unwrap();
    assert_eq!(ordinary, Bounds { fastest: 10, no_input: Some(90) });
}

#[test]
fn native_deadline_and_zero_saving() {
    let mut infinite = seed(90.0);
    infinite.record.automatic = 0.0;
    let bounds = solve(infinite).unwrap();
    assert_eq!(bounds.no_input, None);
    infinite.deadline = Some(80);
    assert_eq!(solve(infinite).unwrap().no_input, Some(80));
    infinite.deadline = Some(2);
    assert_eq!(solve(infinite), Ok(Bounds { fastest: 2, no_input: Some(2) }));
    infinite.record.manual = 0.0;
    assert_eq!(solve(infinite), Ok(Bounds { fastest: 2, no_input: Some(2) }));
    infinite.deadline = None;
    assert_eq!(solve(infinite), Err(Error::NeverEscapes));
    let mut automatic = seed(90.0);
    automatic.record.manual = 0.0;
    assert_eq!(solve(automatic), Ok(Bounds { fastest: 90, no_input: Some(90) }));
}

#[test]
fn native_invalid_seed_never_gets_a_fallback() {
    for value in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
        assert!(matches!(Solver::new(seed(value)), Err(Error::InvalidSeed)));
    }
    let mut invalid = seed(90.0);
    invalid.record.cross_reload = 5;
    assert!(matches!(Solver::new(invalid), Err(Error::InvalidSeed)));
    invalid = seed(90.0);
    invalid.held = 0x10;
    assert!(matches!(Solver::new(invalid), Err(Error::InvalidSeed)));
}

/// Unreduced reference search: every full five-category held mask and every X direction.
/// It retains the actual native latches, accepted mask and previous held categories. This
/// independently challenges the production solver's 400-state category reduction.
fn oracle(seed: Seed, limit: u32) -> Option<u32> {
    let mut layer = vec![(seed.record, seed.held)];
    for tick in 1..=limit {
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
        if seed.deadline == Some(tick) { return Some(tick); }
        layer = next.into_values().collect();
    }
    None
}

#[test]
fn native_solver_matches_full_mask_oracle() {
    let mut cases = vec![seed(1.0), seed(14.5), seed(27.2), seed(34.4)];
    let mut latched = seed(27.2);
    latched.held = BUTTONS;
    latched.record.stick_block = 2;
    latched.record.other_block = 3;
    latched.record.accepted = 0x25;
    cases.push(latched);
    latched.held = 0x21;
    latched.record.button_block = 2;
    cases.push(latched);
    for case in cases {
        let bounds = solve(case).unwrap();
        assert_eq!(oracle(case, bounds.fastest), Some(bounds.fastest), "{case:?}");
    }
}

#[test]
fn native_threshold_solver_matches_full_mask_oracle() {
    for threshold in [0.0_f32, 0.5, 0.9, 1.0_f32.next_down(), 1.0, 1.25] {
        // No useful button deduction: the only acceleration comes from a reachable stick edge.
        let mut s = seed(5.0);
        s.record.threshold = threshold;
        s.record.button_mul = 0.0;
        let bounds = solve(s).unwrap();
        let expected = if threshold < 1.0 { 1 } else { 5 };
        assert_eq!(bounds, Bounds { fastest: expected, no_input: Some(5) });
        assert_eq!(oracle(s, 5), Some(expected), "stick-only {threshold}");
        // At/above 1, buttons still work after releasing the initially held categories.
        s.record.button_mul = 1.8;
        s.held = BUTTONS;
        let expected = if threshold < 1.0 { 1 } else { 2 };
        assert_eq!(solve(s).unwrap().fastest, expected);
        assert_eq!(oracle(s, 2), Some(expected), "held buttons {threshold}");
    }
}

#[test]
fn native_threshold_blocks_unreachable_zero_auto_sticks() {
    for threshold in [1.0, 1.25] {
        let mut s = seed(8.0);
        s.record.threshold = threshold;
        s.record.automatic = 0.0;
        s.record.button_mul = 0.0;
        assert_eq!(oracle(s, 3), None, "full normalized axes never cross {threshold}");
        assert_eq!(solve(s), Err(Error::NeverEscapes));
        s.deadline = Some(3);
        assert_eq!(solve(s), Ok(Bounds { fastest: 3, no_input: Some(3) }));
        s.record.threshold = 0.9;
        assert_eq!(solve(s), Ok(Bounds { fastest: 1, no_input: Some(3) }));
    }
}

#[test]
fn native_solver_work_chunks_do_not_change_the_answer() {
    let s = seed(180.1);
    let expected = solve(s).unwrap();
    for work in [1, 2, 7, 64] {
        let mut solver = Solver::new(s).unwrap();
        assert_eq!(solver.advance(0), None);
        let actual = loop {
            if let Some(answer) = solver.advance(work) { break answer.unwrap(); }
        };
        assert_eq!(actual, expected);
        assert_eq!(solver.advance(1), Some(Ok(expected)));
    }
}

#[test]
fn native_clock_matches_ticks_with_stops() {
    let mut live = record(90.0);
    let mut clock = Clock::new(live).unwrap();
    let mut native_ticks = 0;
    for frame in 0..50 {
        let stopped = frame % 4 == 0;
        let input = Input { axes: [if native_ticks % 2 == 0 { 1.0 } else { -1.0 }, 0.0], trigger: 0 };
        if !stopped {
            live.step(input);
            native_ticks += 1;
        }
        clock.observe(live, input, false).unwrap();
        assert_eq!(clock.elapsed(), native_ticks);
        if live.remaining <= 0.0 { break; }
    }
    assert_eq!(native_ticks, 10);
    assert!(clock.exhausted());
    clock.observe(live, Input::default(), true).unwrap();
    assert_eq!(clock.elapsed(), 10, "escape animation adds no tick");
}

#[test]
fn native_clock_rejects_two_updates_per_sample() {
    let mut live = record(90.0);
    let mut clock = Clock::new(live).unwrap();
    live.step(Input::default());
    live.step(Input::default());
    assert_eq!(clock.observe(live, Input::default(), false), Err(ClockError::TransitionMismatch));
    let mut clock = Clock::new(record(90.0)).unwrap();
    let mut changed = record(91.0);
    assert_eq!(clock.observe(changed, Input::default(), false), Err(ClockError::Reinitialized));
    changed = record(90.0);
    changed.accepted = 4;
    assert_eq!(clock.observe(changed, Input::default(), false), Err(ClockError::Ambiguous));
}

#[test]
fn native_clock_uses_actual_threshold_and_binds_changes() {
    let mut initial = record(20.0);
    initial.threshold = 0.9;
    let mut clock = Clock::new(initial).unwrap();
    let input = Input { axes: [0.75, 0.0], trigger: 0 };
    // Independent expected native record: 0.75 is below the sampled 0.9 threshold.
    let mut live = initial;
    live.remaining = 19.0;
    clock.observe(live, input, false).unwrap();
    assert_eq!(clock.elapsed(), 1);
    let mut changed = live;
    changed.threshold = 0.5;
    assert_ne!(Seed { record: live, held: 0, deadline: None }, Seed { record: changed, held: 0, deadline: None });
    assert_eq!(clock.observe(changed, input, false), Err(ClockError::Ambiguous));
    changed.remaining = 18.0;
    assert_eq!(clock.observe(changed, input, false), Err(ClockError::TransitionMismatch));

    initial.threshold = 0.0;
    let mut clock = Clock::new(initial).unwrap();
    let mut live = initial;
    live.remaining = 11.0;
    live.sign = [1, 0];
    live.button_block = 4;
    clock.observe(live, Input { axes: [0.25, 0.0], trigger: 0 }, false).unwrap();
    assert_eq!(clock.elapsed(), 1, "actual zero threshold accepts the small positive axis");
}

#[test]
fn native_zero_auto_without_input_is_not_observable_from_decreases() {
    let mut live = record(90.0);
    live.automatic = 0.0;
    let initial = live;
    let mut clock = Clock::new(live).unwrap();
    live.step(Input::default());
    live.step(Input::default());
    assert_eq!(live, initial, "two native no-input ticks and no native tick have the same record");
    clock.observe(live, Input::default(), false).unwrap();
    assert_eq!(clock.elapsed(), 0);
}
}

#[test]
fn rebase_adopts_only_a_bit_equal_old() {
    // E-A rc15-frozen113-measure-xps-2: Set old 49.545795 -> new 83.052353.
    let mut clock = Clock::new(record(49.545_795)).unwrap();
    assert!(clock.rebase(49.545_795_f32.to_bits() + 1, 83.052_353_f32.to_bits()).is_err());
    assert!(clock.rebase(49.545_795_f32.to_bits(), 83.052_353_f32.to_bits()).is_ok());
    assert_eq!(clock.last().remaining, 83.052_353);
    assert!(clock.rebase(83.052_353_f32.to_bits(), 0.0_f32.to_bits()).is_err());
}
