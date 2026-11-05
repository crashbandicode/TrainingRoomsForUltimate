//! First Training HUD and save notices from the approved core/state contracts.

use core::fmt::Write;

use super::chords::ChordSet;
use super::notify::{Body, Queue, BODY_MAX, DEFAULT_COLOR};
use super::storage::FixedPath;

/// Every logical button, every separator, and both optional menu alternatives.
/// The permanent chord is 12 ASCII bytes. This bound also detects future
/// button-label growth at compile time.
const _: () = {
    let mut longest = 0;
    let mut i = 0;
    while i < super::menu::chord::NAMES.len() {
        longest += super::menu::chord::NAMES[i].len();
        i += 1;
    }
    longest += (super::menu::chord::BUTTONS - 1) * 3;
    assert!(12 + 2 * (4 + longest) <= BODY_MAX);
};

pub const SHORT_UNITS: usize = 48;
pub const LONG_COLUMNS: usize = 64;

/// The exact UTF-16 body consumed by the native notice writer. Long text uses
/// the separately preallocated multiline pane. Wrapping replaces separator
/// spaces, never button-label characters, and needs no extra buffer capacity.
pub fn notice_units(text: &str, out: &mut [u16; BODY_MAX]) -> usize {
    let mut len = 0;
    for unit in text.encode_utf16().take(out.len()) {
        out[len] = unit;
        len += 1;
    }
    if len > SHORT_UNITS {
        let mut start = 0;
        let mut space = None;
        for i in 0..len {
            if out[i] == u16::from(b' ') { space = Some(i); }
            if out[i] == u16::from(b'\n') { start = i + 1; space = None; continue; }
            if i - start >= LONG_COLUMNS {
                if let Some(at) = space.filter(|at| *at >= start) {
                    out[at] = u16::from(b'\n');
                    start = at + 1;
                    space = None;
                }
            }
        }
    }
    len
}

#[derive(Default)]
pub struct BootNotices {
    posted: bool,
}

/// A settings file that already existed at boot has had its first time, even with no `intro_seen` key.
/// A missing file has not. A stored `intro_seen: true` has, on its own.
pub fn intro_already_seen(file_existed: bool, settings: &super::settings::Settings) -> bool {
    settings.intro_seen() || file_existed
}

impl BootNotices {
    pub const fn new() -> Self {
        Self { posted: false }
    }

    /// One process. `posted` starts consumed when this install has already had its first time.
    pub fn for_install(file_existed: bool, settings: &super::settings::Settings) -> Self {
        Self { posted: intro_already_seen(file_existed, settings) }
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
        let mut text = FixedPath::<BODY_MAX>::new();
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
            let mut text = FixedPath::<BODY_MAX>::new();
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
