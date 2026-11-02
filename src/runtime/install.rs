//! Writing the install line: one line to `events.log` and to the Skyline log, formatted on the boot thread's heap (never cut).
//!
//! NOT YET SWITCH-COMPILED.

use std::fs::OpenOptions;
use std::io::Write as _;

use crate::model::install_line::InstallLine;
use crate::model::storage;

pub fn write(line: &InstallLine<'_>) {
    // Formatted on the heap: the boot thread is not a hook (it allocates freely), and the line has outgrown any fixed buffer (s4-07: the
    // combined tree's line passed 8192 bytes and was cut before `result=`). Nothing is cut.
    let text = line.to_string();
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
