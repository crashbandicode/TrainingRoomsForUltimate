//! INTERFACE STUB (package 4 provides the real module: the macro/replay runtime under `crate::runtime::macros`; the manifest key
//! `modules.root.replay_capture` is repointed when it lands). The fork's `replay_capture`, as far as the Mii replay source reads it.
//!
//! No provider exists in this tree yet, so the answers are the "no capture" ones: nothing is loading and the selection never changes.

/// A replay capture is loading (no capture exists without package 4's runtime).
pub fn loading() -> bool {
    false
}

/// The clip selection's revision counter (constant: no selection exists without package 4's runtime).
pub fn selection_revision() -> u64 {
    0
}