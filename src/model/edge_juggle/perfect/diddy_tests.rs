use super::*;
use crate::model::edge_juggle::perfect::{Segment, SPECIAL};

const RING: [Segment; 8] = [
    [[-79.99, 0.0], [-79.99, -8.319]],
    [[-79.99, -8.319], [-64.40, -25.111]],
    [[-64.40, -25.111], [-41.40, -40.099]],
    [[-41.40, -40.099], [41.416, -40.118]],
    [[41.416, -40.118], [64.387, -25.103]],
    [[64.387, -25.103], [79.99, -8.319]],
    [[79.99, -8.319], [79.99, 0.0]],
    [[79.99, 0.0], [-79.99, 0.0]],
];

fn facts(stage: (i32, u32), position: Point, outline: &[Segment]) -> RouteFacts<'_> {
    RouteFacts {
        stage,
        fighter: KIND,
        position,
        player: [-75.0, 0.0],
        ledges: [[-79.99, 0.0], [79.99, 0.0]],
        outline,
        blast: [-200.0, 200.0, 200.0, -140.0],
        body_clearance: 11.5,
        ledge_distance: 0.0,
        fall_budget: 1.75,
        reach: [0.0; 2],
        player_reach: 14.0,
    }
}

fn sample() -> BarrelSample {
    BarrelSample {
        frame: 0,
        status: FALL,
        position: [-93.99, -30.0],
        velocity: [0.0, 0.0],
        facing: 1.0,
        can_rise: true,
    }
}

#[test]
fn diddy_barrel_crossing_is_offered_only_for_the_measured_stage_region_and_resource() {
    for stage in [(0, 0), (3, 0), (311, 0)] {
        let s = sample();
        assert!(
            BarrelFlight::start(&facts(stage, s.position, &RING), s).is_some(),
            "{stage:?}"
        );
    }
    for stage in [(0, 1), (182, 0), (311, 1)] {
        let s = sample();
        assert!(BarrelFlight::start(&facts(stage, s.position, &RING), s).is_none());
    }
    for invalid in [
        BarrelSample {
            position: [-103.99, -30.0],
            ..sample()
        },
        BarrelSample {
            position: [-93.99, -38.0],
            ..sample()
        },
        BarrelSample {
            velocity: [1.0, -0.1],
            ..sample()
        },
        BarrelSample {
            can_rise: false,
            ..sample()
        },
        BarrelSample {
            facing: -1.0,
            ..sample()
        },
    ] {
        assert!(BarrelFlight::start(&facts((3, 0), invalid.position, &RING), invalid).is_none());
    }
    let mut blocked = RING;
    blocked[3][0][1] = -78.0;
    let s = sample();
    assert!(BarrelFlight::start(&facts((3, 0), s.position, &blocked), s).is_none());
}

#[test]
fn diddy_barrel_crossing_ramps_down_then_holds_a_full_charge_and_turns_up_at_the_exit() {
    // g3-p5bfd-yoga. Lead-in and exact press pose from diddy-p5-fd-in-14-m30-2.
    let mut s = BarrelSample {
        position: [-93.99, 30.0],
        ..sample()
    };
    let mut flight = BarrelFlight::start(&facts((3, 0), s.position, &RING), s).unwrap();
    for frame in 0..45 {
        s.frame = frame;
        let edit = flight.step(s).unwrap();
        assert_eq!(edit.press, 0);
        assert_eq!(edit.stick, Some([0.0, -((frame + 1).min(8) as f32) / 8.0]));
        s.velocity[1] = (s.velocity[1] - 0.125).max(-1.75);
        s.position[1] += s.velocity[1];
    }
    assert!((s.position[1] + 37.375).abs() < 0.001);
    for i in 0..=68 {
        s.frame = 45 + i;
        s.status = match i {
            0 => FALL,
            1..=7 => START,
            8..=67 => CHARGE,
            _ => FLIGHT,
        };
        let edit = flight.step(s).unwrap();
        assert_eq!(edit.press, if i < 68 { SPECIAL } else { 0 }, "elapsed {i}");
        assert_eq!(
            edit.stick,
            Some(if i < 3 { [0.0, 1.0] } else { [1.0, 0.0] })
        );
    }
    s.frame += 1;
    s.position = [5.0, -66.0];
    assert_eq!(flight.step(s).unwrap().stick, Some([1.0, 0.0]));
    s.frame += 1;
    s.position = [60.0, -60.0];
    assert_eq!(flight.step(s).unwrap().stick, Some([0.0, 1.0]));
    s.frame += 1;
    s.position = [78.8783, -44.2307];
    s.velocity = [1.9024, 2.7071];
    s.status = END;
    assert_eq!(flight.step(s).unwrap().stick, Some([-1.0, 0.0]));
    s.frame += 11;
    s.position = [85.7423, -22.7021];
    s.velocity = [0.374, 1.3321];
    s.status = SPECIAL_FALL;
    let edit = flight.step(s).unwrap();
    assert_eq!(edit.press, 0);
    assert!(matches!(edit.stick, Some([-1.0 | 1.0, 0.0])));
}
