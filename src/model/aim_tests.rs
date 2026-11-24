lib_unit_tests! {
use super::*;

const UP: i32 = 0x1de;
const SIDE: i32 = 0x1dd;
const DOWN: i32 = 0x1df;
const NEUTRAL: i32 = 0x1dc;
const A: i32 = 0x1e1;
const B: i32 = 0x1e2;

struct Driver {
    rule: Rule,
    sticks: Vec<(u32, (i8, i8))>,
    seen: Vec<Seen>,
}

impl Driver {
    fn new() -> Self {
        Self { rule: Rule::new(), sticks: Vec::new(), seen: Vec::new() }
    }

    fn push(&mut self, status: i32, pos: (f32, f32), stick: (i8, i8), stop: bool, grounded: bool) -> Seen {
        let frame = self.sticks.len() as u32;
        self.sticks.push((frame, stick));
        let start = match status {
            UP => Some(Family::Up), SIDE => Some(Family::Side),
            DOWN => Some(Family::Down), NEUTRAL => Some(Family::Neutral), _ => None,
        };
        let now = Now { frame, status, start, own: matches!(status, A | B), pos: Some(pos),
            stop, facing: 1, grounded, press: 0 };
        let sticks = &self.sticks;
        let seen = self.rule.step(now, |from, heading, ground| {
            sticks.iter().rev().find_map(|&(frame, stick)| {
                (frame >= from && follows(stick, heading, ground)).then_some(stick)
            })
        });
        self.seen.push(seen);
        seen
    }

    fn reports(&self) -> Vec<(u8, u8)> {
        self.seen.iter().filter_map(|s| (s.part != 0).then_some((s.part, s.dir?))).collect()
    }
}

fn simple(start: i32, movement: (f32, f32), stick: (i8, i8), grounded: bool) -> Vec<(u8, u8)> {
    let mut driver = Driver::new();
    for _ in 0..10 { driver.push(start, (0.0, 0.0), stick, false, grounded); }
    for n in 1..=4 {
        driver.push(A, (movement.0 * n as f32, movement.1 * n as f32), stick, false, grounded);
    }
    driver.reports()
}

#[test]
fn the_outcome_rule_gives_f13s_pikachu_parts() {
    for cast in f13::CASTS {
        for keep_nominal in [false, true] {
            let mut rule = Rule::new();
            rule.keep_nominal = keep_nominal;
            let press = cast.rows.iter().position(|row| row.rel == 0).unwrap() as u32;
            let mut reports = Vec::new();
            for (index, row) in cast.rows.iter().enumerate() {
                let now = Now {
                    frame: index as u32, status: row.status,
                    start: (row.status == UP).then_some(Family::Up), own: row.status > 0x1e0,
                    pos: Some((row.x, row.y)), stop: false, facing: row.lr, grounded: !row.air, press,
                };
                let seen = rule.step(now, |from, heading, ground| {
                    cast.rows[from as usize..=index].iter().rev().find_map(|row| {
                        let stick = (row.lx, row.ly);
                        follows(stick, heading, ground).then_some(stick)
                    })
                });
                if let Some(dir) = seen.dir.filter(|_| seen.part != 0) {
                    reports.push((seen.part, dir));
                }
            }
            assert_eq!(reports.as_slice(), if keep_nominal { cast.all } else { cast.parts }, "{} keep_nominal={keep_nominal}", cast.name);
        }
    }
}

#[test]
fn an_aim_in_the_specials_own_direction_is_no_event() {
    for (family, movement, stick) in [
        (UP, (0.0, 3.0), (0, 100)), (DOWN, (0.0, -3.0), (0, -100)),
        (SIDE, (3.0, 0.0), (100, 0)), (SIDE, (-3.0, 0.0), (-100, 0)),
    ] {
        assert!(simple(family, movement, stick, false).is_empty());
    }
    assert_eq!(simple(NEUTRAL, (0.0, 3.0), (0, 100), false), [(1, 2)]);
    assert_eq!(simple(UP, (-3.0, 0.0), (-100, 0), false), [(1, 4)]);
}

#[test]
fn a_carried_or_drifting_status_is_no_onset() {
    let mut carried = Driver::new();
    for n in 0..10 { carried.push(UP, (-2.0 * n as f32, 0.0), (-100, 0), false, false); }
    for n in 10..18 { carried.push(A, (-2.0 * n as f32, 0.0), (-100, 0), false, false); }
    assert!(carried.reports().is_empty());
    assert!(simple(UP, (-0.15, 0.0), (-100, 0), false).is_empty());
    let mut drift = Driver::new();
    for _ in 0..10 { drift.push(UP, (0.0, 0.0), (-100, 0), false, false); }
    for n in 1..=8 { drift.push(A, (-0.15 * n as f32, 0.0), (-100, 0), false, false); }
    assert!(drift.reports().is_empty());
    assert_eq!(simple(UP, (-3.0, 0.0), (-100, 0), false), [(1, 4)]);
}

#[test]
fn an_onset_the_stick_never_pointed_at_is_no_aim() {
    for stick in [(0, 0), (100, 0), (-20, 0)] {
        assert!(simple(UP, (-5.0, 0.0), stick, false).is_empty());
    }
    assert_eq!(simple(UP, (-5.0, 0.0), (-30, 0), false), [(1, 4)]);
}

#[test]
fn a_ground_travel_follows_the_sticks_side() {
    assert_eq!(simple(UP, (-3.0, 0.0), (-90, -90), true), [(1, 4)]);
    assert!(simple(UP, (-3.0, 0.0), (-90, -90), false).is_empty());
    assert!(simple(UP, (-3.0, 0.0), (90, -90), true).is_empty());
}

#[test]
fn a_reentered_status_is_the_next_part_and_a_third_is_none() {
    for second_stick in [(100, 0), (0, 0)] {
        let mut driver = Driver::new();
        for _ in 0..10 { driver.push(UP, (0.0, 0.0), (-100, 0), false, false); }
        driver.push(A, (-3.0, 0.0), (-100, 0), false, false);
        for _ in 0..9 { driver.push(B, (-3.0, 0.0), second_stick, false, false); }
        driver.push(A, (0.0, 0.0), second_stick, false, false);
        for _ in 0..9 { driver.push(B, (0.0, 0.0), (-90, 90), false, false); }
        driver.push(A, (-3.0, 3.0), (-90, 90), false, false);
        let expected = if second_stick.0 == 0 { vec![(1, 4)] } else { vec![(1, 4), (2, 0)] };
        assert_eq!(driver.reports(), expected);
    }
}

#[test]
fn a_delayed_aim_uses_the_stick_since_the_press() {
    let mut driver = Driver::new();
    for frame in 0..42 {
        driver.push(UP, (0.0, 0.0), if frame < 40 { (-90, 90) } else { (90, 90) }, false, false);
    }
    driver.push(A, (-3.0, 3.0), (90, 90), false, false);
    assert_eq!(driver.reports(), [(1, 3)]);
}

#[test]
fn hitstop_frames_do_not_use_up_the_onset_window() {
    for stop in [false, true] {
        let mut driver = Driver::new();
        for _ in 0..10 { driver.push(UP, (0.0, 0.0), (-100, 0), false, false); }
        driver.push(A, (0.0, 0.0), (-100, 0), false, false);
        for n in 0..6 {
            driver.push(A, (0.0, 0.0), (-100, 0), stop, false);
            if !stop && n == 1 { assert_eq!(driver.rule.pending(), None); }
        }
        let seen = driver.push(A, (-3.0, 0.0), (-100, 0), false, false);
        if stop {
            assert_eq!((seen.part, seen.dir, seen.age), (1, Some(4), 7));
        } else {
            assert_eq!(seen.part, 0);
            assert_eq!(driver.rule.pending(), None);
        }
    }
}

#[test]
fn the_design_tolerances_are_the_stated_defaults() {
    assert_eq!((MOVED, W, DV, FOLLOW, DEAD), (1.0, 3, 1.0, 33.75, 24));
    assert_eq!([7, 8, 64, 400].map(crate::model::action::hold_band), [0, 1, 8, 15]);
}
}
