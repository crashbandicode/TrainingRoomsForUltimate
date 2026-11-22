use super::btt;
use super::panes::{EggLook, EggPicture, EGG_WHOLE};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crosshair {
    pub at: [f32; 2],
    was: u16,
    rep: u8,
}

impl Crosshair {
    pub const fn new(at: [f32; 2]) -> Self {
        Self { at, was: 0, rep: 0 }
    }

    pub fn step(&mut self, pad: &btt::Pad, fit: impl FnOnce([f32; 2]) -> Option<[f32; 2]>) -> bool {
        let motion = btt::placing_motion(pad, self.was, &mut self.rep);
        self.was = pad.held;
        let wanted = [self.at[0] + motion[0], self.at[1] + motion[1]];
        let Some(at) = fit(wanted) else { return false };
        let changed = self.at != at;
        self.at = at;
        changed
    }

    pub fn jump(&mut self, at: [f32; 2]) {
        self.at = at;
        self.rep = 0;
    }

    pub fn look(&self) -> ([f32; 2], EggLook) {
        (self.at, EggLook { picture: EggPicture::Cursor, ..EGG_WHOLE })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marks {
    pub looks: [Option<([f32; 2], EggLook)>; btt::LOOKS],
    pub pointer: Option<[f32; 2]>,
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::model::btt::{self, Pad};
    use crate::model::chords::logical::{B, DPAD_LEFT, DPAD_RIGHT, ZL};
    use crate::model::panes::{EggLook, EggPicture, EGG_WHOLE};

    fn pad(stick: [f32; 2], held: u16) -> Pad {
        Pad { stick, held, ..Pad::default() }
    }

    #[test]
    fn crosshair_step_is_break_the_targets_motion() {
        let right = 1 << DPAD_RIGHT;
        let left = 1 << DPAD_LEFT;
        let samples = [
            vec![pad([0.299, -0.299], 0), pad([0.3, -0.3], 0),
                 pad([-0.299, 0.299], 0), pad([-0.3, 0.3], 0),
                 pad([0.75, -0.9], 0)],
            vec![pad([0.0; 2], right), pad([0.0; 2], 0)],
            vec![pad([0.0; 2], right); 40],
            vec![pad([0.0; 2], right | (1 << ZL)); 40],
            vec![pad([0.0; 2], right | (1 << B)); 40],
            vec![pad([0.0; 2], right), pad([0.0; 2], left),
                 pad([0.0; 2], left), pad([0.0; 2], right)],
        ];
        for sequence in samples {
            let mut cursor = Crosshair::new([10.25, -2.5]);
            let mut reference = cursor.at;
            let (mut was, mut age) = (0, 0);
            for sample in sequence {
                let motion = btt::placing_motion(&sample, was, &mut age);
                reference = [reference[0] + motion[0], reference[1] + motion[1]];
                was = sample.held;
                cursor.step(&sample, Some);
                assert_eq!(cursor.at.map(f32::to_bits), reference.map(f32::to_bits));
            }
        }
    }

    #[test]
    fn crosshair_fit_clamps_and_can_refuse() {
        let right = pad([0.0; 2], 1 << DPAD_RIGHT);
        let mut cursor = Crosshair::new([0.0, 0.0]);
        assert!(cursor.step(&right, |p| Some([p[0].min(0.5), p[1]])));
        assert_eq!(cursor.at, [0.5, 0.0]);
        for _ in 0..14 {
            assert!(!cursor.step(&right, |_| None));
            assert_eq!(cursor.at, [0.5, 0.0]);
        }
        assert!(cursor.step(&right, Some));
        assert_eq!(cursor.at, [1.5, 0.0]);
    }

    #[test]
    fn crosshair_jump_keeps_a_held_dpad_from_stepping() {
        let right = pad([0.0; 2], 1 << DPAD_RIGHT);
        let mut cursor = Crosshair::new([0.0, 0.0]);
        assert!(cursor.step(&right, Some));
        cursor.jump([17.25, -2.0]);
        assert!(!cursor.step(&right, Some));
        assert_eq!(cursor.at, [17.25, -2.0]);
        assert!(!cursor.step(&pad([0.0; 2], 0), Some));
        assert!(cursor.step(&right, Some));
        assert_eq!(cursor.at, [18.25, -2.0]);
    }

    #[test]
    fn crosshair_look_is_the_cursor_picture() {
        let cursor = Crosshair::new([3.5, -8.0]);
        assert_eq!(cursor.look(), ([3.5, -8.0], EggLook { picture: EggPicture::Cursor, ..EGG_WHOLE }));
    }
    }
}
