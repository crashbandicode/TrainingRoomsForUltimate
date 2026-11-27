//! Candidate aim solver over tech_launch's explicitly provisional zero-stick flight.
//! This is host-testable planning, not a native integration/DI parity claim.
use super::super::tech_launch::{self as flight, FlightParams};
use super::{
    add, finite, scale, sub,
    picker::Spot,
    surface::{self, Kind, Surfaces, Walk},
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
        if self.surfaces.state != Walk::Closed
            || line.kind().is_none()
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
        if inside(self.surfaces, start) {
            return None;
        }
        let mut previous = start;
        for f in 0..=frames {
            let p = add(
                start,
                flight::flight(velocity, self.params, f)?.displacement,
            );
            if !self.blast.contains(p) || self.view.is_some_and(|v| !v.contains(p)) {
                return None;
            }
            if f > 0 {
                for l in self.surfaces.lines() {
                    if let Some(t) = surface::crossing(previous, p, l) {
                        // Only the selected line on the final segment may be hit.
                        if l.id != line.id || f < frames || t < 1.0 {
                            return None;
                        }
                    }
                }
            }
            previous = p;
        }
        let end = flight::flight(velocity, self.params, frames)?;
        if line.kind() == Some(Kind::Ceiling)
            && end.damage_velocity.y - end.gravity_speed <= 0.0
        {
            return None;
        }
        Some(Solution {
            start,
            angle,
            frames,
        })
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
