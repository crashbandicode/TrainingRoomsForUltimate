//! Stage tech practice. Native launch mechanics are supplied by a separate adapter.
//! Surface/solver changes require tech-direct review before runtime use (DESIGN §3–4).
pub mod session;
pub mod solver;
pub mod surface;
pub mod variables;

pub use super::tech_launch::Vec2;

pub fn dot(a: Vec2, b: Vec2) -> f32 {
    a.x * b.x + a.y * b.y
}
pub fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2 {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}
pub fn add(a: Vec2, b: Vec2) -> Vec2 {
    Vec2 {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}
pub fn scale(a: Vec2, s: f32) -> Vec2 {
    Vec2 {
        x: a.x * s,
        y: a.y * s,
    }
}
pub fn finite(a: Vec2) -> bool {
    a.x.is_finite() && a.y.is_finite()
}

/// The UI-to-core boundary. Percent is the desired displayed pre-flight percent;
/// hit_damage shapes hitlag and must not be added to percent by the adapter.
/// The core derives theta_rel, speed_vec, native hit height, hitlag and LR seed.
/// KB-to-speed conversion belongs exclusively to the core's native 0x403a20 call;
/// the approximate host solver does not supply a speed vector to this interface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launch {
    pub start: Vec2,
    pub target: Vec2,
    /// WORLD angle in degrees; the core derives the hit-side-relative angle.
    pub angle: f32,
    pub knockback: f32,
    pub percent: f32,
    pub hit_damage: f32,
    pub height: HitHeight,
    pub facing: f32,
    pub hit_from_behind: bool,
    pub di: Option<Vec2>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitHeight {
    Low,
    Middle,
    High,
}

/// Native calls and guards belong to the core, which owns sleep/placement/release.
/// A rejected request does not begin or count an attempt.
pub trait Launcher {
    type Error;
    fn place(&mut self, launch: Launch) -> Result<(), Self::Error>;
    fn launch(&mut self) -> Result<(), Self::Error>;
    fn restore(&mut self);
}

#[cfg(test)]
mod tests;
