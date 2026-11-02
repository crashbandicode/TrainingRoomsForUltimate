//! Whole-capture helpers the plugin does not use: `Capture::load`, `Capture::save` and `validate_capture`, moved here
//! from the library's `archive.rs` and `validate.rs` (slim stream, 2026-11-02). `CaptureExt` keeps the method syntax.

use std::path::Path;

use smash_match_data::archive::{open_timeline, read_manifest, ArchiveError, ArchiveWriter, WriteOptions};
use smash_match_data::model::*;
use smash_match_data::validate::*;

/// The whole-capture methods of `Capture` (inherent methods in the library before the move).
pub trait CaptureExt: Sized {
    fn load(dir: impl AsRef<Path>) -> Result<Self, ArchiveError>;
    fn save(&self, dir: impl AsRef<Path>, options: &WriteOptions) -> Result<Manifest, ArchiveError>;
    fn validate(&self) -> Vec<Issue>;
}

impl CaptureExt for Capture {
    /// Load a capture directory fully into memory.
    fn load(dir: impl AsRef<Path>) -> Result<Capture, ArchiveError> {
        let dir = dir.as_ref();
        let manifest = read_manifest(dir)?;
        let mut reader = open_timeline(dir, &manifest)?;
        let mut events = Vec::new();
        while let Some(event) = reader.next_event()? {
            events.push(event);
        }
        let completeness = reader
            .completeness()
            .cloned()
            .unwrap_or(Completeness::Incomplete {
                problems: vec![TruncationProblem::MissingEnd],
            });
        Ok(Capture {
            manifest,
            events,
            completeness,
        })
    }

    /// Write this capture to `dir`. Any existing `end` event is replaced by a
    /// fresh one so the output is self-consistent. An `incomplete` source is
    /// saved as `truncated` with the original problems in the reason.
    fn save(
        &self,
        dir: impl AsRef<Path>,
        options: &WriteOptions,
    ) -> Result<Manifest, ArchiveError> {
        let mut manifest = self.manifest.clone();
        let mut writer = ArchiveWriter::create(dir, manifest.clone(), options)?;
        let mut last_clocks = Clocks::NONE;
        let mut count = 0u64;
        for event in &self.events {
            if matches!(event.body, EventBody::End { .. }) {
                continue;
            }
            writer.write_event(event)?;
            last_clocks = event.clocks;
            count += 1;
        }
        let end_seq = writer.last_seq().map_or(0, |seq| seq + 1);
        writer.write_event(&Event {
            seq: end_seq,
            clocks: last_clocks,
            body: EventBody::End { events: count },
        })?;
        manifest.completion = match &self.completeness {
            Completeness::Complete => Completion::complete(),
            Completeness::DeclaredTruncated { reason } => Completion {
                state: CompletionState::Truncated,
                reason: reason.clone(),
                events: None,
                last_seq: None,
            },
            other @ Completeness::Incomplete { .. } => {
                Completion::truncated(format!("re-saved {other}"))
            }
        };
        writer.finish(manifest)
    }

    fn validate(&self) -> Vec<Issue> {
        validate_capture(self)
    }
}

pub fn validate_capture(capture: &Capture) -> Vec<Issue> {
    let manifest = &capture.manifest;
    let mut issues = Vec::new();
    if manifest.format != FORMAT_NAME {
        issues.push(Issue::error(
            IssueCode::Format,
            None,
            format!("format is {:?}, expected {FORMAT_NAME:?}", manifest.format),
        ));
    }
    if !FORMAT_VERSION.can_read(manifest.format_version) {
        issues.push(Issue::error(
            IssueCode::Format,
            None,
            format!(
                "format version {} is not readable by {FORMAT_VERSION}",
                manifest.format_version
            ),
        ));
    }
    issues.extend(validate_header(&manifest.capture));

    let mut checker = StreamChecker::new(&manifest.capture);
    for event in &capture.events {
        issues.extend(checker.check(event));
    }

    match &capture.completeness {
        Completeness::Complete => {
            if !checker.ended() {
                issues.push(Issue::error(
                    IssueCode::MissingEnd,
                    None,
                    "capture claims complete but the timeline has no end event",
                ));
            }
        }
        other @ Completeness::DeclaredTruncated { .. } => {
            issues.push(Issue::info(
                IssueCode::Completeness,
                None,
                other.to_string(),
            ));
        }
        other @ Completeness::Incomplete { .. } => {
            issues.push(Issue::warning(
                IssueCode::Completeness,
                None,
                other.to_string(),
            ));
        }
    }

    if manifest.completion.state != CompletionState::Open {
        if manifest.actors != checker.actor_summaries() {
            issues.push(Issue::warning(
                IssueCode::ManifestMismatch,
                None,
                "manifest actor summary differs from the timeline",
            ));
        }
        if manifest.boundaries != checker.boundaries() {
            issues.push(Issue::warning(
                IssueCode::ManifestMismatch,
                None,
                "manifest boundaries differ from the timeline",
            ));
        }
        if let Some(declared) = manifest.completion.events {
            if declared != capture.events.len() as u64 {
                issues.push(Issue::warning(
                    IssueCode::ManifestMismatch,
                    None,
                    format!(
                        "manifest declares {declared} events but the capture holds {}",
                        capture.events.len()
                    ),
                ));
            }
        }
    }

    if manifest.capture.rules.target == RulesetTarget::Competitive {
        match competitive_conformance(&manifest.capture.rules.actual) {
            Conformance::Conforms => {}
            Conformance::Deviates { reasons } => issues.push(Issue::warning(
                IssueCode::RulesTarget,
                None,
                format!(
                    "actual rules deviate from the competitive target: {}",
                    reasons.join(", ")
                ),
            )),
            Conformance::Undetermined { unknown } => issues.push(Issue::info(
                IssueCode::RulesTarget,
                None,
                format!(
                    "competitive conformance undetermined; unknown: {}",
                    unknown.join(", ")
                ),
            )),
        }
    }
    issues
}
