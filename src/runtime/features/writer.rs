//! The small-file writer (lead ruling 2026-11-02, review R5): an INI or settings write requested from a
//! hook, a menu tick or a chord only stores the request in a fixed slot; this thread, which owns its own
//! stack, performs the transactional write (`model::ini_files`) outside hook scope, so the hook path
//! stays allocation-free. A slot holds the newest request of its kind (later requests replace earlier
//! unwritten ones).
//!
//! Until package 5's menu calls `request_shortcuts`, the only caller is the diag build's Eden stand-in,
//! so the module is diag-only (no dead code in the shipped build).
//!
//! NOT YET SWITCH-RUN beyond the Eden stand-in.

use std::path::Path;
use std::time::Duration;

use parking_lot::Mutex;

use crate::model::ini_files;
use crate::model::menu_shortcuts::{self, MenuShortcuts};
use crate::model::storage::FixedPath;
use crate::runtime::diag_log::diagnostic;

/// The stack of the writer thread (a JSON-free path: file I/O and a small string).
const STACK_BYTES: usize = 64 * 1024;
/// How often the thread looks at its slots.
const POLL: Duration = Duration::from_millis(100);

static SHORTCUTS: Mutex<Option<MenuShortcuts>> = Mutex::new(None);

/// Start the writer (boot, outside any hook).
pub fn start() {
    let spawned = std::thread::Builder::new().name("tr-ini-writer".into()).stack_size(STACK_BYTES).spawn(run);
    if spawned.is_err() {
        diagnostic(format_args!("[TrainingRooms] ini writer thread did not start; requests stay unwritten"));
    }
}

/// Queue a `button_config.ini` rewrite with these two shortcuts (draft 9, N7). Allocation-free.
pub fn request_shortcuts(shortcuts: MenuShortcuts) {
    *SHORTCUTS.lock() = Some(shortcuts);
}

fn run() {
    loop {
        std::thread::sleep(POLL);
        let request = SHORTCUTS.lock().take();
        if let Some(shortcuts) = request {
            write_shortcuts(shortcuts);
        }
    }
}

fn write_shortcuts(shortcuts: MenuShortcuts) {
    let Some(path) = FixedPath::<128>::in_root(menu_shortcuts::FILE_NAME) else { return };
    match ini_files::write_shortcuts(Path::new(path.as_str()), shortcuts) {
        Ok(()) => diagnostic(format_args!("[TrainingRooms] button_config.ini written by the writer thread")),
        Err(error) => {
            // The change stays in memory for the session (SPEC-input-04 question 6 (c)); say so once.
            diagnostic(format_args!("[TrainingRooms] button_config.ini not written: {error}; kept for this session"));
            crate::runtime::features::notices::post("Menu shortcuts", "Could not save button_config.ini", 120, [0, 0, 0, 255]);
        }
    }
}
