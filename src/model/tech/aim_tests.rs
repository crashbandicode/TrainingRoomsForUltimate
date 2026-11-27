//! Tech practice, Auto start: how often a factory draw on a flat floor has no solution, and why (the docs pass,
//! 2026-11-27: the drill stopped after three launches with "Approximate aim has no solution for these values").
use super::*;
use crate::model::tech_launch::{self as flight, FlightParams};
use std::collections::BTreeMap;
use surface::{Kind, Line, Source, Surfaces};

fn v(x: f32, y: f32) -> Vec2 {
    Vec2 { x, y }
}

/// One solid block with its floor on top: a flat stage's shape. `half` is the floor's half width.
fn flat(half: f32) -> Surfaces {
    let vertices = [v(half, 0.), v(-half, 0.), v(-half, -60.), v(half, -60.)];
    let normals = [v(0., 1.), v(-1., 0.), v(0., -1.), v(1., 0.)];
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

/// `Aim::at`'s own steps, in its order, naming the first one that refuses.
fn why(aim: &solver::Aim<'_>, angle: f32, frames: u16) -> &'static str {
    if aim.line.snap(aim.target) != Some(aim.target) {
        return "the drawn point is not exactly on its line";
    }
    if !solver::contact_frames(aim.knockback).contains(&f32::from(frames)) {
        return "frames outside the contact window";
    }
    let Some(velocity) = flight::direction_degrees(angle).and_then(|d| flight::launch_velocity(aim.knockback, d)) else {
        return "no launch velocity";
    };
    let Some(start) = flight::solve_start(aim.target, velocity, aim.params, frames,
        aim.knockback * flight::DAMAGE_FRAME_MUL, aim.line.normal, 5.7) else {
        return "no start for this flight";
    };
    if solver::inside(aim.surfaces, start) {
        return "the start is inside the stage";
    }
    let mut previous = start;
    for f in 0..=frames {
        let Some(step) = flight::flight(velocity, aim.params, f) else { return "no flight" };
        let p = add(start, step.displacement);
        if !aim.blast.contains(p) {
            return "the path leaves the blast box";
        }
        if aim.view.is_some_and(|b| !b.contains(p)) {
            return "the path leaves the view";
        }
        if f > 0 {
            for l in aim.surfaces.lines() {
                if let Some(t) = surface::crossing(previous, p, l) {
                    if l.id != aim.line.id {
                        return "the path crosses another line";
                    }
                    if f < frames {
                        return "the path reaches its line early";
                    }
                    if t < 1.0 {
                        return "the last step ends past its line";
                    }
                }
            }
        }
        previous = p;
    }
    "solved"
}

/// Factory settings with Surface = Floor on a flat stage. Every draw the menu can make must have a start.
#[test]
fn factory_draws_on_a_flat_floor_have_a_start() {
    let mut config = crate::model::tech_settings::read(&crate::model::settings::Settings::factory().menu);
    config.surface = 1;
    let blast = solver::editor_bounds([-240., 240., 192., -140.]);
    let mut report = String::new();
    let mut unsolved = 0u32;
    for (fighter, params) in [
        ("gravity 0.087, fall cap 1.5", FlightParams { brake: 0.051, gravity: 0.087, fall_speed_cap: 1.5 }),
        ("gravity 0.085, fall cap 1.85", FlightParams { brake: 0.051, gravity: 0.085, fall_speed_cap: 1.85 }),
    ] {
        let surfaces = flat(85.);
        let line = surfaces.lines().find(|l| l.kind() == Some(Kind::Floor)).unwrap();
        // Every frame count the factory time range can draw at 100 KB, at points the picker's own formula gives.
        let (mut cells, mut none) = (0u32, 0u32);
        let mut preferred: BTreeMap<&str, u32> = BTreeMap::new();
        let mut every: BTreeMap<&str, u32> = BTreeMap::new();
        let d = sub(line.ends[1], line.ends[0]);
        let low = 5.0 / line.length();
        for frames in 20..=38u16 {
            for unit in (0..=65535u32).step_by(97) {
                let target = add(line.ends[0], scale(d, low + (1.0 - 2.0 * low) * unit as f32 / 65535.0));
                let aim = solver::Aim { surfaces: &surfaces, line, target, knockback: 100., params, blast, view: None };
                cells += 1;
                if aim.solve(None, frames).is_none() {
                    none += 1;
                    *preferred.entry(why(&aim, 270., frames)).or_default() += 1;
                    for i in 0..72 {
                        *every.entry(why(&aim, i as f32 * 5., frames)).or_default() += 1;
                    }
                }
            }
        }
        // As played: 1,000 attempts, each a spot from the picker and a plan from the planner, one seeded stream.
        let mut seed = 0x2545_f491u32;
        let mut draw = |n: u32| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            ((u64::from(seed >> 8) * u64::from(n)) >> 24) as u32
        };
        let (mut failed, mut first) = (0u32, None);
        for attempt in 1..=1000u32 {
            let spot = picker::random_spot(&surfaces, &mut Still(&surfaces), Some(Kind::Floor),
                [-1000., 1000., 1000., -1000.], &mut draw, &mut |_, _| {}).expect("a floor spot");
            let context = planner::Context { surfaces: &surfaces, spot, params, blast, view: None, damage_multiplier: Some(1.0) };
            if planner::plan(config, context, &mut draw).is_none() {
                failed += 1;
                first.get_or_insert(attempt);
            }
        }
        unsolved += none + failed;
        report.push_str(&format!(
            "\n{fighter}: grid {none} of {cells} without a start; at the inward angle {preferred:?}; over all 72 angles {every:?}; \
             as played {failed} of 1000 attempts without a plan, the first at attempt {first:?}"));
    }
    assert_eq!(unsolved, 0, "{report}");
}

/// Measurement for the design: is the line's snap a fixed point on the points the producers hand to the solver?
#[test]
fn snap_outputs_measured_over_every_picker_position() {
    let mut report = String::new();
    for (name, a, b) in [("flat", v(85., 0.), v(-85., 0.)), ("sloped", v(-37.3, 12.9), v(61.7, -8.45))] {
        let line = Line { id: 1, ends: [a, b], normal: v(0., 1.), floor: true, wall: false, passable: false };
        let d = sub(b, a);
        let low = 5.0 / line.length();
        let (mut lerp_off, mut snap_off, mut cursor_off, mut twice_off) = (0u32, 0u32, 0u32, 0u32);
        for unit in 0..=65535u32 {
            let p = add(a, scale(d, low + (1.0 - 2.0 * low) * unit as f32 / 65535.0));
            let q = line.snap(p).unwrap();
            if q != p { lerp_off += 1; }
            if line.snap(q) != Some(q) { snap_off += 1; }
            if line.snap(q).and_then(|r| line.snap(r).map(|s| s != r)) == Some(true) { twice_off += 1; }
            // A cursor above the line, as the wizard's picker gives to `nearest`.
            let c = line.snap(add(p, v(0.37, 3.1))).unwrap();
            if line.snap(c) != Some(c) { cursor_off += 1; }
        }
        report.push_str(&format!(
            "\n{name}: of 65536 positions: the drawn point is not its own snap {lerp_off}; a snap is not its own snap {snap_off}; \
             a second snap still moves {twice_off}; a cursor's snap is not its own snap {cursor_off}"));
    }
    panic!("{report}");
}