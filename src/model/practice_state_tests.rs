lib_unit_tests! {
use super::*;

#[test]
fn the_sample_mailbox_is_sized_to_one_poll_of_the_busiest_game() {
    // BTT with Ice Climbers: two fighter passes and one P1 input poll per frame, for the two frames between drains.
    assert_eq!(SAMPLE_SLOTS, (2 + 1) * 2);
    // The merge-5 mailbox holds six inline samples. Extra padding in each sample is paid six times in BSS.
    let bytes = core::mem::size_of::<Fields>();
    assert!(bytes <= 224, "practice fields use {bytes} bytes per slot (merge 5: 224)");
}

#[test]
fn practice_game_wire_fields() {
    for (game, name, phase) in [
        (Game::Eggs, "eggs", "live"),
        (Game::Btt, "btt", "placing"),
        (Game::Mashout, "mashout", "held"),
        (Game::Tech, "tech", "wizard"),
        (Game::Parry, "parry", "armed"),
        (Game::GuardEdge, "guard_edge", "ready"),
        (Game::Juggler, "juggler", "result"),
    ] {
        let mut wire = String::new();
        let fields = Fields {
            phase, wizard_step: "cpu", wizard_selection: "offstage",
            wizard_point: Some([45.25, -5.0]), score: Some(4), attempt_frames: Some(60),
            remaining_frames: Some(12), streak: Some(2), best: Some(7), attempts: Some(3),
            hits: Some(1), target_count: Some(10), last_outcome: "landed", ..Fields::default()
        };
        let mut thinks = [0; CPU_ENTRIES];
        thinks[1] = 23;
        snapshot(Some(game), fields, thinks, &mut wire);
        for line in ["schema=tr.practice/1", &format!("game={name}"), &format!("phase={phase}"),
            "wizard.step=cpu", "wizard.selection=offstage", "wizard.x=45.25", "wizard.y=-5",
            "score=4", "attempt.frames=60", "remaining.frames=12", "streak=2", "best=7",
            "attempts=3", "hits=1", "target.count=10", "last.outcome=landed", "cpu.1.ai_thinks=23"] {
            assert!(wire.lines().any(|actual| actual == line), "missing {line:?} for {name} in {wire:?}");
        }
        assert!(wire.len() < super::super::bridge_state::MAX);
    }
}

#[test]
fn idle_and_starting_have_no_stale_game_fields() {
    let mut wire = String::new();
    snapshot(Some(Game::Btt), Fields::default(), [0; CPU_ENTRIES], &mut wire);
    assert!(wire.contains("game=btt\nphase=starting\n"));
    snapshot(None, Fields { phase: "done", score: Some(12), ..Fields::default() }, [0; CPU_ENTRIES], &mut wire);
    assert!(wire.contains("game=none\nphase=idle\n"));
    assert!(!wire.contains("score="));
}

#[test]
fn a_fresh_run_does_not_inherit_the_previous_outcome() {
    let mut current = Fields { phase: "result", last_outcome: "complete", score: Some(10), ..Fields::default() };
    adopt_sample(&mut current, Fields { phase: "live", score: Some(0), ..Fields::default() });
    assert_eq!(current.last_outcome, "");
    assert_eq!(current.score, Some(0));
    let mut wire = String::new();
    snapshot(Some(Game::Eggs), current, [0; CPU_ENTRIES], &mut wire);
    assert!(!wire.lines().any(|line| line.starts_with("last.outcome=")), "{wire}");
}

#[test]
fn practice_publishes_in_training_and_the_waiting_room() {
    for scene in ["training", "lobby"] {
        assert_eq!(published_activation(scene, || (7, Some(Game::Parry))), (7, Some(Game::Parry)));
    }
    assert_eq!(published_activation("", || panic!("no live layout must not read activation")), (0, None));
}

#[test]
fn think_counts_are_per_entry_and_offline_admitted() {
    let counts = core::array::from_fn::<_, CPU_ENTRIES, _>(|_| core::sync::atomic::AtomicU64::new(0));
    assert_eq!(think_entry(0, Some(1), |_| panic!("Off cannot inspect the other CPU entries")), None);
    assert_eq!(think_entry(2, None, |entry| entry == 0 || entry == 8), None);
    for index in [
        think_entry(2, Some(1), |_| panic!("primary match cannot inspect other entries")),
        think_entry(2, None, |entry| entry == 4),
        think_entry(2, Some(1), |_| false),
        think_entry(2, None, |entry| entry == 7),
    ].into_iter().flatten() {
        counts[index].fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    }
    assert_eq!(counts.map(core::sync::atomic::AtomicU64::into_inner), [0, 2, 0, 0, 1, 0, 0, 1]);
}

#[test]
fn the_practice_snapshot_carries_the_cpu_view() {
    let wire = |wizard_cpu| {
        let mut wire = String::new();
        let fields = Fields {
            phase: "wizard", wizard_step: "cpu", wizard_point: Some([-199.0, -8.0]), wizard_cpu, ..Fields::default()
        };
        snapshot(Some(Game::GuardEdge), fields, [0; CPU_ENTRIES], &mut wire);
        wire
    };
    for (view, text) in [(CpuView::InView, "in_view"), (CpuView::Magnifier, "magnifier")] {
        assert!(wire(view).lines().any(|line| line == format!("wizard.cpu={text}")), "{text}");
    }
    assert!(!wire(CpuView::NotHeld).contains("wizard.cpu"));
}
}
