//! Bounded Quick Attack geometry for Pikachu's Skull Bash crossing.

use super::{niche::{Sample, Sequence}, Correction, Outline, Point, Segment};

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
    super::aerial_clear(outline, path, bottom + SLACK, height + 1.0, HALF_BODY)
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkullBashPath {
    pub direction: f32,
    pub launch_y: f32,
    pub near: Point,
    pub far: Point,
    pub height: f32,
}

// The 201 candidate cells from reference-crossings/gen_regions.py, P-SB on the common ring.
// The same stage layout is mirrored at runtime. Lylat needs a separate geometry and table.
const SKULL_BASH_RING: [u32; 44] = [
    0x0000, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc,
    0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc,
    0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc, 0x03fc,
    0x01fc, 0x01fc, 0x001c, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000,
];

fn in_region(outside: f32, height: f32) -> bool {
    if !outside.is_finite() || !height.is_finite() || outside <= 0.0 || height > 40.0 {
        return false;
    }
    let column = (outside / 4.0) as u32;
    let row = ((40.0 - height) / 4.0) as usize;
    SKULL_BASH_RING
        .get(row)
        .zip(1u32.checked_shl(column))
        .is_some_and(|(&mask, cell)| mask & cell != 0)
}

/// Start outside the lip in a cell baked for the live normal-form floor and blast line.
pub fn skull_bash_start(
    stage: (i32, u32),
    outline: &[Segment],
    [left, right]: [Point; 2],
    [x, y]: Point,
    bottom: f32,
    height: f32,
) -> Option<SkullBashPath> {
    const MODEL_HEIGHT: f32 = 10.0;
    if !matches!(stage, (0 | 3 | 311, 0))
        || !(height > 0.0 && height <= MODEL_HEIGHT)
        || !left.into_iter().chain(right).chain([x, y, bottom]).all(f32::is_finite)
        || !outline.iter().flatten().flatten().all(|v| v.is_finite())
    {
        return None;
    }
    let [near, far] = if x < left[0] { [left, right] } else { [right, left] };
    let direction = (far[0] - near[0]).signum();
    let outside = (near[0] - x) * direction;
    let y = y - near[1];
    let underside = outline.iter().flatten().map(|point| point[1]).reduce(f32::min)?;
    if ((right[0] - left[0]) * 0.5 - 79.99).abs() > 0.5
        || (right[1] - left[1]).abs() > 0.5
        || (underside - near[1] + 40.1).abs() > 1.0
        || (bottom - near[1] + 140.0).abs() > 1.0
    {
        return None;
    }
    if !in_region(outside, y) {
        return None;
    }
    Some(SkullBashPath {
        direction,
        // edge-ref-pika-1: drop 3.64 before flight, then rise 1.82 above
        // its start during flight. Leave two units between the body and the roof.
        launch_y: underside - MODEL_HEIGHT - 2.0 + 3.64 - 1.82,
        near: [near[0] - direction * 1.7, near[1] - 9.9],
        far: [far[0] + direction * 1.7, far[1] - 9.9],
        height: MODEL_HEIGHT,
    })
}

pub struct SkullBashFlight {
    path: SkullBashPath,
    outline: Outline,
    bottom: f32,
    bash_committed: bool,
    jump_accepted: bool,
    zips: Option<ZipPlan>,
}

impl SkullBashFlight {
    pub fn new(path: SkullBashPath, outline: Outline, bottom: f32) -> Self {
        Self {
            path,
            outline,
            bottom,
            bash_committed: false,
            jump_accepted: false,
            zips: None,
        }
    }

    pub fn step(
        &mut self, sequence: &mut Sequence, mut sample: Sample, position: Point, vy: f32,
    ) -> Option<Correction> {
        use super::niche::{self, Motion, Move};
        let movement = sequence.action()?;
        let outline = self.outline.segments();
        let path = self.path;
        if !sample.valid
            || sample.motion == Motion::Interrupted
            || !position.into_iter().chain([vy]).all(f32::is_finite)
            || !clear(outline, [position; 2], self.bottom, path.height)
        {
            return None;
        }
        if movement == Move::Side && !self.bash_committed {
            let ledge_x = path.near[0] + path.direction * 1.7;
            let ledge_y = path.near[1] + 9.9;
            let outside = (ledge_x - position[0]) * path.direction;
            if sample.motion != Motion::Free || !in_region(outside, position[1] - ledge_y) {
                return None;
            }
            if position[1] > path.launch_y {
                // Release before the side-special frame. Before this point, the full fall is reversible.
                return Some(niche::pad([0.0, -1.0], 0));
            }
            if !sample.can_start {
                return None;
            }
            self.bash_committed = true;
        }
        if movement == Move::Jump {
            if sample.motion == Motion::Travel {
                self.jump_accepted = true;
                if vy <= 0.0 {
                    sample.motion = Motion::Free;
                }
            } else if !self.jump_accepted && !sample.can_start {
                return None;
            }
        }
        if movement == Move::QuickAttack && self.zips.is_none() {
            if sample.motion != Motion::Free || !sample.can_start {
                return None;
            }
            let solve = |target| quick_attack(outline, position, target, self.bottom, path.height);
            self.zips = Some(solve(path.far).or_else(|| solve(path.near))?);
        }
        sample.direction = path.direction;
        sample.at_exit = true;
        sample.can_enter = true;
        let mut input = sequence.step(sample)?;
        if movement == Move::QuickAttack {
            let zips = self.zips?;
            input.stick = match sample.motion {
                Motion::Starting | Motion::Travel => Some(zips.first),
                Motion::AimAgain | Motion::Finish => Some(zips.second.unwrap_or([0.0; 2])),
                _ => input.stick,
            };
        }
        Some(input)
    }
}

#[cfg(test)]
lib_unit_tests! {
    #[path = "../../../../tests/common/edge_juggle_pikachu.rs"]
    mod tests;
}
