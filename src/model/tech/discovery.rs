//! Native surface-field decoding and one-ray-per-step discovery schedule.
use super::surface::Line;
use super::{finite, Vec2};

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

/// One vertical column per 10 world units, with the right boundary included.
/// After a hit, continue strictly below it by the next representable f32 value.
/// This visits stacked platforms rather than keeping only the highest hit.
pub struct Rays {
    range: [f32; 4],
    x: f32,
    y: f32,
    done: bool,
}
impl Rays {
    pub fn new(range: [f32; 4]) -> Option<Self> {
        let [left, right, top, bottom] = range;
        (range.into_iter().all(f32::is_finite) && left <= right && bottom < top).then_some(Self {
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
    /// A successful ray must return a finite point on its vertical segment.
    /// Invalid native output ends discovery and is reported as an error.
    pub fn hit(&mut self, point: Vec2) -> Result<(), RayError> {
        let ray = self.current().ok_or(RayError)?;
        if !finite(point) || point.x != ray.origin.x || point.y > self.y || point.y < self.range[3]
        {
            self.done = true;
            return Err(RayError);
        }
        let below = point.y.next_down();
        if below <= self.range[3] || !below.is_finite() {
            self.miss();
        } else {
            self.y = below;
        }
        Ok(())
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
        rays.hit(p(-7.0, 20.0)).unwrap();
        assert_eq!(
            rays.current().unwrap().origin.y.to_bits(),
            20.0_f32.next_down().to_bits()
        );
        rays.hit(p(-7.0, -10.0)).unwrap();
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
            assert_eq!(rays.hit(point), Err(RayError));
            assert_eq!(rays.current(), None);
        }
        assert!(Rays::new([0.0, 10.0, -10.0, 10.0]).is_none());
    }
}
