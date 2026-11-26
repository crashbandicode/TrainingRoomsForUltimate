use crate::engine::macros::{positions::Source, ReplayFrame};

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
}

pub struct PlayTrack;
pub static PLAY: PlayTrack = PlayTrack::new();

impl Default for PlayTrack {
    fn default() -> Self { Self::new() }
}

impl PlayTrack {
    pub const fn new() -> Self { Self }
    pub fn ready(&self) {}
    pub fn sampling(&self) -> bool { false }
    pub fn live(&self, _slot: usize, _frame: &ReplayFrame) {}
    pub fn seed_due(&self, _slot: usize) -> bool { false }
    pub fn fed(&self, _slot: usize, _generation: u32, _count: u32) {}
    pub fn begin(&self, _id: &str, _slot: usize, _generation: u32, _version: (u64, u64), _limit: u32, _pass: bool) -> bool { false }
    pub fn end(&self) -> Option<(Open, Vec<[ReplayFrame; 2]>)> { None }
    pub fn open_run(&self) -> Option<(usize, u32)> { None }
    pub fn note_done(&self, _id: &str, _version: (u64, u64)) {}
    pub fn note_written(&self) {}
    pub fn take_written(&self) -> bool { false }
    pub fn busy(&self) -> u32 { 0 }
    pub fn poll<R>(&self, take: impl FnOnce() -> R) -> R { take() }
    pub fn write_wanted(_existing: Option<(Source, u32)>, _rows: usize, _inputs: usize) -> (bool, bool) { (false, false) }
    #[cfg(test)]
    pub fn reset(&self) {}
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
        track.ready();
        assert!(track.begin("walk", 1, 23, (2, 3), 3, true));
        track.live(1, &sample(1.0));
        track.live(0, &sample(101.0));
        track.fed(0, 23, 1);
        track.fed(1, 22, 1);
        track.fed(1, 23, 4);
        track.fed(1, 23, 1);
        track.live(1, &sample(3.0));
        track.fed(1, 23, 3);
        let (open, rows) = track.end().unwrap();
        assert_eq!((open.id.as_str(), open.slot, open.generation), ("walk", 1, 23));
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0][0].x, rows[0][1].x), (1.0, 101.0));
    }

    fn the_static_is_idle_at_rest() {
        let track = PlayTrack::new();
        assert!(!track.sampling() && track.open_run().is_none() && !track.take_written());
        assert_eq!(Id::new("walk").unwrap().as_str(), "walk");
        assert!(Id::new(&"a".repeat(crate::model::store::MAX_ID_CHARS + 1)).is_none());
        let source = include_str!("play_track.rs");
        assert!(source.contains("pub static PLAY: PlayTrack = PlayTrack::new();"));
    }

    fn runtime_wires_sampling_seed_service_and_adoption() {
        let states = include_str!("../../runtime/features/states.rs");
        let host = include_str!("../../runtime/features/recording_host.rs");
        let editor = include_str!("../../runtime/macros/replay_capture.rs");
        assert!(states.contains("play_sampling()"));
        assert_eq!(host.matches("play_seed_due(").count(), 2);
        assert!(host.contains("play_track_service("));
        assert!(editor.contains("take_generated()") && editor.contains("adopt_positions("));
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
