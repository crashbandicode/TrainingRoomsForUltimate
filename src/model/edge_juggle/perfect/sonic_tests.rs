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
        spring_reach_valid: true,
    }
}

#[test]
fn sonic_counts_the_edge_teeter_wait_as_standing_and_nothing_else_does() {
    let runtime = include_str!("../../../runtime/features/edge_juggle/niche_pad.rs");
    let (_, admission) = runtime.split_once("if fighter == sonic::KIND {").unwrap();
    let (admission, _) = admission.split_once("SonicFlight::start(").unwrap();
    let (_, step) = runtime.split_once("MeasuredFlight::SpinDash(flight)) = &mut self.measured").unwrap();
    let (step, _) = step.split_once("flight.step(SonicBeat").unwrap();
    for block in [admission, step] {
        assert!(block.contains("sonic_guarder_standing(p1)"));
        assert!(!block.contains("grounded_wait(p1)"));
    }
    assert!(runtime.matches("grounded_wait(p1)").count() >= 4);
    let (_, wait) = runtime.split_once("unsafe fn grounded_wait").unwrap();
    let (wait, _) = wait.split_once('}').unwrap();
    assert!(wait.contains("natives::status_kind(p1) == wait"));
    assert!(!wait.contains("Ottotto"));
}

#[test]
fn the_edge_teeter_wait_counts_as_standing() {
    let (wait, teeter) = (0, 0x7f);
    assert!(guarder_standing(true, wait, wait, teeter));
    assert!(guarder_standing(true, teeter, wait, teeter));
    assert!(!guarder_standing(true, 0x7e, wait, teeter));
    assert!(!guarder_standing(true, 0x77, wait, teeter));
    assert!(!guarder_standing(false, wait, wait, teeter));
    assert!(!guarder_standing(false, teeter, wait, teeter));
    assert!(guarder_standing(true, wait, wait, UNRESOLVED));
    for status in [wait, teeter, UNRESOLVED] {
        assert!(!guarder_standing(true, status, UNRESOLVED, UNRESOLVED));
    }
    assert!(!guarder_standing(true, UNRESOLVED, wait, UNRESOLVED));
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

#[test]
fn sonic_retimes_jump_and_spring_after_the_traced_spin_dash_hitstop() {
    let position = [-105.985_6, 33.910_26];
    let mut beat = release(position);
    let mut flight = SonicFlight::start(&facts((3, 0), position), [0.0, -0.09], beat).unwrap();
    assert_eq!(flight.step(beat).unwrap().press, 0);

    // 90b51e2a seqs 28339/28342/28345. The last sample coincides with
    // the old jump time even though the hit pause cost 12 motion steps.
    let old_jump = u32::from(flight.plan.timing.jump());
    beat.status = SPIN;
    let traced = [
        (2, [-40.005_486, -65.990_005]),
        (1, [-38.005_486, -68.390_01]),
        (0, [-36.005_486, -70.790_01]),
    ];
    for (offset, measured) in traced {
        beat.frame = old_jump - offset;
        beat.position = measured;
        assert_eq!(flight.step(beat).expect("the mature dash can still reach").press, 0);
    }

    let timing = flight.plan.timing;
    let jump = u32::from(timing.jump());
    assert!(jump > old_jump);
    for frame in (old_jump + 1)..=jump {
        beat.frame = frame;
        beat.position = [
            -36.005_486 + 2.0 * (frame - old_jump) as f32,
            -70.790_01 - 2.4 * (frame - old_jump) as f32,
        ];
        let edit = flight.step(beat).unwrap();
        assert_eq!(edit.stick, Some(FORWARD));
        assert_eq!(edit.press, if frame == jump { JUMP } else { 0 });
    }

    let spring_catch = simulate_spring(flight.plan.spring).unwrap();
    assert!(spring_catch.1 >= MIN_SPARE);
    beat.status = AERIAL_JUMP;
    beat.jumps = 0;
    beat.frame += 1;
    beat.position = flight.plan.jump.map(|coord| coord as f32);
    assert_eq!(flight.step(beat).unwrap().press, JUMP);

    beat.frame = u32::from(timing.spring());
    beat.status = FALL_AERIAL;
    beat.position = add(
        flight.plan.spring,
        jump_delta(timing.aerial).map(|coord| -coord),
    )
    .map(|coord| coord as f32);
    assert_eq!(flight.step(beat).unwrap().stick, Some(UP_BACK));
}

#[test]
fn sonic_checks_the_live_dash_before_restarting_its_jump_plan() {
    let position = [-105.985_6, 33.910_26];
    let ready = release(position);
    for sample in [
        SonicBeat { status: FALL, ..ready },
        SonicBeat { status: SPIN, facing: -1.0, ..ready },
        SonicBeat { status: SPIN, jumps: 0, ..ready },
        SonicBeat { status: SPIN, can_jump: false, ..ready },
        SonicBeat { status: SPIN, spring_reach_valid: false, ..ready },
        SonicBeat { status: SPIN, position: [0.0, -74.0], ..ready },
    ] {
        let mut flight =
            SonicFlight::start(&facts((3, 0), position), [0.0, -0.09], ready).unwrap();
        assert!(flight.step(ready).is_some());
        let frame = u32::from(flight.plan.timing.jump());
        for (offset, position) in [(2, [-40.005_486, -65.990_005]), (1, [-38.005_486, -68.390_01])] {
            assert!(flight.step(SonicBeat {
                status: SPIN,
                frame: frame - offset,
                position,
                ..ready
            }).is_some());
        }
        let position = if sample.position == ready.position {
            [-36.005_486, -70.790_01]
        } else {
            sample.position
        };
        assert!(flight.step(SonicBeat { frame, position, ..sample }).is_none());
    }
}
