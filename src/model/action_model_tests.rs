//! Acceptance tests for the ACTION model (ruled 2026-11-24 15:01Z): an action is an originating input plus the game
//! state chain it caused, and it owns its release, its hold, its aims and its travel. Idle time only separates
//! unrelated inputs.
//!
//! EXPECTED RED on `e7667f26`: the three tests marked so fail on their assertions with today's recorder and grader,
//! which slice raw inputs by 20 idle frames. The test marked GUARD passes today and must keep passing.
//! The set-up uses today's seams (`InputCapture`, `Helper`, `Tracker`); the assertions are the contract.
lib_unit_tests! {
use super::*;
use crate::model::aim::Seen;
use crate::model::frame_strip::Sample;
use crate::model::input_delay::MappedInputs;

const ATTACK: u32 = 0x1;
const SPECIAL: u32 = 0x2;
/// L, R and A as the game maps them: the Training reset chord.
const RESET_CHORD: u32 = 0x8 | 0x1;

fn snap(frame: u32, buttons: u32) -> Snap {
    Snap { buttons, facing: 1, ..Snap::empty(frame) }
}

/// A drill recorded from button states at the listed frames (today's recorder).
fn recorded(script: &[(u32, u32)]) -> Combo {
    let mut input = InputCapture::new();
    for &(frame, buttons) in script {
        input.step(snap(frame, buttons));
    }
    input.combo
}

fn standing() -> Sample {
    Sample { actionable: true, facing: 1, ..Sample::default() }
}

/// Player 1 in a jab the A press started: the state chain of the cast.
fn jabbing(frame: u32) -> Sample {
    Sample { actionable: false, attack: Some(Move::Jab), status: 39, motion_frame: frame as f32, facing: 1, ..Sample::default() }
}

/// One game frame through the strip's tracker and then the helper, as `combo::after_strip` does.
fn game_frame(tracker: &mut Tracker, helper: &mut Helper, p1: Sample, buttons: u32) {
    tracker.step(p1, standing(), MappedInputs { buttons, ..MappedInputs::NEUTRAL });
    let _out = helper.step(tracker, Settings::factory());
}

/// EXPECTED RED. Flown on install 3 (`mod-inputs-yoga-01`, 2026-11-24): after L+R+A the panel read `cleared` through
/// the whole cast and then `ready`; the cast was never graded. The chord starts no move, so it is no action and
/// opens no attempt; the cast after it is graded as a fresh attempt.
#[test]
fn a_cast_after_a_training_reset_is_graded() {
    let drill = recorded(&[(0, ATTACK), (12, 0)]);
    assert_eq!(drill.len(), 1, "set-up: one step, A held 12 frames");
    let mut tracker = Tracker::new();
    let mut helper = Helper::new();
    helper.set_target(Some(drill));
    for _ in 0..10 {
        game_frame(&mut tracker, &mut helper, standing(), 0);
    }
    for _ in 0..4 {
        game_frame(&mut tracker, &mut helper, standing(), RESET_CHORD);
    }
    for _ in 0..4 {
        game_frame(&mut tracker, &mut helper, standing(), 0);
    }
    // The game initialises the fighters again: the strip clears (`input_display::fighter_initialised`).
    tracker.clear();
    for _ in 0..60 {
        game_frame(&mut tracker, &mut helper, standing(), 0);
    }
    for frame in 0..12 {
        game_frame(&mut tracker, &mut helper, jabbing(frame), ATTACK);
    }
    for frame in 12..20 {
        game_frame(&mut tracker, &mut helper, jabbing(frame), 0);
    }
    for _ in 0..40 {
        game_frame(&mut tracker, &mut helper, standing(), 0);
    }
    let results: Vec<_> = helper.results().map(|s| (s.outcome, s.clean)).collect();
    assert_eq!(results, vec![(Outcome::Completed, true)], "the reset chord leaves no result and the cast is graded clean");
    assert_eq!((helper.streak(), helper.best()), (1, 1));
}

/// Sheik's Vanish as flown: B with the stick up, released on frame 3, the stick held up-left; the travel's direction
/// is known on frame 37, more than 20 idle frames after the release.
fn vanish(frame: u32) -> Snap {
    let known = frame == 37;
    Snap {
        buttons: if frame < 3 { SPECIAL } else { 0 },
        lstick_x: if frame < 3 { 0 } else { -90 },
        lstick_y: if frame < 3 { 127 } else { 90 },
        facing: 1,
        aim: Seen { inside: frame <= 60, part: u8::from(known), dir: known.then_some(3), facing: 1, age: if known { 3 } else { 0 } },
        ..Snap::empty(frame)
    }
}

/// EXPECTED RED. Flown on install 3: Sheik, Fox, Falco and Wolf saved `S1` = B press and release and `S2` = the
/// 1st direction. The first aim belongs to the special that produced it, however long its start-up is.
#[test]
fn a_specials_first_aim_stays_in_its_step_however_late() {
    let mut input = InputCapture::new();
    for frame in 0..=70 {
        input.step(vanish(frame));
    }
    let drill = input.combo;
    assert_eq!(drill.len(), 1, "one special with one direction is one step");
    let step = drill.get(0).unwrap();
    assert_eq!(
        step.events().map(|e| e.kind).collect::<Vec<_>>(),
        vec![Kind::Press(Button::Special), Kind::Release(Button::Special), Kind::Aim(1)]
    );
    assert_eq!(step.events().last().map(|e| e.dir), Some(Dir::BackUp));
    let lines = crate::model::recording_menu::combo_link_lines(&drill);
    assert_eq!(lines.len(), 2, "the list shows one step and its one event line: {lines:?}");
    assert_eq!(lines[0], "S1 Step 1");
}

/// Pikachu's Quick Attack as flown: B with the stick up, then left, then right; the first zip's direction is known on
/// frame 16 and the second's on frame 30 (the script of `aim_steps_tests`).
fn two_zips(frame: u32) -> Snap {
    let (part, dir) = match frame {
        16 => (1, Some(4)),
        30 => (2, Some(0)),
        _ => (0, None),
    };
    Snap {
        buttons: if frame < 3 { SPECIAL } else { 0 },
        lstick_x: if frame < 3 { 0 } else if frame < 20 { -127 } else { 127 },
        lstick_y: if frame < 3 { 127 } else { 0 },
        facing: 1,
        aim: Seen { inside: frame <= 35, part, dir, facing: 1, age: u8::from(part != 0) },
        ..Snap::empty(frame)
    }
}

/// GUARD, green on `e7667f26`. A later aim part keeps its designed step: Pikachu's two-zip drill saves as it does
/// now, `S1` = B press, release and the 1st direction, `S2` = the 2nd direction.
#[test]
fn a_second_aim_part_keeps_its_own_step() {
    let mut input = InputCapture::new();
    for frame in 0..=42 {
        input.step(two_zips(frame));
    }
    let drill = input.combo;
    assert_eq!(drill.len(), 2);
    let events = |i: usize| drill.get(i).unwrap().events().map(|e| (e.kind, e.dir)).collect::<Vec<_>>();
    assert_eq!(
        events(0),
        vec![(Kind::Press(Button::Special), Dir::Up), (Kind::Release(Button::Special), Dir::Back), (Kind::Aim(1), Dir::Back)]
    );
    assert_eq!(events(1), vec![(Kind::Aim(2), Dir::Forward)]);
}

/// EXPECTED RED. A button held longer than 20 frames: its release belongs to the action its press began, so the
/// drill is one step and the same hold grades clean as one step. Today the release starts a second step
/// (`input_drill_tests::input_idle_groups_and_continuations_share_combo_bounds` pins that for a 21-frame hold).
#[test]
fn a_held_buttons_release_stays_in_the_step_of_its_press() {
    let script = [(0, ATTACK), (30, 0)];
    let drill = recorded(&script);
    assert_eq!(drill.len(), 1, "a press and its own release are one step");
    assert_eq!(
        drill.get(0).unwrap().events().map(|e| (e.kind, e.offset)).collect::<Vec<_>>(),
        vec![(Kind::Press(Button::Attack), 0), (Kind::Release(Button::Attack), 30)]
    );
    let mut helper = Helper::new();
    helper.set_target(Some(drill));
    let mut buttons = 0;
    for frame in 0..=55 {
        if let Some(&(_, now)) = script.iter().find(|(at, _)| *at == frame) {
            buttons = now;
        }
        helper.settings = Settings::factory();
        helper.step_inputs(snap(frame, buttons), &mut Out::default());
    }
    let result = *helper.results().next().expect("the hold resolved");
    assert!(result.clean, "the same hold again is clean: {result:?}");
    assert_eq!((result.graded, helper.streak(), helper.best()), (1, 1, 1));
}

}
