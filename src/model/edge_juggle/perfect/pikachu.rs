//! Bounded Quick Attack geometry for Pikachu's Skull Bash crossing.

use super::{Point, Segment};

#[derive(Clone, Copy, Debug)]
pub struct ZipPlan {
    pub first: Point,
    pub second: Option<Point>,
}

pub fn quick_attack(
    _outline: &[Segment],
    _from: Point,
    _target: Point,
    _bottom: f32,
    _height: f32,
) -> Option<ZipPlan> {
    None
}

#[cfg(test)]
lib_unit_tests! {
    #[path = "../../../../tests/common/edge_juggle_pikachu.rs"]
    mod tests;
}
