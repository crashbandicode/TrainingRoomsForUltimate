use super::*;
use crate::model::edge_juggle::perfect::{Segment, JUMP, SPECIAL};

const NEAR: Point = [-79.9856, 0.0];
const LEDGES: [Point; 2] = [NEAR, [79.989, 0.0]];
const OUTLINE: [Segment; 5] = [
    [[-79.986, -8.319], [-64.395, -25.111]],
    [[-64.395, -25.111], [-41.396, -40.099]],
    [[-41.396, -40.099], [41.416, -40.118]],
    [[41.416, -40.118], [64.387, -25.103]],
    [[64.387, -25.103], [79.989, -8.319]],
];

fn fixture(stage: i32, out: f32, below: f32) -> (RouteFacts<'static>, ThunderBeat) {
    let position = [NEAR[0] - out, -below];
    let facts = RouteFacts {
        fighter: 0x0a,
        stage: (stage, 0),
        position,
        player: [-74.0, 0.010259508],
        player_reach: 10.0,
        ledges: LEDGES,
        blast: [-230.0, 230.0, 180.0, -140.0],
        outline: &OUTLINE,
        body_clearance: 8.0,
        ledge_distance: 8.0,
        fall_budget: 1.31,
        reach: [0.0; 2],
    };
    let beat = ThunderBeat {
        frame: 0,
        status: 0xe,
        position,
        velocity: [0.0; 2],
        facing: 1.0,
        jumps: 1,
        can_jump: true,
        can_thunder: true,
    };
    (facts, beat)
}

#[test]
fn ness_offers_the_proven_near_side_programs_with_both_native_resources() {
    for (stage, starts) in [
        (
            3,
            &[
                (20.0, 28.0),
                (84.0, 88.0),
                (120.0, 108.0),
            ][..],
        ),
        (0, &[(20.0, 28.0), (84.0, 88.0)][..]),
        (311, &[(20.0, 28.0), (84.0, 88.0)][..]),
    ] {
        for &(out, below) in starts {
            let (mut facts, release) = fixture(stage, out, below);
            assert!(
                ThunderReturn::start(&facts, release, true).is_some(),
                "{stage} {out} {below}"
            );
            let no_up = ThunderBeat {
                can_thunder: false,
                ..release
            };
            assert!(ThunderReturn::start(&facts, no_up, true).is_none());
            let no_jump = ThunderBeat {
                jumps: 0,
                ..release
            };
            assert!(ThunderReturn::start(&facts, no_jump, true).is_none());
            let away = ThunderBeat {
                facing: -1.0,
                ..release
            };
            assert!(ThunderReturn::start(&facts, away, true).is_none());
            facts.player = [NEAR[0] - 2.0, 0.0];
            assert_eq!(ThunderReturn::start(&facts, release, true).is_some(), deep_stock_loss(facts.stage, out, below));
        }
        for (out, below) in [(20.0, 56.0), (36.0, 24.0), (36.0, 28.0), (84.0, 52.0), (56.0, 72.0), (120.2, 108.2)] {
            let (facts, release) = fixture(stage, out, below);
            assert!(ThunderReturn::start(&facts, release, true).is_none());
        }
    }
    let (facts, release) = fixture(11, 20.0, 28.0);
    assert!(ThunderReturn::start(&facts, release, true).is_none());
}

#[test]
fn ness_deep_route_replays_both_jump_presses_then_casts_the_measured_loop() {
    let (facts, base) = fixture(3, 120.0, 108.0);
    let mut flight = ThunderReturn::start(&facts, base, true).expect("FD deep reference");
    for (frame, status, x, y, vx, vy, jumps, press, stick) in [
        (0, 0xe, -199.986, -108.0, 0.0, 0.0, 1, 0, [0.0, 0.0]),
        (4, 0xe, -199.986, -108.770, 0.0, -0.308, 1, JUMP, [1.0, 0.0]),
        (5, 0xc, -199.886, -109.155, 0.1, -0.385, 0, JUMP, [1.0, 0.0]),
        (54, 0xc, -149.637, -65.785, 0.889, -0.618, 0, 0, [1.0, 0.0]),
        (
            55,
            0xc,
            -148.7692,
            -66.4798,
            0.8675,
            -0.6947,
            0,
            SPECIAL,
            [0.0, 1.0],
        ),
        (
            56,
            0x1de,
            -147.9224,
            -67.2483,
            0.8468,
            -0.7685,
            0,
            SPECIAL,
            [0.0, 1.0],
        ),
        (
            75,
            0x1e4,
            -144.1488,
            -67.2483,
            0.0,
            0.0,
            0,
            0,
            [-118.0 / 120.0, -21.0 / 120.0],
        ),
        (
            112,
            0x1e6,
            -144.1488,
            -73.6743,
            0.0,
            -0.459,
            0,
            0,
            [104.0 / 120.0, 0.5],
        ),
    ] {
        let beat = ThunderBeat {
            frame,
            status,
            position: [x, y],
            velocity: [vx, vy],
            jumps,
            ..base
        };
        let pad = flight.step(beat, Some((facts.player, true))).unwrap_or_else(|| panic!("step {frame}"));
        assert_eq!(
            (pad.press, pad.stick),
            (press, Some(stick)),
            "frame {frame}"
        );
    }
    let mut lost = ThunderReturn::start(&facts, base, true).unwrap();
    let impact = ThunderBeat {
        frame: 55,
        status: 0x48,
        ..base
    };
    assert!(lost.step(impact, Some((facts.player, true))).is_none());
}

#[test]
fn only_ness_variant_zero_can_substitute_the_measured_sentinel() {
    let sentinel = SpecialReach {
        horizontal: -1.0,
        rise: 46.0,
    };
    assert!(!sentinel.valid());
    assert!(measured_envelope(KIND, 0, sentinel));
    for (kind, variant, horizontal, rise) in [
        (0x28, 0, -1.0, 46.0),
        (KIND, 1, -1.0, 46.0),
        (KIND, 0, -2.0, 46.0),
        (KIND, 0, -1.0, 78.0),
        (KIND, 0, f32::NAN, 46.0),
        (KIND, 0, -1.0, f32::INFINITY),
    ] {
        assert!(!measured_envelope(
            kind,
            variant,
            SpecialReach { horizontal, rise }
        ));
    }
}

#[test]
fn ness_leading_fall_is_anchored_to_the_native_release_and_lost_cast_returns_stock() {
    let (mut facts, mut beat) = fixture(3, 20.0, 28.0);
    beat.frame = 3;
    beat.position[1] -= drop(3);
    beat.velocity[1] = -3.0 * GRAVITY;
    facts.position = beat.position;
    let mut flight = ThunderReturn::start(&facts, beat, true).unwrap();
    assert_eq!(flight.step(beat, Some((facts.player, true))).unwrap().press, 0);
    let mut moved = ThunderReturn::start(&facts, beat, true).unwrap();
    assert!(moved.step(beat, Some(([0.0, 0.0], true))).is_none());
    beat.frame = 4;
    beat.position[1] = -28.0 - drop(4);
    beat.can_thunder = false;
    assert!(flight.step(beat, Some((facts.player, true))).is_none());
}

#[test]
fn ness_deep_fixture_is_a_shared_stage_specific_wizard_default() {
    use crate::model::edge_juggle::{Area, Drill, Stage, Wizard};
    let stage = Stage::new(
        [-230.0, 230.0, 180.0, -140.0],
        [-210.0, 210.0, 160.0, -130.0],
        LEDGES,
    )
    .unwrap();
    for (key, expected) in [
        ((3, 0), [120.0, -108.0]),
        ((0, 0), [84.0, -88.0]),
        ((311, 0), [84.0, -88.0]),
    ] {
        let mut wizard =
            Wizard::new(Drill::GuardEdge, stage, [-75.0, 0.0], 0, key, Some(KIND)).unwrap();
        for _ in 0..4 {
            if wizard.setup.area == Area::LowerLeft {
                break;
            }
            wizard.change_area(stage, 1);
        }
        assert_eq!(wizard.setup.area, Area::LowerLeft);
        assert_eq!(wizard.setup.cpu, [NEAR[0] - expected[0], expected[1]]);
    }
}

#[test]
fn ness_jump_to_thunder_launch_matches_six_native_cast_rows() {
    for (out, below, rows, actual_x) in [
        (46.0, 42.0, 14, -102.16),
        (58.0, 74.0, 17, -111.43),
        (84.0, 88.0, 21, -133.85),
        (108.0, 108.0, 48, -134.79),
        (110.0, 110.0, 48, -136.79),
        (114.0, 110.0, 49, -139.91),
    ] {
        let predicted = jump_cast_launch([NEAR[0] - out, -below], rows);
        assert!(
            (predicted[0] - actual_x).abs() <= 0.02,
            "{out} out, {below} below, cast at row {rows}: {predicted:?}, native x {actual_x}"
        );
    }
}

#[test]
fn ness_v3_uses_the_reviewed_program_at_the_live_start() {
    assert_eq!(data::CELLS.len(), 232);
    for (out, below, jump_rows, loop_kind) in [
        (60.0, 76.0, 18, 0),
        (64.0, 84.0, 25, 0),
        (68.0, 84.0, 20, 0),
        (80.0, 56.0, 12, 1),
        (84.0, 56.0, 12, 2),
        (84.0, 60.0, 12, 2),
    ] {
        let selected = program((3, 0), out + 1.0, below + 1.0).unwrap();
        assert_eq!((selected.jump_rows, selected.loop_kind), (jump_rows, loop_kind));
    }

    for (out, below) in [(108.0, 108.0), (110.0, 110.0), (114.0, 110.0)] {
        let (facts, beat) = fixture(3, out, below);
        assert!(ThunderReturn::start(&facts, beat, true).is_some(), "{out} out, {below} below");
    }
}

#[test]
fn ness_standing_window_has_a_lip_minimum_and_no_far_cap() {
    let (mut facts, release) = fixture(3, 20.0, 28.0);
    facts.player = [NEAR[0] + 16.0, 0.01];
    assert!(ThunderReturn::start(&facts, release, true).is_some());
    assert!(ThunderReturn::start(&facts, release, false).is_none());
    facts.player[0] = NEAR[0] + 4.98;
    assert!(ThunderReturn::start(&facts, release, true).is_none());
}

#[test]
fn ness_deep_stock_losses_can_offer_against_a_moving_guarder() {
    for (out, below) in [(108.0, 108.0), (110.0, 110.0), (114.0, 110.0), (120.0, 108.0)] {
        let (mut facts, release) = fixture(3, out, below);
        facts.player = [-125.0, -42.0];
        assert!(ThunderReturn::start(&facts, release, false).is_some(), "{out}, {below}");
    }
    let (mut facts, release) = fixture(3, 84.0, 88.0);
    facts.player = [-125.0, -42.0];
    assert!(ThunderReturn::start(&facts, release, false).is_none());
}

#[test]
fn ness_keeps_the_spent_jump_and_replans_at_cast_against_live_p1() {
    let (facts, start) = fixture(3, 110.0, 110.0);
    let mut flight = ThunderReturn::start(&facts, start, true).unwrap();
    assert_eq!(flight.program.loop_kind, 1);
    // Both measured loops can catch here. In this P1 pose, loop 2 has the
    // wider minimum normalized squared approach margin (-0.772 vs -0.815).
    let moving_guarder = Some(([-126.7856, -80.5964], false));
    let cast = ThunderBeat {
        frame: 52,
        status: JUMP_AERIAL,
        position: [-141.3681, -66.6417],
        jumps: 0,
        ..start
    };
    assert_eq!(flight.step(cast, moving_guarder), Some(pad([0.0, 1.0], SPECIAL)));
    assert_eq!(flight.program.loop_kind, 2, "switch to the farther loop at the cast");
    let no_thunder = ThunderBeat { can_thunder: false, ..cast };
    let mut invalid = ThunderReturn::start(&facts, start, true).unwrap();
    assert!(invalid.step(no_thunder, moving_guarder).is_none());
}
