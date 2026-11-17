//! Native surface-field decoding and one-ray-per-step discovery schedule.
use super::surface::Line;
use super::{finite, Vec2};

/// Outline bootstrap, separate from the fighter's touched platform.
pub struct Bootstrap {
    rays: Option<Rays>,
    seed: Option<(usize, usize)>,
    remaining: u32,
}
impl Bootstrap {
    pub fn new(range: [f32; 4], touched: Option<Line>) -> Option<Self> {
        let rays = Rays::new(range)?;
        let columns = ((range[1] - range[0]) / 10.0).ceil();
        if !columns.is_finite() || columns < 0.0 {
            return None;
        }
        // One bootstrap and one platform pass, at most MAX_LINES hits plus a
        // miss per column, and at most MAX_LINES outline steps.
        let remaining = (columns as u32)
            .checked_add(1)?
            .checked_mul(2 * (super::surface::MAX_LINES as u32 + 1))?
            .checked_add(super::surface::MAX_LINES as u32)?;
        let seed = touched
            .filter(|line| line.valid() && !line.passable)
            .map(|line| (line.id, 1));
        Some(Self {
            rays: seed.is_none().then_some(rays),
            seed,
            remaining,
        })
    }
    /// Charge actual discovery work, not elapsed time or skipped checks.
    pub fn tick(&mut self) -> bool {
        let Some(left) = self.remaining.checked_sub(1) else {
            return false;
        };
        self.remaining = left;
        true
    }
    pub fn seed(&self) -> Option<(usize, usize)> {
        self.seed
    }
    pub fn current(&self) -> Option<Ray> {
        self.rays.as_ref()?.current()
    }
    pub fn hit(&mut self, line: Line, point: Vec2) -> Result<(), RayError> {
        self.rays.as_mut().ok_or(RayError)?.hit(line, point)?;
        if !line.passable && (line.floor || line.normal.y >= 0.7) {
            self.seed = Some((line.id, 1));
            self.rays = None;
        }
        Ok(())
    }
    pub fn miss(&mut self) {
        if let Some(rays) = self.rays.as_mut() {
            rays.miss();
        }
    }
    pub fn finished(&self) -> bool {
        self.seed.is_some()
            || self
                .rays
                .as_ref()
                .is_some_and(|rays| rays.current().is_none())
    }
}

/// 13.0.5 app::ground::{is_floor,is_wall,is_passable}, byte +0xc5.
pub fn decode(id: usize, ends: [Vec2; 2], normal: Vec2, flags: u8) -> Option<Line> {
    let line = Line {
        id,
        ends,
        normal,
        floor: flags & 8 != 0,
        wall: flags & 3 != 0,
        passable: flags & 0x20 != 0,
    };
    line.valid().then_some(line)
}

/// The neighbour shares the queried endpoint; the walk continues from its other end.
/// Disconnected or collapsed geometry is not made connected with a tolerance.
pub fn far_end(shared: Vec2, next: Line) -> Option<usize> {
    match (next.ends[0] == shared, next.ends[1] == shared) {
        (true, false) => Some(1),
        (false, true) => Some(0),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec2,
    pub delta: Vec2,
}

// main+0x526978..0x526988: accepted ray/line parameters, owned 13.0.5.
// Constants at main+0x4471f68 and main+0x44727d8; no fitted tolerance.
const NATIVE_MIN_T: f32 = -1e-5;
const NATIVE_MAX_T: f32 = 1.00001;

/// One vertical column per 10 world units, with the right boundary included.
/// Continue below the true line by its native back-hit window plus one ULP.
/// The native output is biased toward the origin and is never used as on-line.
pub struct Rays {
    range: [f32; 4],
    x: f32,
    y: f32,
    done: bool,
}
impl Rays {
    pub fn new(range: [f32; 4]) -> Option<Self> {
        let [left, right, top, bottom] = range;
        (range.into_iter().all(f32::is_finite)
            && left <= right
            && bottom < top
            && (top - bottom).is_finite())
        .then_some(Self {
            range,
            x: left,
            y: top,
            done: false,
        })
    }
    pub fn current(&self) -> Option<Ray> {
        (!self.done).then_some(Ray {
            origin: Vec2 {
                x: self.x,
                y: self.y,
            },
            delta: Vec2 {
                x: 0.0,
                y: self.range[3] - self.y,
            },
        })
    }
    /// Native reports origin + delta * min(t,1) * 0.99 (origin for t < 0).
    /// Validate that output, but derive continuation from the returned line.
    pub fn hit(&mut self, line: Line, point: Vec2) -> Result<(), RayError> {
        let Some(below) = self.continuation(line, point) else {
            self.done = true;
            return Err(RayError);
        };
        if below <= self.range[3] {
            self.miss();
        } else {
            self.y = below;
        }
        Ok(())
    }
    fn continuation(&self, line: Line, point: Vec2) -> Option<f32> {
        let ray = self.current()?;
        if !finite(point)
            || point.x != ray.origin.x
            || point.y > self.y
            || point.y < self.range[3]
            || !line.valid()
        {
            return None;
        }
        let [a, b] = line.ends;
        let dx = b.x - a.x;
        if dx == 0.0 || !dx.is_finite() {
            return None;
        }
        let u = (self.x - a.x) / dx;
        if !(NATIVE_MIN_T..=NATIVE_MAX_T).contains(&u) {
            return None;
        }
        let line_y = a.y + (b.y - a.y) * u;
        let length = line_y - self.range[3];
        if !line_y.is_finite() || !length.is_finite() {
            return None;
        }
        // If the accepted line is at/beyond the bottom, this column is finished.
        if length <= 0.0 {
            return Some(self.range[3]);
        }
        // Let w=-NATIVE_MIN_T and L=line_y-bottom. The next ray length is
        // smaller than L, so w*L plus one downward ULP clears its entire
        // negative-t window too, including a hit just behind the old origin.
        let below = (line_y - (-NATIVE_MIN_T * length)).next_down();
        (below.is_finite() && below < self.y).then_some(below)
    }
    pub fn miss(&mut self) {
        let right = self.range[1];
        if self.x == right {
            self.done = true;
        } else {
            let next = (self.x + 10.0).min(right);
            if next <= self.x {
                self.done = true;
            } else {
                self.x = next;
                self.y = self.range[2];
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RayError;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::tech::surface::Kind;
    fn p(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }
    #[test]
    fn native_surface_flags_keep_unclassified_solids() {
        let line = decode(1, [p(0.0, 0.0), p(10.0, -20.0)], p(0.8, 0.4), 0).unwrap();
        assert_eq!(line.kind(), None);
        assert!(decode(1, [p(0.0, 0.0); 2], p(0.0, 1.0), 8).is_none());
        assert!(decode(1, [p(f32::NAN, 0.0), p(1.0, 0.0)], p(0.0, 1.0), 8).is_none());
        let platform = decode(2, [p(-10.0, 3.0), p(10.0, 3.0)], p(0.0, 1.0), 0x28).unwrap();
        assert_eq!(platform.kind(), Some(Kind::Floor));
        let underside = Line {
            normal: p(0.0, -1.0),
            ..platform
        };
        assert_eq!(underside.kind(), None);
    }
    #[test]
    fn native_neighbour_uses_exact_shared_end() {
        let line = decode(1, [p(1.0, 2.0), p(3.0, 4.0)], p(1.0, 0.0), 1).unwrap();
        assert_eq!(far_end(p(1.0, 2.0), line), Some(1));
        assert_eq!(far_end(p(3.0, 4.0), line), Some(0));
        assert_eq!(far_end(p(1.0, 2.0_f32.next_up()), line), None);
    }
    #[test]
    fn platform_rays_visit_stacked_surfaces_and_right_edge() {
        let mut rays = Rays::new([-7.0, 16.0, 30.0, -50.0]).unwrap();
        assert_eq!(rays.current().unwrap().delta, p(0.0, -80.0));
        let upper = decode(1, [p(-20.0, 20.0), p(20.0, 20.0)], p(0.0, 1.0), 0x28).unwrap();
        rays.hit(upper, p(-7.0, 20.1)).unwrap();
        assert_eq!(
            rays.current().unwrap().origin.y.to_bits(),
            (20.0 - (-NATIVE_MIN_T * 70.0)).next_down().to_bits()
        );
        let lower = decode(2, [p(-20.0, -10.0), p(20.0, -10.0)], p(0.0, 1.0), 0x28).unwrap();
        rays.hit(lower, p(-7.0, -9.0)).unwrap();
        rays.miss();
        let mut xs = Vec::new();
        while let Some(ray) = rays.current() {
            xs.push(ray.origin.x);
            rays.miss();
        }
        assert_eq!(xs, [3.0, 13.0, 16.0]);
    }
    #[test]
    fn platform_ray_rejects_invalid_native_hits() {
        for point in [p(1.0, 0.0), p(0.0, 11.0), p(0.0, -11.0), p(0.0, f32::NAN)] {
            let mut rays = Rays::new([0.0, 10.0, 10.0, -10.0]).unwrap();
            let line = decode(1, [p(-5.0, 0.0), p(5.0, 0.0)], p(0.0, 1.0), 0x28).unwrap();
            assert_eq!(rays.hit(line, point), Err(RayError));
            assert_eq!(rays.current(), None);
        }
        assert!(Rays::new([0.0, 10.0, -10.0, 10.0]).is_none());
    }
    /// Transcribes the reviewed native intersection window and reported-point
    /// bias independently of Rays' geometry-based continuation.
    struct FakeNative {
        lines: Vec<Line>,
        calls: usize,
        behind_hits: usize,
        shortened_hits: usize,
    }
    impl FakeNative {
        fn new(lines: Vec<Line>) -> Self {
            Self {
                lines,
                calls: 0,
                behind_hits: 0,
                shortened_hits: 0,
            }
        }
        fn ray(&mut self, ray: Ray) -> Option<(Line, Vec2)> {
            self.calls += 1;
            let cross = |a: Vec2, b: Vec2| a.x * b.y - a.y * b.x;
            let mut nearest = f32::INFINITY;
            let mut chosen = None;
            for line in &self.lines {
                let a = line.ends[0];
                let s = p(line.ends[1].x - a.x, line.ends[1].y - a.y);
                let q = p(a.x - ray.origin.x, a.y - ray.origin.y);
                let denominator = cross(ray.delta, s);
                if denominator == 0.0 {
                    continue;
                }
                let t = cross(q, s) / denominator;
                let u = cross(q, ray.delta) / denominator;
                // Owned main constants: -1e-5, 1.00001, and 0.99.
                if !(-1e-5..=1.00001).contains(&t) || !(-1e-5..=1.00001).contains(&u) {
                    continue;
                }
                let fraction = if t < 0.0 { 0.0 } else { t.min(1.0) * 0.99 };
                if fraction < nearest {
                    nearest = fraction;
                    chosen = Some((*line, t, fraction));
                }
            }
            let (line, t, fraction) = chosen?;
            if t < 0.0 {
                self.behind_hits += 1;
            } else {
                self.shortened_hits += 1;
            }
            Some((
                line,
                p(
                    ray.origin.x + ray.delta.x * fraction,
                    ray.origin.y + ray.delta.y * fraction,
                ),
            ))
        }
    }
    fn platform(id: usize, a: Vec2, b: Vec2) -> Line {
        decode(id, [a, b], p(0.0, 1.0), 0x28).unwrap()
    }
    #[test]
    fn native_biased_rays_clear_stacked_sloped_and_negative_t_hits() {
        assert_eq!(NATIVE_MIN_T.to_bits(), 0xb727c5ac);
        assert_eq!(NATIVE_MAX_T.to_bits(), 0x3f800054);
        let horizontal = |id, y| platform(id, p(-5.0, y), p(5.0, y));
        let cases = [
            // A close pair catches any replacement with a fixed 0.5-unit step.
            (
                vec![
                    horizontal(1, 20.0),
                    horizontal(2, 19.99),
                    horizontal(3, -10.0),
                ],
                30.0,
                false,
            ),
            (
                vec![
                    platform(1, p(-5.0, 15.0), p(5.0, 25.0)),
                    platform(2, p(-5.0, -5.0), p(5.0, -15.0)),
                ],
                30.0,
                false,
            ),
            // First cast reports its origin despite the true line being above it.
            (
                vec![horizontal(1, 20.0), horizontal(2, -10.0)],
                20.0_f32.next_down(),
                true,
            ),
        ];
        for (lines, top, negative) in cases {
            let expected: Vec<_> = lines.iter().map(|line| line.id).collect();
            let mut native = FakeNative::new(lines);
            let mut rays = Rays::new([0.0, 0.0, top, -50.0]).unwrap();
            let mut seen = Vec::new();
            while let Some(ray) = rays.current() {
                assert!(
                    native.calls < expected.len() * 3 + 1,
                    "scan failed to progress"
                );
                match native.ray(ray) {
                    Some((line, point)) => {
                        seen.push(line.id);
                        rays.hit(line, point).unwrap();
                    }
                    None => rays.miss(),
                }
            }
            assert_eq!(seen, expected, "each platform must be visited exactly once");
            assert!(native.calls <= expected.len() * 3);
            assert_eq!(native.behind_hits > 0, negative);
            assert!(native.shortened_hits > 0);
        }
    }
    #[test]
    fn accepted_hit_beyond_bottom_finishes_column_from_true_geometry() {
        let line = platform(1, p(-5.0, -10.00005), p(5.0, -10.00005));
        let mut native = FakeNative::new(vec![line]);
        let mut rays = Rays::new([0.0, 0.0, 0.0, -10.0]).unwrap();
        let (line, point) = native.ray(rays.current().unwrap()).unwrap();
        assert!(line.ends[0].y < -10.0);
        assert!(point.y > -10.0, "native .99 output stays inside the ray");
        rays.hit(line, point).unwrap();
        assert_eq!(rays.current(), None);
        assert_eq!(native.calls, 1);
    }
    #[test]
    fn vertical_or_degenerate_hit_geometry_breaks_discovery() {
        let base = platform(1, p(-5.0, 0.0), p(5.0, 0.0));
        for ends in [[p(0.0, -1.0), p(0.0, 1.0)], [p(0.0, 0.0); 2]] {
            let mut rays = Rays::new([0.0, 0.0, 10.0, -10.0]).unwrap();
            assert_eq!(rays.hit(Line { ends, ..base }, p(0.0, 0.1)), Err(RayError));
            assert_eq!(rays.current(), None);
        }
    }
}
