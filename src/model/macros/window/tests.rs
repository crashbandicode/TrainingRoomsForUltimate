lib_unit_tests! {
use super::*;
use crate::engine::macros::test_util::frames;

fn window(total: u32) -> FrameWindow {
    FrameWindow::new(total).unwrap()
}

/// What the runtime would report at the in-point: moving, mid-air.
fn snapshot_at(frame: u32) -> FrameSnapshot {
    FrameSnapshot {
        frame,
        state: StartState {
            fighter_kind: 8,
            costume: 1,
            facing: 1.0,
            situation: 3,
            status: 42,
            speed_x: 0.75,
            speed_y: 2.5,
        },
    }
}

fn crop_refuses_a_snapshot_for_any_other_frame() {
    let source = frames(600);
    let mut w = window(600);
    w.set_in(120);

    for stale in [0, 119, 121, 599] {
        let err = w.crop(&source, &snapshot_at(stale), "Cut", "").unwrap_err();
        assert!(
            matches!(err, MacroError::SnapshotFrame { selected: 120, snapshot } if snapshot == stale),
            "{stale}: {err}"
        );
    }

    // Moving the in-point invalidates a snapshot captured for the old one.
    let old = snapshot_at(120);
    w.set_in(180);
    assert!(matches!(
        w.crop(&source, &old, "Cut", ""),
        Err(MacroError::SnapshotFrame {
            selected: 180,
            snapshot: 120
        })
    ));
    assert!(w.crop(&source, &snapshot_at(180), "Cut", "").is_ok());
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("crop_refuses_a_snapshot_for_any_other_frame", crop_refuses_a_snapshot_for_any_other_frame as fn()),
    ]);
}
}
