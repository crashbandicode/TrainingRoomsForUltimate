//! SPEC-state-06 (L+R+A Reset, reset hub) and SPEC-state-07 (Stage Hazards). Test names are the
//! specs' §9 rows. `state07_scan_matches_pinned` re-runs the instruction scans on the owned 13.0.5
//! `.text`.

use crate::common;

use crate::model;

use model::lra_reset::{self, FrameCounter, FrameCounterType, HubStep};
use model::sites;
use model::stage_hazards::{self as hz, Missing, Plan};

fn words_of(guard: &[u8; 64]) -> Vec<u32> {
    guard.as_chunks::<4>().0.iter().map(|w| u32::from_le_bytes(*w)).collect()
}

#[test]
fn state06_lra_register_rule() {
    // Off in native Training: x8 is cleared so the game's check fails. On: untouched.
    assert_eq!(lra_reset::x8_write(false, true, false), Some(0));
    assert_eq!(lra_reset::x8_write(true, true, false), None);
    // Outside native Training the register is never touched (the fork's early return).
    assert_eq!(lra_reset::x8_write(false, false, false), None);
    assert_eq!(lra_reset::x8_write(true, false, false), None);
    // The setting decodes from the file: [1,0] On, [0,1] Off.
    let mut s = model::settings::Snapshot::factory();
    assert!(s.lra_reset());
    s.set("lra_reset", &[0, 1]).unwrap();
    assert!(!s.lra_reset());
    assert_eq!(lra_reset::x8_write(s.lra_reset(), true, false), Some(0));
    // The inverted upstream gate is not adopted: the hub never runs outside Training.
    assert!(!lra_reset::hub_should_run(true, false, false));
}

#[test]
fn crash0900_no_native_reset_while_a_practice_fighter_loads() {
    // Hardware 2026-11-17 (reship-3): L+R+A during a Player Character pick's load; Eden
    // `crash0900-*-reship34`/`b-card4`: P1's loader stayed at 13 and the install never finished.
    // Mid-load the game's reset check fails even with the setting On ...
    assert_eq!(lra_reset::x8_write(true, true, true), Some(0));
    assert_eq!(lra_reset::x8_write(false, true, true), Some(0));
    // ... and outside native Training nothing changes.
    assert_eq!(lra_reset::x8_write(true, false, true), None);
}

fn facts(x8: u64, loading: bool, now_ms: u64) -> lra_reset::Facts {
    lra_reset::Facts { x8, lra_reset_on: true, native_training: true, loading, now_ms, ..Default::default() }
}

#[test]
fn crash0900_a_reset_pressed_mid_load_fires_once_after_the_load_settles() {
    use lra_reset::{defer_step, RESET_REQUESTED};
    let mut pending = None;
    // The press mid-load: no reset now (x8 cleared), one reset waits, its notice is asked for.
    let step = defer_step(&mut pending, facts(RESET_REQUESTED, true, 1_000));
    assert_eq!((step.write, step.post_notice, step.fired), (Some(0), true, false));
    assert!(pending.is_some());
    pending.as_mut().unwrap().notified = true;
    // Still loading, nothing asked: nothing happens and the notice is not asked for again.
    let step = defer_step(&mut pending, facts(0, true, 1_500));
    assert_eq!((step.write, step.post_notice, step.fired), (None, false, false));
    // A second press while one waits changes nothing (still one reset).
    let step = defer_step(&mut pending, facts(RESET_REQUESTED, true, 2_000));
    assert_eq!((step.write, step.post_notice, step.fired), (Some(0), false, false));
    // The first check after the load settles makes the game's own check reset, exactly once.
    let step = defer_step(&mut pending, facts(0, false, 3_000));
    assert_eq!((step.write, step.fired), (Some(RESET_REQUESTED), true));
    assert!(pending.is_none());
    let step = defer_step(&mut pending, facts(0, false, 3_016));
    assert_eq!((step.write, step.fired), (None, false));
    // A notice the mailbox dropped is asked for again on the next check.
    let mut pending = None;
    defer_step(&mut pending, facts(RESET_REQUESTED, true, 0));
    assert!(defer_step(&mut pending, facts(0, true, 16)).post_notice);
    // No load: a press resets at once, as before; L+R+A Reset Off stays silent.
    let mut none = None;
    assert_eq!(defer_step(&mut none, facts(RESET_REQUESTED, false, 0)), lra_reset::Step::default());
    let off = lra_reset::Facts { lra_reset_on: false, ..facts(RESET_REQUESTED, true, 0) };
    let step = defer_step(&mut none, off);
    assert_eq!((step.write, step.post_notice, none), (Some(0), false, None));
}

#[test]
fn crash0900_each_cancel_drops_the_reset_without_resetting() {
    use lra_reset::{defer_step, Cancel, Facts, RESET_REQUESTED, DEFER_TIMEOUT_MS};
    let base = facts(0, true, 1_000);
    let cases: [(Facts, Cancel); 6] = [
        (Facts { native_training: false, ..base }, Cancel::LeftTraining),
        (Facts { lra_reset_on: false, ..base }, Cancel::SettingOff),
        (Facts { menu_open: true, ..base }, Cancel::MenuOpened),
        (Facts { save_state_loading: true, ..base }, Cancel::SaveStateLoad),
        (Facts { install_aborts: 1, ..base }, Cancel::LoadAborted),
        (Facts { now_ms: DEFER_TIMEOUT_MS, ..base }, Cancel::TimedOut),
    ];
    for (f, cause) in cases {
        let mut pending = None;
        defer_step(&mut pending, facts(RESET_REQUESTED, true, 0));
        let step = defer_step(&mut pending, f);
        assert_eq!((step.cancelled, step.fired, pending), (Some(cause), false, None), "{cause:?}");
        assert_ne!(step.write, Some(RESET_REQUESTED), "{cause:?}");
        // The load settling afterwards resets nothing.
        assert!(!defer_step(&mut pending, facts(0, false, 20_000)).fired, "{cause:?}");
    }
    // Just inside the bound it still waits, and fires on settle.
    let mut pending = None;
    defer_step(&mut pending, facts(RESET_REQUESTED, true, 0));
    assert_eq!(defer_step(&mut pending, facts(0, true, DEFER_TIMEOUT_MS - 1)).cancelled, None);
    assert!(defer_step(&mut pending, facts(0, false, DEFER_TIMEOUT_MS - 1)).fired);
    // The success reasons the roster releases INSTALL with; anything else is an aborted load.
    for ok in ["Applied", "roster ready", "CPU Mii group record publication"] {
        assert!(lra_reset::install_ended_ok(ok), "{ok}");
    }
    // Review R3: a departed load completing after its scene ended is not the asked-for fighter applied.
    for aborted in ["departed roster completed", "Destroyed", "RecoveredKnownMember", "NativeSettledUnowned", "roster destroyed", "preflight refused"] {
        assert!(!lra_reset::install_ended_ok(aborted), "{aborted}");
    }
}

fn at(x8: u64, loading: bool, now_ms: u64, generation: u32) -> lra_reset::Facts {
    lra_reset::Facts { generation, ..facts(x8, loading, now_ms) }
}

#[test]
fn crash0900_r3_a_lifecycle_exit_drops_a_waiting_reset() {
    use lra_reset::{defer_step, Cancel, RESET_REQUESTED};
    // Departure between checks: armed in scene A (generation 7); the scene departs with no reset check in between
    // and the departed load completes successfully (abort count unchanged). The first check in scene B, settled,
    // Training, On, menu closed, before the deadline, resets nothing.
    let mut pending = None;
    defer_step(&mut pending, at(RESET_REQUESTED, true, 0, 7));
    let step = defer_step(&mut pending, at(0, false, 1_200, 8));
    assert_eq!((step.cancelled, step.fired, pending), (Some(Cancel::LifecycleEnded), false, None));
    assert_eq!(step.write, None, "the game's own value stays (no forced reset)");
    // Same-mode departure (Training -> CSS -> Training keeps the mode key; the P1 entry's destruction raises the
    // generation): every other fact is identical to a valid fire, only the generation differs.
    let mut pending = None;
    defer_step(&mut pending, at(RESET_REQUESTED, true, 0, 3));
    assert!(!defer_step(&mut pending, at(0, false, 500, 4)).fired);
    // Cancel exactly at settlement: the exit and the load settling are seen by the same check; the cancel wins.
    for f in [
        at(0, false, 900, 1),
        lra_reset::Facts { menu_open: true, ..at(0, false, 900, 0) },
        lra_reset::Facts { save_state_loading: true, ..at(0, false, 900, 0) },
        lra_reset::Facts { install_aborts: 1, ..at(0, false, 900, 0) },
    ] {
        let mut pending = None;
        defer_step(&mut pending, at(RESET_REQUESTED, true, 0, 0));
        let step = defer_step(&mut pending, f);
        assert!(!step.fired && step.cancelled.is_some() && pending.is_none(), "{f:?}");
        assert_ne!(step.write, Some(RESET_REQUESTED), "{f:?}");
    }
    // Re-entry with a stale pending reset: B's own press with no load resets once, natively (the hook leaves x8),
    // and no stale reset follows; a press in B mid-load arms B's own reset with B's generation.
    let mut pending = None;
    defer_step(&mut pending, at(RESET_REQUESTED, true, 0, 10));
    assert_eq!(defer_step(&mut pending, at(RESET_REQUESTED, false, 300, 11)).write, None);
    assert!(pending.is_none());
    assert!(!defer_step(&mut pending, at(0, false, 316, 11)).fired);
    let step = defer_step(&mut pending, at(RESET_REQUESTED, true, 400, 11));
    assert_eq!((step.write, pending.map(|p| p.generation)), (Some(0), Some(11)));
    assert!(defer_step(&mut pending, at(0, false, 900, 11)).fired);
    // The runtime counter: a lifecycle exit raises it (other tests may raise it too, never lower it).
    let generation = || lra_reset::GENERATION.load(std::sync::atomic::Ordering::Acquire);
    let before = generation();
    lra_reset::lifecycle_exit();
    assert_ne!(generation(), before);
}

#[test]
fn crash0900_r3_every_lifecycle_exit_raises_the_generation() {
    // The wiring the model case relies on: the hook samples the generation, and the roster's scene change (every
    // mode-key change, leaving Training included) and the P1 entry's destruction (same-mode departure) raise it.
    let root = env!("CARGO_MANIFEST_DIR");
    let hook = std::fs::read_to_string(format!("{root}/src/runtime/features/lra_reset.rs")).unwrap();
    assert!(hook.contains("generation: lra_reset::GENERATION.load(Ordering::Acquire),"));
    assert!(hook.contains("PENDING_GENERATION.store(p.generation, Ordering::Release);"));
    let roster = std::fs::read_to_string(format!("{root}/src/runtime/roster/character_roster.rs")).unwrap();
    let scene = roster.split("pub fn scene_changed(mode: game_mode::Mode) {").nth(1).expect("scene_changed");
    assert!(scene[..200].contains("crate::model::lra_reset::lifecycle_exit();"), "scene change raises it first");
    let departure = roster.split("if practice_departure {").nth(1).expect("the P1 departure block");
    assert!(departure[..400].contains("crate::model::lra_reset::lifecycle_exit();"), "P1 entry destruction raises it");
}

#[test]
fn crash0900_r1_the_reset_gate_calls_no_native_entry_lookup() {
    // Review R1: the gate's loading signal is the roster's INSTALL token (an atomic), so the reset hook reaches no
    // native row: a rejected `fighter_manager_get_entry` is never called from it, and the L+R+A unit keeps no
    // dependency that a rejected row would uninstall (`tests/capabilities.rs`: `CROSS_NEEDS` has no `lra_reset`
    // entry; `a_poisoned_row_disables_every_entry_point_that_can_reach_it_and_no_installed_one_calls_it`).
    let root = env!("CARGO_MANIFEST_DIR");
    let hook = std::fs::read_to_string(format!("{root}/src/runtime/features/lra_reset.rs")).unwrap();
    for native in ["native_source", "get_fighter_entry", "fighter_entry(", "FighterManager", "natives::", "native_fields"] {
        assert!(!hook.contains(native), "the reset hook must not reach `{native}`");
    }
    assert!(hook.contains("loading: crate::runtime::roster::character_roster::busy(),"));
    let busy = std::fs::read_to_string(format!("{root}/src/runtime/roster/character_roster.rs")).unwrap();
    assert!(busy.contains("pub fn busy() -> bool {\n    INSTALL.suspends_practice()\n}"), "busy() is the install token alone");
}

#[test]
fn crash0900_r2_the_notice_is_drawn_with_combo_display_off() {
    use lra_reset::{defer_step, RESET_REQUESTED};
    use model::notify::{Body, Queue, DEFAULT_COLOR};
    // The runtime's path: the press mid-load asks for the notice, which is posted required; the HUD tick runs with
    // the game's Combo Display Off (`tick(false, ..)`), which clears every other notice.
    let mut queue = Queue::new();
    queue.post_static("Other", "a plain notice", 60, DEFAULT_COLOR);
    let mut pending = None;
    assert!(defer_step(&mut pending, facts(RESET_REQUESTED, true, 0)).post_notice);
    queue.post_required(lra_reset::DEFERRED_HEADER, Body::copied(lra_reset::DEFERRED_BODY), 120, DEFAULT_COLOR);
    let out = queue.tick(false, false);
    assert!(out.visible);
    let draw = out.draw.expect("the notice is drawn");
    assert_eq!((draw.header, draw.body.as_str()), ("L+R+A Reset", "Resetting when the fighter finishes loading"));
    // Other notices keep their rule: Combo Display Off clears a plain one.
    let mut plain = Queue::new();
    plain.post_static("Other", "a plain notice", 60, DEFAULT_COLOR);
    assert!(!plain.tick(false, false).visible);
}

#[test]
fn state06_hub_trigger_rules() {
    assert!(lra_reset::hub_should_run(true, true, false));
    assert!(!lra_reset::hub_should_run(false, true, false), "only the CPU's init triggers it");
    assert!(!lra_reset::hub_should_run(true, false, false), "native Training only");
    assert!(!lra_reset::hub_should_run(true, true, true), "not during a hot swap");
    // The reset-check hook and the on-init wrapper sit on sites that this build guards.
    assert!(sites::find("reset_check").is_some());
    assert!(sites::find("on_init_slot").is_some());
    assert!(sites::find("on_init").is_some());
}

fn owed_reset_block(source: &str) -> &str {
    source
        .split_once("RESET_OWED.take(")
        .expect("on-init must consume an owed reset")
        .1
        .split_once("\n    }\n    result")
        .expect("the owed-reset block ends before the return")
        .0
}

#[test]
fn reset_hub_runs_the_game_resets_only_for_an_owed_reset() {
    let hub = include_str!("../src/runtime/features/reset_hub.rs");
    let block = owed_reset_block(hub);
    for call in [
        "btt_camera::interrupt_placement()",
        "hub::run(Clear::TrainingReset)",
        "hitbox::eggs_reset()",
        "mashout::reset()",
        "btt::reset()",
        "tech::reset()",
        "parry::reset()",
        "edge_juggle::restart()",
        "egg_zone::reset()",
    ] {
        assert_eq!(hub.matches(call).count(), 1, "{call}");
        assert!(block.contains(call), "{call} must be inside the owed-reset block");
    }
    let reset = include_str!("../src/runtime/features/lra_reset.rs");
    assert_eq!(reset.matches("RESET_OWED.owe()").count(), 1);
    let guarded = reset.split_once("if lra_reset::reset_passes(").expect("reset-pass gate").1;
    assert!(guarded.split_once('}').unwrap().0.contains("RESET_OWED.owe()"));
    let stage = include_str!("../src/runtime/lobby/practice_stage_request.rs");
    assert_eq!(stage.matches("RESET_OWED.stage_published()").count(), 1);
    assert_eq!(stage.matches("RESET_OWED.stage_completed()").count(), 1);
    assert_eq!(stage.matches("RESET_OWED.stage_retained()").count(), 2);
    assert!(!stage.contains("RESET_OWED.owe()"));
    let queue = stage.split_once("fn queue(").unwrap().1.split_once("\n}").unwrap().0;
    assert!(!queue.contains("RESET_OWED"));
    let publish = stage.split_once("publish(commit,descriptor);").unwrap().1;
    assert!(publish.trim_start().starts_with("crate::model::lra_reset::RESET_OWED.stage_published();"));
    let retire = stage.split_once("fn retire(&mut self)").unwrap().1.split_once("\n    }").unwrap().0;
    assert!(retire.contains("self.live.is_some()") && retire.contains("RESET_OWED.stage_retained()"));
    let phase = stage.split_once("fn phase(&mut self, phase: Phase)").unwrap().1.split_once("\n    }").unwrap().0;
    assert!(phase.contains("self.live.is_some() && matches!(phase, Phase::Retained(_))"));
    let complete = stage.split_once("if policy::complete(v,c) => {").unwrap().1.split_once("\n            }").unwrap().0;
    assert!(complete.find("RESET_OWED.stage_completed()").unwrap() < complete.find("state.live=None").unwrap());

    fn check_setters(path: &std::path::Path, setters: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                check_setters(&path, setters);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let source = std::fs::read_to_string(&path).unwrap();
                for _ in source.match_indices("RESET_OWED.owe()") {
                    setters.push(path.clone());
                }
            }
        }
    }
    let mut setters = Vec::new();
    check_setters(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut setters);
    assert_eq!(setters.len(), 1);
    assert!(setters[0].ends_with("src/runtime/features/lra_reset.rs"));
}

#[test]
fn mashout_attempt_survives_a_cpu_transform_init() {
    let hub = include_str!("../src/runtime/features/reset_hub.rs");
    assert!(owed_reset_block(hub).contains("mashout::reset()"));
    let mashout = include_str!("../src/runtime/features/mashout.rs");
    let reset = mashout.split_once("pub fn reset()").unwrap().1.split_once("\n}").unwrap().0;
    assert!(reset.contains("clatter_set::release()") && reset.contains("start()"));
    assert!(mashout.contains("game.p1 != p1 || game.cpu != cpu"));
    assert!(mashout.contains("game.fail(\"Fighter changed · no grade\")"));
}

fn settled_owed() -> lra_reset::ResetOwed {
    let owed = lra_reset::ResetOwed::new();
    assert!(owed.take(true, true, false, 7));
    owed
}

#[test]
fn state06_one_owed_reset_runs_the_hub_once_for_several_cpus() {
    use std::sync::{Arc, Barrier};
    let concurrent = Arc::new(lra_reset::ResetOwed::new());
    assert!(concurrent.take(true, true, false, 1));
    concurrent.owe();
    let barrier = Arc::new(Barrier::new(8));
    let threads: Vec<_> = (0..8).map(|_| {
        let owed = Arc::clone(&concurrent);
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            barrier.wait();
            owed.take(true, true, false, 1)
        })
    }).collect();
    assert_eq!(threads.into_iter().map(|thread| thread.join().unwrap()).filter(|won| *won).count(), 1);

    let owed = lra_reset::ResetOwed::new();
    assert!(owed.take(true, true, false, 1));
    assert!(!owed.take(true, true, false, 2));
    assert!(!owed.take(true, true, false, 3));
    owed.owe();
    assert!(owed.take(true, true, false, 1));
    assert!(!owed.take(true, true, false, 2));
    assert!(!owed.take(true, true, false, 3));
}

#[test]
fn state06_a_stage_request_alone_owes_nothing() {
    let stage = include_str!("../src/runtime/lobby/practice_stage_request.rs");
    assert!(!stage.contains("RESET_OWED"));
}

#[test]
fn state06_hub_needs_an_owed_reset() {
    let owed = lra_reset::ResetOwed::new();
    assert!(!owed.take(false, true, false, 7));
    assert!(owed.take(true, true, false, 7));
    assert!(!owed.take(true, true, false, 7));
    assert!(lra_reset::hub_should_run(true, true, false));
    assert!(!owed.take(true, true, false, 7));
    owed.owe();
    assert!(!owed.take(true, true, true, 7));
    assert!(owed.take(true, true, false, 7));
    assert!(!owed.take(true, true, false, 7));
    assert!(owed.take(true, true, false, 8));
    assert!(!owed.take(true, true, false, 8));
    assert!(lra_reset::reset_passes(2, true));
    assert!(!lra_reset::reset_passes(2, false));
    assert!(!lra_reset::reset_passes(0, true));
}

#[test]
fn state06_unrelated_inits_do_not_spend_an_owed_reset() {
    let owed = settled_owed();
    owed.owe();
    assert!(!owed.take(false, true, false, 7));
    assert!(!owed.take(true, false, false, 7));
    assert!(!owed.take(true, true, true, 7));
    assert!(owed.take(true, true, false, 7));
    assert!(!owed.take(true, true, false, 7));
}

#[test]
fn state06_a_reset_owed_outside_native_training_is_absorbed_by_the_next_scene() {
    let owed = settled_owed();
    owed.owe();
    assert!(!owed.take(true, false, false, 7));
    assert!(owed.take(true, true, false, 8));
    assert!(!owed.take(true, true, false, 8));
}

#[test]
fn state06_owed_follows_the_reset_check_through_a_deferral() {
    let owed = settled_owed();
    let mut pending = None;
    let press = at(lra_reset::RESET_REQUESTED, true, 0, 7);
    let step = lra_reset::defer_step(&mut pending, press);
    assert_eq!(step.write, Some(0));
    assert!(!lra_reset::reset_passes(step.write.unwrap_or(press.x8), press.native_training));
    assert!(!owed.take(true, true, false, 7));
    let cancel = lra_reset::Facts { menu_open: true, ..at(0, false, 10, 7) };
    let step = lra_reset::defer_step(&mut pending, cancel);
    assert_eq!(step.cancelled, Some(lra_reset::Cancel::MenuOpened));
    assert!(!lra_reset::reset_passes(step.write.unwrap_or(cancel.x8), cancel.native_training));
    assert!(!owed.take(true, true, false, 7));

    lra_reset::defer_step(&mut pending, press);
    let settled = at(0, false, 10, 7);
    let step = lra_reset::defer_step(&mut pending, settled);
    assert!(step.fired);
    assert_eq!(step.write, Some(2));
    if lra_reset::reset_passes(step.write.unwrap_or(settled.x8), settled.native_training) {
        owed.owe();
    }
    assert!(owed.take(true, true, false, 7));
    assert!(!owed.take(true, true, false, 7));

    let off = lra_reset::Facts { lra_reset_on: false, ..press };
    let step = lra_reset::defer_step(&mut pending, off);
    assert_eq!(step.write, Some(0));
    assert!(!lra_reset::reset_passes(step.write.unwrap_or(off.x8), off.native_training));
    assert!(!owed.take(true, true, false, 7));
}

#[test]
fn state06_change_stage_owes_one_reset_only_when_it_runs() {
    let owed = settled_owed();
    assert!(!owed.take(true, true, false, 7));
    owed.stage_published();
    assert!(owed.take(true, true, false, 7));
    assert!(!owed.take(true, true, false, 7));
    owed.stage_completed();
    assert!(!owed.take(true, true, false, 7));
    owed.stage_published();
    owed.stage_completed();
    assert!(owed.take(true, true, false, 7));
    assert!(!owed.take(true, true, false, 7));
    owed.stage_published();
    owed.stage_retained();
    assert!(!owed.take(true, true, false, 7));
    assert!(!owed.take(true, true, false, 7));
    owed.stage_published();
    assert!(!owed.take(false, true, false, 7));
    assert!(!owed.take(true, true, true, 7));
    owed.stage_completed();
    assert!(owed.take(true, true, false, 7));
    assert!(!owed.take(true, true, false, 7));
}

#[test]
fn lra_and_change_stage_still_run_the_hub_once() {
    let owed = settled_owed();
    if lra_reset::reset_passes(lra_reset::RESET_REQUESTED, true) {
        owed.owe();
    }
    assert!(owed.take(true, true, false, 7));
    assert!(!owed.take(true, true, false, 7));
    owed.stage_published();
    assert!(owed.take(true, true, false, 7));
    owed.stage_completed();
    assert!(!owed.take(true, true, false, 7));
}

#[test]
fn eggs_run_and_streak_survive_a_cpu_transform_init() {
    use model::eggs::{self, Run, View};
    use model::hit_streak::{Motion, Streak};
    let owed = settled_owed();
    let mut run = Run::IDLE;
    run.start([eggs::TIMED, eggs::ANY, eggs::FAST, eggs::CPU, eggs::IN_VIEW], 7);
    let view = View([14.0; 2], [0.0, -300.0], [-100.0, 100.0], None);
    for _ in 0..30 { run.look(true, || Some(view)); }
    run.step(true, 0.0, &[], false, 0);
    run.cpu_pass(0.0, false);
    run.cpu_pass(0.0, false);
    run.cpu_pass(5.0, false);
    run.cpu_pass(5.0, false);
    run.step(true, 0.0, &[], false, 0);
    assert_eq!(run.score, 1);
    for _ in 0..120 { run.step(true, 0.0, &[], false, 0); }
    let mut streak = Streak::EMPTY;
    streak.restart(true);
    streak.begin(1, Motion { status: 1, kind: 1, frame: 1.0 }, true);
    streak.hit(1, 1);
    assert_eq!(streak.counts(), (1, 1));
    let mut before = String::new();
    run.write_notice(&mut before).unwrap();
    assert!(!before.ends_with("1:00"));
    if owed.take(true, true, false, 7) {
        run.reset(11);
        streak.restart(true);
    }
    let mut after = String::new();
    run.write_notice(&mut after).unwrap();
    assert_eq!(after, before);
    assert_eq!((run.score, streak.counts()), (1, (1, 1)));
    owed.owe();
    if owed.take(true, true, false, 7) {
        run.reset(11);
        streak.restart(true);
    }
    assert_eq!((run.score, streak.counts(), run.view), (0, (0, 1), None));
    let mut restarted = String::new();
    run.write_notice(&mut restarted).unwrap();
    assert_eq!(restarted, "Eggs 0   1:00");
}

#[test]
fn btt_running_run_survives_a_cpu_transform_init() {
    use model::btt::{Btt, Hot, Pad, Phase, MAX};
    use model::storage::FixedPath;
    let owed = settled_owed();
    let mut btt = Btt::IDLE;
    btt.enter(10);
    btt.close_menu(true, true);
    let mut positions = [[0.0; 2]; MAX];
    positions[1] = [20.0, 0.0];
    btt.apply(Hot { id: FixedPath::new(), name: FixedPath::new(), n: 2, at: positions });
    btt.close_menu(true, false);
    btt.feed(Pad { pressed: 1, ..Pad::default() });
    assert_eq!(btt.phase(), Phase::Running);
    assert_eq!(btt.pass_indexed(true, |i, _| i == 0), 1);
    for _ in 0..30 { btt.pass_indexed(true, |_, _| false); }
    let mut before = String::new();
    btt.write_notice(&mut before).unwrap();
    if owed.take(true, true, false, 7) { btt.reset(); }
    let mut after = String::new();
    btt.write_notice(&mut after).unwrap();
    assert_eq!((btt.phase(), after), (Phase::Running, before));
    assert_eq!(btt.pass_indexed(false, |i, _| i == 0), 0);
    owed.owe();
    if owed.take(true, true, false, 7) { btt.reset(); }
    assert_eq!(btt.phase(), Phase::Ready);
    btt.feed(Pad { pressed: 1, ..Pad::default() });
    assert_eq!(btt.pass_indexed(false, |i, _| i == 0), 1);
}

#[test]
fn tech_attempt_and_wizard_survive_a_cpu_transform_init() {
    use model::tech::controller::Controller;
    use model::tech::session::{Grade, Length, Phase};
    let owed = settled_owed();
    let mut controller = Controller::IDLE;
    controller.start(Length::Attempts(10));
    controller.session.summary.record(Grade::Perfect(Some(1)));
    controller.session.elapsed = 42;
    controller.session.phase = Phase::Flight;
    if owed.take(true, true, false, 7) {
        controller.start(controller.session.length);
    }
    assert_eq!((controller.session.phase, controller.session.summary.attempts, controller.session.summary.streak, controller.session.elapsed),
        (Phase::Flight, 1, 1, 42));
    owed.owe();
    if owed.take(true, true, false, 7) {
        controller.start(controller.session.length);
    }
    assert_eq!((controller.session.phase, controller.session.length, controller.session.summary.attempts, controller.session.summary.streak, controller.session.elapsed),
        (Phase::Place, Length::Attempts(10), 0, 0, 0));
}

#[test]
fn parry_reset_does_not_run_on_a_cpu_transform_init() {
    use model::parry::{Detector, Frame};
    let owed = settled_owed();
    let mut detector = Detector::new();
    let held = Frame { shield_held: true, armed: true, ..Frame::default() };
    detector.step(&held, 5);
    detector.step(&Frame { stop: true, guard_damage: true, just_shield: Some(false), ..held }, 6);
    assert!(detector.busy());
    let before = detector;
    // This proves only that parry::reset() does not run on a transform init. An open Parry attempt is still dropped
    // through the frame-strip clear (reset_hub.rs, input_display::fighter_initialised()), which runs on every fighter
    // init. Today's behaviour stays (primary ruling 2026-11-22 option (a)); gating that clear is queued for the operator.
    if owed.take(true, true, false, 7) { detector = Detector::new(); }
    assert_eq!(detector, before);
    owed.owe();
    if owed.take(true, true, false, 7) { detector = Detector::new(); }
    assert!(!detector.busy());
}

#[test]
fn recording_survives_a_cpu_transform_init() {
    use model::recording::{Mode, Recorder};
    let owed = settled_owed();
    let mut recorder = Recorder::new();
    recorder.start_named_playback(20, 1, 0, 1).unwrap();
    assert_eq!(recorder.mode(), Mode::Playback);
    if owed.take(true, true, false, 7) { recorder.stop(); }
    assert_eq!(recorder.mode(), Mode::Playback);
    owed.owe();
    if owed.take(true, true, false, 7) { recorder.stop(); }
    assert_eq!(recorder.mode(), Mode::None);
    let hub = include_str!("../src/runtime/hub.rs");
    assert!(hub.contains("recorder::on_reset(cause)"));
    let runtime = include_str!("../src/runtime/features/recorder.rs");
    let reset = runtime.split_once("pub fn on_reset(").unwrap().1.split_once("\n}").unwrap().0;
    assert!(reset.contains("clear_playback(cause)") && reset.contains("stop()"));
}

#[test]
fn state06_hub_order() {
    assert_eq!(
        lra_reset::HUB_ORDER[..7],
        [HubStep::Mash, HubStep::Sdi, HubStep::FrameCounters, HubStep::Combo, HubStep::Ledge, HubStep::Throw, HubStep::ShieldTilt]
    );
    // This plugin's own step (decision B8) comes after the spec's seven.
    assert_eq!(lra_reset::HUB_ORDER[7], HubStep::InputDelay);
    let mut seen = Vec::new();
    lra_reset::run_hub(|step| seen.push(step));
    assert_eq!(seen, lra_reset::HUB_ORDER);
}

#[test]
fn state06_counter_types() {
    let mut counters = [
        FrameCounter { kind: FrameCounterType::InGame, frames: 40, counting: true },
        FrameCounter { kind: FrameCounterType::InGameNoReset, frames: 41, counting: true },
        FrameCounter { kind: FrameCounterType::Real, frames: 42, counting: true },
    ];
    lra_reset::reset_in_game_counters(&mut counters);
    assert_eq!(counters[0], FrameCounter { kind: FrameCounterType::InGame, frames: 0, counting: false });
    assert_eq!(counters[1].frames, 41);
    assert!(counters[1].counting);
    assert_eq!(counters[2].frames, 42);
    assert!(counters[2].counting);
}

#[test]
fn state06_hub_idempotent() {
    // Running the hub twice in a frame (CPU-as-P1, two CPUs) leaves the same state as once.
    #[derive(Default, PartialEq, Debug, Clone)]
    struct State {
        mash_queue: Vec<u8>,
        counters: Vec<FrameCounter>,
        delay_queue_len: usize,
    }
    let start = State {
        mash_queue: vec![1, 2],
        counters: vec![FrameCounter { kind: FrameCounterType::InGame, frames: 9, counting: true }],
        delay_queue_len: 3,
    };
    let hub = |s: &mut State| {
        lra_reset::run_hub(|step| match step {
            HubStep::Mash => s.mash_queue.clear(),
            HubStep::FrameCounters => lra_reset::reset_in_game_counters(&mut s.counters),
            HubStep::InputDelay => s.delay_queue_len = 0,
            _ => {}
        })
    };
    let mut once = start.clone();
    hub(&mut once);
    let mut twice = start;
    hub(&mut twice);
    hub(&mut twice);
    assert_eq!(once, twice);
}

#[test]
fn state07_flag_address_arithmetic() {
    // The two words at the scan site in 13.0.5 give the pinned byte.
    let site = sites::find("hazard_flag").unwrap();
    assert_eq!(site.offset, hz::FLAG_SCAN_OFFSET);
    let w = words_of(&site.guard);
    assert_eq!(hz::flag_offset(site.offset, w[0], w[1]), Some(hz::FLAG_OFFSET));
    assert_eq!(hz::FLAG_OFFSET, 0x530a985);
    // The pieces: adrp's page offset and add's immediate.
    assert!(hz::is_adrp(w[0]) && hz::is_add_imm(w[1]));
    assert_eq!(hz::adrp_offset(w[0]) + 0x1740000, 0x530a985 - 9 - hz::add_imm_value(w[1]) as i64);
    // A shifted add, a negative adrp, and non-instructions.
    assert_eq!(hz::add_imm_value(0x9140_0400), 0x1000);
    assert_eq!(hz::add_imm_value(0x9100_0400), 1);
    assert_eq!(hz::adrp_offset(0xf000_0000 | (0x7_ffff << 5)), -4096);
    assert_eq!(hz::flag_offset(0x1000, 0x1400_0000, w[1]), None);
    assert_eq!(hz::flag_offset(0x1000, w[0], 0x1400_0000), None);
}

#[test]
fn state07_scan_matches_pinned() {
    // The pinned windows hold exactly the instruction runs the scans look for.
    let flag = sites::find("hazard_flag").unwrap();
    assert_eq!(hz::find_flag_run(&words_of(&flag.guard)), Some(0));
    let window = sites::find("hazard_window").unwrap();
    let hook = sites::find("hazard_hook").unwrap();
    assert_eq!(hz::find_hook_run(&words_of(&window.guard)), Some(0));
    assert_eq!(window.offset + 8, hook.offset);
    assert_eq!(hook.offset, hz::HOOK_SITE_OFFSET);
    // On the owned `.text` the first match of each scan is the pinned one.
    let Some(main) = common::owned_main("state07_scan_matches_pinned") else {
        return;
    };
    let words = main.text_words();
    let flag_run = hz::find_flag_run(&words).expect("flag run");
    assert_eq!(flag_run * 4, hz::FLAG_SCAN_OFFSET);
    assert_eq!(hz::flag_offset(flag_run * 4, words[flag_run], words[flag_run + 1]), Some(hz::FLAG_OFFSET));
    let hook_run = hz::find_hook_run(&words).expect("hook run");
    assert_eq!((hook_run + 2) * 4, hz::HOOK_SITE_OFFSET);
}

#[test]
fn state07_write_rule() {
    assert_eq!(hz::rule_byte(true, true), Some(1));
    assert_eq!(hz::rule_byte(false, true), Some(0));
    assert_eq!(hz::rule_byte(true, false), None);
    assert_eq!(hz::rule_byte(false, false), None);
    let mut s = model::settings::Snapshot::factory();
    assert_eq!(hz::rule_byte(s.stage_hazards(), true), Some(0), "default Off writes 0 in Training");
    s.set("stage_hazards", &[1, 0]).unwrap();
    assert_eq!(hz::rule_byte(s.stage_hazards(), true), Some(1));
}

/// rc.12 (operator 2026-11-13, "yes fix it in rc12"): Change Stage in Training takes the Stage Hazards setting. The
/// request copies the active stage's descriptor, whose hazards byte was fixed at Training entry, and the hook's rule
/// byte is not read on a Change Stage; so Off at entry and On before Change Stage gave no hazards.
#[test]
fn state07_change_stage_takes_the_setting() {
    const TRAINING: (u32, u8) = (0x04040000, 0x20);
    for on in [false, true] {
        assert_eq!(hz::change_stage_byte(true, TRAINING.0, TRAINING.1, None, on), Some(u8::from(on)), "Training, {on}");
        assert_eq!(hz::change_stage_byte(false, TRAINING.0, TRAINING.1, None, on), None, "not initialized");
        // Waiting rooms keep the carried rule's byte (or none).
        assert_eq!(hz::change_stage_byte(true, 0x02010000, 0x0d, Some(u8::from(on)), !on), Some(u8::from(on)));
        assert_eq!(hz::change_stage_byte(true, 0x02010000, 0x0d, None, on), None);
        // A match keeps its own.
        assert_eq!(hz::change_stage_byte(true, 0x02010000, 0x01, None, on), None);
    }
}

#[test]
fn state07_missing_address_disables() {
    assert_eq!(hz::plan(Some(1), true), Plan::Enabled);
    assert_eq!(hz::plan(None, true), Plan::Disabled(Missing::Flag));
    assert_eq!(hz::plan(Some(1), false), Plan::Disabled(Missing::Hook));
    assert_eq!(hz::plan(None, false), Plan::Disabled(Missing::Both));
    // Each condition has its own text (upstream's were swapped).
    assert_ne!(Missing::Flag.text(), Missing::Hook.text());
    assert!(Missing::Flag.text().contains("rule byte"));
    assert!(Missing::Hook.text().contains("hook site"));
}

#[test]
fn state06_hub_trigger_facts_match_the_guarded_words() {
    // The on_init function's first words (guarded by the `on_init` row) hold the two facts the
    // trigger relies on: the entry id is loaded from fighter+0x10, and the FighterManager slot is
    // read through an adrp/ldr pair that lands on the pinned slot.
    let on_init = sites::find("on_init").unwrap();
    let w = words_of(&on_init.guard);
    // ldp w9, w8, [x0, #0xc]
    assert_eq!(w[8], 0x2941_a009);
    assert_eq!(lra_reset::FIGHTER_ENTRY_ID_FIELD, 0x10);
    assert_eq!(0xc + 4, lra_reset::FIGHTER_ENTRY_ID_FIELD);
    // adrp x24 / ldr x27, [x24, #0x4f8]
    assert_eq!(lra_reset::manager_slot_offset(on_init.offset + 9 * 4, w[9], w[10]), Some(lra_reset::FIGHTER_MANAGER_SLOT_OFFSET));
    assert_eq!(lra_reset::manager_slot_offset(0, 0x1400_0000, w[10]), None);
    assert_eq!(lra_reset::manager_slot_offset(0, w[9], 0), None);
    // The two called thunks are the ones the symbols name: `ldr x8, [x0, #8]; ldrb w0, [x8, #0x92]`.
    let cpu = sites::find("is_operation_cpu").unwrap();
    let c = words_of(&cpu.guard);
    assert_eq!((c[0], c[1], c[2]), (0xf940_0408, 0x3942_4900, 0xd65f_03c0));
    // get_fighter_information(manager, entry): `cmp w1, #8` first.
    let info = sites::find("fighter_information").unwrap();
    assert_eq!(words_of(&info.guard)[0], 0x7100_203f);
    assert_eq!(lra_reset::ENTRY_COUNT, 8);
    assert!(sites::feature_usable(sites::Feature::ResetHub, |_| true));
    assert!(!sites::feature_usable(sites::Feature::ResetHub, |s| s.name != "is_operation_cpu"));
}
