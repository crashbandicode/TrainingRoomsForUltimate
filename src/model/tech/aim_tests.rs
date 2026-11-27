//! Tech practice, Auto start: every target a producer hands to the solver is a target the solver takes, and one draw
//! without a start does not end the session (the docs pass, 2026-11-27: with factory settings the drill stopped after
//! three launches with "Approximate aim has no solution for these values").
use super::*;
use crate::model::tech_launch::FlightParams;
use surface::{Kind, Line, RememberedSpot, Source, Surfaces, END_MARGIN};

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
    let (line, point) = surfaces.nearest(cursor, Some(Kind::Floor)).expect("a floor under the cursor");
    picker::Spot { line, point }
}
/// The Chosen point after the stage loaded again (`Picker::enter`, then `Picker::restore`).
fn restored(surfaces: &Surfaces, spot: picker::Spot) -> picker::Spot {
    let mut picker = picker::Picker::EMPTY;
    picker.enter(7, spot.point);
    picker.selected = Some(spot);
    picker.enter(7, spot.point);
    assert!(picker.restore(surfaces));
    picker.selected.expect("a restored spot")
}
/// The solver's own rule for "this target is on its line".
fn on_line(spot: picker::Spot) -> bool {
    spot.line.snap(spot.point) == Some(spot.point)
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
    solver::Aim {
        surfaces,
        line: spot.line,
        target: spot.point,
        knockback: 100.,
        params: context.params,
        blast: context.blast,
        view: None,
    }
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
fn factory_draws_on_a_flat_floor_never_lack_a_start() {
    let surfaces = block(v(85., 0.), v(-85., 0.));
    // Every frame count the factory time range can draw, over the floor.
    let grid = (20..=38u16)
        .flat_map(|frames| (0..=65535u32).step_by(97).map(move |unit| (frames, unit)))
        .filter(|(frames, unit)| !solved(&surfaces, drawn(&surfaces, *unit), *frames))
        .count();
    // As played: 1,000 attempts, each a spot from the picker and a plan from the planner, on one seeded stream.
    let mut seed = 0x2545_f491u32;
    let mut draw = |n: u32| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        ((u64::from(seed >> 8) * u64::from(n)) >> 24) as u32
    };
    let mut refused = 0;
    for _ in 0..1000 {
        let spot = picker::random_spot(&surfaces, &mut Still(&surfaces), Some(Kind::Floor), CAMERA, &mut draw, &mut |_, _| {})
            .expect("a floor spot");
        if planner::plan(factory_floor(), context(&surfaces, spot), &mut draw).is_none() {
            refused += 1;
        }
    }
    assert_eq!((grid, refused), (0, 0), "draws without a start: of the 12,844 of the grid, of the 1,000 attempts");
}

#[test]
fn every_position_of_every_producer_is_a_target_the_solver_takes() {
    let mut report = String::new();
    for (name, a, b) in [("flat", v(85., 0.), v(-85., 0.)), ("sloped", v(0., 0.), v(100., 20.))] {
        let surfaces = block(a, b);
        let mut off = [0u32; 4];
        for unit in 0..=65535u32 {
            let spot = drawn(&surfaces, unit);
            let cursor = chosen(&surfaces, add(spot.point, v(0.37, 3.1)));
            off[0] += u32::from(!on_line(spot));
            off[1] += u32::from(!solved(&surfaces, spot, 20));
            off[2] += u32::from(!on_line(cursor) || unit % 16 == 0 && !solved(&surfaces, cursor, 20));
            off[3] += u32::from(!on_line(restored(&surfaces, cursor)));
        }
        if off != [0; 4] {
            report.push_str(&format!(
                "\n{name}, of 65536 positions: drawn and not on its line {}; drawn and without a start {}; \
                 chosen and refused {}; restored and not on its line {}", off[0], off[1], off[2], off[3]));
        }
    }
    assert!(report.is_empty(), "{report}");
}

/// The two draws of the design review. Snapping the drawn point once does not help: the next snap moves it again.
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
        assert!(on_line(spot) && solved(&surfaces, spot, 20), "unit {unit}: the drawn spot is refused");
    }
}

/// A remembered spot comes back on a stage whose lines were read again, also with their ends the other way round: the
/// same world point bit for bit, which is the retained launch's target.
#[test]
fn a_remembered_spot_returns_to_its_exact_point_on_a_reversed_line() {
    let surfaces = block(v(97.3, 0.), v(-61.7, 0.));
    let spot = drawn(&surfaces, 12345);
    let plan = planner::plan(factory_floor(), context(&surfaces, spot), &mut |n| n / 2).expect("a plan");
    assert_eq!(bits(plan.setup.launch.target), bits(spot.point));
    let remembered = RememberedSpot::capture(spot.line, spot.point).expect("a remembered spot");
    let (line, point) = remembered.resolve(&surfaces).expect("the same lines");
    assert_eq!((line, bits(point)), (spot.line, bits(spot.point)));

    let again = |change: &dyn Fn(Line) -> Line| {
        let mut fresh = Surfaces::EMPTY;
        for l in surfaces.lines() {
            assert!(fresh.push(change(l)));
        }
        fresh.state = surface::Walk::Closed;
        fresh
    };
    let reversed = again(&|l| Line { id: l.id + 10, ends: [l.ends[1], l.ends[0]], ..l });
    let (line, point) = remembered.resolve(&reversed).expect("the reversed lines");
    assert_eq!((line.id, line.ends), (spot.line.id + 10, [spot.line.ends[1], spot.line.ends[0]]));
    assert_eq!(bits(point), bits(plan.setup.launch.target), "the retained launch still aims at the spot");
    assert!(solved(&reversed, picker::Spot { line, point }, 20), "the solver takes the remembered spot");

    let moved = again(&|l| Line { ends: [add(l.ends[0], v(0., 1.)), l.ends[1]], ..l });
    assert_eq!(remembered.resolve(&moved), Err(surface::RecoverError::MissingSurface));
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
