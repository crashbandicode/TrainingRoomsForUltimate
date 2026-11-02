//! Shared match recording format and library for Training, Export and AI.
//!
//! * [`model`]: the data contract (see `docs/FORMAT.md`).
//! * [`archive`]: manifest/JSONL reading and writing with explicit
//!   [`Completeness`] instead of silently accepted truncation.
//! * [`validate`]: rule checks shared by the writer and offline validation.
//!
//! This copy carries what the TrainingRooms plugin uses. The reference producer (`Recorder`), the clip
//! resolution and the whole-capture helpers are the test oracle crate `tests/oracle`.
//!
//! Consumer-specific conversion and behavior belong in the respective Training,
//! Export and AI projects, outside this shared library.

#![forbid(unsafe_code)]

pub mod archive;
pub mod model;
pub mod validate;

pub use archive::{ArchiveError, TimelineReader, WriteOptions};
pub use model::*;
pub use validate::{Conformance, Issue, IssueCode};
