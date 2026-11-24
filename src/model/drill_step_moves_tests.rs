//! EXPECTED RED on e09fc3cf: drill step descriptions, titles, persistence, aim, tilt and throw.
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

fn zips(second: bool) -> Combo {
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
    capture.combo
}

#[test]
fn a_drill_step_stores_the_move_its_press_became() {
    assert_eq!(jab().get(0).unwrap().step_desc().map(|d| d.mv), Some(Move::Jab));
}

#[test]
fn the_first_move_in_a_step_is_not_replaced_by_a_later_press() {
    let mut tracker = Tracker::new();
    let mut capture = InputCapture::new();
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 1, ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, Sample { attack: Some(Move::Jab), status: 0x27, facing: 1,
        ..Sample::default() }, MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, idle(), MappedInputs::NEUTRAL);
    tick(&mut tracker, &mut capture, idle(), MappedInputs { buttons: 2, lstick_y: 127, ..MappedInputs::NEUTRAL });
    tick(&mut tracker, &mut capture, zip_sample(Move::UpSpecial, None), MappedInputs::NEUTRAL);
    assert_eq!(capture.combo.len(), 1);
    assert_eq!(capture.combo.get(0).unwrap().step_desc().map(|d| d.mv), Some(Move::Jab));
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
    assert_eq!(value["input_steps"][0]["shown"], serde_json::json!({"move":"side_tilt","angle":"up"}));
    let read: ComboRecord = serde_json::from_value(value).unwrap();
    assert_eq!(read, record);
    assert_eq!(read.combo.get(0).unwrap().step_desc(), step.step_desc());
    assert_eq!(combo_link_lines(&read.combo), combo_link_lines(&record.combo));
}

#[test]
fn a_joined_split_step_keeps_its_move() {
    let value = serde_json::json!({
        "schema_version":4,"kind":"input_drill","name":"Drill",
        "input_steps":[
            {"shown":{"move":"up_special"},"events":[
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
