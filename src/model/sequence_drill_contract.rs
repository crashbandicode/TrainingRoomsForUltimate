//! Contract for card L1 (sequence drill, model and host halves). Written by the primary before the code; the worker
//! makes these pass without editing this file. Mounted from `engine/mod.rs` under `cfg(test)`.
//!
//! The behaviour comes from `model/drill.rs` (a clean step advances; a drop returns to combo 1, sets the streak to 0
//! and keeps best; a finished run adds one to the streak) and `docs/RECORDING.md` § Sequences and the one drill (a
//! sequence stores combo ids in order; a deleted combo is skipped).


use super::library::DrillKind;
use super::rr_host::drill_grade_target;
use crate::model::drill::SequenceDrill;

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

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("sequence_drill_grade_target_follows_the_drill_kind", sequence_drill_grade_target_follows_the_drill_kind as fn()),
    ]);
}
