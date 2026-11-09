//! Contract for card L1 (sequence drill, model and host halves). Written by the primary before the code; the worker
//! makes these pass without editing this file. Mounted from `engine/mod.rs` under `cfg(test)`.
//!
//! The behaviour comes from `model/drill.rs` (a clean step advances; a drop returns to combo 1, sets the streak to 0
//! and keeps best; a finished run adds one to the streak) and `docs/RECORDING.md` § Sequences and the one drill (a
//! sequence stores combo ids in order; a deleted combo is skipped).

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::library::{DrillKind, Sequence, SEQUENCES_DIR};
use super::rr_host::{drill_grade_target, start_sequence_drill};
use crate::model::combo::{Combo, LinkRec, Outcome, Summary};
use crate::model::combo_record::{ComboLibrary, ComboRecord};
use crate::model::drill::SequenceDrill;
use crate::model::frame_strip::{Move, Verdict};
use crate::model::store::Store;

fn clean() -> Summary {
    Summary { outcome: Outcome::Completed, graded: 1, true_links: 0, total: 1, clean: true, fault: None }
}

fn dropped() -> Summary {
    Summary { outcome: Outcome::Stopped, graded: 1, true_links: 0, total: 1, clean: false, fault: None }
}

fn scratch(tag: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("tr-seq-drill-{tag}-{n}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn save_combo(root: &std::path::Path, name: &str) -> String {
    let combo =
        Combo::from_links([LinkRec::presses(Move::Aerial, Move::Aerial, Verdict::True(None), None, &[])]).unwrap();
    ComboLibrary::new(root.join("combos")).save_new(&ComboRecord::new(name.into(), combo, None)).unwrap()
}

#[test]
fn sequence_drill_grades_each_step_in_order_and_counts_a_finished_run() {
    let mut drill = SequenceDrill::new("seq-1", &["combo-1", "combo-2", "combo-3"]);
    assert_eq!(drill.id(), "seq-1");
    assert_eq!((drill.target(), drill.step()), (Some("combo-1"), (1, 3)));
    assert_eq!(drill.note_summary(&clean()), Some("combo-2"));
    assert_eq!(drill.step(), (2, 3));
    assert_eq!(drill.note_summary(&clean()), Some("combo-3"));
    assert_eq!(drill.step(), (3, 3));
    assert_eq!((drill.streak(), drill.best()), (0, 0), "the run is not finished yet");
    assert_eq!(drill.note_summary(&clean()), Some("combo-1"), "a finished run starts again at step 1");
    assert_eq!((drill.target(), drill.step()), (Some("combo-1"), (1, 3)));
    assert_eq!((drill.streak(), drill.best()), (1, 1));
}

#[test]
fn sequence_drill_drop_returns_to_step_one_and_keeps_best() {
    let mut drill = SequenceDrill::new("seq-1", &["combo-1", "combo-2"]);
    for _ in 0..4 {
        drill.note_summary(&clean());
    }
    assert_eq!((drill.streak(), drill.best()), (2, 2));
    assert_eq!(drill.note_summary(&clean()), Some("combo-2"));
    assert_eq!(drill.note_summary(&dropped()), Some("combo-1"), "a drop returns to combo 1");
    assert_eq!(drill.step(), (1, 2));
    assert_eq!((drill.streak(), drill.best()), (0, 2), "streak 0, best kept");
}

#[test]
fn sequence_drill_of_one_combo_counts_every_clean_attempt() {
    let mut drill = SequenceDrill::new("seq-1", &["combo-1"]);
    assert_eq!(drill.note_summary(&clean()), Some("combo-1"));
    assert_eq!(drill.note_summary(&clean()), Some("combo-1"));
    assert_eq!((drill.step(), drill.streak(), drill.best()), ((1, 1), 2, 2));
    assert_eq!(drill.note_summary(&dropped()), Some("combo-1"));
    assert_eq!((drill.streak(), drill.best()), (0, 2));
}

#[test]
fn sequence_drill_with_no_combo_left_grades_nothing() {
    let mut drill = SequenceDrill::new("seq-empty", &[]);
    assert_eq!((drill.target(), drill.step()), (None, (0, 0)));
    assert_eq!(drill.note_summary(&clean()), None);
    assert_eq!((drill.streak(), drill.best()), (0, 0));
}

#[test]
fn sequence_drill_start_skips_deleted_combos() {
    let root = scratch("start");
    let first = save_combo(&root, "First");
    let second = save_combo(&root, "Second");
    let sequence = Sequence::new("Seq", vec![first.clone(), "deleted-combo".into(), second.clone()]).unwrap();
    let id = Store::<Sequence>::new(root.join(SEQUENCES_DIR)).save_new(&sequence).unwrap();
    let mut drill = start_sequence_drill(&root, &id).expect("the sequence file loads");
    assert_eq!(drill.id(), id);
    assert_eq!((drill.target(), drill.step()), (Some(first.as_str()), (1, 2)), "the deleted combo is not a step");
    assert_eq!(drill.note_summary(&clean()), Some(second.as_str()));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn sequence_drill_start_of_a_missing_file_is_none() {
    let root = scratch("missing");
    assert!(start_sequence_drill(&root, "no-such-sequence").is_none());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn sequence_drill_grade_target_follows_the_drill_kind() {
    let drill = SequenceDrill::new("seq-1", &["combo-1", "combo-2"]);
    assert_eq!(drill_grade_target(Some(DrillKind::Combo), Some("combo-9"), None), Some("combo-9"));
    assert_eq!(drill_grade_target(Some(DrillKind::Combo), Some("combo-9"), Some(&drill)), Some("combo-9"));
    assert_eq!(drill_grade_target(Some(DrillKind::Sequence), Some("seq-1"), Some(&drill)), Some("combo-1"));
    assert_eq!(drill_grade_target(Some(DrillKind::Sequence), Some("seq-2"), Some(&drill)), None, "another sequence");
    assert_eq!(drill_grade_target(Some(DrillKind::Sequence), Some("seq-1"), None), None, "not started yet");
    assert_eq!(drill_grade_target(None, Some("combo-9"), Some(&drill)), None, "an unknown kind grades nothing");
    assert_eq!(drill_grade_target(None, None, None), None);
}
