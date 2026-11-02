//! Diag-only probe (package 6): does Skyline's std `File::sync_all` succeed on the console?
//!
//! The carried `cycle_preload.ini` writer now fails a store when `sync_all` fails (the contract the lead ruled on 2026-11-02, a
//! recorded clean-room change). If the pinned Skyline std reports an error for `sync_all` on every file, every store would fail, so
//! the first Eden boot answers it before the writer is wired: the same sequence as `write_new` (create + truncate + `write_all` +
//! `sync_all`) on a scratch file, the raw `io::Error` printed as is, then `sync_data`, a rename and a removal. The shipped build
//! has no such code (compliance: no diag code in the release NRO).

use std::fs::OpenOptions;
use std::io::Write;

use crate::model::storage;
use crate::runtime::diag_log::diagnostic;

pub fn run() {
    let scratch = format!("{}/sync-probe.tmp", storage::ROOT);
    let renamed = format!("{}/sync-probe.out", storage::ROOT);
    let mut file = match OpenOptions::new().write(true).create(true).truncate(true).open(&scratch) {
        Ok(file) => file,
        Err(error) => {
            diagnostic(format_args!("[SyncProbe] open_err {error:?}"));
            return;
        }
    };
    let write = file.write_all(b"[cycle_preload]\nslots=0\n");
    diagnostic(format_args!("[SyncProbe] write_all {write:?}"));
    let sync_all = file.sync_all();
    diagnostic(format_args!("[SyncProbe] sync_all {sync_all:?}"));
    let sync_data = file.sync_data();
    diagnostic(format_args!("[SyncProbe] sync_data {sync_data:?}"));
    drop(file);
    let rename = std::fs::rename(&scratch, &renamed);
    diagnostic(format_args!("[SyncProbe] rename {rename:?}"));
    let remove = std::fs::remove_file(&renamed);
    diagnostic(format_args!("[SyncProbe] remove {remove:?}"));
    diagnostic(format_args!("[SyncProbe] done sync_all_ok={}", sync_all.is_ok()));
}