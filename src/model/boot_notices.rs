//! First Training HUD and save notices from the approved core/state contracts.

use core::fmt::Write;

use super::chords::ChordSet;
use super::notify::{Body, Queue, DEFAULT_COLOR};
use super::storage::FixedPath;

#[derive(Default)]
pub struct BootNotices {
    posted: bool,
}

impl BootNotices {
    pub const fn new() -> Self {
        Self { posted: false }
    }

    /// Only a real native Training HUD with published settings consumes the boot edge.
    /// Session/layout resets clear the queue, not this once-per-process state.
    pub fn post_once(&mut self, native_hud: bool, chords: Option<ChordSet>, queue: &mut Queue) -> bool {
        if self.posted || !native_hud {
            return false;
        }
        let Some(chords) = chords else { return false };
        self.posted = true;
        queue.post_static("TrainingRooms", "Welcome!", 60, DEFAULT_COLOR);
        let [first, second] = chords.shortcuts.slots();
        let mut text = FixedPath::<256>::new();
        // The permanent B + D-pad Up chord is never disabled by the INI.
        let _ = write!(text, "{}", super::menu_shortcuts::DEFAULT_OPEN_MENU_CONFIG);
        for chord in [first, second].into_iter().filter(|chord| !chord.is_empty()) {
            let _ = write!(text, " or {chord}");
        }
        queue.post("Open Menu", Body::copied(text.as_str()), 120, DEFAULT_COLOR);
        for (header, chord) in [
            ("Save State", chords.save_state),
            ("Load State", chords.load_state),
            ("Input Record", chords.input_record),
            ("Input Playback", chords.input_playback),
        ] {
            let mut text = FixedPath::<256>::new();
            let _ = write!(text, "{chord}");
            queue.post(header, Body::copied(text.as_str()), 120, DEFAULT_COLOR);
        }
        true
    }
}

/// Replace a previous save notice; the slot is zero-based internally.
pub fn saved(queue: &mut Queue, slot: usize) {
    const BODIES: [&str; 5] = ["Saved Slot 1", "Saved Slot 2", "Saved Slot 3", "Saved Slot 4", "Saved Slot 5"];
    let Some(body) = BODIES.get(slot) else { return };
    queue.clear_header("Save State");
    queue.post_static("Save State", body, 120, DEFAULT_COLOR);
}
