//! SPEC-state-01 §5: the save / load machine, the placement rules and the hook predicates (host tests;
//! the planned names are the spec's §9 rows).

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::cpu::rng::Rng;
use model::cpu::settings::Weights;
use model::state::file::SLOTS;
use model::state::hooks::{self, DeathParam};
use model::state::machine::{FrameIn, Machine, Out, Placing, Request, Role, SituationIds, StatusNow};
use model::state::place;
use model::state::record::{Marker, SavedState};
use model::state::settings::{DamageMode, Mirroring, StateSettings};

const SITUATIONS: SituationIds = SituationIds { ground: 0, air: 2, cliff: 1 };
const GROUND: i32 = 0;
const AIR: i32 = 2;
const CLIFF: i32 = 1;

fn input(entry: i32) -> FrameIn {
    FrameIn {
        entry,
        nana: false,
        popo: false,
        kind: 7,
        dead: false,
        status: StatusNow::Other,
        prev_rebirth: false,
        standby: false,
        stage_id: 0x40,
        situation: SITUATIONS,
        save_chord: false,
        load_chord: false,
        kirby: false,
        pokemon: false,
        buffable: false,
    }
}

fn rng() -> Rng {
    Rng::new(7)
}

fn machine_with(player: SavedState, cpu: SavedState, slot: usize) -> Machine {
    let mut machine = Machine::default();
    machine.file.player[slot] = player;
    machine.file.cpu[slot] = cpu;
    machine
}

fn saved(x: f32, y: f32, lr: f32, situation: i32, kind: i32) -> SavedState {
    SavedState { x, y, lr, situation_kind: situation, fighter_kind: kind, percent: 42.0, ..SavedState::never_saved() }
}

fn settings() -> StateSettings {
    StateSettings::default()
}

/// Request a load from the player's callback.
fn request(machine: &mut Machine, settings: &StateSettings, rng: &mut Rng) -> Out {
    machine.frame(&FrameIn { load_chord: true, ..input(0) }, settings, rng)
}

/// Drive both fighters through a full load on the ground and return the outputs of the final frames.
fn run_ground_load(machine: &mut Machine, settings: &StateSettings, rng: &mut Rng) -> (Out, Out) {
    request(machine, settings, rng);
    for entry in [0, 1] {
        let kill = machine.frame(&input(entry), settings, rng);
        assert!(kill.force_dead);
    }
    // Both respawn (not dead any more): cleanup, PosMove, the handshake asks for WAIT.
    for entry in [0, 1] {
        let out = machine.frame(&input(entry), settings, rng);
        assert!(out.cleanup);
        assert_eq!(out.placing.unwrap().request, Some(Request::Wait));
        assert!(out.done.is_none());
    }
    let player = machine.frame(&FrameIn { status: StatusNow::Wait, ..input(0) }, settings, rng);
    let cpu = machine.frame(&FrameIn { status: StatusNow::Wait, ..input(1) }, settings, rng);
    (player, cpu)
}

/// SPEC-state-05 §5.1 step 1: the Pokemon Trainer pre-death rule is asked on KillPlayer and on every
/// WaitForAlive frame for a Pokemon only, and the trainer's switch flag is raised once it is placed.
#[test]
fn state05_pt_predeath() {
    let mut machine = Machine::default();
    let mut rng = rng();
    let pokemon = |entry: i32| FrameIn { pokemon: true, ..input(entry) };
    request(&mut machine, &settings(), &mut rng);
    let kill = machine.frame(&pokemon(0), &settings(), &mut rng);
    assert!(kill.pokemon_predeath && kill.force_dead);
    // Still dead: WaitForAlive repeats the rule.
    let waiting = machine.frame(&FrameIn { dead: true, ..pokemon(0) }, &settings(), &mut rng);
    assert!(waiting.pokemon_predeath && !waiting.cleanup);
    // A non-Pokemon never gets it.
    let other = machine.frame(&input(1), &settings(), &mut rng);
    assert!(!other.pokemon_predeath);
    // Respawned and in WAIT: placed, the trainer may switch, and the record waits for the switch.
    let respawned = machine.frame(&pokemon(0), &settings(), &mut rng);
    assert!(respawned.cleanup);
    let placed = machine.frame(&FrameIn { status: StatusNow::Wait, ..pokemon(0) }, &settings(), &mut rng);
    let done = placed.done.expect("placed");
    assert!(done.pokemon_switch);
    assert_eq!(done.next, Marker::WaitForPokemonSwitch);
    let wait = machine.frame(&FrameIn { status: StatusNow::Wait, ..pokemon(0) }, &settings(), &mut rng);
    assert_eq!(wait.wait, Some(Marker::WaitForPokemonSwitch));
}

#[test]
fn state01_load_request_marks_both() {
    let mut machine = Machine::default();
    let mut rng = rng();
    let out = request(&mut machine, &settings(), &mut rng);
    assert!(out.stop_playback);
    assert_eq!((machine.marker(Role::Player, 0), machine.marker(Role::Cpu, 0)), (Marker::KillPlayer, Marker::KillPlayer));
    assert!(machine.is_loading() && machine.is_killing());
    // Nothing else happens on the request frame, and the request is read only by a non-Nana fighter.
    assert_eq!(out.placing, None);
    let mut other = Machine::default();
    let nana = other.frame(&FrameIn { load_chord: true, nana: true, ..input(0) }, &settings(), &mut rng);
    assert!(!nana.stop_playback && !other.is_loading());
}

#[test]
fn state01_sequence_ground() {
    let mut machine = machine_with(saved(10.0, 0.5, -1.0, GROUND, 7), saved(-20.0, 0.0, 1.0, GROUND, 8), 0);
    let mut rng = rng();
    let (player, cpu) = run_ground_load(&mut machine, &settings(), &mut rng);
    assert_eq!(player.placing, Some(Placing { x: 10.0, y: 0.5, lr: -1.0, grounded: true, request: None }));
    assert!(player.run_hub && player.done.is_some());
    assert!(!player.done.as_ref().unwrap().reset_sound, "only the CPU plays the reset sound");
    assert!(cpu.done.as_ref().unwrap().reset_sound);
    assert_eq!(cpu.placing.unwrap().x, -20.0);
    assert!(!machine.is_loading());
    assert_eq!(machine.marker(Role::Player, 0), Marker::NoAction);
    // The recorder is told once, on the frame the second record finishes.
}

#[test]
fn state01_sequence_air() {
    let mut machine = machine_with(saved(0.0, 30.0, 1.0, AIR, 7), saved(5.0, 40.0, 1.0, AIR, 7), 0);
    let mut rng = rng();
    let s = settings();
    request(&mut machine, &s, &mut rng);
    machine.frame(&input(0), &s, &mut rng);
    let placing = machine.frame(&input(0), &s, &mut rng).placing.unwrap();
    assert_eq!((placing.grounded, placing.request), (false, Some(Request::Fall)));
    let placed = machine.frame(&FrameIn { status: StatusNow::Fall, ..input(0) }, &s, &mut rng);
    assert!(placed.done.is_some() && placed.placing.unwrap().request.is_none());
}

#[test]
fn state01_a_cliff_state_ends_in_fall_or_a_ledge_status_and_raises_by_kind() {
    let mut machine = machine_with(saved(0.0, 10.0, 1.0, CLIFF, 1), saved(0.0, 10.0, 1.0, CLIFF, 1), 0);
    let mut rng = rng();
    let s = settings();
    request(&mut machine, &s, &mut rng);
    machine.frame(&FrameIn { kind: 1, ..input(0) }, &s, &mut rng);
    let first = machine.frame(&FrameIn { kind: 1, ..input(0) }, &s, &mut rng).placing.unwrap();
    assert_eq!((first.y, first.request), (16.0, Some(Request::Fall)), "Donkey Kong is raised by 6");
    let caught = machine.frame(&FrameIn { kind: 1, status: StatusNow::CliffCatch, ..input(0) }, &s, &mut rng);
    assert!(caught.done.is_some(), "a ledge status already counts as placed");
    for (kind, raise) in [(1, 6.0), (0x2a, 6.0), (0x45, 6.0), (3, 4.0), (4, 4.0), (0x19, 4.0), (0x55, 4.0), (0xf, 2.0), (0x5b, 2.0), (0x22, 2.0), (7, 0.0)] {
        assert_eq!(place::ledge_raise(kind), raise, "kind {kind:#x}");
    }
}

#[test]
fn state01_mirror_sign_rules() {
    let mut rng = rng();
    assert_eq!(place::next_mirror_sign(Mirroring::None, -1.0, &mut rng), 1.0);
    assert_eq!(place::next_mirror_sign(Mirroring::Alternate, 1.0, &mut rng), -1.0);
    assert_eq!(place::next_mirror_sign(Mirroring::Alternate, -1.0, &mut rng), 1.0);
    let random: Vec<f32> = (0..200).map(|_| place::next_mirror_sign(Mirroring::Random, 1.0, &mut rng)).collect();
    let minus = random.iter().filter(|s| **s < 0.0).count();
    assert!((60..140).contains(&minus), "about half: {minus}");
    // Alternate flips each load; a Save resets it to +1; the placement mirrors x and lr.
    let s = StateSettings { mirroring: Mirroring::Alternate, ..settings() };
    let mut machine = machine_with(saved(10.0, 0.0, 1.0, GROUND, 7), saved(-4.0, 0.0, -1.0, GROUND, 7), 0);
    let (first, _) = run_ground_load(&mut machine, &s, &mut rng);
    assert_eq!((first.placing.unwrap().x, first.placing.unwrap().lr), (-10.0, -1.0));
    let (second, _) = run_ground_load(&mut machine, &s, &mut rng);
    assert_eq!((second.placing.unwrap().x, second.placing.unwrap().lr), (10.0, 1.0));
    machine.frame(&FrameIn { save_chord: true, ..input(0) }, &s, &mut rng);
    assert_eq!(machine.mirror_sign, 1.0);
}

#[test]
fn state01_stage_offset_table() {
    assert_eq!(place::stage_offset(0x5f), 1.195);
    assert_eq!(place::stage_offset(0x101), 1.448);
    assert_eq!(place::stage_offset(0x56), -1.053);
    assert_eq!(place::stage_offset(0x40), 0.0);
    assert_eq!(place::target_x(10.0, 1.0, 0x5f), 10.0);
    assert!((place::target_x(10.0, -1.0, 0x5f) - (-(10.0 - 1.195))).abs() < 1e-6);
    assert_eq!(place::target_x(10.0, -1.0, 0x40), -10.0);
}

#[test]
fn state01_damage_modes() {
    let mut rng = rng();
    assert_eq!(place::load_damage(DamageMode::Default, 42.0, (0, 150), &mut rng), None);
    assert_eq!(place::load_damage(DamageMode::SaveState, 42.0, (0, 150), &mut rng), Some(42.0));
    for _ in 0..200 {
        let value = place::load_damage(DamageMode::Random, 42.0, (30, 60), &mut rng).unwrap();
        assert!((30.0..=60.0).contains(&value), "{value}");
        // Either order of the stored ends gives the same range (OQ 6).
        let swapped = place::load_damage(DamageMode::Random, 42.0, (60, 30), &mut rng).unwrap();
        assert!((30.0..=60.0).contains(&swapped));
    }
    // Per role: the player's mode and range for entry 0, the CPU's for entry 1.
    let s = StateSettings {
        damage_player: DamageMode::SaveState,
        damage_cpu: DamageMode::Random,
        limits_cpu: (100, 100),
        ..settings()
    };
    let mut machine = machine_with(saved(0.0, 0.0, 1.0, GROUND, 7), saved(0.0, 0.0, 1.0, GROUND, 7), 0);
    let (player, cpu) = run_ground_load(&mut machine, &s, &mut rng);
    assert_eq!(player.done.unwrap().damage, Some(42.0));
    assert_eq!(cpu.done.unwrap().damage, Some(100.0));
}

#[test]
fn state01_random_slot_latch() {
    let mut rng = rng();
    let s = StateSettings { randomize: Weights([0, 0, 0, 1, 0]), slot: 1, ..settings() };
    let mut machine = Machine::default();
    machine.file.player[3] = saved(1.0, 0.0, 1.0, GROUND, 7);
    machine.file.cpu[3] = saved(2.0, 0.0, 1.0, GROUND, 7);
    request(&mut machine, &s, &mut rng);
    assert_eq!(machine.random_slot, 3);
    assert_eq!(machine.marker(Role::Player, 3), Marker::KillPlayer);
    assert_eq!(machine.marker(Role::Player, 1), Marker::NoAction, "Save State Slot is not used");
    // The slot is latched: changing the menu mid-load does not move the load.
    let moved = StateSettings { slot: 4, randomize: Weights([1, 0, 0, 0, 0]), ..s };
    assert_eq!(machine.current_slot(&moved), 3);
    let out = machine.frame(&input(0), &moved, &mut rng);
    assert!(out.force_dead);
    // Idle frames draw nothing: the generator is not advanced by a frame with no request.
    let mut idle = Machine::default();
    let mut idle_rng = Rng::new(5);
    let before = idle_rng.clone();
    for _ in 0..10 {
        idle.frame(&input(0), &s, &mut idle_rng);
    }
    assert_eq!(idle_rng, before);
    // The Save chord always saves into Save State Slot, never into the random slot.
    let out = idle.frame(&FrameIn { save_chord: true, ..input(0) }, &s, &mut idle_rng);
    assert_eq!(out.saved_notice, Some(1));
}

#[test]
fn state01_autoload_on_death() {
    let mut rng = rng();
    let on = StateSettings { autoload: true, ..settings() };
    let mut machine = Machine::default();
    machine.frame(&input(1), &on, &mut rng);
    assert!(!machine.is_loading(), "alive: nothing");
    let out = machine.frame(&FrameIn { dead: true, ..input(1) }, &on, &mut rng);
    assert!(out.stop_playback && machine.is_loading(), "a dead CPU loads");
    let mut off = Machine::default();
    off.frame(&FrameIn { dead: true, ..input(0) }, &settings(), &mut rng);
    assert!(!off.is_loading(), "Auto Save States Off never loads on death");
    // A dead Nana never requests a load.
    let mut nana = Machine::default();
    nana.frame(&FrameIn { dead: true, nana: true, ..input(0) }, &on, &mut rng);
    assert!(!nana.is_loading());
}

#[test]
fn state01_ignore_request_mid_load() {
    let mut rng = rng();
    let s = StateSettings { mirroring: Mirroring::Alternate, ..settings() };
    let mut machine = Machine::default();
    assert!(request(&mut machine, &s, &mut rng).stop_playback);
    let sign = machine.mirror_sign;
    let again = request(&mut machine, &s, &mut rng);
    assert!(!again.stop_playback, "ignored");
    assert_eq!(machine.mirror_sign, sign, "the sign is fixed at the request");
}

#[test]
fn state01_disable_cancels() {
    let mut rng = rng();
    let mut machine = Machine::default();
    request(&mut machine, &settings(), &mut rng);
    machine.frame(&input(0), &settings(), &mut rng);
    assert!(machine.is_loading());
    machine.cancel_all();
    assert!(!machine.is_loading());
    assert_eq!(machine.marker(Role::Player, 0), Marker::NoAction);
    // With Enable Off the machine does nothing at all.
    let off = StateSettings { enable: false, ..settings() };
    let out = machine.frame(&FrameIn { load_chord: true, ..input(0) }, &off, &mut rng);
    assert_eq!(out, Out::default());
}

#[test]
fn state01_nana_handoff() {
    let mut rng = rng();
    let s = settings();
    let mut machine = machine_with(saved(10.0, 0.0, 1.0, GROUND, 0x4b), saved(1.0, 0.0, 1.0, GROUND, 7), 0);
    request(&mut machine, &s, &mut rng);
    machine.frame(&FrameIn { popo: true, kind: 0x4b, ..input(0) }, &s, &mut rng);
    machine.frame(&input(1), &s, &mut rng);
    // Popo out of REBIRTH: placed, then the record is handed to Nana.
    let popo = FrameIn { popo: true, kind: 0x4b, prev_rebirth: true, ..input(0) };
    machine.frame(&popo, &s, &mut rng);
    let placed = machine.frame(&FrameIn { status: StatusNow::Wait, ..popo }, &s, &mut rng);
    assert!(placed.nana_next);
    assert_eq!(machine.marker(Role::Player, 0), Marker::NanaPosMove);
    // Popo's callbacks do nothing while the marker is NanaPosMove; Nana in STANDBY waits.
    assert_eq!(machine.frame(&popo, &s, &mut rng).placing, None);
    let standby = FrameIn { nana: true, standby: true, kind: 0x4c, ..input(0) };
    assert_eq!(machine.frame(&standby, &s, &mut rng).placing, None);
    // Nana placed with the same record: no item, her kind never matches the saved Popo kind.
    let nana = FrameIn { nana: true, kind: 0x4c, status: StatusNow::Wait, ..input(0) };
    let out = machine.frame(&nana, &s, &mut rng);
    assert_eq!(out.placing.unwrap().x, 10.0);
    let done = out.done.unwrap();
    assert!(done.item.is_none() && done.charge.is_none());
    assert_eq!(machine.marker(Role::Player, 0), Marker::NoAction);
    // Nana never saves.
    let mut other = Machine::default();
    other.frame(&FrameIn { save_chord: true, ..input(0) }, &s, &mut rng);
    let nana_save = other.frame(&FrameIn { nana: true, ..input(0) }, &s, &mut rng);
    assert_eq!(nana_save.capture, None);
}

/// Solo Popo removed Nana (the runtime then reports `popo: false` for Popo, `states::nana_removed`): out of REBIRTH the load ends at
/// Popo, no `NanaPosMove` marker waits for a second body that does not exist (SPEC-state-01 §5.3 step 9).
#[test]
fn state01_solo_popo_has_no_handoff() {
    let mut rng = rng();
    let s = settings();
    let mut machine = machine_with(saved(10.0, 0.0, 1.0, GROUND, 0x4b), saved(1.0, 0.0, 1.0, GROUND, 7), 0);
    request(&mut machine, &s, &mut rng);
    machine.frame(&FrameIn { kind: 0x4b, ..input(0) }, &s, &mut rng);
    machine.frame(&input(1), &s, &mut rng);
    let solo = FrameIn { popo: false, kind: 0x4b, prev_rebirth: true, ..input(0) };
    machine.frame(&solo, &s, &mut rng);
    let placed = machine.frame(&FrameIn { status: StatusNow::Wait, ..solo }, &s, &mut rng);
    assert!(!placed.nana_next, "no second body to hand the record to");
    assert_eq!(machine.marker(Role::Player, 0), Marker::NoAction);
}

#[test]
fn state01_save_captures_into_the_save_slot_only() {
    let mut rng = rng();
    let s = StateSettings { slot: 2, ..settings() };
    let mut machine = Machine::default();
    let first = machine.frame(&FrameIn { save_chord: true, ..input(0) }, &s, &mut rng);
    assert_eq!((first.saved_notice, first.capture), (Some(2), Some(2)));
    // The CPU's own callback picks the step up from its record marker.
    let second = machine.frame(&input(1), &s, &mut rng);
    assert_eq!(second.capture, Some(2));
    let state = SavedState { x: 3.0, fighter_kind: 7, ..SavedState::never_saved() };
    assert!(!machine.capture(Role::Player, 2, state.clone()) || machine.marker(Role::Cpu, 2) != Marker::Save);
    assert!(machine.capture(Role::Cpu, 2, state), "the file is written once neither record is still Save");
    assert_eq!(machine.record(Role::Player, 2).x, 3.0);
    for slot in [0, 1, 3, 4] {
        assert_eq!(*machine.record(Role::Player, slot), SavedState::never_saved());
    }
    assert_eq!(SLOTS, 5);
}

#[test]
fn state01_playback_once() {
    let mut rng = rng();
    // The recording or playback after a load is package 4's decision (`Recorder::on_state_restored`): the
    // machine only tells the recorder once, when both records are done.
    let s = settings();
    let mut machine = machine_with(saved(0.0, 0.0, 1.0, GROUND, 7), saved(0.0, 0.0, 1.0, GROUND, 7), 0);
    let (player, cpu) = run_ground_load(&mut machine, &s, &mut rng);
    // The recorder is told once per restore, with whether the CPU was saved on the ledge.
    assert_eq!((player.restored, cpu.restored), (None, Some(false)));
    let mut ledge = machine_with(saved(0.0, 0.0, 1.0, GROUND, 7), saved(0.0, 0.0, 1.0, CLIFF, 7), 0);
    request(&mut ledge, &s, &mut rng);
    ledge.frame(&input(0), &s, &mut rng);
    ledge.frame(&input(1), &s, &mut rng);
    ledge.frame(&input(0), &s, &mut rng);
    ledge.frame(&input(1), &s, &mut rng);
    ledge.frame(&FrameIn { status: StatusNow::Wait, ..input(0) }, &s, &mut rng);
    let done = ledge.frame(&FrameIn { status: StatusNow::CliffCatch, ..input(1) }, &s, &mut rng);
    assert_eq!(done.restored, Some(true));
    let idle = machine.frame(&input(1), &s, &mut rng);
    assert_eq!(idle.restored, None, "not again");
}

#[test]
fn state01_param_overrides() {
    let off = settings();
    for (param, expected) in [
        (DeathParam::DeadRebirthWaitFrame, Some(1)),
        (DeathParam::RebirthMoveFrame, Some(0)),
        (DeathParam::RebirthWaitFrame, Some(0)),
        (DeathParam::RebirthInvincibleFrame, Some(0)),
        (DeathParam::RebirthInvincibleAddFrame, Some(0)),
        (DeathParam::RebirthMoveFrameTrainer, None),
        (DeathParam::ChangeFlyFrame, None),
    ] {
        assert_eq!(hooks::param_override(param, &off, false, true), expected, "{param:?} on a natural death, toggle Off");
    }
    assert_eq!(hooks::param_override(DeathParam::RebirthMoveFrameTrainer, &off, true, true), Some(0));
    assert_eq!(hooks::param_override(DeathParam::ChangeFlyFrame, &off, true, true), Some(0));
    assert_eq!(hooks::param_override(DeathParam::DeadRebirthWaitFrame, &off, false, false), None, "inert outside Training");
}

#[test]
fn state01_respawn_on_overrides_only_while_loading() {
    let on = StateSettings { respawn_invincibility: true, ..settings() };
    for param in [
        DeathParam::DeadRebirthWaitFrame,
        DeathParam::RebirthMoveFrame,
        DeathParam::RebirthWaitFrame,
        DeathParam::RebirthInvincibleFrame,
        DeathParam::RebirthInvincibleAddFrame,
    ] {
        assert_eq!(hooks::param_override(param, &on, false, true), None, "{param:?}: a natural death uses the game's own respawn");
        assert!(hooks::param_override(param, &on, true, true).is_some(), "{param:?}: a load still respawns instantly");
    }
}

#[test]
fn state01_respawn_row_greyed_with_autoload_and_load_unaffected_by_the_toggle() {
    // Auto Save States On: the stored toggle is kept but a death loads, so the respawn stays instant.
    let both = StateSettings { respawn_invincibility: true, autoload: true, ..settings() };
    assert!(hooks::instant_respawn(&both, false));
    assert_eq!(hooks::param_override(DeathParam::RebirthInvincibleFrame, &both, false, true), Some(0));
    // The toggle Off is the fork: instant, no invincibility.
    assert!(hooks::instant_respawn(&settings(), false));
}

#[test]
fn state01_suppression_predicates() {
    assert!(hooks::quake_suppressed(true) && !hooks::quake_suppressed(false));
    assert!(hooks::sound_silenced(true) && !hooks::sound_silenced(false));
    assert!(hooks::effect_hidden(true, false));
    assert!(!hooks::effect_hidden(true, true), "a buffing fighter's visuals show");
    assert!(!hooks::effect_hidden(false, false));
    assert!(hooks::score_aura_hidden(true) && !hooks::score_aura_hidden(false));
}

#[test]
fn state01_settings_default_and_from_the_factory_snapshot() {
    use model::settings::Snapshot;
    let factory = StateSettings::from_snapshot(&Snapshot::factory());
    assert_eq!(factory, StateSettings::default());
    assert!(!factory.respawn_invincibility, "a missing key loads Off");
    let mut snapshot = Snapshot::factory();
    snapshot.set("save_state_enable", &[0, 1]).unwrap();
    snapshot.set("save_state_autoload", &[1, 0]).unwrap();
    snapshot.set("respawn_invincibility", &[1, 0]).unwrap();
    snapshot.set("save_state_slot", &[0, 0, 1, 0, 0]).unwrap();
    snapshot.set("save_state_mirroring", &[0, 1, 0]).unwrap();
    snapshot.set("save_damage_cpu", &[0, 0, 1]).unwrap();
    snapshot.set("save_damage_limits_cpu", &[10, 20]).unwrap();
    snapshot.set("randomize_slots", &[1, 0, 0, 0, 3]).unwrap();
    let read = StateSettings::from_snapshot(&snapshot);
    assert!(!read.enable && read.autoload && read.respawn_invincibility);
    assert_eq!((read.slot, read.mirroring, read.damage_cpu, read.limits_cpu), (2, Mirroring::Alternate, DamageMode::Random, (10, 20)));
    assert_eq!(read.randomize, Weights([1, 0, 0, 0, 3]));
}

/// R2-4 (review R2): the loading fact is true from the acceptance of the load (before the first destructive
/// step) to its completion, the restore is reported to the recorder exactly once, and a cancellation clears it.
#[test]
fn r2_4_loading_is_true_from_acceptance_to_completion_and_the_restore_is_reported_once() {
    let mut rng = rng();
    let mut machine = machine_with(saved(0.0, 0.0, 1.0, GROUND, 7), saved(0.0, 0.0, 1.0, GROUND, 7), 0);
    assert!(!machine.is_loading());
    let accepted = request(&mut machine, &settings(), &mut rng);
    assert!(accepted.stop_playback, "the recorder is told to stop at acceptance");
    assert!(machine.is_loading(), "the fact is true as soon as the load is accepted");
    machine.cancel_all();
    assert!(!machine.is_loading(), "a cancellation clears it");

    let mut machine = machine_with(saved(0.0, 0.0, 1.0, GROUND, 7), saved(0.0, 0.0, 1.0, GROUND, 7), 0);
    let (player, cpu) = run_ground_load(&mut machine, &settings(), &mut rng);
    assert!(!machine.is_loading(), "complete");
    let reported = [player.restored, cpu.restored].into_iter().flatten().count();
    assert_eq!(reported, 1, "once per restore");
    for entry in [0, 1] {
        assert_eq!(machine.frame(&input(entry), &settings(), &mut rng).restored, None, "never again");
    }
}