#[allow(dead_code)]
#[path = "../src/model/tech_launch.rs"]
mod launch;
use launch::{HitHeight, Launch, Vec2};
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
