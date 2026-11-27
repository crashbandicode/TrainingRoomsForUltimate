//! Tech practice, Auto start: every target a producer hands to the solver is a target the solver takes, and one draw
//! without a start does not end the session (the docs pass, 2026-11-27: with factory settings the drill stopped after
//! three launches with "Approximate aim has no solution for these values").
use super::*;
use crate::model::tech_launch::FlightParams;
use crate::model::tech_settings::{Mode, Numeric};
use surface::{Kind, Line, Source, Surfaces, END_MARGIN};

const PARAMS: FlightParams = FlightParams { brake: 0.051, gravity: 0.087, fall_speed_cap: 1.5 };
const CAMERA: [f32; 4] = [-1000., 1000., 1000., -1000.];

fn v(x: f32, y: f32) -> Vec2 {
    Vec2 { x, y }
}
fn bits(p: Vec2) -> [u32; 2] {
    [p.x.to_bits(), p.y.to_bits()]
}

/// One solid block whose top is the floor from `a` to `b`, 60 units deep: a flat or a sloped stage's shape.
fn block(a: Vec2, b: Vec2) -> Surfaces {
    let d = sub(b, a);
    let side = d.x.signum();
    let low = a.y.min(b.y) - 60.;
    let vertices = [a, b, v(b.x, low), v(a.x, low)];
    let normals = [scale(v(-d.y * side, d.x * side), 1. / dot(d, d).sqrt()), v(side, 0.), v(0., -1.), v(-side, 0.)];
    let mut surfaces = Surfaces::EMPTY;
    for i in 0..4 {
        assert!(surfaces.push(Line {
            id: i + 1,
            ends: [vertices[i], vertices[(i + 1) % 4]],
            normal: normals[i],
            floor: i == 0,
            wall: i == 1 || i == 3,
            passable: false,
        }));
    }
    surfaces.state = surface::Walk::Closed;
    surfaces
}

struct Still<'a>(&'a Surfaces);
impl Source for Still<'_> {
    fn read(&mut self, id: usize) -> Option<Line> {
        self.0.lines().find(|l| l.id == id)
    }
    fn next(&mut self, _: Line, _: usize) -> Option<(usize, usize)> {
        None
    }
}

/// The production picker's floor spot for one of its 65,536 positions.
fn drawn(surfaces: &Surfaces, unit: u32) -> picker::Spot {
    picker::random_spot(surfaces, &mut Still(surfaces), Some(Kind::Floor), CAMERA,
        &mut |n| if n == 65536 { unit } else { 0 }, &mut |_, _| {}).expect("a floor spot")
}
/// The Chosen point: the cursor's nearest floor point, as `Picker::feed` takes it.
fn chosen(surfaces: &Surfaces, cursor: Vec2) -> picker::Spot {
    picker::Spot::nearest(surfaces, cursor, Some(Kind::Floor)).expect("a floor under the cursor")
}
/// The Chosen point after the stage loaded again (`Picker::enter`, then `Picker::restore`).
fn restored(surfaces: &Surfaces, spot: picker::Spot) -> picker::Spot {
    let mut picker = picker::Picker::EMPTY;
    picker.enter(7, spot.point());
    picker.selected = Some(spot);
    picker.enter(7, spot.point());
    assert!(picker.restore(surfaces));
    picker.selected.expect("a restored spot")
}
fn context(surfaces: &Surfaces, spot: picker::Spot) -> planner::Context<'_> {
    planner::Context {
        surfaces,
        spot,
        params: PARAMS,
        blast: solver::editor_bounds([-240., 240., 192., -140.]),
        view: None,
        damage_multiplier: Some(1.0),
    }
}
fn solved(surfaces: &Surfaces, spot: picker::Spot, frames: u16) -> bool {
    let context = context(surfaces, spot);
    solver::Aim { surfaces, spot, knockback: 100., params: context.params, blast: context.blast, view: None }
        .solve(None, frames)
        .is_some()
}
/// Factory settings with Surface = Floor: 100 KB, Auto angle and start, a random time of 20 to 38 frames.
fn factory_floor() -> crate::model::tech_settings::Config {
    let mut config = crate::model::tech_settings::read(&crate::model::settings::Settings::factory().menu);
    config.surface = 1;
    config
}

#[test]
fn traced_wall_launch_needs_an_approach_from_the_contact_side() {
    // The two draws at b7-trace-yoga-01:397 and :979 share the right-hand wall.
    // Close the solid above the measured face; no unmeasured stage boundaries are needed.
    let a = v(41.416_157, -40.118_07);
    let b = v(64.387_29, -25.103_106);
    let c = v(a.x, b.y);
    let wall = Line { id: 1, ends: [a, b], normal: v(0.547_131_3, -0.837_046_74),
        floor: false, wall: true, passable: false };
    let mut surfaces = Surfaces::EMPTY;
    for line in [wall, Line { id: 2, ends: [b, c], normal: v(0., 1.), floor: true, wall: false, ..wall },
                 Line { id: 3, ends: [c, a], normal: v(-1., 0.), ..wall }] {
        assert!(surfaces.push(line));
    }
    surfaces.state = surface::Walk::Closed;
    let run = |t, frames, angle: Option<f32>| {
        let mut config = factory_floor();
        config.surface = 2;
        config.time = Numeric { mode: Mode::Set, lo: frames, hi: frames };
        if let Some(degrees) = angle {
            config.angle = Numeric { mode: Mode::Set, lo: degrees, hi: degrees };
        }
        let ctx = planner::Context { damage_multiplier: Some(1.2), ..context(&surfaces, picker::Spot::at(wall, t)) };
        planner::plan(config, ctx, &mut |_| 0).ok().map(|p| p.setup.launch)
    };
    let normal = 123.170_44;
    let hit = run(0.654_409_35, 26., Some(normal)).expect("native wall bounce, request and entry 76");
    assert_eq!((hit.start, hit.knockback, hit.percent), (v(90.056_21, -56.396_698), 100., 72.));
    let t = 0.699_877_26;
    assert_eq!(picker::Spot::at(wall, t).point(), v(57.493_134, -29.609_436));
    let fixed = run(t, 25., Some(80.)).map(|launch| launch.angle);
    let auto = run(t, 25., None).map(|launch| launch.angle);
    // Solid is left of A->B, so rotate its tangent counterclockwise: (-dy, dx).
    let inward = (b.x - a.x).atan2(a.y - b.y).to_degrees();
    assert_eq!((fixed, auto), (None, Some(inward)), "the fly-by must not replace the normal approach");
}

#[test]
fn factory_draws_on_a_flat_floor_never_need_a_second_candidate() {
    for params in [PARAMS, FlightParams { gravity: 0.085, fall_speed_cap: 1.85, ..PARAMS }] {
        factory_first_candidates(params);
    }
}
fn factory_first_candidates(params: FlightParams) {
    let surfaces = block(v(85., 0.), v(-85., 0.));
    // Every frame count the factory time range can draw, over the floor.
    for unit in (0..=65535u32).step_by(97) {
        let spot = drawn(&surfaces, unit);
        for frames in 20..=38u16 {
            let attempt: Result<_, ()> = planner::draw_attempt(|| {
                let ctx = planner::Context { params, ..context(&surfaces, spot) };
                Ok(planner::plan(factory_floor(), ctx, &mut |n| if n == 19 { u32::from(frames - 20) } else { 0 }).ok())
            });
            assert_eq!(attempt.unwrap().map(|(p, nth)| (p.contact_frames, nth)), Some((frames, 1)), "picker unit {unit}");
        }
    }
    // As played: 1,000 attempts, each a spot from the picker and a plan from the planner, on one seeded stream.
    let mut seed = 0x2545_f491u32;
    let mut draw = |n: u32| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        ((u64::from(seed >> 8) * u64::from(n)) >> 24) as u32
    };
    let mut redraws = 0;
    for _ in 0..1000 {
        let attempt: Result<_, ()> = planner::draw_attempt(|| {
            let spot = picker::random_spot(&surfaces, &mut Still(&surfaces), Some(Kind::Floor), CAMERA, &mut draw, &mut |_, _| {})
                .expect("a floor spot");
            Ok(planner::plan(factory_floor(), planner::Context { params, ..context(&surfaces, spot) }, &mut draw).ok())
        });
        redraws += u32::from(attempt.unwrap().expect("a plan within the candidates").1) - 1;
    }
    assert_eq!(redraws, 0, "second candidates in 1,000 seeded attempts");
}

#[test]
fn every_position_of_every_producer_is_a_target_the_solver_takes() {
    let mut report = String::new();
    for (name, a, b) in [("flat", v(85., 0.), v(-85., 0.)), ("sloped", v(0., 0.), v(100., 20.))] {
        let surfaces = block(a, b);
        let mut off = [0u32; 4];
        for unit in 0..=65535u32 {
            let spot = drawn(&surfaces, unit);
            let cursor = chosen(&surfaces, add(spot.point(), v(0.37, 3.1)));
            off[0] += u32::from(!spot.on_segment());
            off[1] += u32::from(!solved(&surfaces, spot, 20));
            off[2] += u32::from(!solved(&surfaces, cursor, 20));
            off[3] += u32::from(!solved(&surfaces, restored(&surfaces, cursor), 20));
        }
        if off != [0; 4] {
            report.push_str(&format!(
                "\n{name}, of 65536 positions: drawn and not on its line {}; drawn and without a start {}; \
                 chosen and refused {}; restored and refused {}", off[0], off[1], off[2], off[3]));
        }
    }
    assert!(report.is_empty(), "{report}");
}

/// The two draws of the design review. Snapping the drawn point once would not help: the next snap moves it again.
#[test]
fn the_two_reviewed_draws_have_a_start_and_one_snap_is_not_why() {
    for (a, b, unit) in [(v(85., 0.), v(-85., 0.), 18296u32), (v(0., 0.), v(100., 20.), 3)] {
        let surfaces = block(a, b);
        let line = surfaces.lines().next().unwrap();
        let low = END_MARGIN / line.length();
        let high = 1.0 - low;
        let raw = add(a, scale(sub(b, a), low + (high - low) * unit as f32 / 65535.0));
        let once = line.snap(raw).unwrap();
        assert_ne!(line.snap(once), Some(once), "unit {unit}: a snapped point is not its own snap");
        let spot = drawn(&surfaces, unit);
        assert_eq!(bits(spot.point()), bits(raw), "unit {unit}: the spot is where the picker drew it");
        assert!(spot.on_segment() && solved(&surfaces, spot, 20), "unit {unit}: the drawn spot is refused");
    }
}

/// On its segment means a parameter from 0 to 1 on a line with a length; nothing compares positions.
#[test]
fn a_target_off_its_segment_is_refused() {
    let surfaces = block(v(85., 0.), v(-85., 0.));
    let line = surfaces.lines().next().unwrap();
    for t in [0.0, 0.5, 1.0] {
        assert!(picker::Spot::at(line, t).on_segment(), "{t}");
    }
    assert!(solved(&surfaces, picker::Spot::at(line, 0.5), 20));
    for t in [-0.001, 1.001, f32::NAN, f32::INFINITY] {
        let spot = picker::Spot::at(line, t);
        assert!(!spot.on_segment() && !solved(&surfaces, spot, 20), "{t}");
    }
    assert!(!picker::Spot::at(Line { ends: [line.ends[0], line.ends[0]], ..line }, 0.5).on_segment(), "a line of no length");
}

#[test]
fn off_segment_targets_never_replace_the_wizard_pick_or_get_a_manual_start() {
    let surfaces = block(v(85., 0.), v(-85., 0.));
    let line = surfaces.lines().next().unwrap();
    let valid = picker::Spot::at(line, 0.25);
    let outside = picker::Spot::at(line, -0.125); // Finite and inside the blast box, but beyond the floor.
    let mut config = factory_floor();
    let mut wizard = wizard::Wizard::begin(config, false);
    assert!(wizard.set_spot(valid));
    let wizard_accepted = wizard.set_spot(outside);
    let planned = [Mode::Set, Mode::Random].map(|mode| {
        config.start = Numeric { mode, lo: 20., hi: 20. };
        let plan = |spot| planner::plan(config, context(&surfaces, spot), &mut |n| n / 4).is_ok();
        assert!(plan(valid), "the control has a start above the floor");
        plan(outside)
    });
    assert_eq!((wizard_accepted, wizard.selected, planned), (false, Some(valid), [false, false]));
}

/// A remembered spot comes back on a stage whose lines were read again, also with their ends the other way round: the
/// same world point bit for bit, which is the retained launch's target.
#[test]
fn a_remembered_spot_returns_to_its_exact_point_on_a_reversed_line() {
    let surfaces = block(v(97.3, 0.), v(-61.7, 0.));
    let spot = drawn(&surfaces, 12345);
    let plan = planner::plan(factory_floor(), context(&surfaces, spot), &mut |n| n / 2).expect("a plan");
    assert_eq!(bits(plan.setup.launch.target), bits(spot.point()));
    let remembered = spot.remember().expect("a remembered spot");
    let same = picker::Spot::recover(remembered, &surfaces).expect("the same lines");
    assert_eq!((same.line, bits(same.point())), (spot.line, bits(spot.point())));

    let again = |change: &dyn Fn(Line) -> Line| {
        let mut fresh = Surfaces::EMPTY;
        for l in surfaces.lines() {
            assert!(fresh.push(change(l)));
        }
        fresh.state = surface::Walk::Closed;
        fresh
    };
    let reversed = again(&|l| Line { id: l.id + 10, ends: [l.ends[1], l.ends[0]], ..l });
    let back = picker::Spot::recover(remembered, &reversed).expect("the reversed lines");
    // The fresh line as the stage reads it now, so the spot is current on it.
    assert_eq!((back.line.id, back.line.ends), (spot.line.id + 10, [spot.line.ends[1], spot.line.ends[0]]));
    assert!(back.current(&mut Still(&reversed)));
    assert_eq!(bits(back.point()), bits(plan.setup.launch.target), "the retained launch still aims at the spot");
    assert!(back.on_segment() && solved(&reversed, back, 20), "the solver takes the remembered spot");
    // Remembered again from the reversed stage, it returns to the first stage's point.
    let home = picker::Spot::recover(back.remember().expect("remembered again"), &surfaces).expect("the first lines");
    assert_eq!((home.line, bits(home.point())), (spot.line, bits(spot.point())));

    let moved = again(&|l| Line { ends: [add(l.ends[0], v(0., 1.)), l.ends[1]], ..l });
    assert_eq!(picker::Spot::recover(remembered, &moved), Err(surface::RecoverError::MissingSurface));
}

/// An attempt draws up to eight candidates in all, in one call; anything else that is missing ends it at once.
#[test]
fn an_attempt_draws_eight_candidates_in_all() {
    assert_eq!(planner::CANDIDATES, 8);
    let mut calls = 0;
    let third: Result<_, ()> = planner::draw_attempt(|| {
        calls += 1;
        Ok((calls == 3).then_some("a plan"))
    });
    assert_eq!((third, calls), (Ok(Some(("a plan", 3))), 3));
    let mut calls = 0;
    let none: Result<Option<((), u8)>, ()> = planner::draw_attempt(|| {
        calls += 1;
        Ok(None)
    });
    assert_eq!((none, calls), (Ok(None), 8));
    let mut calls = 0;
    let ended: Result<Option<((), u8)>, &str> = planner::draw_attempt(|| {
        calls += 1;
        Err("the stage is being read again")
    });
    assert_eq!((ended, calls), (Err("the stage is being read again"), 1));

    // The player's own values that no draw can meet: a fixed angle straight up, at a floor.
    let surfaces = block(v(85., 0.), v(-85., 0.));
    let mut config = factory_floor();
    config.angle = Numeric { mode: Mode::Set, lo: 90., hi: 90. };
    let mut calls = 0u32;
    let stop: Result<_, ()> = planner::draw_attempt(|| {
        calls += 1;
        Ok(planner::plan(config, context(&surfaces, drawn(&surfaces, calls * 5000)), &mut |n| n / 2).ok())
    });
    assert!(matches!(stop, Ok(None)) && calls == 8, "eight different spots, none with a start");
    let why = planner::plan(config, context(&surfaces, drawn(&surfaces, 5000)), &mut |n| n / 2).err();
    assert_eq!(why, Some(planner::Refused::Aim));
}

/// One draw without a start is drawn again, and the stop after the last candidate is a listed restriction.
#[test]
fn an_unsolvable_draw_is_drawn_again_and_the_stop_is_a_listed_restriction() {
    let feature = include_str!("../../runtime/features/tech.rs");
    assert!(feature.contains("planner::draw_attempt("), "one draw decides the session");
    assert!(!feature.contains("Approximate aim has no solution"));
    let stop = "No launch fits these Tech practice values";
    assert_eq!(feature.matches(stop).count(), 1, "one text for the stop");
    assert_eq!(feature.matches("// restriction: tech-no-launch-fits").count(), 2, "both places that show it");
    let row = format!("| `tech-no-launch-fits` | \"{stop}\" |");
    assert!(include_str!("../../../docs/RESTRICTIONS.md").contains(&row));
}
