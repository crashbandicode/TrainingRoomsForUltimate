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

impl Marks {
    /// The marks to draw. While the CPU's own body waits at its point the game shows it there, or its magnifier with
    /// the direction pointer when the point is out of view, so the border pointer is left out.
    pub fn shown(mut self, cpu_held: bool) -> Self {
        if cpu_held {
            self.pointer = None;
        }
        self
    }
}

/// Frames a picker places the CPU awake at its point, with a Fall request, before it holds it asleep there (the
/// drill's own start: place, request Fall, hold).
pub const PREVIEW_SETTLE: u8 = 4;

/// Where a body stood before a picker moved it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Origin {
    pub at: [f32; 2],
    pub grounded: bool,
    pub facing: f32,
}

/// What a picker does with a fighter's body on one frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewAct {
    /// Place it in the air at the point and request Fall.
    Lift,
    /// Hold it asleep at the point.
    Hold,
    /// Stand it awake on its floor point.
    Stand,
}

/// A picker's hold on the CPU: the body itself waits at its start point. In view the game draws it under the
/// crosshair; out of view the game draws its own magnifier of it, with the direction pointer, at the screen edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CpuPreview {
    body: u32,
    origin: Origin,
    frames: u8,
    /// The hold put the body to sleep, as of this frame's write: whoever lets it go wakes it.
    pub asleep: bool,
    /// The game's own `FLAG_IS_LOUPE` of the body, read back after this frame's write.
    pub magnified: bool,
}

impl CpuPreview {
    pub const fn body(&self) -> u32 {
        self.body
    }

    /// Where the body stood when the picker took it: where it goes back when the picker is left.
    pub const fn origin(&self) -> Origin {
        self.origin
    }

    fn act(&mut self, grounded: bool) -> PreviewAct {
        if grounded {
            self.frames = 0;
            PreviewAct::Stand
        } else if self.frames < PREVIEW_SETTLE {
            self.frames += 1;
            PreviewAct::Lift
        } else {
            PreviewAct::Hold
        }
    }
}

/// One frame of a picker's CPU preview: a preview starts for `body` when none runs for it (`origin` is read then),
/// and the frame's act is returned.
pub fn preview_step(slot: &mut Option<CpuPreview>, body: u32, origin: impl FnOnce() -> Origin) -> PreviewAct {
    hold_step(slot, body, false, origin)
}

/// One frame of a picker's hold on either fighter (the placement picker holds both). In the air it is
/// `preview_step`. `grounded`: the body stands on its floor point, awake, every frame; taken into the air after
/// that it settles again before it is held.
pub fn hold_step(slot: &mut Option<CpuPreview>, body: u32, grounded: bool, origin: impl FnOnce() -> Origin) -> PreviewAct {
    if slot.is_none_or(|preview| preview.body != body) {
        *slot = Some(CpuPreview { body, origin: origin(), frames: 0, asleep: false, magnified: false });
    }
    slot.as_mut().map_or(PreviewAct::Lift, |preview| preview.act(grounded))
}

/// How the picker's CPU point is shown, for the practice state: the body is there in view, or the game shows its
/// magnifier of it; empty when the body is not there.
pub fn cpu_view(preview: Option<&CpuPreview>) -> &'static str {
    match preview {
        None => "",
        Some(preview) if preview.magnified => "magnifier",
        Some(_) => "in_view",
    }
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

    fn origin(x: f32) -> Origin {
        Origin { at: [x, 0.0], grounded: true, facing: 1.0 }
    }

    #[test]
    fn the_picker_places_the_cpu_awake_then_holds_it_asleep() {
        let mut slot = None;
        let acts: Vec<_> = (0..PREVIEW_SETTLE + 6).map(|_| preview_step(&mut slot, 7, || origin(3.0))).collect();
        let settle = usize::from(PREVIEW_SETTLE);
        assert!(acts[..settle].iter().all(|act| *act == PreviewAct::Lift));
        assert!(acts[settle..].iter().all(|act| *act == PreviewAct::Hold));
        assert_eq!(slot.unwrap().origin(), origin(3.0));
    }

    #[test]
    fn a_new_cpu_body_starts_its_own_preview_from_where_it_stood() {
        let mut slot = None;
        for _ in 0..PREVIEW_SETTLE + 2 {
            preview_step(&mut slot, 7, || origin(3.0));
        }
        // The origin is read once, when the picker takes the body.
        assert_eq!(preview_step(&mut slot, 7, || panic!("the same body keeps its origin")), PreviewAct::Hold);
        assert_eq!(preview_step(&mut slot, 9, || origin(-40.0)), PreviewAct::Lift);
        let preview = slot.unwrap();
        assert_eq!((preview.body(), preview.origin()), (9, origin(-40.0)));
    }

    #[test]
    fn a_fighter_on_a_floor_stands_at_its_spot_and_one_in_the_air_is_held() {
        let mut slot = None;
        for _ in 0..PREVIEW_SETTLE + 3 {
            assert_eq!(hold_step(&mut slot, 7, true, || origin(3.0)), PreviewAct::Stand);
        }
        let settle = usize::from(PREVIEW_SETTLE);
        let air: Vec<_> = (0..settle + 2).map(|_| hold_step(&mut slot, 7, false, || panic!("the same body"))).collect();
        assert!(air[..settle].iter().all(|act| *act == PreviewAct::Lift));
        assert!(air[settle..].iter().all(|act| *act == PreviewAct::Hold));
        // Back on a floor it stands again, and the next lift settles again before the hold.
        assert_eq!(hold_step(&mut slot, 7, true, || panic!("the same body")), PreviewAct::Stand);
        assert_eq!(hold_step(&mut slot, 7, false, || panic!("the same body")), PreviewAct::Lift);
        assert_eq!(slot.unwrap().origin(), origin(3.0));
    }

    /// The operator's picture: the CPU start of Guard the Edge at (-199, -8), left of the Fixed camera's view.
    fn picker_marks() -> Marks {
        let mut looks = [None; btt::LOOKS];
        looks[0] = Some(([-199.0, -8.0], EGG_WHOLE));
        looks[btt::LOOKS - 1] = Some(Crosshair::new([-199.0, -8.0]).look());
        Marks { looks, pointer: Some([-199.0, -8.0]) }
    }

    #[test]
    fn the_dotted_pointer_is_not_drawn_while_the_cpu_stands_at_its_point() {
        let marks = picker_marks();
        let shown = marks.shown(true);
        assert_eq!(shown.pointer, None);
        // The crosshair and the start mark are the picker's own and stay.
        assert_eq!(shown.looks, marks.looks);
    }

    #[test]
    fn the_dotted_pointer_stays_when_the_cpu_cannot_be_held() {
        let marks = picker_marks();
        assert_eq!(marks.shown(false), marks);
    }

    #[test]
    fn the_practice_state_says_how_the_cpu_point_is_shown() {
        assert_eq!(cpu_view(None), "");
        let mut slot = None;
        preview_step(&mut slot, 7, || origin(3.0));
        assert_eq!(cpu_view(slot.as_ref()), "in_view");
        slot.as_mut().unwrap().magnified = true;
        assert_eq!(cpu_view(slot.as_ref()), "magnifier");
    }
    }
}
