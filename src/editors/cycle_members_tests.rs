//! P-CYCLE: the grid's numbered list is exactly the order the CPU cycle loads. The runtime's order is
//! `mii_cycle::next_cycle` stepped from slot 0 over `cycle_members` until it returns; the menu's is the portrait grid's
//! own badges (`editors::roster::roster_badges`) with the same admission.

lib_unit_tests! {
use crate::editors::roster::{roster_badges, MiiTiles};
use crate::engine::mii::inventory::{MiiClass, Snapshot};
use crate::engine::mii::selection::Role;
use crate::engine::swap::cycle_members::*;
use crate::engine::swap::trio::TrioFighter;
use crate::engine::mii::inventory::{build_snapshot, RawSlot, TaggedId, CHARINFO_LEN};
use crate::engine::mii::selection::Preferences;
use crate::engine::swap::mii_cycle::{next_cycle, Identity, Live, Target};
use crate::engine::swap::roster_grid::Role as GridRole;
use crate::engine::swap::trio::CHOICES;
use crate::model::menu::grid::RosterUse;

const FOX: u32 = 0x07;
const GUNNER: u32 = 0x4a;

fn raw(index: usize, id: u32, class: MiiClass, face: u8) -> RawSlot {
    let mut charinfo = [face; CHARINFO_LEN];
    charinfo[..0x10].fill(index as u8);
    charinfo[0x10..0x24].fill(b'N');
    charinfo[0x24..0x26].fill(0);
    RawSlot {
        index,
        slot_id: id,
        name_len: 1,
        name_units: [u16::from(face), 0, 0, 0, 0, 0, 0, 0, 0, 0],
        colour: 0,
        voice: 37,
        specials: [0, 0, 0, 0],
        tag_outfit: TaggedId(0x1300011bfef7a5f7),
        tag_hat: TaggedId(0x5a00000fa9efa5ea),
        tag_chara: TaggedId(match class {
            MiiClass::Brawler => 0xc1004c13fdfdfb2b,
            MiiClass::Swordfighter => 0xc1004d15d7061f77,
            MiiClass::Gunner => 0xc1004e1257ab1d87,
        }),
        charinfo,
    }
}

fn ranks(kinds: &[u32]) -> Vec<u8> {
    let mut values = vec![0u8; CHOICES.len() + 1];
    for (rank, kind) in kinds.iter().enumerate() {
        values[1 + CHOICES.iter().position(|(k, _)| k == kind).unwrap()] = rank as u8 + 1;
    }
    values
}

/// The CPU cycle's loads from slot 0 back to slot 0 (Mii classes by class kind).
fn runtime_order(members: &[TrioFighter], catalog: &Snapshot, original: Identity) -> Vec<u32> {
    let prefs = Preferences::default();
    let mut live = Live::Original;
    let mut out = Vec::new();
    for _ in 0..=CHOICES.len() {
        let step = next_cycle(members, catalog, &prefs, Role::Cpu, original, live).expect("a cycle step");
        live = match step.target {
            Target::Original => return out,
            Target::Fighter(kind) => {
                out.push(kind);
                Live::Numbered { identity: Identity::Fighter(kind), cursor: step.cursor }
            }
            Target::Saved(choice) => {
                out.push(choice.canonical.class.kind());
                Live::Numbered { identity: Identity::Saved(choice.canonical), cursor: step.cursor }
            }
        };
    }
    panic!("the cycle never returned to slot 0: {out:x?}");
}

/// The grid's numbered list for the CPU cycle: one kind per number, in number order; and the slot-0 tiles.
fn menu_order(values: &[u8], excluded: Option<u32>, tiles: &MiiTiles) -> (Vec<u32>, Vec<u32>) {
    let badges = roster_badges(RosterUse::Cycles, values, excluded, GridRole::Cpu, tiles);
    let mut numbered: Vec<(u8, u32)> =
        CHOICES.iter().zip(&badges).filter_map(|((kind, _), badge)| badge.filter(|n| *n > 0).map(|n| (n, *kind))).collect();
    numbered.sort_by_key(|(n, _)| *n);
    numbered.dedup_by_key(|(n, _)| *n);
    for (i, (n, _)) in numbered.iter().enumerate() {
        assert_eq!(usize::from(*n), i + 1, "the grid's numbers have no gap: {numbered:?}");
    }
    let zero = CHOICES.iter().zip(&badges).filter(|(_, b)| **b == Some(0)).map(|((kind, _), _)| *kind).collect();
    (numbered.into_iter().map(|(_, kind)| kind).collect(), zero)
}

/// The grid and the cycle agree entry by entry (a fighter group or a Mii class per number).
fn assert_same(menu: &[u32], runtime: &[u32]) {
    assert_eq!(menu.len(), runtime.len(), "menu {menu:x?} runtime {runtime:x?}");
    for (m, r) in menu.iter().zip(runtime) {
        assert_eq!(number_of(&[*m], *r), Some(1), "menu {menu:x?} runtime {runtime:x?}");
    }
}

fn tiles(cpu_cycle: MiiAdmission) -> MiiTiles {
    MiiTiles { cpu_cycle, ..MiiTiles::default() }
}

fn a_mii_original_is_slot_zero_and_its_class_is_numbered_only_with_another_configuration() {
    let only = build_snapshot(vec![raw(0, 3137, MiiClass::Gunner, b'f')]);
    let original = only.configs[0].canonical;
    // Its class has no other configuration: not admitted, so it is slot 0 only, on both sides.
    let admitted = cpu_admission(true, &only, Some(original));
    assert_eq!(admitted, NO_MII);
    let values = ranks(&[FOX, GUNNER]);
    let runtime = runtime_order(&cycle_members(roster_of(&values), admitted), &only, Identity::Saved(original));
    assert_eq!(runtime, [FOX]);
    let (menu, zero) = menu_order(&values, Some(GUNNER), &tiles(admitted));
    assert_same(&menu, &runtime);
    assert_eq!(zero, [GUNNER], "the grey 0 on the captured Gunner");
    // A second Gunner: the class is numbered (the other configuration), and slot 0 stays the captured one.
    let two = build_snapshot(vec![raw(0, 3137, MiiClass::Gunner, b'f'), raw(1, 3200, MiiClass::Gunner, b'g')]);
    let original = two.configs.iter().find(|c| c.slot_id == 3137).unwrap().canonical;
    let admitted = cpu_admission(true, &two, Some(original));
    assert_eq!(admitted, [false, false, true]);
    let runtime = runtime_order(&cycle_members(roster_of(&values), admitted), &two, Identity::Saved(original));
    assert_eq!(runtime, [FOX, GUNNER]);
    let (menu, _) = menu_order(&values, Some(GUNNER), &tiles(admitted));
    assert_same(&menu, &runtime);
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("a_mii_original_is_slot_zero_and_its_class_is_numbered_only_with_another_configuration", a_mii_original_is_slot_zero_and_its_class_is_numbered_only_with_another_configuration as fn()),
    ]);
}
}
