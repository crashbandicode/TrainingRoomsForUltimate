use super::super::{AirPhysics, ThreatClock, GUARD};
use super::{AttackForecast, MotionSample, ThreatProjection};

fn projection() -> ThreatProjection {
    ThreatProjection {
        remaining_active: [18, 21],
        attack: [[-7.0, 9.0], [7.0, 26.0]],
        torso: [[0.0, 42.0], [1.0, 56.0]],
        cpu_speed: [0.0, -0.7],
        attacker_speed: [0.0, 0.0],
        cpu_air: AirPhysics {
            horizontal_speed: 1.0,
            horizontal_acceleration: 0.1,
            gravity: 0.08,
            terminal_speed: 1.4,
            jump_speed: 1.7,
        },
        invulnerable: [3, 25],
    }
}

#[test]
fn perfect_j2_startup_forecast_reacts_at_each_floor_without_a_live_hitbox() {
    for floor in [0, 6, 12] {
        let mut clock = ThreatClock::default();
        let mut current = projection();
        let first_dodge = (0..18).find(|&age| {
            // The attack is still in startup and the stock active hitbox buffer is empty.
            let hit_covered = current.dodge_covers_contact();
            current.remaining_active = current.remaining_active.map(|frame| frame - 1);
            current.cpu_speed[1] = (current.cpu_speed[1] - current.cpu_air.gravity)
                .max(-current.cpu_air.terminal_speed);
            current.torso = current.torso.map(|[x, y]| [x, y + current.cpu_speed[1]]);
            clock
                .observe(Some(1), u32::from(age), floor, hit_covered, true, 0)
                .press
                == GUARD
        });
        assert_eq!(first_dodge, Some(floor));
    }
}

#[test]
fn startup_dodge_declines_misses_and_contact_outside_the_intangible_window() {
    let p = projection();
    assert!(p.dodge_covers_contact());
    for reject in [
        ThreatProjection {
            attacker_speed: [2.0, 0.0],
            ..p
        },
        ThreatProjection {
            torso: [[0.0, 80.0], [1.0, 94.0]],
            ..p
        },
        ThreatProjection {
            invulnerable: [3, 17],
            ..p
        },
        ThreatProjection {
            invulnerable: [19, 25],
            ..p
        },
        ThreatProjection {
            remaining_active: [0, 1],
            ..p
        },
        ThreatProjection {
            attack: [[f32::NAN, 9.0], [7.0, 26.0]],
            ..p
        },
    ] {
        assert!(!reject.dodge_covers_contact());
    }
}

#[test]
fn timed_dodge_uses_the_neutral_escape_whose_window_was_projected() {
    let mut clock = ThreatClock::default();
    let edit = clock.observe(Some(1), 12, 0, true, true, 0);
    assert_eq!((edit.press, edit.stick), (GUARD, Some([0.0; 2])));
    assert_eq!(clock.observe(Some(1), 13, 0, true, true, 0).stick, None);
}

#[test]
fn native_startup_requires_a_current_finite_record_and_mirrors_its_attack_side() {
    // Exact 953b6039 Ally seq 3 and 61, same Mario up-smash record before any hitbox.
    let live = MotionSample {
        status: 0x33,
        motion: 0xa5598d745,
        frame: 0.0,
    };
    let forecast = AttackForecast {
        source: live,
        flags: 0x80000000,
        bounds: [[-17.0, 12.0], [3.0, 25.0]],
        countdown: [7, 12],
    };
    assert_eq!(
        forecast.for_motion(live, 1.0),
        Some(([7, 12], [[-17.0, 12.0], [3.0, 25.0]]))
    );
    assert_eq!(
        forecast.for_motion(live, -1.0),
        Some(([7, 12], [[-3.0, 12.0], [17.0, 25.0]]))
    );

    for invalid in [
        AttackForecast {
            source: MotionSample { motion: 0, ..live },
            ..forecast
        },
        AttackForecast {
            source: MotionSample { status: 0, ..live },
            ..forecast
        },
        AttackForecast {
            source: MotionSample { frame: 3.0, ..live },
            ..forecast
        },
        AttackForecast {
            source: MotionSample {
                frame: f32::NAN,
                ..live
            },
            ..forecast
        },
        AttackForecast {
            flags: 0x84000000,
            ..forecast
        },
        AttackForecast {
            countdown: [0, 18],
            ..forecast
        },
        AttackForecast {
            countdown: [7, 268],
            ..forecast
        },
        AttackForecast {
            countdown: [13, 8],
            ..forecast
        },
        AttackForecast {
            bounds: [[-17.0, 26.0], [3.0, 25.0]],
            ..forecast
        },
        AttackForecast {
            bounds: [[f32::INFINITY, 12.0], [3.0, 25.0]],
            ..forecast
        },
    ] {
        assert_eq!(invalid.for_motion(live, 1.0), None);
    }
    assert_eq!(forecast.for_motion(live, 0.0), None);
}

#[test]
fn neutral_dodge_window_uses_each_native_body_and_reserves_a_frame_at_each_end() {
    use super::neutral_dodge_window;
    use crate::model::fighter_choices::CHOICES;

    let by_name = |name| CHOICES.iter().find(|&&(_, entry)| name == entry).unwrap().0 as i32;
    assert_eq!(neutral_dodge_window(by_name("Mario")), Some([4, 28]));
    assert_eq!(neutral_dodge_window(by_name("Pikachu")), Some([3, 25]));
    assert_eq!(neutral_dodge_window(by_name("Bayonetta")), Some([6, 30]));
    assert_eq!(neutral_dodge_window(by_name("Mythra")), Some([7, 30]));
    assert_eq!(neutral_dodge_window(by_name("Ice Climbers")), Some([4, 27]));
    assert_eq!(neutral_dodge_window(-1), None);
    assert_eq!(neutral_dodge_window(0x50), None);
    for &(kind, name) in CHOICES {
        assert!(neutral_dodge_window(kind as i32).is_some(), "{name}");
    }
}
