//! SPEC-state-03: the per-fighter `ApplyBuff` step (host tests; the planned names are the spec's §9 rows).

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::cpu::rng::Rng;
use model::state::buff::{Art, BuffFacts, BuffKind, BuffOut, BuffState, Spell, Waft};
use model::state::settings::Buffs;

fn buffs(options: &[u32]) -> Buffs {
    Buffs(options.iter().fold(0, |bits, o| bits | 1 << o))
}

fn run(kind: BuffKind, buffs: Buffs, frames: &[BuffFacts]) -> Vec<BuffOut> {
    let mut state = BuffState::new();
    let mut rng = Rng::new(3);
    frames.iter().map(|facts| state.step(kind, buffs, facts, &mut rng)).collect()
}

fn idle(n: usize) -> Vec<BuffFacts> {
    vec![BuffFacts::default(); n]
}

fn state03_hero_spell_sequence() {
    // Four spells are cast last to first, each time the status leaves SPECIAL_LW_START.
    let all = buffs(&[Buffs::ACCELERATE, Buffs::OOMPH, Buffs::PSYCHE, Buffs::BOUNCE]);
    let mut state = BuffState::new();
    let mut rng = Rng::new(1);
    let mut cast = Vec::new();
    let casting = BuffFacts { hero_cast_now: true, ..BuffFacts::default() };
    let after = BuffFacts { hero_cast_before: true, ..BuffFacts::default() };
    for _ in 0..4 {
        let out = state.step(BuffKind::Hero, all, &BuffFacts::default(), &mut rng);
        cast.push(out.hero_cast.expect("a spell is cast"));
        assert_eq!(out.motion_rate, None);
        // In the status: played at 50x.
        assert_eq!(state.step(BuffKind::Hero, all, &casting, &mut rng).motion_rate, Some(50.0));
        // The status ended: one fewer to cast.
        let _ = state.step(BuffKind::Hero, all, &after, &mut rng);
    }
    // The order is Bounce, Psyche, Oomph, Accelerate (last to first).
    assert_eq!(cast, vec![Spell::Bounce, Spell::Psyche, Spell::Oomph, Spell::Accelerate]);
}

fn state03_unselected_done_immediately() {
    for kind in [BuffKind::Cloud, BuffKind::Joker, BuffKind::LittleMac, BuffKind::Sephiroth, BuffKind::WiiFit, BuffKind::MinMin, BuffKind::Shulk, BuffKind::Wario] {
        let outs = run(kind, Buffs(0), &idle(1));
        assert!(outs[0].done && !outs[0].buffing, "{kind:?}");
    }
}

fn state03_sephiroth_flag_lifecycle() {
    let wing = buffs(&[Buffs::WING]);
    let outs = run(BuffKind::Sephiroth, wing, &[BuffFacts::default(), BuffFacts::default(), BuffFacts { wing_activated: true, ..BuffFacts::default() }]);
    assert!(outs[0].wing_flags_on && !outs[1].wing_flags_on, "raised once");
    assert!(outs[0].buffing && outs[1].buffing && !outs[0].done);
    assert!(outs[2].wing_flags_off && outs[2].done, "cleared once the wing activates");
    // Deselecting mid-buff clears the flags (OQ 4).
    let mut state = BuffState::new();
    let mut rng = Rng::new(1);
    state.step(BuffKind::Sephiroth, wing, &BuffFacts::default(), &mut rng);
    let off = state.step(BuffKind::Sephiroth, Buffs(0), &BuffFacts::default(), &mut rng);
    assert!(off.wing_flags_off && off.done);
}

fn state03_wario_waft_level() {
    for (option, level) in [(Buffs::WAFT_MINI, Waft::Mini), (Buffs::WAFT_HALF, Waft::Half), (Buffs::WAFT_FULL, Waft::Full)] {
        let outs = run(BuffKind::Wario, buffs(&[option]), &idle(1));
        assert_eq!(outs[0].waft, Some(level));
        assert!(outs[0].done);
    }
    // One level is drawn among the selected, once.
    let any = buffs(&[Buffs::WAFT_MINI, Buffs::WAFT_FULL]);
    for seed in 0..20 {
        let mut state = BuffState::new();
        let out = state.step(BuffKind::Wario, any, &BuffFacts::default(), &mut Rng::new(seed));
        assert!(matches!(out.waft, Some(Waft::Mini) | Some(Waft::Full)));
    }
}

fn state03_shulk_art_sequence() {
    let one = buffs(&[Buffs::MONAD_SPEED]);
    let in_action = BuffFacts { shulk_action_now: true, ..BuffFacts::default() };
    let after = BuffFacts { shulk_action_before: true, ..BuffFacts::default() };
    let outs = run(BuffKind::Shulk, one, &[BuffFacts::default(), in_action, in_action, after, after, after, after]);
    assert_eq!(outs[0].shulk_write, Some(Art::Speed));
    assert_eq!(outs[1].motion_rate, Some(40.0));
    // The art ended: a 3-frame tail for the voice line.
    assert_eq!(outs.iter().position(|o| o.done), Some(6));
    // One art is drawn among the selected, once per load.
    let many = buffs(&[Buffs::MONAD_JUMP, Buffs::MONAD_SMASH]);
    let mut state = BuffState::new();
    let mut rng = Rng::new(9);
    let first = state.step(BuffKind::Shulk, many, &BuffFacts::default(), &mut rng).shulk_write;
    let second = state.step(BuffKind::Shulk, many, &BuffFacts::default(), &mut rng).shulk_write;
    assert!(first.is_some() && first == second, "the draw is made once");
}

fn state03_wiifit_breathing_no_heal() {
    let b = buffs(&[Buffs::BREATHING]);
    let success = BuffFacts { wii_success_now: true, ..BuffFacts::default() };
    let after = BuffFacts { wii_success_before: true, ..BuffFacts::default() };
    let outs = run(BuffKind::WiiFit, b, &[BuffFacts::default(), success, after]);
    assert!(outs[0].wii_force && outs[0].buffing, "the flag is what the heal hook reads");
    assert_eq!(outs[1].motion_rate, Some(40.0));
    assert!(outs[2].done && !outs[2].buffing);
}

fn state03_minmin_sequence() {
    let outs = run(BuffKind::MinMin, buffs(&[Buffs::POWER_DRAGON]), &idle(2));
    assert!(outs[0].minmin && outs[0].done);
    assert!(!outs[0].buffing, "upstream never sets the flag for Min Min");
}

fn state03_bookkeeping_by_record_and_clear() {
    // Two records buff independently (Hero vs Shulk), each with its own counters.
    let mut player = BuffState::new();
    let mut cpu = BuffState::new();
    let mut rng = Rng::new(2);
    player.step(BuffKind::Hero, buffs(&[Buffs::OOMPH]), &BuffFacts::default(), &mut rng);
    cpu.step(BuffKind::Cloud, buffs(&[Buffs::LIMIT]), &BuffFacts::default(), &mut rng);
    assert!(player.buffing() && cpu.buffing());
    // The delay is not shared: the CPU finishes its tail while the player has not started one.
    cpu.step(BuffKind::Cloud, buffs(&[Buffs::LIMIT]), &BuffFacts::default(), &mut rng);
    let done = cpu.step(BuffKind::Cloud, buffs(&[Buffs::LIMIT]), &BuffFacts::default(), &mut rng).done;
    assert!(done && player.buffing());
    // Session end clears the flag and the counters.
    player.clear();
    assert!(!player.buffing());
    assert_eq!(player, BuffState::new());
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("state03_hero_spell_sequence", state03_hero_spell_sequence as fn()),
        ("state03_unselected_done_immediately", state03_unselected_done_immediately as fn()),
        ("state03_sephiroth_flag_lifecycle", state03_sephiroth_flag_lifecycle as fn()),
        ("state03_wario_waft_level", state03_wario_waft_level as fn()),
        ("state03_shulk_art_sequence", state03_shulk_art_sequence as fn()),
        ("state03_wiifit_breathing_no_heal", state03_wiifit_breathing_no_heal as fn()),
        ("state03_minmin_sequence", state03_minmin_sequence as fn()),
        ("state03_bookkeeping_by_record_and_clear", state03_bookkeeping_by_record_and_clear as fn()),
    ]);
}
