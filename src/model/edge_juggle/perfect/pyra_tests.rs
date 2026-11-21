use super::*;
use crate::model::edge_juggle::perfect::{niche::pad, Segment, JUMP, SPECIAL};

const NEAR: Point = [-79.986, 0.0];
const FAR: Point = [79.989, 0.0];
const OUTLINE: [Segment; 5] = [
    [[-79.986, -8.319], [-64.395, -25.111]],
    [[-64.395, -25.111], [-41.396, -40.099]],
    [[-41.396, -40.099], [41.416, -40.118]],
    [[41.416, -40.118], [64.387, -25.103]],
    [[64.387, -25.103], [79.989, -8.319]],
];
const LOW: Beat = Beat {
    frame: 2,
    fighter: PYRA,
    status: FALL,
    position: [-184.986, -50.0],
    velocity: [0.0, -0.156],
    facing: 1.0,
    jumps: 1,
    can_jump: true,
    can_swap: true,
    can_ray: false,
};

fn facts(stage: (i32, u32), beat: Beat) -> RouteFacts<'static> {
    RouteFacts {
        fighter: beat.fighter,
        stage,
        position: beat.position,
        player: [-75.0, 0.0],
        player_reach: 12.0,
        ledges: [NEAR, FAR],
        blast: [-230.0, 230.0, 185.0, -140.0],
        outline: &OUTLINE,
        body_clearance: 15.0,
        ledge_distance: 15.0,
        fall_budget: 1.62,
        reach: [110.0, 80.0],
    }
}

fn tick(frame: u32, fighter: i32, status: i32) -> Beat {
    Beat { frame: LOW.frame + frame, fighter, status, ..LOW }
}

#[test]
fn pyra_low_jump_first_is_admitted_on_measured_stages_with_the_required_resources() {
    for stage in [(3, 0), (0, 0), (311, 0)] {
        assert!(SwapReturn::start(&facts(stage, LOW), LOW).is_some());
        for invalid in [
            Beat { jumps: 0, ..LOW },
            Beat { can_jump: false, ..LOW },
            Beat { can_swap: false, ..LOW },
            Beat { facing: -1.0, ..LOW },
            Beat { fighter: MYTHRA, ..LOW },
            Beat { position: [NEAR[0] - 124.0, -50.0], ..LOW },
            Beat { velocity: [1.0, 0.0], ..LOW },
        ] {
            assert!(SwapReturn::start(&facts(stage, invalid), invalid).is_none());
        }
    }
    for stage in [(44, 0), (3, 1)] {
        assert!(SwapReturn::start(&facts(stage, LOW), LOW).is_none());
    }
    let stage = facts((3, 0), LOW);
    assert!(SwapReturn::start(&RouteFacts { outline: &[], ..stage }, LOW).is_none());
}

#[test]
fn pyra_jump_first_preserves_the_swapped_attempt_and_the_measured_ray_pad_edges() {
    let mut flight = SwapReturn::start(&facts((3, 0), LOW), LOW).unwrap();
    assert_eq!(flight.step(LOW), Some(pad([1.0, 0.0], JUMP)));
    assert_eq!(flight.step(tick(1, PYRA, JUMP_AERIAL)), Some(pad([1.0, 0.0], JUMP)));
    assert_eq!(flight.step(tick(40, PYRA, JUMP_AERIAL)), Some(pad([1.0, 0.0], 0)));
    assert_eq!(flight.step(tick(41, PYRA, JUMP_AERIAL)), Some(pad([0.0, -0.6], SPECIAL)));
    assert!(!flight.waiting_for_mythra(LOW.frame + 42));
    assert_eq!(flight.step(tick(42, PYRA, SWAP_START)), Some(pad([0.0, -0.6], SPECIAL)));
    assert_eq!(flight.step(tick(52, PYRA, SWAP_START)), Some(pad([1.0, 0.0], 0)));
    let handoff = LOW.frame + 53;
    assert!(flight.waiting_for_mythra(handoff));
    assert!(!flight.waiting_for_mythra(LOW.frame + 68));
    assert!(!flight.follow(handoff, 0, SWAP_END));
    assert!(!flight.follow(handoff, MYTHRA, FALL));
    assert_eq!(flight.step(tick(53, MYTHRA, SWAP_END)), None);
    assert!(flight.follow(handoff, MYTHRA, SWAP_END));
    assert!(!flight.waiting_for_mythra(handoff));
    assert_eq!(flight.step(tick(53, MYTHRA, SWAP_END)), Some(pad([1.0, 0.0], 0)));
    let press = tick(68, MYTHRA, SWAP_END);
    assert_eq!(flight.step(press), None, "an unavailable native transition stays stock");
    assert_eq!(flight.step(Beat { can_ray: true, ..press }), Some(pad([0.0, 1.0], SPECIAL)));
    for frame in [69, 70, 71, 72, 75, 76] {
        assert_eq!(flight.step(tick(frame, MYTHRA, RAY_START)), Some(pad([0.0, 1.0], SPECIAL)));
    }
    for (frame, status) in [(73, RAY_START), (74, RAY_START), (78, RAY_RISE), (130, RAY_GLIDE)] {
        assert_eq!(flight.step(tick(frame, MYTHRA, status)), Some(pad([1.0, 0.0], 0)));
    }
    assert_eq!(flight.step(tick(80, MYTHRA, 0x49)), None, "hitstun aborts the reference sequence");
}

#[test]
fn pyra_swap_first_uses_the_transferred_jump_then_ray() {
    let start = Beat { position: [NEAR[0] - 64.0, -10.0], ..LOW };
    let mut flight = SwapReturn::start(&facts((3, 0), start), start).unwrap();
    assert_eq!(flight.step(start), Some(pad([0.0, -0.6], SPECIAL)));
    assert_eq!(flight.step(tick(1, PYRA, SWAP_START)), Some(pad([0.0, -0.6], SPECIAL)));
    assert!(flight.follow(LOW.frame + 12, MYTHRA, SWAP_END));
    assert_eq!(flight.step(tick(26, MYTHRA, SWAP_END)), Some(pad([1.0, 0.0], 0)));
    let jump = tick(27, MYTHRA, SWAP_END);
    assert_eq!(flight.step(Beat { jumps: 0, ..jump }), None);
    assert_eq!(flight.step(jump), Some(pad([1.0, 0.0], JUMP)));
    assert_eq!(flight.step(tick(28, MYTHRA, JUMP_AERIAL)), Some(pad([1.0, 0.0], JUMP)));
    assert_eq!(flight.step(tick(48, MYTHRA, JUMP_AERIAL)), Some(pad([1.0, 0.0], 0)));
    assert_eq!(
        flight.step(Beat { can_ray: true, ..tick(49, MYTHRA, JUMP_AERIAL) }),
        Some(pad([0.0, 1.0], SPECIAL)),
    );
}
