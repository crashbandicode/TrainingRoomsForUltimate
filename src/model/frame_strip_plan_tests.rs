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

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("hidden_writes_only_the_container_once_and_showing_again_writes_only_it", hidden_writes_only_the_container_once_and_showing_again_writes_only_it as fn()),
        ("input_lines", input_lines as fn()),
        ("the_stick_row_scrolls_and_freezes_with_the_strip", the_stick_row_scrolls_and_freezes_with_the_strip as fn()),
    ]);
}
}
