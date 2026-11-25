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
    facing: i8,
}

impl Driver {
    fn new() -> Self {
        Self { rule: Rule::new(), sticks: Vec::new(), seen: Vec::new(), facing: 1 }
    }

    fn push(&mut self, status: i32, pos: (f32, f32), stick: (i8, i8), stop: bool, grounded: bool) -> Seen {
        let frame = self.sticks.len() as u32;
        self.sticks.push((frame, stick));
        let start = match status {
            UP => Some(Family::Up), SIDE => Some(Family::Side),
            DOWN => Some(Family::Down), NEUTRAL => Some(Family::Neutral), _ => None,
        };
        let now = Now { frame, status, start, own: matches!(status, A | B), pos: Some(pos),
            stop, facing: self.facing, grounded, press: 0, stick };
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

/// Each pair uses measured approach positions one game frame apart, then a controlled moving special.
/// Opposite steering and a >2-unit path difference must remain distinguishable at the normal one-frame tolerance.
#[test]
fn entry_motion_one_frame_apart_compares_on_all_44_pikachu_fall_samples() {
    let approach = |rows: &[f13::Row], steer: bool| {
        let mut driver = Driver::new();
        for r in rows { driver.push(0x10, (r.x, r.y), (0, 0), false, false); }
        let r = rows.last().unwrap();
        for n in 1..=6 {
            driver.push(UP, (r.x + if steer { n as f32 * 0.5 } else { 0.0 }, r.y + n as f32),
                if steer { (-90, 90) } else { (0, 0) }, false, false);
        }
        driver.push(0x10, (r.x, r.y), (0, 0), false, false);
        driver.rule.ended().unwrap().1
    };
    let (mut checked, mut strict, mut matching) = (0, 0, 0);
    for cast in f13::CASTS {
        for w in cast.rows.windows(4).filter(|w| w[1..].iter().all(|r| r.air && r.status == 0x10)) {
            let (saved, actual) = (approach(&w[..3], true), approach(&w[1..], false));
            checked += 1;
            strict += usize::from(action::steer_off(&saved, &actual, false, 0).is_some());
            matching += usize::from(action::steer_off(&saved, &actual, false, 1).is_some());
        }
    }
    assert_eq!((checked, strict, matching), (44, 33, 44));
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
                    stick: (row.lx, row.ly),
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
    assert_eq!(crate::model::action::STEER_FLOOR, 2.0);
    assert_eq!((action::ENTRY_BASE, action::ENTRY_CHANGE_PER_FRAME), (16, 1));
}

/// The special's own travel is part 0: from the frame before its start status to its end, in whatever status it
/// moves, with the stick most held on the way and the press that began it.
#[test]
fn a_specials_own_travel_is_measured_from_its_start_to_its_end() {
    for facing in [1, -1] {
        let mut d = Driver::new();
        d.facing = facing;
        d.push(0, (0.0, 0.0), (0, 0), false, true);
        for n in 1..=3 {
            d.push(UP, (0.0, 2.0 * n as f32), (0, 100), false, false);
            assert_eq!(d.rule.ended(), None);
        }
        // Hitstop frames are not positions of the travel.
        d.push(UP, (999.0, 999.0), (-90, 90), true, false);
        for n in 4..=20 {
            d.push(UP, (-0.5 * (n - 3) as f32, 2.0 * n as f32), (-90, 90), false, false);
            assert_eq!(d.rule.ended(), None);
        }
        d.push(0, (-8.5, 38.0), (-90, 90), false, false);
        let (part, travel) = d.rule.ended().expect("the own travel ends with the special");
        assert_eq!((part, d.rule.own_press()), (0, 0));
        assert_eq!(travel.stick, Some((if facing == 1 { 1350 } else { 450 }, 100)), "up-left was held longest");
        assert_eq!(travel.path[7], (if facing == 1 { -9 } else { 9 }, 40));
        assert!(d.reports().is_empty(), "rule D: no aim event");
        d.push(0, (-8.5, 36.0), (-90, 90), false, false);
        assert_eq!(d.rule.ended(), None);
    }
}

#[test]
fn a_special_that_does_not_move_has_no_own_travel() {
    let mut d = Driver::new();
    d.push(0, (0.0, 0.0), (0, 0), false, true);
    for _ in 0..20 {
        d.push(NEUTRAL, (0.5, 0.0), (100, 0), false, true);
        assert_eq!(d.rule.ended(), None);
    }
    d.push(0, (0.5, 0.0), (100, 0), false, true);
    assert_eq!(d.rule.ended(), None, "half a unit is no travel");
}

/// An aimed special's own travel ends where its first aim part begins: nothing on the ground, and in the air only
/// the drift before the part. The aim parts are measured as before.
#[test]
fn the_own_travel_ends_where_the_first_aim_part_begins() {
    let mut d = Driver::new();
    d.push(0, (0.0, 0.0), (0, 0), false, true);
    for _ in 0..10 { d.push(UP, (0.0, 0.0), (-100, 0), false, true); }
    for n in 1..=5 {
        d.push(A, (-4.0 * n as f32, 0.0), (-100, 0), false, false);
        assert_eq!(d.rule.ended(), None, "a start that stands still keeps nothing");
    }
    d.push(0, (-20.0, 0.0), (-100, 0), false, false);
    assert_eq!(d.rule.ended().map(|(part, travel)| (part, travel.path[7])), Some((1, (-20, 0))));
    assert_eq!(d.reports(), vec![(1, 4)]);

    let mut d = Driver::new();
    d.push(0, (0.0, 10.0), (0, 0), false, false);
    for n in 1..=10 { d.push(UP, (0.0, 10.0 - 0.3 * n as f32), (-100, 0), false, false); }
    d.push(A, (-4.0, 7.0), (-100, 0), false, false);
    let (part, drift) = d.rule.ended().expect("the drift before the part is the own travel");
    assert_eq!((part, drift.path[7], drift.stick), (0, (0, -3), Some((1800, 83))));
    for n in 2..=5 { d.push(A, (-4.0 * n as f32, 7.0), (-100, 0), false, false); }
    d.push(0, (-20.0, 7.0), (-100, 0), false, false);
    assert_eq!(d.rule.ended().map(|(part, travel)| (part, travel.path[7])), Some((1, (-20, 0))));
    assert_eq!(d.reports(), vec![(1, 4)]);
}

/// The stick kept with an own travel is the one held for the most frames; neutral when no direction was.
#[test]
fn the_stick_most_held_is_kept_with_the_own_travel() {
    let run = |sticks: &[((i8, i8), usize)]| {
        let mut d = Driver::new();
        d.push(0, (0.0, 0.0), (0, 0), false, true);
        let mut n = 0;
        for &(stick, frames) in sticks {
            for _ in 0..frames {
                n += 1;
                d.push(UP, (0.0, 2.0 * n as f32), stick, false, false);
            }
        }
        d.push(0, (0.0, 2.0 * n as f32), (0, 0), false, false);
        d.rule.ended().unwrap().1.stick
    };
    assert_eq!(run(&[((0, 100), 3), ((-90, 90), 10), ((90, 90), 4)]), Some((1350, 100)));
    assert_eq!(run(&[((0, 100), 3), ((0, 0), 10)]), Some((0, 0)), "no direction held");
    assert_eq!(run(&[((-60, 60), 5), ((-90, 90), 5), ((10, 10), 9)]), Some((1350, 100)), "the newest sample of that direction");
    assert_eq!(run(&[((0, 100), 5), ((0, 0), 5)]), Some((900, 83)), "a tie goes to the direction");
    assert_eq!(run(&[((100, 0), 255), ((0, 100), 256)]), Some((900, 83)), "up was held one frame longer");
}

#[test]
fn a_travel_part_records_its_stick_heading_distance_and_path() {
    for facing in [1, -1] {
        let mut d = Driver::new();
        d.facing = facing;
        for _ in 0..10 { d.push(UP, (0.0, 0.0), (-90, 90), false, false); }
        for n in 1..=10 {
            d.push(A, (-4.0 * n as f32, 4.0 * n as f32), (-90, 90), false, false);
            assert_eq!(d.rule.ended(), None);
            if n == 5 {
                d.push(A, (999.0, 999.0), (-90, 90), true, false);
                assert_eq!(d.rule.ended(), None);
            }
        }
        d.push(B, (-40.0, 40.0), (-90, 90), false, false);
        let (part, travel) = d.rule.ended().unwrap();
        assert_eq!(part, 1);
        assert_eq!(travel.stick, Some((if facing == 1 { 1350 } else { 450 }, 100)));
        assert_eq!((travel.heading, travel.distance), (if facing == 1 { 1350 } else { 450 }, 566));
        assert_eq!(travel.path[7], (if facing == 1 { -40 } else { 40 }, 40));
        assert_eq!(travel.path[0], (if facing == 1 { -5 } else { 5 }, 5));
        d.push(B, (-40.0, 40.0), (-90, 90), false, false);
        assert_eq!(d.rule.ended(), None);
    }
}

#[test]
fn the_deciding_stick_is_the_last_one_that_pointed_along_the_heading() {
    for (last, expected) in [((-50, 50), (1350, 59)), ((90, 90), (1350, 100))] {
        let mut d = Driver::new();
        for _ in 0..40 { d.push(UP, (0.0, 0.0), (-90, 90), false, false); }
        for _ in 0..2 { d.push(UP, (0.0, 0.0), last, false, false); }
        for n in 1..=10 { d.push(A, (-4.0 * n as f32, 4.0 * n as f32), (90, 90), false, false); }
        d.push(B, (-40.0, 40.0), (90, 90), false, false);
        assert_eq!(d.rule.ended().unwrap().1.stick, Some(expected));
    }
}

#[test]
fn a_path_is_eight_points_by_arc_length_facing_relative() {
    let mut d = Driver::new();
    for _ in 0..10 { d.push(UP, (0.0, 0.0), (100, 0), false, false); }
    for n in 1..=8 { d.push(A, (2.0 * n as f32, 0.0), (100, 0), false, false); }
    for n in 1..=8 { d.push(A, (16.0, 2.0 * n as f32), (100, 0), false, false); }
    d.push(B, (16.0, 16.0), (100, 0), false, false);
    assert_eq!(d.rule.ended().unwrap().1.path,
        [(4, 0), (8, 0), (12, 0), (16, 0), (16, 4), (16, 8), (16, 12), (16, 16)]);

    let mut d = Driver::new();
    for _ in 0..10 { d.push(UP, (0.0, 0.0), (-100, 0), false, false); }
    for n in 1..=60 { d.push(A, (-1.5 * n as f32, 0.0), (-100, 0), false, false); }
    d.push(B, (-90.0, 0.0), (-100, 0), false, false);
    let path = d.rule.ended().unwrap().1.path;
    assert_eq!(path[7], (-90, 0));
    for (i, &(x, y)) in path.iter().enumerate() {
        assert!((f32::from(x) + 11.25 * (i + 1) as f32).abs() <= 1.0);
        assert_eq!(y, 0);
    }

    let mut d = Driver::new();
    for _ in 0..10 { d.push(UP, (0.0, 0.0), (-100, 0), false, false); }
    for n in 1..=75 { d.push(A, (-2.0 * n as f32, 0.0), (-100, 0), false, false); }
    d.push(B, (-150.0, 0.0), (-100, 0), false, false);
    assert_eq!(d.rule.ended().unwrap().1.path[7], (-127, 0));
}
}
