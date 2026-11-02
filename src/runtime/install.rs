//! Writing the install line: one line to `events.log` and to the Skyline log, rendered on the boot thread
//! (`install_line::render`: the line has no fixed size).
//!
//! NOT YET SWITCH-COMPILED.

use std::fs::OpenOptions;
use std::io::Write as _;

use crate::model::install_line::InstallLine;
use crate::model::storage;

pub fn write(line: &InstallLine<'_>) {
    // Boot thread only: the line has no fixed size (`install_line::render`).
    let buffer = crate::model::install_line::render(line);
    let text = buffer.as_str();
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
