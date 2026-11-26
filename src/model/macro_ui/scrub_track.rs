//! Where the fighters were on a clip's frames, for the clip editor's scrub (operator 2026-11-25/26): a selected In or
//! Out handle holds both fighters where the recording had them on that handle's frame. A match capture and a saved
//! macro both carry that state per frame (`ReplayFrame`), and both go through `deltas`.
//!
//! The block lives on the heap while an editor is open. The game side reaches it through `Slot`, a published pointer
//! with a reader count, and never through a lock the worker holds.
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use crate::engine::macros::{InputFrame, PackedPose, ReplayFrame, StartState, MAX_FRAMES};
use crate::model::crosshair::CpuPreview;
use crate::model::stage_geometry::{Place, Unread};

/// A fighter's place and facing on one frame; as a delta, how far from the target fighter's home.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub at: [f32; 2],
    pub facing: f32,
    /// The recorded fighter's kind on this frame, and its animation (none when the frame kept no pose).
    pub kind: i32,
    pub pose: PackedPose,
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
    /// The fighters are shown at the frame's places; the macro kept no animations (recorded before they were kept).
    NoMoves,
    /// The fighters are shown at the handle's frame.
    Held,
    /// A fighter's point is not a place to stand on this stage (outside the camera's limits or the blast lines, or
    /// inside a solid): that fighter does not move.
    Off,
    /// The macro's track ends before this frame (a block could not be had while recording): nobody is held.
    End,
}

impl Pos {
    /// The key line's text; empty when there is nothing to say.
    pub const fn text(self) -> &'static str {
        match self {
            Pos::NoTrack => "Play it once to see positions while scrubbing.",
            Pos::NoMoves => "Play it once to see the moves while scrubbing.",
            Pos::Off => "Can't stand there on this stage.", // restriction: scrub-point-not-standable
            Pos::End => "Positions end here.",
            Pos::None | Pos::Held => "",
        }
    }

    /// The bridge's word for the state.
    pub const fn word(self) -> &'static str {
        match self {
            Pos::None => "none",
            Pos::NoTrack => "no_track",
            Pos::Held | Pos::NoMoves => "held",
            Pos::Off => "off",
            Pos::End => "end",
        }
    }
}

/// What the game side did with a role's body on its last frame.
pub const SEEN_IDLE: u32 = 0;
pub const SEEN_HELD: u32 = 1;
pub const SEEN_OFF: u32 = 2;

/// What the game side drew for a role: nothing, the recorded animation, or nothing because the body is another fighter.
pub const POSE_NONE: u32 = 0;
pub const POSE_SHOWN: u32 = 1;
pub const POSE_OTHER: u32 = 2;

/// Whether a point may hold a body: inside the camera's limits `[left, right, top, bottom]`. A body held past them
/// is on its way to a blast line.
pub fn in_bounds(at: [f32; 2], limits: [f32; 4]) -> bool {
    let [left, right, top, bottom] = limits;
    left <= at[0] && at[0] <= right && bottom <= at[1] && at[1] <= top
}

/// Whether a body may be put at a point of this stage, by the shared stage rule (`stage_geometry::classify`): over a
/// floor or a platform, or in open air, also under the stage. Not in a solid, not outside the blast lines, and not
/// on a stage that could not be read.
pub fn can_stand(place: Result<Place, Unread>) -> bool {
    matches!(place, Ok(Place::OverStage | Place::Open | Place::Under))
}

/// The one point rule for captures and macros: each fighter's delta on frame `at` from the selected fighter's place
/// on the anchor frame. `selected` and `other` are the two fighters' states per frame; `target` is the role the
/// selected fighter plays on.
pub fn deltas(selected: impl Fn(u32) -> Option<ReplayFrame>, other: impl Fn(u32) -> Option<ReplayFrame>, anchor: u32, at: u32, target: u8) -> Ask {
    let Some(origin) = selected(anchor) else { return Ask::None };
    let delta = |frame: ReplayFrame| Spot { at: [frame.x - origin.x, frame.y - origin.y], facing: frame.start.facing,
        kind: frame.start.fighter_kind, pose: frame.pose };
    let mut deltas = [None; 2];
    deltas[usize::from(target & 1)] = selected(at).map(delta);
    deltas[usize::from(target & 1 ^ 1)] = other(at).map(delta);
    if deltas == [None; 2] { Ask::None } else { Ask::Group { target: target & 1, deltas } }
}

/// `Block::home`'s role word on the callback that forgot a hold of another body: no role, no home.
const FORGOT: u32 = u32::MAX;

/// A world point as one word, compared exactly.
fn key(at: [f32; 2]) -> u64 {
    u64::from(at[0].to_bits()) << 32 | u64::from(at[1].to_bits())
}

/// One editor's scrub state.
pub struct Block {
    ask: hos_lock::Published<Ask>,
    seen: [AtomicU32; 2],
    posed: [(AtomicU32, AtomicU64); 2],
    /// The ask's generation, plus one, at which the game side found a hold made on another body; zero before any.
    dropped: AtomicU32,
    /// The frame the ask is for, plus one; zero for none.
    shown: AtomicU32,
    /// Each role's last world point and tagged verdict. Only the game side touches these, under `holds`.
    classed: [AtomicU64; 2],
    stands: [AtomicU64; 2],
    /// The group's home while a target that was held is not (`point`): the target role plus one, and the point.
    home: (AtomicU32, AtomicU64),
    /// Each role's last write (`pushed`): its point, and the request's generation plus one.
    wrote: [(AtomicU64, AtomicU64); 2],
    /// Each role's refusal of a body the game pushed off its point: the point, the request, the body's id.
    pushed: [(AtomicU64, AtomicU64, AtomicU32); 2],
    /// The ask's generation, plus one, each role's fighter frame last acted under; zero before its first frame.
    acked: [AtomicU32; 2],
    /// The roles the last group ask moved (bit 0 Player 1, bit 1 the CPU).
    moved: AtomicU32,
    /// A preview's start follows a release and waits for the released CPU by its state (`state::preview::settle`).
    settle: AtomicU32,
    /// Each role's hold, as the placement picker keeps one. The game thread only, with `try_lock`.
    pub holds: hos_lock::Mutex<[Option<CpuPreview>; 2]>,
}

impl Block {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            ask: hos_lock::Published::new(Ask::None),
            seen: [const { AtomicU32::new(SEEN_IDLE) }; 2],
            posed: [const { (AtomicU32::new(POSE_NONE), AtomicU64::new(0)) }; 2],
            dropped: AtomicU32::new(0),
            shown: AtomicU32::new(0),
            classed: [const { AtomicU64::new(0) }; 2],
            stands: [const { AtomicU64::new(0) }; 2],
            home: (AtomicU32::new(0), AtomicU64::new(0)),
            wrote: [const { (AtomicU64::new(0), AtomicU64::new(0)) }; 2],
            pushed: [const { (AtomicU64::new(0), AtomicU64::new(0), AtomicU32::new(0)) }; 2],
            acked: [const { AtomicU32::new(0) }; 2],
            moved: AtomicU32::new(0),
            settle: AtomicU32::new(0),
            holds: hos_lock::Mutex::new([None; 2]),
        })
    }

    pub fn ask(&self) -> Ask {
        self.ask.read()
    }

    /// The worker: what the fighters show from the next frame on.
    pub fn set_ask(&self, ask: Ask) {
        if self.ask.read() != ask {
            self.put(ask);
        }
    }

    /// The worker: what the fighters show from the next frame on, and the frame it is for. Another frame is another
    /// request also when its points are the same (an idle take), so a dropped hold wakes on any move of the handle.
    pub fn ask_at(&self, frame: Option<u32>, ask: Ask) {
        let frame = frame.map_or(0, |frame| frame.wrapping_add(1));
        if self.shown.swap(frame, Ordering::AcqRel) != frame || self.ask.read() != ask {
            self.put(ask);
        }
    }

    /// The clip frame the ask is for; none when no frame is shown.
    pub fn shown(&self) -> Option<u32> {
        self.shown.load(Ordering::Acquire).checked_sub(1)
    }

    /// The game thread: the body a hold was made on is gone (a swapped fighter, a new scene). Nobody is held until
    /// the editor asks for another frame.
    pub fn forget(&self) {
        self.dropped.store(self.ask.generation().wrapping_add(1), Ordering::Release);
        // Another body, maybe another stage: what the stage said is asked again.
        for verdict in &self.stands {
            verdict.store(0, Ordering::Release);
        }
        for slot in 0..2 {
            self.set_pose(slot, POSE_NONE, PackedPose::NONE);
        }
        // The callback that forgets asks for its point next, with the hold of the body that is gone still in its
        // slot: it lets go, whatever the worker has published since (`point`).
        self.home.0.store(FORGOT, Ordering::Release);
    }

    fn forgotten(&self) -> bool {
        self.dropped.load(Ordering::Acquire) == self.ask.generation().wrapping_add(1)
    }

    /// Where the role in `slot` is held, `home` being where the hold found the target role; none when it is not held.
    pub fn held_at(&self, slot: usize, home: [f32; 2]) -> Option<Spot> {
        if self.forgotten() {
            return None;
        }
        let Ask::Group { deltas, .. } = self.ask() else { return None };
        let delta = (*deltas.get(slot)?)?;
        Some(Spot { at: [home[0] + delta.at[0], home[1] + delta.at[1]], facing: delta.facing,
            kind: delta.kind, pose: delta.pose })
    }

    /// Recheck on a new ask even at identical coordinates, or if a moving anchor changes this ask's world point.
    pub fn standable(&self, slot: usize, at: [f32; 2], class: impl FnOnce() -> bool) -> bool {
        let (Some(point), Some(verdict)) = (self.classed.get(slot), self.stands.get(slot)) else { return false };
        let key = key(at);
        // Zero means invalid; the low bit of a tagged answer is the verdict.
        let tag = (u64::from(self.ask.generation()) + 1) << 1;
        let cached = verdict.load(Ordering::Relaxed);
        if cached & !1 == tag && point.load(Ordering::Relaxed) == key {
            return cached & 1 != 0;
        }
        let ok = class();
        point.store(key, Ordering::Relaxed);
        verdict.store(tag | u64::from(ok), Ordering::Relaxed);
        ok
    }

    /// The game thread: where the role in `slot` is asked to be. `origin`: where the target role's hold found its
    /// body, when it is held; `free`: where the target stands when it is not. A target that was held keeps its home when it
    /// is let go while a group is still asked for: its published position is read before the hold's step, so on that
    /// frame it is still the point it was held at, and its partner would be placed from there.
    pub fn point(&self, slot: usize, origin: impl FnOnce(usize) -> Option<[f32; 2]>, free: impl FnOnce(usize) -> Option<[f32; 2]>) -> Option<Spot> {
        let (forgot, target) = (self.home.0.load(Ordering::Relaxed) == FORGOT, self.target_slot());
        let Some(target) = target.filter(|_| !forgot) else {
            self.home.0.store(0, Ordering::Relaxed);
            return None;
        };
        let role = target as u32 + 1;
        let home = if let Some(at) = origin(target) {
            self.home.1.store(key(at), Ordering::Relaxed);
            self.home.0.store(role, Ordering::Relaxed);
            at
        } else if self.home.0.load(Ordering::Relaxed) == role {
            let kept = self.home.1.load(Ordering::Relaxed);
            [f32::from_bits((kept >> 32) as u32), f32::from_bits(kept as u32)]
        } else {
            free(target)?
        };
        self.held_at(slot, home)
    }

    /// The game thread, after the hold's step for the role in `slot`. `moved`: where the game has the body, when the
    /// hold reports that the game moved it off the point written on the frame before. The scrub refuses a body the
    /// game moved sideways: its x is not the x asked, compared exactly as the hold compares (plus and minus zero are
    /// one place: the CPU's Training home is x 0). Walls and the ends of floors do that; gravity and
    /// a floor under the point never move x, so a body the floor takes straight up or down stays held and is shown
    /// where the floor has it. (A ceiling pushing straight down is not refused.) The report is about this request
    /// only when that write was this role's, for this request (`generation`, read before the ask), at this point:
    /// then the body is refused here, and `true` says to let it go. Otherwise this frame's write is noted and the
    /// next frame is judged.
    pub fn pushed(&self, slot: usize, generation: u32, body: u32, at: [f32; 2], moved: Option<[f32; 2]>) -> bool {
        let (Some(wrote), Some(pushed)) = (self.wrote.get(slot), self.pushed.get(slot)) else { return false };
        let (key, request) = (key(at), u64::from(generation) + 1);
        let sideways = moved.is_some_and(|seen| seen[0] != at[0]);
        if sideways && wrote.0.load(Ordering::Relaxed) == key && wrote.1.load(Ordering::Relaxed) == request {
            pushed.0.store(key, Ordering::Relaxed);
            pushed.1.store(request, Ordering::Relaxed);
            pushed.2.store(body, Ordering::Relaxed);
            wrote.1.store(0, Ordering::Relaxed);
            return true;
        }
        wrote.0.store(key, Ordering::Relaxed);
        wrote.1.store(request, Ordering::Relaxed);
        false
    }

    /// The game thread: the game pushed this body off this point under this request (`pushed`). It is not taken again
    /// until the shown request, the point or the body changes.
    pub fn refused(&self, slot: usize, generation: u32, body: u32, at: [f32; 2]) -> bool {
        self.pushed.get(slot).is_some_and(|pushed| {
            pushed.0.load(Ordering::Relaxed) == key(at) && pushed.1.load(Ordering::Relaxed) == u64::from(generation) + 1
                && pushed.2.load(Ordering::Relaxed) == body
        })
    }

    /// Publish an ask. A group ask names the roles a later "nobody" waits for, and ends an older start's wait.
    fn put(&self, ask: Ask) {
        if let Ask::Group { deltas, .. } = ask {
            self.moved.store(u32::from(deltas[0].is_some()) | u32::from(deltas[1].is_some()) << 1, Ordering::Release);
            self.settle.store(0, Ordering::Release);
        }
        self.ask.publish(ask);
    }

    /// The generation of the ask now published. A role's frame reads it before the ask and reports it with `ack`.
    pub fn generation(&self) -> u32 {
        self.ask.generation()
    }

    /// The game thread, at the end of a role's fighter frame: the generation it read before it read the ask.
    pub fn ack(&self, slot: usize, generation: u32) {
        if let Some(acked) = self.acked.get(slot) {
            acked.store(generation.wrapping_add(1), Ordering::Release);
        }
    }

    /// Acknowledgements for the latest ask only. The worker must use `Slot::released` to also check
    /// for callbacks still running and bodies that remain held from an earlier ask.
    pub(super) fn released(&self) -> bool {
        if self.ask() != Ask::None {
            return false;
        }
        let (now, moved) = (self.ask.generation().wrapping_add(1), self.moved.load(Ordering::Acquire));
        self.acked.iter().enumerate().all(|(slot, acked)| {
            let acked = acked.load(Ordering::Acquire);
            moved & (1 << slot) == 0 || acked == 0 || acked == now
        })
    }

    /// The worker, as it starts a play that waited for the release: the start waits for the released CPU.
    pub fn await_settle(&self) {
        self.settle.store(1, Ordering::Release);
    }

    /// The game thread: a start waits for the CPU the hold let go.
    pub fn settling(&self) -> bool {
        self.settle.load(Ordering::Acquire) != 0
    }

    /// The game thread: the released CPU was seen settled, or will not settle; the wait is over.
    pub fn settled(&self) {
        self.settle.store(0, Ordering::Release);
    }

    /// A forgotten group has no anchor, even while its old hold awaits release.
    pub fn target_slot(&self) -> Option<usize> {
        match self.ask() {
            Ask::Group { target, .. } if !self.forgotten() => Some(usize::from(target)),
            _ => None,
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

    /// The game thread, on the role's fighter frame: what it drew, and the animation it read back.
    pub fn set_pose(&self, slot: usize, state: u32, shown: PackedPose) {
        if let Some((status, pose)) = self.posed.get(slot) {
            pose.store(u64::from(shown.0[0]) | u64::from(shown.0[1]) << 32, Ordering::Relaxed);
            status.store(state, Ordering::Relaxed);
        }
    }

    /// What the game side last drew for the role in `slot`.
    pub fn pose(&self, slot: usize) -> (u32, PackedPose) {
        self.posed.get(slot).map_or((POSE_NONE, PackedPose::NONE), |(status, pose)| {
            let word = pose.load(Ordering::Relaxed);
            (status.load(Ordering::Relaxed), PackedPose([word as u32, (word >> 32) as u32]))
        })
    }

    /// The key line's state for what was asked and what the game side did with it.
    pub fn pos(&self) -> Pos {
        match self.ask() {
            Ask::None => Pos::None,
            Ask::Group { .. } if self.forgotten() => Pos::None,
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
        // SAFETY: an even word is a block made by `publish`; `reap` frees it only after `withdraw` made the word odd
        // and the count read `NO_READER`, which this reader's count, taken before the load, prevents.
        let result = (ptr & 1 == 0).then(|| f(unsafe { &*(ptr as *const Block) }));
        self.readers.fetch_sub(1, Ordering::SeqCst);
        result
    }

    /// The worker's release check before it starts a wanted play.
    pub fn released(&self) -> bool {
        // A callback may have read a point but not reported its first hold yet. Once we are the only
        // reader of the cancelled ask, no later callback can start a hold and both last reports count.
        self.with(|b| self.readers.load(Ordering::SeqCst) == NO_READER + 1 && b.released()
            && (0..2).all(|slot| b.seen(slot) != SEEN_HELD)).unwrap_or(true)
    }

    /// The worker: cancel the request and keep the slot reachable until the fighters have released it.
    pub fn close(&self) {
        let idle = self.with(|block| {
            block.set_ask(Ask::None);
            // An earlier reader may have its point but not yet have published SEEN_HELD.
            // Once this is the only reader, any new fighter can see only the cancelled request.
            self.readers.load(Ordering::SeqCst) == NO_READER + 1
                && (0..2).all(|slot| block.seen(slot) == SEEN_IDLE)
        });
        if idle == Some(true) { self.withdraw(); }
        self.reap();
    }

    /// Stop new readers without disturbing ones already inside.
    pub fn withdraw(&self) {
        let ptr = self.ptr.load(Ordering::SeqCst);
        if ptr & 1 == 0 {
            self.ptr.store(ptr | 1, Ordering::SeqCst);
        }
    }

    /// The worker: free a retired block once no reader is inside `with`. True when nothing is left to free.
    pub fn reap(&self) -> bool {
        let ptr = self.ptr.load(Ordering::SeqCst);
        if ptr & 1 == 1 && ptr != NO_BLOCK && self.readers.load(Ordering::SeqCst) == NO_READER {
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
const STATE_CELLS: usize = 11;
const FRAME_CELLS: usize = 2 * STATE_CELLS;

fn cells(frame: &ReplayFrame) -> [u32; STATE_CELLS] {
    let start = frame.start;
    [start.fighter_kind as u32, start.costume, start.facing.to_bits(), start.situation as u32, start.status as u32,
        start.speed_x.to_bits(), start.speed_y.to_bits(), frame.x.to_bits(), frame.y.to_bits(), frame.pose.0[0], frame.pose.0[1]]
}

fn frame_of(cells: [u32; STATE_CELLS]) -> ReplayFrame {
    ReplayFrame {
        pose: PackedPose([cells[9], cells[10]]),
        input: InputFrame::default(),
        start: StartState { fighter_kind: cells[0] as i32, costume: cells[1], facing: f32::from_bits(cells[2]), situation: cells[3] as i32,
            status: cells[4] as i32, speed_x: f32::from_bits(cells[5]), speed_y: f32::from_bits(cells[6]) },
        x: f32::from_bits(cells[7]),
        y: f32::from_bits(cells[8]),
    }
}

/// One block of a take's track, when the memory is there: zeroed in one allocation, not built cell by cell.
fn new_block() -> Option<Box<[AtomicU32]>> {
    let cells = TAKE_CHUNK * FRAME_CELLS;
    let layout = std::alloc::Layout::array::<AtomicU32>(cells).ok()?;
    // SAFETY: the layout is not empty; all-zero bytes are a valid `AtomicU32` (zero, what each cell started as); the
    // box owns exactly this allocation and frees it with the same layout.
    unsafe {
        let ptr = std::alloc::alloc_zeroed(layout) as *mut AtomicU32;
        (!ptr.is_null()).then(|| Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, cells)))
    }
}

/// The worker: free a block. No new push finds it; with a push already inside one, it stays for a later pass.
fn free_block(table: &TakeTable, chunk: &AtomicPtr<AtomicU32>) {
    let ptr = chunk.swap(null_mut(), Ordering::SeqCst);
    if ptr.is_null() {
        return;
    }
    if table.readers.load(Ordering::SeqCst) == 0 {
        // SAFETY: the block was made by `service_with` with this length, and no push is inside one.
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, TAKE_CHUNK * FRAME_CELLS)) });
    } else {
        chunk.store(ptr, Ordering::SeqCst);
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
        self.service_with(open, frames, new_block);
    }

    /// `service` with the block allocator given: one that returns nothing leaves the block out.
    pub fn service_with(&self, open: bool, frames: usize, alloc: impl Fn() -> Option<Box<[AtomicU32]>>) {
        if self.table().is_none() {
            let table = Box::new(TakeTable {
                live: [const { [const { AtomicU32::new(0) }; STATE_CELLS] }; 2],
                chunks: [const { AtomicPtr::new(null_mut()) }; TAKE_CHUNKS],
                len: AtomicUsize::new(0),
                readers: AtomicUsize::new(0),
            });
            // A role with no fighter frame yet is absent (kind -1), as a saved capture marks it.
            for live in &table.live {
                live[0].store(-1i32 as u32, Ordering::Release);
            }
            self.table.store(Box::into_raw(table) as usize, Ordering::Release);
        }
        let Some(table) = self.table() else { return };
        let ahead = if open { (frames / TAKE_CHUNK + 1).min(TAKE_CHUNKS - 1) } else { 0 };
        for (index, chunk) in table.chunks.iter().enumerate() {
            let ptr = chunk.load(Ordering::SeqCst);
            if index <= ahead {
                let made = if ptr.is_null() { alloc().filter(|cells| cells.len() == TAKE_CHUNK * FRAME_CELLS) } else { None };
                if let Some(cells) = made {
                    chunk.store(Box::into_raw(cells) as *mut AtomicU32, Ordering::SeqCst);
                }
            } else if !open && frames == 0 {
                free_block(table, chunk);
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

    /// Seed this role at the start of a new take, before its first input is recorded.
    pub fn seed(&self, slot: usize, frame: Option<ReplayFrame>) {
        if let Some(frame) = frame {
            self.live(slot, &frame);
        } else if let Some(live) = self.table().and_then(|table| table.live.get(slot)) {
            for (index, cell) in live.iter().enumerate() {
                cell.store(if index == 0 { u32::MAX } else { 0 }, Ordering::Release);
            }
        }
    }

    /// The game thread, as frame `index` of the take is stored for the role in `slot`: both fighters' states go
    /// with it. Frame 0 starts the track again. A frame whose block is not there, or that does not follow the last
    /// one kept, is left out: the track ends before it, and the take's inputs go on.
    pub fn push(&self, index: usize, slot: usize) {
        let Some(table) = self.table() else { return };
        if slot > 1 || index != 0 && table.len.load(Ordering::Acquire) != index {
            return;
        }
        table.readers.fetch_add(1, Ordering::SeqCst);
        let chunk = table.chunks.get(index / TAKE_CHUNK).map_or(null_mut(), |chunk| chunk.load(Ordering::SeqCst));
        if chunk.is_null() {
            // The track ends here and keeps what it has. With no first block there is nothing to keep.
            table.len.fetch_min(index, Ordering::AcqRel);
        } else {
            // SAFETY: a block holds TAKE_CHUNK frames of FRAME_CELLS cells, and is freed only with no push counted.
            let cells = unsafe { std::slice::from_raw_parts(chunk.add(index % TAKE_CHUNK * FRAME_CELLS), FRAME_CELLS) };
            for (cell, live) in cells.iter().zip(table.live[slot].iter().chain(&table.live[1 - slot])) {
                cell.store(live.load(Ordering::Acquire), Ordering::Release);
            }
            table.len.store(index + 1, Ordering::Release);
        }
        table.readers.fetch_sub(1, Ordering::SeqCst);
    }

    /// The worker, with the finished take's `frames`: its track, as far as the frames have their state.
    pub fn take(&self, frames: usize) -> Option<Vec<[ReplayFrame; 2]>> {
        let table = self.table()?;
        let kept = table.len.load(Ordering::Acquire).min(frames);
        let mut track = Vec::new();
        track.try_reserve_exact(kept).ok()?;
        for index in 0..kept {
            let Some(slot) = table.chunks.get(index / TAKE_CHUNK) else { break };
            let chunk = slot.load(Ordering::SeqCst);
            if chunk.is_null() {
                break;
            }
            // SAFETY: as in `push`; only this thread (the worker) frees a block.
            let cells = unsafe { std::slice::from_raw_parts(chunk.add(index % TAKE_CHUNK * FRAME_CELLS), FRAME_CELLS) };
            let read = |at: usize| frame_of(std::array::from_fn(|cell| cells[at + cell].load(Ordering::Acquire)));
            track.push([read(0), read(STATE_CELLS)]);
            // A block past the first is done with once its last frame is out: a long track is never held twice.
            if index >= TAKE_CHUNK && (index + 1) % TAKE_CHUNK == 0 {
                free_block(table, slot);
            }
        }
        (!track.is_empty()).then_some(track)
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
        Spot { at: [x, y], facing, kind: -1, pose: PackedPose::NONE }
    }

    /// The recorded (or selected) fighter walks right and up from (20, 0) and turns left on frame 6; the other one
    /// stands 40 to its left at the start and falls: every x, y and facing is its own.
    fn state(fighter: usize, i: u32) -> ReplayFrame {
        let i = i as f32;
        let (x, y, facing) = if fighter == 0 { (20.0 + 3.0 * i, i, if i >= 6.0 { -1.0 } else { 1.0 }) } else { (-20.0 - i, 50.0 - 2.0 * i, 1.0) };
        ReplayFrame { pose: crate::engine::macros::PackedPose::NONE, input: InputFrame::default(), start: StartState { status: 14, facing, ..StartState::neutral(18, 0, 1.0) }, x, y }
    }

    fn recorded_spot(fighter: usize, frame: u32, x: f32, y: f32, facing: f32) -> Spot {
        let recorded = state(fighter, frame);
        Spot { kind: recorded.start.fighter_kind, pose: recorded.pose, ..spot(x, y, facing) }
    }

    fn capture(tracks: usize) -> ClipEditor {
        let tracks = (0..tracks).map(|fighter| ReplayTrack { fighter: None, entry: fighter as i32,
            frames: (0..10).map(|i| ReplayFrame { pose: crate::engine::macros::PackedPose::NONE, input: InputFrame { buttons: i, ..InputFrame::default() }, ..state(fighter, i) }).collect(),
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
    fn the_request_carries_each_fighters_kind_and_animation() {
        let poses = [PackedPose([11, 0x102]), PackedPose([12, 0x203]),
            PackedPose([13, 0x304]), PackedPose([14, 0x405])];
        let mut selected = [state(0, 0), state(0, 1)];
        let mut other = [state(1, 0), state(1, 1)];
        selected[0].start.fighter_kind = 6;
        selected[1].start.fighter_kind = 7;
        other[0].start.fighter_kind = 74;
        other[1].start.fighter_kind = 75;
        selected[0].pose = poses[0];
        selected[1].pose = poses[1];
        other[0].pose = poses[2];
        other[1].pose = poses[3];
        let ask = |at| deltas(|i| selected.get(i as usize).copied(), |i| other.get(i as usize).copied(), 0, at, 0);
        let block = Block::new();
        assert_eq!(block.shown(), None);
        for (frame, kinds, expected) in [(0, [6, 74], [poses[0], poses[2]]), (1, [7, 75], [poses[1], poses[3]])] {
            block.ask_at(Some(frame), ask(frame));
            assert_eq!(block.shown(), Some(frame));
            let Ask::Group { deltas, .. } = block.ask() else { panic!("no group") };
            for slot in 0..2 {
                let delta = deltas[slot].unwrap();
                assert_eq!((delta.kind, delta.pose), (kinds[slot], expected[slot]));
                let held = block.held_at(slot, [5.0, 8.0]).unwrap();
                assert_eq!((held.kind, held.pose), (kinds[slot], expected[slot]));
                assert_eq!(held.at, [5.0 + delta.at[0], 8.0 + delta.at[1]]);
            }
        }
        assert_eq!(block.pose(0), (POSE_NONE, PackedPose::NONE));
        assert_eq!(block.pose(1), (POSE_NONE, PackedPose::NONE));
        block.set_pose(0, POSE_SHOWN, poses[1]);
        block.set_pose(1, POSE_OTHER, poses[3]);
        block.set_pose(2, POSE_SHOWN, poses[0]);
        assert_eq!(block.pose(0), (POSE_SHOWN, poses[1]));
        assert_eq!(block.pose(1), (POSE_OTHER, poses[3]));
        assert_eq!(block.pose(2), (POSE_NONE, PackedPose::NONE));
        block.forget();
        assert_eq!(block.pose(0), (POSE_NONE, PackedPose::NONE));
        assert_eq!(block.pose(1), (POSE_NONE, PackedPose::NONE));
        block.ask_at(None, Ask::None);
        assert_eq!(block.shown(), None);
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
                Some(recorded_spot(1, frame, state(1, frame).x - 20.0, state(1, frame).y, 1.0)),
                Some(recorded_spot(0, frame, 3.0 * frame as f32, frame as f32, state(0, frame).start.facing)),
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
            assert_eq!(clip_help(&editor.view(), "ZL", "ZL", 0).notice, "Play it once to see positions while scrubbing.");
            let line = crate::engine::macro_ui::clip_marks::bridge_clip(&editor.view(), Some("macro.p1"), false).text();
            assert!(line.contains(" running=0 pos=no_track target="), "{line}");
        }
        // X does not stand in for the missing track.
        assert!(matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::PreviewFile { .. }));
        editor.pause();
        editor.on_action(ClipAction::SelectIn);
        assert_eq!(editor.scrub(), (Ask::None, Pos::NoTrack));
        for pos in [Pos::None, Pos::NoTrack, Pos::NoMoves, Pos::Held, Pos::Off, Pos::End] {
            assert!(pos.text().chars().count() <= 67, "{pos:?}");
        }
        assert_eq!([Pos::None, Pos::NoTrack, Pos::NoMoves, Pos::Held, Pos::Off, Pos::End].map(Pos::word), ["none", "no_track", "held", "held", "off", "end"]);
    }

    #[test]
    fn the_anchor_is_the_opening_in_and_never_follows_a_handle() {
        // Home x 10, the anchor's sample 20, In moved to a sample of 35 shows 25.
        let mut editor = capture(1);
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(5));
        let block = Block::new();
        block.set_ask(editor.scrub().0);
        assert_eq!(block.held_at(1, [10.0, 0.0]).map(|spot| spot.at[0]), Some(25.0));
        // Moving In again, X and reselecting do not rebase it.
        editor.on_action(ClipAction::Nudge(1));
        assert!(matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::Preview(_)));
        editor.pause();
        editor.on_action(ClipAction::SelectIn);
        block.set_ask(editor.scrub().0);
        assert_eq!(block.held_at(1, [10.0, 0.0]), Some(recorded_spot(0, 6, 28.0, 6.0, -1.0)));
        editor.on_action(ClipAction::SelectOut);
        block.set_ask(editor.scrub().0);
        assert_eq!(block.held_at(1, [10.0, 0.0]), Some(recorded_spot(0, 9, 37.0, 9.0, -1.0)));
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
        assert_eq!(block.held_at(1, home), Some(recorded_spot(0, 4, 112.0, 11.0, 1.0)));
        assert_eq!(block.held_at(0, home), Some(recorded_spot(1, 4, 100.0 + (-24.0 - 20.0), 7.0 + 42.0, 1.0)));
        // A capture with one track moves one body.
        let mut single = capture(1);
        single.on_action(ClipAction::SelectIn);
        assert!(matches!(single.scrub().0, Ask::Group { target: 1, deltas: [None, Some(_)] }));
        // ZR: the selected fighter plays on Player 1, and the group follows.
        editor.on_action(ClipAction::SwapTarget);
        if editor.scrub().0 == Ask::None {
            editor.on_action(ClipAction::SelectIn);
        }
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
        assert!(runtime.contains("if !in_bounds(spot.at, limits) || "));
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
        assert_eq!(take.take(2).map(|track| track.len()), Some(2), "never more states than frames");
        // Every field of the capture's state is kept.
        let odd = ReplayFrame { pose: crate::engine::macros::PackedPose::NONE, input: InputFrame::default(), x: -88.5, y: 12.25,
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
        assert!(states.contains("let frame = crate::runtime::macros::replay_capture::state(")
            && states.contains("if capture { crate::engine::macro_ui::scrub_track::TAKE.live(role.index(), &frame); }"));
    }

    #[test]
    fn the_take_buffer_grows_in_blocks_and_a_gap_ends_the_track() {
        let take = TakeTrack::new();
        take.service(false, 0);
        let fill = |from: usize, to: usize| {
            for index in from..to {
                take.live(0, &state(0, index as u32));
                take.push(index, 0);
            }
        };
        // The worker never came by: the second block is not there, and the game thread does not make one. The track
        // ends there and keeps what it has; the take's inputs go on.
        fill(0, TAKE_CHUNK + 3);
        let track = take.take(TAKE_CHUNK + 3).expect("the first block's frames");
        assert_eq!(track.len(), TAKE_CHUNK);
        assert_eq!(track[TAKE_CHUNK - 1][0].x, state(0, (TAKE_CHUNK - 1) as u32).x);
        // A block that comes late does not join frames across the gap.
        take.service(true, TAKE_CHUNK + 3);
        fill(TAKE_CHUNK + 3, TAKE_CHUNK + 6);
        assert_eq!(take.take(TAKE_CHUNK + 6).map(|track| track.len()), Some(TAKE_CHUNK));
        // With the worker one block ahead of the take, it goes on.
        fill(0, TAKE_CHUNK - 1);
        take.service(true, TAKE_CHUNK - 1);
        fill(TAKE_CHUNK - 1, TAKE_CHUNK + 5);
        let track = take.take(TAKE_CHUNK + 5).expect("a whole track");
        assert_eq!((track.len(), track[TAKE_CHUNK + 4][0].x), (TAKE_CHUNK + 5, state(0, (TAKE_CHUNK + 4) as u32).x));
        // A block past the first is freed as soon as its frames are out: a second read finds only the first block.
        fill(0, TAKE_CHUNK - 1);
        take.service(true, TAKE_CHUNK - 1);
        fill(TAKE_CHUNK - 1, 2 * TAKE_CHUNK - 1);
        take.service(true, 2 * TAKE_CHUNK - 1);
        fill(2 * TAKE_CHUNK - 1, 2 * TAKE_CHUNK + 1);
        assert_eq!(take.take(2 * TAKE_CHUNK + 1).map(|track| track.len()), Some(2 * TAKE_CHUNK + 1));
        assert_eq!(take.take(2 * TAKE_CHUNK + 1).map(|track| track.len()), Some(TAKE_CHUNK));
        // Saved and idle: the blocks past the first are freed; the first stays for the next take's first frames.
        take.service(false, 0);
        fill(0, TAKE_CHUNK + 1);
        assert_eq!(take.take(TAKE_CHUNK + 1).map(|track| track.len()), Some(TAKE_CHUNK));
        assert_eq!(TakeTrack::new().take(5), None, "a take with no state kept has no track");
        assert_eq!(TAKE_CHUNK * FRAME_CELLS * 4, 52_800);
        assert_eq!(TAKE_CHUNKS * TAKE_CHUNK, MAX_FRAMES);
        assert!(include_str!("scrub_track.rs").contains("Self { table: AtomicUsize::new(NO_BLOCK) }"), "initialised data, not BSS");
    }

    #[test]
    fn a_take_keeps_each_frames_animation() {
        let take = TakeTrack::new();
        take.service(true, 0);
        for i in 0..=TAKE_CHUNK {
            for slot in 0..2 {
                let mut frame = state(slot, i as u32);
                frame.pose = PackedPose([i as u32 + 1 + slot as u32 * 1_000, ((i as u32 + 1) << 8) | slot as u32]);
                take.live(slot, &frame);
            }
            take.push(i, 0);
        }
        let rows = take.take(TAKE_CHUNK + 1).unwrap();
        assert_eq!(rows.len(), TAKE_CHUNK + 1);
        for i in [0, TAKE_CHUNK - 1, TAKE_CHUNK] {
            for (slot, frame) in rows[i].iter().enumerate() {
                assert_eq!(frame.pose, PackedPose([i as u32 + 1 + slot as u32 * 1_000, ((i as u32 + 1) << 8) | slot as u32]));
            }
        }
    }

    #[test]
    fn a_failed_block_saves_a_playable_macro_with_a_shorter_track() {
        let take = TakeTrack::new();
        take.service(false, 0);
        let frames = TAKE_CHUNK + 5;
        for index in 0..frames {
            if index == TAKE_CHUNK - 1 {
                // The worker asks for the next block, and the allocation fails.
                take.service_with(true, index, || None);
            }
            take.live(0, &state(0, index as u32));
            take.live(1, &state(1, index as u32));
            take.push(index, 0);
        }
        let track = take.take(frames).expect("the frames before the failure");
        assert_eq!(track.len(), TAKE_CHUNK);
        // The take keeps every input and is saved with the shorter track.
        let mut recording = MacroRecording::new("Long", "13.0.5", StartState::neutral(18, 0, 1.0),
            (0..frames as u32).map(|i| InputFrame { buttons: i, ..InputFrame::default() }).collect()).unwrap();
        recording.set_track(Some(track));
        recording.validate().expect("a shorter track is valid");
        let scratch = crate::test_scratch::Scratch::new("tr-scrub-short");
        let dir = scratch.join("root");
        let library = crate::engine::macros::Library::new(&dir);
        let id = library.save_new(&recording).unwrap();
        let back = library.load_edited(&id).unwrap();
        assert_eq!((back.frames.len(), back.track.as_ref().map(Vec::len), back.schema_version), (frames, Some(TAKE_CHUNK), 1));
        // In the editor a frame with its state holds the fighters; past the track's end nobody is held.
        let mut editor = ClipEditor::macro_file(back, true).unwrap();
        editor.on_action(ClipAction::SelectIn);
        assert!(matches!(editor.scrub(), (Ask::Group { .. }, Pos::Held)));
        editor.on_action(ClipAction::SelectOut);
        assert_eq!(editor.scrub(), (Ask::None, Pos::End));
        assert_eq!(clip_help(&editor.view(), "ZL", "ZL", 0).notice, "Positions end here.");
        editor.on_action(ClipAction::Nudge(-5));
        assert!(matches!(editor.scrub(), (Ask::Group { .. }, Pos::Held)), "Out - 1 is the track's last frame");
        // A crop wholly past the track's end is still a take whose positions ran out, not an old macro: its
        // positions file has no rows. Reopened, it holds nobody and says so.
        let mut tail = recording.clone();
        tail.keep(TAKE_CHUNK + 1..frames);
        assert_eq!((tail.track.as_ref().map(Vec::len), tail.schema_version), (Some(0), 1));
        tail.validate().expect("an empty track is valid");
        let tail = library.load_edited(&library.save_new(&tail).unwrap()).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!((tail.track.as_ref().map(Vec::len), tail.schema_version), (Some(0), 1), "a positions file with no rows");
        let mut reopened = ClipEditor::macro_file(tail, false).unwrap();
        for action in [ClipAction::SelectIn, ClipAction::SelectOut] {
            reopened.on_action(action);
            assert_eq!(reopened.scrub(), (Ask::None, Pos::End));
            assert_eq!(clip_help(&reopened.view(), "ZL", "ZL", 0).notice, "Positions end here.");
        }
        // A crop across the end keeps the part it has.
        let mut across = recording.clone();
        across.keep(TAKE_CHUNK - 2..frames);
        assert_eq!(across.track.as_ref().map(Vec::len), Some(2));
        across.validate().unwrap();
    }

    #[test]
    fn a_takes_positions_fit_its_frames_and_stay_out_of_its_file() {
        let old = recording(false);
        let new = recording(true);
        new.validate().unwrap();
        // The macro's file is the same with and without positions: versions 1 and 2 only, no key.
        assert_eq!((new.schema_version, serde_json::to_string(&new).unwrap()), (1, serde_json::to_string(&old).unwrap()));
        let back: MacroRecording = serde_json::from_str(&serde_json::to_string(&new).unwrap()).unwrap();
        assert_eq!(back, old);
        let broken: [fn(&mut MacroRecording); 2] = [
            |recording| { let extra = recording.track.as_ref().unwrap()[0]; recording.track.as_mut().unwrap().push(extra); },
            |recording| recording.track.as_mut().unwrap()[0][1].x = f32::NAN,
        ];
        for (index, edit) in broken.iter().enumerate() {
            let mut bad = new.clone();
            edit(&mut bad);
            assert!(bad.validate().is_err(), "edit {index}");
        }
        let mut version = new.clone();
        version.schema_version = 3;
        assert!(version.validate().is_err(), "there is no version 3");
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
        let finish = runtime.split_once("pub fn finish_recording(").unwrap().1.split_once("\n}").unwrap().0;
        assert!(finish.contains("capture.track.borrow_mut().take()") && finish.contains("track.drain(..kept.start.min(track.len()));"));
        assert!(!finish.contains("to_vec()"), "the track is cut in place, not copied");
    }

    #[test]
    fn a_capture_saved_as_a_macro_carries_both_tracks() {
        let mut editor = capture(2);
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(2));
        editor.on_action(ClipAction::SelectOut);
        editor.on_action(ClipAction::Nudge(-1));
        // In 2, Out 9: seven frames, saved with A.
        let ClipEffect::Export(saved) = editor.on_action(ClipAction::Confirm) else { panic!("expected an export") };
        assert_eq!((saved.frames.len(), saved.schema_version), (7, 1));
        assert_eq!(saved.track, Some((2..9).map(|i| [state(0, i), state(1, i)]).collect()));
        saved.validate().unwrap();
        // Reopened from its file, it scrubs both bodies at the capture's spacing, with no preview.
        let scratch = crate::test_scratch::Scratch::new("tr-scrub-capture");
        let dir = scratch.join("root");
        let library = crate::engine::macros::Library::new(&dir);
        let file = library.load_edited(&library.save_new(&saved).unwrap()).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let mut reopened = ClipEditor::macro_file(file, false).unwrap();
        reopened.on_action(ClipAction::SelectOut);
        let block = Block::new();
        block.set_ask(reopened.scrub().0);
        // The file's last frame is the capture's frame 8; its anchor is the capture's frame 2, at (26, 2).
        assert_eq!(block.held_at(1, [0.0, 0.0]), Some(recorded_spot(0, 8, 44.0 - 26.0, 8.0 - 2.0, -1.0)));
        assert_eq!(block.held_at(0, [0.0, 0.0]), Some(recorded_spot(1, 8, -28.0 - 26.0, 34.0 - 2.0, 1.0)));
        // A capture with one fighter saves that fighter's track; the other one is marked absent and is never held.
        let mut single = capture(1);
        single.on_action(ClipAction::SelectIn);
        let ClipEffect::Export(saved) = single.on_action(ClipAction::Confirm) else { panic!("expected an export") };
        assert_eq!(saved.track.as_ref().map(|track| (track.len(), track[0][0], track[0][1].start.fighter_kind)), Some((10, state(0, 0), -1)));
        let mut reopened = ClipEditor::macro_file(*saved, false).unwrap();
        reopened.on_action(ClipAction::SelectIn);
        assert!(matches!(reopened.scrub(), (Ask::Group { target: 1, deltas: [None, Some(_)] }, Pos::NoMoves)));
        // The X preview's clip is not given a track: it is played, not saved.
        let mut previewed = capture(2);
        let ClipEffect::Preview(clip) = previewed.on_action(ClipAction::TogglePlay) else { panic!("expected a preview") };
        assert_eq!((clip.track.is_none(), clip.schema_version), (true, 1));
    }

    #[test]
    fn a_playback_book_holds_no_track() {
        // Playback never reads a track, and the book of every macro is loaded at once: it keeps the inputs only.
        let mut book = crate::engine::playback::Book::new(crate::engine::library::RecordingIndex::fresh());
        book.add_recording("tracked", recording(true));
        book.add_recording("old", recording(false));
        let held = format!("{book:?}");
        assert!(held.contains("track: None") && !held.contains("track: Some"), "a book holds no track");
        assert_eq!(held.matches("buttons: 9").count(), 2, "both macros keep their inputs");
    }

    #[test]
    fn closing_during_the_first_hold_keeps_the_release_reachable() {
        let slot = Slot::new();
        assert!(slot.publish(Block::new()).is_ok());
        slot.with(|block| {
            block.set_ask(Ask::Group { target: 0, deltas: [Some(spot(3.0, 4.0, 1.0)), None] });
            let _hold = block.holds.try_lock().unwrap();
            assert_eq!(block.held_at(0, [1.0, 2.0]), Some(spot(4.0, 6.0, 1.0)));
            // Interleave close after the fighter has its point but before it publishes the first hold.
            slot.close();
            block.set_seen(0, SEEN_HELD);
        }).unwrap();
        slot.close();
        assert!(slot.live(), "the next fighter frame must still find its release");
        slot.with(|block| {
            assert_eq!((block.ask(), block.seen(0)), (Ask::None, SEEN_HELD));
            block.set_seen(0, SEEN_IDLE);
        }).unwrap();
        slot.close();
        assert!(slot.empty());
    }

    #[test]
    fn a_swapped_fighter_drops_the_hold_until_the_handle_moves() {
        let block = Block::new();
        let ask = |x: f32| Ask::Group { target: 1, deltas: [Some(spot(x, 0.0, 1.0)), Some(spot(x + 1.0, 0.0, 1.0))] };
        block.set_ask(ask(3.0));
        assert_eq!(block.pos(), Pos::Held);
        // The game side finds a hold made on another body: a hot swap, a new scene.
        block.forget();
        assert_eq!((block.held_at(0, [0.0, 0.0]), block.held_at(1, [0.0, 0.0])), (None, None), "neither body is held");
        assert_eq!(block.pos(), Pos::None);
        // The same frame asked again changes nothing; a moved handle holds again.
        block.set_ask(ask(3.0));
        assert_eq!(block.held_at(1, [0.0, 0.0]), None);
        block.set_ask(ask(4.0));
        assert_eq!(block.held_at(1, [10.0, 0.0]), Some(spot(15.0, 0.0, 1.0)));
        assert_eq!(block.pos(), Pos::Held);
        // The key line and the bridge follow what the game side said.
        let mut editor = capture(2);
        editor.on_action(ClipAction::SelectIn);
        assert_eq!(editor.view().pos, Pos::Held);
        assert!(editor.note_scrub(Pos::None));
        assert_eq!(editor.view().pos, Pos::None);
        assert!(!editor.note_scrub(Pos::None));
        assert!(editor.note_scrub(Pos::Off));
        assert_eq!(editor.view().pos, Pos::Off);
        assert!(editor.note_scrub(Pos::Held));
        assert_eq!(editor.view().pos, Pos::Held);
        let runtime = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/features/scrub_hold.rs")).unwrap_or_default();
        assert!(runtime.contains("if holds[slot].is_some_and(|hold| hold.body() != natives::battle_object_id(accessor)) {"));
        assert!(runtime.contains("block.forget();"));
    }

    #[test]
    fn a_dropped_hold_wakes_on_another_frame_with_the_same_points() {
        // An idle take: every frame has the same points.
        let ask = Ask::Group { target: 1, deltas: [Some(spot(0.0, 0.0, 1.0)), Some(spot(2.0, 0.0, 1.0))] };
        let block = Block::new();
        block.ask_at(Some(5), ask);
        assert_eq!(block.pos(), Pos::Held);
        block.forget();
        block.ask_at(Some(5), ask);
        assert_eq!((block.held_at(1, [10.0, 0.0]), block.pos()), (None, Pos::None), "the same frame again holds nobody");
        // One nudge: the next frame, the same points.
        block.ask_at(Some(6), ask);
        assert_eq!((block.held_at(1, [10.0, 0.0]), block.pos()), (Some(spot(12.0, 0.0, 1.0)), Pos::Held));
        // Showing a frame again does not publish again.
        let generation = block.ask.generation();
        block.ask_at(Some(6), ask);
        assert_eq!(block.ask.generation(), generation);
        // No frame shown: nothing asked.
        block.ask_at(None, Ask::None);
        assert_eq!(block.pos(), Pos::None);
        let worker = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/macros/replay_capture.rs")).unwrap();
        assert!(worker.contains("block.ask_at(shown, ask);"), "the worker names the shown frame");
    }

    #[test]
    fn the_sampler_reads_nothing_with_no_take() {
        let root = env!("CARGO_MANIFEST_DIR");
        // A fighter frame reads the nine values only for a take or a generated track.
        let states = std::fs::read_to_string(format!("{root}/src/runtime/features/states.rs")).unwrap();
        let gate = states.split_once("let capture = crate::engine::rr_host::capture_role() != 0;").unwrap().1
            .split_once("if !crate::engine::rr_host::fighter_frame(").unwrap().0;
        assert!(gate.contains("let play = crate::engine::rr_host::play_sampling();"));
        assert!(gate.contains("if capture || play {"));
        assert!(gate.contains("if capture { crate::engine::macro_ui::scrub_track::TAKE.live(role.index(), &frame); }"));
        assert!(gate.contains("if play { crate::engine::rr_host::play_live(role.index(), &frame); }"));
        assert_eq!(states.matches("replay_capture::state(").count(), 1);
        // So a take's start reads both bodies once: its first frame has its state.
        let host = std::fs::read_to_string(format!("{root}/src/runtime/features/recording_host.rs")).unwrap();
        assert!(host.contains("TAKE.seed(slot, frame)"));
        assert!(host.contains("store(slot, frame);"));
        let chords = host.split_once("pub fn on_chords(").unwrap().1.split_once("\n}").unwrap().0;
        assert!(chords.contains("seed_take_track(Some(cpu));"), "the Record chord seeds the take");
        let picker = std::fs::read_to_string(format!("{root}/src/runtime/features/record_place.rs")).unwrap();
        assert!(picker.contains("super::recording_host::seed_take_track(recorder::cpu_body());"), "a placed start seeds the take");
        // A fighter that never had a frame is marked absent, not a fighter of kind 0 at the origin.
        let take = TakeTrack::new();
        take.service(false, 0);
        take.live(0, &state(0, 0));
        take.push(0, 0);
        assert_eq!(take.take(1).map(|track| track[0][1].start.fighter_kind), Some(-1));
    }

    #[test]
    fn a_new_take_does_not_reuse_the_previous_takes_missing_partner() {
        let take = TakeTrack::new();
        take.service(false, 0);
        for recorded in [0, 1] {
            let partner = 1 - recorded;
            take.seed(recorded, Some(state(recorded, 4)));
            take.seed(partner, Some(state(partner, 4)));
            take.push(0, recorded);
            assert_eq!(take.take(1).unwrap()[0][1], state(partner, 4));

            take.service(false, 0);
            take.seed(recorded, Some(state(recorded, 9)));
            take.seed(partner, None);
            take.push(0, recorded);
            let pair = take.take(1).unwrap()[0];
            assert_eq!(pair[0], state(recorded, 9));
            assert_eq!(pair[1].start.fighter_kind, -1, "frame zero inherited a partner from another take");
        }
    }

    #[test]
    fn preview_release_proof_keeps_a_partner_held_from_an_earlier_row() {
        let slot = Slot::new();
        assert!(slot.publish(Block::new()).is_ok());
        let place = Some(spot(3.0, 4.0, 1.0));
        slot.with(|b| {
            b.ask_at(Some(4), Ask::Group { target: 0, deltas: [place; 2] });
            for role in 0..2 {
                b.set_seen(role, SEEN_HELD);
                b.ack(role, b.generation());
            }
            // The CPU has not run since row 4. The next row has no partner.
            b.ask_at(Some(5), Ask::Group { target: 0, deltas: [place, None] });
            b.ask_at(None, Ask::None);
            b.set_seen(0, SEEN_IDLE);
            b.ack(0, b.generation());
        });
        assert!(!slot.released(), "the CPU is still holding its row-4 point");
        slot.with(|b| { b.set_seen(1, SEEN_IDLE); b.ack(1, b.generation()); });
        assert!(slot.released(), "both bodies reported their releases");
        slot.close();
    }

    #[test]
    fn preview_release_proof_sees_a_first_callback_that_has_not_reported_yet() {
        let slot = Slot::new();
        assert!(slot.publish(Block::new()).is_ok());
        let place = Some(spot(3.0, 4.0, 1.0));
        slot.with(|b| b.ask_at(Some(4), Ask::Group { target: 0, deltas: [None, place] }));
        slot.with(|b| {
            let generation = b.generation();
            assert!(b.held_at(1, [0.0; 2]).is_some());
            // The worker handles X after the CPU has its point, but before its first result.
            b.ask_at(None, Ask::None);
            assert!(!slot.released(), "the first fighter callback can still acquire the body");
            b.set_seen(1, SEEN_HELD);
            b.ack(1, generation);
        });
        assert!(!slot.released(), "the late hold is now observable");
        slot.with(|b| { b.set_seen(1, SEEN_IDLE); b.ack(1, b.generation()); });
        assert!(slot.released(), "the held CPU came home; absent P1 needs no callback");
        slot.close();
    }

    #[test]
    fn a_play_waits_until_every_moved_role_has_let_go() {
        let block = Block::new();
        assert!(block.released(), "nothing was asked");
        let ask = Ask::Group { target: 0, deltas: [Some(spot(1.0, 0.0, 1.0)), Some(spot(2.0, 0.0, 1.0))] };
        block.ask_at(Some(6), ask);
        // Both roles ran a frame under the ask and hold their bodies.
        for slot in 0..2 {
            block.ack(slot, block.generation());
            block.set_seen(slot, SEEN_HELD);
        }
        assert!(!block.released(), "a frame is shown");
        // X: the worker shows no frame. The bodies stay held until each role's next frame.
        let shown = block.generation();
        block.ask_at(None, Ask::None);
        assert!(!block.released());
        // A frame that had read the old ask does not count.
        block.ack(0, shown);
        assert!(!block.released());
        block.ack(0, block.generation());
        assert!(!block.released(), "the CPU has not let go");
        block.ack(1, block.generation());
        assert!(block.released());
        // A role the ask did not move is not waited for.
        let block = Block::new();
        block.ask_at(Some(3), Ask::Group { target: 0, deltas: [Some(spot(1.0, 0.0, 1.0)), None] });
        block.ack(0, block.generation());
        block.ack(1, block.generation());
        block.ask_at(None, Ask::None);
        assert!(!block.released());
        block.ack(0, block.generation());
        assert!(block.released());
        // Nor one that never ran a frame here: it has no body.
        let block = Block::new();
        block.ask_at(Some(3), ask);
        block.ack(0, block.generation());
        block.ask_at(None, Ask::None);
        assert!(!block.released());
        block.ack(0, block.generation());
        assert!(block.released(), "the CPU never ran a frame");
        // The start that follows waits for the released CPU until the game side ends the wait; a new hold ends it too.
        assert!(!block.settling());
        block.await_settle();
        assert!(block.settling());
        block.settled();
        assert!(!block.settling());
        block.await_settle();
        block.ask_at(Some(4), ask);
        assert!(!block.settling() && !block.released());
        // The game side reads the generation before the ask and reports it after its step.
        let read = |path: &str| std::fs::read_to_string(format!("{}{path}", env!("CARGO_MANIFEST_DIR"))).unwrap_or_default();
        let hold = read("/src/runtime/features/scrub_hold.rs");
        let generation = hold.find("let generation = block.generation();").expect("the generation is read");
        assert!(generation < hold.find("point(block, &holds, slot, accessor)").unwrap(), "the generation is read before the ask");
        assert!(hold.contains("block.set_seen(slot, seen);\n        block.ack(slot, generation);"));
        // The worker: X is kept while a body is held, and started only on a pass that finds the bodies released.
        let worker = read("/src/runtime/macros/replay_capture.rs");
        assert!(worker.contains("if matches!(action, ClipAction::TogglePlay | ClipAction::Restart) && hold_play(action, playback) { return; }"));
        let notices = worker.split_once("pub fn worker_preview_notices() {").map_or("", |(_, rest)| rest);
        assert!(notices.trim_start().starts_with("scrub_service();\n    service_wanted_play();"), "the wanted play is looked at on every pass");
        let service = worker.split_once("fn service_wanted_play() {").map_or("", |(_, rest)| rest);
        let released = service.find("SLOT.released()").expect("the release is read");
        assert!(released < service.find("block.await_settle()").unwrap() && released < service.find("editor.take_wanted_play()").unwrap());
        assert!(worker.contains("[Scrub] play wanted") && worker.contains("[Scrub] released"), "both steps are in the trace");
        assert!(!worker.contains("static WANTED"), "the wanted play lives in the editor, not in a static (BSS)");
    }

    #[test]
    fn a_new_shown_frame_rechecks_the_same_point_on_both_roles() {
        use crate::model::stage_geometry::{classify, Copied, Met};
        let block = Block::new();
        let ask = Ask::Group { target: 0, deltas: [Some(spot(0.0, 0.0, 1.0)); 2] };
        let permits = |floor_y| {
            let floor = [Met { ends: [[-20.0, floor_y], [20.0, floor_y]], stand: true, through: false }];
            can_stand(classify(&mut Copied(&floor), [-100.0, 100.0, 100.0, -100.0], [0.0, 0.0]))
        };
        block.ask_at(Some(10), ask);
        for slot in 0..2 {
            assert!(block.standable(slot, [0.0, 0.0], || permits(-4.0)));
        }
        block.ask_at(Some(10), ask);
        for slot in 0..2 {
            assert!(block.standable(slot, [0.0, 0.0], || panic!("rechecked an unchanged hold")));
        }
        block.ask_at(Some(11), ask);
        for slot in 0..2 {
            assert!(!block.standable(slot, [0.0, 0.0], || permits(4.0)), "frame 11: role {slot} is inside the floor");
            assert!(!block.standable(slot, [0.0, 0.0], || panic!("rechecked an unchanged refusal")));
        }
    }

    /// P3 (tr-primary-2 2026-11-26 16:19:09Z): the target role is let go while its partner stays held. The target's
    /// published position is read at the top of its fighter frame, before the hold's step, so on that frame it is
    /// still the point the target was held at. The partner's point is traced callback by callback.
    #[test]
    fn the_groups_home_stays_when_the_target_is_let_go() {
        let block = Block::new();
        let home = [-30.0, 0.5];
        let group = |target: [f32; 2], partner: [f32; 2]| Ask::Group {
            target: 0,
            deltas: [Some(spot(target[0], target[1], 1.0)), Some(spot(partner[0], partner[1], -1.0))],
        };
        let at = |point: Option<Spot>| point.map(|spot| spot.at);
        // Frame 136: both are held. The target's hold found it at home.
        block.ask_at(Some(136), group([12.0, 30.0], [-40.0, 0.0]));
        assert_eq!(at(block.point(0, |_| Some(home), |_| panic!("a held target has an origin"))), Some([-18.0, 30.5]));
        assert_eq!(at(block.point(1, |_| Some(home), |_| panic!("a held target has an origin"))), Some([-70.0, 0.5]));
        // Frame 137: the stage refuses the target's point and its hold is gone. Its position as published on this
        // frame is still where it was held.
        block.ask_at(Some(137), group([4.0, -8.0], [-52.0, -15.0]));
        let held = [-18.0, 30.5];
        assert_eq!(at(block.point(1, |_| None, |_| Some(held))), Some([-82.0, -14.5]), "the partner's first callback after the target is let go");
        assert_eq!(at(block.point(0, |_| None, |_| Some(held))), Some([-26.0, -7.5]), "the target is asked from home too");
        for _ in 0..3 {
            assert_eq!(at(block.point(1, |_| None, |_| Some(home))), Some([-82.0, -14.5]));
        }
        // Nobody is asked: the home goes with the holds. The next group starts from where the target stands then.
        block.ask_at(None, Ask::None);
        assert_eq!(block.point(1, |_| panic!("nobody is asked"), |_| panic!("nobody is asked")), None);
        block.ask_at(Some(137), group([4.0, -8.0], [-52.0, -15.0]));
        assert_eq!(at(block.point(1, |_| None, |_| Some([10.0, 0.5]))), Some([-42.0, -14.5]));
        // Another target role does not take the first one's home.
        assert_eq!(at(block.point(0, |_| Some(home), |_| None)), Some([-26.0, -7.5]));
        block.ask_at(Some(138), Ask::Group { target: 1, deltas: [Some(spot(1.0, 0.0, 1.0)), Some(spot(0.0, 0.0, 1.0))] });
        assert_eq!(at(block.point(0, |_| None, |_| Some([5.0, 0.5]))), Some([6.0, 0.5]));
        // A hold made on another body is forgotten, and its home with it.
        block.ask_at(Some(139), group([0.0, 0.0], [1.0, 0.0]));
        assert_eq!(at(block.point(1, |_| Some(home), |_| None)), Some([-29.0, 0.5]));
        block.forget();
        // The replacing body's callback asks for its point before release drops the old hold.
        assert_eq!(block.point(0, |_| Some(home), |_| None), None);
        block.ask_at(Some(140), group([0.0, 0.0], [1.0, 0.0]));
        assert_eq!(at(block.point(1, |_| None, |_| Some([7.0, 0.5]))), Some([8.0, 0.5]), "use the replacement's home");
    }

    /// P3: the worker publishes a new request between `forget()` and `point` inside one fighter callback. The callback
    /// still has the hold of the body that is gone in its slot. It lets go on that callback, and the new request is
    /// acted on from the next one, from where the new body stands; the old body's home is neither used nor kept.
    #[test]
    fn a_request_published_while_a_callback_forgets_never_uses_the_old_bodys_home() {
        let block = Block::new();
        let at = |point: Option<Spot>| point.map(|spot| spot.at);
        let group = Ask::Group { target: 0, deltas: [Some(spot(2.0, 0.0, 1.0)), Some(spot(-40.0, 0.0, -1.0))] };
        let (old, new) = ([-30.0, 0.5], [16.0, 0.5]);
        block.ask_at(Some(139), group);
        assert_eq!(at(block.point(0, |_| Some(old), |_| None)), Some([-28.0, 0.5]));
        // The target's body is another: its callback forgets, and before it asks for its point a new frame is shown.
        block.forget();
        block.ask_at(Some(140), group);
        assert_eq!(block.point(0, |_| Some(old), |_| Some(new)), None, "the callback that forgot lets go");
        // From the next callback the request is acted on, from where the new body stands.
        assert_eq!(at(block.point(0, |_| None, |_| Some(new))), Some([18.0, 0.5]));
        assert_eq!(at(block.point(1, |_| None, |_| Some(new))), Some([-24.0, 0.5]), "the old body's home is not kept");
        // The same with the new body refused where it is asked: no hold of it, and still not the old home.
        block.ask_at(Some(141), group);
        assert_eq!(at(block.point(0, |_| Some(new), |_| None)), Some([18.0, 0.5]));
        block.forget();
        block.ask_at(Some(142), group);
        assert_eq!(block.point(1, |_| Some(new), |_| Some(old)), None, "the partner's callback forgot");
        for _ in 0..3 {
            assert_eq!(at(block.point(0, |_| None, |_| Some(old))), Some([-28.0, 0.5]));
            assert_eq!(at(block.point(1, |_| None, |_| Some(old))), Some([-70.0, 0.5]));
        }
    }

    /// P3 (tr-primary-2 2026-11-26 13:43:10Z, 13:47:51Z; rv-preview-vis 13:44:24Z, 16:20:59Z): a body the game pushes
    /// off its point is refused there, for that shown request and that body, and is not taken again.
    #[test]
    fn a_body_the_game_pushes_off_its_point_is_refused_for_that_request_and_body() {
        let block = Block::new();
        let ask = Ask::Group { target: 0, deltas: [Some(spot(0.0, 0.0, 1.0)), Some(spot(-51.0, -15.0, -1.0))] };
        let (wall, open) = ([-81.0, -3.5], [-87.0, -3.5]);
        let (player, cpu, other) = (0x100, 0x200, 0x300);
        block.ask_at(Some(137), ask);
        let request = block.generation();
        // The first frame of a hold writes. Nothing of this request has been judged yet, whatever the hold reports.
        assert!(!block.pushed(1, request, cpu, wall, Some([-81.5, -3.5])), "the write judged was not this request's");
        assert!(!block.refused(1, request, cpu, wall));
        // The next frame reads the body off the point this request wrote.
        assert!(block.pushed(1, request, cpu, wall, Some([-81.5, -3.5])), "pushed off its own write");
        // Ten unchanged callbacks: refused. The other role is held at its point throughout.
        for _ in 0..10 {
            assert!(block.refused(1, request, cpu, wall));
            assert!(!block.refused(0, request, player, open) && !block.pushed(0, request, player, open, None));
        }
        // Another body at the same point under the same request gets its own try.
        assert!(!block.refused(1, request, other, wall));
        // A new shown frame with the same points is another request.
        block.ask_at(Some(138), ask);
        let next = block.generation();
        assert_ne!(next, request);
        assert!(!block.refused(1, next, cpu, wall));
        // The body is written at the wall again; before the next frame the shown frame moves it to open air. The
        // hold's report on that frame is about the wall's write and says nothing of the open point.
        assert!(!block.pushed(1, next, cpu, wall, None));
        block.ask_at(Some(139), ask);
        let moved = block.generation();
        assert!(!block.pushed(1, moved, cpu, open, Some([-81.5, -3.5])), "the write judged was the wall's");
        assert!(!block.pushed(1, moved, cpu, open, None) && !block.refused(1, moved, cpu, open), "exact in open air: held");
        // A request published during the callback: the callback had read the older generation before the ask. The
        // refusal is the older request's; the new one asks afresh.
        assert!(!block.pushed(1, moved, cpu, wall, None));
        block.ask_at(Some(140), ask);
        assert!(block.pushed(1, moved, cpu, wall, Some([-81.5, -3.5])));
        assert!(block.refused(1, moved, cpu, wall) && !block.refused(1, block.generation(), cpu, wall));
    }

    /// P3, the ruled test (tr-primary-2 2026-11-26 17:47:24Z): the scrub refuses a body the game moved sideways, its
    /// x not bit for bit the x asked; a body the floor takes straight up or down stays held. The reports are the
    /// shared hold's (`CpuPreview::observe`), on the words its seam read in a game where there are any.
    #[test]
    fn a_sideways_push_refuses_and_a_vertical_take_holds() {
        use crate::model::crosshair::{hold_step, BodyRead, Origin};
        // The hold's fall step and the asked height of the wall rows (Eden, run place-float-xps-05).
        let fall = [0.0, f32::from_bits(0xbd83_126f)];
        let y = f32::from_bits(0xc061_adaa);
        let air = |at: [f32; 2]| BodyRead { at, landed: false, speed: Some(fall), frame: 0 };
        let landed = |at: [f32; 2]| BodyRead { at, landed: true, speed: Some([0.0; 2]), frame: 0 };
        // What the hold reports on the frame after it wrote `asked`, the game having left the body as `read`.
        let report = |asked: [f32; 2], read: BodyRead| {
            let mut slot = None;
            hold_step(&mut slot, 7, false, || Origin { at: [0.0, 0.01], grounded: true, facing: 1.0 });
            let hold = slot.as_mut().unwrap();
            hold.observe(asked, BodyRead { frame: 1, ..air(asked) }, false);
            hold.observe(asked, BodyRead { frame: 2, ..read }, false);
            hold.displaced.map(|moved| moved.at)
        };
        // The scrub's verdict on the second frame of a hold at `asked`: let go, or held.
        let let_go = |asked: [f32; 2], read: BodyRead| {
            let block = Block::new();
            block.ask_at(Some(137), Ask::Group { target: 0, deltas: [None, Some(spot(0.0, 0.0, 1.0))] });
            let request = block.generation();
            assert!(!block.pushed(1, request, 0x200, asked, None));
            let moved = report(asked, read);
            (moved.is_some(), block.pushed(1, request, 0x200, asked, moved), block.refused(1, request, 0x200, asked))
        };
        // Sideways. The seam's words at the long floor's left wall: asked x -81, left at x -81.55, still in the air.
        let wall = [f32::from_bits(0xc2a3_1bfc), f32::from_bits(0xc065_2266)];
        assert_eq!(let_go([-81.0, y], air(wall)), (true, true, true), "a wall pushed it");
        // A body the game threw out of a solid: its speed carries the push; its x is not the x asked.
        let thrown = BodyRead { at: [-85.852_455, -9.132_209], landed: false, speed: Some([-45.752_274, -0.706_175_8]), frame: 0 };
        assert_eq!(let_go([-40.100_18, -8.426_033], thrown), (true, true, true));
        // Free of the wall, one unit further out: the read is the point plus the fall step, and nothing is reported.
        assert_eq!(let_go([-82.0, y], air([-82.0, y + fall[1]])), (false, false, false), "exact in open air");
        // Vertical. A fighter standing where it was recorded is held as an air hold at the floor's height. Which of
        // these the game reads there is not read in a game yet; none of them lets the body go.
        let stand = [-30.0, 0.01];
        assert_eq!(let_go(stand, landed(stand)), (true, false, false), "landed on the floor at its point");
        assert_eq!(let_go(stand, air(stand)), (true, false, false), "kept in the air at the floor's height");
        assert_eq!(let_go(stand, BodyRead { speed: Some([0.0; 2]), ..air(stand) }), (false, false, false), "at rest");
        // A point within one fall step over the floor: the floor takes the body, a few hundredths under its point.
        assert_eq!(let_go([-30.0, 0.05], landed(stand)), (true, false, false), "taken by the floor: held, shown on it");
        // Minus zero is the same place as zero, for the hold and for the scrub: a fighter standing at x 0 (the CPU's
        // Training home) is not let go on such a read, reported or not.
        assert_eq!(let_go([0.0, y], air([-0.0, y + fall[1]])), (false, false, false));
        assert_eq!(let_go([0.0, 0.01], landed([-0.0, 0.01])), (true, false, false));
        // The smallest step of x away is another place.
        let beside = f32::from_bits(0xc2a2_0001);
        assert_eq!(let_go([-81.0, y], air([beside, y + fall[1]])), (true, true, true), "one bit of x");
        // The game side hands the hold's report over, and asks for the refusal before it takes the body.
        let runtime = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/features/scrub_hold.rs")).unwrap_or_default();
        let hold = runtime.split_once("unsafe fn hold(").map_or("", |(_, rest)| rest);
        let judged = "if block.pushed(index, generation, body, spot.at, slot.and_then(|hold| hold.displaced).map(|moved| moved.at)) {";
        let order = |texts: &[&str]| texts.iter().map(|text| hold.find(text)).collect::<Vec<_>>().windows(2).all(|pair| pair[0].is_some() && pair[0] < pair[1]);
        assert!(order(&["block.refused(index, generation, body, spot.at)", "placement::hold_at(slot, accessor, body, spot.at, false)", judged, "state::set_lr(accessor, spot.facing);"]));
        assert!(hold.contains(&format!("{judged}\n        release(slot, accessor);\n        return SEEN_OFF;")), "a pushed body is let go and reads off");
        assert!(runtime.contains("Some(spot) => hold(block, generation, slot, &mut holds[slot], accessor, spot),"), "the request the callback read");
        assert!(runtime.contains("block.point(slot, |target| holds[target].map(|hold| hold.origin().at), |target| {"), "the group's home is the model's");
    }

    #[test]
    fn a_point_in_the_ground_or_past_the_blast_lines_holds_nobody() {
        use crate::model::stage_geometry::{classify, Copied, Met};
        // A stage as copied lines: solid ground from -150 to 150 at height 0, and two platforms over it.
        let line = |ends, through| Met { ends, stand: true, through };
        let stage = [line([[-150.0, 0.0], [150.0, 0.0]], false), line([[20.0, 30.0], [60.0, 30.0]], true), line([[20.0, 60.0], [60.0, 60.0]], true)];
        let blast = [-240.0, 240.0, 190.0, -140.0];
        let stands = |at: [f32; 2]| can_stand(classify(&mut Copied(&stage), blast, at));
        // SP-10 (dup-audit, 2026-11-26): a capture's frame 137 with Player 1's home at (-30, 0.01) on the ground and
        // the anchor at (40, 24.129) puts both bodies 15.1 under the ground.
        let block = Block::new();
        block.set_ask(Ask::Group { target: 0, deltas: [Some(spot(11.715 - 40.0, 9.02 - 24.129, -1.0)), Some(spot(-11.715 - 40.0, 9.02 - 24.129, 1.0))] });
        for slot in 0..2 {
            let at = block.held_at(slot, [-30.0, 0.01]).unwrap().at;
            assert!(at[1] < -15.0 && !stands(at), "a capture point inside the ground: {at:?}");
            assert!(!block.standable(slot, at, || stands(at)));
        }
        // A point outside the blast lines, and a stage that cannot be read.
        assert!(!stands([0.0, -139.5]) && !stands([-239.5, 50.0]));
        assert!(!can_stand(Err(Unread)));
        // A macro recorded from the upper platform down to the ground, opened with the fighter on the lower platform:
        // every point moves 30 down, and the ground frame lands in the ground.
        let (anchor, landed, home) = ([40.0, 60.01], [40.0, 0.01], [40.0, 30.01]);
        block.set_ask(Ask::Group { target: 0, deltas: [Some(spot(landed[0] - anchor[0], landed[1] - anchor[1], 1.0)), None] });
        let at = block.held_at(0, home).unwrap().at;
        assert_eq!(at, [40.0, 0.01 - 60.01 + 30.01]);
        assert!(!block.standable(0, at, || stands(at)), "a shifted macro point in the ground");
        // Open ground is held exactly: on the ground, on a platform, in the air, a little under a floor's surface (the
        // rule's slack: the body stands on the floor), and off the stage's side.
        for at in [[-58.285, 0.01], home, [0.0, 50.0], [0.0, -0.5], [-200.0, -15.0]] {
            assert!(stands(at) && block.standable(0, at, || stands(at)), "{at:?}");
        }
        block.set_ask(Ask::Group { target: 0, deltas: [Some(spot(0.0, 0.0, 1.0)), None] });
        assert_eq!(block.held_at(0, home), Some(spot(home[0], home[1], 1.0)));
        // The stage is asked once per point and role: again for another point, and again after the hold is dropped.
        let asked = core::cell::Cell::new(0);
        let ask = |at: [f32; 2]| block.standable(1, at, || { asked.set(asked.get() + 1); stands(at) });
        assert!(ask([0.0, 0.01]) && ask([0.0, 0.01]) && ask([0.0, 0.01]));
        assert_eq!(asked.get(), 1);
        assert!(!ask([0.0, -20.0]) && !ask([0.0, -20.0]));
        assert_eq!(asked.get(), 2);
        block.forget();
        assert!(!ask([0.0, -20.0]));
        assert_eq!(asked.get(), 3);
        // The line is true for a point in the ground and for one past a blast line; the bridge's word is unchanged.
        assert_eq!((Pos::Off.text(), Pos::Off.word()), ("Can't stand there on this stage.", "off"));
        let runtime = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/features/scrub_hold.rs")).unwrap_or_default();
        assert!(runtime.contains("if !in_bounds(spot.at, limits) || block.refused(index, generation, body, spot.at) || !block.standable(index, spot.at, || stands(accessor, spot.at)) {"));
        assert!(runtime.contains("RayProbe::new(accessor).ok_or(Unread).and_then(|mut stage| classify(&mut stage, blast, at))"));
        assert!(!runtime.contains(".lock()"), "the fighter frame takes no blocking lock");
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
            slot.withdraw();
            assert!(!slot.live());
            assert_eq!(slot.with(|_| ()), None);
            assert!(!slot.reap(), "a reader is inside");
            assert!(slot.publish(Block::new()).is_err(), "not before the old block is freed");
            assert_eq!(block.held_at(0, [0.0, 0.0]), Some(spot(1.0, 2.0, 1.0)));
        });
        assert!(slot.reap());
        assert!(slot.empty() && slot.reap(), "nothing left to free");
        assert!(slot.publish(Block::new()).is_ok());
        slot.withdraw();
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
        let picker = std::fs::read_to_string(format!("{root}/src/runtime/features/record_place.rs")).unwrap();
        assert_eq!(picker.matches("super::scrub_hold::frame(slot, accessor);").count(), 1);
        let scrub = picker.split_once("unsafe fn scrub(fighter: usize) {").unwrap().1.split_once("\n}").unwrap().0;
        assert!(scrub.contains("!crate::engine::macro_ui::scrub_track::SLOT.live() { return; }"), "nothing is read with no editor");
    }
    }
}
