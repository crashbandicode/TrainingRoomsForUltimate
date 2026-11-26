use super::btt;
use super::panes::{EggLook, EggPicture, EGG_WHOLE};
use super::practice_state::CpuView;

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

/// The game moved a body the hold had put in the air: where it is now, and whether it stands on the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Displaced {
    pub at: [f32; 2],
    pub landed: bool,
}

/// A picker's hold on the CPU: the body itself waits at its start point. In view the game draws it under the
/// crosshair; out of view the game draws its own magnifier of it, with the direction pointer, at the screen edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CpuPreview {
    body: u32,
    origin: Origin,
    frames: u8,
    /// The air point the hold wrote on the frame before.
    wrote: Option<[f32; 2]>,
    /// The caller has asked for the lift again at this point.
    relifted: bool,
    /// The game moved the body off the point written on the frame before, as read at the top of this frame. The
    /// game's physics runs under the hold, so a held body is one fall step under its point on every frame: that
    /// is the hold's own. Moved is the ground, another x, or a y above the point. The callers decide what it means.
    pub displaced: Option<Displaced>,
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

    /// One frame's read of the body, taken before this frame's write. `at`: the point asked for on this frame;
    /// `seen` and `landed`: where the game has the body and whether it stands on the ground; `grounded`: the hold
    /// is a floor stand.
    pub fn observe(&mut self, at: [f32; 2], seen: [f32; 2], landed: bool, grounded: bool) {
        if grounded {
            (self.displaced, self.wrote, self.relifted) = (None, None, false);
            return;
        }
        self.displaced = self.wrote
            .filter(|wrote| landed || seen[0] != wrote[0] || seen[1] > wrote[1])
            .map(|_| Displaced { at: seen, landed });
        if self.wrote != Some(at) {
            self.relifted = false;
        }
        self.wrote = Some(at);
    }

    /// The caller asks for the placing frames again at the point now asked for. Once at a point; `false` when it
    /// has been asked there already.
    pub fn lift_again(&mut self) -> bool {
        if self.relifted {
            return false;
        }
        self.relifted = true;
        self.frames = 0;
        true
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
        *slot = Some(CpuPreview {
            body, origin: origin(), frames: 0, wrote: None, relifted: false, displaced: None, asleep: false, magnified: false,
        });
    }
    slot.as_mut().map_or(PreviewAct::Lift, |preview| preview.act(grounded))
}

/// How the picker's CPU point is shown, for the practice state: the body is there in view, or the game shows its
/// magnifier of it; empty when the body is not there.
pub fn cpu_view(preview: Option<&CpuPreview>) -> CpuView {
    match preview {
        None => CpuView::NotHeld,
        Some(preview) if preview.magnified => CpuView::Magnifier,
        Some(_) => CpuView::InView,
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

    /// A body held in the air at `at` for the settle and one frame more, read one fall step under it each frame.
    fn held_in_air(at: [f32; 2]) -> Option<CpuPreview> {
        let mut slot = None;
        for _ in 0..PREVIEW_SETTLE + 1 {
            hold_step(&mut slot, 7, false, || origin(3.0));
            slot.as_mut().unwrap().observe(at, [at[0], at[1] - 0.087], false, false);
        }
        slot
    }

    /// Eden, run place-float-xps-03 (merge 17): carried down through the long floor at x -40, the body landed at
    /// (-40, 0.01) and stayed there while the crosshair went up to y 29.57.
    #[test]
    fn a_body_the_game_landed_is_reported_and_is_lifted_again_once_at_a_spot() {
        let mut slot = held_in_air([-40.0, 0.573967]);
        let hold = slot.as_mut().unwrap();
        hold.observe([-40.0, -0.426033], [-40.0, 0.486967], false, false);
        assert_eq!(hold.displaced, None, "one fall step under its point is the hold's own");
        hold.observe([-40.0, -1.426033], [-40.0, 0.01], true, false);
        assert_eq!(hold.displaced, Some(Displaced { at: [-40.0, 0.01], landed: true }));
        // The crosshair leaves the ground: the caller asks for the lift, once at this spot.
        hold.observe([-40.0, 3.573967], [-40.0, 0.01], true, false);
        assert!(hold.lift_again());
        assert!(!hold.lift_again(), "a spot is tried once");
        let settle = usize::from(PREVIEW_SETTLE);
        let acts: Vec<_> = (0..settle + 1).map(|_| hold_step(&mut slot, 7, false, || panic!("the same body"))).collect();
        assert!(acts[..settle].iter().all(|act| *act == PreviewAct::Lift));
        assert_eq!(acts[settle], PreviewAct::Hold);
        // In the air again nothing is reported, and another spot may be lifted to again.
        let hold = slot.as_mut().unwrap();
        hold.observe([-40.0, 3.573967], [-40.0, 3.486967], false, false);
        assert_eq!(hold.displaced, None);
        hold.observe([-40.0, 4.573967], [-40.0, 3.486967], false, false);
        assert!(hold.lift_again());
    }

    /// The same run, against the long floor's left wall (x -80) at y -2.43: the body followed to x -82, and read
    /// x -81.08 for an asked -81, still in Fall. SP-10's CPU is the same with a wider body: asked x -81.72, left at
    /// x -86.85; exact at x -86.96.
    #[test]
    fn a_body_a_wall_pushed_is_reported_while_still_in_the_air() {
        let mut slot = held_in_air([-82.0, -2.426033]);
        let hold = slot.as_mut().unwrap();
        hold.observe([-81.0, -2.426033], [-82.0, -2.513033], false, false);
        assert_eq!(hold.displaced, None);
        hold.observe([-80.0, -2.426033], [-81.083, -2.416], false, false);
        assert_eq!(hold.displaced, Some(Displaced { at: [-81.083, -2.416], landed: false }));
        let mut slot = held_in_air([-81.72, -15.10]);
        let hold = slot.as_mut().unwrap();
        hold.observe([-81.72, -15.10], [-86.85, -15.187], false, false);
        assert_eq!(hold.displaced, Some(Displaced { at: [-86.85, -15.187], landed: false }));
        let mut slot = held_in_air([-86.96, -15.10]);
        let hold = slot.as_mut().unwrap();
        hold.observe([-86.96, -15.10], [-86.96, -15.187], false, false);
        assert_eq!(hold.displaced, None);
    }

    /// The 36 Guard the Edge holds of runs place-float-xps-03 and -04: -68.51303 for a picked -68.42603.
    #[test]
    fn a_held_body_one_fall_step_under_its_point_is_not_displaced() {
        let mut slot = held_in_air([-125.0, -68.42603]);
        let hold = slot.as_mut().unwrap();
        hold.observe([-125.0, -68.42603], [-125.0, -68.51303], false, false);
        assert_eq!(hold.displaced, None);
    }

    #[test]
    fn a_first_frame_and_a_floor_stand_report_nothing() {
        let mut slot = None;
        hold_step(&mut slot, 7, false, || origin(3.0));
        let hold = slot.as_mut().unwrap();
        hold.observe([5.0, 9.0], [3.0, 0.0], true, false);
        assert_eq!(hold.displaced, None, "nothing was written before the first frame");
        hold.observe([5.0, 9.0], [5.0, 0.01], true, false);
        assert!(hold.displaced.is_some());
        hold.observe([5.0, 0.0], [5.0, 0.01], true, true);
        assert_eq!(hold.displaced, None, "a floor stand rests where the game puts it");
        // Taken into the air after a stand, the first frame has no written air point to compare with.
        hold.observe([5.0, 9.0], [5.0, 0.01], true, false);
        assert_eq!(hold.displaced, None);
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
        assert_eq!(cpu_view(None), CpuView::NotHeld);
        let mut slot = None;
        preview_step(&mut slot, 7, || origin(3.0));
        assert_eq!(cpu_view(slot.as_ref()), CpuView::InView);
        slot.as_mut().unwrap().magnified = true;
        assert_eq!(cpu_view(slot.as_ref()), CpuView::Magnifier);
    }
    }
}
