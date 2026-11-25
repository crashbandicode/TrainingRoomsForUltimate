//! The step between traced floors (placement, operator 2026-11-25, PC1 B): Left / Right along the current floor,
//! Up / Down to the floor above or below. The floors are the floor lines of Tech practice's surface walk, as world
//! geometry; nothing here knows a stage.
use super::btt::Pad;
use super::chords::logical::{DPAD_DOWN, DPAD_UP};

/// A floor line's two ends, in world units.
pub type Floor = [[f32; 2]; 2];

/// A stick this far up or down, and further than sideways, asks for another floor.
pub const HOP_TILT: f32 = 0.7;
/// The stick has to come back under this before it can ask again.
pub const HOP_REST: f32 = 0.3;

/// The floor's height at `x`, when the floor spans it.
pub fn height(floor: Floor, x: f32) -> Option<f32> {
    let _ = (floor, x);
    None
}

/// The floor point a fighter at `at` stands on: the highest floor at or below it (one unit of slack above).
pub fn under(floors: &[Floor], at: [f32; 2]) -> Option<[f32; 2]> {
    let _ = (floors, at);
    None
}

/// `at` moved `dx` along its floor. At the floor's end it goes on along a floor that shares that end point;
/// otherwise it stops there.
pub fn along(floors: &[Floor], at: [f32; 2], dx: f32) -> [f32; 2] {
    let _ = (floors, dx);
    at
}

/// The nearest floor above (`up`) or below `at`, at the same x. `None`: there is no further floor.
pub fn hop(floors: &[Floor], at: [f32; 2], up: bool) -> Option<[f32; 2]> {
    let _ = (floors, at, up);
    None
}

/// Which way a pad asks to change floor: a D-pad Up / Down press, or one flick of the stick past `HOP_TILT` that is
/// more vertical than horizontal. A held stick asks once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hop {
    spent: bool,
}

impl Hop {
    pub const fn new() -> Self {
        Self { spent: false }
    }

    /// `1` up, `-1` down, `0` stay.
    pub fn vertical(&mut self, pad: &Pad) -> i8 {
        let _ = (pad, DPAD_UP, DPAD_DOWN, self.spent);
        0
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::model::btt::Pad;
    use crate::model::chords::logical::{DPAD_DOWN, DPAD_LEFT, DPAD_UP};

    /// The ground from -60 to 60, a platform at 30 from -40 to -10, one above it at 55 from -30 to -20, and a ramp
    /// that joins the ground's right end and rises to (90, 15).
    fn floors() -> Vec<Floor> {
        vec![
            [[-60.0, 0.0], [60.0, 0.0]],
            [[-10.0, 30.0], [-40.0, 30.0]],
            [[-30.0, 55.0], [-20.0, 55.0]],
            [[60.0, 0.0], [90.0, 15.0]],
        ]
    }

    #[test]
    fn three_stacked_floors_hop_one_at_a_time_both_ways() {
        let floors = floors();
        let ground = [-25.0, 0.0];
        let middle = hop(&floors, ground, true).unwrap();
        assert_eq!(middle, [-25.0, 30.0]);
        let top = hop(&floors, middle, true).unwrap();
        assert_eq!(top, [-25.0, 55.0]);
        assert_eq!(hop(&floors, top, false), Some(middle));
        assert_eq!(hop(&floors, middle, false), Some(ground));
        // Beside the top platform the middle one is the last floor up.
        assert_eq!(hop(&floors, [-35.0, 0.0], true), Some([-35.0, 30.0]));
        assert_eq!(hop(&floors, [-35.0, 30.0], true), None);
    }

    #[test]
    fn no_further_floor_keeps_the_fighter() {
        let floors = floors();
        assert_eq!(hop(&floors, [-25.0, 55.0], true), None);
        assert_eq!(hop(&floors, [-25.0, 0.0], false), None);
        assert_eq!(hop(&floors, [40.0, 0.0], true), None);
        assert_eq!(hop(&[], [0.0, 0.0], true), None);
    }

    #[test]
    fn the_floor_under_a_point_is_the_highest_at_or_below_it() {
        let floors = floors();
        assert_eq!(under(&floors, [-25.0, 80.0]), Some([-25.0, 55.0]));
        assert_eq!(under(&floors, [-25.0, 54.5]), Some([-25.0, 55.0]));
        assert_eq!(under(&floors, [-25.0, 40.0]), Some([-25.0, 30.0]));
        assert_eq!(under(&floors, [-25.0, 10.0]), Some([-25.0, 0.0]));
        assert_eq!(under(&floors, [-25.0, -5.0]), None);
        assert_eq!(under(&floors, [-70.0, 20.0]), None);
        assert_eq!(height(floors[3], 75.0), Some(7.5));
        assert_eq!(height(floors[3], 95.0), None);
    }

    #[test]
    fn sliding_stops_at_a_floors_end_and_crosses_a_shared_point() {
        let floors = floors();
        assert_eq!(along(&floors, [-25.0, 30.0], 2.0), [-23.0, 30.0]);
        // A platform's end is the end: the fighter does not drop off it.
        assert_eq!(along(&floors, [-11.0, 30.0], 2.0), [-10.0, 30.0]);
        assert_eq!(along(&floors, [-39.0, 30.0], -5.0), [-40.0, 30.0]);
        assert_eq!(along(&floors, [-59.0, 0.0], -2.0), [-60.0, 0.0]);
        // The ground and the ramp share (60, 0): the slide goes on, up the ramp and back.
        assert_eq!(along(&floors, [59.0, 0.0], 2.0), [61.0, 0.5]);
        assert_eq!(along(&floors, [61.0, 0.5], -2.0), [59.0, 0.0]);
        assert_eq!(along(&floors, [89.0, 14.5], 2.0), [90.0, 15.0]);
        // A point on no floor stays where it is.
        assert_eq!(along(&floors, [-25.0, 12.0], 2.0), [-25.0, 12.0]);
    }

    #[test]
    fn a_sloped_floor_keeps_the_fighter_on_it() {
        let floors = floors();
        let mut at = [62.0, 1.0];
        for _ in 0..5 {
            at = along(&floors, at, 2.0);
            assert_eq!(Some(at[1]), height(floors[3], at[0]));
        }
        assert_eq!(at, [72.0, 6.0]);
    }

    fn stick(x: f32, y: f32) -> Pad {
        Pad { stick: [x, y], ..Pad::default() }
    }

    fn press(key: u8) -> Pad {
        Pad { held: 1 << key, pressed: 1 << key, ..Pad::default() }
    }

    #[test]
    fn a_diagonal_stick_does_not_change_floor() {
        let mut hop = Hop::new();
        for pad in [stick(0.9, 0.69), stick(0.8, 0.75), stick(-0.9, -0.8), stick(0.7, 0.7), stick(0.0, 0.69)] {
            assert_eq!(hop.vertical(&pad), 0, "{:?}", pad.stick);
        }
        assert_eq!(hop.vertical(&stick(0.3, 0.9)), 1);
        let mut hop = Hop::new();
        assert_eq!(hop.vertical(&stick(-0.6, -0.71)), -1);
    }

    #[test]
    fn a_held_stick_hops_once() {
        let mut hop = Hop::new();
        assert_eq!(hop.vertical(&stick(0.0, 1.0)), 1);
        for _ in 0..40 {
            assert_eq!(hop.vertical(&stick(0.0, 1.0)), 0);
        }
        // Still out of the rest zone: not a new flick.
        assert_eq!(hop.vertical(&stick(0.0, 0.4)), 0);
        assert_eq!(hop.vertical(&stick(0.0, 0.9)), 0);
        assert_eq!(hop.vertical(&stick(0.0, 0.2)), 0);
        assert_eq!(hop.vertical(&stick(0.0, -1.0)), -1);
    }

    #[test]
    fn a_dpad_press_changes_floor_and_a_held_one_does_not_repeat() {
        let mut hop = Hop::new();
        assert_eq!(hop.vertical(&press(DPAD_UP)), 1);
        let held = Pad { held: 1 << DPAD_UP, ..Pad::default() };
        for _ in 0..40 {
            assert_eq!(hop.vertical(&held), 0);
        }
        assert_eq!(hop.vertical(&press(DPAD_DOWN)), -1);
        assert_eq!(hop.vertical(&press(DPAD_LEFT)), 0);
    }

    #[test]
    fn the_hop_uses_live_lines_and_no_stage_ids() {
        let source = include_str!("floor_hop.rs");
        let code = source.split("#[cfg(test)]").next().unwrap();
        for word in ["stage_id", "StageID", "lobby_stages", "fighter_kind"] {
            assert!(!code.contains(word), "{word}");
        }
    }
    }
}
