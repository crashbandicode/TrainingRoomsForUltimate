//! The fork's `common::filesystem`: the writer thread for the diagnostic traces (lead ruling 2026-11-02). A game callback only copies
//! its newest trace into a slot (`publish_*`: no allocation, no file I/O, newer replaces older); this thread, which owns its stack,
//! serialises and writes the file through the transactional write (`engine::lobby::trace_slot`, `model::store::write_replace`).
//! A failed write is logged and the state kept for the next turn; the old file is untouched.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[cfg(any(feature = "mii-editor-alpha", feature = "automation-results-observer"))]
use crate::engine::lobby::trace_slot::{flush_with, LatestSlot};

const STACK_BYTES: usize = 64 * 1024;
const POLL: Duration = Duration::from_millis(100);

static STARTED: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "mii-editor-alpha")]
static MII_EDITOR: LatestSlot<crate::runtime::mii::mii_editor_bridge::Trace> = LatestSlot::new();

/// Start the writer thread (once).
pub fn init() {
    if STARTED.swap(true, Ordering::AcqRel) {
        return;
    }
    let started = std::thread::Builder::new().name("tr-trace-writer".into()).stack_size(STACK_BYTES).spawn(|| loop {
        turn();
        std::thread::sleep(POLL);
    });
    if started.is_err() {
        STARTED.store(false, Ordering::Release);
        crate::engine::compat::logging::diagnostic(format_args!("[TrainingRooms] trace writer thread did not start"));
    }
}

/// The results observer's finished trace (development builds only); the box is moved in, nothing is copied or allocated.
#[cfg(feature = "automation-results-observer")]
static RESULTS_OBSERVER: LatestSlot<Box<crate::runtime::lobby::automation_helper::observer::Trace>> = LatestSlot::new();

/// One pass over the slots.
fn turn() {
    #[cfg(feature = "automation-results-observer")]
    if let Err(error) = flush_with(&RESULTS_OBSERVER, |trace| {
        crate::runtime::lobby::automation_helper::observer::write_trace(trace);
        Ok(())
    }) {
        crate::engine::compat::logging::diagnostic(format_args!("[AutomationObserver] trace write failed: {error}"));
    }
    #[cfg(feature = "mii-editor-alpha")]
    if let Err(error) = flush_with(&MII_EDITOR, crate::runtime::mii::mii_editor_bridge::write_trace) {
        crate::engine::compat::logging::diagnostic(format_args!("[MiiEditor] trace write failed: {error}"));
    }
}

/// The results observer's finished trace.
#[cfg(feature = "automation-results-observer")]
pub fn write_results_observer(trace: Box<crate::runtime::lobby::automation_helper::observer::Trace>) {
    RESULTS_OBSERVER.put(trace);
}

/// The Mii editor bridge's newest trace (`mii-editor-last.json`).
#[cfg(feature = "mii-editor-alpha")]
pub fn publish_mii_editor(trace: &crate::runtime::mii::mii_editor_bridge::Trace) {
    MII_EDITOR.put(trace.clone());
}
