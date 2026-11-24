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
    let lines = crate::model::recording_menu::combo_link_lines(&drill, crate::model::move_names::Namer::OFF);
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
fn a_press_inside_the_tolerance_keeps_its_whole_hold_band() {
    let reference = recorded(&[(0, SPECIAL), (2, 0), (10, ATTACK), (74, 0)]);
    assert_eq!(reference.len(), 1);
    let attack = |press, release| run_drill(reference, &[(0, SPECIAL), (2, 0), (press, ATTACK), (release, 0)], 120);
    assert!(attack(10, 83).clean, "on time, the longest hold in band");
    assert!(attack(11, 84).clean, "one frame late, the same 73-frame hold");
    assert_eq!(attack(11, 85).fault, Some((0, Reason::MissingRelease(Button::Attack))));
    assert!(attack(9, 82).clean, "one frame early, the same hold");
    assert_eq!(attack(9, 83).fault, Some((0, Reason::MissingRelease(Button::Attack))));

    // The press in one step, its release in the next: Attack 10..124 (hold 114: band 14), Special tapped at 60..62.
    let reference = recorded(&[(0, SPECIAL), (2, 0), (10, ATTACK), (60, ATTACK | SPECIAL), (62, ATTACK), (124, 0)]);
    assert_eq!(reference.len(), 2);
    let across = |press, release| run_drill(reference,
        &[(0, SPECIAL), (2, 0), (press, ATTACK), (60, ATTACK | SPECIAL), (62, ATTACK), (release, 0)], 170);
    let whole = across(10, 139);
    assert!(whole.clean && whole.graded == 2);
    assert!(across(11, 140).clean);
    assert_eq!(across(11, 141).fault, Some((1, Reason::MissingRelease(Button::Attack))));
}

#[test]
fn a_repeated_button_uses_its_matching_press_for_the_hold_deadline() {
    let reference = recorded(&[(0, SPECIAL), (2, 0), (5, ATTACK), (7, 0), (10, ATTACK), (74, 0)]);
    assert_eq!(reference.len(), 1);
    assert!(run_drill(reference, &[(0, SPECIAL), (2, 0), (5, ATTACK), (7, 0), (11, ATTACK), (84, 0)], 120).clean);
    assert_eq!(
        run_drill(reference, &[(0, SPECIAL), (2, 0), (5, ATTACK), (7, 0), (11, ATTACK), (85, 0)], 120).fault,
        Some((0, Reason::MissingRelease(Button::Attack)))
    );
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
    assert_eq!(parsed.combo.len(), 1, "the aim is joined to the step of its special");
    assert_eq!(parsed.combo, input.combo);
    let parsed: ComboRecord = serde_json::from_str(SPLIT_HOLD).unwrap();
    assert_eq!(parsed.combo.len(), 1, "the release is joined to the step of its press");
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

fn hit_record(from: &str, to: &str, from_shown: Option<serde_json::Value>, to_shown: Option<serde_json::Value>) -> crate::model::combo_record::ComboRecord {
    let mut value = serde_json::json!({
        "schema_version": 1, "name": "Drill", "links": [{
            "from": from, "to": to, "verdict": {"kind": "true", "link": null},
            "window": null, "events": []
        }]
    });
    if let Some(shown) = from_shown { value["links"][0]["from_shown"] = shown; }
    if let Some(shown) = to_shown { value["links"][0]["to_shown"] = shown; }
    serde_json::from_value(value).unwrap()
}

#[test]
fn the_bar_and_the_menu_name_a_link_as_the_strip_does() {
    use crate::model::frame_strip::{Angle, Verdict};
    use crate::model::frame_strip_plan::{link_moves, Text, TEXT_UNITS};
    use crate::model::combo_plan::Renderer;
    use crate::model::panes::strip_layout::C;
    let mut tracker = Tracker::new();
    let stun = Sample { hitstun: true, reaction: Some(50.0), ..Sample::default() };
    let hit = Sample { stop: true, ..stun };
    tracker.step(Sample { attack: Some(Move::Jab), status: 1, ..Sample::default() }, hit, MappedInputs::NEUTRAL);
    tracker.step(standing(), stun, MappedInputs::NEUTRAL);
    tracker.step(Sample { attack: Some(Move::SideTilt), angle: Some(Angle::Up), status: 2, ..Sample::default() }, hit, MappedInputs::NEUTRAL);
    let link = tracker.latest_link().unwrap();
    assert!(matches!(link.verdict, Verdict::True(_)));
    assert_eq!(String::from_utf16(link_moves(&link, TEXT_UNITS).units()).unwrap(), "Jab > Side tilt ↑");
    let rec = LinkRec::of(&link, &tracker, tracker.exchange().1, true);
    let combo = Combo::from_links([rec]).unwrap();
    assert_eq!(crate::model::recording_menu::combo_link_lines(&combo, crate::model::move_names::Namer::OFF)[0], "L1 jab → side tilt ↑ TRUE");
    let mut helper = Helper::new();
    helper.set_target(Some(combo));
    let mut texts = Vec::new();
    Renderer::new().draw(&helper, &Text::EMPTY, None, true, (&C, false), |_, look, _| {
        if look.visible { texts.push(String::from_utf16_lossy(look.text.units())); }
    });
    assert!(texts.iter().any(|text| text == "1 Jab>Side tilt ↑"), "{texts:?}");
}

#[test]
fn a_hit_combo_tells_an_angled_tilt_from_a_level_one() {
    let empty = || Some(serde_json::json!({}));
    let up = *hit_record("jab", "side_tilt", empty(), Some(serde_json::json!({"angle":"up"}))).combo.get(0).unwrap();
    let level = *hit_record("jab", "side_tilt", empty(), empty()).combo.get(0).unwrap();
    assert!(grade(&up, &level).wrong_move);
    assert!(!grade(&up, &up).wrong_move);
    let mut helper = Helper::new();
    helper.set_target(Some(Combo::from_links([up]).unwrap()));
    helper.on_link(level, true, 1, &mut Out::default());
    assert_eq!(helper.results().next().unwrap().fault, Some((0, Reason::WrongMove(Move::SideTilt))));
    let special = *hit_record("jab", "other", empty(), Some(serde_json::json!({"of":"up_special"}))).combo.get(0).unwrap();
    let other = *hit_record("jab", "other", empty(), Some(serde_json::json!({"of":"side_special"}))).combo.get(0).unwrap();
    assert!(grade(&special, &other).wrong_move);
}

#[test]
fn a_link_without_descriptions_grades_as_before() {
    let old_tilt = *hit_record("jab", "side_tilt", None, None).combo.get(0).unwrap();
    let angled = *hit_record("jab", "side_tilt", Some(serde_json::json!({})), Some(serde_json::json!({"angle":"up"}))).combo.get(0).unwrap();
    assert!(!grade(&old_tilt, &angled).wrong_move);
    let old_other = *hit_record("jab", "other", None, None).combo.get(0).unwrap();
    let thrown = *hit_record("jab", "other", Some(serde_json::json!({})), Some(serde_json::json!({"move":"throw"}))).combo.get(0).unwrap();
    assert_eq!(thrown.to, Move::Throw);
    assert!(!grade(&old_other, &thrown).wrong_move);
}

#[test]
fn link_descriptions_round_trip_in_optional_keys() {
    use crate::model::combo_record::ComboRecord;
    let parsed = hit_record("other", "side_tilt", Some(serde_json::json!({"move":"throw"})), Some(serde_json::json!({"angle":"up"})));
    let record = ComboRecord::new("Drill".into(), parsed.combo, None);
    let mut value = serde_json::to_value(&record).unwrap();
    assert_eq!(value["links"][0]["from"], "other");
    assert_eq!(value["links"][0]["from_shown"], serde_json::json!({"move":"throw"}));
    assert_eq!(value["links"][0]["to_shown"], serde_json::json!({"angle":"up"}));
    let read: ComboRecord = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(read, record);
    value["links"][0]["to_shown"] = serde_json::json!(5);
    let error = serde_json::from_value::<ComboRecord>(value.clone()).unwrap_err();
    assert!(error.to_string().contains("to_shown"));
    value["links"][0].as_object_mut().unwrap().remove("to_shown");
    let partial: ComboRecord = serde_json::from_value(value).unwrap();
    let link = partial.combo.get(0).unwrap();
    assert!(!link.described());
    assert_eq!(link.from, Move::Other);
}

#[test]
fn a_link_record_keeps_its_size() {
    assert_eq!(core::mem::size_of::<LinkRec>(), 124);
}

#[test]
fn the_description_holds_no_names() {
    let source = include_str!("action.rs");
    assert_eq!(source.matches(".name()").count(), 0);
    assert_eq!(source.matches("namer.name(").count(), 1, "the one name call goes through the move names");
    for name in [r#""Jab""#, r#""Up B""#, r#""Side tilt""#] {
        assert!(!source.contains(name));
    }
}

#[test]
fn stick_flicks_inside_any_special_are_not_events() {
    for inside in [false, true] {
        let mut capture = InputCapture::new();
        for (frame, x) in [(0, 0), (1, 127)] {
            capture.step(Snap {
                lstick_x: x, facing: 1, aim: Seen { inside, ..Seen::NONE }, ..Snap::empty(frame)
            });
        }
        let flicks: Vec<_> = capture.combo.links().flat_map(LinkRec::events)
            .filter(|event| matches!(event.kind, Kind::Flick(_))).map(|event| event.kind).collect();
        assert_eq!(flicks, if inside { vec![] } else { vec![Kind::Flick(Flick::Dash)] });
    }
}

fn tilt(stick: (i8, i8), angle: Option<Angle>, mv: Move) -> (Tracker, InputCapture) {
    let mut tracker = Tracker::new();
    let mut capture = InputCapture::new();
    for frame in 1..=6 {
        let p1 = if frame == 1 {
            standing()
        } else {
            Sample { attack: Some(mv), angle, status: 2, motion_frame: (frame - 2) as f32,
                facing: 1, ..Sample::default() }
        };
        let inputs = MappedInputs { buttons: if frame < 3 { ATTACK } else { 0 },
            lstick_x: stick.0, lstick_y: stick.1, ..MappedInputs::NEUTRAL };
        tracker.step(p1, standing(), inputs);
        capture.step_late(tracker.snaps().last().unwrap(), tracker.late());
    }
    (tracker, capture)
}

#[test]
fn a_side_tilts_drill_direction_comes_from_its_animation() {
    for (stick, angle, mv, expected) in [
        ((60, 20), Some(Angle::Up), Move::SideTilt, Dir::ForwardUp),
        ((60, 60), None, Move::SideTilt, Dir::Forward),
        ((-60, 20), Some(Angle::Down), Move::SideSmash, Dir::BackDown),
        ((60, 20), None, Move::Jab, Dir::Forward),
        ((0, 0), Some(Angle::Up), Move::SideTilt, Dir::ForwardUp),
    ] {
        let (_, capture) = tilt(stick, angle, mv);
        let press = capture.combo.get(0).unwrap().events()
            .find(|e| e.kind == Kind::Press(Button::Attack)).unwrap();
        assert_eq!(press.dir, expected);
    }
    for (dir, level, up, down) in [
        (Dir::Neutral, Dir::Forward, Dir::ForwardUp, Dir::ForwardDown),
        (Dir::Forward, Dir::Forward, Dir::ForwardUp, Dir::ForwardDown),
        (Dir::ForwardUp, Dir::Forward, Dir::ForwardUp, Dir::ForwardDown),
        (Dir::Up, Dir::Forward, Dir::ForwardUp, Dir::ForwardDown),
        (Dir::BackUp, Dir::Back, Dir::BackUp, Dir::BackDown),
        (Dir::Back, Dir::Back, Dir::BackUp, Dir::BackDown),
        (Dir::BackDown, Dir::Back, Dir::BackUp, Dir::BackDown),
        (Dir::Down, Dir::Forward, Dir::ForwardUp, Dir::ForwardDown),
        (Dir::ForwardDown, Dir::Forward, Dir::ForwardUp, Dir::ForwardDown),
        (Dir::Side, Dir::Forward, Dir::ForwardUp, Dir::ForwardDown),
    ] {
        assert_eq!([action::angled(dir, None), action::angled(dir, Some(Angle::Up)),
            action::angled(dir, Some(Angle::Down))], [level, up, down]);
    }
    let mut capture = InputCapture::new();
    capture.step_late(Snap { buttons: ATTACK | CSTICK_ON, lstick_x: 60, lstick_y: 20,
        rstick_x: 60, rstick_y: 20, facing: 1, ..Snap::empty(1) },
        action::Late { open: Some((Some(1), Some(Some(Angle::Up)))), ..action::Late::NONE });
    let events = capture.combo.get(0).unwrap().events().map(|e| (e.kind, e.dir)).collect::<Vec<_>>();
    assert_eq!(events, vec![(Kind::CStick, Dir::ForwardUp), (Kind::Press(Button::Attack), Dir::Forward)]);
}

#[test]
fn a_finished_side_attack_keeps_its_animation_direction_in_the_take() {
    for (mv, angle, expected) in [
        (Move::SideTilt, Some(Angle::Up), Dir::ForwardUp),
        (Move::SideSmash, Some(Angle::Down), Dir::ForwardDown),
        (Move::SideTilt, None, Dir::Forward),
    ] {
        let (mut tracker, mut capture) = tilt((60, 0), angle, mv);
        let press = |capture: &InputCapture| capture.combo.get(0).unwrap().events()
            .find(|e| e.kind == Kind::Press(Button::Attack)).unwrap().dir;
        assert_eq!(press(&capture), expected);
        tracker.step(standing(), standing(), MappedInputs::NEUTRAL);
        capture.step_late(tracker.snaps().last().unwrap(), tracker.late());
        assert_eq!(press(&capture), expected, "{mv:?} after returning to idle");
    }
}

#[test]
fn the_strip_mark_and_the_drill_direction_agree() {
    for (angle, mark, dir) in [(Some(Angle::Up), Some(2), Dir::ForwardUp),
        (Some(Angle::Down), Some(6), Dir::ForwardDown), (None, None, Dir::Forward)] {
        let (tracker, capture) = tilt((60, 20), angle, Move::SideTilt);
        assert_eq!(tracker.aims().next().map(|aim| aim.dir), mark);
        assert_eq!(capture.combo.get(0).unwrap().events()
            .find(|e| e.kind == Kind::Press(Button::Attack)).unwrap().dir, dir);
    }
}

#[test]
fn a_buffered_tilt_waits_for_its_animation_before_it_is_graded() {
    let target = tilt((60, 20), Some(Angle::Up), Move::SideTilt).1.combo;
    let events = target.get(0).unwrap().events().map(|e| (e.kind, e.dir, e.offset)).collect::<Vec<_>>();
    assert_eq!(events, vec![(Kind::Press(Button::Attack), Dir::ForwardUp, 0),
        (Kind::Release(Button::Attack), Dir::Forward, 2)]);
    for angle in [Some(Angle::Up), None] {
        let mut tracker = Tracker::new();
        let mut helper = Helper::new();
        helper.set_target(Some(target));
        for frame in 1..=24 {
            let p1 = if frame <= 17 {
                Sample { attack: Some(Move::Jab), status: 1, motion_frame: (frame - 1) as f32,
                    facing: 1, ..Sample::default() }
            } else {
                Sample { attack: Some(Move::SideTilt), angle, status: 2, motion_frame: (frame - 18) as f32,
                    facing: 1, ..Sample::default() }
            };
            let inputs = MappedInputs { buttons: if (10..=11).contains(&frame) { ATTACK } else { 0 },
                lstick_x: if frame >= 10 { 60 } else { 0 }, lstick_y: if frame >= 10 { 20 } else { 0 },
                ..MappedInputs::NEUTRAL };
            tracker.step(p1, standing(), inputs);
            helper.step(&tracker, Settings::factory());
            if (10..=17).contains(&frame) {
                assert!(helper.results().next().is_none(), "frame {frame}");
            }
            if frame == 18 {
                let result = helper.results().next().expect("graded after animation");
                assert_eq!(result.outcome, Outcome::Completed);
                if angle.is_some() { assert!(result.clean); }
                else { assert_eq!(result.fault, Some((0, Reason::WrongDir(Kind::Press(Button::Attack))))); }
            }
        }
    }

    let mut target = tilt((60, 20), None, Move::SideTilt).1.combo;
    assert!(target.push(LinkRec { events: retain([Event { kind: Kind::Press(Button::Special), dir: Dir::Up, offset: 50 }]),
        ..LinkRec::INPUT }));
    let mut helper = Helper::new();
    helper.set_target(Some(target));
    for frame in 1..=4 {
        let late = if frame == 2 {
            action::Late { open: Some((Some(1), Some(None))), ..action::Late::NONE }
        } else { action::Late::NONE };
        helper.step_inputs_late(Snap { buttons: if frame < 3 { ATTACK } else { 0 }, lstick_x: 60, lstick_y: 20,
            facing: 1, ..Snap::empty(frame) }, Some(late), &mut Out::default());
    }
    assert_eq!(helper.attempt().unwrap().len(), 1);
    let mut out = Out::default();
    helper.step_inputs_late(Snap { lstick_x: 60, lstick_y: 20, facing: 1, ..Snap::empty(5) },
        Some(action::Late { open: Some((Some(1), Some(None))), ..action::Late::NONE }), &mut out);
    assert!(out.link.is_none() && out.resolved.is_none());
    helper.step_inputs_late(Snap { lstick_x: 60, lstick_y: 20, facing: 1, ..Snap::empty(6) },
        Some(action::Late { open: Some((Some(1), Some(Some(Angle::Up)))), ..action::Late::NONE }), &mut out);
    assert_eq!(out.link.unwrap().0, 0);
    assert_eq!(out.resolved.unwrap().fault, Some((0, Reason::WrongDir(Kind::Press(Button::Attack)))));

    let mut timeout = Helper::new();
    timeout.set_target(Some(tilt((60, 20), Some(Angle::Up), Move::SideTilt).1.combo));
    for frame in 1..=16 {
        timeout.step_inputs_late(Snap { buttons: if frame < 3 { ATTACK } else { 0 }, lstick_x: 60, lstick_y: 20,
            facing: 1, ..Snap::empty(frame) }, Some(action::Late::NONE), &mut Out::default());
        if frame < 16 { assert!(timeout.results().next().is_none(), "frame {frame}"); }
    }
    assert_eq!(timeout.results().next().unwrap().fault, Some((0, Reason::WrongDir(Kind::Press(Button::Attack)))));
}

#[test]
fn kirbys_angled_tilt_still_saves_as_version_two() {
    let combo = tilt((60, 20), Some(Angle::Up), Move::SideTilt).1.combo;
    let record = crate::model::combo_record::ComboRecord::new("Drill".into(), combo, Some("Kirby".into()));
    let value = serde_json::to_value(&record).unwrap();
    assert_eq!(value["schema_version"], 2);
    assert_eq!(value["kind"], "input_drill");
    assert_eq!(crate::model::recording_menu::combo_link_lines(&combo, crate::model::move_names::Namer::OFF), vec!["S1 Step 1", "↑→ A · release"]);
}

fn travel(end: (i8, i8), stick: (u16, u8)) -> action::Travel {
    let (x, y) = (f32::from(end.0), f32::from(end.1));
    action::Travel {
        stick: Some(stick),
        heading: ((y.atan2(x).to_degrees().rem_euclid(360.0) * 10.0).round() as u16) % 3600,
        distance: (x.hypot(y) * 10.0).round() as u16,
        path: core::array::from_fn(|i| {
            let t = (i + 1) as f32 / 8.0;
            ((x * t).round() as i8, (y * t).round() as i8)
        }),
    }
}

fn steering_snap(frame: u32, second: bool) -> Snap {
    Snap {
        buttons: if frame < 2 { SPECIAL } else if second && frame == 43 { ATTACK } else { 0 },
        facing: 1,
        aim: Seen { inside: frame <= 3, part: if frame == 3 { 1 } else { 0 },
            dir: (frame == 3).then_some(4), facing: 1, age: 0 },
        ..Snap::empty(frame)
    }
}

fn steering_capture(detail: action::Travel, second: bool) -> Combo {
    let mut capture = InputCapture::new();
    for frame in 0..=(if second { 43 } else { 4 }) {
        capture.step_late(steering_snap(frame, second), action::Late {
            travel: (frame == 4).then_some((1, detail)), ..action::Late::NONE
        });
    }
    capture.combo
}

fn steering_attempt(reference: action::Travel, actual: action::Travel, match_stick: bool) -> Summary {
    let mut helper = Helper::new();
    helper.set_target(Some(steering_capture(reference, false)));
    for frame in 0..=4 {
        helper.settings = Settings { match_stick, ..Settings::factory() };
        helper.step_inputs_late(steering_snap(frame, false), Some(action::Late {
            travel: (frame == 4).then_some((1, actual)), ..action::Late::NONE
        }), &mut Out::default());
    }
    let result = *helper.results().next().unwrap();
    result
}

#[test]
fn a_steered_part_is_graded_on_its_path_within_the_tolerance() {
    let reference = travel((-40, 0), (1800, 100));
    assert!(steering_attempt(reference, travel((-40, 8), (1800, 100)), false).clean);
    let result = steering_attempt(reference, travel((-38, 14), (1800, 100)), false);
    assert_eq!(result.fault, Some((0, Reason::AimPath(1))));
    assert_eq!(crate::model::combo_plan::reason_text(&result), crate::model::frame_strip_plan::Text::new("L1 1st path off"));
    assert_eq!(crate::model::combo_plan::mark_text(Mark::Path, "1st direction", 1),
        (crate::model::frame_strip_plan::Text::new("1st path off"), crate::model::frame_strip_plan::BAD));
}

#[test]
fn a_short_warp_fails_a_long_warp_drill() {
    let long = travel((-42, 42), (1350, 100));
    let short = travel((-14, 14), (1350, 100));
    assert_eq!(steering_attempt(long, short, false).fault, Some((0, Reason::AimPath(1))));
    assert_eq!(steering_attempt(short, long, false).fault, Some((0, Reason::AimPath(1))));
}

#[test]
fn the_angle_and_deflection_toggle_grades_the_stick() {
    let reference = travel((-40, 40), (1350, 100));
    for stick in [(1150, 100), (1350, 50)] {
        assert!(steering_attempt(reference, travel((-40, 40), stick), false).clean);
    }
    assert_eq!(steering_attempt(reference, travel((-40, 40), (1150, 100)), true).fault,
        Some((0, Reason::AimAngle(1, -200))));
    assert_eq!(steering_attempt(reference, travel((-40, 40), (1350, 50)), true).fault,
        Some((0, Reason::AimDeflection(1, -50))));
    assert!(steering_attempt(reference, travel((-40, 40), (1270, 90)), true).clean);
    let around_zero = travel((40, 0), (3590, 50));
    assert_eq!(action::stick_off(&around_zero, &travel((40, 0), (90, 65))), None);
    assert_eq!(action::stick_off(&around_zero, &travel((40, 0), (100, 50))), Some(Mark::Angle(110)));
    assert_eq!(action::stick_off(&around_zero, &travel((40, 0), (3590, 34))), Some(Mark::Deflection(-16)));
    assert_eq!(action::stick_of((0, 0), -1), (0, 0));
    assert_eq!(crate::model::combo_plan::mark_text(Mark::Angle(-200), "1st direction", 1).0,
        crate::model::frame_strip_plan::Text::new("1st angle 20 off"));
    // DESIGN section 4 words the fault "deflection", as the menu row does.
    assert_eq!(crate::model::combo_plan::mark_text(Mark::Deflection(-50), "2nd direction", 1).0,
        crate::model::frame_strip_plan::Text::new("2nd deflection 50% low"));
    assert_eq!(crate::model::combo_plan::mark_text(Mark::Deflection(30), "1st direction", 1).0,
        crate::model::frame_strip_plan::Text::new("1st deflection 30% high"));
    let low = steering_attempt(reference, travel((-40, 40), (1350, 50)), true);
    assert_eq!(crate::model::combo_plan::reason_text(&low),
        crate::model::frame_strip_plan::Text::new("L1 1st deflection 50% low"));
}

#[test]
fn a_failed_stick_angle_shows_the_difference_that_was_graded() {
    use crate::model::combo_plan::{mark_text, reason_text};
    let reference = travel((-40, 0), (1800, 83));
    for angle in [1700, 1900] {
        assert!(steering_attempt(reference, travel((-40, 0), (angle, 83)), true).clean);
    }
    for (angle, degrees) in [(1698, "10.2"), (1902, "10.2"), (1600, "20")] {
        let actual = travel((-40, 0), (angle, 83));
        let result = steering_attempt(reference, actual, true);
        let why = reason_text(&result);
        assert_eq!(String::from_utf16(why.units()).unwrap(), format!("L1 1st angle {degrees} off"));
        let mark = action::stick_off(&reference, &actual).unwrap();
        let label = mark_text(mark, "1st direction", 1).0;
        assert_eq!(String::from_utf16(label.units()).unwrap(), format!("1st angle {degrees} off"));
    }
}

/// The travel table's static cost: an entry is packed into 24 bytes, so the table adds 96 bytes to a combo, which
/// was 2,000 bytes without it. A separate combo type without the table for the hit capture was measured and cost
/// more code than the data it saved (`833913b3`), so every combo carries the table.
#[test]
fn the_travel_table_is_packed_into_96_bytes_a_combo() {
    use core::mem::size_of;
    let (entry, combo) = (size_of::<Detail>(), size_of::<Combo>());
    println!("TRAVEL_LAYOUT Travel={} Detail={entry} table={} Combo={combo} Helper={} InputCapture={}",
        size_of::<action::Travel>(), DETAILS * entry, size_of::<Helper>(), size_of::<InputCapture>());
    assert_eq!((entry, DETAILS * entry), (24, 96), "a travel detail and a combo's table");
    assert_eq!(combo, 2000 + DETAILS * entry, "a combo is its links and its table");
}

#[test]
fn the_toggle_row_reads_its_setting_and_is_off_by_default() {
    assert!(!Settings::factory().match_stick);
    let mut snapshot = crate::model::settings::Snapshot::factory();
    snapshot.set("combo_stick", &[1, 0]).unwrap();
    assert!(Settings::from_snapshot(&snapshot).match_stick);
    let rows = &crate::model::menu::table::ROWS;
    let spot = rows.iter().position(|row| row.id == "spots_combo").unwrap();
    assert_eq!(rows[spot + 1].id, "combo_stick");
}

#[test]
fn a_travel_that_ends_after_its_step_was_graded_amends_the_grade() {
    let reference = travel((-40, 0), (1800, 100));
    let target = steering_capture(reference, true);
    assert_eq!(target.len(), 2);
    for (attempt, clean) in [(travel((-38, 14), (1800, 100)), false),
        (travel((-40, 2), (1800, 100)), true)] {
        let mut helper = Helper::new();
        helper.set_target(Some(target));
        for frame in 0..=44 {
            helper.settings = Settings::factory();
            helper.step_inputs_late(steering_snap(frame, true), Some(action::Late {
                travel: (frame == 9).then_some((1, attempt)), ..action::Late::NONE
            }), &mut Out::default());
            if frame == 4 {
                assert_eq!(helper.attempt().unwrap().len(), 1);
                assert!(helper.results().next().is_none());
            }
            if frame == 9 {
                if clean { assert!(helper.results().next().is_none()); }
                else {
                    let result = helper.results().next().unwrap();
                    assert_eq!((result.outcome, result.clean, result.fault),
                        (Outcome::Completed, false, Some((0, Reason::AimPath(1)))));
                }
            }
        }
        if clean {
            let result = helper.results().next().unwrap();
            assert!(result.clean);
            assert_eq!(result.graded, 2);
        }
    }
}

#[test]
fn aim_details_round_trip_in_optional_keys() {
    let detail = travel((-40, 40), (1350, 100));
    let combo = steering_capture(detail, false);
    let record = crate::model::combo_record::ComboRecord::new("Drill".into(), combo, None);
    let value = serde_json::to_value(&record).unwrap();
    let event = &value["input_steps"][0]["events"][2];
    assert_eq!(event["stick"], serde_json::json!({"angle": 135.0, "deflection": 100}));
    assert_eq!(event["heading"], 135.0);
    assert_eq!(event["distance"], 56.6);
    assert_eq!(event["path"].as_array().unwrap().len(), 8);
    assert_eq!((event["kind"].as_str(), event["part"].as_u64(), event["dir"].as_str(), event["offset"].as_i64()),
        (Some("aim"), Some(1), Some("back"), Some(3)));
    let read: crate::model::combo_record::ComboRecord = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(read, record);
    let mut helper = Helper::new();
    helper.set_target(Some(read.combo));
    for frame in 0..=4 {
        helper.settings = Settings::factory();
        helper.step_inputs_late(steering_snap(frame, false), Some(action::Late {
            travel: (frame == 4).then_some((1, detail)), ..action::Late::NONE
        }), &mut Out::default());
    }
    assert!(helper.results().next().unwrap().clean);

    let mut split = value.clone();
    let events = value["input_steps"][0]["events"].as_array().unwrap();
    split["input_steps"] = serde_json::json!([
        {"events": [events[0].clone(), events[1].clone()]},
        {"events": [events[2].clone()]}
    ]);
    let joined: crate::model::combo_record::ComboRecord = serde_json::from_value(split).unwrap();
    assert_eq!(joined.combo.len(), 1);
    assert_eq!(joined.combo.travel(0, 2), Some(detail));

    let mut five = value.clone();
    let events = five["input_steps"][0]["events"].as_array_mut().unwrap();
    let aim = events[2].clone();
    for offset in 4..=7 {
        let mut event = aim.clone();
        event["offset"] = serde_json::json!(offset);
        events.push(event);
    }
    let read: crate::model::combo_record::ComboRecord = serde_json::from_value(five).unwrap();
    for index in 2..6 { assert_eq!(read.combo.travel(0, index), Some(detail)); }
    assert_eq!(read.combo.travel(0, 6), None);
    for (key, invalid) in [
        ("path", serde_json::json!([1, 2])),
        ("path", serde_json::json!([[128, 0], [0, 0], [0, 0], [0, 0], [0, 0], [0, 0], [0, 0], [0, 0]])),
        ("heading", serde_json::json!(360)),
        ("distance", serde_json::json!(-1)),
        ("stick", serde_json::json!({"angle": 0, "deflection": 101})),
    ] {
        let mut bad = value.clone();
        bad["input_steps"][0]["events"][2][key] = invalid;
        assert!(serde_json::from_value::<crate::model::combo_record::ComboRecord>(bad).unwrap_err().to_string().contains(key));
    }
    let mut old = value;
    old["input_steps"][0]["events"][2].as_object_mut().unwrap().remove("path");
    old["input_steps"][0]["events"][2]["heading"] = serde_json::json!("ignored");
    old["input_steps"][0]["events"][2]["stick"] = serde_json::json!("ignored");
    let read: crate::model::combo_record::ComboRecord = serde_json::from_value(old).unwrap();
    assert_eq!(read.combo.travel(0, 2), None);
}

#[test]
fn a_drill_keeps_details_for_its_first_four_aims() {
    let mut combo = Combo::EMPTY;
    combo.input_only = true;
    for i in 0..5 {
        let link = LinkRec { events: retain([Event { kind: Kind::Aim(1), dir: Dir::Back, offset: i * 10 }]),
            ..LinkRec::INPUT };
        assert!(combo.push(link));
    }
    let detail = travel((-40, 0), (1800, 100));
    for i in 0..4 { assert!(combo.attach_travel(i, 0, detail)); }
    assert!(!combo.attach_travel(4, 0, detail));
    let replacement = travel((-30, 0), (1800, 100));
    assert!(combo.attach_travel(0, 0, replacement));
    assert_eq!(combo.travel(0, 0), Some(replacement));
    assert!(combo.attach_travel(0, 0, detail));
    for i in 0..4 { assert_eq!(combo.travel(i, 0), Some(detail)); }
    assert_eq!(combo.travel(4, 0), None);
    let record = crate::model::combo_record::ComboRecord::new("Drill".into(), combo, None);
    let crop = record.crop_units(1, 4).unwrap();
    for i in 0..3 { assert_eq!(crop.combo.travel(i, 0), Some(detail)); }

    let mut split = Combo::EMPTY;
    split.input_only = true;
    for event in [Event { kind: Kind::Press(Button::Special), dir: Dir::Back, offset: 0 },
        Event { kind: Kind::Aim(1), dir: Dir::Back, offset: 1 },
        Event { kind: Kind::Aim(2), dir: Dir::Back, offset: 2 }] {
        assert!(split.push(LinkRec { events: retain([event]), ..LinkRec::INPUT }));
    }
    assert!(split.attach_travel(1, 0, detail));
    assert!(split.attach_travel(2, 0, replacement));
    split.join_split_steps();
    assert_eq!(split.len(), 2);
    assert_eq!(split.travel(0, 1), Some(detail));
    assert_eq!(split.travel(1, 0), Some(replacement));

    let mut unordered = Combo::from_links([LinkRec { events: retain([
        Event { kind: Kind::Aim(1), dir: Dir::Back, offset: 0 },
        Event { kind: Kind::Aim(2), dir: Dir::Back, offset: 1 },
    ]), ..LinkRec::INPUT }]).unwrap();
    unordered.input_only = true;
    assert!(unordered.attach_travel(0, 1, replacement));
    assert!(unordered.attach_travel(0, 0, detail));
    let record = crate::model::combo_record::ComboRecord::new("Drill".into(), unordered, None);
    let read: crate::model::combo_record::ComboRecord = serde_json::from_value(serde_json::to_value(&record).unwrap()).unwrap();
    assert_eq!(read, record);
}

/// A saved link keeps no inputs before its first hit, so its first move's travel arrows are not in it. That move is
/// the follow-up of the link before, which has them: a later link shows them, the combo's first link shows none, and
/// a crop that makes a link the first one stops borrowing. Nothing is stored for it.
#[test]
fn a_later_links_first_move_borrows_its_travel_arrows() {
    use crate::model::combo_plan::Renderer;
    use crate::model::combo_record::ComboRecord;
    use crate::model::frame_strip_plan::Text;
    use crate::model::panes::strip_layout::C;
    let empty = || Some(serde_json::json!({}));
    let first = LinkRec { events: retain([
        Event { kind: Kind::Press(Button::Special), dir: Dir::Up, offset: 1 },
        Event { kind: Kind::Aim(1), dir: Dir::Back, offset: 10 },
        Event { kind: Kind::Aim(2), dir: Dir::Forward, offset: 20 },
    ]), ..*hit_record("jab", "up_special", empty(), empty()).combo.get(0).unwrap() };
    let second = *hit_record("up_special", "side_tilt", empty(), Some(serde_json::json!({"angle":"up"}))).combo.get(0).unwrap();
    let combo = Combo::from_links([first, second]).unwrap();
    let heads = |combo: &Combo| -> Vec<String> {
        crate::model::recording_menu::combo_link_lines(combo, crate::model::move_names::Namer::OFF).into_iter()
            .filter(|line| line.starts_with('L') && line.contains(" → ")).collect()
    };
    assert_eq!(heads(&combo), vec!["L1 jab → up b ← → TRUE", "L2 up b ← → → side tilt ↑ TRUE"]);
    let mut helper = Helper::new();
    helper.set_target(Some(combo));
    let mut texts = Vec::new();
    Renderer::new().draw(&helper, &Text::EMPTY, None, true, (&C, false), |_, look, _| {
        if look.visible { texts.push(String::from_utf16_lossy(look.text.units())); }
    });
    assert!(texts.iter().any(|text| text == "1 Jab>Up B ← →"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "2 Up B ← →>Side tilt ↑"), "{texts:?}");
    // The crop's first link has no link before it.
    let record = ComboRecord::new("Drill".into(), combo, None);
    assert_eq!(heads(&record.crop_units(1, 2).unwrap().combo), vec!["L1 up b → side tilt ↑ TRUE"]);
    // Nothing new is stored: the link keeps its size and the file reads back equal.
    assert_eq!(core::mem::size_of::<LinkRec>(), 124);
    let read: ComboRecord = serde_json::from_value(serde_json::to_value(&record).unwrap()).unwrap();
    assert_eq!(read, record);
}

/// Edge: the travel ends on the very next frame after its step was graded. The grade is amended on that frame.
#[test]
fn a_travel_that_ends_the_frame_after_its_step_was_graded_amends_the_grade() {
    let target = steering_capture(travel((-40, 0), (1800, 100)), true);
    for (attempt, clean) in [(travel((-38, 14), (1800, 100)), false), (travel((-40, 2), (1800, 100)), true)] {
        let mut helper = Helper::new();
        helper.set_target(Some(target));
        for frame in 0..=44 {
            helper.settings = Settings::factory();
            helper.step_inputs_late(steering_snap(frame, true), Some(action::Late {
                travel: (frame == 5).then_some((1, attempt)), ..action::Late::NONE
            }), &mut Out::default());
            if frame == 4 {
                assert_eq!(helper.attempt().unwrap().len(), 1, "step 1 is graded before its travel ends");
                assert!(helper.results().next().is_none());
            }
            if frame == 5 {
                let result = helper.results().next().map(|r| (r.outcome, r.clean, r.fault));
                assert_eq!(result, (!clean).then_some((Outcome::Completed, false, Some((0, Reason::AimPath(1))))));
            }
        }
        if clean {
            let result = helper.results().next().unwrap();
            assert!(result.clean);
            assert_eq!(result.graded, 2);
        }
    }
}

/// Edge: *Match angle and deflection* flipped while a drill is being attempted. A step keeps the grade it got under
/// the setting in force when it was graded; a travel that ends later is graded under the setting in force then.
#[test]
fn the_stick_toggle_flipped_mid_drill_applies_from_the_next_grade() {
    let target = steering_capture(travel((-40, 0), (1800, 100)), true);
    let off_angle = travel((-40, 0), (1600, 100));
    // The frame the travel is handed over, the frame the toggle flips, whether it is On before the flip, the result.
    for (handed, flips, first, clean) in [
        (4, 5, false, true),
        (4, 5, true, false),
        (9, 6, false, false),
        (9, 6, true, true),
    ] {
        let mut helper = Helper::new();
        helper.set_target(Some(target));
        let mut resolved = None;
        for frame in 0..=44 {
            helper.settings = Settings { match_stick: first != (frame >= flips), ..Settings::factory() };
            helper.step_inputs_late(steering_snap(frame, true), Some(action::Late {
                travel: (frame == handed).then_some((1, off_angle)), ..action::Late::NONE
            }), &mut Out::default());
            if resolved.is_none() {
                resolved = helper.results().next().copied();
            }
        }
        let result = resolved.expect("the drill resolved");
        assert_eq!(result.clean, clean, "handed {handed}, flips {flips}, On first {first}");
        if !clean {
            assert_eq!(result.fault, Some((0, Reason::AimAngle(1, -200))));
        }
    }
}

}
