//! The runtime's diagnostics channel (`src/runtime/diag_log.rs`) is plain `std` code on top of the
//! model, so it is compiled and exercised on the host by path. `init_diagnostics` and the drain
//! thread touch the card path and are not run here; the Eden rows `core01-E8` cover them.

#[path = "../src/model/mod.rs"]
#[allow(dead_code)]
mod model;

#[path = "../src/runtime/diag_log.rs"]
#[allow(dead_code)]
mod diag_log;

#[test]
fn shipped_lines_enter_the_ring_and_others_do_not() {
    // `core01_diag_ring_admission` at the channel level: a shipped tag is committed, a probe tag is not.
    let before = diag_log::ring_stats().0.committed;
    diag_log::diagnostic(format_args!("[TrainingRooms] boot test"));
    // No file is open on the host, so the drain leaves the line in the file-less state; it was
    // still counted as committed by the ring.
    let after = diag_log::ring_stats().0.committed;
    assert_eq!(after, before + 1);
    diag_log::diagnostic(format_args!("[Hud] probe line"));
    assert_eq!(diag_log::ring_stats().0.committed, after, "a probe tag is not written by a shipped build");
    assert_eq!(diag_log::ring_stats().0.dropped, 0);
}
