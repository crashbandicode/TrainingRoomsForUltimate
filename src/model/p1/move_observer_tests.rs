lib_unit_tests! {
use super::*;

fn sample(game_frame: u64, status: i32, prev0: i32) -> Observation {
    Observation {
        game_frame,
        status,
        prev0,
        p1_damage: 0.0,
        native_interrupted: None,
    }
}

fn completed_trace(statuses: &[i32], expected: Family) {
    let mut observer = MoveObserver::default();
    let mut completed = Vec::new();
    for (tick, &status) in statuses.iter().enumerate() {
        let event = observer.step(sample(tick as u64, status, 0));
        assert!(event.interrupted.is_none());
        completed.extend(event.completed);
    }
    assert_eq!(completed, vec![expected]);
    assert_eq!(observer.active_family(), None);
}

fn aerial_and_special_aftermath_and_extended_phases_continue_one_instance() {
    completed_trace(&[0x36, 0x18, 0x18, 0], Family::Aerial);
    completed_trace(&[0x1de, 0x10, 0x19, 0], Family::UpSpecial);
    completed_trace(&[0x1df, 0x1e1, 0x1e2, 0x1e2, 0], Family::DownSpecial);
}

fn explicit_native_signal_and_common_interruptions_are_separate_from_armor_policy() {
    let mut observer = MoveObserver::default();
    observer.step(sample(0, 0x27, 0));
    let native = observer.step(Observation {
        native_interrupted: Some(true),
        ..sample(1, 0x27, 0)
    });
    assert_eq!(
        native.interrupted.unwrap().reason,
        InterruptionReason::NativeSignal
    );
    assert_eq!(observer.step(sample(2, 0x27, 0)).active_family, None);
    observer.reset();
    observer.step(sample(3, 0x2d, 0));
    let armor = observer.step(Observation {
        p1_damage: 5.0,
        native_interrupted: Some(false),
        ..sample(4, 0x2e, 0x2d)
    });
    assert_eq!(armor.interrupted, None);
    assert_eq!(armor.active_family, Some(Family::SideSmash));
    let hitstun = observer.step(Observation {
        p1_damage: 5.0,
        native_interrupted: Some(false),
        ..sample(5, 0x47, 0x2e)
    });
    assert_eq!(
        hitstun.interrupted.unwrap().reason,
        InterruptionReason::CommonStatus
    );
    assert_eq!(hitstun.completed, None);
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("aerial_and_special_aftermath_and_extended_phases_continue_one_instance", aerial_and_special_aftermath_and_extended_phases_continue_one_instance as fn()),
        ("explicit_native_signal_and_common_interruptions_are_separate_from_armor_policy", explicit_native_signal_and_common_interruptions_are_separate_from_armor_policy as fn()),
    ]);
}
}
