//! Original regressions for the shipped hidden Developer rows.
// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;
mod common;

use model::developer_reload::{qualifies, Counts, Queue, Reader};

fn developer_published_snapshot_matches_displayed_on_off_and_hidden_page() {
    use model::developer_ini::{control_enabled, Control};
    use model::settings::Snapshot;
    for control in [Control::DebugHooks, Control::ReloadConfig] {
        let mut snapshot = Snapshot::factory();
        assert_eq!(snapshot.get(control.id()), Some([0, 1].as_slice()));
        assert!(!control_enabled(&snapshot, true, control), "factory Off must stay inert");
        for (row, enabled) in [([0, 1], false), ([1, 0], true), ([0, 0], false)] {
            snapshot.set(control.id(), &row).unwrap();
            assert_eq!(control_enabled(&snapshot, true, control), enabled);
            let before = snapshot.clone();
            assert!(!control_enabled(&snapshot, false, control));
            assert_eq!(snapshot, before, "hidden-page publication must preserve the stored choice");
        }
    }
}

fn developer_held_chord_counts_each_p1_poll_and_no_other_input() {
    use model::chords::bit;
    let q = Queue::<16>::new();
    let chord = bit::L | bit::R | bit::A;
    assert!(!q.request());
    q.set_enabled(true);
    for missing in [bit::L, bit::R, bit::A] { assert!(!qualifies(0, true, chord & !missing)); }
    assert!(!qualifies(1, true, chord));
    assert!(!qualifies(0, false, chord));
    for _ in 0..7 { assert!(qualifies(0, true, chord | bit::B)); assert!(q.request()); }
    assert_eq!(q.counts(false), Counts { requested: 7, pending: 7, ..Counts::default() });
    for _ in 0..7 { q.finish(q.take().unwrap()); }
    assert_eq!(q.counts(false), Counts { requested: 7, completed: 7, ..Counts::default() });
}

fn developer_reader_continues_large_files_and_reports_missing_files() {
    let dir = std::env::temp_dir().join(format!("tr-developer-reader-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("dev.json");
    let text = path.to_str().unwrap();
    let mut bytes = vec![b' '; 100_000];
    bytes[0] = b'{';
    bytes[99_999] = b'}';
    std::fs::write(&path, &bytes).unwrap();
    let q = Queue::<8>::new();
    q.set_enabled(true);
    q.request();
    q.request();
    let mut reader = Reader::default();
    let first = reader.drain(&q, text);
    assert_eq!(first.pending, 2);
    assert_eq!(first.completed, 0, "a turn bounds work, not accepted file size");
    for _ in 0..16 { if reader.drain(&q, text).pending == 0 { break; } }
    assert_eq!(q.counts(false).completed, 2);
    assert_eq!(reader.bytes, 200_000);
    assert_eq!(std::fs::read(&path).unwrap(), bytes, "reload never rewrites the file");
    std::fs::remove_file(&path).unwrap();
    q.request();
    assert_eq!(reader.drain(&q, text).completed, 3);
    assert_eq!(reader.errors, 1, "an unreadable request is reported, never silently lost");
    std::fs::remove_dir(&dir).unwrap();
}

fn developer_debug_preserves_original_once_and_observes_only_after_it() {
    use std::cell::RefCell;
    for eligible in [false, true] {
        let calls = RefCell::new(Vec::new());
        model::developer_debug::write_then_observe(
            || calls.borrow_mut().push("original"),
            || { calls.borrow_mut().push("eligibility"); eligible },
            || calls.borrow_mut().push("backshield"),
        );
        assert_eq!(*calls.borrow(), if eligible { vec!["original", "eligibility", "backshield"] }
            else { vec!["original", "eligibility"] });
    }
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("developer_published_snapshot_matches_displayed_on_off_and_hidden_page", developer_published_snapshot_matches_displayed_on_off_and_hidden_page as fn()),
        ("developer_held_chord_counts_each_p1_poll_and_no_other_input", developer_held_chord_counts_each_p1_poll_and_no_other_input as fn()),
        ("developer_reader_continues_large_files_and_reports_missing_files", developer_reader_continues_large_files_and_reports_missing_files as fn()),
        ("developer_debug_preserves_original_once_and_observes_only_after_it", developer_debug_preserves_original_once_and_observes_only_after_it as fn()),
    ]);
}
