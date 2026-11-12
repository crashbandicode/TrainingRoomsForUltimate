//! Portable core of the named CPU macros: the recording model, a directory
//! backed library and replay-window selection. It depends on std, serde and
//! the shared match-data module (no Skyline), so it is fully host-testable. The runtime
//! owns recording, playback, character loading and applying a placement.
//!
//! A macro file is compact JSON of [`MacroRecording`] (schema version 1). The
//! library is one instantiation of the generic [`Store`]; any other value type
//! implementing [`Record`] gets the same atomic, id-checked, size-bounded files.
//!
//! ```
//! use training_mod_macros::*;
//!
//! # fn main() -> Result<(), MacroError> {
//! let dir = std::env::temp_dir().join(format!("tmm-doc-{}", std::process::id()));
//! let library = Library::new(&dir);
//!
//! // Recording finished: the runtime supplies the frames and the fighter state.
//! let frames = vec![InputFrame { buttons: 1, lstick_x: 127, ..InputFrame::default() }];
//! let mut rec = MacroRecording::new("Fox up-B", "13.0.4", StartState::neutral(8, 0, 1.0), frames)?;
//! let id = library.save_new(&rec)?;
//!
//! // Edit Location later: only the placement changes, then overwrite explicitly.
//! rec.set_placement(Placement { delta_y: 40.0, ..Placement::default() })?;
//! library.overwrite(&id, &rec)?;
//! assert_eq!(library.list()?.entries[0].id, id);
//! # std::fs::remove_dir_all(&dir).ok();
//! # Ok(())
//! # }
//! ```


#[cfg(any(feature = "diag", test))]
pub mod completion_trace;
pub mod handoff;
mod capture_archive;
mod capture_toggle;
mod library;
mod live_capture;
mod live_frame;
mod move_start;
mod recording;
mod replay;
mod shared_capture;
mod window;

pub use capture_archive::{CaptureArchive, CaptureSaveOutcome, SavedCapture};
#[cfg(test)]
pub use capture_archive::{CaptureEntry, CaptureListing};
pub use crate::model::capture_settings::{recover_previous_ini, CaptureSettings, IniRecovery, DEFAULT_CAPTURE_INI};
#[cfg(test)]
pub use crate::model::capture_settings::{host_persistence_key, CAPTURE_CONFIG_KEY};
pub use capture_toggle::{CaptureToggle, RecordOwner, RecordOwnership, ToggleRequest, CAPTURE_ON};
#[cfg(test)]
pub use capture_toggle::{RecordEnded, RecordPhase, ToggleSnapshot, CAPTURE_OFF};
pub use crate::model::store::SkippedFile;
#[cfg(test)]
pub use crate::model::store::{Listing, Record, Store, MAX_FILE_BYTES, MAX_ID_CHARS};
pub use library::Library;
#[cfg(test)]
pub use library::MacroEntry;
pub use live_capture::{LiveCapturePool, NativeRecordKey, MAX_RETAINED_CAPTURES};
#[cfg(any(feature = "diag", test))]
pub use live_capture::{FirstSeal, SealReason};
pub use live_frame::{CaptureBreak, LiveFrame, LiveInput, LiveSession, LiveTrack, MAX_BREAKS};
pub use recording::{InputFrame, MacroError, MacroRecording, MomentumMode, Placement, StartState, MAX_FRAMES, MAX_GAME_VERSION_BYTES, MAX_NAME_CHARS};
#[cfg(test)]
pub use recording::{DEFAULT_FPS, SCHEMA_VERSION};
pub use window::{FrameSnapshot, FrameWindow};
#[cfg(test)]
pub use window::{timestamp, Step};
pub use replay::{ReplayFrame, ReplaySession, ReplayTrack};
pub use move_start::{MoveFamily, MoveStart, MoveStartRules};
#[cfg(test)]
pub use move_start::MAX_MOVE_SETUP;
pub use shared_capture::SharedCaptureArchive;

#[cfg(test)]
mod test_util;

