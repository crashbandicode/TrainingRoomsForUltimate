use super::*;

const COMMON: [crate::model::edge_juggle::perfect::Segment; 5] = [
    [[-79.986, -8.319], [-64.395, -25.111]],
    [[-64.395, -25.111], [-41.396, -40.099]],
    [[-41.396, -40.099], [41.416, -40.118]],
    [[41.416, -40.118], [64.387, -25.103]],
    [[64.387, -25.103], [79.989, -8.319]],
];

fn facts() -> RouteFacts<'static> {
    RouteFacts {
        fighter: KIND,
        stage: (3, 0),
        position: [-99.986, -4.0],
        player: [-75.0, 0.0],
        player_reach: 12.0,
        ledges: [[-79.986, 0.0], [79.989, 0.0]],
        blast: [-240.0, 240.0, 180.0, -140.0],
        outline: &COMMON,
        body_clearance: 17.0,
        ledge_distance: 20.0,
        fall_budget: 1.73,
        reach: [50.0, 106.0],
    }
}

fn initial(stage: &RouteFacts<'_>) -> CypherSample {
    CypherSample {
        frame: 0,
        status: 0xe,
        position: stage.position,
        velocity: [0.0, 0.0],
        facing: -1.0,
        jumps: 1,
        can_jump: true,
        can_rise: true,
    }
}

#[test]
fn snake_accepts_a_measured_left_release_on_all_three_normal_stages() {
    for stage in [(0, 0), (3, 0), (311, 0)] {
        let facts = RouteFacts { stage, ..facts() };
        let flight =
            CypherFlight::start(&facts, initial(&facts)).expect("flown 20-out, 4-below start");
        assert_eq!(flight.exit_point(), facts.ledges[1]);
    }
}

#[test]
fn snake_rejects_unflown_shape_form_facing_and_resources() {
    let base = facts();
    let mut altered = COMMON;
    altered[3][1][1] -= 2.0;
    for facts in [
        RouteFacts {
            stage: (5, 0),
            ..facts()
        },
        RouteFacts {
            stage: (3, 1),
            ..facts()
        },
        RouteFacts {
            outline: &altered,
            ..facts()
        },
        RouteFacts {
            position: [-127.986, 0.0],
            ..facts()
        },
        RouteFacts {
            position: [-109.986, -30.0],
            ..facts()
        },
        RouteFacts {
            position: [99.989, -4.0],
            ..facts()
        },
    ] {
        assert!(CypherFlight::start(&facts, initial(&facts)).is_none());
    }
    for sample in [
        CypherSample {
            status: 0xc,
            ..initial(&base)
        },
        CypherSample {
            facing: 1.0,
            ..initial(&base)
        },
        CypherSample {
            jumps: 0,
            ..initial(&base)
        },
        CypherSample {
            can_rise: false,
            ..initial(&base)
        },
        CypherSample {
            position: [f32::NAN, -4.0],
            ..initial(&base)
        },
    ] {
        assert!(CypherFlight::start(&base, sample).is_none());
    }
}

#[test]
fn snake_reference_pads_drift_jump_arm_then_reverse_after_ride() {
    use crate::model::edge_juggle::perfect::{niche::pad, JUMP, SPECIAL};
    let facts = facts();
    let first = initial(&facts);
    let mut flight = CypherFlight::start(&facts, first).expect("reference start");
    let events = [
        (first, pad([1.0, 0.0], 0)),
        (
            CypherSample {
                frame: 75,
                position: [-43.68303, -112.52009],
                velocity: [0.987, -1.73],
                ..first
            },
            pad([1.0, 0.0], 0),
        ),
        (
            CypherSample {
                frame: 76,
                position: [-42.69603, -114.25009],
                velocity: [0.987, -1.73],
                ..first
            },
            pad([1.0, 0.0], JUMP),
        ),
        (
            CypherSample {
                frame: 77,
                status: 0xc,
                position: [-41.70903, -115.980095],
                jumps: 0,
                velocity: [0.987, -1.73],
                ..first
            },
            pad([1.0, 0.0], JUMP),
        ),
        (
            CypherSample {
                frame: 78,
                status: 0xc,
                position: [-40.77903, -113.68532],
                jumps: 0,
                velocity: [0.93, 2.294777],
                ..first
            },
            pad([1.0, 0.0], 0),
        ),
        (
            CypherSample {
                frame: 101,
                status: 0xc,
                position: [-18.105042, -82.98537],
                jumps: 0,
                velocity: [0.987, 0.45478058],
                ..first
            },
            pad([1.0, 0.0], 0),
        ),
        (
            CypherSample {
                frame: 102,
                status: 0xc,
                position: [-17.118042, -82.61059],
                jumps: 0,
                velocity: [0.987, 0.37477875],
                ..first
            },
            pad([0.0, 1.0], SPECIAL),
        ),
        (
            CypherSample {
                frame: 103,
                status: 0x1de,
                position: [-16.1, -82.3],
                jumps: 0,
                velocity: [0.98, 0.215],
                ..first
            },
            pad([0.0, 1.0], SPECIAL),
        ),
        (
            CypherSample {
                frame: 104,
                status: 0x1de,
                position: [-15.2, -82.2],
                jumps: 0,
                velocity: [0.97, 0.135],
                ..first
            },
            pad([0.0, 1.0], SPECIAL),
        ),
        (
            CypherSample {
                frame: 105,
                status: 0x1de,
                position: [-14.2, -82.2],
                jumps: 0,
                velocity: [0.97, 0.055],
                ..first
            },
            pad([1.0, 0.0], 0),
        ),
        (
            CypherSample {
                frame: 109,
                status: 0x1f4,
                position: [-10.2, -82.2],
                jumps: 0,
                velocity: [0.98, 0.0],
                ..first
            },
            pad([1.0, 0.0], 0),
        ),
        (
            CypherSample {
                frame: 196,
                status: 0x1f4,
                position: [75.4, -30.3],
                jumps: 0,
                velocity: [0.85, 0.91],
                ..first
            },
            pad([1.0, 0.0], 0),
        ),
        (
            CypherSample {
                frame: 197,
                status: 0x1f5,
                position: [76.2, -29.5],
                jumps: 0,
                velocity: [0.85, 0.83],
                ..first
            },
            pad([-1.0, 0.0], 0),
        ),
    ];
    for (sample, expected) in events {
        assert_eq!(
            flight.step(sample),
            Some(expected),
            "macro frame {}",
            sample.frame
        );
    }
}

#[test]
fn snake_repeats_the_committed_press_on_duplicate_thinks() {
    let facts = facts();
    let first = initial(&facts);
    let mut flight = CypherFlight::start(&facts, first).expect("reference start");
    let jump = CypherSample {
        frame: 76,
        position: [-42.69603, -114.25009],
        velocity: [0.987, -1.73],
        ..first
    };
    assert_eq!(flight.step(jump), Some(pad(FORWARD, JUMP)));
    assert_eq!(flight.step(jump), Some(pad(FORWARD, JUMP)));
    assert_eq!(
        flight.step(CypherSample { frame: 77, status: JUMP_AERIAL, jumps: 0, ..jump }),
        Some(pad(FORWARD, JUMP))
    );
    let rise = CypherSample {
        frame: 102,
        status: JUMP_AERIAL,
        position: [-17.118042, -82.61059],
        velocity: [0.987, 0.37477875],
        jumps: 0,
        ..first
    };
    assert_eq!(flight.step(rise), Some(pad(UP, SPECIAL)));
    assert_eq!(flight.step(rise), Some(pad(UP, SPECIAL)));
    assert_eq!(
        flight.step(CypherSample { frame: 103, status: CYPHER_START, ..rise }),
        Some(pad(UP, SPECIAL))
    );
}
