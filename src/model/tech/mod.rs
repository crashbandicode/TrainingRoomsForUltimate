//! Stage tech practice. Native launch mechanics are supplied by a separate adapter.
//! Surface/solver changes require tech-direct review before runtime use (DESIGN §3–4).
pub mod controller;
pub mod picker;
pub mod discovery;
pub mod practice_state;
pub mod session;
pub mod solver;
pub mod surface;
pub mod variables;
pub mod wizard;
pub mod wizard_marks;
#[cfg(test)]
mod wizard_marks_tests;

pub use super::tech_launch::{Attempt, AttemptState, ContactReceipt, HitHeight, Launch, Launcher, Vec2};

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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration_tests;

pub mod planner;


#[cfg(test)]
mod epoch_tests;

#[cfg(test)]
mod wizard_tests;
