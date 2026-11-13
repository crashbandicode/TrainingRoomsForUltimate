//! SPEC-cpu-05 (CPU shield, out-of-shield options, OoS offset, reaction time, shield tilt) and the
//! decisions it carries: B6 (one state per fighter), B7 (shield tilt In / Out are opponent-relative),
//! Q18 A (a changed Shield Tilt applies at the next CPU frame), N3 (OoS Offset and Reaction Time are a
//! checklist or a range). Test names are the spec's §9 rows.

// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::cpu::counter::CounterId;
use model::cpu::facts::FrameFacts;
use model::cpu::ids::{Table, C};
use model::cpu::mash::Action;
use model::cpu::playback::PlaybackState;
use model::cpu::role::FighterKey;
use model::cpu::settings::{CpuSettings, DelayRow, ShieldMode};
use model::cpu::shield::{
    damage_multiplier, decay_override_active, guard_button, queue_requests_shield, should_hold, ButtonAnswer, HoldFacts, OosFacts,
    SHIELD_DECAY_OVERRIDE, SHIELD_RECOVERY_OVERRIDE,
};
use model::cpu::state::Cpu;
use model::cpu::status::Situation;
use model::lra_reset::HubStep;
use model::menu::reroll;
use model::settings::Snapshot;

const WAIT: i32 = 1;
const GUARD: i32 = 50;
const GUARD_DAMAGE: i32 = 51;
const ESCAPE: i32 = 60;
const ESCAPE_F: i32 = 61;
const ESCAPE_B: i32 = 62;

fn table() -> Table {
    Table::with(&[
        (C::StatusWait, WAIT),
        (C::StatusGuard, GUARD),
        (C::StatusGuardOn, 49),
        (C::StatusGuardOff, 52),
        (C::StatusGuardDamage, GUARD_DAMAGE),
        (C::StatusDamage, 10),
        (C::StatusDamageFall, 14),
        (C::StatusEscape, ESCAPE),
        (C::StatusEscapeF, ESCAPE_F),
        (C::StatusEscapeB, ESCAPE_B),
    ])
}

fn settings(rows: &[(&str, Vec<u8>)]) -> CpuSettings {
    let mut snap = Snapshot::factory();
    for (id, row) in rows {
        snap.set(id, row).unwrap();
    }
    CpuSettings::from_snapshot(&snap)
}

/// A Shield Toggles row: None 0, Infinite 1, Hold 2, Constant 3.
fn mode(i: usize) -> (&'static str, Vec<u8>) {
    let mut row = vec![0u8; 4];
    row[i] = 1;
    ("shield_state", row)
}

fn delay(at: &[usize]) -> Vec<u8> {
    let mut row = vec![0u8; 31];
    for i in at {
        row[*i] = 1;
    }
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
    let mut cpu = Cpu::new(settings(rows), 33);
    cpu.ids = table();
    let key = FighterKey::new(1, false).unwrap();
    cpu.fighters[key.index()].pending = 0;
    cpu.fighters[key.index()].defence.stale_written = Some(false);
    (cpu, key)
}

fn frame(cpu: &mut Cpu, key: FighterKey, status: i32, prev: i32) {
    cpu.tick_clock();
    cpu.frame(key, &FrameFacts { status, prev_status: prev, situation: Situation::Ground, ..FrameFacts::default() });
}

fn cpu05_should_hold_shield() {
    let base = HoldFacts::default();
    // Any of Hold, Infinite and Constant holds; None holds only while the queue asks for the shield.
    for m in [ShieldMode::Infinite, ShieldMode::Hold, ShieldMode::Constant] {
        assert!(should_hold(m, &base), "{m:?}");
    }
    assert!(!should_hold(ShieldMode::None, &base));
    assert!(should_hold(ShieldMode::None, &HoldFacts { queue_requests_shield: true, ..base }));
    // Never in playback, standby, a save-state load, or right after being airborne.
    for blocked in [
        HoldFacts { playback_running: true, ..base },
        HoldFacts { standby: true, ..base },
        HoldFacts { save_state_loading: true, ..base },
        HoldFacts { was_airborne: true, ..base },
    ] {
        assert!(!should_hold(ShieldMode::Infinite, &blocked), "{blocked:?}");
        assert!(!should_hold(ShieldMode::None, &HoldFacts { queue_requests_shield: true, ..blocked }));
    }
    // The queue asks for the shield: SHIELD, or AIR_DODGE while grounded.
    assert!(queue_requests_shield(Some(Action::SHIELD), false));
    assert!(queue_requests_shield(Some(Action::AIR_DODGE), true));
    assert!(!queue_requests_shield(Some(Action::AIR_DODGE), false));
    assert!(!queue_requests_shield(Some(Action::JAB), true) && !queue_requests_shield(None, true));
    // The button hooks: held and not released while holding and not suspended; no opinion otherwise.
    assert_eq!(guard_button(true, true, false), ButtonAnswer::Held(true));
    assert_eq!(guard_button(false, true, false), ButtonAnswer::Held(false));
    assert_eq!(guard_button(true, true, true), ButtonAnswer::Original);
    assert_eq!(guard_button(true, false, false), ButtonAnswer::Original);
    // Through the state: Hold On, a grounded body that was not airborne.
    let (mut cpu, key) = cpu_with(&[mode(2)]);
    frame(&mut cpu, key, WAIT, WAIT);
    assert_eq!(cpu.guard_button(key, true), ButtonAnswer::Held(true));
    cpu.save_loading = true;
    assert_eq!(cpu.guard_button(key, true), ButtonAnswer::Original);
}

fn cpu05_decay_override_rules() {
    assert!(decay_override_active(ShieldMode::Infinite, false) && decay_override_active(ShieldMode::Infinite, true));
    assert!(decay_override_active(ShieldMode::Constant, true));
    // Hold: no decay until the first hit on this shield, then normal decay.
    assert!(decay_override_active(ShieldMode::Hold, false));
    assert!(!decay_override_active(ShieldMode::Hold, true));
    // None is native: the override is never active (a visible change from the fork, decision B8).
    assert!(!decay_override_active(ShieldMode::None, false));
    assert_eq!((SHIELD_DECAY_OVERRIDE, SHIELD_RECOVERY_OVERRIDE), (0.0, 999.0));
    // Through the state, with Hold flipping when the body leaves shieldstun, and again when it stops guarding.
    let (mut cpu, key) = cpu_with(&[mode(2)]);
    assert_eq!(cpu.shield_param(key, true), Some(0.0));
    assert_eq!(cpu.shield_param(key, false), Some(999.0));
    cpu.out_of_shield(key, &OosFacts { left_shieldstun: true, shielding: true, ..OosFacts::default() });
    assert!(cpu.fighters[key.index()].shield.decay_enabled);
    assert_eq!(cpu.shield_param(key, true), None, "normal decay after the first hit");
    frame(&mut cpu, key, WAIT, WAIT);
    assert!(!cpu.fighters[key.index()].shield.decay_enabled, "the flag drops when the body is not in GUARD");
    assert_eq!(cpu.shield_param(key, true), Some(0.0));
    // The override does not act during a playback.
    cpu.playback = PlaybackState { running: true, ..PlaybackState::default() };
    assert_eq!(cpu.shield_param(key, true), None);
    let (cpu, key) = cpu_with(&[mode(0)]);
    assert_eq!(cpu.shield_param(key, true), None, "None keeps the native decay");
}

fn cpu05_damage_mul_restore() {
    // Infinite in Training writes 0.0; every other mode, and every frame outside Training, writes the
    // value the game booted with, so leaving Training restores it.
    assert_eq!(damage_multiplier(ShieldMode::Infinite, true, Some(1.19)), Some(0.0));
    assert_eq!(damage_multiplier(ShieldMode::Infinite, false, Some(1.19)), Some(1.19));
    assert_eq!(damage_multiplier(ShieldMode::Hold, true, Some(1.19)), Some(1.19));
    assert_eq!(damage_multiplier(ShieldMode::None, true, None), None, "nothing to write until the game's value is known");
    let (mut cpu, _) = cpu_with(&[mode(1)]);
    assert_eq!(cpu.damage_multiplier(true, 1.19), Some(0.0));
    assert_eq!(cpu.shield_damage_cache, Some(1.19), "the first look caches the game's value");
    assert_eq!(cpu.damage_multiplier(false, 0.0), Some(1.19), "restored outside Training even when the game now reads 0");
    // The cache survives a session clear (the restore must still work).
    cpu.clear_session();
    assert_eq!(cpu.damage_multiplier(false, 0.0), Some(1.19));
}

/// A shielded hit: one frame of shieldstun, then the frame after it (`left_shieldstun`), and the OoS
/// decision, returning whether the CPU acted (a mash action was buffered or a status requested).
fn hit_and_decide(cpu: &mut Cpu, key: FighterKey, facts: OosFacts) -> model::cpu::shield::OosOut {
    frame(cpu, key, GUARD_DAMAGE, GUARD);
    frame(cpu, key, GUARD, GUARD_DAMAGE);
    cpu.out_of_shield(key, &OosFacts { left_shieldstun: true, shielding: true, ..facts })
}

fn shieldstun_triggers() -> Vec<u8> {
    let mut t = vec![0u8; 17];
    t[1] = 1;
    t
}

fn cpu05_oos_offset_count() {
    // OoS Offset 2: the CPU may take two hits and acts out of the third (the offset starts at N + 1).
    let (mut cpu, key) = cpu_with(&[
        mode(2),
        ("oos_offset", delay(&[2])),
        ("mash_state", actions(&[3])),
        ("mash_triggers", shieldstun_triggers()),
    ]);
    frame(&mut cpu, key, WAIT, WAIT);
    assert_eq!(cpu.fighters[key.index()].shield.offset, 3);
    for hit in 1..=3 {
        hit_and_decide(&mut cpu, key, OosFacts { cont_escape: false, ..OosFacts::default() });
        let acted = !cpu.fighters[key.index()].mash.queue.is_empty();
        assert_eq!(acted, hit == 3, "hit {hit}");
        cpu.fighters[key.index()].mash.queue.clear();
        frame(&mut cpu, key, GUARD, GUARD);
    }
    // Idle or hitstun resets the count.
    frame(&mut cpu, key, WAIT, GUARD);
    assert_eq!(cpu.fighters[key.index()].shield.offset, 3);
    // With the shield held the CPU is back in its shield straight from the dodge (or from any other status), with no
    // frame of WAIT or hitstun between (wa-diag10: `31 → 27`, `65 → 27`); leaving the shield starts the count again,
    // so the next three hits go the same way.
    for (gone, back) in [(ESCAPE, ESCAPE), (99, 99)] {
        cpu.fighters[key.index()].shield.offset = 0;
        frame(&mut cpu, key, gone, GUARD);
        frame(&mut cpu, key, 49, back);
        frame(&mut cpu, key, GUARD, 49);
        for hit in 1..=3 {
            hit_and_decide(&mut cpu, key, OosFacts { cont_escape: false, ..OosFacts::default() });
            let acted = !cpu.fighters[key.index()].mash.queue.is_empty();
            assert_eq!(acted, hit == 3, "after status {gone}, hit {hit}");
            cpu.fighters[key.index()].mash.queue.clear();
            frame(&mut cpu, key, GUARD, GUARD);
        }
    }
    // An empty row means 0: the first hit acts.
    let (mut cpu, key) = cpu_with(&[mode(2), ("mash_state", actions(&[3])), ("mash_triggers", shieldstun_triggers())]);
    frame(&mut cpu, key, WAIT, WAIT);
    hit_and_decide(&mut cpu, key, OosFacts::default());
    assert!(!cpu.fighters[key.index()].mash.queue.is_empty());
}

fn cpu05_reaction_time() {
    // Reaction 10 frames: counted after the shieldstun, the decision waits, then acts; a parry blocks.
    let (mut cpu, key) = cpu_with(&[
        mode(2),
        ("reaction_time", delay(&[10])),
        ("mash_state", actions(&[3])),
        ("mash_triggers", shieldstun_triggers()),
    ]);
    frame(&mut cpu, key, WAIT, WAIT);
    frame(&mut cpu, key, GUARD_DAMAGE, GUARD);
    assert_eq!(cpu.fighters[key.index()].shield.reaction, 10);
    let mut acted_at = None;
    for n in 0..20 {
        frame(&mut cpu, key, GUARD, GUARD_DAMAGE);
        cpu.out_of_shield(key, &OosFacts { left_shieldstun: true, shielding: true, ..OosFacts::default() });
        if !cpu.fighters[key.index()].mash.queue.is_empty() {
            acted_at = Some(n);
            break;
        }
    }
    assert_eq!(acted_at, Some(10));
    // A parry in progress stops the decision.
    let (mut cpu, key) = cpu_with(&[mode(2), ("mash_state", actions(&[3])), ("mash_triggers", shieldstun_triggers())]);
    frame(&mut cpu, key, WAIT, WAIT);
    hit_and_decide(&mut cpu, key, OosFacts { in_parry: true, ..OosFacts::default() });
    assert!(cpu.fighters[key.index()].mash.queue.is_empty());
    // Not shielding any more: the reaction counter resets and nothing happens.
    let (mut cpu, key) = cpu_with(&[mode(2), ("mash_state", actions(&[3])), ("mash_triggers", shieldstun_triggers())]);
    frame(&mut cpu, key, WAIT, WAIT);
    frame(&mut cpu, key, GUARD_DAMAGE, GUARD); // the hit uses up the offset
    frame(&mut cpu, key, GUARD, GUARD_DAMAGE);
    cpu.fighters[key.index()].counters.get(CounterId::Reaction).should_delay(5);
    cpu.fighters[key.index()].counters.tick();
    assert_eq!(cpu.fighters[key.index()].counters.count(CounterId::Reaction), 1);
    cpu.out_of_shield(key, &OosFacts { left_shieldstun: true, shielding: false, ..OosFacts::default() });
    assert_eq!(cpu.fighters[key.index()].counters.count(CounterId::Reaction), 0);
}

fn cpu05_oos_action_table() {
    let facts = OosFacts { cont_escape: true, ..OosFacts::default() };
    let decide = |action: Action, state: usize| {
        let (mut cpu, key) = cpu_with(&[mode(state), ("mash_state", actions(&[usize::from(action.0)])), ("mash_triggers", shieldstun_triggers())]);
        frame(&mut cpu, key, WAIT, WAIT);
        let out = hit_and_decide(&mut cpu, key, facts);
        (out, cpu.fighters[key.index()].shield.suspended)
    };
    // Spot dodge and rolls: a status change on the script object, with CONT_ESCAPE enabled.
    assert_eq!(decide(Action::SPOT_DODGE, 2).0.escape_status, Some(ESCAPE));
    assert_eq!(decide(Action::ROLL_F, 2).0.escape_status, Some(ESCAPE_F));
    assert_eq!(decide(Action::ROLL_B, 2).0.escape_status, Some(ESCAPE_B));
    // Without CONT_ESCAPE a dodge falls through to the drop rule.
    let (mut cpu, key) = cpu_with(&[mode(2), ("mash_state", actions(&[3])), ("mash_triggers", shieldstun_triggers())]);
    frame(&mut cpu, key, WAIT, WAIT);
    let out = hit_and_decide(&mut cpu, key, OosFacts::default());
    assert_eq!(out.escape_status, None);
    assert!(cpu.fighters[key.index()].shield.suspended);
    // Jump, aerials, up special and up smash: nothing here (the native release does them), not suspended.
    for action in [Action::JUMP, Action::NAIR, Action::UAIR, Action::UP_B, Action::U_SMASH] {
        let (out, suspended) = decide(action, 2);
        assert_eq!((out.escape_status, suspended), (None, false), "{action:?}");
    }
    // Air dodge suspends the shield when it is being held (Hold / Infinite / Constant), not with None.
    assert!(decide(Action::AIR_DODGE, 2).1);
    assert!(!decide(Action::AIR_DODGE, 0).1);
    // Shield stays; grab works from the shield; every other action needs it dropped.
    assert!(!decide(Action::SHIELD, 2).1);
    assert!(!decide(Action::GRAB, 2).1);
    for action in [Action::JAB, Action::F_SMASH, Action::F_TILT, Action::DASH, Action::NEUTRAL_B] {
        assert!(decide(action, 2).1, "{action:?} needs the shield dropped");
    }
}

fn cpu05_shieldstun_override() {
    // The shieldstun override replaces mash_state; without it the trigger buffers mash_state; with
    // neither nothing is buffered.
    let (mut cpu, key) = cpu_with(&[
        mode(2),
        ("shieldstun_override", actions(&[6])),
        ("mash_state", actions(&[18])),
        ("mash_triggers", shieldstun_triggers()),
    ]);
    frame(&mut cpu, key, WAIT, WAIT);
    hit_and_decide(&mut cpu, key, OosFacts::default());
    assert_eq!(cpu.fighters[key.index()].mash.queue.current().map(|a| a.0), Some(6));
    let (mut cpu, key) = cpu_with(&[mode(2), ("mash_state", actions(&[18])), ("mash_triggers", shieldstun_triggers())]);
    frame(&mut cpu, key, WAIT, WAIT);
    hit_and_decide(&mut cpu, key, OosFacts::default());
    assert_eq!(cpu.fighters[key.index()].mash.queue.current().map(|a| a.0), Some(18));
    let (mut cpu, key) = cpu_with(&[mode(2), ("mash_state", actions(&[18]))]);
    let cpu_settings = CpuSettings { mash_triggers: 0, ..cpu.settings };
    cpu.settings = cpu_settings;
    frame(&mut cpu, key, WAIT, WAIT);
    hit_and_decide(&mut cpu, key, OosFacts::default());
    assert!(cpu.fighters[key.index()].mash.queue.is_empty());
}

fn cpu05_shield_tilt() {
    // Up: the vector (0, 1) whichever side the player stands on; empty is the native stick.
    let (mut cpu, key) = cpu_with(&[("shield_tilt", {
        let mut row = vec![0u8; 11];
        row[2] = 1;
        row
    })]);
    // A fresh body has the tilt re-read pending (as `Fighter::new` leaves it); the first frame takes it.
    cpu.fighters[key.index()].pending = reroll::SHIELD_TILT;
    frame(&mut cpu, key, WAIT, WAIT);
    assert_eq!(cpu.fighters[key.index()].shield.tilt, Some(2));
    let v = cpu.shield_tilt_stick(key, 10.0, 0.0).unwrap();
    assert!((v.0).abs() < 1e-5 && (v.1 - 1.0).abs() < 1e-5);
    let (mut cpu, key) = cpu_with(&[]);
    frame(&mut cpu, key, WAIT, WAIT);
    assert_eq!(cpu.shield_tilt_stick(key, 10.0, 0.0), None);
}

fn cpu05_shield_tilt_applies_next_frame() {
    // Q18 A: no reset is needed; the menu commit leaves the re-read pending and the next frame takes it.
    let (mut cpu, key) = cpu_with(&[]);
    frame(&mut cpu, key, WAIT, WAIT);
    assert_eq!(cpu.fighters[key.index()].shield.tilt, None);
    let mut row = vec![0u8; 11];
    row[6] = 1; // Down
    cpu.publish(settings(&[("shield_tilt", row)]), reroll::SHIELD_TILT);
    assert_eq!(cpu.fighters[key.index()].shield.tilt, None, "nothing changes before the next CPU frame");
    frame(&mut cpu, key, WAIT, WAIT);
    assert_eq!(cpu.fighters[key.index()].shield.tilt, Some(6));
    assert_eq!(cpu.fighters[key.index()].pending & reroll::SHIELD_TILT, 0);
    // The reset bundle re-reads it too.
    cpu.fighters[key.index()].shield.tilt = None;
    cpu.hub_step(HubStep::ShieldTilt);
    assert_eq!(cpu.fighters[key.index()].shield.tilt, Some(6));
}

fn cpu05_shield_tilt_opponent_relative() {
    // B7: In is toward the opponent and Out away on both sides; Left and Right are absolute.
    let tilt = |dir: usize, player_x: f32| {
        let mut row = vec![0u8; 11];
        row[dir] = 1;
        let (mut cpu, key) = cpu_with(&[("shield_tilt", row)]);
        cpu.fighters[key.index()].pending = reroll::SHIELD_TILT;
        frame(&mut cpu, key, WAIT, WAIT);
        cpu.shield_tilt_stick(key, player_x, 0.0).unwrap()
    };
    let near = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).abs() < 1e-5 && (a.1 - b.1).abs() < 1e-5;
    assert!(near(tilt(4, -8.0), (-1.0, 0.0)), "In with the player on the left points left");
    assert!(near(tilt(4, 8.0), (1.0, 0.0)), "In with the player on the right points right");
    assert!(near(tilt(0, -8.0), (1.0, 0.0)) && near(tilt(0, 8.0), (-1.0, 0.0)));
    assert!(near(tilt(9, 8.0), (-1.0, 0.0)) && near(tilt(10, -8.0), (1.0, 0.0)), "Left and Right never mirror");
}

fn cpu05_oos_offset_range_pick() {
    // N3: a set range is picked uniformly where the checklist is picked today (at each offset reset).
    let s = settings(&[("oos_offset_range", vec![1, 3])]);
    assert_eq!(s.oos_offset, DelayRow::Range { min: 1, max: 3, step: 1 });
    let (mut cpu, key) = cpu_with(&[("oos_offset_range", vec![1, 3])]);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..200 {
        frame(&mut cpu, key, WAIT, WAIT);
        seen.insert(cpu.fighters[key.index()].shield.offset);
    }
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), vec![2, 3, 4], "the offset is the pick plus one");
}

fn cpu05_reaction_time_range_pick() {
    let (mut cpu, key) = cpu_with(&[("reaction_time_range", vec![5, 8])]);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..200 {
        frame(&mut cpu, key, GUARD_DAMAGE, GUARD);
        seen.insert(cpu.fighters[key.index()].shield.reaction);
        frame(&mut cpu, key, GUARD, GUARD_DAMAGE);
    }
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), vec![5, 6, 7, 8]);
}

fn cpu05_per_fighter_state() {
    // B6: Popo's offset and Hold flag do not leak into Nana's.
    let (mut cpu, popo) = cpu_with(&[mode(2), ("oos_offset", delay(&[4]))]);
    let nana = FighterKey::new(1, true).unwrap();
    cpu.fighters[nana.index()].pending = 0;
    frame(&mut cpu, popo, WAIT, WAIT);
    cpu.out_of_shield(popo, &OosFacts { left_shieldstun: true, shielding: true, ..OosFacts::default() });
    assert_eq!(cpu.fighters[popo.index()].shield.offset, 5);
    assert!(cpu.fighters[popo.index()].shield.decay_enabled);
    assert_eq!(cpu.fighters[nana.index()].shield.offset, 0);
    assert!(!cpu.fighters[nana.index()].shield.decay_enabled);
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("cpu05_should_hold_shield", cpu05_should_hold_shield as fn()),
        ("cpu05_decay_override_rules", cpu05_decay_override_rules as fn()),
        ("cpu05_damage_mul_restore", cpu05_damage_mul_restore as fn()),
        ("cpu05_oos_offset_count", cpu05_oos_offset_count as fn()),
        ("cpu05_reaction_time", cpu05_reaction_time as fn()),
        ("cpu05_oos_action_table", cpu05_oos_action_table as fn()),
        ("cpu05_shieldstun_override", cpu05_shieldstun_override as fn()),
        ("cpu05_shield_tilt", cpu05_shield_tilt as fn()),
        ("cpu05_shield_tilt_applies_next_frame", cpu05_shield_tilt_applies_next_frame as fn()),
        ("cpu05_shield_tilt_opponent_relative", cpu05_shield_tilt_opponent_relative as fn()),
        ("cpu05_oos_offset_range_pick", cpu05_oos_offset_range_pick as fn()),
        ("cpu05_reaction_time_range_pick", cpu05_reaction_time_range_pick as fn()),
        ("cpu05_per_fighter_state", cpu05_per_fighter_state as fn()),
    ]);
}
