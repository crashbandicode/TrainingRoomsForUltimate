//! Per-attempt variables; an external unbiased random source is supplied by runtime.
use super::{HitHeight, Launch, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Set,
    Random,
    Auto,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Variable {
    pub mode: Mode,
    pub low: f32,
    pub high: f32,
    pub step: f32,
}
impl Variable {
    /// `draw(n)` must return a uniform integer in 0..n. Auto has no sampled value.
    pub fn sample(self, draw: &mut impl FnMut(u32) -> u32) -> Option<f32> {
        if ![self.low, self.high, self.step]
            .into_iter()
            .all(f32::is_finite)
            || self.step <= 0.0
            || self.low > self.high
        {
            return None;
        }
        match self.mode {
            Mode::Auto => None,
            Mode::Set => Some(self.low),
            Mode::Random => {
                let n = ((self.high - self.low) / self.step).floor() as u32 + 1;
                let index = draw(n);
                (index < n).then_some(self.low + index as f32 * self.step)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Di {
    Off,
    Player,
    Set(Vec2),
    Random,
}
impl Di {
    pub fn sample(self, draw: &mut impl FnMut(u32) -> u32) -> Option<Vec2> {
        match self {
            Self::Player => None,
            Self::Off => Some(Vec2::default()),
            Self::Set(v) => Some(v),
            Self::Random => {
                const D: f32 = core::f32::consts::FRAC_1_SQRT_2;
                let all = [
                    Vec2 { x: 1.0, y: 0.0 },
                    Vec2 { x: D, y: D },
                    Vec2 { x: 0.0, y: 1.0 },
                    Vec2 { x: -D, y: D },
                    Vec2 { x: -1.0, y: 0.0 },
                    Vec2 { x: -D, y: -D },
                    Vec2 { x: 0.0, y: -1.0 },
                    Vec2 { x: D, y: -D },
                    Vec2 { x: 0.0, y: 0.0 },
                ];
                all.get(draw(9) as usize).copied()
            }
        }
    }
}

pub const HIT_DAMAGE_DEFAULT: f32 = 12.0;
pub const HIT_DAMAGE_RANGE: [f32; 2] = [1.0, 30.0];

/// Presets change just the hit description; flight geometry and percent stay.
pub fn preset(launch: &mut Launch, damage: f32, height: HitHeight) {
    launch.hit_damage = damage;
    launch.height = height;
}
