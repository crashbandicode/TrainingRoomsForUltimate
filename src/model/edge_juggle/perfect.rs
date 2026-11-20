//! Perfect-mode recovery and escape decisions. The runtime applies only pad inputs.

pub type Point = [f32; 2];
pub type Segment = [Point; 2];

/// A complete, bounded walk around the main solid stage component.
#[derive(Clone, Copy)]
pub struct Outline {
    segments: [Segment; 64],
    len: usize,
}

impl Outline {
    pub const fn new() -> Self {
        Self { segments: [[[0.0; 2]; 2]; 64], len: 0 }
    }

    pub fn add(&mut self, segment: Segment) {
        if let Some(slot) = self.segments.get_mut(self.len) {
            *slot = segment;
            self.len += 1;
        }
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments[..self.len]
    }
}

impl Default for Outline {
    fn default() -> Self {
        Self::new()
    }
}

/// The foot heights that leave enough room below a closed main-stage outline.
/// The caller passes live segments from one walk in the current stage.
pub fn corridor(
    outline: &[Segment],
    ledges: [Point; 2],
    blast_bottom: f32,
    body_clearance: f32,
    fall_budget: f32,
) -> Option<[f32; 2]> {
    let [[left, left_y], [right, right_y]] = ledges;
    if ![left, left_y, right, right_y, blast_bottom, body_clearance, fall_budget]
        .into_iter().all(f32::is_finite)
        || left >= right || body_clearance <= 0.0 || fall_budget < 0.0 || outline.is_empty()
    {
        return None;
    }
    let mut underside = left_y.min(right_y);
    for &[a, b] in outline {
        if !a.into_iter().chain(b).all(f32::is_finite) {
            return None;
        }
        let ([x0, y0], [x1, y1]) = if a[0] <= b[0] { (a, b) } else { (b, a) };
        let start = x0.max(left);
        let end = x1.min(right);
        if start <= end {
            let low = if x0 == x1 {
                y0.min(y1)
            } else {
                let at = |x| y0 + (y1 - y0) * ((x - x0) / (x1 - x0));
                at(start).min(at(end))
            };
            underside = underside.min(low);
        }
    }
    let low = blast_bottom + fall_budget;
    let high = underside - body_clearance;
    (low.is_finite() && high.is_finite() && low < high).then_some([low, high])
}

/// Native fighter parameters in world units and frames.
#[derive(Clone, Copy, Debug)]
pub struct AirPhysics {
    pub horizontal_speed: f32,
    pub horizontal_acceleration: f32,
    pub gravity: f32,
    pub terminal_speed: f32,
    pub jump_speed: f32,
}

impl AirPhysics {
    pub fn valid(self) -> bool {
        [self.horizontal_speed, self.horizontal_acceleration, self.gravity,
            self.terminal_speed, self.jump_speed]
            .into_iter().all(|value| value.is_finite() && value > 0.0)
    }

    /// Drift towards the near ledge, preserving the native drift velocity,
    /// then spend the remaining jump. Stop at the first crossing of ledge x.
    pub fn reaches_with_jump(
        self, from: Point, speed: Point, ledge: Point, delay: u8, blast_bottom: f32,
    ) -> bool {
        if !self.valid() || !from.into_iter().chain(speed).chain(ledge).chain([blast_bottom]).all(f32::is_finite) {
            return false;
        }
        let direction = (ledge[0] - from[0]).signum();
        let mut dx = (ledge[0] - from[0]).abs();
        let mut y = from[1];
        let [mut vx, mut vy] = [speed[0] * direction, speed[1]];
        const SIMULATION_FRAMES: u8 = 90;
        for frame in 0..SIMULATION_FRAMES {
            if y <= blast_bottom {
                return false;
            }
            vy = if frame == delay {
                self.jump_speed
            } else {
                (vy - self.gravity).max(-self.terminal_speed)
            };
            vx = (vx + self.horizontal_acceleration).min(self.horizontal_speed);
            dx -= vx;
            y += vy;
            if dx <= 0.0 {
                return y >= ledge[1];
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn recovery_jump_can_wait_only_with_enough_horizontal_and_vertical_reach() {
        let air = AirPhysics {
            horizontal_speed: 2.0,
            horizontal_acceleration: 0.15,
            gravity: 0.12,
            terminal_speed: 2.1,
            jump_speed: 3.5,
        };
        let position = [-95., -14.];
        let velocity = [0., -1.4];
        let ledge = [-70., 0.];
        assert!(air.reaches_with_jump(position, velocity, ledge, 4, -95.));
        assert!(!air.reaches_with_jump(position, velocity, ledge, 30, -95.));
        assert!(!air.reaches_with_jump(position, velocity, [40., 0.], 0, -95.));
        assert!(!air.reaches_with_jump([-95., -94.], velocity, ledge, 1, -95.));
    }

    #[test]
    fn opposite_ledge_requires_a_full_corridor_above_the_blast_zone() {
        let ledges = [[-70., 0.], [70., 0.]];
        let fd = [
            [ledges[0], ledges[1]],
            [ledges[0], [0., -32.]],
            [[0., -32.], ledges[1]],
        ];
        assert_eq!(corridor(&fd, ledges, -105., 12., 15.), Some([-90., -44.]));
        let wedge = [fd[0], [ledges[0], [0., -110.]], [[0., -110.], ledges[1]]];
        assert_eq!(corridor(&wedge, ledges, -105., 12., 15.), None);
        assert_eq!(corridor(&fd, ledges, -60., 18., 15.), None);
        assert_eq!(corridor(&[], ledges, -105., 12., 15.), None);
    }
    }
}
