use super::*;
use crate::model::edge_juggle::perfect::{Segment, GUARD, JUMP, SPECIAL};

const NEAR: Point = [-79.9856, 0.0];
const LEDGES: [Point; 2] = [NEAR, [79.989, 0.0]];
const OUTLINE: [Segment; 5] = [
    [[-79.986, -8.319], [-64.395, -25.111]],
    [[-64.395, -25.111], [-41.396, -40.099]],
    [[-41.396, -40.099], [41.416, -40.118]],
    [[41.416, -40.118], [64.387, -25.103]],
    [[64.387, -25.103], [79.989, -8.319]],
];

fn fixture(stage: (i32, u32), out: f32, above: f32) -> (RouteFacts<'static>, LucasBeat) {
    let position = [NEAR[0] - out, above];
    let facts = RouteFacts {
        fighter: KIND,
        stage,
        position,
        player: [-55.0, 0.01],
        player_reach: 10.0,
        ledges: LEDGES,
        blast: [-230.0, 230.0, 180.0, -140.0],
        outline: &OUTLINE,
        body_clearance: 9.0,
        ledge_distance: 9.0,
        fall_budget: 1.37,
        reach: [0.0; 2],
    };
    let beat = LucasBeat {
        frame: 0,
        status: 0x0e,
        position,
        velocity: [0.0; 2],
        facing: 1.0,
        jumps: 1,
        can_dodge: true,
        can_jump: true,
        can_thunder: true,
    };
    (facts, beat)
}

#[test]
fn lucas_at_rest_admission_requires_the_flown_box_geometry_and_all_three_resources() {
    for stage in [(3, 0), (0, 0), (311, 0)] {
        for (out, above) in [(10.0, 2.0), (14.0, 2.0), (18.0, 2.0), (22.0, 2.0)] {
            let (mut facts, beat) = fixture(stage, out, above);
            assert!(UnderStage::start(&facts, beat).is_some());
            for rejected in [
                LucasBeat { can_dodge: false, ..beat },
                LucasBeat { can_jump: false, ..beat },
                LucasBeat { can_thunder: false, ..beat },
                LucasBeat { jumps: 0, ..beat },
                LucasBeat { velocity: [0.0, -1.37], ..beat },
            ] {
                assert!(UnderStage::start(&facts, rejected).is_none());
            }
            assert!(UnderStage::start(&facts, beat).unwrap().guarder_clear(facts.player));
            assert!(!UnderStage::start(&facts, beat).unwrap().guarder_clear([-78.0, 0.0]));
            facts.blast[3] = -110.0;
            assert!(UnderStage::start(&facts, beat).is_none());
        }
    }
    for (stage, out, above) in [
        ((3, 0), 12.0, -4.0), ((3, 0), 12.0, 38.1), ((3, 0), 6.0, 2.0),
        ((3, 0), 26.0, 2.0), ((11, 0), 10.0, 2.0), ((0, 1), 10.0, 2.0),
    ] {
        let (facts, beat) = fixture(stage, out, above);
        assert!(UnderStage::start(&facts, beat).is_none());
    }
}

#[test]
fn lucas_tracks_the_first_native_rows_and_replays_the_recorded_pad_edges() {
    let (facts, entry) = fixture((3, 0), 10.0, 1.91);
    let entry = LucasBeat { velocity: [0.0, -0.09], ..entry };
    let mut flight = UnderStage::start(&facts, entry).expect("fg2-g1-yoga-22, at rest");
    // Exact nominal L-X trace at (10 out, +2), first attempt.
    for (frame, status, x, y, vx, vy, facing, jumps, press, sx, sy) in [
        (0, 0x0e, -89.986, 1.91, 0.0, -0.09, 1.0, 1, 0, 0, 0),
        (4, 0x0e, -89.986, 1.01, 0.0, -0.36, 1.0, 1, GUARD, 85, -85),
        (5, 0x22, -89.91579, 0.56, 0.070206, -0.45, 1.0, 1, GUARD, 85, -85),
        (6, 0x22, -92.39067, 3.034873, -2.474876, 2.474874, 1.0, 1, 0, 120, 0),
        (87, 0x22, -11.205375, -105.36808, 1.155, -1.37, 1.0, 1, 0, 120, 0),
        (88, 0x22, -10.050375, -106.73808, 1.155, -1.37, 1.0, 1, JUMP, 120, 0),
        (89, 0x0c, -8.895375, -108.10809, 1.155, -1.37, 1.0, 0, JUMP, 120, 0),
        (90, 0x0c, -7.993622, -105.82233, 0.901753, 2.285751, 1.0, 0, 0, 120, 0),
        (146, 0x0f, 56.444622, -68.11978, 1.15477, -0.67, 1.0, 0, 0, 120, 0),
        (162, 0x0f, 74.924606, -87.65976, 1.155, -1.37, 1.0, 0, SPECIAL, 0, 120),
        (163, 0x1de, 76.065605, -89.02976, 1.141, -1.37, 1.0, 0, SPECIAL, 0, 120),
        (164, 0x1de, 76.508003, -89.02976, 0.442398, 0.0, 1.0, 0, 0, 0, 0),
        (181, 0x1de, 81.88678, -89.02976, 0.204399, 0.0, 1.0, 0, 0, 0, 0),
        (182, 0x1e4, 82.07718, -89.02976, 0.190399, 0.0, 1.0, 0, 0, 111, 45),
        (195, 0x1e4, 83.27837, -89.02976, 0.0084, 0.0, 1.0, 0, 0, 17, -119),
        (196, 0x1e4, 83.27837, -89.02976, 0.0, 0.0, 1.0, 0, 0, 0, -120),
        (232, 0x1e4, 83.27837, -96.93475, 0.0, -0.510002, 1.0, 0, 0, 0, -120),
        (233, 0x1e4, 83.27837, -97.46175, 0.0, -0.527, 1.0, 0, 0, -111, -45),
        (254, 0x1e4, 83.27837, -112.45576, 0.0, -0.884, 1.0, 0, 0, 99, 67),
        (255, 0x1e4, 83.27837, -113.35676, 0.0, -0.901, 1.0, 0, 0, 0, 120),
        (256, 0x1e6, 83.27837, -114.27476, 0.0, -0.918, -1.0, 0, 0, 0, 120),
        (300, 0x1e6, 74.23047, -21.14794, 0.836304, 0.890093, -1.0, 0, 0, 0, 120),
    ] {
        let beat = LucasBeat { frame, status, position: [x, y], velocity: [vx, vy], facing, jumps, ..entry };
        let actual = flight.step(beat).unwrap_or_else(|| panic!("frame {frame} status {status:#x}"));
        assert_eq!(actual.press, press, "frame {frame}");
        assert_eq!(actual.stick, Some([sx as f32 / 120.0, sy as f32 / 120.0]), "frame {frame}");
    }
}

#[test]
fn lucas_only_admits_the_exact_scoped_variant_zero_sentinel() {
    let reach = SpecialReach { rise: 78.0, horizontal: -1.0 };
    assert!(!reach.valid());
    assert!(measured_envelope(KIND, 0, reach));
    assert!(!measured_envelope(KIND, 1, reach));
    assert!(!measured_envelope(0x0a, 0, reach));
    assert!(!measured_envelope(KIND, 0, SpecialReach { rise: f32::INFINITY, ..reach }));
    assert!(!measured_envelope(KIND, 0, SpecialReach { horizontal: -2.0, ..reach }));
}

#[test]
fn lucas_wait_rule_requires_rest_and_the_dodges_own_measured_height() {
    for (out, above, press_at) in [
        (8.0, 24.0, 23), (10.0, 38.0, 33), (12.0, 20.0, 20),
        (14.0, 14.0, 15), (18.0, 22.0, 21), (22.0, 38.0, 33), (24.0, 10.0, 12),
    ] {
        let (facts, beat) = fixture((3, 0), out, above);
        let mut flight = UnderStage::start(&facts, beat).expect("reviewed at-rest wait rule");
        let mut sample = beat;
        let mut y = above;
        let mut vy: f32 = 0.0;
        for frame in 0..=press_at {
            sample.frame = frame;
            sample.position[1] = y;
            sample.velocity[1] = vy;
            assert_eq!(flight.step(sample).unwrap().press, if frame == press_at { GUARD } else { 0 },
                "{out} out, {above} above, frame {frame}");
            vy = (vy - GRAVITY).max(-1.37);
            y += vy;
        }
        assert!(UnderStage::start(&facts, LucasBeat { velocity: [0.0, -1.37], ..beat }).is_none());
    }

    let (facts, beat) = fixture((3, 0), 12.0, 0.0);
    for (y, can_jump, can_thunder) in [(-1.8, true, true), (-0.9, false, true), (-0.9, true, false)] {
        let mut flight = UnderStage::start(&facts, beat).unwrap();
        let at_press = LucasBeat {
            frame: 4, position: [beat.position[0], y], velocity: [0.0, -0.36],
            can_jump, can_thunder, ..beat
        };
        assert_eq!(flight.step(at_press), None);
    }
}

#[test]
fn lucas_accepts_the_lowest_measured_native_dodge_height() {
    let (facts, release) = fixture((3, 0), 12.0, 0.0);
    let mut flight = UnderStage::start(&facts, release).unwrap();
    // fg2-g1-yoga-21, lucas-r7-8out-y0-held28, attempt 2, frame 4.
    let press = LucasBeat {
        frame: 4,
        position: [release.position[0], -1.44],
        velocity: [0.0, -0.36],
        ..release
    };
    assert_eq!(flight.step(press).map(|pad| pad.press), Some(GUARD));
}
