//! Package 2 core (SPEC-cpu-01 shared parts, SPEC-cpu-07 counters, decisions B4, B5/N4, B6, Q18 A,
//! B11): the constant table, the plugin's own generator and picks, the CPU settings value, the CPU
//! role rule, the per-body state, the frame counters and the mash queue. Test names are the specs'
//! §9 rows where one exists.

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use model::cpu::counter::{CounterId, Counters};
use model::cpu::facts::FrameFacts;
use model::cpu::hash::{hash40, lua_bind_hash, xxh64, SEED};
use model::hash40::crc32;
use model::cpu::ids::{self, Table, C, UNRESOLVED};
use model::cpu::mash::{Action, BufferContext, Buffered, Mash};
use model::cpu::playback::{covered_setting_applies, PlaybackState};
use model::cpu::rng::Rng;
use model::cpu::role::{is_cpu_role, FighterKey, RoleTable, Scene, LOBBY_CPU_ENTRY};
use model::cpu::settings::{CpuSettings, DelayRow, Strength, Weights};
use model::cpu::state::Cpu;
use model::cpu::status::{self, Situation};
use model::lra_reset::HubStep;
use model::menu::reroll;
use model::settings::Snapshot;

// ---- a counting allocator, per thread, for the no-allocation rows ---------------------------------

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|c| c.set(c.get() + 1));
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|c| c.set(c.get() + 1));
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn allocations() -> usize {
    ALLOCATIONS.with(Cell::get)
}

fn rng() -> Rng {
    Rng::new(0x1234_5678_9abc_def0)
}

/// A constant table with consecutive synthetic values in declaration order (tests only; the real
/// values come from the game at run time).
fn synthetic_table() -> Table {
    let all: Vec<(C, i32)> = (0..ids::COUNT).map(|i| (unsafe_c(i), 100 + i as i32)).collect();
    Table::with(&all)
}

fn unsafe_c(i: usize) -> C {
    // `C` is a fieldless `repr(u16)` enum numbered from 0 in declaration order.
    assert!(i < ids::COUNT);
    // SAFETY: every value below COUNT is a discriminant of `C`.
    unsafe { std::mem::transmute::<u16, C>(i as u16) }
}

// ---- the game's constant table --------------------------------------------------------------------

#[test]
fn cpu_ids_hash_matches_the_published_vectors() {
    // Vectors of the third-party MIT crate `lua_bind_hash` 1.0.1 (its own unit test).
    assert_eq!(xxh64(b"password", SEED), 0xe58325ff3537c13a);
    assert_eq!(xxh64(b"password1234", SEED), 0xa26a67b576fe1e83);
    assert_eq!(lua_bind_hash("FIGHTER_STATUS_DAMAGE_WORK_FLOAT_VECOR_CORRECT_STICK_X"), 0xa4d50a730e36970e);
    // Standard XXH64 with seed 0 (the reference algorithm): the empty string.
    assert_eq!(xxh64(b"", 0), 0xef46db3751d8e999);
}

#[test]
fn cpu_ids_hash40_is_the_crc_and_the_length() {
    // CRC-32 check value of the standard test string; Hash40 puts the length above the CRC.
    assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    assert_eq!(hash40("hip") >> 32, 3);
    assert_eq!(hash40("sys_clatter") >> 32, 11);
    assert_eq!(hash40("hip") as u32, crc32(b"hip"));
    assert_ne!(hash40("hip"), hash40("sys_clatter"));
}

#[test]
fn cpu_ids_names_are_unique_and_hash_apart() {
    let mut names = std::collections::BTreeSet::new();
    let mut hashes = std::collections::BTreeSet::new();
    for name in ids::NAMES {
        assert!(names.insert(*name), "duplicate {name}");
        assert!(hashes.insert(lua_bind_hash(name)), "hash clash for {name}");
        assert!(name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'), "{name}");
    }
    assert_eq!(ids::NAMES.len(), ids::COUNT);
    assert_eq!(ids::CLATTER_STATUSES.len(), 43);
}

#[test]
fn cpu_ids_resolve_asks_by_hash_and_lists_the_missing() {
    let known = lua_bind_hash("FIGHTER_STATUS_KIND_WAIT");
    let mut table = Table::unresolved();
    let missing = table.resolve(|key| (key == known).then_some(0));
    assert_eq!(missing, ids::COUNT - 1);
    assert!(table.is(C::StatusWait, 0));
    assert!(!table.resolved(C::StatusDamage));
    assert_eq!(table.missing().count(), ids::COUNT - 1);
    assert!(!table.missing().any(|n| n == "FIGHTER_STATUS_KIND_WAIT"));
    // An unresolved constant matches no status, whatever the status.
    assert!(!table.is(C::StatusDamage, UNRESOLVED));
    assert!(!status::in_hitstun(&table, 5));
}

#[test]
fn cpu01_situation_predicates() {
    // Synthetic numbering: ranges follow the declaration order of the table, so a status is "in a
    // range" exactly when its constants say so; the predicates are the spec's definitions.
    let t = Table::with(&[
        (C::StatusDamage, 10),
        (C::StatusDamageFly, 12),
        (C::StatusDamageFall, 14),
        (C::StatusTreadDamage, 20),
        (C::StatusTreadFall, 22),
        (C::StatusGuardOn, 30),
        (C::StatusGuard, 31),
        (C::StatusGuardOff, 32),
        (C::StatusGuardDamage, 33),
        (C::StatusDownStand, 40),
        (C::StatusDownStandAttack, 43),
        (C::StatusPassive, 50),
        (C::StatusPassiveCeil, 53),
        (C::StatusCliffRobbed, 60),
        (C::StatusLanding, 70),
        (C::StatusLandingDamageLight, 73),
        (C::ClatterBury, 80),
        (C::StatusWait, 1),
        (C::SituationGround, 0),
        (C::SituationAir, 1),
        (C::SituationCliff, 2),
    ]);
    // Hitstun is end-exclusive and contains the start of tumble; tumble is inclusive of DAMAGE_FALL.
    assert!(status::in_hitstun(&t, 10) && status::in_hitstun(&t, 13) && !status::in_hitstun(&t, 14));
    assert!(!status::in_tumble(&t, 11) && status::in_tumble(&t, 12) && status::in_tumble(&t, 14) && !status::in_tumble(&t, 15));
    assert!(status::in_footstool(&t, 20) && status::in_footstool(&t, 22) && !status::in_footstool(&t, 23));
    assert!(status::shielding(&t, 30) && status::shielding(&t, 33) && !status::shielding(&t, 34));
    assert!(status::in_shieldstun(&t, 33, 0));
    assert!(status::in_shieldstun(&t, 32, 33) && !status::in_shieldstun(&t, 32, 31));
    assert!(status::in_tech(&t, 41) && status::in_tech(&t, 52) && !status::in_tech(&t, 45));
    assert!(status::in_ledge_trump(&t, 60) && !status::in_ledge_trump(&t, 61));
    assert!(status::in_landing(&t, 71) && !status::in_landing(&t, 74));
    assert!(status::in_clatter(&t, 80) && !status::in_clatter(&t, 81));
    assert!(status::idle(&t, 1));
    assert_eq!(Situation::of(&t, 0), Situation::Ground);
    assert_eq!(Situation::of(&t, 1), Situation::Air);
    assert_eq!(Situation::of(&t, 2), Situation::Cliff);
    assert_eq!(Situation::of(&t, 9), Situation::Other);
}

// ---- the generator and the picks ------------------------------------------------------------------

#[test]
fn cpu01_weighted_pick_distribution() {
    let mut r = rng();
    // Weight 0 never; weights proportional; an empty row picks nothing.
    let row = Weights::<4>([0, 1, 3, 0]);
    let mut counts = [0u32; 4];
    for _ in 0..40_000 {
        counts[row.pick(&mut r).unwrap()] += 1;
    }
    assert_eq!(counts[0] + counts[3], 0);
    let ratio = f64::from(counts[2]) / f64::from(counts[1]);
    assert!((2.8..3.2).contains(&ratio), "ratio {ratio}");
    assert_eq!(Weights::<4>([0; 4]).pick(&mut r), None);
    // A weight-5 option beats a weight-1 option five to one over many picks.
    let row = Weights::<2>([5, 1]);
    let hits = (0..60_000).filter(|_| row.pick(&mut r) == Some(0)).count();
    assert!((48_500..51_500).contains(&hits), "{hits}");
    // The generator's own stream: same seed, same picks; it never reads the game.
    let (mut a, mut b) = (Rng::new(7), Rng::new(7));
    assert!((0..100).all(|_| a.next_u64() == b.next_u64()));
}

#[test]
fn cpu01_weighted_pick_no_alloc() {
    let mut r = rng();
    let row = Weights::<30>::from_row(&[1, 2, 3, 0, 5, 1, 1]);
    let delay = DelayRow::read(&[1, 1, 0, 1], &[], 0, 5);
    let range = DelayRow::Range { min: 10, max: 60, step: 10 };
    let before = allocations();
    let mut sum = 0u64;
    for _ in 0..1000 {
        sum += row.pick(&mut r).unwrap_or(0) as u64 + u64::from(delay.pick(&mut r)) + u64::from(range.pick(&mut r));
    }
    assert!(sum > 0);
    assert_eq!(allocations() - before, 0, "picks must not allocate");
}

#[test]
fn cpu_core_delay_rows_pick_checklists_and_ranges() {
    let mut r = rng();
    assert_eq!(DelayRow::Empty.pick(&mut r), 0);
    assert!(!DelayRow::Empty.is_set());
    // A checklist of {0, 10, 30} frames at step 10 (weights over option indices).
    let set = DelayRow::read(&[1, 1, 0, 1], &[], 0, 10);
    assert!(set.is_set());
    for _ in 0..500 {
        assert!([0u16, 10, 30].contains(&set.pick(&mut r)));
    }
    // A range [min, max] by step is uniform over its step values and hits both ends.
    let range = DelayRow::read(&[], &[2, 5], 0, 10);
    assert_eq!(range, DelayRow::Range { min: 20, max: 50, step: 10 });
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..2000 {
        seen.insert(range.pick(&mut r));
    }
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), vec![20, 30, 40, 50]);
    // min = max is a fixed delay.
    let fixed = DelayRow::read(&[], &[3, 3], 0, 5);
    assert!((0..50).all(|_| fixed.pick(&mut r) == 15));
    // A set range counts as a selection even where the checklist is empty.
    assert!(DelayRow::read(&[0, 0], &[0, 0], 0, 5).is_set());
}

// ---- the settings value ---------------------------------------------------------------------------

#[test]
fn cpu_settings_read_the_factory_rows() {
    let s = CpuSettings::from_snapshot(&Snapshot::factory());
    assert!(s.mash_state.is_empty() && s.follow_up.is_empty());
    // Hitstun, Shieldstun, Tumble, Clatter.
    assert_eq!(s.mash_triggers, 0b1000_1011);
    assert!(s.trigger_on(0) && s.trigger_on(1) && s.trigger_on(3) && s.trigger_on(7) && !s.trigger_on(16));
    assert_eq!(s.sdi_strength, Strength::None);
    assert_eq!(s.clatter_strength, Strength::None);
    assert_eq!(s.ledge_state.0[..5], [1, 1, 1, 1, 0]);
    assert_eq!(s.ledge_delay, DelayRow::Empty);
    assert!(s.stale_dodges && !s.crouch && !s.tech_hide);
    assert!(!s.override_playback, "decision N4: Settings override playback defaults Off");
    assert!(s.playback_mash);
    assert_eq!(s.shield_tilt, None);
    // Full Hop defaults to Yes (index 0 = True), Falling Aerials to No.
    assert_eq!(s.full_hop.0, [1, 0]);
    assert_eq!(s.falling_aerials.0, [0, 1]);
}

#[test]
fn cpu_settings_read_a_changed_snapshot() {
    let mut snap = Snapshot::factory();
    snap.set("sdi_strength", &[0, 0, 1, 0]).unwrap();
    snap.set("sdi_state", &[0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 1]).unwrap();
    snap.set("ledge_delay_range", &[1, 4]).unwrap();
    snap.set("shield_tilt", &[0, 0, 0, 0, 1]).unwrap();
    snap.set("settings_override_playback", &[1, 0]).unwrap();
    snap.set("stale_dodges", &[0, 1]).unwrap();
    let s = CpuSettings::from_snapshot(&snap);
    assert_eq!(s.sdi_strength, Strength::Medium);
    assert_eq!(s.sdi_state.0[2], 2);
    assert_eq!(s.ledge_delay, DelayRow::Range { min: 10, max: 40, step: 10 });
    assert_eq!(s.shield_tilt, Some(4));
    assert!(s.override_playback && !s.stale_dodges);
}

// ---- the CPU role (B4), per-body state (B6) --------------------------------------------------------

#[test]
fn cpu01_cpu_role_only_when_p1_ai() {
    // Entry 0 is never a CPU role, even when an AI (the pilot or the fallback) operates it.
    assert!(!is_cpu_role(Scene::Training, 0, true));
    assert!(!is_cpu_role(Scene::Lobby, 0, true));
    // The CPU role: entry 1 in Training (any non-zero operated entry), entry 4 in the waiting room.
    assert!(is_cpu_role(Scene::Training, 1, true));
    assert!(is_cpu_role(Scene::Lobby, 4, true));
    assert!(!is_cpu_role(Scene::Lobby, 1, true), "only entries 0 and 4 are serviced in the lobby; 0 is never a CPU");
    // A human-operated entry is not a role; out-of-range entries are refused.
    assert!(!is_cpu_role(Scene::Training, 1, false));
    assert!(!is_cpu_role(Scene::Training, 8, true) && !is_cpu_role(Scene::Training, -1, true));
}

#[test]
fn cpu_core_role_table_finds_bodies_by_accessor() {
    let mut roles = RoleTable::new();
    let scene = Some(Scene::Training);
    roles.observe_scene(scene);
    let receipt = roles.receipt(1).unwrap();
    let popo = FighterKey::new(1, false).unwrap();
    let nana = FighterKey::new(1, true).unwrap();
    assert!(roles.set(receipt, 0x1000, popo));
    assert!(roles.set(receipt, 0x2000, nana));
    assert_eq!(roles.find(scene, 0x1000), Some(popo));
    assert_eq!(roles.find(scene, 0x2000), Some(nana));
    assert_eq!(roles.find(scene, 0x3000), None);
    assert_eq!(roles.find(scene, 0), None);
    assert_ne!(popo.index(), nana.index());
    // A body replaced (the accessor moved) forgets the old accessor; a body that stops being a CPU
    // is forgotten.
    assert!(roles.set(receipt, 0x1800, popo));
    assert_eq!(roles.find(scene, 0x1000), None);
    assert_eq!(roles.accessor_of(scene, popo), Some(0x1800));
    roles.forget(0x2000);
    assert_eq!(roles.find(scene, 0x2000), None);
    roles.clear();
    assert_eq!(roles.find(scene, 0x1800), None);
    assert!(FighterKey::new(8, false).is_none() && FighterKey::new(-1, false).is_none());
    let keys: std::collections::BTreeSet<_> =
        (0..8).flat_map(|e| [false, true].map(|p| FighterKey::new(e, p).unwrap().index())).collect();
    assert_eq!(keys.len(), FighterKey::COUNT);
}

/// B4: under the P1 pilot the CPU settings can never write entry 0. The role table is the one place a
/// body becomes a CPU role (every consumer resolves bodies through it), and `set` refuses entry 0 even
/// with a current receipt and an AI-operated scene — so nothing downstream can resolve or write it.
#[test]
fn cpu_core_role_table_never_registers_entry_0() {
    let mut roles = RoleTable::new();
    let scene = Some(Scene::Training);
    roles.observe_scene(scene);
    let p1 = FighterKey::new(0, false).unwrap();
    let receipt = roles.receipt(0).unwrap();
    assert!(!roles.set(receipt, 0x3000, p1), "entry 0 is never a CPU role");
    assert_eq!(roles.find(scene, 0x3000), None);
    assert_eq!(roles.accessor_of(scene, p1), None);
    assert!(!roles.any());
    // The waiting room is the same: only the practice CPU's entry 4 ever registers.
    let mut lobby = RoleTable::new();
    lobby.observe_scene(Some(Scene::Lobby));
    let cpu = FighterKey::new(LOBBY_CPU_ENTRY, false).unwrap();
    let receipt = lobby.receipt(LOBBY_CPU_ENTRY).unwrap();
    assert!(lobby.set(receipt, 0x4000, cpu));
    assert_eq!(lobby.accessor_of(Some(Scene::Lobby), p1), None);
    assert_eq!(lobby.find(Some(Scene::Lobby), 0x4000), Some(cpu));
}

#[test]
fn cpu_core_bodies_keep_separate_state() {
    // Decision B6: Popo's and Nana's state are separate values.
    let mut cpu = Cpu::factory(9);
    let popo = FighterKey::new(1, false).unwrap();
    let nana = FighterKey::new(1, true).unwrap();
    cpu.fighters[popo.index()].defence.di_case = Some(3);
    cpu.fighters[popo.index()].mash.queue.push(Action::JAB);
    assert_eq!(cpu.fighters[nana.index()].defence.di_case, None);
    assert!(cpu.fighters[nana.index()].mash.queue.is_empty());
}

// ---- the playback rule (B5 / N4) --------------------------------------------------------------------

#[test]
fn cpu_core_covered_settings_yield_to_playback_by_default() {
    let idle = PlaybackState::default();
    let playing = PlaybackState { running: true, ..PlaybackState::default() };
    let recording = PlaybackState { running: true, recording: true, ..PlaybackState::default() };
    assert!(covered_setting_applies(idle, false));
    assert!(!covered_setting_applies(playing, false), "the recording wins by default");
    assert!(covered_setting_applies(playing, true), "Settings override playback On restores the upstream rule");
    assert!(covered_setting_applies(recording, false), "a recording being made does not possess the CPU");
}

// ---- the counters (SPEC-cpu-07) ---------------------------------------------------------------------

#[test]
fn cpu07_should_delay() {
    let mut counters = Counters::new();
    let c = counters.get(CounterId::Ledge);
    // Delay 0 is never waiting and never starts the counter.
    assert!(!c.should_delay(0));
    assert!(!c.counting);
    // The first call starts counting and waits; the count advances with ticks; at the delay it resets
    // and stops waiting.
    assert!(c.should_delay(3));
    assert!(c.counting);
    for _ in 0..3 {
        assert!(c.should_delay(3), "still waiting");
        c.tick();
    }
    assert_eq!(c.frames, 3);
    assert!(!c.should_delay(3));
    assert_eq!((c.frames, c.counting), (0, false));
}

#[test]
fn cpu07_tick_types_and_reset() {
    let mut counters = Counters::new();
    counters.get(CounterId::Aerial).should_delay(5);
    counters.get(CounterId::Throw).should_delay(5);
    counters.tick();
    counters.tick();
    assert_eq!(counters.count(CounterId::Aerial), 2);
    assert_eq!(counters.count(CounterId::Throw), 2);
    // A counter that was never started does not move.
    assert_eq!(counters.count(CounterId::Reaction), 0);
    counters.reset_all();
    assert_eq!(counters.count(CounterId::Aerial), 0);
    assert!(!counters.get(CounterId::Aerial).counting);
}

#[test]
fn cpu07_single_tick_per_frame() {
    // Two bodies on one entry (Ice Climbers) each have their own counter; the clock ticks once per
    // simulation frame whatever the number of bodies, so a delay of N frames takes N frames.
    let mut cpu = Cpu::factory(1);
    let popo = FighterKey::new(1, false).unwrap();
    let nana = FighterKey::new(1, true).unwrap();
    cpu.fighters[popo.index()].counters.get(CounterId::Ledge).should_delay(10);
    cpu.fighters[nana.index()].counters.get(CounterId::Ledge).should_delay(10);
    for _ in 0..10 {
        cpu.tick_clock();
    }
    assert_eq!(cpu.fighters[popo.index()].counters.count(CounterId::Ledge), 10);
    assert_eq!(cpu.fighters[nana.index()].counters.count(CounterId::Ledge), 10);
}

// ---- the mash queue and its buffer entry ------------------------------------------------------------

fn settings_with(rows: &[(&str, &[u8])]) -> CpuSettings {
    let mut snap = Snapshot::factory();
    for (id, row) in rows {
        snap.set(id, row).unwrap();
    }
    CpuSettings::from_snapshot(&snap)
}

fn row30(at: &[usize]) -> Vec<u8> {
    let mut row = vec![0u8; 30];
    for i in at {
        row[*i] = 1;
    }
    row
}

#[test]
fn cpu01_queue_main_then_followup() {
    let s = settings_with(&[("mash_state", &row30(&[6])), ("follow_up", &row30(&[1]))]);
    let mut r = rng();
    let mut mash = Mash::new();
    let out = mash.buffer_menu_mash(&s, &mut r, BufferContext::default(), Some(Action::NAIR));
    assert_eq!(out, Buffered::Queued { follow_up: true });
    assert_eq!(mash.queue.current(), Some(Action::NAIR));
    // Completion pops the main action; the follow-up becomes current; the aerial delay clears.
    assert_eq!(mash.complete(), Some(Action::NAIR));
    assert_eq!(mash.queue.current(), Some(Action::JUMP));
    assert_eq!(mash.complete(), Some(Action::JUMP));
    assert!(mash.queue.is_empty());
    // A non-empty queue refuses a new buffer; an empty pick buffers nothing.
    mash.queue.push(Action::JAB);
    assert_eq!(mash.buffer_menu_mash(&s, &mut r, BufferContext::default(), Some(Action::GRAB)), Buffered::Refused);
    mash.full_reset();
    assert_eq!(mash.buffer_menu_mash(&s, &mut r, BufferContext::default(), None), Buffered::Refused);
}

#[test]
fn cpu01_playback_interrupt() {
    let s = settings_with(&[("mash_state", &row30(&[18]))]);
    let mut r = rng();
    let playing = PlaybackState { running: true, ..PlaybackState::default() };
    let ctx = BufferContext { playback: playing };
    let mut mash = Mash::new();
    // Playback Mash Interrupt On (the default): a mash while a playback runs stops it and buffers
    // nothing.
    assert_eq!(mash.buffer_menu_mash(&s, &mut r, ctx, Some(Action::JAB)), Buffered::StopPlayback);
    assert!(mash.queue.is_empty());
    // A playback action is itself allowed to queue; a recording, standby or a queued playback blocks
    // the interrupt.
    assert_eq!(mash.buffer_menu_mash(&s, &mut r, ctx, Some(Action(25))), Buffered::Queued { follow_up: false });
    mash.full_reset();
    let queued = BufferContext { playback: PlaybackState { running: true, queued: true, ..Default::default() } };
    assert_eq!(mash.buffer_menu_mash(&s, &mut r, queued, Some(Action::JAB)), Buffered::Queued { follow_up: false });
    // Playback Mash Interrupt Off: the mash buffers normally.
    mash.full_reset();
    let off = settings_with(&[("playback_mash", &[0, 1])]);
    assert_eq!(mash.buffer_menu_mash(&off, &mut r, ctx, Some(Action::JAB)), Buffered::Queued { follow_up: false });
}

#[test]
fn cpu06_roll_points() {
    // Buffering rolls the attack angle and, for an aerial, the aerial delay; a menu mash also rolls
    // full hop, fast fall and falling aerial (an empty row is false).
    let s = settings_with(&[
        ("attack_angle", &[0, 1, 0]),
        ("aerial_delay", &{
            let mut row = vec![0u8; 31];
            row[7] = 1;
            row
        }),
        ("fast_fall", &[1, 0]),
        ("full_hop", &[0, 1]),
    ]);
    let mut r = rng();
    let mut mash = Mash::new();
    mash.buffer_menu_mash(&s, &mut r, BufferContext::default(), Some(Action::UAIR));
    assert_eq!(mash.rolled.attack_angle, Some(1));
    assert_eq!(mash.rolled.aerial_delay, 7);
    assert!(mash.rolled.fast_fall);
    assert!(!mash.rolled.full_hop);
    assert!(!mash.rolled.falling_aerial);
    // A grounded action leaves the aerial delay alone (it is only rolled for an aerial).
    mash.full_reset();
    mash.buffer_menu_mash(&s, &mut r, BufferContext::default(), Some(Action::JAB));
    assert_eq!(mash.rolled.aerial_delay, 0);
}

// ---- the menu's re-roll events and the hub ----------------------------------------------------------

#[test]
fn cpu_core_publish_consumes_the_menu_rerolls() {
    let mut cpu = Cpu::factory(5);
    let key = FighterKey::new(1, false).unwrap();
    // A fresh body has its SDI roll pending, so its first frame rolls.
    assert_ne!(cpu.fighters[key.index()].pending & reroll::SDI_DIRECTION, 0);
    cpu.fighters[key.index()].pending = 0;
    cpu.fighters[key.index()].ledge.case = Some(4);
    cpu.fighters[key.index()].ledge.delay = Some(30);
    let s = settings_with(&[("sdi_state", &[0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0])]);
    cpu.publish(s, reroll::SDI_DIRECTION | reroll::LEDGE_OPTIONS);
    let f = &cpu.fighters[key.index()];
    assert_eq!(f.pending, reroll::SDI_DIRECTION, "SDI waits for the next CPU frame");
    assert_eq!(f.ledge.case, None, "B11: the held ledge case clears at once");
    assert_eq!(f.ledge.delay, Some(30), "the held delay is not touched");
    // A publish with no re-roll bit leaves everything as it was.
    cpu.fighters[key.index()].ledge.case = Some(1);
    cpu.publish(s, 0);
    assert_eq!(cpu.fighters[key.index()].ledge.case, Some(1));
}

#[test]
fn cpu_core_request_rerolls_is_the_publish_half_without_new_settings() {
    let mut cpu = Cpu::factory(5);
    let key = FighterKey::new(1, false).unwrap();
    cpu.fighters[key.index()].pending = 0;
    cpu.fighters[key.index()].ledge.case = Some(4);
    cpu.fighters[key.index()].throw.case = Some(2);
    let before = cpu.settings;
    cpu.request_rerolls(reroll::SDI_DIRECTION | reroll::SHIELD_TILT | reroll::LEDGE_OPTIONS | reroll::THROW_OPTIONS);
    let f = &cpu.fighters[key.index()];
    assert_eq!(f.pending, reroll::SDI_DIRECTION | reroll::SHIELD_TILT);
    assert_eq!((f.ledge.case, f.throw.case), (None, None));
    assert_eq!(cpu.settings, before, "no settings change");
}
#[test]
fn state06_hub_order_clears_cpu_state() {
    let s = settings_with(&[("sdi_state", &[0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0])]);
    let mut cpu = Cpu::new(s, 3);
    let key = FighterKey::new(1, false).unwrap();
    let f = &mut cpu.fighters[key.index()];
    f.mash.queue.push(Action::JAB);
    f.counters.get(CounterId::Ledge).should_delay(10);
    f.ledge.delay = Some(10);
    let mut seen = Vec::new();
    model::lra_reset::run_hub(|step| {
        seen.push(step);
        cpu.hub_step(step);
    });
    assert_eq!(seen, model::lra_reset::HUB_ORDER);
    let f = &cpu.fighters[key.index()];
    assert!(f.mash.queue.is_empty());
    assert_eq!(f.counters.count(CounterId::Ledge), 0);
    assert_eq!(f.ledge.delay, None);
    assert_eq!(f.defence.sdi_dir, Some(2), "the hub rolls the SDI direction");
    assert_eq!(f.pending & reroll::SDI_DIRECTION, 0);
    let _ = HubStep::Mash;
}

#[test]
fn cpu_core_frame_with_no_work_asks_for_nothing() {
    // Settings at the factory, a body on the ground: the only thing a frame does is write the
    // dodge-staling flag once (the row is On, so the flag is false).
    let mut cpu = Cpu::factory(2);
    cpu.ids = synthetic_table();
    let key = FighterKey::new(1, false).unwrap();
    let facts = FrameFacts { status: 5, ..FrameFacts::default() };
    let out = cpu.frame(key, &facts);
    assert_eq!(out.stale_flag, Some(false));
    assert!(out.clatter.is_none() && out.request_status.is_none() && out.playback.is_none());
    let again = cpu.frame(key, &facts);
    assert_eq!(again, Default::default());
}

#[test]
fn cpu_ids_resolved_entries_lists_exactly_what_resolved_by_name() {
    // The diag build prints these so a log's ids can be read by name.
    let table = Table::with(&[(C::StatusWait, 7), (C::SituationCliff, 3)]);
    let entries: Vec<(&str, i32)> = table.resolved_entries().collect();
    assert_eq!(entries.len(), 2);
    assert!(entries.contains(&("FIGHTER_STATUS_KIND_WAIT", 7)));
    assert!(entries.contains(&("SITUATION_KIND_CLIFF", 3)));
    assert_eq!(Table::unresolved().resolved_entries().count(), 0);
    assert_eq!(table.resolved_entries().count() + table.missing().count(), ids::COUNT);
}

#[test]
fn eden_package_2_settings_files_parse_without_issues_and_change_the_rows_they_name() {
    // The files `eden/mk_settings.py` writes: every row has the model's length and parses.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("eden/settings");
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if !path.file_name().unwrap().to_string_lossy().starts_with("p2-") {
            continue;
        }
        let loaded = model::settings::parse_bytes(&std::fs::read(&path).unwrap());
        assert!(loaded.issues.is_empty(), "{}: {:?}", path.display(), loaded.issues);
        assert_ne!(loaded.settings.menu, Snapshot::factory(), "{} changes nothing", path.display());
        seen += 1;
    }
    assert!(seen >= 10, "the package-2 settings files are missing");
}
