//! Review checks for the capture-directory boundary of the public archive API.
use smash_match_data::archive::{open_timeline, ArchiveWriter, WriteOptions};
use smash_match_data::model::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct OwnedTemp(PathBuf);

impl OwnedTemp {
    fn new() -> Self {
        let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "smash-match-data-archive-review-{}-{stamp}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for OwnedTemp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn timeline_cannot_overwrite_its_own_manifest() {
    let temp = OwnedTemp::new();
    let mut manifest = Manifest::new(CaptureHeader::default());
    manifest.timeline_file = MANIFEST_FILE.into();
    assert!(ArchiveWriter::create(&temp.0, manifest, &WriteOptions::default()).is_err());
    assert!(!temp.0.join(MANIFEST_FILE).exists());
}

#[test]
fn timeline_writer_stays_inside_its_capture_directory() {
    let temp = OwnedTemp::new();
    let capture_dir = temp.0.join("capture");
    let mut manifest = Manifest::new(CaptureHeader::default());
    manifest.timeline_file = "../outside.jsonl".into();
    assert!(ArchiveWriter::create(capture_dir, manifest, &WriteOptions::default()).is_err());
    assert!(!temp.0.join("outside.jsonl").exists());
}

#[test]
fn timeline_reader_rejects_a_manifest_pointing_outside_the_capture() {
    let temp = OwnedTemp::new();
    let capture_dir = temp.0.join("capture");
    fs::create_dir(&capture_dir).unwrap();
    fs::write(temp.0.join("outside.jsonl"), b"").unwrap();
    let mut manifest = Manifest::new(CaptureHeader::default());
    manifest.timeline_file = "../outside.jsonl".into();
    assert!(open_timeline(&capture_dir, &manifest).is_err());
}
