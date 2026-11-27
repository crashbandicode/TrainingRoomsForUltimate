lib_unit_tests! {
use super::picker::Spot;
use super::surface::Line;
use super::wizard::{Action, Outcome, Step, Wizard};
use super::Vec2;
use crate::model::tech_settings::{Config, Mode, Numeric};

#[test]
fn wizard_step_metadata_has_one_complete_order_and_notice_heading() {
    use super::wizard::STEPS;
    assert_eq!(STEPS.len(), 5);
    for (index, (step, title)) in STEPS.iter().enumerate() {
        assert_eq!(step.index(), index);
        assert_eq!(step.title(), *title);
        assert!(title.starts_with("Tech · "));
        assert!(title.contains(&format!("{}/5 ", index + 1)));
    }
}

fn seed() -> Config {
    let random = Numeric {
        mode: Mode::Random,
        lo: 20.,
        hi: 40.,
    };
    Config {
        session: 2,
        surface: 1,
        damage: random,
        speed: random,
        hit_damage: random,
        angle: random,
        start: random,
        time: random,
        delay: random,
        di: 2,
        di_direction: 7,
        facing: 3,
        camera: 1,
    }
}
fn spot(normal: Vec2) -> Spot {
    Spot::near(Line {
            id: 42,
            ends: [Vec2 { x: -20., y: 11. }, Vec2 { x: 20., y: 11. }],
            normal,
            floor: normal.y >= 0.7,
            wall: normal.y == 0.,
            passable: false,
        }, Vec2 { x: 7., y: 11. })
}
fn ready() -> Wizard {
    let mut w = Wizard::begin(seed(), false);
    assert!(w.set_spot(spot(Vec2 { x: 0., y: 1. })));
    w
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 0.0001, "{a} != {b}");
}

#[test]
fn wizard_defaults_and_every_forward_transition() {
    let mut w = ready();
    assert_eq!(
        (w.degrees, w.distance, w.speed, w.hit),
        (90., 30., 100., 12.)
    );
    assert!(!w.hit_focus);
    for step in [Step::Angle, Step::Distance, Step::Hit, Step::Preview] {
        assert_eq!(w.action(Action::Next), Outcome::None);
        assert_eq!(w.step, step);
    }
    assert_eq!(w.action(Action::Next), Outcome::Start);
}
#[test]
fn stored_skip_preview_controls_a_on_hit() {
    use crate::model::{settings, tech_settings};

    for (state, file, expected) in [
        ("factory", None, Outcome::None),
        ("missing setting", Some(r#"{"menu": {}}"#), Outcome::None),
        ("saved On", Some(r#"{"menu": {"tech_wizard_skip": [1,0]}}"#), Outcome::Start),
        ("saved Off", Some(r#"{"menu": {"tech_wizard_skip": [0,1]}}"#), Outcome::None),
    ] {
        let settings = file.map_or_else(settings::Settings::factory, |text| {
            let loaded = settings::parse(text);
            assert!(loaded.issues.is_empty(), "{state}: {:?}", loaded.issues);
            loaded.settings
        });
        let skip = tech_settings::selected(&settings.menu, "tech_wizard_skip") == 0;
        let mut w = Wizard::begin(tech_settings::read(&settings.menu), skip);
        assert!(w.set_spot(spot(Vec2 { x: 0., y: 1. })));
        for step in [Step::Angle, Step::Distance, Step::Hit] {
            assert_eq!(w.action(Action::Next), Outcome::None, "{state}");
            assert_eq!(w.step, step, "{state}");
        }
        assert_eq!(w.action(Action::Next), expected, "{state}");
        let step = if expected == Outcome::Start { Step::Hit } else { Step::Preview };
        assert_eq!(w.step, step, "{state}");
    }
}

#[test]
fn wizard_every_back_transition_and_cancel_preserve_draft() {
    let mut w = ready();
    w.step = Step::Preview;
    w.degrees = 37.;
    w.hit_focus = true;
    for step in [Step::Hit, Step::Distance, Step::Angle, Step::Spot] {
        assert_eq!(w.action(Action::Back), Outcome::None);
        assert_eq!(w.step, step);
        assert_eq!(w.degrees, 37.);
        assert!(w.selected.is_some() && w.hit_focus);
    }
    assert_eq!(w.action(Action::Back), Outcome::Cancelled);
}
#[test]
fn wizard_no_spot_cannot_advance_preview_or_start() {
    for step in [
        Step::Spot,
        Step::Angle,
        Step::Distance,
        Step::Hit,
        Step::Preview,
    ] {
        let mut w = Wizard::begin(seed(), true);
        w.step = step;
        for action in [Action::Next, Action::Defaults, Action::Start] {
            assert_eq!(w.action(action), Outcome::None);
            assert_eq!(w.step, step);
            assert!(w.config().is_none());
        }
    }
}
#[test]
fn wizard_start_from_every_valid_step_and_skip_only_hit_next() {
    for step in [
        Step::Spot,
        Step::Angle,
        Step::Distance,
        Step::Hit,
        Step::Preview,
    ] {
        let mut w = ready();
        w.step = step;
        assert_eq!(w.action(Action::Start), Outcome::Start);
        assert_eq!(w.step, step);
    }
    let mut w = Wizard::begin(seed(), true);
    w.set_spot(spot(Vec2 { x: 0., y: 1. }));
    for step in [Step::Angle, Step::Distance, Step::Hit] {
        assert_eq!(w.action(Action::Next), Outcome::None);
        assert_eq!(w.step, step);
    }
    assert_eq!(w.action(Action::Next), Outcome::Start);
    assert_eq!(w.action(Action::Defaults), Outcome::None);
    assert_eq!(w.step, Step::Preview); // Explicit defaults still request preview.
}
#[test]
fn wizard_defaults_fill_only_remaining_fields() {
    for step in [
        Step::Spot,
        Step::Angle,
        Step::Distance,
        Step::Hit,
        Step::Preview,
    ] {
        let mut w = ready();
        w.step = step;
        (w.degrees, w.distance, w.speed, w.hit) = (17., 53., 170., 23.);
        assert_eq!(w.action(Action::Defaults), Outcome::None);
        assert_eq!(w.step, Step::Preview);
        let expected = match step {
            Step::Spot | Step::Angle => (90., 30., 100., 12.),
            Step::Distance => (17., 30., 100., 12.),
            Step::Hit => (17., 53., 100., 12.),
            Step::Preview => (17., 53., 170., 23.),
        };
        assert_eq!((w.degrees, w.distance, w.speed, w.hit), expected);
    }
}
#[test]
fn wizard_cardinal_angles_and_exact_normal_offset_marker() {
    for (normal, world) in [
        (Vec2 { x: 0., y: 1. }, 270.),
        (Vec2 { x: 0., y: -1. }, 90.),
        (Vec2 { x: -1., y: 0. }, 0.),
        (Vec2 { x: 1., y: 0. }, 180.),
    ] {
        let mut w = ready();
        w.set_spot(spot(normal));
        assert_eq!(w.world_angle(), Some(world));
        assert_eq!(
            w.start_marker(),
            Some(Vec2 {
                x: 7. + normal.x * 30.,
                y: 11. + normal.y * 30.
            })
        );
    }
}
#[test]
fn wizard_slopes_endpoint_order_and_one_degree_precision() {
    let n = Vec2 { x: 0.6, y: 0.8 };
    let mut w = ready();
    let mut s = spot(n);
    w.set_spot(s);
    let inward = (-n.y).atan2(-n.x).to_degrees().rem_euclid(360.);
    close(w.world_angle().unwrap(), inward);
    let before = w.config().unwrap();
    let mut reversed = s.line;
    reversed.ends.reverse();
    s = Spot::near(reversed, s.point());
    w.set_spot(s);
    assert_eq!(w.config().unwrap(), before);
    for a in 0..=360 {
        w.degrees = a as f32;
        close(
            w.world_angle().unwrap(),
            (inward + 90. - a as f32).rem_euclid(360.),
        );
    }
    w.degrees = 91.;
    let config = w.config().unwrap();
    assert_eq!(config.angle.lo, w.world_angle().unwrap());
    assert_ne!(config.angle.lo % 5., 0.);
    assert_eq!(w.start_marker(), Some(Vec2 { x: 25., y: 35. }));
}
#[test]
fn wizard_rejects_invalid_geometry_without_replacing_selection() {
    let mut w = ready();
    let original = w.selected;
    for normal in [
        Vec2 { x: 0., y: 0. },
        Vec2 { x: f32::NAN, y: 1. },
        Vec2 {
            x: f32::INFINITY,
            y: 0.,
        },
    ] {
        assert!(!w.set_spot(spot(normal)));
        assert_eq!(w.selected, original);
    }
    let mut invalid = Spot::at(original.unwrap().line, f32::NAN);
    assert!(!w.set_spot(invalid));
    invalid = original.unwrap();
    invalid.line.ends[1] = invalid.line.ends[0];
    assert!(!w.set_spot(invalid));
    invalid = spot(Vec2 { x: 0.8, y: 0.6 }); // Solid unflagged positive slope isn't targetable.
    assert!(!w.set_spot(invalid));
    invalid = original.unwrap();
    invalid.line.passable = true;
    invalid.line.normal = Vec2 { x: 0., y: -1. };
    assert!(!w.set_spot(invalid));
    // Caller-owned public drafts cannot bypass validation at commit time.
    w.selected = Some(invalid);
    assert!(w.config().is_none());
}
#[test]
fn wizard_native_normal_direction_is_not_renormalized_for_distance() {
    let mut w = ready();
    assert!(w.set_spot(spot(Vec2 {
        x: 0.,
        y: 1.0000001
    })));
    assert_eq!(w.world_angle(), Some(270.));
    assert_eq!(w.start_marker().unwrap().y, 11. + 1.0000001 * 30.);
}
#[test]
fn wizard_invalid_angles_and_numeric_bounds_cannot_start() {
    for value in [f32::NAN, f32::INFINITY, -1., 361.] {
        let mut w = ready();
        w.degrees = value;
        assert_eq!(w.action(Action::Start), Outcome::None);
        assert!(w.config().is_none());
    }
    for (field, values) in [
        (0, [-1., 121., f32::NAN]),
        (1, [79., 251., 105.]),
        (2, [0., 31., 1.5]),
    ] {
        for value in values {
            let mut w = ready();
            match field {
                0 => w.distance = value,
                1 => w.speed = value,
                _ => w.hit = value,
            }
            assert!(w.config().is_none());
            assert_eq!(w.action(Action::Start), Outcome::None);
        }
    }
    for (distance, speed, hit) in [(0., 80., 1.), (120., 250., 30.)] {
        let mut w = ready();
        (w.distance, w.speed, w.hit) = (distance, speed, hit);
        assert!(w.config().is_some());
    }
}
#[test]
fn wizard_config_preserves_all_advanced_seed_fields() {
    let w = ready();
    let mut expected = seed();
    expected.surface = 4;
    let set = |n| Numeric {
        mode: Mode::Set,
        lo: n,
        hi: n,
    };
    expected.angle = set(270.);
    expected.start = set(30.);
    expected.speed = set(100.);
    expected.hit_damage = set(12.);
    assert_eq!(w.config(), Some(expected));
}

#[test]
fn wizard_marker_and_world_angle_match_existing_planner() {
    use super::{planner, solver::Box2, surface::Surfaces};
    let surfaces = Surfaces::EMPTY;
    let bounds = Box2 {
        min: Vec2 { x: -500., y: -500. },
        max: Vec2 { x: 500., y: 500. },
    };
    for n in [
        Vec2 { x: 0., y: 1. },
        Vec2 { x: 0., y: -1. },
        Vec2 { x: -1., y: 0. },
        Vec2 { x: 1., y: 0. },
        Vec2 { x: 0.6, y: 0.8 },
    ] {
        let mut w = ready();
        w.set_spot(spot(n));
        w.degrees = 37.;
        let plan = planner::plan(
            w.config().unwrap(),
            planner::Context {
                surfaces: &surfaces,
                spot: w.selected.unwrap(),
                params: crate::model::tech_launch::FlightParams {
                    brake: 0.051,
                    gravity: 0.087,
                    fall_speed_cap: 1.5,
                },
                blast: bounds,
                view: None,
                damage_multiplier: Some(1.),
            },
            &mut |_| 0,
        )
        .unwrap();
        assert_eq!(plan.setup.launch.start, w.start_marker().unwrap());
        assert_eq!(plan.setup.launch.angle, w.world_angle().unwrap());
        assert!(!plan.approximate);
    }
}

#[test]
fn wizard_adjust_zero_input_and_inactive_steps_do_not_drift() {
    for step in [
        Step::Spot,
        Step::Angle,
        Step::Distance,
        Step::Hit,
        Step::Preview,
    ] {
        let mut w = ready();
        w.step = step;
        w.hit_focus = true;
        for _ in 0..100 {
            w.adjust(0., 0.);
        }
        assert_eq!(
            (w.degrees, w.distance, w.speed, w.hit),
            (90., 30., 100., 12.)
        );
        assert!(w.hit_focus);
        assert_eq!(w.step, step);
        w.adjust(f32::NAN, f32::INFINITY);
        assert_eq!(
            (w.degrees, w.distance, w.speed, w.hit),
            (90., 30., 100., 12.)
        );
        assert!(w.hit_focus);
        if matches!(step, Step::Spot | Step::Preview) {
            w.adjust(2., -2.);
            assert_eq!(
                (w.degrees, w.distance, w.speed, w.hit),
                (90., 30., 100., 12.)
            );
            assert!(w.hit_focus);
        }
    }
}

#[test]
fn wizard_adjust_angle_one_degree_wrap_and_distance_limits() {
    let mut w = ready();
    w.step = Step::Angle;
    w.degrees = 359.;
    w.adjust(2., 0.);
    assert_eq!(w.degrees, 0.);
    w.adjust(-2., 0.);
    assert_eq!(w.degrees, 359.);
    for _ in 0..360 {
        w.adjust(1., 0.);
    }
    assert_eq!(w.degrees, 359.);
    w.step = Step::Distance;
    for _ in 0..200 {
        w.adjust(2., 0.);
    }
    assert_eq!(w.distance, 120.);
    for _ in 0..200 {
        w.adjust(-2., 0.);
    }
    assert_eq!(w.distance, 0.);
    assert_eq!(w.degrees, 359.);
}

#[test]
fn wizard_adjust_hit_vertical_focus_and_horizontal_steps_are_independent() {
    let mut w = ready();
    w.step = Step::Hit;
    w.adjust(2., 0.);
    assert_eq!((w.speed, w.hit), (110., 12.));
    w.adjust(0., -2.);
    assert!(w.hit_focus);
    assert_eq!((w.speed, w.hit), (110., 12.));
    w.adjust(-2., 0.);
    assert_eq!((w.speed, w.hit), (110., 11.));
    w.adjust(2., 2.);
    assert!(!w.hit_focus);
    assert_eq!((w.speed, w.hit), (120., 11.));
    for _ in 0..100 {
        w.adjust(2., 0.);
    }
    assert_eq!(w.speed, 250.);
    for _ in 0..100 {
        w.adjust(-2., 0.);
    }
    assert_eq!(w.speed, 80.);
    for _ in 0..100 {
        w.adjust(2., -2.);
    }
    assert_eq!((w.speed, w.hit), (80., 30.));
    for _ in 0..100 {
        w.adjust(-2., 0.);
    }
    assert_eq!((w.speed, w.hit), (80., 1.));
    assert_eq!(w.step, Step::Hit);
}

#[test]
fn wizard_rebind_fresh_entry_ignores_latched_reset_but_clears_borrowed_ids() {
    for had_epoch in [false, true] {
        let mut w = ready();
        w.step = Step::Preview;
        w.degrees = 37.;
        let cancel = w.wizard_rebind_geometry(true, had_epoch, true);
        assert!(w.selected.is_none());
        assert_eq!(w.step, Step::Spot);
        assert_eq!(w.degrees, 37.);
        assert!(!cancel, "a reset predating this fresh draft must not cancel it");
        let mut rebuilt = spot(Vec2 { x: 0., y: 1. });
        rebuilt.line.id = 99;
        assert!(w.set_spot(rebuilt));
        assert_eq!(w.selected.unwrap().line.id, 99);
        assert_eq!(w.action(Action::Next), Outcome::None);
        assert_eq!(w.step, Step::Angle);
    }
}

#[test]
fn wizard_rebind_live_reset_or_epoch_change_cancels_and_invalidates_geometry() {
    for (had_epoch, reset) in [(true, true), (true, false), (false, true)] {
        let mut w = ready();
        w.step = Step::Hit;
        assert!(w.wizard_rebind_geometry(false, had_epoch, reset));
        assert!(w.selected.is_none());
        assert_eq!(w.step, Step::Spot);
        assert!(w.config().is_none());
    }
}

#[test]
fn wizard_rebind_initial_epoch_does_not_cancel_and_still_requires_fresh_spot() {
    let mut w = ready();
    w.step = Step::Distance;
    assert!(!w.wizard_rebind_geometry(false, false, false));
    assert!(w.selected.is_none());
    assert_eq!(w.step, Step::Spot);
    assert_eq!(w.action(Action::Start), Outcome::None);
}

}
