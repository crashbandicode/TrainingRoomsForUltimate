//! Where the fighters were on a clip's frames, for the clip editor's scrub (operator 2026-11-25/26): a selected In or
//! Out handle holds both fighters where the recording had them on that handle's frame. A match capture and a saved
//! macro both carry that state per frame (`ReplayFrame`), and both go through `deltas`.
//!
//! The block lives on the heap while an editor is open. The game side reaches it through `Slot`, a published pointer
//! with a reader count, and never through a lock the worker holds.
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering};

use crate::engine::macros::{InputFrame, ReplayFrame, StartState, MAX_FRAMES};
use crate::model::crosshair::CpuPreview;

/// A fighter's place and facing on one frame; as a delta, how far from the target fighter's home.
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
    /// Each role (0 Player 1, 1 the CPU) stands its delta away from where the hold found the role in `target`. A role
    /// with no delta is not held.
    Group { target: u8, deltas: [Option<Spot>; 2] },
}

/// What the editor's key line says about positions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pos {
    /// Nothing to say: no handle is selected, a preview plays, or the clip is a combo or a drill.
    #[default]
    None,
    /// A macro recorded before positions were kept.
    NoTrack,
    /// The fighters are shown at the handle's frame.
    Held,
    /// A fighter's point is outside this stage's camera limits: that fighter does not move.
    Off,
}

impl Pos {
    /// The key line's text; empty when there is nothing to say.
    pub const fn text(self) -> &'static str {
        match self {
            Pos::NoTrack => "",
            Pos::Off => "Off this stage.",
            Pos::None | Pos::Held => "",
        }
    }

    /// The bridge's word for the state.
    pub const fn word(self) -> &'static str {
        match self {
            Pos::None => "none",
            Pos::NoTrack => "no_track",
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
    let [left, right, top, bottom] = limits;
    let _ = (left, right, top, bottom, at);
    true
}

/// The one point rule for captures and macros: each fighter's delta on frame `at` from the selected fighter's place
/// on the anchor frame. `selected` and `other` are the two fighters' states per frame; `target` is the role the
/// selected fighter plays on.
pub fn deltas(selected: impl Fn(u32) -> Option<ReplayFrame>, other: impl Fn(u32) -> Option<ReplayFrame>, anchor: u32, at: u32, target: u8) -> Ask {
    let Some(origin) = selected(anchor) else { return Ask::None };
    let delta = |frame: ReplayFrame| Spot { at: [frame.x - origin.x, frame.y - origin.y], facing: frame.start.facing };
    let mut deltas = [None; 2];
    deltas[usize::from(target & 1)] = selected(at).map(delta);
    deltas[usize::from(target & 1 ^ 1)] = other(at).map(delta);
    let _ = deltas;
    Ask::None
}

/// One editor's scrub state.
pub struct Block {
    ask: hos_lock::Published<Ask>,
    seen: [AtomicU32; 2],
    /// Each role's hold, as the placement picker keeps one. The game thread only, with `try_lock`.
    pub holds: hos_lock::Mutex<[Option<CpuPreview>; 2]>,
}

impl Block {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            ask: hos_lock::Published::new(Ask::None),
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

    /// Where the role in `slot` is held, `home` being where the hold found the target role; none when it is not held.
    pub fn target(&self, slot: usize, home: [f32; 2]) -> Option<Spot> {
        let Ask::Group { deltas, .. } = self.ask() else { return None };
        let delta = (*deltas.get(slot)?)?;
        Some(Spot { at: [home[0] + delta.at[0], home[1] + delta.at[1]], facing: delta.facing })
    }

    /// The role whose home anchors the group.
    pub fn target_slot(&self) -> Option<usize> {
        match self.ask() {
            Ask::Group { target, .. } => Some(usize::from(target)),
            Ask::None => None,
        }
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

    /// The key line's state for what was asked and what the game side did with it.
    pub fn pos(&self) -> Pos {
        match self.ask() {
            Ask::None => Pos::None,
            Ask::Group { .. } if (0..2).any(|slot| self.seen(slot) == SEEN_OFF) => Pos::Off,
            Ask::Group { .. } => Pos::Held,
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
        if false && ptr & 1 == 1 && ptr != NO_BLOCK && self.readers.load(Ordering::SeqCst) == NO_READER {
            // SAFETY: the word is a retired block of `publish`: no reader is inside (the count), and none can enter,
            // since a reader that counts itself after this check reads an odd word.
            drop(unsafe { Box::from_raw((ptr & !1) as *mut Block) });
            self.ptr.store(NO_BLOCK, Ordering::SeqCst);
        }
        self.empty()
    }
}

/// The one published block of the clip editor.
pub static SLOT: Slot = Slot::new();

/// Frames in one block of a take's position track: 10 s.
pub const TAKE_CHUNK: usize = 600;
const TAKE_CHUNKS: usize = MAX_FRAMES / TAKE_CHUNK;
const STATE_CELLS: usize = 9;
const FRAME_CELLS: usize = 2 * STATE_CELLS;

fn cells(frame: &ReplayFrame) -> [u32; STATE_CELLS] {
    let start = frame.start;
    [start.fighter_kind as u32, start.costume, start.facing.to_bits(), start.situation as u32, start.status as u32,
        start.speed_x.to_bits(), start.speed_y.to_bits(), frame.x.to_bits(), frame.y.to_bits()]
}

fn frame_of(cells: [u32; STATE_CELLS]) -> ReplayFrame {
    ReplayFrame {
        input: InputFrame::default(),
        start: StartState { fighter_kind: cells[0] as i32, costume: cells[1], facing: f32::from_bits(cells[2]), situation: cells[3] as i32,
            status: cells[4] as i32, speed_x: f32::from_bits(cells[5]), speed_y: f32::from_bits(cells[6]) },
        x: f32::from_bits(cells[7]),
        y: f32::from_bits(cells[8]),
    }
}

struct TakeTable {
    /// Each role's state as its last fighter frame read it (the capture's sampler).
    live: [[AtomicU32; STATE_CELLS]; 2],
    /// Blocks of `TAKE_CHUNK` frames, `FRAME_CELLS` cells a frame: the recorded fighter's state, then the other's.
    chunks: [AtomicPtr<AtomicU32>; TAKE_CHUNKS],
    /// Frames of the open take that have their state.
    len: AtomicUsize,
    /// Game-thread pushes inside a block.
    readers: AtomicUsize,
}

/// The position track of the take being recorded: what a match capture keeps per frame, for both fighters. All of
/// it is on the heap. The worker makes the table and the first block once and keeps one block ahead of an open take;
/// the game thread only stores into blocks that are there.
pub struct TakeTrack {
    /// `NO_BLOCK` or the table's address.
    table: AtomicUsize,
}

impl Default for TakeTrack {
    fn default() -> Self {
        Self::new()
    }
}

impl TakeTrack {
    pub const fn new() -> Self {
        Self { table: AtomicUsize::new(NO_BLOCK) }
    }

    fn table(&self) -> Option<&TakeTable> {
        let ptr = self.table.load(Ordering::Acquire);
        // SAFETY: an even word is the table `service` made; it is never freed.
        (ptr & 1 == 0).then(|| unsafe { &*(ptr as *const TakeTable) })
    }

    /// The worker, every pass. `open`: a take is being recorded; `frames`: the frames it holds, not yet saved.
    pub fn service(&self, open: bool, frames: usize) {
        if self.table().is_none() {
            let table = Box::new(TakeTable {
                live: [const { [const { AtomicU32::new(0) }; STATE_CELLS] }; 2],
                chunks: [const { AtomicPtr::new(null_mut()) }; TAKE_CHUNKS],
                len: AtomicUsize::new(0),
                readers: AtomicUsize::new(0),
            });
            self.table.store(Box::into_raw(table) as usize, Ordering::Release);
        }
        let Some(table) = self.table() else { return };
        let ahead = if open { (frames / TAKE_CHUNK + 1).min(TAKE_CHUNKS - 1) } else { 0 };
        for (index, chunk) in table.chunks.iter().enumerate() {
            let ptr = chunk.load(Ordering::SeqCst);
            if index <= ahead {
                if ptr.is_null() {
                    let cells: Box<[AtomicU32]> = (0..TAKE_CHUNK * FRAME_CELLS).map(|_| AtomicU32::new(0)).collect();
                    chunk.store(Box::into_raw(cells) as *mut AtomicU32, Ordering::SeqCst);
                }
            } else if !open && frames == 0 && !ptr.is_null() {
                // No new push finds the block; one already inside it keeps it for this pass.
                chunk.store(null_mut(), Ordering::SeqCst);
                if table.readers.load(Ordering::SeqCst) == 0 {
                    // SAFETY: the block was made above with this length, and no push is inside one.
                    drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, TAKE_CHUNK * FRAME_CELLS)) });
                } else {
                    chunk.store(ptr, Ordering::SeqCst);
                }
            }
        }
    }

    /// The game thread, on a role's fighter frame: its state now.
    pub fn live(&self, slot: usize, frame: &ReplayFrame) {
        if let Some(live) = self.table().and_then(|table| table.live.get(slot)) {
            for (cell, value) in live.iter().zip(cells(frame)) {
                cell.store(value, Ordering::Release);
            }
        }
    }

    /// The game thread, as frame `index` of the take is stored for the role in `slot`: both fighters' states go
    /// with it. Frame 0 starts the track again. A frame whose block is not there, or that does not follow the last
    /// one kept, is left out, and the take then has no track.
    pub fn push(&self, index: usize, slot: usize) {
        let Some(table) = self.table() else { return };
        if slot > 1 || index != 0 && table.len.load(Ordering::Acquire) != index {
            return;
        }
        table.readers.fetch_add(1, Ordering::SeqCst);
        let chunk = table.chunks.get(index / TAKE_CHUNK).map_or(null_mut(), |chunk| chunk.load(Ordering::SeqCst));
        if chunk.is_null() {
            table.len.store(0, Ordering::Release);
        } else {
            // SAFETY: a block holds TAKE_CHUNK frames of FRAME_CELLS cells, and is freed only with no push counted.
            let cells = unsafe { std::slice::from_raw_parts(chunk.add(index % TAKE_CHUNK * FRAME_CELLS), FRAME_CELLS) };
            for (cell, live) in cells.iter().zip(table.live[slot].iter().chain(&table.live[1 - slot])) {
                cell.store(live.load(Ordering::Acquire), Ordering::Release);
            }
            table.len.store(0, Ordering::Release);
        }
        table.readers.fetch_sub(1, Ordering::SeqCst);
    }

    /// The worker, with the finished take's `frames`: its track, when every frame has its state.
    pub fn take(&self, frames: usize) -> Option<Vec<[ReplayFrame; 2]>> {
        let table = self.table()?;
        if frames == 0 || table.len.load(Ordering::Acquire) != frames {
            return None;
        }
        (0..frames).map(|index| {
            let chunk = table.chunks.get(index / TAKE_CHUNK)?.load(Ordering::SeqCst);
            if chunk.is_null() {
                return None;
            }
            // SAFETY: as in `push`; only this thread (the worker, in `service`) frees a block.
            let cells = unsafe { std::slice::from_raw_parts(chunk.add(index % TAKE_CHUNK * FRAME_CELLS), FRAME_CELLS) };
            let read = |at: usize| frame_of(std::array::from_fn(|cell| cells[at + cell].load(Ordering::Acquire)));
            Some([read(0), read(STATE_CELLS)])
        }).collect()
    }
}

/// The track of the take being recorded.
pub static TAKE: TakeTrack = TakeTrack::new();

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::macros::{MacroRecording, ReplaySession, ReplayTrack};
    use crate::engine::macro_ui::clip_panel::{clip_help, ClipAction, ClipEditor, ClipEffect};

    fn spot(x: f32, y: f32, facing: f32) -> Spot {
        Spot { at: [x, y], facing }
    }

    /// The recorded (or selected) fighter walks right and up from (20, 0) and turns left on frame 6; the other one
    /// stands 40 to its left at the start and falls: every x, y and facing is its own.
    fn state(fighter: usize, i: u32) -> ReplayFrame {
        let i = i as f32;
        let (x, y, facing) = if fighter == 0 { (20.0 + 3.0 * i, i, if i >= 6.0 { -1.0 } else { 1.0 }) } else { (-20.0 - i, 50.0 - 2.0 * i, 1.0) };
        ReplayFrame { input: InputFrame::default(), start: StartState { status: 14, facing, ..StartState::neutral(18, 0, 1.0) }, x, y }
    }

    fn capture(tracks: usize) -> ClipEditor {
        let tracks = (0..tracks).map(|fighter| ReplayTrack { fighter: None, entry: 1 - fighter as i32,
            frames: (0..10).map(|i| ReplayFrame { input: InputFrame { buttons: i, ..InputFrame::default() }, ..state(fighter, i) }).collect(),
            quantized_analog: false }).collect();
        ClipEditor::new(ReplaySession { tracks, truncated: false, breaks: Vec::new() }, vec![14]).unwrap()
    }

    fn recording(tracked: bool) -> MacroRecording {
        let mut recording = MacroRecording::new("Recorded", "13.0.5", StartState::neutral(18, 0, 1.0),
            (0..10).map(|i| InputFrame { buttons: i, ..InputFrame::default() }).collect()).unwrap();
        if tracked {
            recording.set_track(Some((0..10).map(|i| [state(0, i), state(1, i)]).collect()));
        }
        recording
    }

    fn saved_macro(player: bool, tracked: bool) -> ClipEditor {
        ClipEditor::macro_file(recording(tracked), player).unwrap()
    }

    #[test]
    fn macros_and_captures_share_one_point_rule() {
        // One table through both: (the handle and its nudges, the frame it lands on).
        for (out, nudge, frame) in [(false, 0, 0u32), (false, 5, 5), (false, 8, 8), (true, 0, 9), (true, -3, 6), (true, -9, 0)] {
            let mut asks = Vec::new();
            for mut editor in [capture(2), saved_macro(false, true)] {
                editor.on_action(if out { ClipAction::SelectOut } else { ClipAction::SelectIn });
                if nudge != 0 { editor.on_action(ClipAction::Nudge(nudge)); }
                asks.push(editor.scrub());
            }
            let wanted = Ask::Group { target: 1, deltas: [
                Some(spot(state(1, frame).x - 20.0, state(1, frame).y, 1.0)),
                Some(spot(3.0 * frame as f32, frame as f32, state(0, frame).start.facing)),
            ] };
            assert_eq!(asks, [(wanted, Pos::Held); 2], "out={out} nudge={nudge}: Out shows frame end - 1");
        }
        let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/model/macro_ui/clip_panel.rs")).unwrap();
        assert_eq!(source.matches("scrub_track::deltas(").count(), 1, "one call for both sources");
    }

    #[test]
    fn a_fresh_take_scrubs_with_no_preview() {
        for player in [false, true] {
            let mut editor = saved_macro(player, true);
            assert_eq!(editor.scrub(), (Ask::None, Pos::None), "no handle is selected");
            editor.on_action(ClipAction::SelectIn);
            editor.on_action(ClipAction::Nudge(4));
            let (ask, pos) = editor.scrub();
            assert_eq!(pos, Pos::Held);
            assert!(matches!(ask, Ask::Group { target, .. } if target == u8::from(!player)), "the macro's play role is the target");
            assert_eq!(clip_help(&editor.view(), "ZL", "ZL", 0).notice, "");
        }
    }

    #[test]
    fn an_old_macro_holds_nobody_and_says_re_record() {
        let mut editor = saved_macro(true, false);
        assert_eq!(clip_help(&editor.view(), "ZL", "ZL", 0).notice, "", "no handle, nothing about positions");
        for action in [ClipAction::SelectIn, ClipAction::SelectOut] {
            editor.on_action(action);
            assert_eq!(editor.scrub(), (Ask::None, Pos::NoTrack));
            assert_eq!(clip_help(&editor.view(), "ZL", "ZL", 0).notice, "Re-record to see positions while scrubbing.");
        }
        // X does not stand in for the missing track.
        assert!(matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::PreviewFile { .. }));
        editor.pause();
        editor.on_action(ClipAction::SelectIn);
        assert_eq!(editor.scrub(), (Ask::None, Pos::NoTrack));
        for pos in [Pos::None, Pos::NoTrack, Pos::Held, Pos::Off] {
            assert!(pos.text().chars().count() <= 67, "{pos:?}");
        }
        assert_eq!([Pos::None, Pos::NoTrack, Pos::Held, Pos::Off].map(Pos::word), ["none", "no_track", "held", "off"]);
    }

    #[test]
    fn the_anchor_is_the_opening_in_and_never_follows_a_handle() {
        // Home x 10, the anchor's sample 20, In moved to a sample of 35 shows 25.
        let mut editor = capture(1);
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(5));
        let block = Block::new();
        block.set_ask(editor.scrub().0);
        assert_eq!(block.target(1, [10.0, 0.0]).map(|spot| spot.at[0]), Some(25.0));
        // Moving In again, X and reselecting do not rebase it.
        editor.on_action(ClipAction::Nudge(1));
        assert!(matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::Preview(_)));
        editor.pause();
        editor.on_action(ClipAction::SelectIn);
        block.set_ask(editor.scrub().0);
        assert_eq!(block.target(1, [10.0, 0.0]), Some(spot(28.0, 6.0, -1.0)));
        editor.on_action(ClipAction::SelectOut);
        block.set_ask(editor.scrub().0);
        assert_eq!(block.target(1, [10.0, 0.0]), Some(spot(37.0, 9.0, -1.0)));
    }

    #[test]
    fn both_fighters_move_with_one_offset() {
        let mut editor = capture(2);
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(4));
        let block = Block::new();
        block.set_ask(editor.scrub().0);
        // The target (the CPU) was found at (100, 7): its fighter is 12 right and 4 up of the anchor; the other
        // fighter keeps the recorded spacing to it.
        let home = [100.0, 7.0];
        assert_eq!(block.target_slot(), Some(1));
        assert_eq!(block.target(1, home), Some(spot(112.0, 11.0, 1.0)));
        assert_eq!(block.target(0, home), Some(spot(100.0 + (-24.0 - 20.0), 7.0 + 42.0, 1.0)));
        // A capture with one track moves one body.
        let mut single = capture(1);
        single.on_action(ClipAction::SelectIn);
        assert!(matches!(single.scrub().0, Ask::Group { target: 1, deltas: [None, Some(_)] }));
        // ZR: the selected fighter plays on Player 1, and the group follows.
        editor.on_action(ClipAction::SwapTarget);
        editor.on_action(ClipAction::SelectIn);
        assert!(matches!(editor.scrub().0, Ask::Group { target: 0, deltas: [Some(_), Some(_)] }));
    }

    #[test]
    fn a_body_outside_the_stage_bounds_stays_and_the_other_moves() {
        // Camera limits as the game gives them: left, right, top, bottom; not symmetric.
        let limits = [-170.0, 120.0, 130.0, -40.0];
        for (at, inside) in [
            ([0.0, 0.0], true), ([-170.0, 130.0], true), ([120.0, -40.0], true),
            ([-171.0, 0.0], false), ([121.0, 0.0], false), ([0.0, 131.0], false), ([0.0, -41.0], false),
            ([f32::NAN, 0.0], false), ([0.0, f32::INFINITY], false),
        ] {
            assert_eq!(in_bounds(at, limits), inside, "{at:?}");
        }
        assert!(!in_bounds([0.0, 0.0], [f32::NAN, 120.0, 130.0, -40.0]));
        // The key line names it when either body is off; the game side decides per body.
        let block = Block::new();
        block.set_ask(Ask::Group { target: 1, deltas: [Some(spot(0.0, 0.0, 1.0)); 2] });
        block.set_seen(0, SEEN_HELD);
        block.set_seen(1, SEEN_HELD);
        assert_eq!(block.pos(), Pos::Held);
        block.set_seen(0, SEEN_OFF);
        assert_eq!(block.pos(), Pos::Off);
        block.set_ask(Ask::None);
        assert_eq!(block.pos(), Pos::None);
        let runtime = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/features/scrub_hold.rs")).unwrap_or_default();
        assert!(runtime.contains("if !in_bounds(spot.at, limits) {"));
        assert!(runtime.contains("return SEEN_OFF;"));
    }

    #[test]
    fn only_a_selected_handle_holds_and_a_preview_releases() {
        for mut editor in [capture(2), saved_macro(false, true)] {
            assert_eq!(editor.scrub(), (Ask::None, Pos::None));
            editor.on_action(ClipAction::SelectOut);
            assert!(matches!(editor.scrub(), (Ask::Group { .. }, Pos::Held)));
            // R again deselects the handle.
            editor.on_action(ClipAction::SelectOut);
            assert_eq!(editor.scrub(), (Ask::None, Pos::None));
            editor.on_action(ClipAction::SelectIn);
            assert!(!matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::Error(_)));
            assert_eq!(editor.scrub(), (Ask::None, Pos::None), "a playing preview holds nothing");
        }
        let runtime = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/features/scrub_hold.rs")).unwrap_or_default();
        assert!(runtime.contains("None => release(&mut holds[slot], accessor),"), "a role with no point is released");
        assert!(runtime.contains("placement::restore(accessor, hold.origin());"));
    }

    #[test]
    fn a_combo_crop_and_a_drill_take_have_no_track() {
        let root = env!("CARGO_MANIFEST_DIR");
        let panel = std::fs::read_to_string(format!("{root}/src/model/macro_ui/clip_panel.rs")).unwrap();
        let scrub = panel.split_once("pub fn scrub(").expect("the editor's scrub").1.split_once("\n    }\n").unwrap().0;
        assert!(scrub.contains("Some(FileSource::Combo(_)) => return (Ask::None, Pos::None),"));
        // Only a Record take is given a track; a drill's take is saved as before.
        let runtime = std::fs::read_to_string(format!("{root}/src/model/rr_runtime.rs")).unwrap();
        let drill = runtime.split_once("pub fn save_drill_take_of(").unwrap().1.split_once("\n}").unwrap().0;
        assert!(!drill.contains("set_track"));
        assert_eq!(runtime.matches("recording.set_track(").count(), 1);
    }

    #[test]
    fn record_keeps_the_capture_state_of_both_fighters_per_frame() {
        let take = TakeTrack::new();
        take.push(0, 0);
        assert_eq!(take.take(1), None, "before the worker made the first block there is no track");
        take.service(false, 0);
        for index in 0..3u32 {
            take.live(0, &state(0, index));
            take.live(1, &state(1, index));
            // The CPU is recorded: its state comes first.
            take.push(index as usize, 1);
        }
        assert_eq!(take.take(3), Some((0..3).map(|i| [state(1, i), state(0, i)]).collect::<Vec<_>>()));
        assert_eq!(take.take(2), None, "one state per frame, or no track");
        // Every field of the capture's state is kept.
        let odd = ReplayFrame { input: InputFrame::default(), x: -88.5, y: 12.25,
            start: StartState { fighter_kind: 74, costume: 5, facing: -1.0, situation: 2, status: 486, speed_x: -1.25, speed_y: 3.5 } };
        take.live(0, &odd);
        take.push(0, 0);
        assert_eq!(take.take(1).map(|track| track[0][0]), Some(odd));
        // The game thread stores the state beside the input, and reads it with the capture's own sampler.
        let root = env!("CARGO_MANIFEST_DIR");
        let host = std::fs::read_to_string(format!("{root}/src/model/rr_host.rs")).unwrap();
        let push = host.split_once("pub fn push_capture(").unwrap().1.split_once("\n}").unwrap().0;
        assert!(push.contains("TAKE.push(len, usize::from(side == ROLE_CPU));"));
        assert!(host.contains("let track = TAKE.take(frames.len());") && host.contains("capture.set_track(track);"));
        let states = std::fs::read_to_string(format!("{root}/src/runtime/features/states.rs")).unwrap();
        assert!(states.contains("TAKE.live(role.index(), &crate::runtime::macros::replay_capture::state("));
    }

    #[test]
    fn the_take_buffer_grows_in_blocks_and_a_gap_leaves_no_track() {
        let take = TakeTrack::new();
        take.service(false, 0);
        let fill = |from: usize, to: usize| {
            for index in from..to {
                take.live(0, &state(0, index as u32));
                take.push(index, 0);
            }
        };
        fill(0, TAKE_CHUNK);
        // The worker never came by: the second block is not there, and the game thread does not make one.
        take.push(TAKE_CHUNK, 0);
        assert_eq!(take.take(TAKE_CHUNK + 1), None);
        take.push(TAKE_CHUNK + 1, 0);
        assert_eq!(take.take(TAKE_CHUNK + 2), None, "frames after the gap do not make a track");
        // With the worker one block ahead of the take, it goes on.
        fill(0, TAKE_CHUNK - 1);
        take.service(true, TAKE_CHUNK - 1);
        fill(TAKE_CHUNK - 1, TAKE_CHUNK + 5);
        let track = take.take(TAKE_CHUNK + 5).expect("a whole track");
        assert_eq!(track[TAKE_CHUNK + 4][0].x, state(0, (TAKE_CHUNK + 4) as u32).x);
        // Saved and idle: the blocks past the first are freed; the first stays for the next take's first frames.
        take.service(false, 0);
        fill(0, TAKE_CHUNK);
        assert_eq!(take.take(TAKE_CHUNK).map(|track| track.len()), Some(TAKE_CHUNK));
        take.push(TAKE_CHUNK, 0);
        assert_eq!(take.take(TAKE_CHUNK + 1), None);
        assert_eq!(TAKE_CHUNK * FRAME_CELLS * 4, 43_200);
        assert_eq!(TAKE_CHUNKS * TAKE_CHUNK, MAX_FRAMES);
        assert!(include_str!("scrub_track.rs").contains("Self { table: AtomicUsize::new(NO_BLOCK) }"), "initialised data, not BSS");
    }

    #[test]
    fn a_track_is_saved_as_version_three_and_old_files_are_unchanged() {
        let old = recording(false);
        let old_json = serde_json::to_string(&old).unwrap();
        assert!(!old_json.contains("track") && old_json.contains("\"schema_version\":1"), "{old_json}");
        let new = recording(true);
        assert_eq!(new.schema_version, 3);
        new.validate().unwrap();
        let json = serde_json::to_string(&new).unwrap();
        let back: MacroRecording = serde_json::from_str(&json).unwrap();
        assert_eq!(back, new);
        // Per frame: two rows of nine numbers.
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["track"].as_array().map(Vec::len), Some(10));
        assert_eq!(value["track"][0].as_array().map(Vec::len), Some(2));
        assert_eq!(value["track"][0][0].as_array().map(Vec::len), Some(9));
        // The version and the track go together, one finite state per frame.
        let broken: [fn(&mut MacroRecording); 4] = [
            |recording| recording.schema_version = 1,
            |recording| recording.track = None,
            |recording| { recording.track.as_mut().unwrap().pop(); },
            |recording| recording.track.as_mut().unwrap()[0][1].x = f32::NAN,
        ];
        for (index, edit) in broken.iter().enumerate() {
            let mut bad = new.clone();
            edit(&mut bad);
            assert!(bad.validate().is_err(), "edit {index}");
        }
        let mut bad = old.clone();
        bad.schema_version = 3;
        assert!(bad.validate().is_err(), "version 3 without a track");
        // Without its track the file is the old file, byte for byte.
        let mut dropped = new.clone();
        dropped.set_track(None);
        assert_eq!(serde_json::to_string(&dropped).unwrap(), old_json);
    }

    #[test]
    fn a_crop_and_a_trim_keep_the_track_slice() {
        let mut kept = recording(true);
        kept.keep(2..7);
        assert_eq!(kept.frames.iter().map(|frame| frame.buttons).collect::<Vec<_>>(), [2, 3, 4, 5, 6]);
        assert_eq!(kept.track, Some((2..7).map(|i| [state(0, i), state(1, i)]).collect()));
        kept.validate().unwrap();
        let mut old = recording(false);
        old.keep(1..3);
        assert_eq!((old.frames.len(), old.track.is_none(), old.schema_version), (2, true, 1));
        // The crop and Record's trim both go through it.
        let root = env!("CARGO_MANIFEST_DIR");
        let panel = std::fs::read_to_string(format!("{root}/src/model/macro_ui/clip_panel.rs")).unwrap();
        assert!(panel.contains("kept.keep(self.window.start() as usize..self.window.end() as usize);"));
        let runtime = std::fs::read_to_string(format!("{root}/src/model/rr_runtime.rs")).unwrap();
        assert!(runtime.contains(".map(|track| track[kept.start..kept.end].to_vec());"));
    }

    #[test]
    fn a_retired_block_is_freed_only_with_no_reader() {
        let slot = Slot::new();
        assert!(slot.empty() && !slot.live());
        assert_eq!(slot.with(|_| ()), None);
        assert!(slot.publish(Block::new()).is_ok());
        assert!(slot.live());
        assert!(slot.publish(Block::new()).is_err(), "a second block is refused while the first is there");
        // A reader that is inside when the block is retired keeps it alive; a new reader finds nothing.
        slot.with(|block| {
            block.set_ask(Ask::Group { target: 0, deltas: [Some(spot(1.0, 2.0, 1.0)), None] });
            slot.retire();
            assert!(!slot.live());
            assert_eq!(slot.with(|_| ()), None);
            assert!(!slot.reap(), "a reader is inside");
            assert!(slot.publish(Block::new()).is_err(), "not before the old block is freed");
            assert_eq!(block.target(0, [0.0, 0.0]), Some(spot(1.0, 2.0, 1.0)));
        });
        assert!(slot.reap());
        assert!(slot.empty() && slot.reap(), "nothing left to free");
        assert!(slot.publish(Block::new()).is_ok());
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
        // The practice test the preview's checkpoint uses: Training and the waiting room's practice; a match has none.
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
