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

#[test]
fn buttons_held_through_a_clear_are_not_presses() {
    let mut tracker = Tracker::new();
    let mut helper = Helper::new();
    helper.set_target(Some(recorded(&[(0, ATTACK), (12, 0)])));
    for _ in 0..10 {
        game_frame(&mut tracker, &mut helper, standing(), 0);
    }
    for _ in 0..2 {
        game_frame(&mut tracker, &mut helper, standing(), RESET_CHORD);
    }
    tracker.clear();
    for _ in 0..6 {
        game_frame(&mut tracker, &mut helper, standing(), RESET_CHORD);
        assert!(tracker.presses().next().is_none());
        assert!(helper.attempt().is_none());
    }
    for _ in 0..4 {
        game_frame(&mut tracker, &mut helper, standing(), 0);
        assert!(helper.attempt().is_none());
    }
    assert!(helper.results().next().is_none());
}

#[test]
fn a_release_without_its_press_is_not_an_event() {
    let mut input = InputCapture::new();
    input.step(snap(0, ATTACK));
    input.restart();
    input.step(snap(5, ATTACK));
    input.step(snap(9, 0));
    assert!(input.combo.is_empty());
    input.step(snap(20, SPECIAL));
    input.step(snap(23, 0));
    assert_eq!(
        input.combo.get(0).unwrap().events().map(|e| (e.kind, e.offset)).collect::<Vec<_>>(),
        vec![(Kind::Press(Button::Special), 0), (Kind::Release(Button::Special), 3)]
    );
}

#[test]
fn a_button_held_when_a_take_starts_is_a_press() {
    let drill = recorded(&[(0, ATTACK), (3, 0)]);
    assert_eq!(drill.len(), 1);
    assert_eq!(
        drill.get(0).unwrap().events().map(|e| (e.kind, e.offset)).collect::<Vec<_>>(),
        vec![(Kind::Press(Button::Attack), 0), (Kind::Release(Button::Attack), 3)]
    );
}

#[test]
fn a_clear_mid_attempt_is_cleared_and_the_next_cast_is_graded() {
    let drill = recorded(&[(0, ATTACK), (2, 0), (30, SPECIAL), (32, 0)]);
    assert_eq!(drill.len(), 2);
    let mut tracker = Tracker::new();
    let mut helper = Helper::new();
    helper.set_target(Some(drill));
    for _ in 0..10 {
        game_frame(&mut tracker, &mut helper, standing(), 0);
    }
    for _ in 0..2 {
        game_frame(&mut tracker, &mut helper, standing(), ATTACK);
    }
    for _ in 0..6 {
        game_frame(&mut tracker, &mut helper, standing(), 0);
    }
    assert!(helper.attempt().is_some_and(|a| a.open() && a.len() == 1));
    tracker.clear();
    for _ in 0..60 {
        game_frame(&mut tracker, &mut helper, standing(), 0);
    }
    for _ in 0..2 {
        game_frame(&mut tracker, &mut helper, standing(), ATTACK);
    }
    for _ in 0..28 {
        game_frame(&mut tracker, &mut helper, standing(), 0);
    }
    for _ in 0..2 {
        game_frame(&mut tracker, &mut helper, standing(), SPECIAL);
    }
    for _ in 0..12 {
        game_frame(&mut tracker, &mut helper, standing(), 0);
    }
    assert_eq!(
        helper.results().map(|s| (s.outcome, s.clean)).collect::<Vec<_>>(),
        vec![(Outcome::Completed, true), (Outcome::Cleared, false)]
    );
    assert_eq!(helper.streak(), 1);
}

#[test]
fn unrelated_inputs_still_split_on_idle() {
    let long = recorded(&[(0, ATTACK), (60, 0), (65, SPECIAL), (67, 0)]);
    assert_eq!(long.len(), 1);
    assert_eq!(
        long.get(0).unwrap().events().map(|e| e.kind).collect::<Vec<_>>(),
        vec![Kind::Press(Button::Attack), Kind::Release(Button::Attack), Kind::Press(Button::Special), Kind::Release(Button::Special)]
    );
    assert_eq!(recorded(&[(0, ATTACK), (2, 0), (30, SPECIAL), (32, 0)]).len(), 2);
    let overlap = recorded(&[(0, ATTACK), (40, ATTACK | SPECIAL), (50, SPECIAL)]);
    assert_eq!(overlap.len(), 2);
    assert_eq!(
        overlap.get(0).unwrap().events().map(|e| (e.kind, e.offset)).collect::<Vec<_>>(),
        vec![(Kind::Press(Button::Attack), 0)]
    );
    assert_eq!(
        overlap.get(1).unwrap().events().map(|e| (e.kind, e.offset)).collect::<Vec<_>>(),
        vec![(Kind::Press(Button::Special), 40), (Kind::Release(Button::Attack), 50)]
    );
}

#[test]
fn a_release_has_no_direction_to_grade() {
    let press = Snap { buttons: SPECIAL, lstick_y: 127, facing: 1, ..Snap::empty(0) };
    let release = Snap { lstick_x: -90, facing: 1, ..Snap::empty(3) };
    let mut input = InputCapture::new();
    input.step(press);
    input.step(release);
    let reference = input.combo;
    assert_eq!(
        reference.get(0).unwrap().events().map(|e| (e.kind, e.dir)).collect::<Vec<_>>(),
        vec![(Kind::Press(Button::Special), Dir::Up), (Kind::Release(Button::Special), Dir::Back)]
    );
    let mut helper = Helper::new();
    helper.set_target(Some(reference));
    let mut actual = InputCapture::new();
    for frame in 0..=10 {
        let now = if frame < 3 {
            Snap { frame, ..press }
        } else {
            Snap { frame, lstick_x: 90, facing: 1, ..Snap::empty(frame) }
        };
        actual.step(now);
        helper.settings = Settings::factory();
        helper.step_inputs(now, &mut Out::default());
    }
    let result = helper.results().next().unwrap();
    assert!(result.clean);
    assert_eq!(result.graded, 1);
    assert!(grade_mode(reference.get(0).unwrap(), actual.combo.get(0).unwrap(), true, false).wrong_dir().is_none());
}

#[test]
fn a_special_that_owns_aims_is_graded_by_its_aims_not_its_stick() {
    let mut input = InputCapture::new();
    for frame in 0..=70 {
        input.step(vanish(frame));
    }
    let mut helper = Helper::new();
    helper.set_target(Some(input.combo));
    for frame in 0..=70 {
        let now = Snap { lstick_x: -90, lstick_y: 90, ..vanish(frame) };
        helper.settings = Settings::factory();
        helper.step_inputs(now, &mut Out::default());
    }
    assert!(helper.results().next().unwrap().clean);

    let mut input = InputCapture::new();
    input.step(Snap { buttons: SPECIAL, lstick_y: 127, facing: 1, ..Snap::empty(0) });
    input.step(snap(3, 0));
    let mut helper = Helper::new();
    helper.set_target(Some(input.combo));
    for frame in 0..=10 {
        let now = if frame < 3 {
            Snap { buttons: SPECIAL, lstick_x: -90, lstick_y: 90, facing: 1, ..Snap::empty(frame) }
        } else {
            snap(frame, 0)
        };
        helper.settings = Settings::factory();
        helper.step_inputs(now, &mut Out::default());
    }
    assert_eq!(helper.results().next().unwrap().fault, Some((0, Reason::WrongDir(Kind::Press(Button::Special)))));
}

fn run_drill(reference: Combo, script: &[(u32, u32)], end: u32) -> Summary {
    let mut helper = Helper::new();
    helper.set_target(Some(reference));
    let mut buttons = 0;
    for frame in 0..=end {
        if let Some(&(_, now)) = script.iter().find(|(at, _)| *at == frame) {
            buttons = now;
        }
        helper.settings = Settings::factory();
        helper.step_inputs(snap(frame, buttons), &mut Out::default());
    }
    let result = *helper.results().next().expect("the drill resolved");
    result
}

#[test]
fn a_hold_is_graded_on_its_length_within_a_band() {
    let reference = recorded(&[(0, ATTACK), (64, 0)]);
    assert!(run_drill(reference, &[(0, ATTACK), (72, 0)], 110).clean);
    assert!(run_drill(reference, &[(0, ATTACK), (55, 0)], 110).clean);
    assert_eq!(run_drill(reference, &[(0, ATTACK), (74, 0)], 110).fault, Some((0, Reason::MissingRelease(Button::Attack))));
    assert_eq!(run_drill(reference, &[(0, ATTACK), (54, 0)], 110).fault, Some((0, Reason::ReleaseOff(Button::Attack, -10))));
    let reference = recorded(&[(0, SPECIAL), (2, 0), (10, ATTACK), (74, 0)]);
    assert_eq!(reference.len(), 1);
    assert!(run_drill(reference, &[(0, SPECIAL), (2, 0), (11, ATTACK), (83, 0)], 110).clean);
}

#[test]
fn a_short_hold_keeps_the_on_time_tolerance() {
    let reference = recorded(&[(0, ATTACK), (4, 0)]);
    assert!(run_drill(reference, &[(0, ATTACK), (5, 0)], 20).clean);
    assert_eq!(run_drill(reference, &[(0, ATTACK), (6, 0)], 20).fault, Some((0, Reason::MissingRelease(Button::Attack))));
    assert_eq!(run_drill(reference, &[(0, ATTACK), (2, 0)], 20).fault, Some((0, Reason::ReleaseOff(Button::Attack, -2))));
}

#[test]
fn a_reference_release_without_its_press_is_not_graded() {
    use crate::model::combo_record::ComboRecord;
    let full = recorded(&[(0, ATTACK), (30, ATTACK | SPECIAL), (40, SPECIAL), (42, 0)]);
    assert_eq!(full.len(), 2);
    let cropped = ComboRecord::new("Drill".into(), full, None).crop_units(1, 2).unwrap().combo;
    assert_eq!(
        cropped.get(0).unwrap().events().map(|e| (e.kind, e.offset)).collect::<Vec<_>>(),
        vec![(Kind::Press(Button::Special), 0), (Kind::Release(Button::Attack), 10), (Kind::Release(Button::Special), 12)]
    );
    assert!(run_drill(cropped, &[(0, SPECIAL), (12, 0)], 25).clean);
}

const SPLIT_VANISH: &str = r#"{"input_steps":[{"events":[{"button":"special","dir":"up","kind":"press","offset":0},{"button":"special","dir":"back_up","kind":"release","offset":3}]},{"events":[{"dir":"back_up","kind":"aim","offset":37,"part":1}]}],"kind":"input_drill","name":"Drill 1","schema_version":4}"#;
const SPLIT_HOLD: &str = r#"{"input_steps":[{"events":[{"button":"attack","dir":"neutral","kind":"press","offset":0}]},{"events":[{"button":"attack","dir":"neutral","kind":"release","offset":30}]}],"kind":"input_drill","name":"Drill 2","schema_version":2}"#;
const TWO_ZIPS: &str = r#"{"input_steps":[{"events":[{"button":"special","dir":"up","kind":"press","offset":0},{"button":"special","dir":"back","kind":"release","offset":3},{"dir":"back","kind":"aim","offset":16,"part":1}]},{"events":[{"dir":"forward","kind":"aim","offset":30,"part":2}]}],"kind":"input_drill","name":"Drill 3","schema_version":4}"#;

#[test]
fn a_split_step_file_reads_as_the_new_recorder_saves_it() {
    use crate::model::combo_record::ComboRecord;
    let parsed: ComboRecord = serde_json::from_str(SPLIT_VANISH).unwrap();
    let mut input = InputCapture::new();
    for frame in 0..=70 {
        input.step(vanish(frame));
    }
    assert_eq!(parsed.combo, input.combo);
    let parsed: ComboRecord = serde_json::from_str(SPLIT_HOLD).unwrap();
    assert_eq!(parsed.combo, recorded(&[(0, ATTACK), (30, 0)]));
    let parsed: ComboRecord = serde_json::from_str(TWO_ZIPS).unwrap();
    let mut input = InputCapture::new();
    for frame in 0..=42 {
        input.step(two_zips(frame));
    }
    assert_eq!(parsed.combo.len(), 2);
    assert_eq!(parsed.combo, input.combo);
}

#[test]
fn a_joined_file_is_written_back_unchanged() {
    use crate::model::combo_record::ComboRecord;
    for literal in [SPLIT_VANISH, SPLIT_HOLD] {
        let parsed: ComboRecord = serde_json::from_str(literal).unwrap();
        assert_eq!(serde_json::to_string(&parsed).unwrap(), literal);
    }
}

#[test]
fn a_release_records_its_hold_in_the_file() {
    use crate::model::combo_record::ComboRecord;
    let record = ComboRecord::new("Drill".into(), recorded(&[(0, ATTACK), (30, 0)]), None);
    let mut value = serde_json::to_value(&record).unwrap();
    assert_eq!(value["input_steps"][0]["events"][1]["hold"], 30);
    let read: ComboRecord = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(read, record);
    value["input_steps"][0]["events"][1]["hold"] = serde_json::json!("x");
    let error = serde_json::from_value::<ComboRecord>(value.clone()).unwrap_err();
    assert!(error.to_string().contains("hold"));
    value["input_steps"][0]["events"][1]["hold"] = serde_json::json!(7);
    let read: ComboRecord = serde_json::from_value(value).unwrap();
    assert_eq!(read, record);
}

}
