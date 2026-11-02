//! Writing the install line: one line to `events.log` and to the Skyline log, formatted in a stack
//! buffer.
//!
//! NOT YET SWITCH-COMPILED.

use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::Write as _;

use crate::model::install_line::InstallLine;
use crate::model::storage;

pub fn write(line: &InstallLine<'_>) {
    // Formatted on the boot thread, which may allocate (never a hook): a heap string, so the line is never cut. An 8 KiB stack
    // buffer cut the line once the site rows passed it (2026-11-02, Eden hb-01: the feature fields and `result=` were lost after
    // the hitbox rows), and the feature fields are what the Eden gates read.
    let mut text = String::with_capacity(16 * 1024);
    let _ = write!(text, "{line}");
    let text = text.as_str();
    skyline::logging::log(text);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(storage::EVENTS_LOG_PATH) {
        let _ = writeln!(file, "{text}");
        let _ = file.sync_data();
    }
}

/// One more line in `events.log` and the Skyline log, for what installs after boot (the hooks in the
/// `common` script module install when that module loads).
pub fn write_text(text: &str) {
    skyline::logging::log(text);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(storage::EVENTS_LOG_PATH) {
        let _ = writeln!(file, "{text}");
        let _ = file.sync_data();
    }
}
