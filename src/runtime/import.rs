//! The first-boot import at boot (SPEC-core-08): the blocking part on a thread with an explicit stack,
//! joined before the settings load (so the imported settings and saves are what Training opens with), then
//! the captures on their own thread that nothing waits for. The rules and the receipt are
//! `model::migration_run`; this file only schedules it and words the install line.
//!
//! NOT YET SWITCH-RUN.

use std::fmt::Write as _;
use std::path::Path;

use crate::model::migration_run::{run_background, run_blocking, status_word, Blocking, Roots, Stage};
use crate::model::storage::{self, FixedPath};
use crate::runtime::clock;
use crate::runtime::diag_log::diagnostic;

// The import threads' stack sizes are rows of `model::thread_stacks` (the 64 KiB copy buffer is on the heap).

fn roots() -> Roots<'static> {
    Roots { source: Path::new(storage::FORK_ROOT), dest: Path::new(storage::ROOT) }
}

/// Run the blocking part now. Returns the install line's `migration=` words (`done captures=running skipped_staging=3`,
/// `skipped`, `none`, …) and the background part, which `start_background` runs once boot is done (stream/stack: on the
/// boot loaders' static stack, which is free by then).
pub fn start() -> (FixedPath<96>, Option<Blocking>) {
    let blocking = crate::runtime::threads::run_joined(crate::model::thread_stacks::ThreadId::Import, || {
        run_blocking(roots(), clock::uptime_ms(), env!("CARGO_PKG_VERSION"), &mut |line| diagnostic(format_args!("{line}")))
    });
    let stage = match blocking {
        Ok(stage) => stage,
        _ => {
            diagnostic(format_args!("[Migration] the import thread did not run"));
            return (words("skipped", None, 0), None);
        }
    };
    let word = status_word(&stage);
    match stage {
        Stage::Imported { blocking, marker } => {
            let staging = blocking.receipt.skipped_staging;
            if let Some(Err(error)) = &marker {
                diagnostic(format_args!("[Migration] marker not written ({error}); the import runs again next boot"));
            }
            // A present path that could not be listed withholds the marker (R2-8): the import is incomplete and
            // the next boot runs it again.
            let withheld = blocking.marker_withheld();
            if blocking.pending_captures() > 0 {
                (words(word, Some("running"), staging), Some(blocking))
            } else if withheld {
                (words(word, Some("incomplete"), staging), None)
            } else {
                (words(word, Some("done"), staging), None)
            }
        }
        Stage::NoSource { marker } => {
            if let Err(error) = marker {
                diagnostic(format_args!("[Migration] marker not written ({error})"));
            }
            (words(word, None, 0), None)
        }
        Stage::MarkerPresent => (words(word, None, 0), None),
    }
}

/// The captures (boot's last step, after the install line): on the boot loaders' stack, which nothing uses after them.
pub fn start_background(blocking: Blocking) {
    let spawned = crate::runtime::threads::spawn(crate::model::thread_stacks::ThreadId::ImportCaptures, move || {
        let (receipt, marker) = run_background(roots(), blocking, &mut |line| diagnostic(format_args!("{line}")));
        match marker {
            Ok(()) => diagnostic(format_args!(
                "[Migration] captures=done result={} copied={} skipped_staging={}",
                receipt.result().word(),
                receipt.files.len(),
                receipt.skipped_staging
            )),
            Err(error) => diagnostic(format_args!("[Migration] captures=done but the marker was not written: {error}")),
        }
    });
    if spawned.is_err() {
        diagnostic(format_args!("[Migration] the capture thread did not start; the next boot copies them"));
    }
}

fn words(result: &str, captures: Option<&str>, staging: usize) -> FixedPath<96> {
    let mut out = FixedPath::<96>::new();
    let _ = write!(out, "{result}");
    if let Some(captures) = captures {
        let _ = write!(out, " captures={captures} skipped_staging={staging}");
    }
    out
}