//! SPEC-cpu-04 (CPU tech, missed-tech options, hidden tech animations) and the decisions it carries:
//! B5 / N4 (the recording wins by default), B6 (one state per fighter), B8 (Mistech Options no longer
//! depend on Tech Options). Test names are the spec's §9 rows.

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::cpu::facts::{FrameFacts, TechFacts};
use model::cpu::hash::hash40;
use model::cpu::ids::{Table, C};
use model::cpu::playback::PlaybackState;
use model::cpu::role::FighterKey;
use model::cpu::settings::CpuSettings;
use model::cpu::state::Cpu;
use model::cpu::tech::{
    camera_centre, laydown_lockout, quake_suppressed, MotionHashes, Roll, StageClass, TechChange, TechRequest,
    HIDDEN_CURSOR_OFFSET,
};
use model::settings::Snapshot;

// Synthetic status and term numbers (the real ones come from the game's table at run time).
const DOWN: i32 = 100;
const REFLECT_D: i32 = 101;
const PASSIVE: i32 = 102;
const PASSIVE_FB: i32 = 103;
const STOP_WALL: i32 = 104;
const REFLECT_LR: i32 = 105;
const PASSIVE_WALL: i32 = 106;
const PASSIVE_WALL_JUMP: i32 = 107;
const STOP_CEIL: i32 = 108;
const REFLECT_U: i32 = 109;
const PASSIVE_CEIL: i32 = 110;
const GANON_FALL: i32 = 111;
const DOWN_WAIT: i32 = 120;
const DOWN_WAIT_CONTINUE: i32 = 121;
const DOWN_STAND: i32 = 122;
const DOWN_STAND_ATTACK: i32 = 123;
const DOWN_STAND_FB: i32 = 124;
const LAY_DOWN: i32 = 125;
const SLIP_WAIT: i32 = 130;
const SLIP_STAND: i32 = 131;
const SLIP_STAND_ATTACK: i32 = 132;
const SLIP_STAND_F: i32 = 133;
const SLIP_STAND_B: i32 = 134;
const WAIT: i32 = 1;

fn table() -> Table {
    Table::with(&[
        (C::StatusDown, DOWN),
        (C::StatusDamageFlyReflectD, REFLECT_D),
        (C::StatusPassive, PASSIVE),
        (C::StatusPassiveFb, PASSIVE_FB),
        (C::StatusStopWall, STOP_WALL),
        (C::StatusDamageFlyReflectLr, REFLECT_LR),
        (C::StatusPassiveWall, PASSIVE_WALL),
        (C::StatusPassiveWallJump, PASSIVE_WALL_JUMP),
        (C::StatusStopCeil, STOP_CEIL),
        (C::StatusDamageFlyReflectU, REFLECT_U),
        (C::StatusPassiveCeil, PASSIVE_CEIL),
        (C::ClatterCatchedAirFallGanon, GANON_FALL),
        (C::StatusDownWait, DOWN_WAIT),
        (C::StatusDownWaitContinue, DOWN_WAIT_CONTINUE),
        (C::StatusDownStand, DOWN_STAND),
        (C::StatusDownStandAttack, DOWN_STAND_ATTACK),
        (C::StatusDownStandFb, DOWN_STAND_FB),
        (C::StatusLayDown, LAY_DOWN),
        (C::StatusSlipWait, SLIP_WAIT),
        (C::StatusSlipStand, SLIP_STAND),
        (C::StatusSlipStandAttack, SLIP_STAND_ATTACK),
        (C::StatusSlipStandF, SLIP_STAND_F),
        (C::StatusSlipStandB, SLIP_STAND_B),
        (C::StatusWait, WAIT),
    ])
}

fn settings(rows: &[(&str, Vec<u8>)]) -> CpuSettings {
    let mut snap = Snapshot::factory();
    for (id, row) in rows {
        snap.set(id, row).unwrap();
    }
    CpuSettings::from_snapshot(&snap)
}

/// A Tech Options row with only `option` selected (No Tech 0, Roll F 1, Roll B 2, In Place 3).
fn tech_only(option: usize) -> Vec<u8> {
    let mut row = vec![0u8; 4];
    row[option] = 1;
    row
}

fn actions(at: &[usize]) -> Vec<u8> {
    let mut row = vec![0u8; 30];
    for i in at {
        row[*i] = 1;
    }
    row
}

fn cpu_with(rows: &[(&str, Vec<u8>)]) -> (Cpu, FighterKey) {
    let mut cpu = Cpu::new(settings(rows), 21);
    cpu.ids = table();
    let key = FighterKey::new(1, false).unwrap();
    cpu.fighters[key.index()].pending = 0;
    cpu.fighters[key.index()].defence.stale_written = Some(false);
    (cpu, key)
}

fn request(requested: i32) -> TechRequest {
    TechRequest { requested, second_prev: WAIT, passive: true, passive_wall: true, passive_ceil: true, script_passive_enabled: true }
}

#[test]
fn cpu04_ground_tech_table() {
    for (option, want, roll) in [
        (3usize, Some(PASSIVE), None),
        (1, Some(PASSIVE_FB), Some(Roll::Forward)),
        (2, Some(PASSIVE_FB), Some(Roll::Backward)),
        (0, None, None),
    ] {
        for requested in [DOWN, REFLECT_D] {
            let (mut cpu, key) = cpu_with(&[("tech_state", tech_only(option))]);
            let change = cpu.tech_status_change(key, &request(requested));
            assert_eq!(change, want.map_or(TechChange::Unchanged, TechChange::Status), "option {option}, request {requested}");
            assert_eq!(cpu.fighters[key.index()].tech.tech_roll, roll);
        }
    }
    // No Tech leaves the request unchanged (a missed tech); so does an empty row.
    let (mut cpu, key) = cpu_with(&[("tech_state", vec![0; 4])]);
    assert_eq!(cpu.tech_status_change(key, &request(DOWN)), TechChange::Unchanged);
    // A tech needs the PASSIVE term and the script's own check.
    let (mut cpu, key) = cpu_with(&[("tech_state", tech_only(3))]);
    for r in [
        TechRequest { passive: false, ..request(DOWN) },
        TechRequest { script_passive_enabled: false, ..request(DOWN) },
    ] {
        assert_eq!(cpu.tech_status_change(key, &r), TechChange::Unchanged);
    }
    // Ganondorf's aerial side special slam (second-previous status) cannot be teched.
    let ganon = TechRequest { second_prev: GANON_FALL, ..request(DOWN) };
    assert_eq!(cpu.tech_status_change(key, &ganon), TechChange::Unchanged);
    // A request that is not a tech situation rolls nothing and changes nothing.
    let before = cpu.rng.clone();
    assert_eq!(cpu.tech_status_change(key, &request(WAIT)), TechChange::Unchanged);
    assert_eq!(cpu.rng, before, "no roll unless a tech situation applies");
}

#[test]
fn cpu04_wall_ceiling_tech() {
    // Wall: In Place wall-techs, Roll Forwards wall-jumps, Roll Backwards and No Tech do nothing.
    for (option, want) in [(3usize, Some(PASSIVE_WALL)), (1, Some(PASSIVE_WALL_JUMP)), (2, None), (0, None)] {
        for requested in [STOP_WALL, REFLECT_LR] {
            let (mut cpu, key) = cpu_with(&[("tech_state", tech_only(option))]);
            let got = cpu.tech_status_change(key, &request(requested));
            assert_eq!(got, want.map_or(TechChange::Unchanged, TechChange::Status), "wall, option {option}");
        }
    }
    // The wall term and the script check are needed.
    let (mut cpu, key) = cpu_with(&[("tech_state", tech_only(3))]);
    assert_eq!(cpu.tech_status_change(key, &TechRequest { passive_wall: false, ..request(STOP_WALL) }), TechChange::Unchanged);
    // Ceiling: any pick except No Tech techs in place.
    for (option, want) in [(3usize, Some(PASSIVE_CEIL)), (1, Some(PASSIVE_CEIL)), (2, Some(PASSIVE_CEIL)), (0, None)] {
        for requested in [STOP_CEIL, REFLECT_U] {
            let (mut cpu, key) = cpu_with(&[("tech_state", tech_only(option))]);
            let got = cpu.tech_status_change(key, &request(requested));
            assert_eq!(got, want.map_or(TechChange::Unchanged, TechChange::Status), "ceiling, option {option}");
        }
    }
    let (mut cpu, key) = cpu_with(&[("tech_state", tech_only(3))]);
    assert_eq!(cpu.tech_status_change(key, &TechRequest { passive_ceil: false, ..request(STOP_CEIL) }), TechChange::Unchanged);
}

#[test]
fn cpu04_motion_substitution() {
    let m = MotionHashes::new();
    assert_eq!(m.passive_stand_f, hash40("passive_stand_f"));
    // Tech Roll Backwards held: the forward tech animation becomes the backward one, and the reverse.
    let (mut cpu, key) = cpu_with(&[("tech_state", tech_only(2))]);
    cpu.tech_status_change(key, &request(DOWN));
    assert_eq!(cpu.substitute_motion(key, m.passive_stand_f), m.passive_stand_b);
    let (mut cpu, key) = cpu_with(&[("tech_state", tech_only(1))]);
    cpu.tech_status_change(key, &request(DOWN));
    assert_eq!(cpu.substitute_motion(key, m.passive_stand_b), m.passive_stand_f);
    // Missed-tech getup animations follow the held miss-tech roll direction, face-up and face-down apart.
    let (mut cpu, key) = cpu_with(&[("miss_tech_state", {
        let mut row = vec![0u8; 4];
        row[3] = 1; // Roll Backwards
        row
    })]);
    let mut r = cpu.rng.clone();
    cpu.fighters[key.index()].tech.mistech(&cpu.ids, &cpu.settings, &mut r, DOWN_WAIT, false);
    assert_eq!(cpu.fighters[key.index()].tech.miss_roll, Some(Roll::Backward));
    assert_eq!(cpu.substitute_motion(key, m.down_forward_u), m.down_back_u);
    assert_eq!(cpu.substitute_motion(key, m.down_forward_d), m.down_back_d);
    // Everything else passes through; so does an unheld direction; and an empty row leaves them alone.
    assert_eq!(cpu.substitute_motion(key, hash40("wait")), hash40("wait"));
    let (cpu2, key2) = cpu_with(&[("tech_state", tech_only(1))]);
    assert_eq!(cpu2.substitute_motion(key2, m.passive_stand_b), m.passive_stand_b, "no direction held yet");
    let (cpu3, key3) = cpu_with(&[("tech_state", vec![0; 4])]);
    assert_eq!(cpu3.substitute_motion(key3, m.passive_stand_f), m.passive_stand_f);
}

fn miss_only(option: usize) -> Vec<(&'static str, Vec<u8>)> {
    let mut row = vec![0u8; 4];
    row[option] = 1;
    vec![("miss_tech_state", row)]
}

fn frame_in(cpu: &mut Cpu, key: FighterKey, status: i32) -> model::cpu::facts::FrameOut {
    cpu.frame(key, &FrameFacts { status, ..FrameFacts::default() })
}

#[test]
fn cpu04_mistech_table() {
    // Down-wait (both kinds): Getup, Getup Attack, Roll Forwards, Roll Backwards.
    for (option, want) in [(0usize, DOWN_STAND), (1, DOWN_STAND_ATTACK), (2, DOWN_STAND_FB), (3, DOWN_STAND_FB)] {
        for status in [DOWN_WAIT, DOWN_WAIT_CONTINUE] {
            let (mut cpu, key) = cpu_with(&miss_only(option));
            assert_eq!(frame_in(&mut cpu, key, status).force_status, Some(want), "option {option}, status {status}");
        }
    }
    // The roll directions are held for the animation hook.
    let (mut cpu, key) = cpu_with(&miss_only(2));
    frame_in(&mut cpu, key, DOWN_WAIT);
    assert_eq!(cpu.fighters[key.index()].tech.miss_roll, Some(Roll::Forward));
    // Slip-wait (banana and similar).
    for (option, want) in [(0usize, SLIP_STAND), (1, SLIP_STAND_ATTACK), (2, SLIP_STAND_F), (3, SLIP_STAND_B)] {
        let (mut cpu, key) = cpu_with(&miss_only(option));
        assert_eq!(frame_in(&mut cpu, key, SLIP_WAIT).force_status, Some(want), "slip option {option}");
    }
    // Any other status forces nothing; an empty row forces nothing.
    let (mut cpu, key) = cpu_with(&miss_only(0));
    assert_eq!(frame_in(&mut cpu, key, WAIT).force_status, None);
    let (mut cpu, key) = cpu_with(&[("miss_tech_state", vec![0; 4])]);
    assert_eq!(frame_in(&mut cpu, key, DOWN_WAIT).force_status, None);
}

#[test]
fn cpu04_mistech_gate_is_its_own_row() {
    // B8 (question 1): clearing Tech Options no longer disables Mistech Options.
    let mut rows = miss_only(0);
    rows.push(("tech_state", vec![0; 4]));
    let (mut cpu, key) = cpu_with(&rows);
    assert_eq!(frame_in(&mut cpu, key, DOWN_WAIT).force_status, Some(DOWN_STAND));
}

#[test]
fn cpu04_laydown_lockout() {
    // lockout = min(base + damage / damage_max * (max - base), max), truncated.
    assert_eq!(laydown_lockout(10.0, 50.0, 100.0, 0.0), 10);
    assert_eq!(laydown_lockout(10.0, 50.0, 100.0, 50.0), 30);
    assert_eq!(laydown_lockout(10.0, 50.0, 100.0, 100.0), 50);
    assert_eq!(laydown_lockout(10.0, 50.0, 100.0, 300.0), 50, "capped at max");
    assert_eq!(laydown_lockout(10.0, 50.0, 100.0, 33.0), 23, "truncated: 10 + 0.33 * 40 = 23.2");
    assert_eq!(laydown_lockout(10.0, 50.0, 0.0, 80.0), 10, "no damage_max: no scaling");
    // The body waits out the lock-out in LAY_DOWN, then the getup is forced; the clock ticks once per frame.
    let (mut cpu, key) = cpu_with(&miss_only(0));
    let facts = FrameFacts {
        status: LAY_DOWN,
        tech: TechFacts { laydown_base: 3.0, laydown_max: 3.0, laydown_damage_max: 100.0, damage: 0.0, ..TechFacts::default() },
        ..FrameFacts::default()
    };
    let mut forced_at = None;
    for n in 0..10 {
        cpu.tick_clock();
        if cpu.frame(key, &facts).force_status.is_some() {
            forced_at = Some(n);
            break;
        }
    }
    assert_eq!(forced_at, Some(3), "a 3-frame lock-out: waiting on the first frame and the next two, forced on the fourth");
    // Any other status resets the counter.
    let (mut cpu, key) = cpu_with(&miss_only(0));
    cpu.frame(key, &facts);
    cpu.tick_clock();
    frame_in(&mut cpu, key, WAIT);
    assert_eq!(cpu.fighters[key.index()].counters.count(model::cpu::counter::CounterId::LayDown), 0);
}

#[test]
fn cpu04_tech_triggers_mash() {
    // A tech with the Tech Option trigger ticked buffers the override (non-empty) else mash_state.
    let mut triggers = vec![0u8; 17];
    triggers[9] = 1;
    let (mut cpu, key) = cpu_with(&[
        ("tech_state", tech_only(3)),
        ("mash_triggers", triggers.clone()),
        ("mash_state", actions(&[18])),
        ("tech_action_override", actions(&[16])),
    ]);
    cpu.tech_status_change(key, &request(DOWN));
    assert_eq!(cpu.fighters[key.index()].mash.queue.current().map(|a| a.0), Some(16), "override first");
    let (mut cpu, key) = cpu_with(&[("tech_state", tech_only(3)), ("mash_triggers", triggers.clone()), ("mash_state", actions(&[18]))]);
    cpu.tech_status_change(key, &request(DOWN));
    assert_eq!(cpu.fighters[key.index()].mash.queue.current().map(|a| a.0), Some(18));
    // Without the trigger nothing is buffered; a missed tech (No Tech) buffers nothing either.
    let (mut cpu, key) = cpu_with(&[("tech_state", tech_only(3)), ("mash_triggers", vec![0; 17]), ("mash_state", actions(&[18]))]);
    cpu.tech_status_change(key, &request(DOWN));
    assert!(cpu.fighters[key.index()].mash.queue.is_empty());
    let (mut cpu, key) = cpu_with(&[("tech_state", tech_only(0)), ("mash_triggers", triggers), ("mash_state", actions(&[18]))]);
    cpu.tech_status_change(key, &request(DOWN));
    assert!(cpu.fighters[key.index()].mash.queue.is_empty());
    // The Mistech Option trigger does the same for the forced getup.
    let mut mis = vec![0u8; 17];
    mis[10] = 1;
    let mut rows = miss_only(0);
    rows.push(("mash_triggers", mis));
    rows.push(("mash_state", actions(&[18])));
    let (mut cpu, key) = cpu_with(&rows);
    frame_in(&mut cpu, key, DOWN_WAIT);
    assert_eq!(cpu.fighters[key.index()].mash.queue.current().map(|a| a.0), Some(18));
}

#[test]
fn cpu04_a_departed_entry_forgets_its_hidden_record_and_the_scene_forgets_any() {
    let (mut cpu, key) = cpu_with(&[("tech_hide", vec![1, 0])]);
    cpu.frame(key, &hide_facts(DOWN, 2, 40, 12.5));
    let entry = usize::from(cpu.hidden.expect("record made").entry);
    // Another entry leaving does not touch the record; its own entry's departure drops it.
    cpu.forget_hidden_entry(entry + 1);
    assert!(cpu.hidden.is_some());
    cpu.forget_hidden_entry(entry);
    assert!(cpu.hidden.is_none());
    // A scene end drops whatever is left.
    cpu.frame(key, &hide_facts(DOWN, 2, 40, 12.5));
    assert!(cpu.hidden.is_some());
    cpu.forget_hidden();
    assert!(cpu.hidden.is_none());
}
fn hide_facts(status: i32, frame: i32, end: i32, cursor: f32) -> FrameFacts {
    FrameFacts { status, motion_frame: frame, tech: TechFacts { end_frame: end, cursor_offset: cursor, ..TechFacts::default() }, ..FrameFacts::default() }
}

#[test]
fn cpu04_hide_tech_lifecycle() {
    let (mut cpu, key) = cpu_with(&[("tech_hide", vec![1, 0])]);
    // First frame of a missed tech: the record is made and the cursor goes off screen; nothing hidden yet.
    let out = cpu.frame(key, &hide_facts(DOWN, 2, 40, 12.5)).hide;
    assert_eq!(out.cursor, Some(HIDDEN_CURSOR_OFFSET));
    assert_eq!((out.model_visible, out.hide_effects), (None, false));
    assert!(cpu.hidden.is_some());
    // From motion frame 6: the model and the five effects are hidden.
    let out = cpu.frame(key, &hide_facts(DOWN, 6, 40, 800.0)).hide;
    assert_eq!((out.model_visible, out.hide_effects), (Some(false), true));
    // The last five frames show the model again; the effects stay hidden.
    let out = cpu.frame(key, &hide_facts(DOWN, 35, 40, 800.0)).hide;
    assert_eq!((out.model_visible, out.hide_effects), (Some(true), true));
    // The status ends: restore the model and write the saved cursor back, once.
    let out = cpu.frame(key, &hide_facts(WAIT, 1, 10, 800.0)).hide;
    assert_eq!((out.model_visible, out.cursor), (Some(true), Some(12.5)));
    assert!(cpu.hidden.is_none());
    assert_eq!(cpu.frame(key, &hide_facts(WAIT, 2, 10, 12.5)).hide, Default::default());
    // Tech in place and tech roll hide too.
    for status in [PASSIVE, PASSIVE_FB] {
        let out = cpu.frame(key, &hide_facts(status, 8, 30, 0.0)).hide;
        assert_eq!(out.model_visible, Some(false), "status {status}");
        cpu.frame(key, &hide_facts(WAIT, 1, 10, 0.0));
    }
}

#[test]
fn cpu04_hide_tech_toggle_off_mid_tech() {
    let (mut cpu, key) = cpu_with(&[("tech_hide", vec![1, 0])]);
    cpu.frame(key, &hide_facts(DOWN, 2, 40, 7.0));
    cpu.frame(key, &hide_facts(DOWN, 8, 40, 800.0));
    // The player turns Hide Tech Animations Off while the body is hidden: restored on the next frame.
    cpu.publish(settings(&[("tech_hide", vec![0, 1])]), 0);
    let out = cpu.frame(key, &hide_facts(DOWN, 9, 40, 800.0)).hide;
    assert_eq!((out.model_visible, out.cursor), (Some(true), Some(7.0)));
    assert!(cpu.hidden.is_none());
}

#[test]
fn cpu04_primary_body_only() {
    // Hide tech acts on the primary body only; Nana stays visible; a replaced body forgets the record.
    let (mut cpu, popo) = cpu_with(&[("tech_hide", vec![1, 0])]);
    let nana = FighterKey::new(1, true).unwrap();
    cpu.fighters[nana.index()].pending = 0;
    cpu.fighters[nana.index()].defence.stale_written = Some(false);
    assert_eq!(cpu.frame(nana, &hide_facts(DOWN, 8, 40, 0.0)).hide, Default::default());
    assert!(cpu.hidden.is_none());
    assert_eq!(cpu.frame(popo, &hide_facts(DOWN, 8, 40, 0.0)).hide.model_visible, Some(false));
    assert!(cpu.hidden.is_some());
    // An extra CPU on another entry is not the primary body either.
    let extra = FighterKey::new(2, false).unwrap();
    cpu.fighters[extra.index()].pending = 0;
    cpu.fighters[extra.index()].defence.stale_written = Some(false);
    assert_eq!(cpu.frame(extra, &hide_facts(DOWN, 8, 40, 0.0)).hide, Default::default());
    cpu.clear_session();
    assert!(cpu.hidden.is_none(), "the record dies with the session");
}

#[test]
fn cpu04_quake_rule() {
    let t = table();
    let on = settings(&[("tech_hide", vec![1, 0])]);
    let off = settings(&[]);
    assert!(quake_suppressed(&t, &on, DOWN));
    assert!(!quake_suppressed(&t, &on, WAIT), "only in the missed-tech status");
    assert!(!quake_suppressed(&t, &off, DOWN), "only with Hide Tech Animations On");
}

#[test]
fn cpu04_camera_table() {
    assert_eq!(camera_centre(StageClass::Standard), Some([0.0, 50.0, 230.0]));
    assert_eq!(camera_centre(StageClass::Smashville), Some([0.0, 35.0, 350.0]));
    assert_eq!(camera_centre(StageClass::HollowBastion), Some([0.0, 50.0, 210.0]));
    assert_eq!(camera_centre(StageClass::Other), None, "the game's own centre stays");
}

#[test]
fn cpu04_per_fighter_state() {
    // B6: Popo's held roll directions do not leak into Nana's.
    let (mut cpu, popo) = cpu_with(&[("tech_state", tech_only(2))]);
    let nana = FighterKey::new(1, true).unwrap();
    cpu.tech_status_change(popo, &request(DOWN));
    assert_eq!(cpu.fighters[popo.index()].tech.tech_roll, Some(Roll::Backward));
    assert_eq!(cpu.fighters[nana.index()].tech.tech_roll, None);
}

#[test]
fn cpu04_playback_wins_default() {
    let playing = PlaybackState { running: true, ..PlaybackState::default() };
    // Tech, mistech and the animation substitution yield to a playing recording.
    let mut rows = miss_only(0);
    rows.push(("tech_state", tech_only(3)));
    let (mut cpu, key) = cpu_with(&rows);
    cpu.playback = playing;
    assert_eq!(cpu.tech_status_change(key, &request(DOWN)), TechChange::Unchanged);
    assert_eq!(frame_in(&mut cpu, key, DOWN_WAIT).force_status, None);
    // They apply again when the playback ends.
    cpu.playback = PlaybackState::default();
    assert_eq!(cpu.tech_status_change(key, &request(DOWN)), TechChange::Status(PASSIVE));
    assert_eq!(frame_in(&mut cpu, key, DOWN_WAIT).force_status, Some(DOWN_STAND));
}

#[test]
fn cpu04_playback_override_on() {
    let mut rows = miss_only(0);
    rows.push(("tech_state", tech_only(3)));
    rows.push(("settings_override_playback", vec![1, 0]));
    let (mut cpu, key) = cpu_with(&rows);
    cpu.playback = PlaybackState { running: true, ..PlaybackState::default() };
    assert_eq!(cpu.tech_status_change(key, &request(DOWN)), TechChange::Status(PASSIVE));
    assert_eq!(frame_in(&mut cpu, key, DOWN_WAIT).force_status, Some(DOWN_STAND));
}

#[test]
fn cpu04_stage_class_follows_the_native_ids_and_the_record_form() {
    use model::cpu::tech::{stage, stage_class};
    // The stages themselves (form 0).
    for id in [stage::BATTLEFIELD, stage::FINAL_DESTINATION, stage::SMALL_BATTLEFIELD, stage::TOWN_AND_CITY, stage::POKEMON_STADIUM_2] {
        assert_eq!(stage_class(id, Some(0)), StageClass::Standard, "{id}");
    }
    assert_eq!(stage_class(stage::SMASHVILLE, Some(0)), StageClass::Smashville);
    assert_eq!(stage_class(stage::HOLLOW_BASTION, Some(0)), StageClass::HollowBastion);
    // Every Omega (1) and Battlefield (2) form of every stage is Standard, Smashville's and Hollow Bastion's too.
    for (id, form) in [(96, 1), (97, 2), (362, 1), (363, 2), (1, 1), (4, 2), (500, 1)] {
        assert_eq!(stage_class(id, Some(form)), StageClass::Standard, "{id} form {form}");
    }
    // Big Battlefield (2) and any other stage: the game's centre stays. An id outside the table too.
    assert_eq!(stage_class(2, Some(0)), StageClass::Other);
    assert_eq!(stage_class(5, Some(0)), StageClass::Other);
    assert_eq!(stage_class(-1, None), StageClass::Other);
    assert_eq!(stage_class(stage::STAGE_COUNT, None), StageClass::Other);
}

#[test]
fn cpu04_camera_setup_and_mode_apply_the_stage_value_and_restore_the_default() {
    let game = [0.0, 40.0, 200.0];
    // Hide Tech Animations On: the stage's value goes in, at the setup and at the mode change.
    let (mut cpu, _) = cpu_with(&[("tech_hide", vec![1, 0])]);
    assert_eq!(cpu.camera_setup(StageClass::Smashville, game), Some([0.0, 35.0, 350.0]));
    assert_eq!(cpu.stage_default, Some(game), "the game's value is remembered as the stage default");
    assert_eq!(cpu.camera_mode(StageClass::Smashville, 4), Some([0.0, 35.0, 350.0]));
    assert_eq!(cpu.camera_mode(StageClass::Smashville, 3), None, "only the fixed mode");
    assert_eq!(cpu.camera_mode(StageClass::Other, 4), None, "no value for this stage: the game's stays");
    assert_eq!(cpu.camera_setup(StageClass::Other, game), None);
    // A game centre that already is the stage's value is not rewritten.
    assert_eq!(cpu.camera_setup(StageClass::Standard, [0.0, 50.0, 230.0]), None);
    // Off: the default captured at the last setup comes back at the next fixed-mode change.
    let (mut cpu, _) = cpu_with(&[]);
    assert_eq!(cpu.camera_mode(StageClass::Smashville, 4), None, "nothing captured yet");
    assert_eq!(cpu.camera_setup(StageClass::Smashville, game), None, "Off: the setup passes through");
    assert_eq!(cpu.camera_mode(StageClass::Smashville, 4), Some(game));
}
