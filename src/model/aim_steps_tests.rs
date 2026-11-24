//! Trace-derived directional-special steps and legacy compatibility.
lib_unit_tests! {
use super::*;
use crate::model::aim::Seen;
use crate::model::combo_record::ComboRecord;
use crate::model::frame_strip::Sample;
use crate::model::input_delay::MappedInputs;

fn strip_sample(attack: Move, pos: Option<(f32, f32)>, facing: i8) -> Sample {
    let status = if attack == Move::UpSpecial { 1 } else if pos.is_some() { 2 } else { 3 };
    Sample { attack: Some(attack), status, pos: Some(pos.unwrap_or((0.0, 0.0))), facing, ..Sample::default() }
}

#[test]
fn the_strip_writes_each_part_on_the_snap_it_was_known() {
    fn run(start: usize, travel: usize, between: usize, first: (f32, f32), second: (f32, f32)) -> Vec<Seen> {
        let mut tracker = Tracker::new();
        let axis = |value: f32| if value > 0.0 { 100 } else if value < 0.0 { -100 } else { 0 };
        let stick = |direction: (f32, f32)| MappedInputs {
            lstick_x: axis(direction.0),
            lstick_y: axis(direction.1),
            ..MappedInputs::NEUTRAL
        };
        for _ in 0..start {
            tracker.step(strip_sample(Move::UpSpecial, None, 1), Sample::default(), stick(first));
            assert!(tracker.snaps().last().unwrap().aim.inside);
        }
        let mut entry_frames = Vec::new();
        let mut pos = (0.0, 0.0);
        for (direction, length) in [(first, travel), (second, travel)] {
            let entry = tracker.frame() + 1;
            entry_frames.push(entry);
            for f in 0..length {
                tracker.step(strip_sample(Move::Other, Some((pos.0 + direction.0 * f as f32, pos.1 + direction.1 * f as f32)), -1),
                    Sample::default(), stick(direction));
                assert_eq!(tracker.aim_pending(), (f == 0).then_some(entry));
            }
            pos = (pos.0 + direction.0 * (length - 1) as f32, pos.1 + direction.1 * (length - 1) as f32);
            for _ in 0..between {
                tracker.step(Sample { pos: Some(pos), ..strip_sample(Move::Other, None, -1) }, Sample::default(), stick(second));
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
fn an_active_special_can_still_supply_a_nonfinal_steps_aim() {
    let target = captured(Some(4), Some(0), true, false, true);
    for (first, expected) in [(Some(4), None), (Some(2), Some(Reason::WrongAim(1))), (None, Some(Reason::MissingAim(1)))] {
        let mut helper = Helper::new();
        helper.set_target(Some(target));
        for frame in 0..=36 {
            let mut now = script(frame, None, Some(0), true, false);
            if frame == 19 && first.is_some() {
                now.aim = Seen { part: 1, dir: first, ..now.aim };
            }
            helper.step_inputs(now, &mut Out::default());
            if frame < 19 {
                assert!(helper.results().next().is_none(), "the first part could still arrive at frame {frame}");
            }
        }
        let summary = helper.results().next().expect("the special ended");
        assert_eq!(summary.clean, expected.is_none());
        assert_eq!(summary.fault, expected.map(|why| (0, why)));
    }
}

#[test]
fn waiting_for_direction_does_not_extend_a_button_deadline() {
    let target = captured(Some(4), Some(0), true, false, true);
    let mut helper = Helper::new();
    helper.set_target(Some(target));
    for frame in 0..=17 {
        let mut now = script(frame, None, None, true, false);
        now.buttons = 2;
        helper.step_inputs(now, &mut Out::default());
    }
    let summary = helper.results().next().expect("the saved release is already missing");
    assert_eq!(summary.fault, Some((0, Reason::MissingRelease(Button::Special))));
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

#[test]
fn a_one_zip_drill_fails_a_two_zip_attempt_with_no_later_input() {
    let target = captured(Some(4), None, true, false, true);
    assert_eq!(target.len(), 1);
    let mut helper = Helper::new();
    helper.set_target(Some(target));
    helper.settings = Settings { load_after: true, ..Settings::factory() };
    for frame in 0..=35 {
        let mut out = Out::default();
        helper.step_inputs(script(frame, Some(4), Some(0), true, false), &mut out);
        assert!(helper.results().next().is_none(), "resolved inside at frame {frame}");
        assert_eq!(helper.streak(), 0);
        assert!(!out.load && !helper.pending_load, "load requested inside at frame {frame}");
    }
    let mut out = Out::default();
    helper.step_inputs(script(36, Some(4), Some(0), true, false), &mut out);
    let result = *helper.results().next().expect("result after leaving the special");
    assert!(!result.clean);
    assert_eq!(result.fault, Some((0, Reason::ExtraAim(2))));
    assert_eq!(helper.streak(), 0);
}

#[test]
fn a_one_zip_drill_is_clean_only_after_the_special_ends() {
    let target = captured(Some(4), None, true, false, true);
    assert_eq!(target.len(), 1);
    let mut helper = Helper::new();
    helper.set_target(Some(target));
    helper.settings = Settings { load_after: true, ..Settings::factory() };
    for frame in 0..=35 {
        let mut out = Out::default();
        helper.step_inputs(script(frame, Some(4), None, true, false), &mut out);
        assert!(helper.results().next().is_none(), "resolved inside at frame {frame}");
        assert_eq!(helper.streak(), 0);
        assert!(!out.load && !helper.pending_load, "load requested inside at frame {frame}");
    }
    let mut out = Out::default();
    helper.step_inputs(script(36, Some(4), None, true, false), &mut out);
    let result = *helper.results().next().expect("result after leaving the special");
    assert!(result.clean);
    assert_eq!(result.graded, 1);
    assert_eq!(helper.streak(), 1);
}

#[test]
fn an_old_drill_still_resolves_at_its_deadline() {
    let target = captured(Some(4), None, true, false, false);
    assert!(!target.has_aim());
    let deadline = target.get(0).unwrap().events().last().unwrap().offset as u32 + 1;
    assert!(deadline < 36);
    let mut helper = Helper::new();
    helper.set_target(Some(target));
    for frame in 0..=deadline {
        let mut out = Out::default();
        let snap = script(frame, Some(4), Some(0), true, false);
        assert!(snap.aim.inside);
        helper.step_inputs(snap, &mut out);
        assert_eq!(helper.results().next().is_some(), frame == deadline, "frame {frame}");
    }
    assert!(helper.results().next().unwrap().clean);
}

fn full_step_snap(frame: u32, second: bool) -> Snap {
    let part = if frame == 16 { 1 } else if frame == 30 && second { 2 } else { 0 };
    Snap {
        buttons: if frame < 10 { u32::from(frame.is_multiple_of(2)) } else { 2 },
        lstick_y: if frame >= 10 { 127 } else { 0 },
        facing: 1,
        aim: Seen {
            inside: frame <= 35,
            part,
            dir: if part == 1 { Some(4) } else if part == 2 { Some(0) } else { None },
            facing: 1,
            age: u8::from(part != 0),
        },
        ..Snap::empty(frame)
    }
}

fn full_step_reference() -> Combo {
    let mut capture = InputCapture::new();
    for frame in 0..=36 {
        capture.step(full_step_snap(frame, false));
    }
    let target = capture.combo;
    assert_eq!(target.len(), 1);
    let step = target.get(0).unwrap();
    assert_eq!(step.events().count(), STEPS);
    assert_eq!(step.events().last().unwrap().kind, Kind::Aim(1));
    target
}

#[test]
fn a_full_last_step_still_fails_an_extra_second_part() {
    let mut helper = Helper::new();
    helper.set_target(Some(full_step_reference()));
    for frame in 0..=35 {
        let mut out = Out::default();
        helper.step_inputs(full_step_snap(frame, true), &mut out);
        assert!(helper.results().next().is_none(), "resolved inside at frame {frame}");
        assert_eq!(helper.streak(), 0);
    }
    assert_eq!(helper.input.combo.len(), 2);
    assert!(!helper.input.combo.overflow);
    assert!(helper.input.combo.get(1).unwrap().events().any(|e| e.kind == Kind::Aim(2)));
    helper.step_inputs(full_step_snap(36, true), &mut Out::default());
    let result = helper.results().next().expect("result after the special");
    assert!(!result.clean, "the second direction must fail a full step");
    assert_eq!(result.fault, Some((0, Reason::ExtraAim(2))));
    assert_eq!(helper.streak(), 0);
}

#[test]
fn a_full_last_step_without_a_second_part_is_clean() {
    let mut helper = Helper::new();
    helper.set_target(Some(full_step_reference()));
    for frame in 0..=35 {
        helper.step_inputs(full_step_snap(frame, false), &mut Out::default());
        assert!(helper.results().next().is_none(), "resolved inside at frame {frame}");
    }
    helper.step_inputs(full_step_snap(36, false), &mut Out::default());
    let result = helper.results().next().expect("result after the special");
    assert!(result.clean);
    assert_eq!(result.graded, 1);
    assert_eq!(helper.streak(), 1);
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
    let jab = Sample { attack: Some(Move::Jab), status: 0x27, facing: 1, pos: Some((0.0, 0.0)), ..Sample::default() };
    tracker.step(jab, hit_cpu, MappedInputs::NEUTRAL);
    helper.step(&tracker, Settings::factory());
    tracker.step(Sample { actionable: true, facing: 1, pos: Some((0.0, 0.0)), ..Sample::default() }, stunned_cpu, MappedInputs::NEUTRAL);
    helper.step(&tracker, Settings::factory());
    let up_b = MappedInputs { buttons: 2, lstick_y: 127, ..MappedInputs::NEUTRAL };
    tracker.step(strip_sample(Move::UpSpecial, None, 1), stunned_cpu, up_b);
    helper.step(&tracker, Settings::factory());
    let left = MappedInputs { lstick_x: -127, ..MappedInputs::NEUTRAL };
    tracker.step(Sample { stop: true, ..strip_sample(Move::Other, Some((0.0, 0.0)), 1) }, hit_cpu, left);
    helper.step(&tracker, Settings::factory());
    assert!(tracker.latest_link().is_some());
    assert_eq!(tracker.aim_pending(), Some(tracker.frame()));
    assert!(helper.capture.current.is_empty(), "the hit frame must wait for its heading");
    tracker.step(strip_sample(Move::Other, Some((-11.0, 0.0)), 1), hit_cpu, left);
    helper.step(&tracker, Settings::factory());
    let rec = helper.capture.current.get(0).expect("the pending link is captured");
    assert!(rec.events().any(|e| e.kind == Kind::Aim(1) && e.dir == Dir::Back));
}

fn travel_hit_out(helper: &mut Helper, pending: bool) -> (Tracker, Sample, MappedInputs, Out) {
    let mut tracker = Tracker::new();
    let hit_cpu = Sample { stop: true, hitstun: true, reaction: Some(20.0), ..Sample::default() };
    let stunned_cpu = Sample { hitstun: true, reaction: Some(19.0), ..Sample::default() };
    let jab = Sample { attack: Some(Move::Jab), status: 0x27, facing: 1, pos: Some((0.0, 0.0)), ..Sample::default() };
    tracker.step(jab, hit_cpu, MappedInputs::NEUTRAL);
    helper.step(&tracker, Settings::factory());
    tracker.step(Sample { actionable: true, facing: 1, pos: Some((0.0, 0.0)), ..Sample::default() }, stunned_cpu, MappedInputs::NEUTRAL);
    helper.step(&tracker, Settings::factory());
    let up_b = MappedInputs { buttons: 2, lstick_y: 127, ..MappedInputs::NEUTRAL };
    tracker.step(strip_sample(Move::UpSpecial, None, 1), stunned_cpu, up_b);
    helper.step(&tracker, Settings::factory());
    let left = MappedInputs { lstick_x: -127, ..MappedInputs::NEUTRAL };
    let pos = if pending { (0.0, 0.0) } else { (-11.0, 0.0) };
    tracker.step(Sample { stop: pending, ..strip_sample(Move::Other, Some(pos), 1) }, hit_cpu, left);
    let out = helper.step(&tracker, Settings::factory());
    (tracker, hit_cpu, left, out)
}

fn travel_hit(helper: &mut Helper) -> (Tracker, Sample, MappedInputs) {
    let (tracker, cpu, left, _) = travel_hit_out(helper, true);
    (tracker, cpu, left)
}

fn legacy_hit_reference() -> LinkRec {
    let mut helper = Helper::new();
    let (tracker, _, _) = travel_hit(&mut helper);
    LinkRec::of(&tracker.latest_link().unwrap(), &tracker, tracker.exchange().1, false)
}

fn aim_hit_reference(dir: Dir) -> LinkRec {
    let mut helper = Helper::new();
    let (tracker, _, _) = travel_hit(&mut helper);
    let mut reference = LinkRec::of(&tracker.latest_link().unwrap(), &tracker, tracker.exchange().1, true);
    *reference.events.iter_mut().find(|slot| slot.is_none()).unwrap() =
        Some(event(Kind::Aim(1), dir, 0));
    reference
}

#[test]
fn a_hit_keeps_a_direction_that_arrives_three_frames_later() {
    for (x, dir) in [(-11.0, Dir::Back), (11.0, Dir::Forward)] {
        for aim_target in [false, true] {
            let mut helper = Helper::new();
            if aim_target {
                helper.set_target(Some(Combo::from_links([aim_hit_reference(dir)]).unwrap()));
            }
            let (mut tracker, cpu, left) = travel_hit(&mut helper);
            let hit = tracker.frame();
            let stick = if x > 0.0 { MappedInputs { lstick_x: 127, ..left } } else { left };
            for elapsed in 1..=2 {
                tracker.step(Sample { stop: true, ..strip_sample(Move::Other, Some((0.0, 0.0)), 1) }, cpu, stick);
                helper.step(&tracker, Settings::factory());
                assert!(helper.capture.current.is_empty(), "link taken before heading at +{elapsed}");
            }
            tracker.step(strip_sample(Move::Other, Some((x, 0.0)), 1), cpu, stick);
            helper.step(&tracker, Settings::factory());
            assert_eq!(tracker.frame(), hit + 3);
            let rec = helper.capture.current.get(0).expect("link captured with heading");
            assert!(rec.events().any(|e| e.kind == Kind::Aim(1) && e.dir == dir));
            if aim_target {
                assert!(helper.results().next().expect("aim target graded").clean);
            }
        }
    }
}

#[test]
fn a_hit_keeps_a_travel_that_exits_without_moving() {
    let mut helper = Helper::new();
    let (mut tracker, cpu, left) = travel_hit(&mut helper);
    for _ in 0..2 {
        tracker.step(strip_sample(Move::Other, Some((0.0, 0.0)), 1), cpu, left);
        helper.step(&tracker, Settings::factory());
        assert!(helper.capture.current.is_empty(), "stationary travel is still pending");
    }
    tracker.step(strip_sample(Move::Other, Some((0.0, 0.0)), 1), cpu, left);
    helper.step(&tracker, Settings::factory());
    let rec = helper.capture.current.get(0).expect("link captured when the onset window ends");
    assert!(!rec.events().any(|e| matches!(e.kind, Kind::Aim(_))));
}

#[test]
fn a_legacy_combo_is_graded_on_the_hit_frame() {
    let reference = legacy_hit_reference();
    assert!(reference.events().any(|e| e.kind == Kind::Flick(Flick::Dash)));
    let target = Combo::from_links([reference]).unwrap();
    assert!(!target.has_aim());
    let mut baseline = Helper::new();
    baseline.set_target(Some(target));
    let (_, _, _, baseline_out) = travel_hit_out(&mut baseline, false);
    let (baseline_index, baseline_grade) = baseline_out.link.expect("baseline graded on hit");
    assert_eq!(baseline_index, 0);
    assert!(baseline_grade.clean(1));
    let mut helper = Helper::new();
    helper.set_target(Some(target));
    let (tracker, _, _, out) = travel_hit_out(&mut helper, true);
    assert_eq!(tracker.aim_pending(), Some(tracker.frame()));
    let (index, grade) = out.link.expect("legacy target graded on hit while travel is pending");
    assert_eq!(index, 0);
    assert_eq!(grade, baseline_grade);
    assert!(grade.clean(1));
}

#[test]
fn a_newer_link_does_not_wait_behind_a_held_one() {
    let mut helper = Helper::new();
    let (mut tracker, hit_cpu, left) = travel_hit(&mut helper);
    let first = tracker.latest_link().unwrap();
    let stunned_cpu = Sample { hitstun: true, reaction: Some(19.0), ..Sample::default() };
    tracker.step(Sample { stop: true, motion_frame: 1.0, ..strip_sample(Move::Other, Some((0.0, 0.0)), 1) }, stunned_cpu, left);
    helper.step(&tracker, Settings::factory());
    assert!(helper.capture.current.is_empty());
    tracker.step(strip_sample(Move::Other, Some((-11.0, 0.0)), 1), hit_cpu, left);
    let second = tracker.latest_link().unwrap();
    assert_ne!(first.hit, second.hit);
    helper.step(&tracker, Settings::factory());
    assert_eq!(helper.capture.current.len(), 2);
    let a = helper.capture.current.get(0).unwrap();
    let b = helper.capture.current.get(1).unwrap();
    assert_eq!((a.from, a.to), (first.from, first.to));
    assert_eq!((b.from, b.to), (second.from, second.to));
}

#[test]
fn a_held_link_is_recorded_before_an_exchange_changes() {
    let mut helper = Helper::new();
    helper.set_target(Some(Combo::from_links([aim_hit_reference(Dir::Back)]).unwrap()));
    let (mut tracker, hit_cpu, left) = travel_hit(&mut helper);
    let exchange = tracker.exchange().0;
    let free_cpu = Sample { actionable: true, ..Sample::default() };
    for frame in 0..=super::super::frame_strip::DROP_MAX + 1 {
        tracker.step(Sample { stop: true, motion_frame: frame as f32 + 1.0,
            ..strip_sample(Move::Other, Some((0.0, 0.0)), 1) }, free_cpu, left);
        let out = helper.step(&tracker, Settings::factory());
        assert!(helper.capture.current.is_empty());
        assert!(helper.results().next().is_none());
        assert!(out.resolved.is_none());
    }
    assert!(!tracker.exchange_open());
    tracker.step(strip_sample(Move::Other, Some((-11.0, 0.0)), 1), hit_cpu, left);
    assert_ne!(tracker.exchange().0, exchange);
    let out = helper.step(&tracker, Settings::factory());
    assert!(out.resolved.expect("held link graded before exchange reset").clean);
    assert!(helper.capture.current.is_empty());
    let saved = helper.capture.previous.get(0).expect("previous exchange retained");
    assert!(saved.events().any(|e| e.kind == Kind::Aim(1) && e.dir == Dir::Back));
}

#[test]
fn a_legacy_combo_grades_the_same_link_on_the_hit_frame() {
    let mut first_helper = Helper::new();
    let (mut first, cpu, left) = travel_hit(&mut first_helper);
    let hit = first.frame();
    first.step(strip_sample(Move::Other, Some((-11.0, 0.0)), 1), cpu, left);
    first_helper.step(&first, Settings::factory());
    let reference = LinkRec::of(&first.latest_link().unwrap(), &first, first.exchange().1, false);
    assert!(reference.events().any(|e| e.kind == Kind::Flick(Flick::Dash)));
    assert!(!reference.events().any(|e| matches!(e.kind, Kind::Aim(_))));
    let target = Combo::from_links([reference]).unwrap();
    assert!(!target.has_aim());

    let mut helper = Helper::new();
    helper.set_target(Some(target));
    let (mut tracker, cpu, left, out) = travel_hit_out(&mut helper, true);
    assert_eq!(tracker.frame(), hit);
    assert!(helper.capture.current.is_empty());
    assert_eq!(tracker.aim_pending(), Some(hit));
    let (index, grade) = out.link.expect("legacy link was graded on the hit");
    assert_eq!(index, 0);
    assert!(grade.clean(1), "{grade:?}");
    let summary = out.resolved.expect("legacy combo completed on the hit");
    assert!(summary.clean, "{summary:?}");
    assert_eq!(summary.outcome, Outcome::Completed);
    tracker.step(strip_sample(Move::Other, Some((-11.0, 0.0)), 1), cpu, left);
    let out = helper.step(&tracker, Settings::factory());
    assert_eq!(tracker.frame(), hit + 1);
    let actual = LinkRec::of(&tracker.latest_link().unwrap(), &tracker, tracker.exchange().1, false);
    assert_eq!(actual.events, reference.events);
    assert!(!actual.events().any(|e| matches!(e.kind, Kind::Aim(_))));
    assert!(actual.events().any(|e| e.kind == Kind::Flick(Flick::Dash)));
    assert!(out.link.is_none(), "the link was already graded");
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
    assert_eq!(crate::model::recording_menu::combo_link_lines(&drill, crate::model::move_names::Namer::OFF), vec!["S1 Step 1", "↑ B · release · 1st ←", "S2 Step 2", "2nd →"]);
    assert_eq!(crate::model::combo_plan::mark_text(Mark::WrongDir, "1st direction", 1).0, Text::new("1st direction wrong"));
    assert_eq!(crate::model::combo_plan::mark_text(Mark::WrongDir, "C-stick", 1).0, Text::new("C-stick wrong direction"));
}
}
