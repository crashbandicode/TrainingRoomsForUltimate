lib_unit_tests! {
use super::*;
use crate::model::aim::Seen;
use crate::model::combo_record::ComboRecord;
use crate::model::frame_strip::Sample;
use crate::model::input_delay::MappedInputs;

const A: u32 = 1;
const B: u32 = 2;
const RESET: u32 = 0x8 | A;

fn standing() -> Sample {
    Sample { actionable: true, facing: 1, ..Sample::default() }
}

fn active(mv: Move, status: i32) -> Sample {
    Sample { attack: Some(mv), status, facing: 1, ..Sample::default() }
}

fn advance(tracker: &mut Tracker, helper: &mut Helper, sample: Sample, buttons: u32, x: i8, y: i8, settings: Settings) {
    tracker.step(sample, standing(), MappedInputs { buttons, lstick_x: x, lstick_y: y, ..MappedInputs::NEUTRAL });
    helper.step(tracker, settings);
}

#[derive(Clone, Copy)]
enum Shape { Tap, Charge, Held }

fn shape_frame(shape: Shape, frame: u32, release: u32, delay: u32) -> (Sample, MappedInputs) {
    let (button, x, y) = match shape { Shape::Charge => (A, 60, 0), _ => (B, 0, 60) };
    let p1 = match shape {
        Shape::Tap if (2..=15).contains(&frame) => active(Move::UpSpecial, 0x1de),
        Shape::Held if (2..=60).contains(&frame) => active(Move::UpSpecial, 0x1de),
        Shape::Charge if frame >= 2 => active(Move::SideSmash,
            if frame < 9 { 0x2e } else if frame < release.saturating_add(delay) { 0x2f } else { 0x30 }),
        _ => standing(),
    };
    let inputs = MappedInputs { buttons: if frame < release { button } else { 0 }, lstick_x: x, lstick_y: y,
        ..MappedInputs::NEUTRAL };
    (p1, inputs)
}

fn recorded_shape(shape: Shape, hold: u32, delay: u32) -> Combo {
    let mut tracker = Tracker::new();
    let mut input = InputCapture::new();
    for frame in 1..=80 {
        let (p1, inputs) = shape_frame(shape, frame, hold + 1, delay);
        tracker.step(p1, standing(), inputs);
        input.step_late(tracker.snaps().last().unwrap(), tracker.late());
    }
    input.combo
}

fn replay(shape: Shape, target: Combo, hold: u32, delay: u32, tolerance: u8) -> Summary {
    let mut tracker = Tracker::new();
    let mut helper = Helper::new();
    helper.set_target(Some(target));
    for frame in 1..=90 {
        let (p1, inputs) = shape_frame(shape, frame, hold.saturating_add(1), delay);
        tracker.step(p1, standing(), inputs);
        helper.step(&tracker, Settings { tolerance, ..Settings::factory() });
    }
    let result = *helper.results().next().expect("drill resolved");
    result
}

fn release_kind(combo: &Combo) -> Kind {
    combo.links().flat_map(LinkRec::events).find_map(|event| match event.kind {
        Kind::Release(_) | Kind::LetGo(_) => Some(event.kind),
        _ => None,
    }).expect("release recorded")
}

#[test]
fn a_tap_is_not_graded_on_its_length() {
    let reference = recorded_shape(Shape::Tap, 1, 0);
    assert_eq!(release_kind(&reference), Kind::Release(Button::Special));
    for hold in [1, 3, 9] {
        assert!(replay(Shape::Tap, reference, hold, 0, 1).clean, "{hold}");
    }
    let value = serde_json::to_value(ComboRecord::new("Tap".into(), reference, None)).unwrap();
    assert_eq!(value["input_steps"][0]["events"][1]["reacted"], false);
}

#[test]
fn a_charge_release_is_graded_on_its_hold() {
    for delay in [0, 1] {
        let reference = recorded_shape(Shape::Charge, 40, delay);
        assert_eq!(release_kind(&reference), Kind::LetGo(Button::Attack), "delay {delay}");
        let value = serde_json::to_value(ComboRecord::new("Charge".into(), reference, None)).unwrap();
        assert_eq!(value["input_steps"][0]["events"][1]["reacted"], true);
        assert_eq!(value["input_steps"][0]["events"][1]["hold"], 40);
        for hold in [40, 45, 35, 46, 34] {
            assert!(replay(Shape::Charge, reference, hold, delay, 1).clean, "delay {delay}, hold {hold}");
        }
        assert_eq!(replay(Shape::Charge, reference, 47, delay, 1).fault, Some((0, Reason::MissingRelease(Button::Attack))));
        assert_eq!(replay(Shape::Charge, reference, 33, delay, 1).fault, Some((0, Reason::ReleaseOff(Button::Attack, -7))));
        assert!(!replay(Shape::Charge, reference, 2, delay, 1).clean);
    }
}

#[test]
fn a_held_special_that_ignores_its_release_is_not_graded() {
    let reference = recorded_shape(Shape::Held, 30, 0);
    assert_eq!(release_kind(&reference), Kind::Release(Button::Special));
    for hold in [5, 30, 55, u32::MAX] {
        assert!(replay(Shape::Held, reference, hold, 0, 1).clean, "{hold}");
    }
}

#[test]
fn a_status_change_outside_the_tolerance_is_no_reaction() {
    let reference = recorded_shape(Shape::Charge, 40, 2);
    assert_eq!(release_kind(&reference), Kind::Release(Button::Attack));
    assert!(replay(Shape::Charge, reference, 30, 2, 2).clean);
}

#[test]
fn an_older_files_tap_is_not_graded_and_its_long_hold_is() {
    const TAP_ONE: &str = r#"{"input_steps":[{"events":[{"button":"special","dir":"up","kind":"press","offset":0},{"button":"special","dir":"back","hold":1,"kind":"release","offset":1},{"dir":"back","kind":"aim","offset":15,"part":1}]},{"events":[{"dir":"forward","kind":"aim","offset":29,"part":2}]}],"kind":"input_drill","name":"Drill 1","recorded_on":"Pikachu","schema_version":4,"take":"drill-take-6dcdd38f4a4e0afd"}"#;
    const TAP_THREE: &str = r#"{"input_steps":[{"events":[{"button":"special","dir":"up","kind":"press","offset":0},{"button":"special","dir":"back","hold":3,"kind":"release","offset":3},{"dir":"back","kind":"aim","offset":15,"part":1}]},{"events":[{"dir":"forward","kind":"aim","offset":29,"part":2}]}],"kind":"input_drill","name":"Drill 6","recorded_on":"Pikachu","schema_version":4,"take":"drill-take-2fe35bd7f4b965de"}"#;
    let zips = |target: Combo, hold: u32, first: bool| {
        let mut helper = Helper::new();
        helper.set_target(Some(target));
        for frame in 0..=45 {
            let part = if frame == 15 && first { 1 } else if frame == 29 { 2 } else { 0 };
            let dir = if part == 1 { Some(4) } else if part == 2 { Some(0) } else { None };
            helper.step_inputs(Snap { buttons: if frame < hold { B } else { 0 },
                lstick_x: if frame < 3 { 0 } else if frame < 20 { -127 } else { 127 },
                lstick_y: if frame < 3 { 127 } else { 0 }, facing: 1,
                aim: Seen { inside: frame <= 35, part, dir, facing: 1, age: u8::from(part != 0) },
                ..Snap::empty(frame) }, &mut Out::default());
        }
        let result = *helper.results().next().expect("attempt resolved");
        result
    };
    for literal in [TAP_ONE, TAP_THREE] {
        let record: ComboRecord = serde_json::from_str(literal).unwrap();
        assert_eq!(release_kind(&record.combo), Kind::Release(Button::Special));
        for hold in [1, 3, 6] {
            let result = zips(record.combo, hold, true);
            assert!(result.clean, "{hold}: {result:?}");
            assert_eq!(result.graded, 2);
        }
        assert_eq!(zips(record.combo, 3, false).fault, Some((0, Reason::MissingAim(1))));
        assert_eq!(serde_json::to_string(&record).unwrap(), literal);
    }
    const OLD: &str = r#"{"input_steps":[{"events":[{"button":"attack","dir":"neutral","kind":"press","offset":0}]},{"events":[{"button":"attack","dir":"neutral","kind":"release","offset":30}]}],"kind":"input_drill","name":"Drill 2","schema_version":2}"#;
    let record: ComboRecord = serde_json::from_str(OLD).unwrap();
    assert_eq!(release_kind(&record.combo), Kind::LetGo(Button::Attack));
    let run = |hold| {
        let mut helper = Helper::new();
        helper.set_target(Some(record.combo));
        for frame in 0..=80 {
            helper.step_inputs(Snap { buttons: if frame < hold { A } else { 0 }, facing: 1,
                ..Snap::empty(frame) }, &mut Out::default());
        }
        let result = *helper.results().next().unwrap();
        result
    };
    for hold in [30, 33] {
        assert!(run(hold).clean, "{hold}");
    }
    assert_eq!(run(3).fault, Some((0, Reason::ReleaseOff(Button::Attack, -27))));
    assert_eq!(run(60).fault, Some((0, Reason::MissingRelease(Button::Attack))));
    assert_eq!(serde_json::to_string(&record).unwrap(), OLD);
    for (hold, kind) in [(7, Kind::Release(Button::Attack)), (8, Kind::LetGo(Button::Attack))] {
        let mut value: serde_json::Value = serde_json::from_str(OLD).unwrap();
        value["input_steps"][1]["events"][0]["offset"] = serde_json::json!(hold);
        let boundary: ComboRecord = serde_json::from_value(value).unwrap();
        assert_eq!(release_kind(&boundary.combo), kind);
    }
    let mut value: serde_json::Value = serde_json::from_str(OLD).unwrap();
    value["input_steps"][0]["events"][0]["button"] = serde_json::json!("special");
    let orphan: ComboRecord = serde_json::from_value(value).unwrap();
    assert_eq!(release_kind(&orphan.combo), Kind::Release(Button::Attack));
}

#[test]
fn a_new_file_says_whether_the_game_reacted() {
    let ignored = ComboRecord::new("Held".into(), recorded_shape(Shape::Held, 30, 0), None);
    let value = serde_json::to_value(&ignored).unwrap();
    assert_eq!(value["input_steps"][0]["events"][1]["reacted"], false);
    let read: ComboRecord = serde_json::from_value(value).unwrap();
    assert_eq!(release_kind(&read.combo), Kind::Release(Button::Special));
    for hold in [3, 60] { assert!(replay(Shape::Held, read.combo, hold, 0, 1).clean); }

    let charge = ComboRecord::new("Charge".into(), recorded_shape(Shape::Charge, 40, 0), None);
    assert_eq!(serde_json::to_value(&charge).unwrap()["input_steps"][0]["events"][1]["reacted"], true);
    let mut short = target(A, 0, 0, 3);
    short.links[0].as_mut().unwrap().events[1].as_mut().unwrap().kind = Kind::LetGo(Button::Attack);
    let value = serde_json::to_value(ComboRecord::new("Short".into(), short, None)).unwrap();
    let read: ComboRecord = serde_json::from_value(value).unwrap();
    let run = |hold| {
        let mut helper = Helper::new();
        helper.set_target(Some(read.combo));
        for frame in 0..=20 {
            helper.step_inputs(Snap { buttons: if frame < hold { A } else { 0 }, facing: 1,
                ..Snap::empty(frame) }, &mut Out::default());
        }
        let result = *helper.results().next().unwrap();
        result
    };
    for hold in 2..=4 { assert!(run(hold).clean, "{hold}"); }
    assert_eq!(run(6).fault, Some((0, Reason::MissingRelease(Button::Attack))));

    let mut link = LinkRec::INPUT;
    link.events[0] = Some(Event { kind: Kind::Press(Button::Attack), dir: Dir::Neutral, offset: 0 });
    link.events[1] = Some(Event { kind: Kind::Release(Button::Attack), dir: Dir::Neutral, offset: 3 });
    let hit = ComboRecord::new("Hit".into(), Combo::from_links([link]).unwrap(), None);
    let value = serde_json::to_value(hit).unwrap();
    assert!(value["links"][0]["events"][1].get("reacted").is_none());
}

#[test]
fn a_reacted_release_round_trips() {
    let combo = recorded_shape(Shape::Charge, 40, 1);
    let record = ComboRecord::new("Charge".into(), combo, None);
    let mut value = serde_json::to_value(&record).unwrap();
    assert_eq!(serde_json::from_value::<ComboRecord>(value.clone()).unwrap(), record);
    value["input_steps"][0]["events"][1]["reacted"] = serde_json::json!(false);
    let plain: ComboRecord = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(release_kind(&plain.combo), Kind::Release(Button::Attack));
    value["input_steps"][0]["events"][1]["reacted"] = serde_json::json!(1);
    assert!(serde_json::from_value::<ComboRecord>(value).unwrap_err().to_string().contains("reacted"));
    let crop = record.crop_units(0, 1).unwrap();
    assert_eq!(release_kind(&crop.combo), Kind::LetGo(Button::Attack));
    let mut two = InputCapture::new();
    two.step_late(Snap { buttons: A, ..Snap::empty(0) }, Late::NONE);
    two.step_late(Snap::empty(2), Late { turned: Some(false), ..Late::NONE });
    two.step_late(Snap { buttons: B, ..Snap::empty(30) }, Late::NONE);
    two.step_late(Snap::empty(40), Late { turned: Some(false), ..Late::NONE });
    let cropped = ComboRecord::new("Two".into(), two.combo, None).crop_units(1, 2).unwrap();
    assert_eq!(release_kind(&cropped.combo), Kind::LetGo(Button::Special));
    assert_eq!(cropped.combo.get(0).unwrap().events().map(|event| event.offset).collect::<Vec<_>>(), vec![0, 10]);
    let split = serde_json::json!({"schema_version": 2, "kind": "input_drill", "name": "Charge",
        "input_steps": [
            {"events": [{"kind": "press", "button": "attack", "dir": "forward", "offset": 0}]},
            {"events": [{"kind": "release", "button": "attack", "dir": "forward", "offset": 40, "reacted": true}]}
        ]});
    let joined: ComboRecord = serde_json::from_value(split).unwrap();
    assert_eq!(joined.combo.len(), 1);
    assert_eq!(release_kind(&joined.combo), Kind::LetGo(Button::Attack));

    let mut input = InputCapture::new();
    input.step_late(Snap { buttons: A | B, ..Snap::empty(1) }, Late::NONE);
    input.step_late(Snap::empty(2), Late::NONE);
    input.step_late(Snap::empty(3), Late { turned: Some(false), ..Late::NONE });
    let releases = input.combo.links().flat_map(LinkRec::events).filter_map(|event| match event.kind {
        Kind::LetGo(button) => Some(button),
        _ => None,
    }).collect::<Vec<_>>();
    assert_eq!(releases, vec![Button::Attack, Button::Special]);
}

fn target(button: u32, x: i8, y: i8, hold: u32) -> Combo {
    let mut input = InputCapture::new();
    input.step(Snap { buttons: button, lstick_x: x, lstick_y: y, facing: 1, ..Snap::empty(0) });
    input.step(Snap { lstick_x: x, lstick_y: y, facing: 1, ..Snap::empty(hold) });
    input.combo
}

fn clean_attempt(tracker: &mut Tracker, helper: &mut Helper, button: u32, x: i8, y: i8, hold: u32) {
    let mv = if button == A { Move::Jab } else { Move::UpSpecial };
    for _ in 0..hold { advance(tracker, helper, active(mv, 0x27), button, x, y, Settings::factory()); }
    for _ in 0..26 { advance(tracker, helper, standing(), 0, 0, 0, Settings::factory()); }
}

fn chord(tracker: &mut Tracker, helper: &mut Helper, frames: usize, opens: bool) {
    for _ in 0..frames {
        let sample = if opens { active(Move::Jab, 0x27) } else { standing() };
        advance(tracker, helper, sample, RESET, 0, 0, Settings::factory());
    }
}

fn clear_held_chord(tracker: &mut Tracker, helper: &mut Helper) {
    tracker.clear();
    for _ in 0..4 { advance(tracker, helper, standing(), RESET, 0, 0, Settings::factory()); }
    advance(tracker, helper, standing(), 0, 0, 0, Settings::factory());
}

#[test]
fn a_reset_chord_alone_leaves_no_result() {
    for (button, x, y, hold) in [(A, 60, 60, 2), (B, 0, 60, 3)] {
        for count in [0, 2, RESULTS] {
            let mut tracker = Tracker::new();
            let mut helper = Helper::new();
            helper.set_target(Some(target(button, x, y, hold)));
            for _ in 0..10 { advance(&mut tracker, &mut helper, standing(), 0, 0, 0, Settings::factory()); }
            for _ in 0..count { clean_attempt(&mut tracker, &mut helper, button, x, y, hold); }
            let before = helper.results().copied().collect::<Vec<_>>();
            let stats = (helper.streak(), helper.best());
            assert_eq!(before.len(), count);
            assert!(before.iter().all(|result| result.clean));
            assert_eq!(stats.0, count as u16);
            chord(&mut tracker, &mut helper, 8, true);
            assert_eq!(helper.results().count(), (count + 1).min(RESULTS), "the chord resolved before clear");
            assert!(!helper.results().next().unwrap().clean);
            clear_held_chord(&mut tracker, &mut helper);
            assert_eq!(helper.results().copied().collect::<Vec<_>>(), before);
            assert_eq!((helper.streak(), helper.best()), stats);
            assert!(helper.attempt().is_none());
        }
    }
}

#[test]
fn a_reset_before_the_deadline_leaves_no_result() {
    let mut tracker = Tracker::new();
    let mut helper = Helper::new();
    helper.set_target(Some(target(A, 60, 60, 2)));
    for _ in 0..10 { advance(&mut tracker, &mut helper, standing(), 0, 0, 0, Settings::factory()); }
    chord(&mut tracker, &mut helper, 2, false);
    assert!(helper.results().next().is_none());
    clear_held_chord(&mut tracker, &mut helper);
    assert!(helper.results().next().is_none());
    assert!(helper.attempt().is_none());
}

#[test]
fn a_real_attempt_then_a_reset_records_as_today() {
    let mut capture = InputCapture::new();
    for (frame, buttons) in [(0, A), (2, 0), (30, B), (32, 0)] {
        capture.step(Snap { buttons, ..Snap::empty(frame) });
    }
    for event in capture.combo.links[0].as_mut().unwrap().events.iter_mut().flatten() {
        if event.kind == Kind::Release(Button::Attack) { event.kind = Kind::LetGo(Button::Attack); }
    }
    let mut tracker = Tracker::new();
    let mut helper = Helper::new();
    helper.set_target(Some(capture.combo));
    clean_attempt(&mut tracker, &mut helper, A, 0, 0, 2);
    assert!(helper.attempt().is_some_and(|a| a.open() && a.len() == 1));
    chord(&mut tracker, &mut helper, 2, true);
    assert!(helper.input.gesture_step(1));
    clear_held_chord(&mut tracker, &mut helper);
    assert_eq!(helper.results().map(|s| (s.outcome, s.clean)).collect::<Vec<_>>(), vec![(Outcome::Cleared, false)]);

    let mut tracker = Tracker::new();
    let mut helper = Helper::new();
    helper.set_target(Some(target(A, 0, 0, 2)));
    clean_attempt(&mut tracker, &mut helper, A, 0, 0, 2);
    for _ in 0..4 { advance(&mut tracker, &mut helper, standing(), 0, 0, 0, Settings::factory()); }
    let before = helper.results().copied().collect::<Vec<_>>();
    assert_eq!((before.len(), helper.streak()), (1, 1));
    chord(&mut tracker, &mut helper, 8, true);
    clear_held_chord(&mut tracker, &mut helper);
    assert_eq!(helper.results().copied().collect::<Vec<_>>(), before);
    assert_eq!(helper.streak(), 1);
}

#[test]
fn a_failed_attempt_is_not_taken_back_by_a_later_reset() {
    let mut tracker = Tracker::new();
    let mut helper = Helper::new();
    helper.set_target(Some(target(A, 0, 0, 2)));
    advance(&mut tracker, &mut helper, active(Move::UpSpecial, 0x1de), B, 0, 0, Settings::factory());
    advance(&mut tracker, &mut helper, standing(), 0, 0, 0, Settings::factory());
    let before = helper.results().copied().collect::<Vec<_>>();
    assert_eq!(before.len(), 1);
    assert!(!before[0].clean);
    chord(&mut tracker, &mut helper, 8, true);
    clear_held_chord(&mut tracker, &mut helper);
    assert_eq!(helper.results().copied().collect::<Vec<_>>(), before);
}
}
