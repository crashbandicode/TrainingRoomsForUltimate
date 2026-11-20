lib_unit_tests! {
use super::*;

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
fn think_counts_are_per_entry_and_offline_admitted() {
    let counts = core::array::from_fn::<_, CPU_ENTRIES, _>(|_| core::sync::atomic::AtomicU64::new(0));
    for (admission, matched, entry) in [(0, true, 1), (2, false, 1), (2, true, 0), (2, true, -1),
        (2, true, 8), (2, true, 1), (2, true, 4), (2, true, 1)] {
        if let Some(index) = think_entry(admission, matched, entry) {
            counts[index].fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        }
    }
    assert_eq!(counts.map(core::sync::atomic::AtomicU64::into_inner), [0, 2, 0, 0, 1, 0, 0, 0]);
}
}

