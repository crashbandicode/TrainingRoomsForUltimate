use super::*;
use crate::engine::macros::{LiveInput, StartState};
use crate::engine::macros::handoff::{FixedReplayId, FixedReplayTitle, LiveArchive, LiveEvent};

fn frame(value: u32) -> LiveFrame {
    LiveFrame { input: LiveInput { buttons: value, axes: [0.25, -0.5, 0.0, 0.0] },
        start: StartState::neutral(8, 0, 1.0), x: value as f32, y: 2.0,
        position_z: 0.0, speed_z: 0.0, body: 42 }
}

#[test]
fn worker_first_seal_gap_backward_and_simultaneous_invalid_preserve_prefix() {
    // The next rejected runtime counter is unknown. These are distinct counterexamples,
    // not an invented observation of the p4c-03 failure.
    for (next, invalid, flags) in [(3816, false, 1), (3813, false, 1), (3816, true, 3)] {
        let mut pool = LiveCapturePool::new();
        let key = pool.start_record(0);
        for tick in 215..3815 {
            for entry in 0..2 { assert!(pool.push(key, entry, tick, frame(tick))); }
        }
        assert!(pool.take_first_seal().is_none());
        let mut bad = frame(999);
        if invalid { bad.input.axes[3] = f32::NAN; }
        assert!(!pool.push(key, 1, next, bad));
        assert!(!pool.push(key, 1, 3815, frame(11)));
        pool.seal_entry(key, 0);
        pool.finish_record(); // Later clean completion must not erase first cause.
        let saved = pool.snapshot(key).unwrap();
        let note = pool.take_first_seal().unwrap();
        assert_eq!(note, FirstSeal { key, reason: SealReason::Sample, flags, entry: Some(1),
            frames: 3600, first: Some(215), last: Some(3814), next: Some(next) });
        assert!(pool.take_first_seal().is_none());
        assert!(saved.truncated);
        assert_eq!(saved.tracks.iter().map(|t| t.frames.len()).collect::<Vec<_>>(), [3600, 3600]);
    }
}

#[test]
fn worker_first_seal_late_entry_and_invalid_first_sample_are_distinct() {
    let mut pool = LiveCapturePool::new();
    let key = pool.start_record(7);
    assert!(pool.push(key, 0, 19, frame(1)));
    assert!(!pool.push(key, 2, 20, frame(2)));
    let note = pool.take_first_seal().unwrap();
    assert_eq!((note.flags, note.frames, note.first, note.last, note.next), (5, 0, Some(19), None, Some(20)));
    assert_eq!(note.entry, Some(2));
    assert!(!pool.push(key, 2, 19, frame(3)));
    assert!(pool.take_first_seal().is_none());

    let key = pool.start_record(7);
    let mut invalid = frame(4); invalid.start.fighter_kind = -1;
    assert!(!pool.push(key, 0, 71, invalid));
    let note = pool.take_first_seal().unwrap();
    assert_eq!((note.reason, note.flags, note.frames, note.first, note.last), (SealReason::Sample, 2, 0, None, None));
    assert!(pool.snapshot(key).is_none());
}

#[test]
fn worker_first_seal_limit_explicit_and_partial_finish_use_actual_branches() {
    let mut pool = LiveCapturePool::new();
    let key = pool.start_record(0);
    for tick in 0..MAX_FRAMES as u32 { assert!(pool.push(key, 0, tick, frame(tick))); }
    assert!(!pool.push(key, 0, MAX_FRAMES as u32, frame(0)));
    let note = pool.take_first_seal().unwrap();
    assert_eq!((note.reason, note.frames, note.last, note.next),
        (SealReason::Limit, 36000, Some(35999), Some(36000)));
    pool.finish_partial();
    assert!(pool.take_first_seal().is_none());

    let key = pool.start_record(2);
    assert!(pool.push(key, 3, 8, frame(1)));
    pool.seal_entry(key, 3);
    let note = pool.take_first_seal().unwrap();
    assert_eq!((note.reason, note.entry, note.frames, note.last, note.next),
        (SealReason::Explicit, Some(3), 1, Some(8), None));
    pool.seal_entry(key, 3);
    pool.finish_partial();
    assert!(pool.take_first_seal().is_none());

    let key = pool.start_record(3);
    assert!(pool.push(key, 0, 8, frame(1)));
    assert!(pool.push(key, 1, 8, frame(2)));
    pool.finish_partial();
    let note = pool.take_first_seal().unwrap();
    assert_eq!((note.reason, note.entry, note.frames), (SealReason::PartialFinish, None, 2));
    assert_eq!(note.key, key);
    pool.finish_partial();
    assert!(pool.take_first_seal().is_none());
}

#[test]
fn worker_first_seal_reservation_pure_fact_probe_has_no_allocator_framework() {
    // Exercise the exact bounded fact collector used by the actual reserve-error branch.
    // This does not claim to force a real allocation failure.
    let key = NativeRecordKey { generation: 81, index: 4 };
    let mut live = LiveRecord { session: LiveSession::default(), last_tick: [None; 8], sealed: [false; 8] };
    let mut out = None;
    observe_seal(&mut out, &live, key, SealReason::Reservation, 0, Some(7), Some(123));
    assert_eq!(out.unwrap(), FirstSeal { key, reason: SealReason::Reservation, flags: 0,
        entry: Some(7), frames: 0, first: None, last: None, next: Some(123) });
    live.session.truncated = true;
    observe_seal(&mut out, &live, key, SealReason::Limit, 0, Some(0), Some(999));
    assert_eq!(out.unwrap().reason, SealReason::Reservation);
}

#[test]
fn worker_first_seal_stale_keys_reset_invalidation_and_reuse_do_not_relabel() {
    let mut pool = LiveCapturePool::new();
    let old = pool.start_record(0);
    assert!(pool.push(old, 0, 2, frame(1)));
    let key = pool.start_record(0);
    pool.seal_entry(old, 0);
    pool.seal_entry(key, -1);
    pool.seal_entry(key, 8);
    assert!(!pool.push(old, 0, 3, frame(2)));
    assert!(pool.take_first_seal().is_none());
    assert!(pool.push(key, 0, 2, frame(1)));
    pool.seal_entry(key, 0);
    pool.invalidate_record(0);
    assert!(pool.take_first_seal().is_none());
    let key = pool.start_record(1);
    pool.seal_entry(key, 0);
    pool.clear();
    assert!(pool.take_first_seal().is_none());
    let next = pool.start_record(1);
    assert_ne!(next, key);
    assert!(pool.push(next, 0, 10, frame(2)));
    assert!(pool.take_first_seal().is_none());
}

#[test]
fn worker_first_seal_handoff_save_binding_and_loss_isolation() {
    let mut archive = LiveArchive::new();
    let key = NativeRecordKey { generation: 1, index: 0 };
    let id = FixedReplayId::new("first-seal").unwrap();
    let title = FixedReplayTitle::new(&[70, 97, 99, 116, 115]).unwrap();
    archive.apply(1, LiveEvent::Start { key }, 0);
    archive.apply(2, LiveEvent::Sample { key, entry: 0, tick: 12, sample: frame(7) }, 0);
    archive.apply(3, LiveEvent::Sample { key, entry: 0, tick: 14, sample: frame(8) }, 0);
    archive.apply(4, LiveEvent::Finish { partial: false }, 0);
    archive.apply(5, LiveEvent::Bind { save: 9, key }, 0);
    let note = archive.take_first_seal().unwrap();
    assert_eq!((note.key, note.frames, note.last, note.next), (key, 1, Some(12), Some(14)));
    let commit = archive.apply(6, LiveEvent::Commit { save: 9, id, title, miis: false }, 0).unwrap();
    assert!(commit.session.truncated);
    assert_eq!(commit.session.tracks[0].frames[0].input.buttons, 7);
    assert!(archive.take_first_seal().is_none());

    let next = NativeRecordKey { generation: 2, index: 0 };
    archive.apply(7, LiveEvent::Start { key: next }, 0);
    archive.apply(8, LiveEvent::Seal { key: next, entry: 1 }, 0);
    // Loss clears both uncertain capture identity and its unconsumed diagnostic.
    archive.apply(10, LiveEvent::Bind { save: 10, key: next }, 9);
    assert!(archive.take_first_seal().is_none());
    assert!(archive.apply(11, LiveEvent::Commit { save: 10, id, title, miis: false }, 9).is_none());
    let fresh = NativeRecordKey { generation: 3, index: 0 };
    archive.apply(12, LiveEvent::Start { key: fresh }, 9);
    archive.apply(13, LiveEvent::Seal { key: next, entry: 1 }, 9);
    assert!(archive.take_first_seal().is_none());
    archive.apply(14, LiveEvent::Sample { key: fresh, entry: 0, tick: 2, sample: frame(3) }, 9);
    archive.apply(15, LiveEvent::Finish { partial: true }, 9);
    assert_eq!(archive.take_first_seal().unwrap().key, fresh);
}

#[test]
fn worker_first_seal_serialized_bytes_match_independent_expected_prefix() {
    use crate::engine::macros::{SharedCaptureArchive, test_util::TempDir};
    let roots = [TempDir::new(), TempDir::new(), TempDir::new()];
    let expected = LiveSession { tracks: vec![LiveTrack { entry: 0, frames: vec![frame(11), frame(22)] }],
        truncated: true, first_counter: Some(41) };
    for (index, consume) in [false, true].into_iter().enumerate() {
        let mut pool = LiveCapturePool::new();
        let key = pool.start_record(0);
        assert!(pool.push(key, 0, 41, frame(11)));
        assert!(pool.push(key, 0, 42, frame(22)));
        assert!(!pool.push(key, 0, 44, frame(33)));
        if consume { assert!(pool.take_first_seal().is_some()); }
        pool.finish_record();
        let saved = pool.snapshot(key).unwrap();
        SharedCaptureArchive::new(roots[index].path()).save_with_outcome("byte-oracle", "Facts", "13.0.5", &saved).unwrap();
    }
    let saved = SharedCaptureArchive::new(roots[2].path()).save_with_outcome("byte-oracle", "Facts", "13.0.5", &expected).unwrap();
    let dir = format!("{}.smd", saved.id);
    for name in ["manifest.json".to_owned(), "timeline.jsonl".to_owned(), format!("{}.tmrc", saved.id)] {
        let expected = std::fs::read(roots[2].path().join(&dir).join(&name)).unwrap();
        for root in &roots[..2] {
            assert_eq!(std::fs::read(root.path().join(&dir).join(&name)).unwrap(), expected, "{name}");
        }
    }
}

#[test]
fn worker_first_seal_fixed_facts_and_worst_formatted_line_fit_budget() {
    assert!(std::mem::size_of::<FirstSeal>() <= 80);
    for reason in [SealReason::Sample, SealReason::Reservation, SealReason::Limit, SealReason::Explicit, SealReason::PartialFinish] {
        for entry in [None, Some(i32::MIN), Some(i32::MAX)] {
            let note = FirstSeal { key: NativeRecordKey { generation: u64::MAX, index: u32::MAX },
                reason, flags: u8::MAX, entry, frames: u32::MAX,
                first: Some(u32::MAX), last: Some(u32::MAX), next: Some(u32::MAX) };
            let line = format!("[ReplayCaptureSeal] serial={} {}", u64::MAX, note);
            assert!(line.len() <= 256, "{} {line}", line.len());
        }
    }
}
