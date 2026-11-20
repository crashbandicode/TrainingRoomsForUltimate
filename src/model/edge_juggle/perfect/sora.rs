//! Map the measured Sonic Blade flight to the live far ledge.

use super::{Point, Segment};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BladePath {
    /// Angles relative to the direction of the crossing. A first angle is always level.
    pub angles: [f32; 3],
    pub count: u8,
    pub spare: f32,
}

pub fn blade_path(
    _outline: &[Segment],
    _origin: Point,
    _hang: Point,
    _direction: f32,
    _bottom: f32,
) -> Option<BladePath> {
    None
}

#[cfg(test)]
lib_unit_tests! {
    #[path = "../../../../tests/common/edge_juggle_sora.rs"]
    mod tests;
}
