//! Writing the install line: one line to `events.log` and to the Skyline log, formatted in a stack
//! buffer.
//!
//! NOT YET SWITCH-COMPILED.

use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::Write as _;

use crate::model::install_line::InstallLine;
use crate::model::storage::{self, FixedPath};

pub fn write(line: &InstallLine<'_>) {
    let mut buffer = FixedPath::<2048>::new();
    // A line that does not fit is cut, not dropped.
    let _ = write!(buffer, "{line}");
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
