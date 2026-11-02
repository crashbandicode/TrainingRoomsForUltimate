//! What the CPU behaviours need to know about input playback, and the rule of decisions B5 / N4.
//!
//! By default a recording that plays on the CPU wins over the DI, SDI, air-dodge, tech and throw
//! settings (they apply again when the playback ends); the advanced row *Settings override playback*
//! (default Off) restores the rule that those settings override the recording.

/// The input-recording interface the CPU specs read (SPEC-cpu-01 §3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlaybackState {
    /// A playback is running on the CPU.
    pub running: bool,
    /// A recording is being made.
    pub recording: bool,
    /// The recorder is in standby / lockout.
    pub standby: bool,
    /// A playback is queued to start.
    pub queued: bool,
}

impl PlaybackState {
    /// A playback has possession of the CPU's inputs now.
    pub const fn possesses_cpu(&self) -> bool {
        self.running && !self.recording
    }
}

/// Whether a setting that B5 / N4 covers (DI, SDI, air-dodge direction, tech and mistech, throws)
/// may act on the CPU now: always, except while a playback possesses it and the override row is Off.
pub const fn covered_setting_applies(playback: PlaybackState, override_playback: bool) -> bool {
    !playback.possesses_cpu() || override_playback
}
