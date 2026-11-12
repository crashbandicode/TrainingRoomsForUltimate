lib_unit_tests! {
use super::*;
use crate::engine::macros::{LiveInput, StartState};
use crate::engine::macros::handoff::{FixedReplayId, FixedReplayTitle, LiveArchive, LiveEvent};

fn frame(value: u32) -> LiveFrame {
    LiveFrame { input: LiveInput { buttons: value, axes: [0.25, -0.5, 0.0, 0.0] },
        start: StartState::neutral(8, 0, 1.0), x: value as f32, y: 2.0,
        position_z: 0.0, speed_z: 0.0, body: 42 }
}

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

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("worker_first_seal_handoff_save_binding_and_loss_isolation", worker_first_seal_handoff_save_binding_and_loss_isolation as fn()),
        ("worker_first_seal_fixed_facts_and_worst_formatted_line_fit_budget", worker_first_seal_fixed_facts_and_worst_formatted_line_fit_budget as fn()),
    ]);
}
}
