//! Bounded Quick Attack geometry for Pikachu's Skull Bash crossing.

use super::{Point, Segment};

#[derive(Clone, Copy, Debug)]
pub struct ZipPlan {
    pub first: Point,
    pub second: Option<Point>,
}

// The traced first zip and vl.prc second-zip length, minimum turn and motion limits.
// See phase23/edge-juggle/reference-crossings/MODEL.md §4 for the candidate margins.
const FIRST: f32 = 55.0;
const SECOND: f32 = 49.5;
const MIN_TURN: f32 = 35.0;
const SLACK: f32 = 4.0;
const GRAB_RADIUS: f32 = 5.0;
const HALF_BODY: f32 = 1.5;

fn dot(a: Point, b: Point) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

fn difference(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}

fn advance(from: Point, direction: Point, distance: f32) -> Point {
    [from[0] + distance * direction[0], from[1] + distance * direction[1]]
}

fn clear(outline: &[Segment], path: Segment, bottom: f32, height: f32) -> bool {
    path.into_iter().all(|p| p[1] > bottom + SLACK)
        && [-HALF_BODY, 0.0, HALF_BODY].into_iter().all(|offset| {
            let path = path.map(|p| [p[0] + offset, p[1]]);
            !outline.iter().any(|&line| super::blocked(path, line, height + 1.0))
        })
}

fn second_zip(
    outline: &[Segment],
    from: Point,
    target: Point,
    first: Point,
    bottom: f32,
    height: f32,
) -> Option<(f32, Point)> {
    let [dx, dy] = difference(target, from);
    let distance = dx.hypot(dy);
    if !(distance > 0.0 && distance <= SECOND - SLACK) {
        return None;
    }
    let direction = [dx / distance, dy / distance];
    let turn = dot(first, direction).clamp(-1.0, 1.0).acos().to_degrees();
    let margin = (SECOND - distance).min(turn - MIN_TURN);
    if margin < SLACK || !clear(outline, [from, target], bottom, height) {
        return None;
    }
    Some((margin, direction))
}

/// Aim one or two zips from the actual apex. The second zip must have spare travel
/// and a different angle, and neither segment may carry the body through the stage.
pub fn quick_attack(
    outline: &[Segment],
    from: Point,
    target: Point,
    bottom: f32,
    height: f32,
) -> Option<ZipPlan> {
    if outline.is_empty()
        || !from.into_iter().chain(target).chain([bottom, height]).all(f32::is_finite)
        || !outline.iter().flatten().flatten().all(|v| v.is_finite())
        || height <= 0.0
    {
        return None;
    }
    let mut best = None;
    let mut room = SLACK;
    let toward = difference(target, from);
    for step in 0..72 {
        let angle = (-90.0 + 5.0 * step as f32).to_radians();
        let (sine, cosine) = angle.sin_cos();
        let first = [cosine, sine];
        let projection = dot(toward, first);
        if projection > 0.0 && projection <= FIRST - room {
            let catch = advance(from, first, projection);
            let [dx, dy] = difference(catch, target);
            let margin = (GRAB_RADIUS - dx.hypot(dy)).min(FIRST - projection);
            if margin >= room && clear(outline, [from, catch], bottom, height) {
                best = Some(ZipPlan { first, second: None });
                room = margin;
            }
        }
        let end = advance(from, first, FIRST);
        if let Some((margin, second)) = second_zip(outline, end, target, first, bottom, height) {
            if margin >= room && clear(outline, [from, end], bottom, height) {
                best = Some(ZipPlan { first, second: Some(second) });
                room = margin;
            }
        }
    }
    best
}

#[cfg(test)]
lib_unit_tests! {
    #[path = "../../../../tests/common/edge_juggle_pikachu.rs"]
    mod tests;
}
