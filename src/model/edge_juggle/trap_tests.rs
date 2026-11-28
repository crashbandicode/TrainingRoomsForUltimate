lib_unit_tests! {
use super::*;
use super::trap::{self, Rules, Trap};

fn ground() -> Observation {
    Observation { grounded: true, on_run: true, ..Observation::default() }
}

fn live(rules: Rules) -> Session {
    let mut session = Session::new(Drill::LedgeTrap, Some(10)).with_rules(rules);
    session.placed();
    for _ in 0..Drill::LedgeTrap.ready_frames() {
        session.advance_attempt(Observation::default());
    }
    assert_eq!(session.phase, Phase::Live);
    session
}

fn stage() -> Stage {
    Stage::new(
        [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
        [[-80.0, 0.0], [80.0, 0.0]],
    ).unwrap()
}

#[test]
fn trap_fails_when_the_cpu_stands_for_the_fail_time() {
    for fail in [120, 180] {
        let rules = Rules { fail, time: None };
        let mut trap = Trap::new(rules);
        for frame in 1..u32::from(fail) {
            assert_eq!(trap.step(frame, false, ground()), None);
        }
        assert_eq!(trap.step(u32::from(fail), false, ground()), Some(Outcome::OnStage));
    }
    let mut session = live(Rules::DEFAULT);
    for _ in 0..120 { session.advance_attempt(ground()); }
    assert_eq!((session.attempts, session.secured, session.streak), (1, 0, 0));
    assert_eq!(session.phase.card(), "On stage");
    assert_eq!(session.practice_fields(None, None).last_outcome, "on_stage");
}

#[test]
fn trap_clock_holds_in_the_air_and_clears_off_stage() {
    let mut trap = Trap::new(Rules::DEFAULT);
    for frame in 1..=48 { assert_eq!(trap.step(frame, false, ground()), None); }
    assert_eq!(trap.clock(), 48);
    for frame in 49..=78 {
        assert_eq!(trap.step(frame, false, Observation { on_run: true, ..Observation::default() }), None);
    }
    assert_eq!(trap.clock(), 48);
    for frame in 79..=102 { assert_eq!(trap.step(frame, false, ground()), None); }
    assert_eq!(trap.clock(), 72);
    trap.step(103, false, Observation::default());
    assert_eq!(trap.clock(), 0);
    trap.step(104, false, Observation { grounded: true, ..Observation::default() });
    assert_eq!(trap.clock(), 0);
    trap.step(105, false, Observation { hanging: true, on_run: true, ..Observation::default() });
    assert_eq!(trap.clock(), 0);
}

#[test]
fn a_grab_inside_the_window_clears_the_clock() {
    let mut session = live(Rules::DEFAULT);
    for _ in 0..78 { session.advance_attempt(ground()); }
    assert_eq!(session.trap().clock(), 78);
    session.advance_attempt(Observation { grabbed: true, ..ground() });
    assert_eq!((session.trap().clock(), session.hits), (0, 1));
    session.advance_attempt(Observation { on_run: true, ..Observation::default() });
    assert_eq!(session.trap().clock(), 0);
    for _ in 0..119 { session.advance_attempt(ground()); }
    assert_eq!(session.phase, Phase::Live);
    session.advance_attempt(ground());
    assert!(matches!(session.phase, Phase::Result(Outcome::OnStage, _)));
}

#[test]
fn trap_ko_and_round_time_count_as_kept_off() {
    let mut session = live(Rules::DEFAULT);
    session.advance_attempt(Observation { hit: true, ..Observation::default() });
    session.advance_attempt(Observation { dead_or_rebirthing: true, ..Observation::default() });
    assert!(matches!(session.phase, Phase::Result(Outcome::KeptOffKo, _)));
    assert_eq!((session.attempts, session.secured, session.streak), (1, 1, 1));
    assert_eq!(session.record_result(&mut [None; 2], 1u8), Some(2));

    let mut unresolved = live(Rules::DEFAULT);
    unresolved.advance_attempt(Observation { dead_or_rebirthing: true, ..Observation::default() });
    assert!(matches!(unresolved.phase, Phase::Result(Outcome::Unresolved, _)));
    assert_eq!(unresolved.attempts, 0);

    let hang = Observation { hanging: true, ..Observation::default() };
    let mut timed = Trap::new(Rules::DEFAULT);
    for frame in 1..1800 { assert_eq!(timed.step(frame, false, hang), None); }
    assert_eq!(timed.step(1800, false, hang), Some(Outcome::KeptOffTime));
    let mut timed_session = live(Rules::DEFAULT);
    for _ in 0..1800 { timed_session.advance_attempt(hang); }
    assert!(matches!(timed_session.phase, Phase::Result(Outcome::KeptOffTime, _)));
    assert_eq!((timed_session.attempts, timed_session.secured, timed_session.streak), (1, 1, 1));
    let mut endless = Trap::new(Rules { fail: 120, time: None });
    for frame in 1..=3600 { assert_eq!(endless.step(frame, false, hang), None); }
    let mut simultaneous = Trap::new(Rules { fail: 120, time: Some(120) });
    for frame in 1..120 { assert_eq!(simultaneous.step(frame, false, ground()), None); }
    assert_eq!(simultaneous.step(120, false, ground()), Some(Outcome::OnStage));
    assert_eq!(Trap::new(Rules::DEFAULT).step(1, false, Observation { player_ko: true, ..Observation::default() }), Some(Outcome::PlayerKo));
    let mut no_grab = Trap::new(Rules::DEFAULT);
    for frame in 1..120 { assert_eq!(no_grab.step(frame, false, Observation::default()), None); }
    assert_eq!(no_grab.step(120, false, Observation::default()), Some(Outcome::Unresolved));
}

#[test]
fn trap_rules_follow_the_two_rows() {
    for (index, fail) in [120, 150, 180].into_iter().enumerate() {
        assert_eq!(Rules::from_settings(index, 0).fail, fail);
    }
    for (index, time) in [Some(1200), Some(1800), Some(3600), None].into_iter().enumerate() {
        assert_eq!(Rules::from_settings(0, index).time, time);
    }
    assert_eq!(Rules::from_settings(usize::MAX, usize::MAX), Rules { fail: 180, time: None });
}

#[test]
fn trap_pad_drifts_in_then_goes_neutral_on_the_ledge() {
    let mut trap = Trap::new(Rules::DEFAULT);
    let buttons = perfect::FLICK_JUMP | perfect::ATTACK | perfect::SPECIAL;
    let drift = trap.pad(false, -1.0, buttons);
    assert_eq!((drift.stick, drift.release), (Some([-1.0, 0.0]), buttons & !perfect::FLICK_JUMP));
    let hang = trap.pad(true, 1.0, buttons);
    assert_eq!((hang.stick, hang.release), (Some([0.0, 0.0]), buttons & !perfect::FLICK_JUMP));
    trap.step(1, false, Observation { hanging: true, ..Observation::default() });
    assert_eq!(trap.pad(false, 1.0, buttons), perfect::Correction::default());
}

#[test]
fn trap_wizard_opens_on_a_ledge_and_y_takes_the_other() {
    let stage = stage();
    let mut wizard = Wizard::new(Drill::LedgeTrap, stage, [0.0; 2], 60, (3, 0), None).unwrap();
    assert_eq!(wizard.step, WizardStep::Cpu);
    assert_eq!(wizard.setup.cpu, [-88.0, 0.0]);
    wizard.steer(stage, &btt::Pad { stick: [1.0, 0.0], ..btt::Pad::default() });
    assert_eq!(wizard.setup.cpu, [-88.0, 0.0]);
    wizard.set_side_default(stage, true);
    assert_eq!(wizard.setup.cpu, [88.0, 0.0]);
    for place in [geometry::Place::Outside, geometry::Place::Solid, geometry::Place::OverStage, geometry::Place::Open, geometry::Place::Under] {
        assert!(wizard.takes(place, StartOptions::default()));
    }
    assert_eq!(wizard.confirm_start(wizard.setup.cpu, Ok(geometry::Place::Open), StartOptions::default()), Some(true));
    assert_eq!(wizard.step, WizardStep::Player);
    wizard.advance();
    assert_eq!(wizard.step, WizardStep::Preview);
    assert_eq!(wizard.kept_setup(stage, wizard.kept(stage), StartOptions::default()), Some(wizard.setup));
}

#[test]
fn trap_player_start_takes_a_floor_or_open_air() {
    let at = [100.0, 25.0];
    let floor = [100.0, 0.0];
    assert_eq!(trap::player_start(Some(floor), Ok(geometry::Place::Under), at), Some(floor));
    assert_eq!(trap::player_start(None, Ok(geometry::Place::Open), at), Some(at));
    for place in [geometry::Place::Under, geometry::Place::Solid, geometry::Place::Outside] {
        assert_eq!(trap::player_start(None, Ok(place), at), None);
    }
    assert_eq!(trap::player_start(None, Err(geometry::Unread), at), None);
    assert_eq!(stage().player_cursor(Drill::LedgeTrap, at), Some(at));
}

/// Eden boot 5, row 4: a smash at 300% landed with 4 frames left and the attempt was graded On stage.
#[test]
fn a_ko_hit_with_four_frames_left_is_kept_off_not_on_stage() {
    let mut session = live(Rules::DEFAULT);
    for _ in 0..116 { session.advance_attempt(ground()); }
    assert_eq!((session.trap().clock(), session.phase), (116, Phase::Live));
    // The hit: the CPU is frozen on the ground for the hit's own frames, then flies in hitstun, then is KO'd.
    session.advance_attempt(Observation { hit: true, stunned: true, ..ground() });
    for _ in 0..24 { session.advance_attempt(Observation { stunned: true, ..ground() }); }
    assert_eq!((session.trap().clock(), session.phase, session.hits), (116, Phase::Live, 1), "hitlag holds the clock");
    for _ in 0..40 { session.advance_attempt(Observation { stunned: true, on_run: true, ..Observation::default() }); }
    session.advance_attempt(Observation { dead_or_rebirthing: true, ..Observation::default() });
    assert!(matches!(session.phase, Phase::Result(Outcome::KeptOffKo, _)));
    assert_eq!((session.attempts, session.secured, session.streak), (1, 1, 1));
    assert_eq!(session.phase.card(), "Kept off · KO");
}

#[test]
fn a_hit_holds_the_clock_but_does_not_clear_it() {
    let mut trap = Trap::new(Rules::DEFAULT);
    for frame in 1..=60 { assert_eq!(trap.step(frame, false, ground()), None); }
    // A jab: ten frames of hitlag and grounded hitstun. Held, not cleared; only a grab clears.
    for frame in 61..=70 { assert_eq!(trap.step(frame, true, Observation { stunned: true, ..ground() }), None); }
    assert_eq!(trap.clock(), 60);
    for frame in 71..=129 { assert_eq!(trap.step(frame, true, ground()), None); }
    assert_eq!(trap.step(130, true, ground()), Some(Outcome::OnStage));
}
}
