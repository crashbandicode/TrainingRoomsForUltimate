//! Candidate aim solver over tech_launch's explicitly provisional zero-stick flight.
//! This is host-testable planning, not a native integration/DI parity claim.
use super::super::tech_launch::{self as flight, FlightParams};
use super::{
    add, finite, scale, sub,
    picker::Spot,
    surface::{self, Kind, Line, Surfaces, Walk},
    Vec2,
};

#[derive(Clone, Copy, Debug)]
pub struct Box2 {
    pub min: Vec2,
    pub max: Vec2,
}
impl Box2 {
    pub fn contains(self, p: Vec2) -> bool {
        finite(p) && p.x > self.min.x && p.x < self.max.x && p.y > self.min.y && p.y < self.max.y
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Solution {
    pub start: Vec2,
    pub angle: f32,
    pub frames: u16,
}

/// A complete closed outline is needed for the inside-solid test. Disconnected
/// passable platforms do not contribute to the odd/even solid boundary.
pub fn inside(surfaces: &Surfaces, p: Vec2) -> bool {
    let mut crossings = 0;
    for l in surfaces.lines().filter(|l| !l.passable) {
        let [a, b] = l.ends;
        let d = sub(b, a);
        if let Some(q) = l.snap(p) {
            if q == p {
                return true;
            }
        }
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * d.x / d.y {
            crossings += 1;
        }
    }
    crossings % 2 == 1
}

pub(super) fn contact_frames(knockback: f32) -> core::ops::RangeInclusive<f32> {
    10.0..=(knockback * flight::DAMAGE_FRAME_MUL - 2.0).min(60.0)
}

/// Whether the game takes a touch of `line` by a flight in this state as a contact: any floor; in hitstun a wall met
/// with horizontal damage speed into it (stock tests signed horizontal speed, not speed along the normal) and a
/// ceiling met rising.
fn takes(line: Line, end: flight::Flight, hitstun: bool) -> bool {
    match line.kind() {
        Some(Kind::Floor) => true,
        Some(Kind::Wall) => hitstun && end.damage_velocity.x * line.normal.x < 0.0,
        Some(Kind::Ceiling) => hitstun && end.damage_velocity.y > end.gravity_speed,
        None => false,
    }
}

pub struct Aim<'a> {
    pub surfaces: &'a Surfaces,
    pub spot: Spot,
    pub knockback: f32,
    pub params: FlightParams,
    pub blast: Box2,
    pub view: Option<Box2>,
}
impl Aim<'_> {
    /// Translate the path to the target, then reject an early collision, a solid
    /// start, an out-of-bounds path or a contact outside the timed passive regime.
    pub fn at(&self, angle: f32, frames: u16) -> Option<Solution> {
        let (line, target) = (self.spot.line, self.spot.point());
        line.kind()?;
        if self.surfaces.state != Walk::Closed
            || !self.spot.on_segment()
            || !contact_frames(self.knockback).contains(&f32::from(frames))
        {
            return None;
        }
        let velocity = flight::launch_velocity(self.knockback, flight::direction_degrees(angle)?)?;
        let start = flight::solve_start(
            target,
            velocity,
            self.params,
            frames,
            self.knockback * flight::DAMAGE_FRAME_MUL,
            line.normal,
            5.7,
        )?;
        let in_bounds = |p| self.blast.contains(p) && self.view.is_none_or(|v| v.contains(p));
        if !in_bounds(start) || inside(self.surfaces, start) {
            return None;
        }
        let mut previous = start;
        for f in 1..=frames {
            let p = if f == frames {
                target
            } else {
                add(start, flight::flight(velocity, self.params, f)?.displacement)
            };
            // The last segment ends at the chosen point on this line by construction.
            if !in_bounds(p) || self.surfaces.lines().any(|l| {
                (f != frames || l.id != line.id) && surface::crossing(previous, p, l).is_some()
            }) {
                return None;
            }
            previous = p;
        }
        let end = flight::flight(velocity, self.params, frames)?;
        if !takes(line, end, true) {
            return None;
        }
        Some(Solution {
            start,
            angle,
            frames,
        })
    }

    /// A launch flown forward from `start` with neutral input: the frame of its first touch the session grades, and the
    /// frames flown. Any face `takes` in hitstun, then floors until the session abandons the attempt.
    pub fn contact(&self, start: Vec2, angle: f32) -> (Option<u16>, u16) {
        let reaction = self.knockback * flight::DAMAGE_FRAME_MUL;
        let Some(mut path) = flight::direction_degrees(angle)
            .and_then(|direction| flight::launch_velocity(self.knockback, direction))
            .and_then(|velocity| flight::Path::new(velocity, self.params))
        else {
            return (None, 0);
        };
        let mut previous = start;
        let mut flown = 0;
        for f in 1..=flight::MAX_FRAMES {
            if f32::from(f) >= reaction + f32::from(super::session::ABANDON) {
                break;
            }
            flown = f;
            let Some(end) = path.step() else { return (None, f) };
            let p = add(start, end.displacement);
            if !self.blast.contains(p) {
                return (None, f);
            }
            if let Some((_, line)) = self.surfaces.lines()
                .filter_map(|line| surface::crossing(previous, p, line).map(|fraction| (fraction, line)))
                .min_by(|a, b| a.0.total_cmp(&b.0))
            {
                return (takes(line, end, f32::from(f) < reaction).then_some(f), f);
            }
            previous = p;
        }
        (None, flown)
    }

    /// Prefer the normal's opposite direction, then every five-degree menu angle.
    /// Fixed angle/time candidates are never silently changed by this method.
    pub fn solve(&self, angle: Option<f32>, frames: u16) -> Option<Solution> {
        if let Some(a) = angle {
            return self.at(a, frames);
        }
        let inward = scale(self.spot.line.normal, -1.0);
        let preferred = inward.y.atan2(inward.x).to_degrees().rem_euclid(360.0);
        self.at(preferred, frames)
            .or_else(|| (0..72).find_map(|i| self.at(i as f32 * 5.0, frames)))
    }
}

/// BTT placing camera's published rectangle, extended to the camera range's
/// bottom for underside editing; BTT's ground-minus-ten bound excludes it.
pub fn editor_bounds(range: [f32; 4]) -> Box2 {
    Box2 {
        min: Vec2 {
            x: range[0],
            y: range[3],
        },
        max: Vec2 {
            x: range[1],
            y: range[2],
        },
    }
}
