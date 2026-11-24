//! EXPECTED RED on e09fc3cf: drill step descriptions, titles, persistence, aim, tilt and throw.
//! EXPECTED RED on 4e701753: first/last actions, their file keys and special aim ownership.
//! GUARDS: the packed sizes do not grow and a press outside the take describes no step.

lib_unit_tests! {
use super::*;
use crate::model::combo_plan::{self, Renderer};
use crate::model::combo_record::ComboRecord;
use crate::model::frame_strip::Sample;
use crate::model::frame_strip_plan::Text;
use crate::model::input_delay::MappedInputs;
use crate::model::panes::{cb, strip_layout::C};
use crate::model::recording_menu::combo_link_lines;

fn idle() -> Sample {
    Sample { actionable: true, facing: 1, ..Sample::default() }
}

fn tick(tracker: &mut Tracker, capture: &mut InputCapture, p1: Sample, inputs: MappedInputs) {
    tracker.step(p1, idle(), inputs);
    capture.step_late(tracker.snaps().last().unwrap(), tracker.late());
}

fn desc(mv: Move, of: Option<Move>, angle: Option<Angle>, normal: u8) -> ActionDesc {
    ActionDesc { mv, of, angle, normal, parts: [None; 2] }
}

fn event(kind: Kind, dir: Dir, offset: i32) -> Event {
    Event { kind, dir, offset }
}

fn step(events: impl IntoIterator<Item = Event>, description: ActionDesc) -> LinkRec {
    let mut step = LinkRec { events: retain(events), ..LinkRec::INPUT };
    step.set_step(description);
    step
}

fn drill(steps: impl IntoIterator<Item = LinkRec>) -> Combo {
    let mut combo = Combo::from_links(steps).unwrap();
    combo.input_only = true;
    combo
}

fn title(step: &LinkRec, tight: bool) -> String {
    let mut text = String::new();
    step.write_title(&mut text, tight).unwrap();
    text
}

fn bar(combo: Combo) -> Vec<String> {
    let mut helper = Helper::new();
    helper.set_target(Some(combo));
    let mut titles = vec![String::new(); cb::CELLS];
    Renderer::new().draw(&helper, &Text::EMPTY, None, true, (&C, false), |i, look, _| {
        if (combo_plan::at::TITLE..combo_plan::at::VERDICT).contains(&i) && look.visible {
            titles[i - combo_plan::at::TITLE] = String::from_utf16(look.text.units()).unwrap();
        }
    });
    titles
}

fn jab() -> Combo {
    let mut tracker = Tracker::new();
    let mut capture = InputCapture::new();
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 1, ..MappedInputs::NEUTRAL });
    for frame in 0..4 {
        tick(&mut tracker, &mut capture, Sample { attack: Some(Move::Jab), status: 0x27,
            motion_frame: frame as f32, facing: 1, ..Sample::default() }, MappedInputs::NEUTRAL);
    }
    capture.combo
}

fn zip_sample(attack: Move, pos: Option<(f32, f32)>) -> Sample {
    Sample { attack: Some(attack), status: if attack == Move::UpSpecial { 1 } else if pos.is_some() { 2 } else { 3 },
        pos: Some(pos.unwrap_or((0.0, 0.0))), facing: 1, ..Sample::default() }
}

fn zips_capture(second: bool) -> (Tracker, InputCapture) {
    let mut tracker = Tracker::new();
    let mut capture = InputCapture::new();
    for frame in 0..14 {
        tick(&mut tracker, &mut capture, zip_sample(Move::UpSpecial, None),
            MappedInputs { buttons: if frame < 3 { 2 } else { 0 }, lstick_y: 127, ..MappedInputs::NEUTRAL });
    }
    for frame in 0..5 {
        tick(&mut tracker, &mut capture, zip_sample(Move::Other, Some((-11.0 * frame as f32, 0.0))),
            MappedInputs { lstick_x: -100, ..MappedInputs::NEUTRAL });
    }
    if second {
        for _ in 0..9 {
            tick(&mut tracker, &mut capture, Sample { pos: Some((-44.0, 0.0)), ..zip_sample(Move::Other, None) },
                MappedInputs { lstick_x: 100, ..MappedInputs::NEUTRAL });
        }
        for frame in 0..5 {
            tick(&mut tracker, &mut capture, zip_sample(Move::Other, Some((-44.0 + 9.9 * frame as f32, 0.0))),
                MappedInputs { lstick_x: 100, ..MappedInputs::NEUTRAL });
        }
    }
    (tracker, capture)
}

fn zips(second: bool) -> Combo {
    zips_capture(second).1.combo
}

#[test]
fn a_drill_step_stores_the_move_its_press_became() {
    assert_eq!(jab().get(0).unwrap().step_desc().map(|d| d.mv), Some(Move::Jab));
}

fn two_described() -> Combo {
    let mut tracker = Tracker::new();
    let mut capture = InputCapture::new();
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 1, ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, Sample { attack: Some(Move::Jab), status: 0x27, facing: 1,
        ..Sample::default() }, MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, idle(), MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 2, ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, Sample { attack: Some(Move::NeutralSpecial), status: 0x1dc,
        facing: 1, ..Sample::default() }, MappedInputs::NEUTRAL);
    capture.combo
}

fn side(link: &LinkRec, side: usize) -> Option<ActionDesc> {
    (link.description_bits(side) & 31 != 0).then(|| link.desc(side))
}

#[test]
fn a_step_with_two_actions_shows_both() {
    let mut tracker = Tracker::new();
    let mut capture = InputCapture::new();
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 4, ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, Sample { status: 0x19, facing: 1, ..Sample::default() }, MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, idle(), MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 1, ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, Sample { attack: Some(Move::Aerial), status: 0x36, facing: 1,
        ..Sample::default() }, MappedInputs::NEUTRAL);
    assert_eq!(capture.combo.len(), 1);
    assert_eq!(bar(capture.combo)[0], "1 Jump > Aerial");
    assert_eq!(combo_link_lines(&capture.combo)[0], "S1 jump → aerial");
    let step = capture.combo.get(0).unwrap();
    assert_eq!(side(step, 0), None);
    assert_eq!(side(step, 1).map(|d| d.mv), Some(Move::Aerial));

    let combo = two_described();
    assert_eq!(combo.len(), 1);
    assert_eq!(bar(combo)[0], "1 Jab > Neutral B");
    let step = combo.get(0).unwrap();
    assert_eq!((step.from, step.to), (Move::Jab, Move::NeutralSpecial));
}

#[test]
fn a_step_with_three_actions_shows_the_first_and_the_last() {
    let mut tracker = Tracker::new();
    let mut capture = InputCapture::new();
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 1, ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, Sample { attack: Some(Move::Jab), status: 0x27, facing: 1,
        ..Sample::default() }, MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, idle(), MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 4, ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, Sample { status: 0x19, facing: 1, ..Sample::default() }, MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, idle(), MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 2, ..MappedInputs::NEUTRAL });
    assert_eq!(capture.combo.len(), 1);
    assert_eq!(bar(capture.combo)[0], "1 Jab > Special");
    tick(&mut tracker, &mut capture, Sample { attack: Some(Move::NeutralSpecial), status: 0x1dc,
        facing: 1, ..Sample::default() }, MappedInputs::NEUTRAL);
    assert_eq!(bar(capture.combo)[0], "1 Jab > Neutral B");
    assert_eq!(combo_link_lines(&capture.combo)[0], "S1 jab → neutral b");
}

#[test]
fn a_simultaneous_dash_flick_does_not_steal_the_attack_description() {
    let mut tracker = Tracker::new();
    let mut capture = InputCapture::new();
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 1, lstick_x: 127, ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, Sample { attack: Some(Move::DashAttack), status: 0x27, facing: 1,
        ..Sample::default() }, MappedInputs { lstick_x: 127, ..MappedInputs::NEUTRAL });
    assert_eq!(capture.combo.len(), 1);
    assert_eq!(bar(capture.combo)[0], "1 Dash attack");
    let step = capture.combo.get(0).unwrap();
    assert_eq!(side(step, 0), None);
    assert_eq!(side(step, 1).map(|desc| desc.mv), Some(Move::DashAttack));
}

#[test]
fn one_physical_input_is_one_title_input() {
    for left in [0, 127] {
        let mut tracker = Tracker::new();
        let mut capture = InputCapture::new();
        tick(&mut tracker, &mut capture, idle(), MappedInputs {
            buttons: 1 | CSTICK_ON, lstick_x: left, rstick_x: 127, ..MappedInputs::NEUTRAL
        });
        tick(&mut tracker, &mut capture, Sample { attack: Some(Move::SideSmash), status: 0x27, facing: 1,
            ..Sample::default() }, MappedInputs { lstick_x: left, ..MappedInputs::NEUTRAL });
        assert_eq!(capture.combo.len(), 1);
        assert_eq!(bar(capture.combo)[0], "1 Side smash");
        let step = capture.combo.get(0).unwrap();
        assert_eq!(side(step, 0), None);
        assert_eq!(side(step, 1).map(|desc| desc.mv), Some(Move::SideSmash));
    }

    let mut tracker = Tracker::new();
    let mut capture = InputCapture::new();
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 1 | 8, ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, Sample { named: Some(Move::Grab), status: 0x37, facing: 1,
        ..Sample::default() }, MappedInputs::NEUTRAL);
    assert_eq!(bar(capture.combo)[0], "1 Grab");
    assert_eq!(side(capture.combo.get(0).unwrap(), 1).map(|d| d.mv), Some(Move::Grab));

    let mut tracker = Tracker::new();
    let mut capture = InputCapture::new();
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: FLICK_JUMP, lstick_y: 127,
        ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, idle(), MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 1, ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, Sample { attack: Some(Move::Aerial), status: 0x36, facing: 1,
        ..Sample::default() }, MappedInputs::NEUTRAL);
    assert_eq!(bar(capture.combo)[0], "1 Jump ↑ > Aerial");

    let mut capture = InputCapture::new();
    capture.step(Snap { buttons: CSTICK_ON, rstick_x: 127, facing: 1, ..Snap::empty(0) });
    assert_eq!(bar(capture.combo)[0], "1 C-stick →");
    let mut capture = InputCapture::new();
    capture.step(Snap { buttons: CSTICK_ON | 1, rstick_x: 127, facing: 1, ..Snap::empty(0) });
    assert_eq!(bar(capture.combo)[0], "1 C-stick →");
}

#[test]
fn a_stick_flick_is_a_title_input_only_without_a_button() {
    let mut capture = InputCapture::new();
    capture.step(Snap { lstick_x: 127, facing: 1, ..Snap::empty(0) });
    assert_eq!(bar(capture.combo)[0], "1 Dash →");
    capture.step(Snap { facing: 1, ..Snap::empty(1) });
    capture.step(Snap { lstick_x: -127, facing: 1, ..Snap::empty(2) });
    assert_eq!(bar(capture.combo)[0], "1 Dash → > Dash ←");

    let mut tracker = Tracker::new();
    let mut capture = InputCapture::new();
    tick(&mut tracker, &mut capture, Sample { airborne: true, ..idle() },
        MappedInputs { lstick_y: -127, ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, Sample { airborne: true, ..idle() }, MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, Sample { airborne: true, ..idle() },
        MappedInputs { buttons: 1, ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, Sample { attack: Some(Move::Aerial), status: 0x36, airborne: true, facing: 1,
        ..Sample::default() }, MappedInputs::NEUTRAL);
    assert_eq!(bar(capture.combo)[0], "1 Aerial");
}

#[test]
fn a_later_aim_parts_step_keeps_its_special_before_a_new_press() {
    let (mut tracker, mut capture) = zips_capture(true);
    let plain = ComboRecord::new("Drill".into(), capture.combo, None);
    let value = serde_json::to_value(&plain).unwrap();
    assert_eq!(bar(plain.combo)[1], "2 Up B →");
    assert_eq!(value["input_steps"][1]["from_shown"], serde_json::json!({"move":"up_special"}));
    assert!(value["input_steps"][1].get("to_shown").is_none());
    assert_eq!(serde_json::from_value::<ComboRecord>(value).unwrap(), plain);

    tick(&mut tracker, &mut capture, idle(), MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 1, ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, Sample { attack: Some(Move::Aerial), status: 0x36, facing: 1,
        ..Sample::default() }, MappedInputs::NEUTRAL);
    let record = ComboRecord::new("Drill".into(), capture.combo, None);
    assert_eq!(record.combo.len(), 2);
    assert_eq!(bar(record.combo)[1], "2 Up B → > Aerial");
    assert_eq!(combo_link_lines(&record.combo)[2], "S2 up b → → aerial");
    let value = serde_json::to_value(&record).unwrap();
    assert_eq!(value["input_steps"][1]["from_shown"], serde_json::json!({"move":"up_special"}));
    assert_eq!(value["input_steps"][1]["to_shown"], serde_json::json!({"move":"aerial"}));
    assert_eq!(serde_json::from_value::<ComboRecord>(value).unwrap(), record);

    tick(&mut tracker, &mut capture, idle(), MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 8, ..MappedInputs::NEUTRAL });
    assert_eq!(bar(capture.combo)[1], "2 Up B → > Shield");
    assert_eq!(side(capture.combo.get(1).unwrap(), 0).map(|d| d.mv), Some(Move::UpSpecial));
    assert_eq!(side(capture.combo.get(1).unwrap(), 1), None);
}

#[test]
fn a_delayed_description_uses_the_first_input_frame() {
    let mut capture = InputCapture::new();
    capture.step(Snap { buttons: 1, facing: 1, ..Snap::empty(0) });
    capture.step(Snap { buttons: 4, facing: 1, ..Snap::empty(1) });
    capture.step_late(Snap { buttons: 4, facing: 1, ..Snap::empty(2) },
        Late { named: Some((Some(0), Move::Jab, None, 0)), ..Late::NONE });
    assert_eq!(bar(capture.combo)[0], "1 Jab > Jump");
    assert_eq!(side(capture.combo.get(0).unwrap(), 0).map(|d| d.mv), Some(Move::Jab));
}

#[test]
fn two_described_actions_survive_saving_and_reloading() {
    let record = ComboRecord::new("Drill".into(), two_described(), None);
    let value = serde_json::to_value(&record).unwrap();
    assert_eq!(value["input_steps"][0]["from_shown"], serde_json::json!({"move":"jab"}));
    assert_eq!(value["input_steps"][0]["to_shown"], serde_json::json!({"move":"neutral_special"}));
    let read: ComboRecord = serde_json::from_value(value).unwrap();
    assert_eq!(read, record);
    let crop = read.crop_units(0, 1).unwrap();
    assert_eq!((side(crop.combo.get(0).unwrap(), 0).map(|d| d.mv),
        side(crop.combo.get(0).unwrap(), 1).map(|d| d.mv)), (Some(Move::Jab), Some(Move::NeutralSpecial)));
    let one = serde_json::to_value(ComboRecord::new("Drill".into(), jab(), None)).unwrap();
    assert!(one["input_steps"][0].get("from_shown").is_none());
    assert_eq!(one["input_steps"][0]["to_shown"], serde_json::json!({"move":"jab"}));
}

#[test]
fn joining_a_split_step_reseats_the_descriptions_of_both_inputs() {
    let value = serde_json::json!({
        "schema_version":2,"kind":"input_drill","name":"Drill",
        "input_steps":[
            {"to_shown":{"move":"jab"},"events":[
                {"kind":"press","button":"attack","dir":"neutral","offset":0}]},
            {"to_shown":{"move":"neutral_special"},"events":[
                {"kind":"release","button":"attack","dir":"neutral","offset":2},
                {"kind":"press","button":"special","dir":"neutral","offset":4}]}
        ]
    });
    let record: ComboRecord = serde_json::from_value(value).unwrap();
    assert_eq!(record.combo.len(), 1);
    let step = record.combo.get(0).unwrap();
    assert_eq!((side(step, 0).map(|d| d.mv), side(step, 1).map(|d| d.mv)),
        (Some(Move::Jab), Some(Move::NeutralSpecial)));
    assert_eq!(bar(record.combo)[0], "1 Jab > Neutral B");
}

#[test]
fn joining_split_steps_preserves_frame_sides_and_aim_led_continuations() {
    let same_frame = serde_json::json!({
        "schema_version":2,"kind":"input_drill","name":"Drill",
        "input_steps":[
            {"to_shown":{"move":"jab"},"events":[
                {"kind":"press","button":"attack","dir":"neutral","offset":0}]},
            {"to_shown":{"move":"jab"},"events":[
                {"kind":"release","button":"attack","dir":"neutral","offset":0},
                {"kind":"cstick","dir":"forward","offset":0}]}
        ]
    });
    let read: ComboRecord = serde_json::from_value(same_frame).unwrap();
    assert_eq!(read.combo.len(), 1);
    assert_eq!(bar(read.combo)[0], "1 Jab");
    assert_eq!((side(read.combo.get(0).unwrap(), 0), side(read.combo.get(0).unwrap(), 1).map(|d| d.mv)),
        (None, Some(Move::Jab)));

    let aim_led = serde_json::json!({
        "schema_version":4,"kind":"input_drill","name":"Drill",
        "input_steps":[
            {"events":[{"kind":"press","button":"special","dir":"up","offset":0}]},
            {"from_shown":{"move":"up_special"},"events":[
                {"kind":"aim","part":2,"dir":"forward","offset":30}]},
            {"to_shown":{"move":"aerial"},"events":[
                {"kind":"release","button":"attack","dir":"neutral","offset":31},
                {"kind":"press","button":"attack","dir":"neutral","offset":32}]}
        ]
    });
    let read: ComboRecord = serde_json::from_value(aim_led).unwrap();
    assert_eq!(read.combo.len(), 2);
    assert_eq!(bar(read.combo)[1], "2 Up B → > Aerial");
    assert_eq!((side(read.combo.get(1).unwrap(), 0).map(|d| d.mv),
        side(read.combo.get(1).unwrap(), 1).map(|d| d.mv)), (Some(Move::UpSpecial), Some(Move::Aerial)));
}

#[test]
fn a_new_capture_and_its_crop_write_descriptions_without_a_parse() {
    let record = ComboRecord::new("Drill".into(), two_described(), None);
    let value = serde_json::to_value(&record).unwrap();
    assert_eq!(value["input_steps"][0]["from_shown"], serde_json::json!({"move":"jab"}));
    assert_eq!(value["input_steps"][0]["to_shown"], serde_json::json!({"move":"neutral_special"}));
    let crop = record.crop_units(0, 1).unwrap();
    let value = serde_json::to_value(&crop).unwrap();
    assert_eq!(value["input_steps"][0]["from_shown"], serde_json::json!({"move":"jab"}));
    assert_eq!(value["input_steps"][0]["to_shown"], serde_json::json!({"move":"neutral_special"}));
}

#[test]
fn authentic_legacy_drills_show_inputs_and_round_trip_without_descriptions() {
    let kirby = include_str!("../../tests/fixtures/drill-step-kirby-v2.json");
    let pikachu = include_str!("../../tests/fixtures/drill-step-pikachu-v4.json");
    for (fixture, titles, menu) in [
        (kirby, &["1 Attack ↑→"][..], &["S1 attack ↑→"][..]),
        (pikachu, &["1 Special ↑", "2 2nd direction →"][..],
            &["S1 special ↑", "S2 2nd direction →"][..]),
    ] {
        let value: serde_json::Value = serde_json::from_str(fixture).unwrap();
        let record: ComboRecord = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(&bar(record.combo)[..titles.len()], titles);
        let lines = combo_link_lines(&record.combo);
        for (i, expected) in menu.iter().enumerate() { assert_eq!(&lines[i * 2], expected); }
        let saved = serde_json::to_value(record).unwrap();
        assert_eq!(saved, value);
        for step in saved["input_steps"].as_array().unwrap() {
            assert!(step.get("from_shown").is_none());
            assert!(step.get("to_shown").is_none());
        }
    }
}

#[test]
fn a_specials_aim_arrow_stays_with_it_when_a_later_input_is_aerial() {
    let mut tracker = Tracker::new();
    let mut capture = InputCapture::new();
    for frame in 0..14 {
        tick(&mut tracker, &mut capture, zip_sample(Move::UpSpecial, None),
            MappedInputs { buttons: if frame < 3 { 2 } else { 0 }, lstick_y: 127, ..MappedInputs::NEUTRAL });
    }
    for frame in 0..5 {
        tick(&mut tracker, &mut capture, zip_sample(Move::Other, Some((-11.0 * frame as f32, 0.0))),
            MappedInputs { lstick_x: -100, ..MappedInputs::NEUTRAL });
    }
    tick(&mut tracker, &mut capture, idle(), MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 1, ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, Sample { attack: Some(Move::Aerial), status: 0x36, facing: 1,
        ..Sample::default() }, MappedInputs::NEUTRAL);
    assert_eq!(capture.combo.len(), 1);
    assert!(capture.combo.get(0).unwrap().events().any(|e| e.kind == Kind::Aim(1) && e.dir == Dir::Back));
    assert_eq!(bar(capture.combo)[0], "1 Up B ← > Aerial");
    assert_eq!(combo_link_lines(&capture.combo)[0], "S1 up b ← → aerial");
}

#[test]
fn aim_arrows_follow_the_last_described_special_not_an_undescribed_input() {
    let mut rec = LinkRec { events: retain([
        event(Kind::Press(Button::Special), Dir::Up, 0),
        event(Kind::Aim(1), Dir::Back, 4),
        event(Kind::Press(Button::Attack), Dir::Neutral, 5),
    ]), ..LinkRec::INPUT };
    rec.set_step_side(0, Some(desc(Move::UpSpecial, None, None, 0)));
    assert_eq!(title(&rec, false), "Up B ← > Attack");
    rec.set_step_side(1, Some(desc(Move::NeutralSpecial, None, None, 0)));
    assert_eq!(title(&rec, false), "Up B > Neutral B ←");
    rec.set_step_side(1, Some(desc(Move::Aerial, None, None, 0)));
    assert_eq!(title(&rec, false), "Up B ← > Aerial");
}

#[test]
fn an_own_status_updates_the_same_press_without_losing_its_base() {
    let mut tracker = Tracker::new();
    let mut capture = InputCapture::new();
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 2, lstick_y: 127, ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, zip_sample(Move::UpSpecial, None), MappedInputs::NEUTRAL);
    assert_eq!(capture.combo.get(0).unwrap().step_desc().map(|d| d.mv), Some(Move::UpSpecial));
    tick(&mut tracker, &mut capture, zip_sample(Move::Other, Some((0.0, 0.0))), MappedInputs::NEUTRAL);
    let described = capture.combo.get(0).unwrap().step_desc();
    assert_eq!(described.map(|d| (d.mv, d.of)), Some((Move::Other, Some(Move::UpSpecial))));
    assert_eq!(title(capture.combo.get(0).unwrap(), false), "Up B");
}

#[test]
fn a_move_buffered_before_the_take_does_not_describe_a_later_step() {
    let mut tracker = Tracker::new();
    tracker.step(idle(), idle(), MappedInputs { buttons: 1, ..MappedInputs::NEUTRAL });
    let mut capture = InputCapture::new();
    tick(&mut tracker, &mut capture, Sample { attack: Some(Move::Jab), status: 0x27, facing: 1,
        ..Sample::default() }, MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, idle(), MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 8, ..MappedInputs::NEUTRAL });
    assert_eq!(capture.combo.len(), 1);
    assert_eq!(capture.combo.get(0).unwrap().step_desc(), None);
}

#[test]
fn a_step_is_described_by_the_links_own_packing() {
    for expected in [
        desc(Move::SideTilt, None, Some(Angle::Up), 0),
        desc(Move::Other, Some(Move::UpSpecial), None, 0),
        desc(Move::Throw, None, None, 22),
    ] {
        let mut rec = LinkRec::INPUT;
        rec.set_step(expected);
        assert_eq!(rec.step_desc(), Some(expected));
    }
    assert_eq!(core::mem::size_of::<LinkRec>(), 124);
}

#[test]
fn a_step_description_adds_no_bytes() {
    assert_eq!(core::mem::size_of::<LinkRec>(), 124);
    assert_eq!(core::mem::size_of::<Combo>(), 2000);
}

#[test]
fn the_bar_and_the_menu_show_a_steps_move() {
    let combo = jab();
    assert_eq!(combo_link_lines(&combo)[0], "S1 jab");
    assert_eq!(bar(combo)[0], "1 Jab");
}

#[test]
fn no_drill_view_says_step() {
    let mut no_action = InputCapture::new();
    no_action.step(Snap { buttons: 8, facing: 1, ..Snap::empty(0) });
    let old: ComboRecord = serde_json::from_value(serde_json::json!({
        "schema_version": 2, "kind": "input_drill", "name": "old",
        "input_steps": [{"events":[{"kind":"press","button":"special","dir":"up","offset":0}]}]
    })).unwrap();
    for combo in [jab(), no_action.combo, old.combo] {
        assert!(combo_link_lines(&combo).iter().all(|line| !line.to_lowercase().contains("step")));
        assert!(bar(combo).iter().all(|line| !line.to_lowercase().contains("step")));
    }
}

#[test]
fn a_step_whose_action_never_opened_shows_its_input() {
    let mut capture = InputCapture::new();
    capture.step(Snap { buttons: 8, facing: 1, ..Snap::empty(0) });
    capture.step(Snap { facing: 1, ..Snap::empty(1) });
    capture.step(Snap { buttons: 4, lstick_y: 127, facing: 1, ..Snap::empty(30) });
    let combo = capture.combo;
    assert_eq!(combo.len(), 2);
    assert_eq!(combo.get(0).unwrap().step_desc(), None);
    assert_eq!(combo.get(1).unwrap().step_desc(), None);
    assert_eq!(bar(combo)[..2], ["1 Shield", "2 Jump ↑"]);
    assert_eq!(combo_link_lines(&combo)[0], "S1 shield");
    assert_eq!(combo_link_lines(&combo)[2], "S2 jump ↑");
}

#[test]
fn undescribed_titles_name_originating_gestures_and_fall_back_to_the_first_event() {
    for (kind, dir, expected) in [
        (Kind::CStick, Dir::Forward, "C-stick →"),
        (Kind::Flick(Flick::Dash), Dir::Forward, "Dash →"),
        (Kind::Flick(Flick::FastFall), Dir::Down, "Fast-fall ↓"),
        (Kind::Flick(Flick::TapJump), Dir::Up, "Tap-jump ↑"),
        (Kind::Aim(2), Dir::Forward, "2nd direction →"),
    ] {
        let rec = LinkRec { events: retain([event(kind, dir, 0)]), ..LinkRec::INPUT };
        assert_eq!(title(&rec, false), expected);
    }
    let rec = LinkRec { events: retain([
        event(Kind::Release(Button::Jump), Dir::Neutral, 0),
        event(Kind::Press(Button::Shield), Dir::Up, 1),
    ]), ..LinkRec::INPUT };
    assert_eq!(title(&rec, false), "Shield ↑");
}

#[test]
fn a_step_title_follows_the_combo_cells_capacity_rule() {
    let rec = step([
        event(Kind::Press(Button::Special), Dir::Up, 0),
        event(Kind::Aim(1), Dir::BackUp, 1),
        event(Kind::Aim(2), Dir::ForwardDown, 2),
    ], desc(Move::UpSpecial, None, None, 0));
    let capacity = usize::from(cb::units(combo_plan::at::TITLE));
    let loose = title(&rec, false);
    let tight = title(&rec, true);
    assert_eq!(loose, "Up B ↑← ↓→");
    assert_eq!(tight, "Up B↑←↓→");
    assert!(format!("1 {loose}").encode_utf16().count() <= capacity);
    assert!(format!("1 {tight}").encode_utf16().count() <= capacity);
    assert_eq!(bar(drill([rec]))[0], format!("1 {loose}"));
}

#[test]
fn a_steps_move_survives_saving_and_reloading() {
    let step = step([event(Kind::Press(Button::Attack), Dir::ForwardUp, 0)],
        desc(Move::SideTilt, None, Some(Angle::Up), 0));
    let record = ComboRecord::new("Drill".into(), drill([step]), None);
    let value = serde_json::to_value(&record).unwrap();
    assert_eq!(value["input_steps"][0]["to_shown"], serde_json::json!({"move":"side_tilt","angle":"up"}));
    let read: ComboRecord = serde_json::from_value(value).unwrap();
    assert_eq!(read, record);
    assert_eq!(read.combo.get(0).unwrap().step_desc(), step.step_desc());
    assert_eq!(combo_link_lines(&read.combo), combo_link_lines(&record.combo));
}

#[test]
fn a_steps_shown_requires_a_valid_move_and_accepts_all_move_keys() {
    for (mv, key) in [
        (Move::Jab, "jab"), (Move::DashAttack, "dash_attack"), (Move::SideTilt, "side_tilt"),
        (Move::UpTilt, "up_tilt"), (Move::DownTilt, "down_tilt"), (Move::SideSmash, "side_smash"),
        (Move::UpSmash, "up_smash"), (Move::DownSmash, "down_smash"), (Move::Aerial, "aerial"),
        (Move::NeutralSpecial, "neutral_special"), (Move::SideSpecial, "side_special"),
        (Move::UpSpecial, "up_special"), (Move::DownSpecial, "down_special"), (Move::Grab, "grab"),
        (Move::Throw, "throw"), (Move::FinalSmash, "final_smash"), (Move::Other, "other"),
    ] {
        let rec = step([event(Kind::Press(Button::Attack), Dir::Neutral, 0)], desc(mv, None, None, 0));
        let record = ComboRecord::new("Drill".into(), drill([rec]), None);
        let value = serde_json::to_value(&record).unwrap();
        assert_eq!(value["input_steps"][0]["to_shown"]["move"], key);
        assert_eq!(serde_json::from_value::<ComboRecord>(value).unwrap(), record);
    }
    for shown in [serde_json::json!({}), serde_json::json!({"move":"not_a_move"}), serde_json::json!(null)] {
        let value = serde_json::json!({
            "schema_version":2,"kind":"input_drill","name":"Drill",
            "input_steps":[{"to_shown":shown,"events":[{"kind":"press","button":"attack","dir":"neutral","offset":0}]}]
        });
        assert!(serde_json::from_value::<ComboRecord>(value).is_err());
    }
}

#[test]
fn a_joined_split_step_keeps_its_move() {
    let value = serde_json::json!({
        "schema_version":4,"kind":"input_drill","name":"Drill",
        "input_steps":[
            {"to_shown":{"move":"up_special"},"events":[
                {"kind":"press","button":"special","dir":"up","offset":0},
                {"kind":"release","button":"special","dir":"back","offset":3}]},
            {"events":[{"kind":"aim","part":1,"dir":"back","offset":16}]}
        ]
    });
    let read: ComboRecord = serde_json::from_value(value).unwrap();
    assert_eq!(read.combo.len(), 1);
    assert_eq!(read.combo.get(0).unwrap().step_desc().map(|d| (d.mv, d.parts[0])),
        Some((Move::UpSpecial, Some(Dir::Back))));
    assert_eq!(combo_link_lines(&read.combo)[0], "S1 up b ←");
}

#[test]
fn a_cropped_drill_keeps_each_steps_move() {
    let steps = [
        step([event(Kind::Press(Button::Attack), Dir::Neutral, 0)], desc(Move::Jab, None, None, 0)),
        step([event(Kind::Press(Button::Attack), Dir::ForwardUp, 30)], desc(Move::SideTilt, None, Some(Angle::Up), 0)),
        step([event(Kind::Press(Button::Grab), Dir::Neutral, 60)], desc(Move::Grab, None, None, 18)),
    ];
    let cropped = ComboRecord::new("Drill".into(), drill(steps), None).crop_units(1, 3).unwrap();
    assert_eq!(cropped.combo.get(0).unwrap().step_desc(), Some(desc(Move::SideTilt, None, Some(Angle::Up), 0)));
    assert_eq!(cropped.combo.get(1).unwrap().step_desc(), Some(desc(Move::Grab, None, None, 18)));
    assert_eq!(combo_link_lines(&cropped.combo)[0], "S1 side tilt ↑");
    assert_eq!(combo_link_lines(&cropped.combo)[2], "S2 grab");
}

#[test]
fn an_aimed_specials_steps_show_their_arrows() {
    let combo = zips(true);
    assert_eq!(combo.len(), 2);
    assert_eq!(combo.get(1).unwrap().events().next().unwrap().kind, Kind::Aim(2));
    assert_eq!(bar(combo)[..2], ["1 Up B ←", "2 Up B →"]);
    let mut tracker = Tracker::new();
    let mut capture = InputCapture::new();
    for frame in 0..10 {
        tick(&mut tracker, &mut capture, zip_sample(Move::UpSpecial, None),
            MappedInputs { buttons: if frame < 3 { 2 } else { 0 }, lstick_y: 127, ..MappedInputs::NEUTRAL });
    }
    assert_eq!(bar(capture.combo)[0], "1 Up B");
}

#[test]
fn an_angled_tilts_step_keeps_its_arrow_after_it_ends() {
    let mut tracker = Tracker::new();
    let mut capture = InputCapture::new();
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 1, lstick_x: 60, ..MappedInputs::NEUTRAL });
    for frame in 0..4 {
        tick(&mut tracker, &mut capture, Sample { attack: Some(Move::SideTilt), angle: Some(Angle::Up),
            status: 2, motion_frame: frame as f32, facing: 1, ..Sample::default() }, MappedInputs::NEUTRAL);
    }
    assert_eq!(title(capture.combo.get(0).unwrap(), false), "Side tilt ↑");
    tick(&mut tracker, &mut capture, idle(), MappedInputs::NEUTRAL);
    assert_eq!(title(capture.combo.get(0).unwrap(), false), "Side tilt ↑");
    for _ in 0..30 { tick(&mut tracker, &mut capture, idle(), MappedInputs::NEUTRAL); }
    assert_eq!(title(capture.combo.get(0).unwrap(), false), "Side tilt ↑");
}

#[test]
fn a_grab_step_becomes_a_throw_when_it_throws() {
    let mut tracker = Tracker::new();
    let mut capture = InputCapture::new();
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 0x10, ..MappedInputs::NEUTRAL });
    for status in [0x37, 0x3c] {
        tick(&mut tracker, &mut capture, Sample { named: Some(Move::Grab), status, motion: 0x05_c56b_7b64,
            facing: 1, ..Sample::default() }, MappedInputs::NEUTRAL);
    }
    assert_eq!(title(capture.combo.get(0).unwrap(), false), "Grab");
    tick(&mut tracker, &mut capture, Sample { named: Some(Move::Throw), status: 0x40, motion: 0x07_49f7_09a8,
        facing: 1, ..Sample::default() }, MappedInputs::NEUTRAL);
    assert_eq!(capture.combo.len(), 1);
    assert_eq!(title(capture.combo.get(0).unwrap(), false), "Throw");
    assert_eq!(capture.combo.get(0).unwrap().step_desc().unwrap().normal, 22);
}

#[test]
fn an_old_drill_file_shows_its_inputs_and_is_written_back_unchanged() {
    for version in [2, 4] {
        let mut value = serde_json::json!({
            "schema_version":version,"kind":"input_drill","name":"Drill",
            "input_steps":[{"events":[{"kind":"press","button":"special","dir":"up","offset":0}]}]
        });
        if version == 4 {
            value["input_steps"].as_array_mut().unwrap().push(serde_json::json!({
                "events":[{"kind":"aim","part":2,"dir":"forward","offset":30}]
            }));
        }
        let read: ComboRecord = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(read.combo.get(0).unwrap().step_desc(), None);
        assert_eq!(bar(read.combo)[0], "1 Special ↑");
        assert_eq!(combo_link_lines(&read.combo)[0], "S1 special ↑");
        if version == 4 {
            assert_eq!(bar(read.combo)[1], "2 2nd direction →");
            assert_eq!(combo_link_lines(&read.combo)[2], "S2 2nd direction →");
        }
        assert_eq!(serde_json::to_value(read).unwrap(), value);
    }
}
}
