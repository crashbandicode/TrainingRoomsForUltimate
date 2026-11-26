//! Where the fighters were on a clip's frames, for the clip editor's scrub (operator 2026-11-25): a selected In or Out
//! handle holds the fighter at the place and facing it had on that handle's frame. A match capture has those values in
//! its own frames; a saved macro gets them from its X preview, sampled on the game thread as each input is fed.
//!
//! The block lives on the heap while an editor is open. The game side reaches it through `Slot`, a published pointer
//! with a reader count, and never through a lock the worker holds.
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use crate::model::crosshair::CpuPreview;

/// Frames of a preview that are remembered: 60 s. A longer window keeps its first 60 s.
pub const MAX_TRACK_FRAMES: u32 = 3_600;
const CELLS: usize = 6;
const NO_START: u32 = u32::MAX;

/// A fighter's place and facing on one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub at: [f32; 2],
    pub facing: f32,
}

/// What the editor asks the fighters to show.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Ask {
    /// Nothing is held; a held body goes back.
    #[default]
    None,
    /// A match capture: the role in `slot` (0 Player 1, 1 the CPU) stands `delta` away from where the hold found it.
    Relative { slot: u8, delta: [f32; 2], facing: f32 },
    /// A saved macro: both fighters at sample `index` of the remembered track.
    Sampled { index: u32 },
}

/// What the editor's key line says about positions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pos {
    /// Nothing to say: no handle is selected, a preview plays, or the clip has no positions (a combo, a drill).
    #[default]
    None,
    /// A saved macro with no track yet.
    Missing,
    /// A saved macro whose track is from another In, or does not reach the handle's frame.
    Stale,
    /// The fighter is shown at the handle's frame.
    Held,
    /// The point is outside this stage's camera limits: nothing moves.
    Off,
}

impl Pos {
    /// The key line's text; empty when there is nothing to say.
    pub const fn text(self) -> &'static str {
        match self {
            Pos::Missing => "X to remember positions.",
            Pos::Stale => "X again to update positions.",
            Pos::Off => "Off this stage.",
            Pos::None | Pos::Held => "",
        }
    }

    /// The bridge's word for the state.
    pub const fn word(self) -> &'static str {
        match self {
            Pos::None => "none",
            Pos::Missing => "missing",
            Pos::Stale => "stale",
            Pos::Held => "held",
            Pos::Off => "off",
        }
    }
}

/// What the game side did with a role's body on its last frame.
pub const SEEN_IDLE: u32 = 0;
pub const SEEN_HELD: u32 = 1;
pub const SEEN_OFF: u32 = 2;

/// Whether a point may hold a body: inside the camera's limits `[left, right, top, bottom]`. A body held past them
/// is on its way to a blast line.
pub fn in_bounds(at: [f32; 2], limits: [f32; 4]) -> bool {
    let _ = (at, limits);
    true
}

/// One editor's scrub state and position track.
pub struct Block {
    ask: hos_lock::Published<Ask>,
    /// The role whose fed frames are sampled, plus one; zero while none is.
    armed: AtomicU32,
    /// The In the track was previewed with.
    start: AtomicU32,
    len: AtomicU32,
    /// Six `f32` bit patterns per frame: Player 1's x, y and facing, then the CPU's.
    cells: Box<[AtomicU32]>,
    seen: [AtomicU32; 2],
    /// Each role's hold, as the placement picker keeps one. The game thread only, with `try_lock`.
    pub holds: hos_lock::Mutex<[Option<CpuPreview>; 2]>,
}

impl Block {
    /// A block that can remember `frames` frames (at most `MAX_TRACK_FRAMES`); none for a clip with no preview.
    pub fn new(frames: u32) -> Box<Self> {
        let cells = (0..frames.min(MAX_TRACK_FRAMES) as usize * CELLS).map(|_| AtomicU32::new(0)).collect();
        Box::new(Self {
            ask: hos_lock::Published::new(Ask::None),
            armed: AtomicU32::new(0),
            start: AtomicU32::new(NO_START),
            len: AtomicU32::new(0),
            cells,
            seen: [const { AtomicU32::new(SEEN_IDLE) }; 2],
            holds: hos_lock::Mutex::new([None; 2]),
        })
    }

    pub fn ask(&self) -> Ask {
        self.ask.read()
    }

    /// The worker: what the fighters show from the next frame on.
    pub fn set_ask(&self, ask: Ask) {
        if self.ask.read() != ask {
            self.ask.publish(ask);
        }
    }

    /// The worker, as a saved macro's preview is asked for: the frames fed to `slot` from now on are the track of
    /// the window that starts at `start`. The earlier track is dropped.
    pub fn arm(&self, slot: usize, start: u32) {
        let _ = (slot, start);
    }

    /// The worker, when the preview is over: no further frame is sampled.
    pub fn disarm(&self) {
        self.armed.store(0, Ordering::Release);
    }

    /// The game thread, as `slot` is fed the `fed`-th frame of its run (one-based): both fighters' places then.
    pub fn sample(&self, slot: usize, fed: u32, spots: [Spot; 2]) {
        let _ = (slot, fed, spots);
    }

    /// The In the track was previewed with and the frames it holds; none before a preview fed a frame.
    pub fn track(&self) -> Option<(u32, u32)> {
        None
    }

    /// Both fighters' places on sample `index`.
    pub fn sampled(&self, index: u32) -> Option<[Spot; 2]> {
        let _ = index;
        None
    }

    /// Where the role in `slot` is held, `home` being where the hold found it; none when it is not held.
    pub fn target(&self, slot: usize, home: [f32; 2]) -> Option<Spot> {
        let _ = (slot, home);
        None
    }

    pub fn seen(&self, slot: usize) -> u32 {
        self.seen.get(slot).map_or(SEEN_IDLE, |seen| seen.load(Ordering::Acquire))
    }

    /// The game thread: what it did with the role's body this frame.
    pub fn set_seen(&self, slot: usize, seen: u32) {
        if let Some(cell) = self.seen.get(slot) {
            cell.store(seen, Ordering::Release);
        }
    }
}

const NO_BLOCK: usize = 1;
const NO_READER: usize = 1;

/// The published block. Both words start non-zero, so the static is initialised data and not zero-filled memory.
pub struct Slot {
    /// `NO_BLOCK`, a live block's address (even), or a retired block's address with bit 0 set.
    ptr: AtomicUsize,
    /// Readers inside `with`, plus `NO_READER`.
    readers: AtomicUsize,
}

impl Default for Slot {
    fn default() -> Self {
        Self::new()
    }
}

impl Slot {
    pub const fn new() -> Self {
        Self { ptr: AtomicUsize::new(NO_BLOCK), readers: AtomicUsize::new(NO_READER) }
    }

    /// Whether a block is published.
    pub fn live(&self) -> bool {
        self.ptr.load(Ordering::SeqCst) & 1 == 0
    }

    /// Whether nothing is published and nothing waits to be freed.
    pub fn empty(&self) -> bool {
        self.ptr.load(Ordering::SeqCst) == NO_BLOCK
    }

    /// The worker: publish a block. Refused, and the block handed back, while an earlier one is live or not yet freed.
    pub fn publish(&self, block: Box<Block>) -> Result<(), Box<Block>> {
        if !self.empty() {
            return Err(block);
        }
        self.ptr.store(Box::into_raw(block) as usize, Ordering::SeqCst);
        Ok(())
    }

    /// Any thread: run `f` on the live block. The reader is counted before the pointer is read, so a block is never
    /// freed under it. Never waits.
    pub fn with<R>(&self, f: impl FnOnce(&Block) -> R) -> Option<R> {
        self.readers.fetch_add(1, Ordering::SeqCst);
        let ptr = self.ptr.load(Ordering::SeqCst);
        // SAFETY: an even word is a block made by `publish`; `reap` frees it only after `retire` made the word odd
        // and the count read `NO_READER`, which this reader's count, taken before the load, prevents.
        let result = (ptr & 1 == 0).then(|| f(unsafe { &*(ptr as *const Block) }));
        self.readers.fetch_sub(1, Ordering::SeqCst);
        result
    }

    /// The worker: no new reader finds the block. It is freed by `reap`.
    pub fn retire(&self) {
        let ptr = self.ptr.load(Ordering::SeqCst);
        if ptr & 1 == 0 {
            self.ptr.store(ptr | 1, Ordering::SeqCst);
        }
    }

    /// The worker: free a retired block once no reader is inside `with`. True when nothing is left to free.
    pub fn reap(&self) -> bool {
        let ptr = self.ptr.load(Ordering::SeqCst);
        if ptr & 1 == 1 && ptr != NO_BLOCK {
            self.ptr.store(NO_BLOCK, Ordering::SeqCst);
        }
        self.ptr.load(Ordering::SeqCst) == NO_BLOCK
    }
}

/// The one published block of the clip editor.
pub static SLOT: Slot = Slot::new();

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::macros::{InputFrame, MacroRecording, ReplayFrame, ReplaySession, ReplayTrack, StartState};
    use crate::engine::macro_ui::clip_panel::{clip_help, ClipAction, ClipEditor, ClipEffect};

    fn spot(x: f32, y: f32, facing: f32) -> Spot {
        Spot { at: [x, y], facing }
    }

    /// A ten-frame capture: x is 20, 23, 26, ... and y climbs by one; facing turns left on frame 6.
    fn capture() -> ClipEditor {
        let frames = (0..10).map(|i| ReplayFrame {
            input: InputFrame { buttons: i, ..InputFrame::default() },
            start: StartState { status: 14, facing: if i >= 6 { -1.0 } else { 1.0 }, ..StartState::neutral(18, 0, 1.0) },
            x: 20.0 + 3.0 * i as f32,
            y: i as f32,
        }).collect();
        ClipEditor::new(ReplaySession { tracks: vec![ReplayTrack { fighter: None, entry: 1, frames, quantized_analog: false }],
            truncated: false, breaks: Vec::new() }, vec![14]).unwrap()
    }

    fn saved_macro(player: bool) -> ClipEditor {
        let recording = MacroRecording::new("Recorded", "13.0.5", StartState::neutral(18, 0, 1.0),
            (0..10).map(|i| InputFrame { buttons: i, ..InputFrame::default() }).collect()).unwrap();
        ClipEditor::macro_file(recording, player).unwrap()
    }

    /// A block whose track is `frames` fed frames of the window that starts at `start`: Player 1 walks right from
    /// x 100, the CPU stands at (-50, 0) facing left.
    fn tracked(start: u32, frames: u32) -> Box<Block> {
        let block = Block::new(10);
        block.arm(0, start);
        for fed in 1..=frames {
            block.sample(0, fed, [spot(100.0 + fed as f32, 5.0, 1.0), spot(-50.0, 0.0, -1.0)]);
        }
        block
    }

    #[test]
    fn the_track_keeps_one_sample_per_fed_frame_and_stops_at_its_cap() {
        let block = Block::new(4);
        assert_eq!(block.track(), None);
        // Frames fed before a preview is armed, and frames fed to the other role, are not a track.
        block.sample(0, 1, [spot(1.0, 1.0, 1.0); 2]);
        block.arm(1, 30);
        block.sample(0, 1, [spot(1.0, 1.0, 1.0); 2]);
        assert_eq!(block.track(), None);
        for fed in 1..=6 {
            block.sample(1, fed, [spot(fed as f32, 2.0, 1.0), spot(-(fed as f32), 3.0, -1.0)]);
        }
        // Four frames fit; the fifth and sixth are not kept and nothing is written past the end.
        assert_eq!(block.track(), Some((30, 4)));
        assert_eq!(block.sampled(0), Some([spot(1.0, 2.0, 1.0), spot(-1.0, 3.0, -1.0)]));
        assert_eq!(block.sampled(3), Some([spot(4.0, 2.0, 1.0), spot(-4.0, 3.0, -1.0)]));
        assert_eq!(block.sampled(4), None);
        // A new run of the same preview starts the track again from its first frame.
        block.sample(1, 1, [spot(9.0, 9.0, 1.0); 2]);
        assert_eq!(block.track(), Some((30, 1)));
        assert_eq!(block.sampled(1), None);
        // Arming again drops the track; a disarmed block samples nothing.
        block.arm(1, 31);
        assert_eq!(block.track(), None);
        block.disarm();
        block.sample(1, 1, [spot(9.0, 9.0, 1.0); 2]);
        assert_eq!(block.track(), None);
        assert_eq!(Block::new(100_000).cells.len(), MAX_TRACK_FRAMES as usize * CELLS);
        // The sample is taken where the preview credits a fed frame, from the poses the fighter frames publish.
        let host = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/model/rr_host.rs")).unwrap();
        let credit = host.split_once("fn credit_frame(").unwrap().1.split_once("\n}").unwrap().0;
        assert!(credit.contains("SLOT.with(|block| block.sample(role_index(role), next, [live_spot(0), live_spot(1)]));"));
    }

    #[test]
    fn a_macro_has_no_position_before_its_first_preview() {
        for player in [false, true] {
            let mut editor = saved_macro(player);
            assert_eq!(editor.scrub(None), (Ask::None, Pos::None), "no handle is selected");
            editor.on_action(ClipAction::SelectIn);
            assert_eq!(editor.scrub(None), (Ask::None, Pos::Missing));
            editor.on_action(ClipAction::Nudge(3));
            assert_eq!(editor.scrub(None), (Ask::None, Pos::Missing));
            editor.on_action(ClipAction::SelectOut);
            assert_eq!(editor.scrub(None), (Ask::None, Pos::Missing));
        }
        // A block with no track holds nobody, whatever is asked of it.
        let block = Block::new(10);
        block.set_ask(Ask::Sampled { index: 0 });
        assert_eq!((block.target(0, [0.0, 0.0]), block.target(1, [0.0, 0.0])), (None, None));
    }

    #[test]
    fn a_macro_handle_inside_the_previewed_window_shows_the_sampled_point() {
        let mut editor = saved_macro(true);
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(2));
        // X previewed In 2 .. Out 10: eight frames.
        let block = tracked(2, 8);
        let track = block.track();
        assert_eq!(editor.scrub(track), (Ask::Sampled { index: 0 }, Pos::Held), "In is the track's first frame");
        editor.on_action(ClipAction::SelectOut);
        assert_eq!(editor.scrub(track), (Ask::Sampled { index: 7 }, Pos::Held), "Out shows the last kept frame");
        editor.on_action(ClipAction::Nudge(-3));
        let (ask, pos) = editor.scrub(track);
        assert_eq!((ask, pos), (Ask::Sampled { index: 4 }, Pos::Held));
        // Both fighters stand where they were on that frame, wherever they stand now.
        block.set_ask(ask);
        assert_eq!(block.target(0, [777.0, 777.0]), Some(spot(105.0, 5.0, 1.0)));
        assert_eq!(block.target(1, [777.0, 777.0]), Some(spot(-50.0, 0.0, -1.0)));
        block.set_ask(Ask::None);
        assert_eq!((block.target(0, [0.0, 0.0]), block.target(1, [0.0, 0.0])), (None, None));
    }

    #[test]
    fn a_macro_track_is_stale_after_in_moves() {
        let mut editor = saved_macro(false);
        let block = tracked(0, 10);
        editor.on_action(ClipAction::SelectIn);
        assert_eq!(editor.scrub(block.track()), (Ask::Sampled { index: 0 }, Pos::Held));
        // In moves: every later position of a preview from the start save would differ.
        editor.on_action(ClipAction::Nudge(1));
        assert_eq!(editor.scrub(block.track()), (Ask::None, Pos::Stale));
        editor.on_action(ClipAction::SelectOut);
        assert_eq!(editor.scrub(block.track()), (Ask::None, Pos::Stale), "Out too: the track is another In's");
        // Back on the previewed In the track holds again.
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(-1));
        assert_eq!(editor.scrub(block.track()), (Ask::Sampled { index: 0 }, Pos::Held));
        // A frame the preview did not reach has no sample: a short track, then Out past it.
        let short = tracked(0, 4);
        editor.on_action(ClipAction::SelectOut);
        assert_eq!(editor.scrub(short.track()), (Ask::None, Pos::Stale));
        editor.on_action(ClipAction::Nudge(-6));
        assert_eq!(editor.scrub(short.track()), (Ask::Sampled { index: 3 }, Pos::Held));
    }

    #[test]
    fn the_key_line_names_a_missing_or_stale_track() {
        let help = |editor: &ClipEditor| clip_help(&editor.view(), "ZL", "ZL", 0).notice;
        let mut editor = saved_macro(true);
        assert_eq!(help(&editor), "", "no handle, nothing about positions");
        editor.on_action(ClipAction::SelectIn);
        editor.note_scrub(Pos::Missing);
        assert_eq!(help(&editor), "X to remember positions.");
        editor.note_scrub(Pos::Stale);
        assert_eq!(help(&editor), "X again to update positions.");
        editor.note_scrub(Pos::Off);
        assert_eq!(help(&editor), "Off this stage.");
        editor.note_scrub(Pos::Held);
        assert_eq!(help(&editor), "");
        assert_eq!(editor.view().pos, Pos::Held);
        for pos in [Pos::None, Pos::Missing, Pos::Stale, Pos::Held, Pos::Off] {
            assert!(pos.text().chars().count() <= 67, "{pos:?}");
        }
        assert_eq!([Pos::None, Pos::Missing, Pos::Stale, Pos::Held, Pos::Off].map(Pos::word), ["none", "missing", "stale", "held", "off"]);
    }

    #[test]
    fn a_replay_handle_shows_the_captured_offset_from_the_anchor() {
        let mut editor = capture();
        editor.on_action(ClipAction::SelectIn);
        // On the anchor itself the fighter stays where it stands.
        assert_eq!(editor.scrub(None), (Ask::Relative { slot: 1, delta: [0.0, 0.0], facing: 1.0 }, Pos::Held));
        editor.on_action(ClipAction::Nudge(5));
        let (ask, pos) = editor.scrub(None);
        assert_eq!((ask, pos), (Ask::Relative { slot: 1, delta: [15.0, 5.0], facing: 1.0 }, Pos::Held));
        editor.on_action(ClipAction::SelectOut);
        assert_eq!(editor.scrub(None).0, Ask::Relative { slot: 1, delta: [27.0, 9.0], facing: -1.0 }, "Out is frame 9, facing left");
        // The play target's fighter moves, from where the hold found it; the other fighter is not touched.
        let block = Block::new(0);
        block.set_ask(ask);
        assert_eq!(block.target(1, [10.0, 0.0]), Some(spot(25.0, 5.0, 1.0)));
        assert_eq!(block.target(0, [10.0, 0.0]), None);
        editor.on_action(ClipAction::SwapTarget);
        editor.on_action(ClipAction::SelectIn);
        assert!(matches!(editor.scrub(None).0, Ask::Relative { slot: 0, .. }), "the play target is Player 1 now");
    }

    #[test]
    fn the_anchor_moves_only_at_a_preview_or_a_new_track() {
        // rv-preview-vis 2026-11-26 00:07:47Z: home x 10, the anchor's sample 20, In moved to a sample of 35 shows 25.
        let mut editor = capture();
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(5));
        let (ask, _) = editor.scrub(None);
        let block = Block::new(0);
        block.set_ask(ask);
        assert_eq!(block.target(1, [10.0, 0.0]).map(|spot| spot.at[0]), Some(25.0));
        // Moving In again does not rebase: the body follows the handle.
        editor.on_action(ClipAction::Nudge(1));
        assert_eq!(editor.scrub(None).0, Ask::Relative { slot: 1, delta: [18.0, 6.0], facing: -1.0 });
        // X moves the anchor to that In.
        assert!(matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::Preview(_)));
        assert_eq!(editor.scrub(None), (Ask::None, Pos::None), "a playing preview holds nothing");
        editor.pause();
        editor.on_action(ClipAction::SelectIn);
        assert_eq!(editor.scrub(None).0, Ask::Relative { slot: 1, delta: [0.0, 0.0], facing: -1.0 });
        editor.on_action(ClipAction::SelectOut);
        assert_eq!(editor.scrub(None).0, Ask::Relative { slot: 1, delta: [9.0, 3.0], facing: -1.0 });
    }

    #[test]
    fn a_combo_crop_never_shows_a_position() {
        let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/model/macro_ui/clip_panel.rs")).unwrap();
        let scrub = source.split_once("pub fn scrub(").expect("the editor's scrub").1.split_once("\n    }\n").unwrap().0;
        assert!(scrub.contains("Some(FileSource::Combo(_)) => (Ask::None, Pos::None),"));
        // A block made for a clip with no preview remembers nothing.
        let block = Block::new(0);
        block.arm(0, 0);
        block.sample(0, 1, [spot(1.0, 1.0, 1.0); 2]);
        assert_eq!(block.track(), None);
    }

    #[test]
    fn only_a_selected_handle_holds_and_a_preview_releases() {
        let mut editor = capture();
        assert_eq!(editor.scrub(None), (Ask::None, Pos::None));
        editor.on_action(ClipAction::SelectOut);
        assert!(matches!(editor.scrub(None), (Ask::Relative { .. }, Pos::Held)));
        // R again deselects the handle.
        editor.on_action(ClipAction::SelectOut);
        assert_eq!(editor.scrub(None), (Ask::None, Pos::None));
        editor.on_action(ClipAction::SelectIn);
        assert!(matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::Preview(_)));
        assert_eq!(editor.scrub(None), (Ask::None, Pos::None));
        // The game side lets a body go as soon as nothing is asked of its role, and when the block is gone.
        let runtime = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/features/scrub_hold.rs")).unwrap_or_default();
        assert!(runtime.contains("None => release(block, slot, accessor),"), "a role with no target is released");
        assert!(runtime.contains("placement::restore(accessor, hold.origin());"));
    }

    #[test]
    fn a_point_outside_the_stage_bounds_is_not_shown() {
        // Camera limits as the game gives them: left, right, top, bottom.
        let limits = [-170.0, 170.0, 130.0, -80.0];
        for (at, inside) in [
            ([0.0, 0.0], true), ([-169.0, 129.0], true), ([169.0, -79.0], true),
            ([-171.0, 0.0], false), ([171.0, 0.0], false), ([0.0, 131.0], false), ([0.0, -81.0], false),
            ([f32::NAN, 0.0], false), ([0.0, f32::INFINITY], false),
        ] {
            assert_eq!(in_bounds(at, limits), inside, "{at:?}");
        }
        assert!(!in_bounds([0.0, 0.0], [f32::NAN, 170.0, 130.0, -80.0]));
        let runtime = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/features/scrub_hold.rs")).unwrap_or_default();
        assert!(runtime.contains("if !in_bounds(spot.at, limits) {"));
        assert!(runtime.contains("block.set_seen(slot, SEEN_OFF);"));
    }

    #[test]
    fn a_retired_track_is_freed_only_with_no_reader() {
        let slot = Slot::new();
        assert!(slot.empty() && !slot.live());
        assert_eq!(slot.with(|_| ()), None);
        assert!(slot.publish(Block::new(8)).is_ok());
        assert!(slot.live());
        // A second block is refused while the first is there.
        assert!(slot.publish(Block::new(8)).is_err());
        assert_eq!(slot.with(|block| block.cells.len()), Some(8 * CELLS));
        // A reader that is inside when the block is retired keeps it alive; a new reader finds nothing.
        slot.with(|block| {
            slot.retire();
            assert!(!slot.live());
            assert_eq!(slot.with(|_| ()), None);
            assert!(!slot.reap(), "a reader is inside");
            assert!(!slot.empty());
            assert!(slot.publish(Block::new(1)).is_err(), "not before the old block is freed");
            assert_eq!(block.cells.len(), 8 * CELLS);
        });
        assert!(slot.reap());
        assert!(slot.empty());
        assert!(slot.reap(), "nothing left to free");
        assert!(slot.publish(Block::new(2)).is_ok());
        slot.retire();
        assert!(slot.reap());
        // The static is initialised data: both words start non-zero.
        let source = include_str!("scrub_track.rs");
        assert!(source.contains("const NO_BLOCK: usize = 1;") && source.contains("const NO_READER: usize = 1;"));
        assert!(source.contains("Self { ptr: AtomicUsize::new(NO_BLOCK), readers: AtomicUsize::new(NO_READER) }"));
    }

    #[test]
    fn the_hold_runs_only_in_training_and_room_practice() {
        let root = env!("CARGO_MANIFEST_DIR");
        let runtime = std::fs::read_to_string(format!("{root}/src/runtime/features/scrub_hold.rs")).unwrap_or_default();
        let frame = runtime.split_once("pub(super) unsafe fn frame(").map_or("", |(_, rest)| rest);
        // The same practice test the preview's checkpoint uses: Training and the waiting room's practice; a match has none.
        let gate = frame.find("if !practice::enabled() || !practice::owned() {").expect("the practice gate");
        let read = frame.find("SLOT.with(").expect("the block is read through the slot");
        assert!(gate < read, "nothing is read outside a practice");
        assert!(!runtime.contains(".lock()"), "the fighter frame takes no blocking lock");
        assert!(!runtime.contains("Scene::") && !runtime.contains("scene::"), "one path for Training and the room");
        let states = std::fs::read_to_string(format!("{root}/src/runtime/features/states.rs")).unwrap();
        assert_eq!(states.matches("super::scrub_hold::frame(role.index(), accessor);").count(), 1);
    }
    }
}
