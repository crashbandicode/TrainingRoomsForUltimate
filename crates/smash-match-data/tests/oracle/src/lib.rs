//! Test oracle of `smash-match-data` (test code only; a dev-dependency of the crate and of the plugin): the reference
//! producer `Recorder` (`capture.rs`), the clip resolution (`clip.rs`) and the whole-capture helpers (`whole.rs`). The
//! library keeps what the TrainingRooms plugin uses (slim stream, 2026-11-02, docs/SIZE-PLAN.md "smash-match-data");
//! a consumer that needs these takes them from here or from the upstream `smash-match-data-design` crate (`faac04e`).
//! The modules below mirror the library's API, so `capture.rs`, `clip.rs` and the tests use their old paths.

pub mod capture;
pub mod clip;
mod whole;

pub mod archive {
    pub use smash_match_data::archive::*;
}
pub mod model {
    pub use smash_match_data::model::*;
}
pub mod validate {
    pub use crate::whole::validate_capture;
    pub use smash_match_data::validate::*;
}

pub use capture::{CaptureError, Finished, Recorder, RecorderOptions};
pub use clip::{
    clip_context, clip_context_for, clip_events, clip_events_for, selection_end, selection_end_for, CaptureClip, ClipContext,
    Resolved, SelectAt,
};
pub use smash_match_data::*;
pub use whole::{validate_capture, CaptureExt};
