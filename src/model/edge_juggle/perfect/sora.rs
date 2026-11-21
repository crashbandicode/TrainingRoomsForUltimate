//! Map the measured Sonic Blade flight to the live far ledge.

use super::{Point, Segment};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BladePath {
    /// Angles relative to the direction of the crossing. A first angle is always level.
    pub angles: [f32; 3],
    pub count: u8,
    pub spare: f32,
}

// Model §5.2: measure each Blade through the first sample of the next
// native status. The first moves level, then carries a further 4.9.
const FIRST_AND_CARRY: f32 = 36.4 + 4.9;
const SECOND_UP: f32 = 30.2;
const THIRD_UP: f32 = 26.3;
const MIN_SPARE: f32 = 4.0;
// The 13.0.5 native underside contact gap is 7.43, plus ~1 unit of clearance.
const BODY_HEIGHT: f32 = 8.4;
const HALF_BODY: f32 = 2.0;

pub fn blade_path(
    outline: &[Segment],
    origin: Point,
    hang: Point,
    direction: f32,
    bottom: f32,
) -> Option<BladePath> {
    if outline.is_empty()
        || direction.abs() != 1.0
        || !origin.into_iter().chain(hang).chain([bottom]).all(f32::is_finite)
        || !outline.iter().flatten().flatten().all(|v| v.is_finite())
    {
        return None;
    }
    let clear = |from, to| {
        super::aerial_clear(outline, [from, to], bottom + MIN_SPARE, BODY_HEIGHT, HALF_BODY)
    };
    let target = [hang[0] + direction, hang[1] - 1.0];
    let final_dash = |from: Point, length: f32| {
        let dx = (target[0] - from[0]) * direction;
        let dy = target[1] - from[1];
        let angle = dy.atan2(dx).to_degrees();
        let spare = length - dx.hypot(dy);
        (spare >= MIN_SPARE && (90.0..=140.0).contains(&angle) && clear(from, target))
            .then_some((angle, spare))
    };

    let mut best: Option<BladePath> = None;
    let mut consider = |path: BladePath| {
        if best.is_none_or(|best| path.spare > best.spare) {
            best = Some(path);
        }
    };
    for (level, sign) in [(0.0, 1.0), (180.0, -1.0)] {
        let after_level = [origin[0] + sign * direction * FIRST_AND_CARRY, origin[1]];
        if !clear(origin, after_level) {
            continue;
        }
        if let Some((angle, spare)) = final_dash(after_level, SECOND_UP) {
            consider(BladePath { angles: [level, angle, 0.0], count: 2, spare: spare + THIRD_UP });
            continue;
        }
        for step in (40i32..=140).step_by(5) {
            let middle = step as f32;
            let (sine, cosine) = middle.to_radians().sin_cos();
            let after_middle = [
                after_level[0] + direction * cosine * SECOND_UP,
                after_level[1] + sine * SECOND_UP,
            ];
            if clear(after_level, after_middle) {
                if let Some((angle, spare)) = final_dash(after_middle, THIRD_UP) {
                    consider(BladePath { angles: [level, middle, angle], count: 3, spare });
                }
            }
        }
    }
    best
}

#[cfg(test)]
lib_unit_tests! {
    #[path = "../../../../tests/common/edge_juggle_sora.rs"]
    mod tests;
}
