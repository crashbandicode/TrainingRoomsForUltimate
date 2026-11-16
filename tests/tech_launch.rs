#[allow(dead_code)]
#[path = "../src/model/tech_launch.rs"]
mod tech_launch;
use tech_launch::*;

fn params() -> FlightParams {
    FlightParams {
        brake: 0.5,
        gravity: 0.25,
        fall_speed_cap: 0.5,
    }
}
fn v(x: f32, y: f32) -> Vec2 {
    Vec2 { x, y }
}

#[test]
fn reaction_min_and_adjacent_f32_boundary_bits() {
    // Adjacent KB values around 12.5; decimal constants would hide the boundary.
    for (kb, expected) in [
        (0x4147ffff, 0x40a00000),
        (0x41480000, 0x40a00000),
        (0x41480001, 0x40a00001),
        (0x49742410, 0x48c3500d),
    ] {
        assert_eq!(
            reaction_frames(f32::from_bits(kb), DAMAGE_FRAME_MUL, DAMAGE_FRAME_MIN)
                .unwrap()
                .to_bits(),
            expected
        );
    }
    assert_eq!(reaction_frames(100.0, 0.25, 30.0), Some(30.0));
    assert_eq!(
        reaction_frames(0.0, DAMAGE_FRAME_MUL, DAMAGE_FRAME_MIN),
        Some(5.0)
    );
}

#[test]
fn launch_rounds_scalar_before_direction_and_has_no_percent_input() {
    let direction = v(0.6, 0.8);
    // KB=1000001 rounds KB*.03 to bits 46ea600f before vector scaling.
    assert_eq!(
        launch_velocity(1_000_001.0, v(1.0, 0.0))
            .unwrap()
            .x
            .to_bits(),
        0x46ea600f
    );
    let a = launch_velocity(100.0, direction).unwrap();
    assert_eq!(a, v(1.8000001, 2.4));
    assert_eq!(DAMAGE_AIR_BRAKE.to_bits(), 0x3d50e560);
}

#[test]
fn cardinal_angles_are_exact_including_wrap() {
    for (angle, expected) in [
        (0.0, v(1.0, 0.0)),
        (90.0, v(0.0, 1.0)),
        (180.0, v(-1.0, 0.0)),
        (270.0, v(0.0, -1.0)),
        (-90.0, v(0.0, -1.0)),
        (360.0, v(1.0, 0.0)),
    ] {
        assert_eq!(direction_degrees(angle), Some(expected));
        assert_eq!(
            launch_velocity(100.0, expected),
            Some(v(expected.x * 3.0, expected.y * 3.0))
        );
    }
}

#[test]
fn hand_computed_frame_decay_gravity_cap_and_stop() {
    // Move first, then decay/accelerate: all values here are exactly binary fractions.
    for (frames, displacement, damage, gravity) in [
        (0, v(0.0, 0.0), v(2.0, 0.0), 0.0),
        (1, v(2.0, 0.0), v(1.5, 0.0), 0.25),
        (2, v(3.5, -0.25), v(1.0, 0.0), 0.5),
        (3, v(4.5, -0.75), v(0.5, 0.0), 0.5),
        (4, v(5.0, -1.25), v(0.0, 0.0), 0.5),
        (5, v(5.0, -1.75), v(0.0, 0.0), 0.5),
    ] {
        assert_eq!(
            flight(v(2.0, 0.0), params(), frames),
            Some(Flight {
                displacement,
                damage_velocity: damage,
                gravity_speed: gravity
            })
        );
    }
}

#[test]
fn wall_and_underside_solver_arrivals_are_hand_computed() {
    // Right wall face at x=10, outward normal left; two frames travel (3.5,-.25).
    assert_eq!(
        solve_start(
            v(10.0, 4.0),
            v(2.0, 0.0),
            params(),
            2,
            10.0,
            v(-1.0, 0.0),
            6.0
        ),
        Some(v(6.5, 4.25))
    );
    // Underside y=10, outward normal down; two frames travel (0,3.25).
    assert_eq!(
        solve_start(
            v(3.0, 10.0),
            v(0.0, 2.0),
            params(),
            2,
            10.0,
            v(0.0, -1.0),
            6.0
        ),
        Some(v(3.0, 6.75))
    );
}

#[test]
fn solver_requires_inward_arrival_live_hitstun_and_strict_passive_limit() {
    let solve = |frames, reaction, normal, limit| {
        solve_start(
            v(10.0, 0.0),
            v(2.0, 0.0),
            params(),
            frames,
            reaction,
            normal,
            limit,
        )
    };
    assert_eq!(solve(2, 2.0, v(-1.0, 0.0), 6.0), None);
    assert!(solve(2, f32::from_bits(0x40000001), v(-1.0, 0.0), 6.0).is_some());
    assert_eq!(solve(2, 10.0, v(1.0, 0.0), 6.0), None);
    assert_eq!(solve(2, 10.0, v(-1.0, 0.0), 1.0), None);
    assert!(solve(2, 10.0, v(-1.0, 0.0), f32::from_bits(0x3f800001)).is_some());
    assert_eq!(solve(0, 10.0, v(-1.0, 0.0), 6.0), None);
    // Gravity can exceed the limit while damage energy remains techable.
    let p = FlightParams {
        brake: 0.0,
        gravity: 8.0,
        fall_speed_cap: 8.0,
    };
    assert!(solve_start(v(10.0, 0.0), v(1.0, 0.0), p, 2, 10.0, v(-1.0, 0.0), 6.0).is_some());
}

#[test]
fn invalid_domains_do_not_produce_a_launch_or_solution() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0] {
        assert_eq!(reaction_frames(bad, 0.4, 5.0), None);
        assert_eq!(reaction_frames(100.0, bad, 5.0), None);
        assert_eq!(reaction_frames(100.0, 0.4, bad), None);
        assert_eq!(launch_velocity(bad, v(1.0, 0.0)), None);
        assert_eq!(
            flight(
                v(1.0, 0.0),
                FlightParams {
                    gravity: bad,
                    ..params()
                },
                1
            ),
            None
        );
    }
    assert_eq!(reaction_frames(f32::MAX, 2.0, 5.0), None);
    assert_eq!(direction_degrees(f32::NAN), None);
    assert_eq!(launch_velocity(100.0, v(0.0, 0.0)), None);
    assert_eq!(launch_velocity(100.0, v(f32::NAN, 1.0)), None);
    assert_eq!(flight(v(f32::MAX, f32::MAX), params(), 1), None);
    assert_eq!(flight(v(1.0, 0.0), params(), MAX_FRAMES + 1), None);
    assert_eq!(
        solve_start(
            v(f32::NAN, 0.0),
            v(2.0, 0.0),
            params(),
            2,
            10.0,
            v(-1.0, 0.0),
            6.0
        ),
        None
    );
}
