//! Allocation-free projections of neutral 13.0.5 projectile flights for Eggsercise.
//! The game keeps ownership of fighters, hitboxes, spawn geometry and drill timing.

#[derive(Clone, Copy, Debug)]
pub enum Path {
    Axe,
    Galaxian,
    Boomerang,
    Missile,
}

impl Path {
    pub fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::Axe),
            1 => Some(Self::Galaxian),
            2 => Some(Self::Boomerang),
            3 => Some(Self::Missile),
            _ => None,
        }
    }

    /// Relative displacement bounds, with positive x in the direction of release.
    pub fn envelope(self) -> [[f32; 2]; 2] {
        match self {
            Self::Axe => [[0.0, -80.0], [78.0, 23.0]],
            Self::Galaxian => [[0.0, -2.0], [180.0, 24.0]],
            Self::Boomerang => [[0.0, 0.0], [65.0, 0.0]],
            // The decay bounds total distance by 54 units. Sustained maximum
            // pitch change bounds vertical travel by 27 units in either direction.
            Self::Missile => [[0.0, -28.0], [55.0, 28.0]],
        }
    }

    fn lifetime(self) -> u8 {
        match self {
            Self::Axe => 68,
            Self::Galaxian => 130,
            Self::Boomerang => 129,
            Self::Missile => 120,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Flight {
    path: Path,
    right: bool,
    frames: u8,
    half_pending: bool,
    step: [f32; 2],
    speed: f32,
    auxiliary: f32,
    x: f32,
}

impl Flight {
    pub fn new(path: Path, facing_right: bool) -> Self {
        let (speed, auxiliary) = match path {
            Path::Axe => {
                let (sin, cos) = 60.0_f32.to_radians().sin_cos();
                (2.9 * cos, 2.9 * sin)
            }
            Path::Galaxian => (1.5, 0.0),
            Path::Boomerang => (2.4, 0.0),
            Path::Missile => (1.15, 0.0),
        };
        Self {
            path,
            right: facing_right,
            frames: 0,
            half_pending: false,
            step: [0.0; 2],
            speed,
            auxiliary,
            x: 0.0,
        }
    }

    /// One active game tick. The target vector points from the egg toward P1 in world space.
    /// Both half-steps of a stock frame share one velocity, so changing speed cannot skip phases.
    pub fn advance(&mut self, speed: u8, target: [f32; 2]) -> [f32; 2] {
        let halves = match speed {
            1 => 1,
            2 => 3,
            _ => 2,
        };
        let facing = if self.right { 1.0 } else { -1.0 };
        let local_target = [target[0] * facing, target[1]];
        let mut displacement = [0.0, 0.0];
        for _ in 0..halves {
            if !self.half_pending {
                self.step = self.stock_frame(local_target);
            }
            self.half_pending = !self.half_pending;
            displacement[0] += self.step[0] * 0.5;
            displacement[1] += self.step[1] * 0.5;
        }
        [displacement[0] * facing, displacement[1]]
    }

    fn stock_frame(&mut self, target: [f32; 2]) -> [f32; 2] {
        if self.frames >= self.path.lifetime() {
            return [0.0, 0.0];
        }
        let frame = self.frames;
        self.frames += 1;
        match self.path {
            Path::Axe => {
                // Richter's two native kinetic energies apply the x brake and y gravity.
                self.speed = (self.speed - 0.01).max(0.0);
                self.auxiliary = (self.auxiliary - 0.15).max(-2.1);
                [self.speed, self.auxiliary]
            }
            Path::Galaxian => self.galaxian_frame(frame),
            Path::Boomerang => self.boomerang_frame(),
            Path::Missile => self.missile_frame(frame, target),
        }
    }

    fn galaxian_frame(&mut self, frame: u8) -> [f32; 2] {
        const STRAIGHT_FRAMES: u8 = 25;
        // The stock countdown is drawn from 36 through 40. Use the 36-frame
        // release to give the drill one reproducible reference.
        const TURN_FRAMES: u8 = 36;
        if (STRAIGHT_FRAMES..STRAIGHT_FRAMES + TURN_FRAMES).contains(&frame) {
            self.auxiliary += 10.0;
            self.speed = (self.speed + 0.10).min(2.0);
        }
        let (sin, cos) = self.auxiliary.to_radians().sin_cos();
        [self.speed * cos, self.speed * sin]
    }

    fn boomerang_frame(&mut self) -> [f32; 2] {
        let delta = if self.speed > 0.0 {
            let outbound = self.speed;
            let next = outbound - 0.047;
            self.speed = if next > 0.3 { next } else { -0.3 };
            outbound
        } else {
            self.speed = (self.speed - 0.047).max(-3.0);
            self.speed.max(-self.x)
        };
        self.x += delta;
        [delta, 0.0]
    }

    fn missile_frame(&mut self, frame: u8, target: [f32; 2]) -> [f32; 2] {
        if frame != 0 && self.speed > 0.4 {
            self.speed *= 0.95;
        }
        let (sin, cos) = self.auxiliary.sin_cos();
        let distance = target[0].hypot(target[1]);
        if frame != 0 && frame <= 85 && distance.is_finite() && distance > 0.0 {
            let aim = [target[0] / distance, target[1] / distance];
            let dot = cos * aim[0] + sin * aim[1];
            // Compare against cos(1°) to reproduce the native angle deadband
            // without evaluating acos. The y components choose the direction of rotation.
            let pitch_error = aim[1] - sin - 0.001;
            if dot <= 1.0_f32.to_radians().cos() && pitch_error != 0.0 {
                let turn = 0.65_f32.to_radians().copysign(pitch_error);
                let cap = 70.0_f32.to_radians();
                self.auxiliary = (self.auxiliary + turn).clamp(-cap, cap);
            }
        }
        let (sin, cos) = self.auxiliary.sin_cos();
        [self.speed * cos, self.speed * sin]
    }
}

#[cfg(test)]
lib_unit_tests! {
    mod tests {
        use super::{Flight, Path};

        fn close(actual: [f32; 2], expected: [f32; 2]) {
            for (actual, expected) in actual.into_iter().zip(expected) {
                assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
            }
        }

        #[test]
        fn stock_axe_does_not_use_the_vertical_bounce() {
            let mut axe = Flight::new(Path::Axe, true);
            close(axe.advance(0, [0.0; 2]), [1.44, 2.361_474]);
            for _ in 1..35 {
                axe.advance(0, [0.0; 2]);
            }
            close(axe.advance(0, [0.0; 2]), [1.09, -2.1]);
        }

        #[test]
        fn stock_galaxian_travels_forward_then_loops() {
            let mut galaxian = Flight::new(Path::Galaxian, true);
            for _ in 0..25 {
                close(galaxian.advance(0, [0.0; 2]), [1.5, 0.0]);
            }
            close(galaxian.advance(0, [0.0; 2]), [1.575_692, 0.277_837]);
            for _ in 26..40 {
                galaxian.advance(0, [0.0; 2]);
            }
            assert!(galaxian.advance(0, [0.0; 2])[0] < 0.0);
            for _ in 41..61 {
                galaxian.advance(0, [0.0; 2]);
            }
            for _ in 61..130 {
                close(galaxian.advance(0, [0.0; 2]), [2.0, 0.0]);
            }
            close(galaxian.advance(0, [0.0; 2]), [0.0; 2]);
        }

        #[test]
        fn stock_boomerang_flies_out_then_returns() {
            let mut boomerang = Flight::new(Path::Boomerang, true);
            let mut distance = 0.0;
            let first = boomerang.advance(0, [0.0; 2]);
            close(first, [2.4, 0.0]);
            distance += first[0];
            for _ in 1..45 {
                distance += boomerang.advance(0, [0.0; 2])[0];
            }
            assert!((distance - 61.47).abs() < 0.001);
            assert!(boomerang.advance(0, [0.0; 2])[0] < 0.0);
            for _ in 46..120 {
                boomerang.advance(0, [0.0; 2]);
            }
            close(boomerang.advance(0, [0.0; 2]), [0.0; 2]);
        }

        #[test]
        fn stock_homing_missile_has_forward_speed_and_steers_live() {
            let mut missile = Flight::new(Path::Missile, true);
            close(missile.advance(0, [20.0, 100.0]), [1.15, 0.0]);
            for _ in 1..10 {
                assert!(missile.advance(0, [20.0, 100.0])[1] > 0.0);
            }
            let mut inertia = missile;
            for _ in 10..30 {
                missile.advance(0, [20.0, -100.0]);
                inertia.advance(0, [20.0, 100.0]);
            }
            assert!(missile.advance(0, [20.0, -100.0])[1] < 0.0);
            assert!(inertia.advance(0, [20.0, 100.0])[1] > 0.0);
            for _ in 31..86 {
                inertia.advance(0, [20.0, 100.0]);
            }
            let after_homing = inertia.advance(0, [20.0, 100.0]);
            close(inertia.advance(0, [20.0, -100.0]), after_homing);
            close([after_homing[0].hypot(after_homing[1]), 0.0], [0.391_645, 0.0]);
        }

        #[test]
        fn selected_paths_use_stock_samples_and_mirror() {
            assert!(Path::from_index(4).is_none());
            for path in (0..4).map(|index| Path::from_index(index).unwrap()) {
                let (mut right, mut left) = (Flight::new(path, true), Flight::new(path, false));
                let [lo, hi] = path.envelope();
                let mut position = [0.0; 2];
                for frame in 0..path.lifetime() {
                    let y = if frame % 2 == 0 { 100.0 } else { -100.0 };
                    let delta = right.advance(0, [20.0, y]);
                    close(left.advance(0, [-20.0, y]), [-delta[0], delta[1]]);
                    for axis in 0..2 {
                        position[axis] += delta[axis];
                        assert!((lo[axis]..=hi[axis]).contains(&position[axis]));
                    }
                }
            }
        }

        #[test]
        fn half_steps_leave_the_stock_phase_timing_intact() {
            for path in (0..4).map(|index| Path::from_index(index).unwrap()) {
                let mut normal = Flight::new(path, true);
                let mut slow = normal;
                let mut fast = normal;
                let target = [20.0, 80.0];
                for _ in 0..50 {
                    let mut sum = [0.0; 2];
                    for _ in 0..3 {
                        let expected = normal.advance(0, target);
                        let (a, b) = (slow.advance(1, target), slow.advance(1, target));
                        close([a[0] + b[0], a[1] + b[1]], expected);
                        sum[0] += expected[0];
                        sum[1] += expected[1];
                    }
                    let (a, b) = (fast.advance(2, target), fast.advance(2, target));
                    close([a[0] + b[0], a[1] + b[1]], sum);
                }
            }
        }
    }
}
