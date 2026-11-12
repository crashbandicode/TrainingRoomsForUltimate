//! SPEC-input-06 §3.2–3.4 and §5.2: what the strip's panes show, and that the renderer writes only what changed.

lib_unit_tests! {
use super::*;
use crate::model::frame_strip::{Move, Sample, Tracker};
use crate::model::input_delay::MappedInputs;

const NONE: MappedInputs = MappedInputs::NEUTRAL;
const A: MappedInputs = MappedInputs { buttons: 0x1, lstick_x: 0, lstick_y: 0, rstick_x: 0, rstick_y: 0 };
const UP_A: MappedInputs = MappedInputs { buttons: 0x1, lstick_x: 0, lstick_y: 100, rstick_x: 0, rstick_y: 0 };

fn idle() -> Sample {
    Sample { actionable: true, ..Sample::default() }
}

fn attack(mv: Move, status: i32, frame: u32) -> Sample {
    Sample { attack: Some(mv), status, motion_frame: frame as f32, ..Sample::default() }
}

/// The CPU hit on 7 with 4 frames of hitlag and 18 of hitstun (free on 29), as in the model's tests.
fn cpu(frame: u32) -> Sample {
    match frame {
        0..=6 => idle(),
        7..=10 => Sample { stop: true, hitstun: true, reaction: Some(18.0), ..Sample::default() },
        f if f < 29 => Sample { hitstun: true, reaction: Some(18.0 - (f - 11) as f32), ..Sample::default() },
        _ => idle(),
    }
}

/// Jab → up tilt, a true combo by 3 (the up tilt pressed one frame late), then the CPU in the second hit's hitlag.
fn true_combo() -> Tracker {
    let mut t = Tracker::new();
    for f in 1..=26u32 {
        let (p1, inputs) = match f {
            1 | 2 | 20 => (idle(), NONE),
            3..=6 => (attack(Move::Jab, 0x27, f - 3), if f == 3 { A } else { NONE }),
            7..=10 => (Sample { stop: true, active: true, ..attack(Move::Jab, 0x27, 4) }, NONE),
            11..=19 => (Sample { active: f == 11, ..attack(Move::Jab, 0x27, f - 6) }, NONE),
            21..=25 => (attack(Move::UpTilt, 0x2b, f - 21), if f == 21 { UP_A } else { NONE }),
            _ => (Sample { stop: true, active: true, ..attack(Move::UpTilt, 0x2b, 5) }, NONE),
        };
        let cpu = if f == 26 { Sample { stop: true, hitstun: true, reaction: Some(30.0), ..Sample::default() } } else { cpu(f) };
        t.step(p1, cpu, inputs);
    }
    t
}

fn draw_all(r: &mut Renderer, t: &Tracker, shown: bool) -> (usize, Vec<(usize, Look, Changed)>) {
    let mut seen = Vec::new();
    let n = r.draw(t, shown, |i, look, changed| seen.push((i, *look, changed)));
    (n, seen)
}

fn text_of(seen: &[(usize, Look, Changed)], i: usize) -> String {
    let look = seen.iter().find(|(at, ..)| *at == i).map(|(_, l, _)| *l).expect("written");
    String::from_utf16(look.text.units()).unwrap()
}

fn the_strip_stays_near_the_estimated_pane_count() {
    // The input log it replaced had 271. The stick row adds one pane per frame (60).
    assert_eq!(COUNT, 181);
}

fn the_first_draw_writes_every_pane_and_an_unchanged_tracker_writes_none() {
    let t = true_combo();
    let mut r = Renderer::new();
    let (first, _) = draw_all(&mut r, &t, true);
    assert_eq!(first, COUNT);
    let (second, _) = draw_all(&mut r, &t, true);
    assert_eq!(second, 0);
}

fn hidden_writes_only_the_container_once_and_showing_again_writes_only_it() {
    let t = true_combo();
    let mut r = Renderer::new();
    draw_all(&mut r, &t, true);
    let (n, seen) = draw_all(&mut r, &t, false);
    assert_eq!(n, 1);
    assert_eq!(seen[0].0, at::CONTAINER);
    assert!(!seen[0].1.visible);
    assert_eq!(draw_all(&mut r, &t, false).0, 0, "hidden stays quiet");
    let (n, seen) = draw_all(&mut r, &t, true);
    assert_eq!((n, seen[0].0), (1, at::CONTAINER), "nothing else changed while hidden");
}

fn a_new_frame_writes_part_of_the_strip_and_invalidate_writes_it_all() {
    let mut t = true_combo();
    let mut r = Renderer::new();
    draw_all(&mut r, &t, true);
    t.step(Sample { stop: true, active: true, ..attack(Move::UpTilt, 0x2b, 5) }, Sample { stop: true, hitstun: true, ..Sample::default() }, NONE);
    let (n, _) = draw_all(&mut r, &t, true);
    assert!(n > 0 && n < COUNT, "a scroll moves runs and chips, not the labels: {n}");
    r.invalidate();
    assert_eq!(draw_all(&mut r, &t, true).0, COUNT);
}

fn the_readouts_and_the_last_row_tell_the_link() {
    let t = true_combo();
    let mut r = Renderer::new();
    let (_, seen) = draw_all(&mut r, &t, true);
    assert_eq!(text_of(&seen, at::READ_VALUE + 1), "-3");
    assert_eq!(text_of(&seen, at::READ_SUB + 1), "TRUE combo");
    assert_eq!(text_of(&seen, at::READ_VALUE + 2), "+1");
    assert_eq!(text_of(&seen, at::READ_SUB + 2), "1f late, 3 spare");
    assert_eq!(text_of(&seen, at::READ_VALUE), "-", "no exchange has ended yet");
    assert_eq!(text_of(&seen, at::WINDOW_TEXT), "window 5f");
    assert_eq!(text_of(&seen, at::PILL_TEXT), "TRUE -3");
    assert_eq!(text_of(&seen, at::MOVE_TEXT), "Jab > Up tilt");
    assert_eq!(text_of(&seen, at::BRACKET_TEXT), "link -3");
    let pill = seen.iter().find(|(i, ..)| *i == at::PILL).unwrap().1;
    assert_eq!(pill.color, GOOD);
}

fn lanes_are_runs_ending_at_the_newest_frame() {
    let t = true_combo();
    let mut r = Renderer::new();
    let (_, seen) = draw_all(&mut r, &t, true);
    let newest = seen.iter().find(|(i, ..)| *i == at::RUN).unwrap().1;
    // P1's newest run is the up tilt's hitlag on the newest frame: one cell, at the right edge.
    assert_eq!(newest.color, Cell::Hitlag.color());
    assert_eq!(newest.w, CELL_W);
    assert_eq!(newest.x, FIRST_X + 59.0 * PITCH + CELL_W / 2.0);
    let cpu_stun = seen.iter().filter(|(i, ..)| (at::RUN + RUNS..at::RUN + 2 * RUNS).contains(i)).find(|(_, l, _)| l.color == Cell::Hitstun.color());
    assert!(cpu_stun.is_some(), "the CPU's hitstun run is drawn");
}

fn chips_sit_above_the_frame_they_landed_on() {
    let t = true_combo();
    let mut r = Renderer::new();
    let (_, seen) = draw_all(&mut r, &t, true);
    // The newest press (up + A on frame 21) is 5 frames before the newest (26): slot 54. It has the stick arrow.
    let chip = seen.iter().find(|(i, ..)| *i == at::CHIP_BOX).unwrap().1;
    assert_eq!(chip.x, FIRST_X + 54.0 * PITCH + CELL_W / 2.0);
    assert!(chip.w > CHIP);
    let shaft = seen.iter().find(|(i, ..)| *i == at::CHIP_SHAFT).unwrap().1;
    assert!(shaft.visible);
    assert_eq!(shaft.rot, 0.0, "up: the shaft stays vertical");
    // The jab's chip has no arrow.
    let shaft = seen.iter().find(|(i, ..)| *i == at::CHIP_SHAFT + 1).unwrap().1;
    assert!(!shaft.visible);
}

fn input_lines() {
    let base = true_combo().latest_link().unwrap();
    let mut l = base;
    l.input = -3;
    assert_eq!(String::from_utf16(input_line(&l).units()).unwrap(), "buffered 3f, 3 spare");
    l.input = 0;
    l.verdict = crate::model::frame_strip::Verdict::Drop(2);
    assert_eq!(String::from_utf16(input_line(&l).units()).unwrap(), "on time, 2f past");
    l.verdict = crate::model::frame_strip::Verdict::True(None);
    assert_eq!(String::from_utf16(input_line(&l).units()).unwrap(), "on time");
}

fn look_at(seen: &[(usize, Look, Changed)], i: usize) -> Look {
    seen.iter().find(|(at, ..)| *at == i).map(|(_, look, _)| *look).expect("pane written")
}

fn held(x: i8, y: i8) -> MappedInputs {
    MappedInputs { lstick_x: x, lstick_y: y, ..NONE }
}

fn the_stick_row_shows_the_held_direction_every_frame() {
    const DIRS: [(i8, i8, Option<u8>); 9] = [
        (100, 0, Some(0)),
        (100, 100, Some(1)),
        (0, 100, Some(2)),
        (-100, 100, Some(3)),
        (-100, 0, Some(4)),
        (-100, -100, Some(5)),
        (0, -100, Some(6)),
        (100, -100, Some(7)),
        (0, 0, None),
    ];
    let mut t = Tracker::new();
    for (x, y, _) in DIRS {
        t.step(idle(), idle(), held(x, y));
    }
    let columns: Vec<_> = t.columns().map(|c| c.stick).collect();
    assert_eq!(columns, DIRS.iter().map(|(_, _, dir)| *dir).collect::<Vec<_>>());
    let mut r = Renderer::new();
    let (_, seen) = draw_all(&mut r, &t, true);
    let first = WINDOW - t.len();
    for (i, dir) in columns.iter().enumerate() {
        let look = look_at(&seen, at::STICK + first + i);
        match dir {
            None => assert!(!look.visible, "a neutral frame is blank"),
            Some(dir) => {
                assert!(look.visible);
                assert_eq!(look.text, Text::new(">"));
                assert_eq!(look.rot, f32::from(*dir) * 45.0, "dir {dir} is {dir} steps of 45 degrees from right");
                assert_eq!(look.y, STICK_Y);
            }
        }
    }
}

fn the_stick_row_scrolls_and_freezes_with_the_strip() {
    use crate::model::frame_strip::FREEZE_AFTER;
    let mut t = Tracker::new();
    for frame in 1..=80u32 {
        let (x, y) = if frame % 2 == 0 { (100, 0) } else { (0, 100) };
        t.step(idle(), idle(), held(x, y));
    }
    assert!(!t.frozen(), "the freeze waits while the stick moves");
    assert_eq!(t.len(), WINDOW);
    let moving: Vec<_> = t.columns().map(|c| (c.frame, c.stick)).collect();
    assert_eq!(moving.first().copied(), Some((21, Some(2))));
    assert_eq!(moving.last().copied(), Some((80, Some(0))));
    for (frame, stick) in &moving {
        assert_eq!(*stick, if frame % 2 == 0 { Some(0) } else { Some(2) });
    }
    let mut r = Renderer::new();
    let (_, moving_draw) = draw_all(&mut r, &t, true);
    for (i, (_, stick)) in moving.iter().enumerate() {
        let look = look_at(&moving_draw, at::STICK + i);
        assert!(look.visible);
        assert_eq!(look.rot, if *stick == Some(0) { 0.0 } else { 90.0 });
    }
    for _ in 0..u32::from(FREEZE_AFTER) {
        t.step(idle(), idle(), held(100, 0));
    }
    assert!(t.frozen());
    let newest = t.newest();
    assert_eq!(newest, Some(80 + u32::from(FREEZE_AFTER) - 1));
    let held_row: Vec<_> = t.columns().map(|c| (c.frame, c.stick)).collect();
    let _ = draw_all(&mut r, &t, true);
    for _ in 0..5 {
        t.step(idle(), idle(), held(100, 0));
    }
    assert!(t.frozen());
    assert_eq!(t.newest(), newest, "a held direction does not scroll the row");
    assert_eq!(t.columns().map(|c| (c.frame, c.stick)).collect::<Vec<_>>(), held_row);
    let (n, _) = draw_all(&mut r, &t, true);
    assert_eq!(n, 0, "the frozen row is not rewritten");
    t.step(idle(), idle(), held(0, 100));
    assert!(!t.frozen());
    assert_eq!(t.columns().last().map(|c| c.stick), Some(Some(2)));
    let (_, moved) = draw_all(&mut r, &t, true);
    let look = look_at(&moved, at::STICK + WINDOW - 1);
    assert!(look.visible);
    assert_eq!(look.rot, 90.0);
}

fn a_c_stick_attack_takes_the_c_stick_arrow() {
    let attack = MappedInputs { buttons: 0x1 | 0x80, lstick_x: 0, lstick_y: 100, rstick_x: 100, rstick_y: 0 };
    let mut t = Tracker::new();
    t.step(idle(), idle(), attack);
    assert_eq!(t.presses().next().map(|p| p.stick), Some(Some(0)));
    let mut r = Renderer::new();
    let (_, seen) = draw_all(&mut r, &t, true);
    let shaft = look_at(&seen, at::CHIP_SHAFT);
    assert!(shaft.visible);
    assert_eq!(shaft.rot, -90.0, "the chip arrow is the C-stick's right, not the left stick's up");
    let row = look_at(&seen, at::STICK + WINDOW - 1);
    assert_eq!(row.rot, 90.0, "the row stays the left stick");
    let neutral_c = MappedInputs { buttons: 0x1 | 0x80, lstick_x: 0, lstick_y: 100, rstick_x: 0, rstick_y: 0 };
    assert_eq!(
        crate::model::frame_strip::press_of(neutral_c, 0, 1).map(|p| p.stick),
        Some(Some(2)),
        "a C-stick that does not bin keeps the left-stick arrow"
    );
}

/// Eden `rc5-opus-cstick1` (2026-11-08), P1's mapped record around a C-stick smash, frame by frame: buttons `0x80a1`
/// (attack, smash, C-stick, flick-jump) for the hold; the left stick bytes are the C-stick's `(100, 0)` on the first 4
/// frames; the right stick bytes stay 0. `left` is the physical left stick the record replaced on those 4 frames.
fn c_stick_smash(left: (i8, i8)) -> Vec<(MappedInputs, crate::model::input_delay::Sticks)> {
    use crate::model::input_delay::Sticks;
    let mut frames = vec![(MappedInputs { buttons: 0x8000, lstick_x: left.0, lstick_y: left.1, ..NONE }, Sticks { lx: left.0, ly: left.1, rx: 0, ry: 0 })];
    for f in 0..9 {
        let copy = if f < 4 { (100, 0) } else { left };
        let record = MappedInputs { buttons: 0x80a1, lstick_x: copy.0, lstick_y: copy.1, rstick_x: 0, rstick_y: 0 };
        frames.push((record, Sticks { lx: left.0, ly: left.1, rx: 120, ry: 0 }));
    }
    frames
}

fn a_c_stick_smash_with_the_left_stick_neutral_leaves_the_stick_row_blank() {
    let mut t = Tracker::new();
    for (record, sticks) in c_stick_smash((0, 0)) {
        t.step(idle(), idle(), sticks.onto(record));
    }
    let row: Vec<_> = t.columns().map(|c| c.stick).collect();
    assert!(row.iter().all(Option::is_none), "the stick row is the left stick, which never moved: {row:?}");
    assert_eq!(t.presses().next().map(|p| p.stick), Some(Some(0)), "the chip arrow is the C-stick's right");
}

fn a_left_stick_held_through_a_c_stick_attack_stays_in_the_stick_row() {
    let mut t = Tracker::new();
    for (record, sticks) in c_stick_smash((0, 72)) {
        t.step(idle(), idle(), sticks.onto(record));
    }
    let row: Vec<_> = t.columns().map(|c| c.stick).collect();
    assert!(row.iter().all(|dir| *dir == Some(2)), "up on every frame, also the 4 the record replaced: {row:?}");
    assert_eq!(t.presses().next().map(|p| p.stick), Some(Some(0)), "the chip arrow is still the C-stick's right");
}

fn a_left_stick_press_keeps_its_arrow() {
    let attack = MappedInputs { buttons: 0x1, lstick_x: 0, lstick_y: 100, rstick_x: 100, rstick_y: 0 };
    let mut t = Tracker::new();
    t.step(idle(), idle(), attack);
    assert_eq!(t.presses().next().map(|p| p.stick), Some(Some(2)));
    let mut r = Renderer::new();
    let (_, seen) = draw_all(&mut r, &t, true);
    let shaft = look_at(&seen, at::CHIP_SHAFT);
    assert!(shaft.visible);
    assert_eq!(shaft.rot, 0.0, "up: the shaft stays vertical even while the right stick is held");
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("the_strip_stays_near_the_estimated_pane_count", the_strip_stays_near_the_estimated_pane_count as fn()),
        ("the_first_draw_writes_every_pane_and_an_unchanged_tracker_writes_none", the_first_draw_writes_every_pane_and_an_unchanged_tracker_writes_none as fn()),
        ("hidden_writes_only_the_container_once_and_showing_again_writes_only_it", hidden_writes_only_the_container_once_and_showing_again_writes_only_it as fn()),
        ("a_new_frame_writes_part_of_the_strip_and_invalidate_writes_it_all", a_new_frame_writes_part_of_the_strip_and_invalidate_writes_it_all as fn()),
        ("the_readouts_and_the_last_row_tell_the_link", the_readouts_and_the_last_row_tell_the_link as fn()),
        ("lanes_are_runs_ending_at_the_newest_frame", lanes_are_runs_ending_at_the_newest_frame as fn()),
        ("chips_sit_above_the_frame_they_landed_on", chips_sit_above_the_frame_they_landed_on as fn()),
        ("input_lines", input_lines as fn()),
        ("the_stick_row_shows_the_held_direction_every_frame", the_stick_row_shows_the_held_direction_every_frame as fn()),
        ("the_stick_row_scrolls_and_freezes_with_the_strip", the_stick_row_scrolls_and_freezes_with_the_strip as fn()),
        ("a_c_stick_attack_takes_the_c_stick_arrow", a_c_stick_attack_takes_the_c_stick_arrow as fn()),
        ("a_c_stick_smash_with_the_left_stick_neutral_leaves_the_stick_row_blank", a_c_stick_smash_with_the_left_stick_neutral_leaves_the_stick_row_blank as fn()),
        ("a_left_stick_held_through_a_c_stick_attack_stays_in_the_stick_row", a_left_stick_held_through_a_c_stick_attack_stays_in_the_stick_row as fn()),
        ("a_left_stick_press_keeps_its_arrow", a_left_stick_press_keeps_its_arrow as fn()),
    ]);
}
}
