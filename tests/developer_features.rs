//! Original regressions for the shipped hidden Developer rows.
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;
mod common;

use model::developer_reload::{qualifies, Counts, Queue, Reader};

#[test]
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

#[test]
fn developer_displayed_off_cancels_pending_and_inflight_reload() {
    use model::developer_ini::{control_enabled, Control};
    let mut snapshot = model::settings::Snapshot::factory();
    let q = Queue::<8>::new();
    let publish = |snapshot: &model::settings::Snapshot, page| {
        q.set_enabled(control_enabled(snapshot, page, Control::ReloadConfig));
    };
    publish(&snapshot, true);
    assert!(!q.request(), "UI20 Off negative control must not queue reads");
    snapshot.set("dev_reload_config", &[1, 0]).unwrap();
    publish(&snapshot, true);
    assert!(q.request());
    assert!(q.request());
    let old = q.take().unwrap();
    snapshot.set("dev_reload_config", &[0, 1]).unwrap();
    publish(&snapshot, true);
    assert!(!q.request());
    assert!(!q.current(old));
    q.finish(old);
    assert_eq!(q.counts(false), Counts { requested: 2, dropped: 2, ..Counts::default() });
    snapshot.set("dev_reload_config", &[1, 0]).unwrap();
    publish(&snapshot, true);
    assert!(q.request());
    publish(&snapshot, false);
    assert!(!q.request());
    assert_eq!(q.counts(false), Counts { requested: 3, dropped: 3, ..Counts::default() });
    assert_eq!(snapshot.get("dev_reload_config"), Some([1, 0].as_slice()));
}

#[test]
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

#[test]
fn developer_disable_cancels_queued_and_inflight_across_reenable() {
    let q = Queue::<8>::new();
    q.set_enabled(true);
    for _ in 0..3 { q.request(); }
    let old = q.take().unwrap();
    q.set_enabled(false);
    assert!(!q.current(old));
    assert!(!q.request());
    q.set_enabled(true);
    assert!(!q.current(old), "old work cannot reappear when enabled again");
    q.finish(old);
    q.request();
    q.finish(q.take().unwrap());
    assert_eq!(q.counts(false), Counts { requested: 4, completed: 1, dropped: 3, pending: 0 });
}

#[test]
fn developer_overflow_is_counted_without_coalescing_or_wrapping() {
    let q = Queue::<2>::new();
    q.set_enabled(true);
    for _ in 0..5 { assert!(q.request()); }
    assert_eq!(q.counts(false), Counts { requested: 5, completed: 0, pending: 2, dropped: 3 });
    q.finish(q.take().unwrap());
    q.request();
    q.set_enabled(true); // Publishing unchanged On must not cancel the backlog.
    assert_eq!(q.counts(false), Counts { requested: 6, completed: 1, pending: 2, dropped: 3 });
}

#[test]
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

#[test]
fn developer_reader_cancels_an_incomplete_file_before_next_chunk() {
    let dir = std::env::temp_dir().join(format!("tr-developer-cancel-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("dev.json");
    std::fs::write(&path, vec![0u8; 100_000]).unwrap();
    let q = Queue::<8>::new();
    q.set_enabled(true);
    q.request();
    let mut reader = Reader::default();
    assert_eq!(reader.drain(&q, path.to_str().unwrap()).pending, 1);
    let read_before = reader.bytes;
    q.set_enabled(false);
    assert_eq!(reader.drain(&q, path.to_str().unwrap()), Counts { requested: 1, completed: 0, pending: 0, dropped: 1 });
    assert_eq!(reader.bytes, read_before);
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(&dir).unwrap();
}

#[test]
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

#[test]
fn developer_debug_native_guards_and_complete_closure() {
    use model::{capabilities, developer_debug, sites::Feature};
    let Some(elf) = common::owned_main("developer_debug_native_guards_and_complete_closure") else { return };
    for site in developer_debug::SITES {
        assert_eq!(site.guard, elf.window(site.offset), "{} full64", site.name);
    }
    let get_float = capabilities::find("work_get_float").unwrap();
    assert!(developer_debug::SITES[0].overlaps(get_float));
    assert!(capabilities::unit_usable(Feature::DeveloperDebug, &|_| true));
    for rejected in ["developer_set_float", "developer_backshield", "lua_const_lookup",
        "utility_get_category", "utility_get_kind", "is_training_mode"] {
        assert!(!capabilities::unit_usable(Feature::DeveloperDebug, &|s| s.name != rejected), "{rejected}");
    }
}
