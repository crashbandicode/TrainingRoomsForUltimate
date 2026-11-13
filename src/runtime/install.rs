//! Writing the install line: one line to `events.log` and to the Skyline log, rendered on the boot thread into a fixed buffer of the
//! line's worst case (`install_capacity::INSTALL_LINE_CAPACITY`), taken from the boot heap outside every hook scope and freed after the
//! write, so nothing of it stays resident. A line that would not fit ends `result=line_overflow` (`install_line::render_into`).
//!
//! Part of the Switch build.

use std::fs::OpenOptions;
use std::io::Write as _;

use crate::install_capacity::INSTALL_LINE_CAPACITY;
use crate::model::install_line::{render_into, InstallLine};
use crate::model::storage;

pub fn write(line: &InstallLine<'_>) {
    // Boot thread only, outside every hook scope (`heap_allocations` counts hook scopes): one buffer of the worst case, never grown.
    let mut buffer = vec![0u8; INSTALL_LINE_CAPACITY];
    let (text, _overflow) = render_into(&mut buffer, line);
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
