//! The runtime's diagnostics channel (`src/runtime/diag_log.rs`) is plain `std` code on top of the
//! model, so it is compiled and exercised on the host by path, against temporary files (defect D1:
//! `practice-diagnostics.log` stayed empty in Eden). The Switch's file calls are `std::fs` ones; what
//! the host cannot show is the card's behaviour, which the Eden rows cover.

use std::fs;

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

#[path = "../src/runtime/clock.rs"]
pub mod clock;

// The drain thread starts through `runtime::threads::spawn` (stream/stack); plain std without the diag feature.
#[path = "../src/runtime/threads.rs"]
#[allow(dead_code)]
pub mod threads;

// `diag_log` names the clock as `crate::runtime::clock` and the spawn as `crate::runtime::threads`.
mod runtime {
    pub use super::clock;
    pub use super::threads;
}

#[path = "../src/runtime/diag_log.rs"]
#[allow(dead_code)]
mod diag_log;

fn read(path: &std::path::Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn the_ring_reaches_the_file_rotates_and_reports_failures() {
    let dir = std::env::temp_dir().join(format!("trfu-diag-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let (cur, prev, events) = (dir.join("d.log"), dir.join("d.previous.log"), dir.join("events.log"));
    let path = |p: &std::path::Path| p.to_str().unwrap().to_owned();

    // A shipped line reaches the file, a probe tag does not; lines carry "<ms> #<seq> ".
    diag_log::init_at(&path(&cur), &path(&prev), &path(&events));
    assert!(cur.exists(), "the empty current file is created at start");
    diag_log::diagnostic(format_args!("[PracticeDiagnostics] adapter=ring session_start"));
    diag_log::diagnostic(format_args!("[TrainingRooms] boot test"));
    diag_log::diagnostic(format_args!("[Hud] probe line"));
    diag_log::flush();
    let text = read(&cur);
    assert!(text.contains(" #1 [PracticeDiagnostics] adapter=ring session_start\n"), "{text:?}");
    assert!(text.contains(" #2 [TrainingRooms] boot test\n"), "{text:?}");
    assert!(!text.contains("probe"), "a shipped build does not write probe tags: {text:?}");
    assert!(!events.exists(), "no failure was reported");

    // A second start rotates: the old file becomes the previous one, the new one starts empty.
    diag_log::init_at(&path(&cur), &path(&prev), &path(&events));
    assert_eq!(read(&prev), text);
    assert_eq!(read(&cur), "");

    // Enqueue owns the bytes without the pre-writer filesystem fallback.
    let committed = diag_log::ring_stats().0.committed;
    let mut owned_note = String::from("[TrainingRooms] queued serial=731 first=Some(215)");
    diag_log::enqueue_diagnostic(format_args!("{owned_note}"));
    owned_note.clear(); // Capture lifecycle no longer owns the queued bytes.
    assert_eq!(read(&cur), "");
    assert_eq!(diag_log::ring_stats().0.committed, committed + 1);
    diag_log::diagnostic(format_args!("[Hud] filtered must not drain queued evidence"));
    assert_eq!(read(&cur), "", "a filtered normal call must still skip draining");
    assert_eq!(diag_log::ring_stats().0.committed, committed + 1);
    diag_log::diagnostic(format_args!("[TrainingRooms] normal fallback after queued"));
    let queued = read(&cur);
    let first = queued.find("queued serial=731 first=Some(215)").unwrap();
    let normal = queued.find("normal fallback after queued").unwrap();
    assert!(first < normal);
    assert_eq!(queued.matches("queued serial=731").count(), 1);

    // Passing the filter is not the same as a successful commit: a full ring
    // must still be drained by the normal pre-writer API, including its drops.
    for _ in 0..2048 {
        diag_log::enqueue_diagnostic(format_args!(
            "[TrainingRooms] queued fill 0123456789012345678901234567890123456789"));
    }
    let dropped = diag_log::ring_stats().0.dropped;
    assert!(dropped > 0, "the bounded ring must have filled without a caller drain");
    let before_full_drain = read(&cur);
    diag_log::diagnostic(format_args!("[TrainingRooms] full ring normal fallback"));
    assert!(diag_log::ring_stats().0.dropped > dropped);
    assert_ne!(read(&cur), before_full_drain, "a dropped normal line must still trigger draining");

    // Many lines: the files stay within the limit and the newest lines are in the current file.
    for n in 0..3000 {
        diag_log::diagnostic(format_args!("[TrainingRooms] line {n:05} {}", "x".repeat(60)));
        if n % 100 == 99 {
            diag_log::flush();
        }
    }
    diag_log::flush();
    let (cur_len, prev_len) = (fs::metadata(&cur).unwrap().len(), fs::metadata(&prev).unwrap().len());
    assert!(cur_len <= model::storage::DIAGNOSTIC_LIMIT && prev_len <= model::storage::DIAGNOSTIC_LIMIT, "{cur_len} {prev_len}");
    assert!(prev_len > 100_000, "the first file filled up before rotating: {prev_len}");
    assert!(read(&cur).contains("line 02999"), "the newest line is in the current file");
    assert!(read(&cur).ends_with('\n'));

    // A file that cannot be created is reported, with its error, to the failures file.
    let missing = dir.join("no-such-folder").join("d.log");
    diag_log::init_at(&path(&missing), &path(&prev), &path(&events));
    let failure = read(&events);
    assert!(failure.contains("diag_log start:"), "{failure:?}");
    diag_log::diagnostic(format_args!("[TrainingRooms] after the failure"));
    diag_log::flush();
    assert!(!missing.exists());

    // The actual writer starts by marking readiness, then draining. Readiness
    // alone does not write; both pre-start and ready enqueues survive to flush.
    diag_log::init_at(&path(&cur), &path(&prev), &path(&events));
    diag_log::enqueue_diagnostic(format_args!("[TrainingRooms] before writer serial=901"));
    assert_eq!(read(&cur), "");
    diag_log::writer_started();
    assert_eq!(read(&cur), "");
    diag_log::flush();
    let started = read(&cur);
    assert_eq!(started.matches("before writer serial=901").count(), 1);
    diag_log::enqueue_diagnostic(format_args!("[TrainingRooms] after writer serial=902"));
    assert_eq!(read(&cur), started, "enqueue-only stays asynchronous after readiness");
    diag_log::flush();
    diag_log::flush();
    let flushed = read(&cur);
    assert_eq!(flushed.matches("before writer serial=901").count(), 1);
    assert_eq!(flushed.matches("after writer serial=902").count(), 1);
    assert!(flushed.find("serial=901").unwrap() < flushed.find("serial=902").unwrap());

    let _ = fs::remove_dir_all(&dir);
}
