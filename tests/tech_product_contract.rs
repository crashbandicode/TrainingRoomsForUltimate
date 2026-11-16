#[allow(dead_code)]
#[path = "../src/model/tech_launch.rs"]
mod launch;
use launch::{Attempt, AttemptState, ContactReceipt, HitHeight, Launch, Surface, Vec2};
fn request() -> Launch {
    Launch {
        start: Vec2 { x: -30.0, y: 20.0 },
        target: Vec2 { x: 0.0, y: 0.0 },
        angle: 315.0,
        knockback: 100.0,
        percent: 60.0,
        hit_damage: 12.0,
        height: HitHeight::Middle,
        facing: 1.0,
        hit_from_behind: false,
        grounded_hit: false,
        di: Some(Vec2::default()),
    }
}
#[test]
fn product_request_rejects_nonfinite_fields_before_native_mutation() {
    let good = request();
    assert!(good.valid());
    for bad in [
        Launch {
            start: Vec2 {
                x: f32::NAN,
                y: 0.0,
            },
            ..good
        },
        Launch {
            target: Vec2 {
                x: 0.0,
                y: f32::INFINITY,
            },
            ..good
        },
        Launch {
            angle: f32::NAN,
            ..good
        },
        Launch {
            knockback: 0.0,
            ..good
        },
        Launch {
            percent: -1.0,
            ..good
        },
        Launch {
            hit_damage: f32::NAN,
            ..good
        },
        Launch {
            facing: 0.0,
            ..good
        },
        Launch {
            di: Some(Vec2 {
                x: f32::INFINITY,
                y: 0.0,
            }),
            ..good
        },
    ] {
        assert!(!bad.valid(), "{bad:?}");
    }
}
#[test]
fn hit_side_is_independent_of_world_angle() {
    for angle in [38.0, 142.0, 218.0, 322.0] {
        for (facing, behind, reserve, away_right) in [
            (1.0, false, 1.0, false),
            (1.0, true, -1.0, true),
            (-1.0, false, -1.0, true),
            (-1.0, true, 1.0, false),
        ] {
            let r = Launch {
                angle,
                facing,
                hit_from_behind: behind,
                ..request()
            };
            assert!(r.valid());
            assert_eq!(r.reserve_damage_lr(), reserve);
            assert_eq!(r.away_from_hit_toward_right(), away_right);
            assert_eq!(r.angle.to_bits(), angle.to_bits());
        }
    }
}
#[test]
fn neutral_di_and_player_di_are_distinct_requests() {
    let neutral = request();
    let player = Launch {
        di: None,
        ..neutral
    };
    assert!(neutral.valid() && player.valid());
    assert_ne!(neutral.di, player.di);
    assert!(!Launch {
        di: Some(Vec2 { x: 1.01, y: 0.0 }),
        ..neutral
    }
    .valid());
    assert_eq!(neutral.percent, 60.0);
    assert_eq!(neutral.hit_damage, 12.0);
}

#[test]
fn airborne_start_never_requests_ground_to_air() {
    for angle in [38.0, 142.0, 218.0, 322.0] {
        for facing in [-1.0, 1.0] {
            let air = Launch {
                angle,
                facing,
                grounded_hit: false,
                ..request()
            };
            assert!(!air.ground_to_air());
            assert!(Launch {
                grounded_hit: true,
                ..air
            }
            .ground_to_air());
        }
    }
    let source = include_str!("../src/runtime/features/tech_adapter.rs");
    assert!(source.contains("ground_to_air: owned.launch.ground_to_air()"));
    assert!(!source.contains("ground_to_air: owned.saved.grounded"));
}

fn contact(passive_enabled: bool) -> ContactReceipt {
    ContactReceipt {
        surface: Surface::Wall,
        requested_status: 42,
        passive_enabled,
        position: Vec2 { x: 20.0, y: 30.0 },
    }
}

#[test]
fn attempt_observations_do_not_invent_contact() {
    let mut attempt = Attempt::held(1);
    attempt.observe_stop(false);
    attempt.record_contact(contact(true));
    assert_eq!(attempt.state, AttemptState::Held);
    assert_eq!(attempt.contact, None);
    attempt.launched();
    assert_eq!(attempt.state, AttemptState::Stopped);
    attempt.observe_stop(true);
    assert_eq!(attempt.state, AttemptState::Stopped);
    attempt.observe_stop(false);
    assert_eq!(attempt.state, AttemptState::Flying);
    assert_eq!(attempt.contact, None);
    attempt.mark_abandoned();
    attempt.record_contact(contact(true));
    assert_eq!(attempt.state, AttemptState::Abandoned);
    assert_eq!(attempt.contact, None);
    assert_eq!(Attempt::held(2).contact, None);
}

#[test]
fn contact_receipt_is_first_native_decision() {
    for stopped in [true, false] {
        let mut attempt = Attempt::held(3);
        attempt.launched();
        attempt.observe_stop(stopped);
        let mut invalid = contact(true);
        invalid.position.x = f32::NAN;
        attempt.record_contact(invalid);
        assert_eq!(attempt.contact, None);
        attempt.record_contact(contact(false));
        assert_eq!(attempt.state, AttemptState::Contact);
        attempt.record_contact(contact(true));
        assert_eq!(attempt.contact, Some(contact(false)));
        attempt.mark_abandoned();
        assert_eq!(attempt.state, AttemptState::Abandoned);
        assert_eq!(attempt.contact, Some(contact(false)));
    }
}
