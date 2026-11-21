use super::*;
use crate::model::edge_juggle::perfect::Segment;

const RING: [Segment; 5] = [
    [[-79.986, -8.319], [-64.395, -25.111]],
    [[-64.395, -25.111], [-41.396, -40.099]],
    [[-41.396, -40.099], [41.416, -40.118]],
    [[41.416, -40.118], [64.387, -25.103]],
    [[64.387, -25.103], [79.989, -8.319]],
];

fn facts(stage: (i32, u32), position: Point) -> RouteFacts<'static> {
    RouteFacts {
        fighter: KIND,
        stage,
        position,
        player: [-75.0, 0.0],
        player_reach: 10.0,
        ledges: [[-79.986, 0.0], [79.989, 0.0]],
        blast: [-230.0, 230.0, 180.0, -140.0],
        outline: &RING,
        body_clearance: 12.0,
        ledge_distance: 17.25,
        fall_budget: 1.65,
        reach: [150.0, 70.0],
    }
}

fn release(position: Point) -> SonicBeat {
    SonicBeat {
        frame: 0,
        status: FALL,
        position,
        facing: 1.0,
        jumps: 1,
        can_spindash: true,
        can_jump: true,
        can_spring: true,
    }
}

#[test]
fn sonic_admits_the_proven_cells_and_refuses_unproven_conditions() {
    for stage in [(3, 0), (0, 0), (311, 0)] {
        for position in [
            [-105.986, 6.0],
            [-105.986, 34.0],
            [-125.986, 34.0],
            [-117.986, 22.0],
            [-113.986, 26.0],
            [-121.986, 30.0],
        ] {
            let flight = SonicFlight::start(&facts(stage, position), [0.0; 2], release(position))
                .expect("native Spin Dash, jump and turned Spring completed from this cell");
            assert_eq!(flight.exit_point(), [79.989, 0.0]);
            assert!((140..=170).contains(&flight.crossing_frames()));
        }
        let outside = [-125.986, 10.0];
        assert!(SonicFlight::start(&facts(stage, outside), [0.0; 2], release(outside)).is_none());
    }
    let position = [-105.986, 34.0];
    let ring = facts((3, 0), position);
    let fresh = release(position);
    for sample in [
        SonicBeat {
            can_spindash: false,
            ..fresh
        },
        SonicBeat {
            can_jump: false,
            ..fresh
        },
        SonicBeat {
            can_spring: false,
            ..fresh
        },
        SonicBeat { jumps: 0, ..fresh },
        SonicBeat {
            facing: -1.0,
            ..fresh
        },
    ] {
        assert!(SonicFlight::start(&ring, [0.0; 2], sample).is_none());
    }
    assert!(SonicFlight::start(&ring, [0.0, -0.6], fresh).is_none());
    assert!(SonicFlight::start(&facts((89, 0), position), [0.0; 2], fresh).is_none());
    let right = [105.989, 34.0];
    assert!(SonicFlight::start(&facts((3, 0), right), [0.0; 2], release(right)).is_none());
}

#[test]
fn sonic_keeps_the_two_frame_up_back_press_and_uses_the_native_turn() {
    let position = [-105.986, 6.0];
    let mut beat = release(position);
    let mut flight = SonicFlight::start(&facts((3, 0), position), [0.0; 2], beat).unwrap();
    assert_eq!(flight.step(beat).unwrap().stick, Some([0.0; 2]));
    beat.frame = 3;
    beat.position = [-105.986, 5.46];
    assert_eq!(flight.step(beat).unwrap().press, SPECIAL);
    beat.frame = 4;
    beat.status = SIDE;
    assert_eq!(flight.step(beat).unwrap().press, SPECIAL);
    beat.frame = 20;
    beat.status = SPIN;
    beat.position = [-106.886, -12.69];
    assert_eq!(flight.step(beat).unwrap().press, 0);
    beat.frame = 68;
    beat.position = [-11.406, -99.54];
    assert_eq!(flight.step(beat).unwrap().press, JUMP);
    beat.frame = 69;
    beat.status = AERIAL_JUMP;
    beat.position = [-9.406, -101.94];
    beat.jumps = 0;
    assert_eq!(flight.step(beat).unwrap().press, JUMP);
    beat.frame = 112;
    beat.status = FALL_AERIAL;
    beat.position = [42.186, -77.2159];
    let first = flight.step(beat).unwrap();
    assert_eq!(first.stick, Some([-0.5, 104.0 / 120.0]));
    assert_eq!(first.press, SPECIAL);
    beat.frame = 113;
    beat.status = SPRING;
    beat.position = [43.384, -78.6209];
    beat.facing = -1.0;
    assert_eq!(flight.step(beat).unwrap(), first);
    beat.frame = 114;
    assert_eq!(flight.step(beat).unwrap().press, 0);
    let mut interrupted =
        SonicFlight::start(&facts((3, 0), position), [0.0; 2], release(position)).unwrap();
    assert_eq!(interrupted.step(release(position)).unwrap().press, 0);
    beat.frame = 112;
    beat.status = FALL_AERIAL;
    beat.position[1] = -103.0;
    assert!(interrupted.step(beat).is_none());
}

#[test]
fn sonic_declines_the_crossing_when_the_near_ledge_guarder_can_drop_into_spin_dash() {
    let position = [-105.986, 33.91026];
    let sample = release(position);
    let mut ring = facts((3, 0), position);
    ring.player = [-79.03, 0.01];
    assert!(SonicFlight::start(&ring, [0.0, -0.09], sample).is_some());
    ring.player = [-81.7, -10.7];
    assert!(SonicFlight::start(&ring, [0.0, -0.09], sample).is_none());
}
