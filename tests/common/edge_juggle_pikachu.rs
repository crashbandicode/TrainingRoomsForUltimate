use super::{quick_attack, skull_bash_start};
use crate::model::edge_juggle::perfect::{Point, Segment};

// Stock common ring from the Battlefield and Waiting Room LVD, in a near-ledge frame.
const RING: [Segment; 8] = [
    [[0.0, 0.0], [0.0, -8.319]],
    [[0.0, -8.319], [15.591, -25.111]],
    [[15.591, -25.111], [38.590, -40.099]],
    [[38.590, -40.099], [121.402, -40.118]],
    [[121.402, -40.118], [144.373, -25.103]],
    [[144.373, -25.103], [159.975, -8.319]],
    [[159.975, -8.319], [159.975, 0.0]],
    [[159.975, 0.0], [0.0, 0.0]],
];
const TARGET: Point = [161.675, -9.9];
const APEX: Point = [136.825, -76.025];
const BOTTOM: f32 = -140.0;

fn degrees([x, y]: Point) -> f32 {
    y.atan2(x).to_degrees()
}

#[test]
fn skull_bash_apex_aims_two_distinct_zips_around_the_far_lip() {
    for sign in [1.0, -1.0] {
        let mirror = |[x, y]: Point| [sign * x, y];
        let stage = RING.map(|line| line.map(mirror));
        let route = quick_attack(&stage, mirror(APEX), mirror(TARGET), BOTTOM, 10.0)
            .expect("the baked Skull Bash approach leaves both zips");
        let first = degrees(mirror(route.first));
        let second = degrees(mirror(route.second.expect("one zip falls short")));
        assert!((first - 50.0).abs() < 0.01);
        assert!((second - 114.0).abs() < 1.0);
        assert!(second - first > 39.0);

        let mut wall = stage.to_vec();
        // Any approach from the apex through this wall intersects the live collision lines.
        wall.push([mirror([158.0, -135.0]), mirror([158.0, 30.0])]);
        assert!(quick_attack(&wall, mirror(APEX), mirror(TARGET), BOTTOM, 10.0).is_none());
    }
}

#[test]
fn quick_attack_declines_an_invalid_or_unreachable_apex() {
    assert!(quick_attack(&RING, [f32::NAN, -50.0], TARGET, BOTTOM, 10.0).is_none());
    assert!(quick_attack(&RING, [12.0, -76.0], TARGET, BOTTOM, 10.0).is_none());
    assert!(quick_attack(&RING, [158.0, -137.0], TARGET, BOTTOM, 10.0).is_none());
}


#[test]
fn skull_bash_starts_only_inside_its_baked_region_and_live_stage_shape() {
    let ledges = [[0.0, 0.0], [159.975, 0.0]];
    let release = [-20.0, -28.0];
    for stage in [0, 3, 311] {
        let path = skull_bash_start((stage, 0), &RING, ledges, release, BOTTOM, 8.5).unwrap();
        assert_eq!(path.direction, 1.0);
        assert!((path.launch_y - (-50.298)).abs() < 0.002);
        assert_eq!(path.near, [-1.7, -9.9]);
        assert_eq!(path.far, TARGET);
        assert_eq!(path.height, 10.0);
    }
    let mirror = |[x, y]: Point| [159.975 - x, y];
    let right = skull_bash_start((0, 0), &RING, ledges, mirror(release), BOTTOM, 10.0).unwrap();
    assert_eq!(right.direction, -1.0);
    assert!((right.far[0] - mirror(TARGET)[0]).abs() < 0.001);

    for stage in [(89, 0), (33, 0), (0, 1), (3, 2)] {
        assert!(skull_bash_start(stage, &RING, ledges, release, BOTTOM, 10.0).is_none());
    }
    assert!(skull_bash_start((3, 0), &RING, ledges, [-39.0, -55.0], BOTTOM, 10.0).is_some());
    for start in [
        [-60.0, -28.0],
        [-20.0, -69.0],
        [-21.0, -66.0],
        [-6.0, -28.0],
        [f32::NAN, -28.0],
    ] {
        assert!(skull_bash_start((3, 0), &RING, ledges, start, BOTTOM, 10.0).is_none());
    }
    assert!(skull_bash_start((3, 0), &RING, ledges, release, -125.0, 10.0).is_none());
    assert!(skull_bash_start((3, 0), &RING, ledges, release, BOTTOM, 15.0).is_none());
    let shape_changed = RING.map(|line| line.map(|[x, y]| [x, y * 0.9]));
    assert!(skull_bash_start((3, 0), &shape_changed, ledges, release, BOTTOM, 10.0).is_none());
}


#[test]
fn skull_bash_flies_the_native_side_jump_and_two_aim_sequence() {
    use super::SkullBashFlight;
    use crate::model::edge_juggle::perfect::{
        niche::{self, Motion, Move, Sample, Sequence},
        Outline, GUARD, JUMP, SPECIAL,
    };
    let start = [-20.0, -28.0];
    let path = skull_bash_start((3, 0), &RING, [[0.0, 0.0], [159.975, 0.0]], start, BOTTOM, 8.5)
        .unwrap();
    let mut outline = Outline::new();
    for line in RING {
        outline.add(line);
    }
    let mut flight = SkullBashFlight::new(path, outline, BOTTOM);
    let mut sequence = Sequence::new(niche::measured_row(3, niche::CANDIDATE_ROWS).unwrap());
    let mut input = |frame, motion, can_start, position, vy| {
        flight.step(&mut sequence, Sample {
            frame,
            motion,
            can_start,
            valid: true,
            at_exit: false,
            can_enter: false,
            low: false,
            hold_over: false,
            direction: 0.0,
            aim: [0.0; 2],
        }, position, vy, u8::from(frame < 98)).unwrap()
    };
    let descend = input(0, Motion::Free, true, start, -1.55);
    assert_eq!(descend.stick, Some([0.0, -1.0]));
    assert_eq!(descend.press, 0);
    assert_ne!(descend.release & GUARD, 0);
    let before_clearance = input(13, Motion::Free, true, [-20.0, -49.95], -1.55);
    assert_eq!(before_clearance.press, 0);
    let press = input(14, Motion::Free, true, [-20.0, -51.5], -1.55);
    assert_eq!((press.stick, press.press), (Some([1.0, 0.0]), SPECIAL));
    let startup = input(15, Motion::Starting, false, [-20.0, -51.5], -0.1);
    assert_eq!(startup.press, 0, "uncharged Bash releases the special button");
    input(50, Motion::Travel, false, [50.0, -53.32], 0.0);
    input(90, Motion::Finish, false, [100.0, -94.0], -1.55);
    let bash_end = [111.2, -111.52];
    input(96, Motion::Free, false, bash_end, -1.55);
    assert_eq!(input(97, Motion::Free, true, bash_end, -1.55).press, JUMP);
    assert_eq!(input(98, Motion::Travel, false, [112.0, -109.0], 2.55).press, 0);
    assert_eq!(input(123, Motion::Travel, false, [136.4, -76.2], 0.1).press, 0);
    assert_eq!(input(124, Motion::Travel, false, APEX, -0.01).press, 0);
    let up = input(125, Motion::Free, true, APEX, -0.1);
    assert_eq!((up.stick, up.press), (Some([0.0, 1.0]), SPECIAL));

    let first = input(126, Motion::Starting, false, APEX, 0.0);
    assert!((degrees(first.stick.unwrap()) - 50.0).abs() < 0.01);
    let second = input(146, Motion::Finish, false, [172.0, -34.0], 0.0);
    assert!((degrees(second.stick.unwrap()) - 114.0).abs() < 1.0);
    assert_eq!(second.press, 0, "the native END reads the second direction");
    assert_eq!(sequence.action(), Some(Move::QuickAttack));
}

#[test]
fn skull_bash_uses_the_rebaked_199_cell_region() {
    let count: u32 = super::SKULL_BASH_RING.iter().map(|row| row.count_ones()).sum();
    assert_eq!(count, 199);
    let ledges = [[0.0, 0.0], [159.975, 0.0]];
    for (release, offered) in [([-13.0, -66.0], true), ([-17.0, -66.0], false),
                              ([-29.0, -62.0], true), ([-33.0, -62.0], false)] {
        assert_eq!(skull_bash_start((3, 0), &RING, ledges, release, BOTTOM, 8.5).is_some(),
                   offered, "{release:?}");
    }
}

#[test]
fn second_zip_keeps_four_degrees_beyond_the_measured_turn_floor() {
    let from = [210.0, -80.0];
    for (angle, offered) in [(40.0_f32, false), (43.0, false), (44.1, true)] {
        let (sin, cos) = angle.to_radians().sin_cos();
        let target = [from[0] + 30.0 * cos, from[1] + 30.0 * sin];
        assert_eq!(super::second_zip(&RING, from, target, [1.0, 0.0], BOTTOM, 10.0).is_some(),
                   offered, "{angle} degrees");
    }
}

#[test]
fn skull_bash_requires_the_jump_at_launch_and_at_first_fall() {
    use super::SkullBashFlight;
    use crate::model::edge_juggle::perfect::{
        niche::{self, Motion, Sample, Sequence}, Outline, JUMP, SPECIAL,
    };
    let start = [-20.0, -55.0];
    let path = skull_bash_start((3, 0), &RING, [[0.0, 0.0], [159.975, 0.0]], start, BOTTOM, 10.0)
        .unwrap();
    let mut outline = Outline::new();
    for line in RING {
        outline.add(line);
    }
    let row = niche::measured_row(3, niche::CANDIDATE_ROWS).unwrap();
    let sample = |frame, motion, can_start| Sample {
        frame, motion, can_start, valid: true, at_exit: false, can_enter: false,
        low: false, hold_over: false, direction: 0.0, aim: [0.0; 2],
    };
    let start_with = |jumps| {
        SkullBashFlight::new(path, outline, BOTTOM).step(
            &mut Sequence::new(row), sample(0, Motion::Free, true), start, -0.095, jumps,
        )
    };
    for (jumps, expected) in [(0, None), (1, Some(SPECIAL))] {
        assert_eq!(start_with(jumps).map(|pad| pad.press), expected, "before Bash, {jumps} jumps");
    }
    for (jumps, expected) in [(0, None), (1, Some(JUMP))] {
        let mut flight = SkullBashFlight::new(path, outline, BOTTOM);
        let mut sequence = Sequence::new(row);
        let mut step = |f, motion, can_start, pos, jumps| {
            flight.step(&mut sequence, sample(f, motion, can_start), pos, -1.55, jumps)
        };
        step(0, Motion::Free, true, start, 1).unwrap();
        step(1, Motion::Starting, false, start, 1).unwrap();
        step(16, Motion::Travel, false, [-15.0, -58.0], 1).unwrap();
        step(54, Motion::Finish, false, [58.0, -59.0], 1).unwrap();
        step(98, Motion::Free, false, [114.0, -118.5], jumps).unwrap();
        assert_eq!(
            step(99, Motion::Free, true, [114.7, -120.0], jumps).map(|pad| pad.press),
            expected, "at first jump opportunity, {jumps} jumps",
        );
    }
}

#[test]
fn fake_out_waits_for_a_rising_zip_step_inside_the_near_catch_band() {
    use super::fake_out_zip;
    for sign in [1.0, -1.0] {
        let mirror = |[x, y]: Point| [x * sign, y];
        let outline = RING.map(|line| line.map(mirror));
        let aim = |from| fake_out_zip(&outline, mirror(from), [0.0, 0.0], sign, BOTTOM, 10.0);
        assert_eq!(aim([-7.0, -18.0]), None, "wait for the natural apex catch");
        assert_eq!(aim([-7.0, -24.3]), Some([0.0, 1.0]));
        assert_eq!(aim([-7.0, -35.0]), Some([0.0, 1.0]));
        assert_eq!(aim([-130.0, -35.0]), None);
        assert_eq!(aim([-7.0, f32::NAN]), None);
        assert_eq!(aim([f32::NAN, -35.0]), None);
    }
}

#[test]
fn fake_out_brakes_to_hold_seven_units_outside_the_near_ledge() {
    use super::FakeOutDrift;
    // The Surface trace puts the jump's first rising horizontal step at 0.90,
    // capped at 0.957 afterwards, with 0.10 per opposing stick frame.
    for outside in [12.0_f32, 20.0, 28.0, 36.0, 38.0, 40.0] {
        let mut x = -outside;
        let mut vx = if outside > 12.0 { 0.9 } else { 0.0 };
        let mut drift = FakeOutDrift::default();
        let mut braked = false;
        for _ in 0..60 {
            x += vx;
            let stick = drift.stick(-7.0 - x, vx);
            braked |= stick < 0.0 && vx > 0.0;
            vx = (vx + 0.1 * stick).clamp(-0.957, 0.957);
            if vx.abs() < 0.05 {
                vx = 0.0;
            }
        }
        assert!(braked, "the route brakes from {outside} out");
        assert!((-8.5..=-5.5).contains(&x), "{outside} out stops at {x}");
        assert_eq!(vx, 0.0, "{outside} out stabilizes at the ledge");
    }
}

#[test]
fn fake_out_spends_only_the_jump_near_the_lip_and_a_fresh_special_if_needed() {
    use super::{FakeOutFlight, FakeOutSample};
    use crate::model::edge_juggle::perfect::{niche::Motion, Outline, JUMP, SPECIAL};

    let mut outline = Outline::new();
    for line in RING {
        outline.add(line);
    }
    let start = [-20.0, -28.0];
    let path = skull_bash_start(
        (3, 0), &RING, [[0.0, 0.0], [159.975, 0.0]], start, BOTTOM, 8.5,
    ).unwrap();
    let mut flight = FakeOutFlight::new(path, outline, BOTTOM);
    let sample = |frame, motion, position, velocity, jumps, can_start| FakeOutSample {
        frame, motion, position, velocity, jumps, can_start,
    };
    let frames = [
        (sample(0, Motion::Free, start, [0.0, -1.55], 1, true), [0.0, -1.0], 0),
        (sample(14, Motion::Free, [-20.0, -51.5], [0.0, -1.55], 1, true), [1.0, 0.0], JUMP),
        (sample(14, Motion::Free, [-20.0, -51.5], [0.0, -1.55], 1, true), [1.0, 0.0], JUMP),
        (sample(15, Motion::Travel, [-19.1, -49.0], [0.9, 2.55], 0, false), [1.0, 0.0], 0),
        (sample(25, Motion::Travel, [-10.9, -31.0], [0.957, 1.35], 0, false), [-1.0, 0.0], 0),
        (sample(35, Motion::Travel, [-7.0, -19.0], [0.0, 0.25], 0, false), [0.0, 0.0], 0),
        (sample(45, Motion::Free, [-7.0, -23.5], [0.0, -0.65], 0, true), [0.0, 0.0], 0),
        (sample(46, Motion::Free, [-7.0, -24.3], [0.0, -0.75], 0, true), [0.0, 1.0], SPECIAL),
        (sample(47, Motion::Starting, [-7.0, -25.1], [0.0, 0.0], 0, false), [0.0, 1.0], 0),
        (sample(66, Motion::Finish, [-7.0, -14.1], [0.0, 0.0], 0, false), [0.0, 0.0], 0),
    ];
    for (state, stick, press) in frames {
        let pad = flight.step(state).map(|pad| (pad.stick, pad.press));
        assert_eq!(pad, Some((Some(stick), press)), "native frame {}", state.frame);
    }
    assert!(flight.step(sample(67, Motion::Interrupted, [-7.0, -14.1], [0.0, 0.0], 0, false)).is_none());

    let mut no_jump = FakeOutFlight::new(path, outline, BOTTOM);
    assert!(no_jump.step(sample(0, Motion::Free, [-20.0, -55.0], [0.0, -1.0], 0, true)).is_none());
    let mut too_deep = FakeOutFlight::new(path, outline, BOTTOM);
    assert!(too_deep.step(sample(0, Motion::Free, [-20.0, -65.0], [0.0, -1.0], 1, true)).is_none());
}

#[test]
fn fake_out_roll_keeps_the_crossing_and_high_recovery_and_varies_the_near_return() {
    use super::fake_out_selected;
    use crate::model::edge_juggle::perfect::{recovery::Zone, Route};

    let rolls = |normal, zone, previous| {
        let mut draw = 0x390b_26d1_u32;
        (0..4096).filter(|_| {
            draw = draw.wrapping_mul(1664525).wrapping_add(1013904223);
            fake_out_selected(normal, zone, previous, draw)
        }).count()
    };
    for zone in [Zone::Mid, Zone::Ledge] {
        for route in [Route::Near, Route::LateLow] {
            assert!((350..=480).contains(&rolls(Some(route), zone, None)));
            assert!((733..=890).contains(&rolls(Some(route), zone, Some(Route::Opposite))));
        }
        for route in [None, Some(Route::High), Some(Route::Opposite)] {
            assert_eq!(rolls(route, zone, Some(Route::Opposite)), 0);
        }
    }
    for zone in [Zone::Far, Zone::Hang, Zone::OffHigh, Zone::OffLow] {
        assert_eq!(rolls(Some(Route::Near), zone, Some(Route::Opposite)), 0);
    }
}
