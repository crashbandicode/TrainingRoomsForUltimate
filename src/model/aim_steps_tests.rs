//! Trace-derived directional-special steps and legacy compatibility.
lib_unit_tests! {
use super::*;
use crate::model::aim::{Now, Parts, Seen};
use crate::model::combo_record::ComboRecord;
use crate::model::frame_strip::Sample;
use crate::model::input_delay::MappedInputs;

fn now(start: bool, special: bool, travel: bool, entered: bool, aimed: Option<u8>, facing: i8) -> Now {
    Now { aimer: true, start, special, travel, entered, aimed, facing }
}

#[test]
fn parts_count_two_travels_and_report_each_once() {
    let mut parts = Parts::new();
    let mut seen = Vec::new();
    for _ in 0..14 { seen.push(parts.step(now(true, true, false, false, None, 1))); }
    for f in 0..5 { seen.push(parts.step(now(false, true, true, f == 0, (f == 1).then_some(4), -1))); }
    for _ in 0..9 { seen.push(parts.step(now(false, true, false, false, None, -1))); }
    for f in 0..5 { seen.push(parts.step(now(false, true, true, f == 0, (f == 1).then_some(0), -1))); }
    seen.push(parts.step(now(false, true, false, false, None, -1)));
    assert!(seen.iter().all(|s| s.inside && s.facing == 1));
    assert_eq!(seen.iter().filter(|s| s.part != 0).map(|s| (s.part, s.dir, s.age)).collect::<Vec<_>>(),
        vec![(1, Some(4), 1), (2, Some(0), 1)]);
}

#[test]
fn a_travel_that_never_moves_reports_no_direction() {
    let mut parts = Parts::new();
    parts.step(now(true, true, false, false, None, 1));
    for f in 0..5 {
        assert_eq!(parts.step(now(false, true, true, f == 0, None, -1)).part, 0);
    }
    let end = parts.step(now(false, true, false, false, None, -1));
    assert_eq!((end.inside, end.part, end.dir, end.facing, end.age), (true, 1, None, 1, 5));
    assert_eq!(parts.step(now(false, true, false, false, None, -1)).part, 0);
}

#[test]
fn a_third_travel_and_another_move_are_not_parts() {
    let mut parts = Parts::new();
    assert_eq!(parts.step(now(false, true, false, false, None, 1)), Seen::NONE);
    let mut unsupported = now(true, true, true, true, Some(4), 1);
    unsupported.aimer = false;
    assert_eq!(parts.step(unsupported), Seen::NONE);
    parts.step(now(true, true, false, false, None, 1));
    for part in 1..=3 {
        let got = parts.step(now(false, true, true, true, Some(4), -1));
        assert_eq!(got.part, if part < 3 { part } else { 0 });
        parts.step(now(false, true, false, false, None, -1));
    }
    assert_eq!(parts.step(now(false, false, false, false, None, -1)), Seen::NONE);
    parts.step(now(true, true, false, false, None, 1));
    assert_eq!(parts.step(now(false, true, true, true, Some(0), 1)).part, 1);
}

#[test]
fn a_pending_travel_reports_once_when_the_special_is_left() {
    let mut parts = Parts::new();
    parts.step(now(true, true, false, false, None, 1));
    parts.step(now(false, true, true, true, None, -1));
    parts.step(now(false, true, true, false, None, -1));
    let end = parts.step(now(false, false, false, false, None, -1));
    assert_eq!((end.inside, end.part, end.dir, end.facing, end.age), (false, 1, None, 1, 2));
    assert_eq!(parts.step(now(false, false, false, false, None, -1)), Seen::NONE);
}

#[test]
fn a_long_pending_travel_saturates_its_age() {
    let mut parts = Parts::new();
    parts.step(now(true, true, false, false, None, 1));
    for f in 0..300 {
        parts.step(now(false, true, true, f == 0, None, 1));
    }
    let end = parts.step(now(false, true, false, false, None, 1));
    assert_eq!((end.part, end.dir, end.age), (1, None, u8::MAX));
}

fn strip_sample(attack: Move, travel: Option<(f32, f32)>, facing: i8) -> Sample {
    Sample { attack: Some(attack), travel, aimer: true, facing, ..Sample::default() }
}

#[test]
fn the_strip_writes_each_part_on_the_snap_it_was_known() {
    fn run(start: usize, travel: usize, between: usize, first: (f32, f32), second: (f32, f32)) -> Vec<Seen> {
        let mut tracker = Tracker::new();
        for _ in 0..start {
            tracker.step(strip_sample(Move::UpSpecial, None, 1), Sample::default(), MappedInputs::NEUTRAL);
            assert!(tracker.snaps().last().unwrap().aim.inside);
        }
        let mut entry_frames = Vec::new();
        for (direction, length) in [(first, travel), (second, travel)] {
            let entry = tracker.frame() + 1;
            entry_frames.push(entry);
            for f in 0..length {
                tracker.step(strip_sample(Move::Other, Some((direction.0 * f as f32, direction.1 * f as f32)), -1),
                    Sample::default(), MappedInputs::NEUTRAL);
                assert_eq!(tracker.aim_pending(), f == 0);
            }
            for _ in 0..between {
                tracker.step(strip_sample(Move::Other, None, -1), Sample::default(), MappedInputs::NEUTRAL);
            }
        }
        let reports: Vec<_> = tracker.snaps().filter(|s| s.aim.part != 0).collect();
        assert_eq!(reports.len(), 2);
        for (snap, entry) in reports.iter().zip(entry_frames) {
            assert_eq!(snap.frame - u32::from(snap.aim.age), entry);
            assert_eq!(snap.aim.facing, 1);
        }
        reports.into_iter().map(|s| s.aim).collect()
    }
    let pika = run(14, 5, 9, (-11.0, 0.0), (9.9, 0.0));
    assert_eq!(pika.iter().map(|s| (s.part, s.dir)).collect::<Vec<_>>(), vec![(1, Some(4)), (2, Some(0))]);
    let greninja = run(16, 11, 1, (-2.62, 2.62), (2.35, 2.35));
    assert_eq!(greninja.iter().map(|s| (s.part, s.dir)).collect::<Vec<_>>(), vec![(1, Some(3)), (2, Some(1))]);
}

fn script(frame: u32, first: Option<u8>, second: Option<u8>, dash: bool, attack: bool) -> Snap {
    let part = if frame == 16 && first.is_some() { 1 } else if frame == 30 && second.is_some() { 2 } else { 0 };
    let dir = if part == 1 { first } else if part == 2 { second } else { None };
    Snap {
        buttons: if frame < 3 { 2 } else if attack && frame == 40 { 1 } else { 0 },
        lstick_x: if frame < 3 || !dash { 0 } else if frame < 20 { -127 } else { 127 },
        lstick_y: if frame < 3 { 127 } else { 0 },
        facing: 1,
        aim: Seen { inside: frame <= 35, part, dir, facing: 1, age: u8::from(part != 0) },
        ..Snap::empty(frame)
    }
}

fn captured(first: Option<u8>, second: Option<u8>, dash: bool, attack: bool, aims: bool) -> Combo {
    let mut capture = InputCapture::new();
    for frame in 0..=42 {
        capture.step_mode(script(frame, first, second, dash, attack), true, aims);
    }
    capture.combo
}

fn run(target: Combo, first: Option<u8>, second: Option<u8>, dash: bool, attack: bool) -> Summary {
    let mut helper = Helper::new();
    helper.set_target(Some(target));
    for frame in 0..=45 {
        helper.step_inputs(script(frame, first, second, dash, attack), &mut Out::default());
    }
    let result = *helper.results().next().expect("attempt resolved");
    result
}

#[test]
fn a_two_zip_drill_is_two_steps_without_stick_gestures() {
    let combo = captured(Some(4), Some(0), true, false, true);
    assert_eq!(combo.len(), 2);
    let events = |i| combo.get(i).unwrap().events().map(|e| (e.kind, e.dir)).collect::<Vec<_>>();
    assert_eq!(events(0), vec![(Kind::Press(Button::Special), Dir::Up), (Kind::Release(Button::Special), Dir::Back), (Kind::Aim(1), Dir::Back)]);
    assert_eq!(events(1), vec![(Kind::Aim(2), Dir::Forward)]);
    assert!(!combo.links().flat_map(LinkRec::events).any(|e| matches!(e.kind, Kind::Flick(_))));
}

#[test]
fn the_two_zip_drill_passes_only_with_both_directions() {
    let target = captured(Some(4), Some(0), true, false, true);
    let same = run(target, Some(4), Some(0), true, false);
    assert!(same.clean);
    assert_eq!(same.graded, 2);
    assert_eq!(run(target, Some(2), None, true, false).fault, Some((0, Reason::WrongAim(1))));
    assert_eq!(run(target, Some(4), None, true, false).fault, Some((1, Reason::MissingAim(2))));
    assert_eq!(run(target, Some(4), Some(4), true, false).fault, Some((1, Reason::WrongAim(2))));
}

#[test]
fn an_attempt_that_adds_a_second_part_fails_a_one_part_drill_step() {
    let target = captured(Some(4), None, true, true, true);
    assert_eq!(target.len(), 2);
    let result = run(target, Some(4), Some(0), true, true);
    assert!(!result.clean);
    assert_eq!(result.fault, Some((1, Reason::ExtraAim(2))));
}

#[test]
fn an_old_drill_grades_as_before() {
    let old = captured(Some(4), None, true, false, false);
    assert!(!old.has_aim());
    assert!(old.links().flat_map(LinkRec::events).any(|e| e.kind == Kind::Flick(Flick::Dash)));
    assert!(run(old, Some(4), Some(0), true, false).clean);
    assert_eq!(run(old, Some(4), Some(0), false, false).fault, Some((0, Reason::MissingFlick(Flick::Dash))));
}

fn event(kind: Kind, dir: Dir, offset: i32) -> Event { Event { kind, dir, offset } }
fn link(events: impl IntoIterator<Item = Event>) -> LinkRec {
    LinkRec { to: Move::UpSpecial, verdict: Verdict::True(None), events: retain(events), ..LinkRec::INPUT }
}
fn summary(g: Grade) -> Summary {
    let mut attempt = Attempt::new(1, false);
    attempt.grades[0] = Some(g);
    attempt.len = 1;
    summarize(&attempt, 1, Outcome::Completed, 1, false)
}

#[test]
fn a_hit_combo_tells_up_b_left_from_up_b_right() {
    let press = event(Kind::Press(Button::Special), Dir::Up, 0);
    let left = event(Kind::Aim(1), Dir::Back, 16);
    let reference = link([press, left]);
    let wrong = grade_mode(&reference, &link([press, event(Kind::Aim(1), Dir::Forward, 20)]), false, true);
    assert!(wrong.wrong_dir().is_some());
    assert!(!wrong.clean(1));
    assert_eq!(summary(wrong).fault, Some((0, Reason::WrongAim(1))));
    let missing = grade_mode(&reference, &link([press]), false, true);
    assert_eq!(summary(missing).fault, Some((0, Reason::MissingAim(1))));
    let extra = grade_mode(&reference, &link([press, left, event(Kind::Aim(2), Dir::Forward, 30)]), false, true);
    assert_eq!(extra.extra_aim, Some(2));
    let late = grade_mode(&reference, &link([press, event(Kind::Aim(1), Dir::Back, 40)]), false, true);
    assert_eq!(late.marks[1], Some(Mark::At(0)));
    assert!(late.clean(1));
    let old = grade_mode(&link([press]), &link([press, left]), false, false);
    assert!(old.clean(1));
}

#[test]
fn a_legacy_hit_reference_ignores_an_extra_aim() {
    let press = event(Kind::Press(Button::Special), Dir::Up, 0);
    let old = grade_mode(&link([press]), &link([press, event(Kind::Aim(1), Dir::Back, 16)]), false, false);
    assert!(old.clean(1));
}

#[test]
fn a_link_takes_the_direction_of_a_travel_that_hit_on_its_first_frame() {
    let mut tracker = Tracker::new();
    tracker.step(strip_sample(Move::UpSpecial, None, 1), Sample::default(), MappedInputs::NEUTRAL);
    let left = MappedInputs { lstick_x: -127, ..MappedInputs::NEUTRAL };
    tracker.step(strip_sample(Move::Other, Some((0.0, 0.0)), 1), Sample::default(), left);
    let hit = tracker.frame();
    tracker.step(strip_sample(Move::Other, Some((-11.0, 0.0)), 1), Sample::default(), left);
    let (events, _) = events_between(&tracker, 0, hit, 1, true);
    assert_eq!(events.iter().flatten().copied().collect::<Vec<_>>(), vec![event(Kind::Aim(1), Dir::Back, 1)]);
    let (old, _) = events_between(&tracker, 0, hit, 1, false);
    assert!(old.iter().flatten().any(|e| e.kind == Kind::Flick(Flick::Dash)));
    assert!(!old.iter().flatten().any(|e| matches!(e.kind, Kind::Aim(_))));
}

#[test]
fn a_hit_link_waits_for_the_pending_heading() {
    let mut tracker = Tracker::new();
    let mut helper = Helper::new();
    let hit_cpu = Sample { stop: true, hitstun: true, reaction: Some(20.0), ..Sample::default() };
    let stunned_cpu = Sample { hitstun: true, reaction: Some(19.0), ..Sample::default() };
    let jab = Sample { attack: Some(Move::Jab), status: 0x27, facing: 1, ..Sample::default() };
    tracker.step(jab, hit_cpu, MappedInputs::NEUTRAL);
    helper.step(&tracker, Settings::factory());
    tracker.step(Sample { actionable: true, facing: 1, ..Sample::default() }, stunned_cpu, MappedInputs::NEUTRAL);
    helper.step(&tracker, Settings::factory());
    let up_b = MappedInputs { buttons: 2, lstick_y: 127, ..MappedInputs::NEUTRAL };
    tracker.step(strip_sample(Move::UpSpecial, None, 1), stunned_cpu, up_b);
    helper.step(&tracker, Settings::factory());
    let left = MappedInputs { lstick_x: -127, ..MappedInputs::NEUTRAL };
    tracker.step(strip_sample(Move::Other, Some((0.0, 0.0)), 1), hit_cpu, left);
    helper.step(&tracker, Settings::factory());
    assert!(tracker.latest_link().is_some());
    assert!(tracker.aim_pending());
    assert!(helper.capture.current.is_empty(), "the hit frame must wait for its heading");
    tracker.step(strip_sample(Move::Other, Some((-11.0, 0.0)), 1), hit_cpu, left);
    helper.step(&tracker, Settings::factory());
    let rec = helper.capture.current.get(0).expect("the pending link is captured");
    assert!(rec.events().any(|e| e.kind == Kind::Aim(1) && e.dir == Dir::Back));
}

#[test]
fn aim_files_are_versions_three_and_four_and_old_files_read_as_before() {
    let drill = captured(Some(4), Some(0), true, false, true);
    let record = ComboRecord::new("Drill 1".into(), drill, Some("Mario".into()));
    let value = serde_json::to_value(&record).unwrap();
    assert_eq!(value["schema_version"], 4);
    assert_eq!(value["kind"], "input_drill");
    assert!(value["input_steps"][0]["events"].as_array().unwrap().iter().any(|e| e["kind"] == "aim" && e["part"] == 1 && e["dir"] == "back"));
    assert_eq!(serde_json::from_value::<ComboRecord>(value).unwrap(), record);
    let hit = ComboRecord::new("Hit".into(), Combo::from_links([link([event(Kind::Aim(1), Dir::Back, 16)])]).unwrap(), None);
    let value = serde_json::to_value(&hit).unwrap();
    assert_eq!(value["schema_version"], 3);
    assert_eq!(serde_json::from_value::<ComboRecord>(value.clone()).unwrap(), hit);
    let root = std::env::temp_dir().join(format!("tr-aim-steps-{}", std::process::id()));
    let lib = crate::model::combo_record::ComboLibrary::new(&root);
    let drill_id = lib.save_new(&record).unwrap();
    let hit_id = lib.save_new(&hit).unwrap();
    assert_eq!(lib.load(&drill_id).unwrap(), record);
    assert_eq!(lib.load(&hit_id).unwrap(), hit);
    std::fs::remove_dir_all(root).unwrap();
    let old = r#"{"input_steps":[{"events":[{"button":"attack","dir":"neutral","kind":"press","offset":0},{"button":"attack","dir":"neutral","kind":"release","offset":1}]}],"kind":"input_drill","name":"Drill 1","recorded_on":"Mario","schema_version":2}"#;
    let parsed: ComboRecord = serde_json::from_str(old).unwrap();
    assert!(!parsed.combo.has_aim());
    assert_eq!(serde_json::to_string(&parsed).unwrap(), old);
    let mut bad: serde_json::Value = serde_json::from_str(old).unwrap();
    bad["input_steps"][0]["events"].as_array_mut().unwrap().push(serde_json::json!({"kind":"aim","part":1,"dir":"back","offset":2}));
    assert!(serde_json::from_value::<ComboRecord>(bad.clone()).unwrap_err().to_string().contains("event"));
    bad["schema_version"] = 5.into();
    assert!(serde_json::from_value::<ComboRecord>(bad.clone()).unwrap_err().to_string().contains("schema_version"));
    bad["schema_version"] = 4.into();
    bad["input_steps"][0]["events"][2]["part"] = 3.into();
    assert!(serde_json::from_value::<ComboRecord>(bad).unwrap_err().to_string().contains("event"));
}

#[test]
fn old_drill_file_roundtrips_and_refuses_an_aim_in_version_two() {
    let old = r#"{"input_steps":[{"events":[{"button":"attack","dir":"neutral","kind":"press","offset":0},{"button":"attack","dir":"neutral","kind":"release","offset":1}]}],"kind":"input_drill","name":"Drill 1","recorded_on":"Mario","schema_version":2}"#;
    let parsed: ComboRecord = serde_json::from_str(old).unwrap();
    assert!(!parsed.combo.has_aim());
    assert_eq!(serde_json::to_string(&parsed).unwrap(), old);
    let mut bad: serde_json::Value = serde_json::from_str(old).unwrap();
    bad["input_steps"][0]["events"].as_array_mut().unwrap().push(serde_json::json!({"kind":"aim","part":1,"dir":"back","offset":2}));
    assert!(serde_json::from_value::<ComboRecord>(bad.clone()).unwrap_err().to_string().contains("event"));
    bad["schema_version"] = 5.into();
    assert!(serde_json::from_value::<ComboRecord>(bad).unwrap_err().to_string().contains("schema_version"));
}

#[test]
fn the_drill_reads_its_directions_and_names_its_faults() {
    use crate::model::frame_strip_plan::Text;
    let drill = captured(Some(4), Some(0), true, false, true);
    assert_eq!(crate::model::recording_menu::combo_link_lines(&drill), vec!["S1 Step 1", "↑ B · release · 1st ←", "S2 Step 2", "2nd →"]);
    assert_eq!(crate::model::combo_plan::mark_text(Mark::WrongDir, "1st direction", 1).0, Text::new("1st direction wrong"));
    assert_eq!(crate::model::combo_plan::mark_text(Mark::WrongDir, "C-stick", 1).0, Text::new("C-stick wrong direction"));
}
}
