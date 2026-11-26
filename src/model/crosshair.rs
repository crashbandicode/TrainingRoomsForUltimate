use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

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

/// A body as the game left it, read at the top of a hold's frame, before that frame's write.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyRead {
    pub at: [f32; 2],
    /// The game has it on the ground.
    pub landed: bool,
    /// Its own speed, read with the position; `None` when that read is not there.
    pub speed: Option<[f32; 2]>,
    /// The game frame of the read (it wraps).
    pub frame: u32,
}

/// The hold's clock: the game's own frames of the body a hold holds, one count for Player 1's body (seat 0) and
/// one for the CPU's (seat 1). The fighter-frame wrapper tells it of every fighter's frame as the game's frame
/// function returns, before anything of ours can return early, so a hold call that was skipped shows as a second
/// frame. The UI facts' frame number is not this: it stands still on a frame whose facts were not published.
#[derive(Default)]
pub struct HoldClock {
    held: [AtomicUsize; 2],
    frames: [AtomicU32; 2],
}

impl HoldClock {
    pub const fn new() -> Self {
        Self { held: [AtomicUsize::new(0), AtomicUsize::new(0)], frames: [AtomicU32::new(0), AtomicU32::new(0)] }
    }

    /// The game ran a frame of the fighter whose body is `body`.
    pub fn ran(&self, _body: usize) {
        self.frames[0].fetch_add(1, Ordering::Relaxed);
    }

    /// The count of `body`'s game frames, kept in `seat`. A body the seat was not counting starts two past every
    /// count given so far, in either seat: no write made before it reads as one frame old.
    pub fn frame(&self, seat: usize, body: usize) -> u32 {
        self.held[seat & 1].store(body, Ordering::Relaxed);
        self.frames[0].load(Ordering::Relaxed)
    }
}

/// A picker's hold on the CPU: the body itself waits at its start point. In view the game draws it under the
/// crosshair; out of view the game draws its own magnifier of it, with the direction pointer, at the screen edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CpuPreview {
    body: u32,
    origin: Origin,
    frames: u8,
    /// The air point the hold last wrote, and the game frame it wrote it on.
    wrote: Option<([f32; 2], u32)>,
    /// The caller has asked for the lift again at this point.
    relifted: bool,
    /// The game moved the body off the air point written on the frame before, as read at the top of this frame.
    /// The game's physics runs under the hold, so a held body reads one fall step under its point on every frame:
    /// that step is its own speed, read with it. Not moved is: in the air, at the x written, at the y written plus
    /// that speed, compared exactly. Anything else is moved. Only a write of the frame before is judged, and only
    /// with the speed read: without either, nothing is reported. The callers decide what a moved body means.
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

    /// One frame of the hold. `at`: the point asked for on this frame; `read`: the body before this frame's write;
    /// `grounded`: the hold is a floor stand, which rests where the game puts it and is not judged.
    pub fn observe(&mut self, at: [f32; 2], read: BodyRead, grounded: bool) {
        if grounded {
            (self.displaced, self.wrote, self.relifted) = (None, None, false);
            return;
        }
        self.displaced = self.wrote
            .filter(|(_, frame)| read.frame == frame.wrapping_add(1))
            .zip(read.speed)
            .filter(|((wrote, _), speed)| read.landed || read.at != [wrote[0], wrote[1] + speed[1]])
            .map(|_| Displaced { at: read.at, landed: read.landed });
        if self.wrote.map(|(wrote, _)| wrote) != Some(at) {
            self.relifted = false;
        }
        self.wrote = Some((at, read.frame));
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

    /// Kirby's fall step under the hold, as the hold's seam read it (Eden, run place-float-xps-05): -0.064.
    const FALL: [f32; 2] = [0.0, f32::from_bits(0xbd83_126f)];

    fn air(at: [f32; 2], frame: u32) -> BodyRead {
        BodyRead { at, landed: false, speed: Some(FALL), frame }
    }

    /// A body held in the air at `at` through the settle: the hold with its last write on frame `PREVIEW_SETTLE`.
    fn held_in_air(at: [f32; 2]) -> Option<CpuPreview> {
        let mut slot = None;
        for frame in 0..=u32::from(PREVIEW_SETTLE) {
            hold_step(&mut slot, 7, false, || origin(3.0));
            slot.as_mut().unwrap().observe(at, air([at[0], at[1] + FALL[1]], frame), false);
        }
        slot
    }
    const NEXT: u32 = PREVIEW_SETTLE as u32 + 1;

    /// The seam's words in open air: asked (-125, -68.42603), read (-125, -68.490036), speed (0, -0.064). The
    /// read is the point asked on the frame before plus the speed, bit for bit, on a moving point too.
    #[test]
    fn a_held_body_reads_its_point_plus_its_own_fall_step_exactly() {
        let y = |bits: u32| f32::from_bits(bits);
        let mut slot = held_in_air([-125.0, y(0xc288_da21)]);
        let hold = slot.as_mut().unwrap();
        hold.observe([-125.0, y(0xc284_da21)], air([-125.0, y(0xc288_fae6)], NEXT), false);
        assert_eq!(hold.displaced, None);
        // The crosshair went up two units: the body reads the point of the frame before, not this frame's.
        hold.observe([-125.0, y(0xc280_da21)], air([-125.0, y(0xc284_fae6)], NEXT + 1), false);
        assert_eq!(hold.displaced, None);
        hold.observe([-123.0, y(0xc280_da21)], air([-125.0, y(0xc280_da21) + FALL[1]], NEXT + 2), false);
        assert_eq!(hold.displaced, None);
        hold.observe([-123.0, y(0xc280_da21)], air([-123.0, y(0xc280_da21) + FALL[1]], NEXT + 3), false);
        assert_eq!(hold.displaced, None);
    }

    /// The same run: carried down through the long floor at x -40, the body landed at (-40.1, 0.01).
    #[test]
    fn a_body_the_game_landed_is_reported_and_is_lifted_again_once_at_a_spot() {
        let mut slot = held_in_air([-40.100_18, 0.573_967]);
        let hold = slot.as_mut().unwrap();
        let landed = |frame| BodyRead { at: [-40.100_18, 0.01], landed: true, speed: Some([0.0; 2]), frame };
        hold.observe([-40.100_18, -0.426_033], landed(NEXT), false);
        assert_eq!(hold.displaced, Some(Displaced { at: [-40.100_18, 0.01], landed: true }));
        // The crosshair leaves the ground: the caller asks for the lift, once at this spot.
        hold.observe([-40.100_18, 3.573_967], landed(NEXT + 1), false);
        assert!(hold.lift_again());
        assert!(!hold.lift_again(), "a spot is tried once");
        let settle = usize::from(PREVIEW_SETTLE);
        let acts: Vec<_> = (0..settle + 1).map(|_| hold_step(&mut slot, 7, false, || panic!("the same body"))).collect();
        assert!(acts[..settle].iter().all(|act| *act == PreviewAct::Lift));
        assert_eq!(acts[settle], PreviewAct::Hold);
        // In the air again nothing is reported, and another spot may be lifted to again.
        let hold = slot.as_mut().unwrap();
        hold.observe([-40.100_18, 4.573_967], air([-40.100_18, 3.573_967 + FALL[1]], NEXT + 2), false);
        assert_eq!(hold.displaced, None);
        assert!(hold.lift_again());
    }

    /// The same run, at the long floor's left wall (x -80) and its edge, asked y -3.5262246: the words read.
    #[test]
    fn a_body_a_wall_or_a_floor_edge_pushed_is_reported() {
        let y = f32::from_bits(0xc061_adaa);
        let pushed = |asked: [f32; 2], read: BodyRead| {
            let mut slot = held_in_air(asked);
            let hold = slot.as_mut().unwrap();
            hold.observe(asked, read, false);
            hold.displaced
        };
        // Asked x -81: left at x -81.55, still in the air.
        let wall = [f32::from_bits(0xc2a3_1bfc), f32::from_bits(0xc065_2266)];
        assert_eq!(pushed([-81.0, y], air(wall, NEXT)), Some(Displaced { at: wall, landed: false }));
        // Asked x -79: pushed up at the floor's edge on one frame, then put on the floor.
        let up = [-79.0, f32::from_bits(0xbd5d_2f1c)];
        assert_eq!(pushed([-79.0, y], air(up, NEXT)), Some(Displaced { at: up, landed: false }));
        let floor = BodyRead { at: [-79.0, 0.01], landed: true, speed: Some([0.0; 2]), frame: NEXT };
        assert_eq!(pushed([-79.0, y], floor), Some(Displaced { at: [-79.0, 0.01], landed: true }));
        // Asked x -82: free of the wall.
        assert_eq!(pushed([-82.0, y], air([-82.0, y + FALL[1]], NEXT)), None);
        // A body the game threw out of a solid: its speed carries the push, so the sum is exact; its x is not.
        let thrown = BodyRead { at: [-85.852_455, -9.132_209], landed: false, speed: Some([-45.752_274, -0.706_175_8]), frame: NEXT };
        assert_eq!(pushed([-40.100_18, -8.426_033], thrown), Some(Displaced { at: thrown.at, landed: false }));
    }

    /// Not read in a game yet: a body kept down by a ceiling is under the point by more than its own fall step.
    #[test]
    fn a_body_pushed_down_is_reported_though_it_is_under_its_point() {
        let mut slot = held_in_air([-250.0, -45.0]);
        let hold = slot.as_mut().unwrap();
        hold.observe([-250.0, -45.0], air([-250.0, -46.2], NEXT), false);
        assert_eq!(hold.displaced, Some(Displaced { at: [-250.0, -46.2], landed: false }));
    }

    /// A hold call the picker skipped: the write is two game frames old and the body fell twice. A frame without
    /// the speed read. Neither is judged, whatever the body reads; the next frame is.
    #[test]
    fn a_stale_write_or_a_missing_speed_read_is_not_judged() {
        let at = [-125.0, -68.426_03];
        let mut slot = held_in_air(at);
        let hold = slot.as_mut().unwrap();
        let landed = |speed, frame| BodyRead { at: [-125.0, 0.01], landed: true, speed, frame };
        hold.observe(at, landed(Some([0.0; 2]), NEXT + 1), false);
        assert_eq!(hold.displaced, None, "the write was two frames old");
        hold.observe(at, landed(None, NEXT + 2), false);
        assert_eq!(hold.displaced, None, "no speed was read");
        hold.observe(at, landed(Some([0.0; 2]), NEXT + 3), false);
        assert_eq!(hold.displaced, Some(Displaced { at: [-125.0, 0.01], landed: true }));
    }

    /// The seam's words on a floor: asked (x, 0), read (x, 0.01), every frame. A floor stand is not an air place.
    #[test]
    fn a_first_frame_and_a_floor_stand_report_nothing() {
        let mut slot = None;
        hold_step(&mut slot, 7, false, || origin(3.0));
        let hold = slot.as_mut().unwrap();
        let stood = |frame| BodyRead { at: [-30.0, 0.01], landed: true, speed: Some([0.0; 2]), frame };
        hold.observe([5.0, 9.0], stood(0), false);
        assert_eq!(hold.displaced, None, "nothing was written before the first frame");
        hold.observe([5.0, 9.0], stood(1), false);
        assert!(hold.displaced.is_some());
        hold.observe([-30.0, 0.0], stood(2), true);
        assert_eq!(hold.displaced, None, "a floor stand rests where the game puts it");
        hold.observe([-30.0, 0.0], stood(3), true);
        assert_eq!(hold.displaced, None);
        // Taken into the air after a stand, the first frame has no written air point to compare with.
        hold.observe([5.0, 9.0], stood(4), false);
        assert_eq!(hold.displaced, None);
    }

    const KIRBY: usize = 0x7100_4000;
    const OTHER: usize = 0x7100_8000;

    /// rv-edge-juggle-sol, 2026-11-26 14:58:37Z: the UI facts' frame number stands still on a game frame whose
    /// facts were not published, so a hold call skipped on one frame and facts not published on the next read as
    /// one frame. The clock counts the game's frames of the very body the hold holds, and of no other fighter
    /// (Nana runs under Popo's entry).
    #[test]
    fn the_holds_clock_counts_the_game_frames_of_the_body_it_holds() {
        let clock = HoldClock::new();
        let at = [-125.0, -68.426_03];
        let mut slot = None;
        hold_step(&mut slot, 7, false, || origin(3.0));
        let hold = slot.as_mut().unwrap();
        let landed = |frame| BodyRead { at: [-125.0, 0.01], landed: true, speed: Some([0.0; 2]), frame };
        let moved = Some(Displaced { at: [-125.0, 0.01], landed: true });
        let wrote = clock.frame(1, KIRBY);
        hold.observe(at, air(at, wrote), false);
        // One game frame: the game runs each fighter once.
        clock.ran(OTHER);
        clock.ran(KIRBY);
        assert_eq!(clock.frame(1, KIRBY), wrote + 1);
        hold.observe(at, landed(wrote + 1), false);
        assert_eq!(hold.displaced, moved, "one game frame after the write");
        // The hold's caller returned early on the next frame: the game ran the body twice since the last write.
        for _ in 0..2 {
            clock.ran(OTHER);
            clock.ran(KIRBY);
        }
        assert_eq!(clock.frame(1, KIRBY), wrote + 3);
        hold.observe(at, landed(wrote + 3), false);
        assert_eq!(hold.displaced, None, "two game frames after the write");
        // The game did not run the body (only the other fighter): the count stands, and nothing is judged.
        clock.ran(OTHER);
        assert_eq!(clock.frame(1, KIRBY), wrote + 3);
        hold.observe(at, landed(wrote + 3), false);
        assert_eq!(hold.displaced, None, "no game frame after the write");
        clock.ran(KIRBY);
        hold.observe(at, landed(clock.frame(1, KIRBY)), false);
        assert_eq!(hold.displaced, moved);
    }

    /// A body the clock was not counting: its frames since a write are not known, so its first count is past
    /// every count given before, in either seat, by more than one.
    #[test]
    fn a_body_the_clock_starts_to_count_is_never_one_frame_after_an_earlier_write() {
        let clock = HoldClock::new();
        clock.frame(0, KIRBY);
        clock.ran(KIRBY);
        let wrote = clock.frame(0, KIRBY);
        // Another body is read in the other seat, then this one: its frames were not counted there.
        let given = clock.frame(1, OTHER).max(wrote);
        clock.ran(KIRBY);
        let taken = clock.frame(1, KIRBY);
        assert!(taken > given + 1, "counts up to {given} were given before {taken}");
        // Back in its own seat, after another body sat there for one call.
        clock.frame(0, OTHER);
        clock.ran(KIRBY);
        let back = clock.frame(0, KIRBY);
        assert!(back > taken + 1, "{taken} was given before {back}");
        // From there the count is the body's own game frames again.
        clock.ran(KIRBY);
        assert_eq!(clock.frame(0, KIRBY), back + 1);
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
