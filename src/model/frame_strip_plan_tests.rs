//! SPEC-input-06 §3.2–3.4 and §5.2: what the strip's panes show, and that the renderer writes only what changed.

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

/// The CPU hit on 7 with 4 frames of hitlag and 18 of hitstun (free on 30), as in the model's tests.
fn cpu(frame: u32) -> Sample {
    match frame {
        0..=6 => idle(),
        7..=10 => Sample { stop: true, hitstun: true, reaction: Some(18.0), ..Sample::default() },
        f if f < 30 => Sample { hitstun: true, reaction: Some(18.0 - (f - 11) as f32), ..Sample::default() },
        _ => idle(),
    }
}

/// Jab → up tilt, a true combo by 4 (the up tilt pressed one frame late), then the CPU in the second hit's hitlag.
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

#[test]
fn the_strip_stays_near_the_estimated_pane_count() {
    assert_eq!(COUNT, 121);
    assert!(COUNT < 271, "fewer panes than the input log it replaces");
}

#[test]
fn the_first_draw_writes_every_pane_and_an_unchanged_tracker_writes_none() {
    let t = true_combo();
    let mut r = Renderer::new();
    let (first, _) = draw_all(&mut r, &t, true);
    assert_eq!(first, COUNT);
    let (second, _) = draw_all(&mut r, &t, true);
    assert_eq!(second, 0);
}

#[test]
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

#[test]
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

#[test]
fn the_readouts_and_the_last_row_tell_the_link() {
    let t = true_combo();
    let mut r = Renderer::new();
    let (_, seen) = draw_all(&mut r, &t, true);
    assert_eq!(text_of(&seen, at::READ_VALUE + 1), "-4");
    assert_eq!(text_of(&seen, at::READ_SUB + 1), "TRUE combo");
    assert_eq!(text_of(&seen, at::READ_VALUE + 2), "+1");
    assert_eq!(text_of(&seen, at::READ_SUB + 2), "1f late, 4 spare");
    assert_eq!(text_of(&seen, at::READ_VALUE), "-", "no exchange has ended yet");
    assert_eq!(text_of(&seen, at::WINDOW_TEXT), "window 6f");
    assert_eq!(text_of(&seen, at::PILL_TEXT), "TRUE -4");
    assert_eq!(text_of(&seen, at::MOVE_TEXT), "Jab > Up tilt");
    assert_eq!(text_of(&seen, at::BRACKET_TEXT), "link -4");
    let pill = seen.iter().find(|(i, ..)| *i == at::PILL).unwrap().1;
    assert_eq!(pill.color, GOOD);
}

#[test]
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

#[test]
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

#[test]
fn input_lines() {
    let base = true_combo().latest_link().unwrap();
    let mut l = base;
    l.input = -3;
    assert_eq!(String::from_utf16(input_line(&l).units()).unwrap(), "buffered 3f, 4 spare");
    l.input = 0;
    l.verdict = crate::model::frame_strip::Verdict::Drop(2);
    assert_eq!(String::from_utf16(input_line(&l).units()).unwrap(), "on time, 2f past");
    l.verdict = crate::model::frame_strip::Verdict::True(None);
    assert_eq!(String::from_utf16(input_line(&l).units()).unwrap(), "on time");
}
