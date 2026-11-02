//! Shared match recording format and library for Training, Export and AI.
//!
//! * [`model`]: the data contract (see `docs/FORMAT.md`).
//! * [`capture`]: producer-side [`Recorder`] that validates as it records and
//!   writes a streaming manifest + JSONL capture, or keeps events in memory.
//! * [`archive`]: manifest/JSONL reading and writing with explicit
//!   [`Completeness`] instead of silently accepted truncation.
//! * [`clip`]: selected-frame and clip-start state resolution.
//! * [`validate`]: rule checks shared by the recorder and offline validation.
//!
//!
//! Consumer-specific conversion and behavior belong in the respective Training,
//! Export and AI projects, outside this shared library.

#![forbid(unsafe_code)]

pub mod archive;
pub mod capture;
pub mod clip;
pub mod model;
pub mod validate;

pub use archive::{ArchiveError, TimelineReader, WriteOptions};
pub use capture::{CaptureError, Finished, Recorder, RecorderOptions};
pub use clip::{
    clip_context, clip_context_for, clip_events, clip_events_for, selection_end, selection_end_for,
    ClipContext, Resolved, SelectAt,
};
pub use model::*;
pub use validate::{validate_capture, Conformance, Issue, IssueCode};
