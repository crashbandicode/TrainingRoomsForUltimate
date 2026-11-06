//! The small-file writer (lead ruling 2026-11-02, review R5): a settings or INI write requested from a hook, a
//! menu tick or a chord only stores the request in a fixed slot; this thread, which owns its own stack, performs
//! the transactional write (`model::store::write_replace`, the Store's one write, through `ini_files::write_atomic`: temporary file, sync, replace with a backup) outside
//! hook scope, so the hook path stays allocation-free and does no file I/O. A slot holds the newest request of its
//! kind (later requests replace earlier unwritten ones). The files:
//!
//! * `button_config.ini`: the two menu shortcuts (draft 9, N7), requested by `chords::commit_shortcut`.
//! * `replay_capture.ini`: the Replay capture toggle (Q7), `request_replay_capture` (the menu row) and
//!   `persist_replay_capture` (the Deluxe Plugins-page row, `runtime::deluxe_row`); this thread is the file's only writer.
//! * `settings.json`: the menu's settings on close, `request_settings` (the menu worker thread allocates the text;
//!   the writer only moves it to the file).
//!
//! A failed write keeps the change in memory for the session and says so with a notice (SPEC-input-04 question 6
//! (c)); the old file is untouched.
//!
//! NOT YET SWITCH-RUN beyond the Eden stand-in.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use hos_lock::Mutex;

use crate::model::capture_settings::{self, CaptureSettings};
use crate::model::ini_files::{self, FileRead};
use crate::model::menu_shortcuts::{self, MenuShortcuts};
use crate::model::storage::{self, FixedPath};
use crate::runtime::diag_log::diagnostic;
use crate::model::notify::DEFAULT_COLOR;
use crate::runtime::features::notices;

/// How often the thread looks at its slots.
const POLL: Duration = Duration::from_millis(100);

static SHORTCUTS: Mutex<Option<MenuShortcuts>> = Mutex::new(None);
/// A `replay_capture.ini` write is due. The value is read when the write happens ([`replay_capture`]), so the file ends at
/// the newest preference whichever row (the menu, or the Deluxe Plugins page) changed it last.
static REPLAY_CAPTURE_DUE: AtomicBool = AtomicBool::new(false);
static SETTINGS: Mutex<Option<String>> = Mutex::new(None);

/// The Replay capture toggle as the file holds it (boot) or as the menu last set it; the preference itself once the capture
/// adapter is ready (see [`replay_capture`]).
static CAPTURE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Start the writer (boot, outside any hook).
pub fn start() {
    let spawned = crate::runtime::threads::spawn(crate::model::thread_stacks::ThreadId::IniWriter, run);
    if spawned.is_err() {
        diagnostic(format_args!("[TrainingRooms] ini writer thread did not start; requests stay unwritten"));
    }
}

/// Queue a `button_config.ini` rewrite with these two shortcuts (draft 9, N7). Allocation-free.
pub fn request_shortcuts(shortcuts: MenuShortcuts) {
    *SHORTCUTS.lock() = Some(shortcuts);
}

/// Queue a `replay_capture.ini` rewrite (Q7) and request the same live preference.
/// The adapter preserves the current record's latch; this governs the next record.
pub fn request_replay_capture(enabled: bool) {
    let result = crate::runtime::macros::replay_store::toggle().request_choice(usize::from(enabled));
    diagnostic(format_args!("[ReplayCapture] menu preference enabled={enabled} result={result:?}"));
    CAPTURE_ENABLED.store(enabled, Ordering::Release);
    persist_replay_capture();
}

/// Queue a `replay_capture.ini` rewrite of the current preference. Lock-free and allocation-free: the Deluxe row calls it from
/// the host's draw thread after it changed the preference.
pub fn persist_replay_capture() {
    REPLAY_CAPTURE_DUE.store(true, Ordering::Release);
}

/// Queue a `settings.json` rewrite with this text (the menu's close). The text is moved, not copied.
pub fn request_settings(json: String) {
    *SETTINGS.lock() = Some(json);
}

/// The Replay capture preference now (the menu shows it, the INI is written from it): the capture layer's own preference
/// once its adapter is ready (it may have been changed on the Deluxe page), else the value the file or the menu gave.
pub fn replay_capture() -> bool {
    let toggle = crate::runtime::macros::replay_store::toggle();
    if toggle.is_ready() {
        toggle.desired()
    } else {
        CAPTURE_ENABLED.load(Ordering::Acquire)
    }
}

/// Boot: read `replay_capture.ini` (fail-closed: anything but a clean `[capture] enabled=on` reads as off).
/// A missing file is not created (the first toggle writes it).
pub fn load_replay_capture() {
    let Some(path) = FixedPath::<128>::in_root(capture_settings::FILE_NAME) else { return };
    let (settings, why) = match ini_files::read_capped(Path::new(path.as_str()), capture_settings::MAX_BYTES) {
        FileRead::Bytes(bytes) => CaptureSettings::from_bytes(Some(&bytes)),
        _ => CaptureSettings::from_bytes(None),
    };
    if let Some(why) = why {
        diagnostic(format_args!("[TrainingRooms] replay_capture.ini read as off: {why}"));
    }
    CAPTURE_ENABLED.store(settings.enabled, Ordering::Release);
}

fn run() {
    loop {
        std::thread::sleep(POLL);
        #[cfg(feature = "diag")]
        diag_requests();
        let shortcuts = SHORTCUTS.lock().take();
        if let Some(shortcuts) = shortcuts {
            write_shortcuts(shortcuts);
        }
        if REPLAY_CAPTURE_DUE.swap(false, Ordering::AcqRel) {
            write_replay_capture(replay_capture());
        }
        let settings = SETTINGS.lock().take();
        if let Some(json) = settings {
            write_settings(&json);
        }
    }
}

/// Diag build: the Eden tests ask for what the menu's close asks, without driving the menu, by creating request
/// files in the data folder: `diag-shortcut.req` holds `<slot> <chord mask>` (decimal) and goes through
/// `chords::commit_shortcut`; `diag-capture.req` holds `on` or `off` and goes through `request_replay_capture`.
/// Not in the shipped NRO.
#[cfg(feature = "diag")]
fn diag_requests() {
    let read = |name: &str| {
        let path = FixedPath::<128>::in_root(name)?;
        let text = std::fs::read_to_string(path.as_str()).ok()?;
        let _ = std::fs::remove_file(path.as_str());
        Some(text)
    };
    if let Some(text) = read("diag-shortcut.req") {
        let mut words = text.split_whitespace().filter_map(|word| word.parse::<u16>().ok());
        if let (Some(slot), Some(mask)) = (words.next(), words.next()) {
            let result = crate::runtime::features::chords::commit_shortcut(usize::from(slot), crate::model::menu::chord::Chord(mask));
            diagnostic(format_args!("[TrainingRooms] diag_request shortcut slot={slot} mask={mask} result={result:?}"));
        }
    }
    if let Some(text) = read("diag-capture.req") {
        let enabled = text.trim() == "on";
        request_replay_capture(enabled);
        diagnostic(format_args!("[TrainingRooms] diag_request replay_capture enabled={enabled}"));
    }
}

fn write_shortcuts(shortcuts: MenuShortcuts) {
    let Some(path) = FixedPath::<128>::in_root(menu_shortcuts::FILE_NAME) else { return };
    match ini_files::write_shortcuts(Path::new(path.as_str()), shortcuts) {
        Ok(()) => diagnostic(format_args!("[TrainingRooms] button_config.ini written by the writer thread")),
        Err(error) => {
            // The change stays in memory for the session (SPEC-input-04 question 6 (c)); say so once.
            diagnostic(format_args!("[TrainingRooms] button_config.ini not written: {error}; kept for this session"));
            notices::post("Menu shortcuts", "Could not save button_config.ini", 120, DEFAULT_COLOR);
        }
    }
}

fn write_replay_capture(enabled: bool) {
    let Some(path) = FixedPath::<128>::in_root(capture_settings::FILE_NAME) else { return };
    let path = Path::new(path.as_str());
    // The file's own text is kept (comments, other lines); an unreadable or oversized file is replaced by a
    // document with the toggle alone, as a missing one.
    let original = match ini_files::read_capped(path, capture_settings::MAX_BYTES) {
        FileRead::Bytes(bytes) => String::from_utf8(bytes).unwrap_or_default(),
        _ => String::new(),
    };
    match ini_files::write_atomic(path, CaptureSettings::to_ini_text(&original, enabled).as_bytes()) {
        Ok(()) => diagnostic(format_args!("[TrainingRooms] replay_capture.ini written by the writer thread enabled={enabled}")),
        Err(error) => {
            diagnostic(format_args!("[TrainingRooms] replay_capture.ini not written: {error}; kept for this session"));
            notices::post("Replay capture", "Could not save replay_capture.ini", 120, DEFAULT_COLOR);
        }
    }
}

fn write_settings(json: &str) {
    let Some(path) = FixedPath::<128>::in_root(storage::file::SETTINGS) else { return };
    match ini_files::write_atomic(Path::new(path.as_str()), json.as_bytes()) {
        Ok(()) => diagnostic(format_args!("[TrainingRooms] settings.json written by the writer thread")),
        Err(error) => {
            diagnostic(format_args!("[TrainingRooms] settings.json not written: {error}; kept for this session"));
            notices::post("Settings", "Could not save settings.json", 120, DEFAULT_COLOR);
        }
    }
}
