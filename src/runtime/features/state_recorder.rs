//! The save states' contact surface with package 4's input recorder (SPEC-input-01), behind one small
//! adapter so this package compiles without it. When pkg4's `recorder` module is merged, the bodies
//! below call it (`recorder::stop()`, `recorder::on_state_restored(saved_on_ledge)`, the playback and
//! record starts); nothing else changes. No behaviour is implemented here.

/// A load request was accepted: stop any input-recording playback (SPEC-state-01 §5.1 step 3).
pub fn stop() {
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!("[TrainingRooms] state recorder_stop"));
}

/// Both records of a load are done (once per restore); `saved_on_ledge` is whether the CPU was saved on
/// the ledge.
pub fn on_state_restored(saved_on_ledge: bool) {
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!("[TrainingRooms] state recorder_restored ledge={saved_on_ledge}"));
    #[cfg(not(feature = "diag"))]
    let _ = saved_on_ledge;
}

/// Recording Trigger "Save State Load": start an input recording.
pub fn start_record() {
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!("[TrainingRooms] state recorder_record"));
}

/// Save State Playback: play this slot.
pub fn start_playback(slot: u8) {
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!("[TrainingRooms] state recorder_playback slot={slot}"));
    #[cfg(not(feature = "diag"))]
    let _ = slot;
}
