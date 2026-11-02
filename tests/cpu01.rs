//! SPEC-cpu-01 (mash, follow-up, triggers, overrides: the per-frame run) and the decisions it carries:
//! B4, B6, B8 (one roll per situation entry; a disabled mash empties the queue once), N4. Test names
//! are the spec's §9 rows. The shared pieces (queue, weighted pick, role) are in `tests/cpu_core.rs`.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::cpu::command::{pick_situation, CmdFacts, Situ};
use model::cpu::ids::{self, Table, C};
use model::cpu::mash::Action;
use model::cpu::role::FighterKey;
use model::cpu::settings::CpuSettings;
use model::cpu::state::Cpu;
use model::cpu::status::Situation;
use model::settings::Snapshot;

fn c(i: usize) -> C {
    assert!(i < ids::COUNT);
    // SAFETY: `C` is a fieldless `repr(u16)` enum numbered from 0; every value below COUNT is one.
    unsafe { std::mem::transmute::<u16, C>(i as u16) }
}

/// Every constant distinct (1000 + index), then the explicit values (later entries win).
fn table(extra: &[(C, i32)]) -> Table {
    let mut all: Vec<(C, i32)> = (0..ids::COUNT).map(|i| (c(i), 1000 + i as i32)).collect();
    all.extend_from_slice(extra);
    Table::with(&all)
}

const DAMAGE: i32 = 10;
const DAMAGE_FLY: i32 = 12;
const DAMAGE_FALL: i32 = 14;
const TECH: i32 = 40;
const CLATTER: i32 = 80;
const LANDING: i32 = 70;
const WAIT: i32 = 1;

fn base() -> Table {
    table(&[
        (C::StatusDamage, DAMAGE),
        (C::StatusDamageFly, DAMAGE_FLY),
        (C::StatusDamageFall, DAMAGE_FALL),
        (C::StatusDownStand, TECH),
        (C::StatusDownStandAttack, TECH + 3),
        (C::StatusPassive, 50),
        (C::StatusPassiveCeil, 53),
        (C::ClatterBury, CLATTER),
        (C::StatusLanding, LANDING),
        (C::StatusLandingDamageLight, LANDING + 3),
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

fn actions(at: &[usize]) -> Vec<u8> {
    let mut row = vec![0u8; 30];
    for i in at {
        row[*i] = 1;
    }
    row
}

fn triggers(at: &[usize]) -> Vec<u8> {
    let mut row = vec![0u8; 17];
    for i in at {
        row[*i] = 1;
    }
    row
}

fn cpu_with(t: Table, rows: &[(&str, Vec<u8>)]) -> (Cpu, FighterKey) {
    let mut cpu = Cpu::new(settings(rows), 55);
    cpu.ids = t;
    let key = FighterKey::new(1, false).unwrap();
    cpu.fighters[key.index()].pending = 0;
    (cpu, key)
}

fn facts(status: i32) -> CmdFacts {
    CmdFacts { status, situation: Situation::Ground, ..CmdFacts::default() }
}

#[test]
fn cpu01_trigger_priority() {
    // tech > clatter > tumble > hitstun > parry > footstool > trump > landing > generic.
    let t = base();
    let s = settings(&[
        ("mash_state", actions(&[18])),
        // Hitstun, Tumble, Clatter, Landing, Tech Option, Always.
        ("mash_triggers", triggers(&[0, 3, 7, 4, 9, 16])),
        ("tumble_override", actions(&[3])),
    ]);
    let pick = |status: i32| pick_situation(&t, &s, &facts(status)).map(|(situ, _)| situ);
    assert_eq!(pick(TECH + 1), Some(Situ::Tech));
    assert_eq!(pick(CLATTER), Some(Situ::Clatter));
    // Tumble (12..=14) is checked before hitstun (10..14), which contains it.
    assert_eq!(pick(DAMAGE_FLY), Some(Situ::Tumble));
    assert_eq!(pick(DAMAGE), Some(Situ::Hitstun));
    assert_eq!(pick(LANDING + 1), Some(Situ::Landing));
    assert_eq!(pick(WAIT), Some(Situ::Generic), "Always");
    // The parry comes from the motion kind.
    let parry = CmdFacts { in_parry: true, ..facts(WAIT) };
    let s2 = settings(&[("mash_state", actions(&[18])), ("mash_triggers", triggers(&[2]))]);
    assert_eq!(pick_situation(&t, &s2, &parry).map(|(x, _)| x), Some(Situ::Parry));
    assert!(pick_situation(&t, &s2, &facts(WAIT)).is_none());
}

#[test]
fn cpu01_override_precedence() {
    let t = base();
    // An override with options replaces mash_state even when its trigger is not ticked; an empty
    // override falls back to the trigger rule.
    let only_override = settings(&[("hitstun_override", actions(&[6])), ("mash_state", actions(&[18])), ("mash_triggers", triggers(&[]))]);
    let (_, row) = pick_situation(&t, &only_override, &facts(DAMAGE)).unwrap();
    assert_eq!((row.weight(6), row.weight(18)), (1, 0));
    let fallback = settings(&[("mash_state", actions(&[18])), ("mash_triggers", triggers(&[0]))]);
    let (_, row) = pick_situation(&t, &fallback, &facts(DAMAGE)).unwrap();
    assert_eq!(row.weight(18), 1);
    let neither = settings(&[("mash_state", actions(&[18])), ("mash_triggers", triggers(&[]))]);
    assert!(pick_situation(&t, &neither, &facts(DAMAGE)).is_none());
}

#[test]
fn cpu01_distance_thresholds() {
    let t = base();
    let near = |d: f32, trig: usize| {
        let s = settings(&[("mash_state", actions(&[18])), ("mash_triggers", triggers(&[trig]))]);
        pick_situation(&t, &s, &CmdFacts { distance: d, ..facts(WAIT) }).is_some()
    };
    // Close 16, Mid 37, Far 64 (strictly below).
    assert!(near(15.9, 13) && !near(16.0, 13));
    assert!(near(36.9, 14) && !near(37.0, 14));
    assert!(near(63.9, 15) && !near(64.0, 15));
    // No player: infinite distance.
    assert!(!near(f32::INFINITY, 15));
}

#[test]
fn cpu01_one_roll_per_situation() {
    // B8: the pick is made when the situation is entered and kept while it lasts; it is made again
    // the next time the situation starts.
    let (mut cpu, key) = cpu_with(base(), &[("mash_state", actions(&[18, 19, 20, 21])), ("mash_triggers", triggers(&[0]))]);
    let f = facts(DAMAGE);
    cpu.command_flags(key, false, &f);
    let first = cpu.fighters[key.index()].mash.queue.current();
    assert!(first.is_some());
    // The action is performed and completes only when its expected status shows up; until then the
    // queue holds the same action however many frames the hitstun lasts.
    for _ in 0..30 {
        cpu.command_flags(key, false, &f);
        assert_eq!(cpu.fighters[key.index()].mash.queue.current(), first);
    }
    // The situation ends and starts again: a new roll (over many entries every option shows up).
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..100 {
        cpu.command_flags(key, false, &facts(WAIT));
        cpu.fighters[key.index()].mash.queue.clear();
        cpu.command_flags(key, false, &f);
        seen.insert(cpu.fighters[key.index()].mash.queue.current().unwrap().0);
    }
    assert_eq!(seen.len(), 4);
}

/// (action, flag, expected status) for the straightforward actions of the table.
fn action_rows() -> Vec<(Action, C, C)> {
    vec![
        (Action::JAB, C::FlagAttackN, C::StatusAttack),
        (Action::F_TILT, C::FlagAttackS3, C::StatusAttackS3),
        (Action::U_TILT, C::FlagAttackHi3, C::StatusAttackHi3),
        (Action::D_TILT, C::FlagAttackLw3, C::StatusAttackLw3),
        (Action::F_SMASH, C::FlagAttackS4, C::StatusAttackS4Start),
        (Action::U_SMASH, C::FlagAttackHi4, C::StatusAttackHi4Start),
        (Action::D_SMASH, C::FlagAttackLw4, C::StatusAttackLw4Start),
        (Action::NEUTRAL_B, C::FlagSpecialN, C::StatusSpecialN),
        (Action::SIDE_B, C::FlagSpecialS, C::StatusSpecialS),
        (Action::UP_B, C::FlagSpecialHi, C::StatusSpecialHi),
        (Action::DOWN_B, C::FlagSpecialLw, C::StatusSpecialLw),
        (Action::SPOT_DODGE, C::FlagEscape, C::StatusEscape),
        (Action::ROLL_F, C::FlagEscapeF, C::StatusEscapeF),
        (Action::ROLL_B, C::FlagEscapeB, C::StatusEscapeB),
        (Action::GRAB, C::FlagCatch, C::StatusCatch),
    ]
}

fn queued(action: Action) -> (Cpu, FighterKey) {
    let (mut cpu, key) = cpu_with(base(), &[]);
    cpu.fighters[key.index()].mash.queue.push(action);
    (cpu, key)
}

#[test]
fn cpu01_action_flags_table() {
    let t = base();
    for (action, flag, expected) in action_rows() {
        let (mut cpu, key) = queued(action);
        let out = cpu.command_flags(key, false, &facts(WAIT));
        assert_eq!(out.bits, t.get(flag) as u32, "{action:?}");
        assert_eq!(cpu.fighters[key.index()].mash.queue.current(), Some(action), "not complete before the status");
        cpu.command_flags(key, false, &facts(t.get(expected)));
        assert!(cpu.fighters[key.index()].mash.queue.is_empty(), "{action:?} completes at its expected status");
    }
    // Jab only grounded; grab blocked by an invalid-catch frame; shield airborne returns 0 and stays queued.
    let (mut cpu, key) = queued(Action::JAB);
    let air = CmdFacts { situation: Situation::Air, ..facts(WAIT) };
    assert_eq!(cpu.command_flags(key, false, &air).bits, 0);
    let (mut cpu, key) = queued(Action::GRAB);
    assert_eq!(cpu.command_flags(key, false, &CmdFacts { invalid_catch_frame: 3, ..facts(WAIT) }).bits, 0);
    let (mut cpu, key) = queued(Action::SHIELD);
    assert_eq!(cpu.command_flags(key, false, &air).bits, 0);
    assert_eq!(cpu.fighters[key.index()].mash.queue.current(), Some(Action::SHIELD));
    // Grounded shield and grounded air dodge: the harmless bit, completing at GUARD_ON; airborne air dodge: ESCAPE_AIR.
    let (mut cpu, key) = queued(Action::SHIELD);
    assert_eq!(cpu.command_flags(key, false, &facts(WAIT)).bits, t.get(C::FlagAirEscape) as u32);
    let (mut cpu, key) = queued(Action::AIR_DODGE);
    assert_eq!(cpu.command_flags(key, false, &air).bits, t.get(C::FlagAirEscape) as u32);
    cpu.command_flags(key, false, &CmdFacts { status: t.get(C::StatusEscapeAir), ..air });
    assert!(cpu.fighters[key.index()].mash.queue.is_empty());
    // Jump: the squat when grounded, the aerial jump in the air.
    let (mut cpu, key) = queued(Action::JUMP);
    assert_eq!(cpu.command_flags(key, false, &facts(WAIT)).bits, t.get(C::FlagJumpButton) as u32);
    cpu.command_flags(key, false, &facts(t.get(C::StatusJumpSquat)));
    assert!(cpu.fighters[key.index()].mash.queue.is_empty());
    // A playback action never produces bits.
    let (mut cpu, key) = queued(Action(25));
    assert_eq!(cpu.command_flags(key, false, &facts(WAIT)).bits, 0);
}

#[test]
fn cpu01_dash_attack_sequence() {
    let t = base();
    let dash = t.get(C::StatusDash);
    let (mut cpu, key) = queued(Action::DASH_ATTACK);
    // Not dashing: request a dash (when CONT_DASH is enabled), no bits.
    let out = cpu.command_flags(key, false, &CmdFacts { cont_dash: true, ..facts(WAIT) });
    assert_eq!((out.bits, out.request_status), (0, Some(dash)));
    // In DASH at frame 0 with the dash motion: the first such frame sets the latch, the next requests the dash attack.
    let in_dash = CmdFacts { motion_is_dash: true, ..facts(dash) };
    assert_eq!(cpu.command_flags(key, false, &in_dash).request_status, None);
    assert!(cpu.fighters[key.index()].mash.transitioning_dash);
    assert_eq!(cpu.command_flags(key, false, &in_dash).request_status, Some(t.get(C::StatusAttackDash)));
    // The dash attack completes the action; the latch clears.
    cpu.command_flags(key, false, &facts(t.get(C::StatusAttackDash)));
    assert!(cpu.fighters[key.index()].mash.queue.is_empty());
    // A dash is never interrupted with bits, and no re-buffer happens while beginning a dash attack.
    let (mut cpu, key) = cpu_with(base(), &[("mash_state", actions(&[4])), ("mash_triggers", triggers(&[16]))]);
    cpu.fighters[key.index()].mash.queue.push(Action::DASH_ATTACK);
    cpu.command_flags(key, false, &facts(dash));
    assert_eq!(cpu.fighters[key.index()].mash.queue.current(), Some(Action::DASH_ATTACK), "ROLL_F did not replace it");
}

#[test]
fn cpu01_character_specific_completion() {
    let t = table(&[
        (C::KindKoopa, 7),
        (C::KindKirby, 8),
        (C::KoopaSpecialHiG, 500),
        (C::KoopaSpecialHiA, 501),
        (C::KirbyMiiSpecialN, 600),
        (C::KirbyNum, 640),
    ]);
    let run = |action: Action, kind: i32, status: i32| {
        let mut cpu = Cpu::new(settings(&[]), 1);
        cpu.ids = t.clone();
        let key = FighterKey::new(1, false).unwrap();
        cpu.fighters[key.index()].mash.queue.push(action);
        cpu.command_flags(key, false, &CmdFacts { kind, ..facts(status) });
        cpu.fighters[key.index()].mash.queue.is_empty()
    };
    // Bowser's up special completes in either of its two statuses.
    assert!(run(Action::UP_B, 7, 500) && run(Action::UP_B, 7, 501));
    assert!(!run(Action::UP_B, 3, 500));
    // Kirby's whole copied-neutral-special range completes Neutral Special (fork fix).
    assert!(run(Action::NEUTRAL_B, 8, 600) && run(Action::NEUTRAL_B, 8, 639));
    assert!(!run(Action::NEUTRAL_B, 8, 640) && !run(Action::NEUTRAL_B, 8, 599));
}

#[test]
fn cpu01_menu_disable_clears_queue() {
    let (mut cpu, key) = cpu_with(base(), &[("mash_state", actions(&[18])), ("mash_triggers", triggers(&[0]))]);
    cpu.fighters[key.index()].mash.queue.push(Action::JAB);
    // Other menu edits leave the queue alone.
    cpu.publish(settings(&[("mash_state", actions(&[18])), ("mash_triggers", triggers(&[0, 1]))]), 0);
    assert!(!cpu.fighters[key.index()].mash.queue.is_empty());
    // Both rows emptied, at least one changed: the queue is emptied once.
    cpu.publish(settings(&[("mash_state", actions(&[])), ("mash_triggers", triggers(&[]))]), 0);
    assert!(cpu.fighters[key.index()].mash.queue.is_empty());
    // Pushing again and publishing the same empty rows does not empty it again.
    cpu.fighters[key.index()].mash.queue.push(Action::JAB);
    cpu.publish(settings(&[("mash_state", actions(&[])), ("mash_triggers", triggers(&[]))]), 0);
    assert!(!cpu.fighters[key.index()].mash.queue.is_empty());
}

#[test]
fn cpu01_reset_clears_queue() {
    // No choice while a save state loads; the reset bundle empties the queue and the situation latch.
    let (mut cpu, key) = cpu_with(base(), &[("mash_state", actions(&[18])), ("mash_triggers", triggers(&[0]))]);
    cpu.save_loading = true;
    cpu.command_flags(key, false, &facts(DAMAGE));
    assert!(cpu.fighters[key.index()].mash.queue.is_empty());
    cpu.save_loading = false;
    cpu.command_flags(key, false, &facts(DAMAGE));
    assert!(!cpu.fighters[key.index()].mash.queue.is_empty());
    cpu.hub_step(model::lra_reset::HubStep::Mash);
    assert!(cpu.fighters[key.index()].mash.queue.is_empty());
    assert_eq!(cpu.fighters[key.index()].mash.latch, None);
}

#[test]
fn cpu01_nana_and_missing_accessor() {
    // Nana is her own body and gets the same bits as Popo on her own query; a table with nothing
    // resolved asks for nothing and never panics.
    let (mut cpu, popo) = cpu_with(base(), &[]);
    let nana = FighterKey::new(1, true).unwrap();
    cpu.fighters[popo.index()].mash.queue.push(Action::JAB);
    cpu.fighters[nana.index()].mash.queue.push(Action::JAB);
    let a = cpu.command_flags(popo, false, &facts(WAIT)).bits;
    let b = cpu.command_flags(nana, false, &facts(WAIT)).bits;
    assert_eq!(a, b);
    assert_ne!(a, 0);
    let mut empty = Cpu::new(settings(&[]), 2);
    empty.fighters[popo.index()].mash.queue.push(Action::JAB);
    assert_eq!(empty.command_flags(popo, false, &facts(WAIT)).bits, 0);
}

#[test]
fn cpu01_playback_stops_on_a_triggered_mash() {
    use model::cpu::playback::PlaybackState;
    let (mut cpu, key) = cpu_with(base(), &[("mash_state", actions(&[18])), ("mash_triggers", triggers(&[0]))]);
    cpu.playback = PlaybackState { running: true, ..PlaybackState::default() };
    let out = cpu.command_flags(key, false, &facts(DAMAGE));
    assert!(out.stop_playback);
    assert!(cpu.fighters[key.index()].mash.queue.is_empty());
    // After the playback stopped the mash triggers again.
    cpu.playback = PlaybackState::default();
    cpu.command_flags(key, false, &facts(DAMAGE));
    assert!(!cpu.fighters[key.index()].mash.queue.is_empty());
}
