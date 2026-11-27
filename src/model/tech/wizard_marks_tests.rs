lib_unit_tests! {
use super::picker::Spot;
use super::surface::Line;
use super::wizard::{Step, Wizard};
use super::Vec2;
use crate::model::btt::LOOKS;
use crate::model::panes::{EggLook, EggPicture, EGG_WHOLE};
use crate::model::tech_launch::FlightParams;

const CURSOR: Vec2 = Vec2 { x: 19., y: -7. };
// Preserve the six wizard presentation regressions through the shared API.
fn marks(wizard: &Wizard, cursor: Vec2, params: Option<FlightParams>, admitted: bool)
    -> [Option<([f32;2], EggLook)>; LOOKS] {
    super::wizard_marks::marks(wizard.selected, (wizard.step==Step::Spot).then_some(cursor), Some(wizard), params, admitted)
}
fn params() -> FlightParams {
    FlightParams {
        brake: 0.051,
        gravity: 0.087,
        fall_speed_cap: 1.5,
    }
}
fn wizard(selected: bool) -> Wizard {
    let mut w = Wizard::begin(
        crate::model::tech_settings::read(&crate::model::settings::Snapshot::factory()),
        false,
    );
    if selected {
        w.set_spot(spot(Vec2 { x: 0., y: 1. }));
    }
    w
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

#[test]
fn wizard_marks_full_picker_matches_original_normal_arrow_bit_exactly() {
    use super::{add,scale};
    for normal in [Vec2{x:0.,y:1.},Vec2{x:0.,y:-1.},Vec2{x:-1.,y:0.},Vec2{x:1.,y:0.},Vec2{x:0.6,y:0.8}] {
        let s=spot(normal);
        let out=super::wizard_marks::marks(Some(s),Some(CURSOR),None,Some(params()),true);
        let tip=add(s.point(),scale(normal,5.));
        let side=Vec2{x:-normal.y,y:normal.x};
        let back=add(tip,scale(normal,-1.5));
        let expected=[add(s.point(),scale(normal,2.5)),tip,add(back,side),add(back,scale(side,-1.))];
        assert_eq!(out[0],Some(([s.point().x,s.point().y],EggLook{scale:0.3,..EGG_WHOLE})));
        for (slot,point) in out[1..5].iter().zip(expected) {
            assert_eq!(*slot,Some(([point.x,point.y],EggLook{scale:0.12,..EGG_WHOLE})));
        }
        assert!(out[5..LOOKS-1].iter().all(Option::is_none));
        assert_eq!(out[LOOKS-1].unwrap().1.picture,EggPicture::Cursor);
        let running=super::wizard_marks::marks(Some(s),None,None,None,false);
        assert_eq!(running[0],out[0]);
        assert!(running[1..].iter().all(Option::is_none));
    }
    let no_selection=super::wizard_marks::marks(None,Some(CURSOR),None,None,false);
    assert!(no_selection[..LOOKS-1].iter().all(Option::is_none));
    assert_eq!(no_selection[LOOKS-1].unwrap().0,[CURSOR.x,CURSOR.y]);
}

#[test]
fn wizard_marks_spot_cursor_survives_missing_or_invalid_selection() {
    let mut w = wizard(false);
    for selected in [
        None,
        Some(spot(Vec2 { x: 0., y: 0. })),
        Some(spot(Vec2 { x: f32::NAN, y: 1. })),
    ] {
        w.selected = selected;
        let out = marks(&w, CURSOR, Some(params()), true);
        let (xy, look) = out[LOOKS - 1].unwrap();
        assert_eq!(xy, [CURSOR.x, CURSOR.y]);
        assert_eq!(look.picture, EggPicture::Cursor);
        assert!(out[1..LOOKS - 1].iter().all(Option::is_none));
        if selected.is_none() {
            assert!(out[..LOOKS - 1].iter().all(Option::is_none));
        }
    }
}

#[test]
fn wizard_marks_every_step_obeys_the_typed_pool_capacity() {
    for step in [
        Step::Spot,
        Step::Angle,
        Step::Distance,
        Step::Hit,
        Step::Preview,
    ] {
        for selected in [false, true] {
            for admitted in [false, true] {
                for params in [None, Some(params())] {
                    let mut w = wizard(selected);
                    w.step = step;
                    let out = marks(&w, CURSOR, params, admitted);
                    let mut whole = 0;
                    let mut cursor = 0;
                    let mut cracked = 0;
                    for (_, look) in out.into_iter().flatten() {
                        match look.picture {
                            EggPicture::Whole => whole += 1,
                            EggPicture::Cursor => cursor += 1,
                            EggPicture::Cracked => cracked += 1,
                        }
                    }
                    assert!(whole <= 10 && cursor <= 1 && cracked == 0);
                    assert_eq!(cursor, usize::from(step == Step::Spot));
                    if !selected {
                        assert_eq!(whole, 0);
                    } else {
                        let expected = match step {
                            Step::Spot | Step::Angle => 5,
                            Step::Distance | Step::Hit => 6,
                            Step::Preview if admitted && params.is_some() => 10,
                            Step::Preview => 6,
                        };
                        assert_eq!(whole, expected);
                    }
                }
            }
        }
    }
}

#[test]
fn wizard_marks_ghost_matches_existing_planner_normal_offset() {
    use super::{planner, solver::Box2, surface::Surfaces};
    let surfaces = Surfaces::EMPTY;
    let bounds = Box2 {
        min: Vec2 { x: -500., y: -500. },
        max: Vec2 { x: 500., y: 500. },
    };
    for normal in [
        Vec2 { x: 0., y: 1. },
        Vec2 { x: 0., y: -1. },
        Vec2 { x: -1., y: 0. },
        Vec2 { x: 1., y: 0. },
        Vec2 { x: 0.6, y: 0.8 },
    ] {
        let mut w = wizard(false);
        w.set_spot(spot(normal));
        w.degrees = 37.;
        let plan = planner::plan(
            w.config().unwrap(),
            planner::Context {
                surfaces: &surfaces,
                spot: w.selected.unwrap(),
                params: params(),
                blast: bounds,
                view: None,
                damage_multiplier: Some(1.),
            },
            &mut |_| 0,
        )
        .unwrap();
        for step in [Step::Distance, Step::Hit, Step::Preview] {
            w.step = step;
            let out = marks(&w, CURSOR, None, false);
            let (xy, look) = out[5].unwrap();
            assert_eq!(xy, [plan.setup.launch.start.x, plan.setup.launch.start.y]);
            assert_eq!(
                (look.scale, look.alpha, look.picture),
                (0.65, 110, EggPicture::Whole)
            );
            assert_eq!(out[1].unwrap().0, xy);
        }
    }
}

#[test]
fn wizard_marks_perpendicular_arrow_points_into_each_surface() {
    for n in [
        Vec2 { x: 0., y: 1. },
        Vec2 { x: 0., y: -1. },
        Vec2 { x: -1., y: 0. },
        Vec2 { x: 1., y: 0. },
        Vec2 { x: 0.6, y: 0.8 },
    ] {
        let mut w = wizard(false);
        w.set_spot(spot(n));
        w.step = Step::Angle;
        let out = marks(&w, CURSOR, None, false);
        let tail = out[1].unwrap().0;
        let tip = out[2].unwrap().0;
        let along_normal = (tip[0] - tail[0]) * n.x + (tip[1] - tail[1]) * n.y;
        assert!(along_normal < 0.);
    }
}

#[test]
fn wizard_marks_path_uses_only_existing_approximate_preview_flight() {
    use crate::model::tech_launch::{flight, launch_velocity};
    let mut w = wizard(true);
    w.step = Step::Preview;
    let start = w.start_marker().unwrap();
    let angle = w.world_angle().unwrap().to_radians();
    let initial = launch_velocity(
        w.speed,
        Vec2 {
            x: angle.cos(),
            y: angle.sin(),
        },
    )
    .unwrap();
    let out = marks(&w, CURSOR, Some(params()), true);
    for (slot, frame) in (6..10).zip([3, 6, 9, 12]) {
        let displacement = flight(initial, params(), frame).unwrap().displacement;
        assert_eq!(
            out[slot].unwrap().0,
            [start.x + displacement.x, start.y + displacement.y]
        );
        assert_eq!(
            (out[slot].unwrap().1.scale, out[slot].unwrap().1.alpha),
            (0.1, 160)
        );
    }
    for step in [
        Step::Spot,
        Step::Angle,
        Step::Distance,
        Step::Hit,
        Step::Preview,
    ] {
        w.step = step;
        for (p, admitted) in [(None, true), (Some(params()), false)] {
            assert!(marks(&w, CURSOR, p, admitted)[6..10]
                .iter()
                .all(Option::is_none));
        }
        if step != Step::Preview {
            assert!(marks(&w, CURSOR, Some(params()), true)[6..10]
                .iter()
                .all(Option::is_none));
        }
    }
}

#[test]
fn wizard_marks_invalid_params_or_draft_do_not_fabricate_path() {
    let mut w = wizard(true);
    w.step = Step::Preview;
    for p in [
        FlightParams {
            brake: f32::NAN,
            ..params()
        },
        FlightParams {
            gravity: -1.,
            ..params()
        },
        FlightParams {
            fall_speed_cap: f32::INFINITY,
            ..params()
        },
    ] {
        let out = marks(&w, CURSOR, Some(p), true);
        assert!(out[6..10].iter().all(Option::is_none));
        assert!(out[5].is_some()); // The valid actual start marker remains.
    }
    w.speed = f32::NAN;
    assert!(marks(&w, CURSOR, Some(params()), true)[6..10]
        .iter()
        .all(Option::is_none));
    w.degrees = f32::NAN;
    assert!(marks(&w, CURSOR, Some(params()), true)[1..]
        .iter()
        .all(Option::is_none));
}

}
