use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;

use crate::engine::macros::{positions::Source, InputFrame, ReplayFrame};
use crate::engine::playback::Book;
use super::scrub_track::TakeTrack;

const IDLE: u64 = u64::MAX;

#[derive(Clone, Copy)]
pub struct Id {
    bytes: [u8; crate::model::store::MAX_ID_CHARS],
    len: u8,
}

impl Id {
    pub fn new(id: &str) -> Option<Self> {
        let bytes = id.as_bytes();
        if bytes.len() > crate::model::store::MAX_ID_CHARS { return None; }
        let mut out = Self { bytes: [0; crate::model::store::MAX_ID_CHARS], len: bytes.len() as u8 };
        out.bytes[..bytes.len()].copy_from_slice(bytes);
        Some(out)
    }

    pub fn as_str(&self) -> &str { std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or("") }
}

pub struct Open {
    pub id: Id,
    pub slot: usize,
    pub generation: u32,
    pub version: (u64, u64),
    pub pass: bool,
    pub retired_inputs: Option<Vec<InputFrame>>,
}

pub struct PlayTrack {
    rows: TakeTrack,
    key: AtomicU64,
    limit: AtomicU32,
    count: AtomicU32,
    written: AtomicBool,
    made: AtomicBool,
    busy: AtomicU32,
    state: Mutex<State>,
}

struct State {
    open: Option<Open>,
    done: Vec<String>,
    done_version: (u64, u64),
}

pub static PLAY: PlayTrack = PlayTrack::new();

impl Default for PlayTrack {
    fn default() -> Self { Self::new() }
}

impl PlayTrack {
    pub const fn new() -> Self {
        Self {
            rows: TakeTrack::new(), key: AtomicU64::new(IDLE), limit: AtomicU32::new(0), count: AtomicU32::new(0),
            written: AtomicBool::new(false), made: AtomicBool::new(false), busy: AtomicU32::new(0),
            state: Mutex::new(State { open: None, done: Vec::new(), done_version: (0, 0) }),
        }
    }

    pub fn ready(&self) {
        if self.made.load(Ordering::SeqCst) {
            self.rows.service(self.sampling(), self.count.load(Ordering::Acquire) as usize);
        }
    }

    pub fn sampling(&self) -> bool { self.key.load(Ordering::SeqCst) != IDLE }
    pub fn allocated(&self) -> bool { self.made.load(Ordering::SeqCst) }
    pub fn live(&self, slot: usize, frame: &ReplayFrame) { self.rows.live(slot, frame); }
    pub fn seed(&self, slot: usize, frame: Option<ReplayFrame>) { self.rows.seed(slot, frame); }
    pub fn fed(&self, slot: usize, generation: u32, count: u32, seed: impl FnOnce()) {
        let key = u64::from(generation) << 32 | slot as u64;
        if self.key.load(Ordering::SeqCst) == key && count > 0 && count <= self.limit.load(Ordering::Acquire)
            && (count == 1 || self.count.load(Ordering::Acquire) != 0) {
            if count == 1 { seed(); }
            self.rows.push(count as usize - 1, slot);
            self.count.store(count, Ordering::Release);
        }
    }

    pub fn begin(&self, id: &str, slot: usize, generation: u32, version: (u64, u64), limit: u32, pass: bool) -> bool {
        let Some(name) = Id::new(id) else { return false };
        if slot > 1 { return false; }
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if state.done_version != version {
            state.done.clear();
            state.done_version = version;
        }
        if self.key.load(Ordering::SeqCst) != IDLE || state.open.is_some() || state.done.iter().any(|done| done == id) {
            return false;
        }
        if !self.made.load(Ordering::SeqCst) {
            self.rows.service(true, 0);
            self.made.store(true, Ordering::SeqCst);
        }
        self.limit.store(limit, Ordering::Release);
        self.count.store(0, Ordering::Release);
        state.open = Some(Open { id: name, slot, generation, version, pass, retired_inputs: None });
        self.key.store(u64::from(generation) << 32 | slot as u64, Ordering::SeqCst);
        true
    }

    pub fn end(&self) -> Option<(Open, Vec<[ReplayFrame; 2]>)> {
        self.key.store(IDLE, Ordering::SeqCst);
        let open = self.state.lock().unwrap_or_else(|error| error.into_inner()).open.take()?;
        let count = self.count.swap(0, Ordering::AcqRel);
        self.rows.take(count as usize).map(|rows| (open, rows))
    }

    pub fn open_run(&self) -> Option<(usize, u32)> {
        self.state.lock().unwrap_or_else(|error| error.into_inner()).open.as_ref().map(|open| (open.slot, open.generation))
    }

    /// Keep the exact source of an outstanding run when the worker replaces its playback book.
    pub fn retire_book(&self, book: &mut Book) {
        if !self.sampling() { return; }
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(open) = state.open.as_mut().filter(|open| open.retired_inputs.is_none()) {
            open.retired_inputs = book.take_inputs(open.id.as_str());
        }
    }

    pub fn note_done(&self, id: &str, version: (u64, u64)) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if state.done_version != version {
            state.done.clear();
            state.done_version = version;
        }
        if !state.done.iter().any(|done| done == id) { state.done.push(id.to_owned()); }
    }

    pub fn note_written(&self) { self.written.store(true, Ordering::Release); }
    pub fn take_written(&self) -> bool { self.written.swap(false, Ordering::AcqRel) }
    pub fn busy(&self) -> u32 { self.busy.load(Ordering::SeqCst) }
    pub fn poll<R>(&self, take: impl FnOnce() -> R) -> R {
        if self.key.load(Ordering::SeqCst) == IDLE { return take(); }
        self.busy.fetch_add(1, Ordering::SeqCst);
        let result = take();
        self.busy.fetch_sub(1, Ordering::SeqCst);
        result
    }

    pub fn write_wanted(existing: Option<(Source, u32)>, rows: usize, inputs: usize) -> (bool, bool) {
        match existing {
            Some((Source::Recorded, _)) => (false, true),
            Some((Source::Generated, count)) if count as usize >= rows => (false, count as usize >= inputs),
            _ => (rows > 0, rows >= inputs),
        }
    }

    #[cfg(test)]
    pub fn reset(&self) {
        self.key.store(IDLE, Ordering::SeqCst);
        self.limit.store(0, Ordering::Release);
        self.count.store(0, Ordering::Release);
        self.written.store(false, Ordering::Release);
        // Tests leak the old table: TakeTrack never frees it.
        self.made.store(false, Ordering::SeqCst);
        self.busy.store(0, Ordering::SeqCst);
        *self.state.lock().unwrap_or_else(|error| error.into_inner()) =
            State { open: None, done: Vec::new(), done_version: (0, 0) };
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::macros::{InputFrame, StartState};

    fn sample(x: f32) -> ReplayFrame {
        ReplayFrame { input: InputFrame::default(), start: StartState::neutral(7, 0, 1.0), x, y: 0.0 }
    }

    fn write_wanted_follows_the_rule() {
        assert_eq!(PlayTrack::write_wanted(Some((Source::Recorded, 0)), 5, 5), (false, true));
        assert_eq!(PlayTrack::write_wanted(Some((Source::Generated, 5)), 2, 5), (false, true));
        assert_eq!(PlayTrack::write_wanted(Some((Source::Generated, 3)), 2, 5), (false, false));
        assert_eq!(PlayTrack::write_wanted(Some((Source::Generated, 2)), 3, 5), (true, false));
        assert_eq!(PlayTrack::write_wanted(None, 5, 5), (true, true));
        assert_eq!(PlayTrack::write_wanted(None, 0, 5), (false, false));
        assert_eq!(PlayTrack::write_wanted(None, 0, 0), (false, true));
    }

    fn fed_keeps_only_its_runs_rows_in_order() {
        let track = PlayTrack::new();
        assert!(!track.allocated());
        assert!(track.begin("walk", 1, 23, (2, 3), 3, true));
        track.live(1, &sample(1.0));
        track.live(0, &sample(101.0));
        track.fed(0, 23, 1, || panic!("wrong role seeded"));
        track.fed(1, 22, 1, || panic!("old generation seeded"));
        track.fed(1, 23, 4, || panic!("past the limit"));
        track.fed(1, 23, 1, || {});
        track.live(1, &sample(3.0));
        track.fed(1, 23, 3, || panic!("not the first input"));
        let (open, rows) = track.end().unwrap();
        assert_eq!((open.id.as_str(), open.slot, open.generation), ("walk", 1, 23));
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0][0].x, rows[0][1].x), (1.0, 101.0));
        assert!(track.begin("walk", 1, 24, (2, 3), 3, true));
        track.fed(1, 24, 2, || panic!("missing first input"));
        assert!(track.end().is_none());
        track.note_done("walk", (2, 3));
        assert!(!track.begin("walk", 1, 25, (2, 3), 3, true));
        assert!(track.begin("walk", 1, 25, (2, 4), 3, true));
        assert!(track.end().is_none());
    }

    fn the_static_is_idle_at_rest() {
        let track = PlayTrack::new();
        assert!(!track.allocated() && !track.sampling() && track.open_run().is_none() && !track.take_written());
        assert_eq!(Id::new("walk").unwrap().as_str(), "walk");
        assert!(Id::new(&"a".repeat(crate::model::store::MAX_ID_CHARS + 1)).is_none());
        let source = include_str!("play_track.rs");
        assert!(source.contains("pub static PLAY: PlayTrack = PlayTrack::new();"));
        assert!(source.contains("const IDLE: u64 = u64::MAX;") && source.contains("key: AtomicU64::new(IDLE)"));
    }

    fn runtime_wires_sampling_seed_service_and_adoption() {
        let states = include_str!("../../runtime/features/states.rs");
        let host = include_str!("../../runtime/features/recording_host.rs");
        let editor = include_str!("../../runtime/macros/replay_capture.rs");
        assert!(states.contains("play_sampling()"));
        assert!(host.contains("take_cpu_frame_seeded(") && host.contains("on_player_poll_seeded("));
        assert_eq!(host.matches("seed_both(super::recorder::cpu_body(), rr_host::play_seed)").count(), 2);
        assert!(host.contains("TAKE.seed(slot, frame)"));
        assert!(host.contains("store(slot, frame);"));
        assert!(host.contains("play_track_service("));
        assert!(editor.contains("take_generated()") && editor.contains("adopt_positions("));
        assert!(include_str!("../rr_host.rs").contains("#[cfg(not(test))]\nfn track_version() -> (u64, u64) { recording_version() }"));
        let worker = include_str!("../rr_host.rs").split_once("pub fn play_track_service(").unwrap().1
            .split_once("\n}").unwrap().0;
        assert!(worker.find("PLAY_PLAYER.load").unwrap() < worker.find("PLAY.ready();").unwrap());
    }

    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("write_wanted_follows_the_rule", write_wanted_follows_the_rule as fn()),
            ("fed_keeps_only_its_runs_rows_in_order", fed_keeps_only_its_runs_rows_in_order as fn()),
            ("the_static_is_idle_at_rest", the_static_is_idle_at_rest as fn()),
            ("runtime_wires_sampling_seed_service_and_adoption", runtime_wires_sampling_seed_service_and_adoption as fn()),
        ]);
    }
    }
}
