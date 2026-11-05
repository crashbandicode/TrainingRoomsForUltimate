//! P1-CYCLE: the P1 Cycles grid numbers exactly the order the Player cycle walks, and a missing roster key is the
//! same roster on the grid and in the typed view.

use crate::editors::roster::{roster_badges, MiiTiles, RosterEditor};
use crate::engine::compat::menu::TrainingModpackMenu;
use crate::engine::mii::inventory::{
    build_snapshot, MiiClass, RawSlot, Snapshot, TaggedId, CHARINFO_LEN,
};
use crate::engine::mii::selection::{PlayerOrigin, Preferences, Role};
use crate::engine::swap::cycle_members::{
    cycle_members, number_of, player_admission, roster_of, MiiAdmission, NO_MII,
};
use crate::engine::swap::mii_cycle::{next_cycle, Identity, Live, Target};
use crate::engine::swap::ordered_roster::OrderedFighterRoster;
use crate::engine::swap::roster_grid::Role as GridRole;
use crate::engine::swap::trio::{TrioFighter, CHOICES};
use crate::model::menu::grid::RosterUse;
use crate::model::settings::Snapshot as SettingsSnapshot;

const MARIO: u32 = 0x00;
const FOX: u32 = 0x07;
const KIRBY: u32 = 0x06;
const BRAWLER: u32 = 0x48;
const SWORD: u32 = 0x49;
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

fn catalog() -> Snapshot {
    build_snapshot(vec![
        raw(0, 2622, MiiClass::Swordfighter, b'a'),
        raw(1, 2700, MiiClass::Swordfighter, b'b'),
        raw(2, 4155, MiiClass::Brawler, b'c'),
        raw(3, 4200, MiiClass::Brawler, b'd'),
        raw(4, 3137, MiiClass::Gunner, b'f'),
    ])
}

fn all_values() -> Vec<u8> {
    let mut values = vec![0u8; CHOICES.len() + 1];
    values[0] = 1;
    values
}

fn ranks(kinds: &[u32]) -> Vec<u8> {
    let mut values = vec![0u8; CHOICES.len() + 1];
    for (rank, kind) in kinds.iter().enumerate() {
        values[1 + CHOICES.iter().position(|(k, _)| k == kind).unwrap()] = rank as u8 + 1;
    }
    values
}

fn tiles(player_cycle: MiiAdmission) -> MiiTiles {
    MiiTiles {
        player_cycle,
        ..MiiTiles::default()
    }
}

/// The Player cycle's loads from slot 0 back to slot 0, over the one list.
fn walked(members: &[TrioFighter], catalog: &Snapshot, original: Identity) -> Vec<u32> {
    let prefs = Preferences::default();
    let mut live = Live::Original;
    let mut out = Vec::new();
    for _ in 0..=CHOICES.len() {
        let step = next_cycle(members, catalog, &prefs, Role::Player, original, live)
            .expect("a cycle step");
        live = match step.target {
            Target::Original => return out,
            Target::Fighter(kind) => {
                out.push(kind);
                Live::Numbered {
                    identity: Identity::Fighter(kind),
                    cursor: step.cursor,
                }
            }
            Target::Saved(choice) => {
                out.push(choice.canonical.class.kind());
                Live::Numbered {
                    identity: Identity::Saved(choice.canonical),
                    cursor: step.cursor,
                }
            }
        };
    }
    panic!("the cycle never returned to slot 0: {out:x?}");
}

/// The grid's numbered list for the Player cycle, one kind per number, and the slot-0 tiles.
fn menu_order(values: &[u8], excluded: Option<u32>, tiles: &MiiTiles) -> (Vec<u32>, Vec<u32>) {
    let badges = roster_badges(RosterUse::Cycles, values, excluded, GridRole::Player, tiles);
    let mut numbered: Vec<(u8, u32)> = CHOICES
        .iter()
        .zip(&badges)
        .filter_map(|((kind, _), badge)| badge.filter(|n| *n > 0).map(|n| (n, *kind)))
        .collect();
    numbered.sort_by_key(|(n, _)| *n);
    numbered.dedup_by_key(|(n, _)| *n);
    for (i, (n, _)) in numbered.iter().enumerate() {
        assert_eq!(
            usize::from(*n),
            i + 1,
            "the grid's numbers have no gap: {numbered:?}"
        );
    }
    let zero = CHOICES
        .iter()
        .zip(&badges)
        .filter(|(_, b)| **b == Some(0))
        .map(|((kind, _), _)| *kind)
        .collect();
    (numbered.into_iter().map(|(_, kind)| kind).collect(), zero)
}

fn assert_same(menu: &[u32], runtime: &[u32]) {
    assert_eq!(
        menu.len(),
        runtime.len(),
        "menu {menu:x?} runtime {runtime:x?}"
    );
    for (m, r) in menu.iter().zip(runtime) {
        assert_eq!(
            number_of(&[*m], *r),
            Some(1),
            "menu {menu:x?} runtime {runtime:x?}"
        );
    }
}

#[test]
fn the_player_grid_numbers_exactly_what_the_player_cycle_walks() {
    let catalog = catalog();
    // The split this closes: the unfiltered roster walks the Gunner, and a grid with no Player-cycle admission
    // (what `alpha_player_cycle` publishes while P1 is not a Mii) does not number it.
    let unfiltered = walked(
        &roster_of(&all_values()).fighters(),
        &catalog,
        Identity::Fighter(MARIO),
    );
    assert!(
        unfiltered.contains(&GUNNER),
        "the operator's class of bug, on P1"
    );
    let (hidden, _) = menu_order(&all_values(), Some(MARIO), &tiles(NO_MII));
    assert!(!hidden.contains(&GUNNER));

    // All. P1 is Mario, so the origin is not a Mii. Every class with a configuration is in both lists.
    let admitted = player_admission(&catalog, PlayerOrigin::NotMii);
    assert_eq!(admitted, [true, true, true]);
    let members = cycle_members(roster_of(&all_values()), admitted);
    let runtime = walked(&members, &catalog, Identity::Fighter(MARIO));
    let (menu, zero) = menu_order(&all_values(), Some(MARIO), &tiles(admitted));
    assert_same(&menu, &runtime);
    assert_eq!(zero, [MARIO]);
    for class in [BRAWLER, SWORD, GUNNER] {
        assert!(
            runtime.contains(&class) && menu.contains(&class),
            "All numbers an admitted Mii class: {class:x}"
        );
    }

    // Explicit ranks, Gunner included, in the stored order.
    let values = ranks(&[FOX, GUNNER, KIRBY]);
    let runtime = walked(
        &cycle_members(roster_of(&values), admitted),
        &catalog,
        Identity::Fighter(MARIO),
    );
    assert_eq!(runtime, [FOX, GUNNER, KIRBY]);
    let (menu, _) = menu_order(&values, Some(MARIO), &tiles(admitted));
    assert_same(&menu, &runtime);

    // A greyed class: no Brawler configuration, and a stored rank does not cycle it.
    let bare = build_snapshot(vec![raw(0, 3137, MiiClass::Gunner, b'f')]);
    let admitted = player_admission(&bare, PlayerOrigin::NotMii);
    assert_eq!(admitted, [false, false, true]);
    let values = ranks(&[FOX, BRAWLER, KIRBY]);
    let runtime = walked(
        &cycle_members(roster_of(&values), admitted),
        &bare,
        Identity::Fighter(MARIO),
    );
    assert_eq!(runtime, [FOX, KIRBY]);
    let (menu, _) = menu_order(&values, Some(MARIO), &tiles(admitted));
    assert_same(&menu, &runtime);
    assert!(!menu.contains(&BRAWLER) && !runtime.contains(&BRAWLER));

    // A preset-only Gunner is admitted, numbered, and walked.
    use crate::engine::mii::draft::{Draft, Library, Saved};
    let with_gunner = build_snapshot(vec![raw(0, 3137, MiiClass::Gunner, b'f')]);
    let mut draft = Draft::from_template(&with_gunner.configs[0]).unwrap();
    draft.set_specials([1, 0, 0, 0]).unwrap();
    let native = build_snapshot(vec![raw(0, 2622, MiiClass::Swordfighter, b'a')]);
    let mut library = Library::default();
    assert!(matches!(
        library.import(draft.configuration().unwrap(), "Preset Gunner", &native),
        Ok(Saved::Created(_))
    ));
    let preset = native.with_library(&library);
    let admitted = player_admission(&preset, PlayerOrigin::NotMii);
    assert_eq!(
        admitted,
        [false, true, true],
        "the preset admits the Gunner"
    );
    let runtime = walked(
        &cycle_members(roster_of(&all_values()), admitted),
        &preset,
        Identity::Fighter(MARIO),
    );
    assert!(runtime.contains(&GUNNER));
    let (menu, _) = menu_order(&all_values(), Some(MARIO), &tiles(admitted));
    assert_same(&menu, &runtime);

    // A Mii original whose class has no other configuration is slot 0 only, on both sides.
    let only = build_snapshot(vec![raw(0, 3137, MiiClass::Gunner, b'f')]);
    let original = only.configs[0].canonical;
    let admitted = player_admission(
        &only,
        PlayerOrigin::Mii(crate::engine::mii::selection::Origin(original)),
    );
    assert_eq!(admitted, NO_MII);
    let values = ranks(&[FOX, GUNNER]);
    let runtime = walked(
        &cycle_members(roster_of(&values), admitted),
        &only,
        Identity::Saved(original),
    );
    assert_eq!(runtime, [FOX]);
    let (menu, zero) = menu_order(&values, Some(GUNNER), &tiles(admitted));
    assert_same(&menu, &runtime);
    assert_eq!(zero, [GUNNER], "the grey 0 on the captured Gunner");

    // A second Gunner, whatever its hat: the class is numbered and walked, and slot 0 stays the captured one.
    let two = build_snapshot(vec![
        raw(0, 3137, MiiClass::Gunner, b'f'),
        raw(1, 3200, MiiClass::Gunner, b'g'),
    ]);
    let original = two
        .configs
        .iter()
        .find(|c| c.slot_id == 3137)
        .unwrap()
        .canonical;
    let admitted = player_admission(
        &two,
        PlayerOrigin::Mii(crate::engine::mii::selection::Origin(original)),
    );
    assert_eq!(admitted, [false, false, true]);
    let runtime = walked(
        &cycle_members(roster_of(&values), admitted),
        &two,
        Identity::Saved(original),
    );
    assert_eq!(runtime, [FOX, GUNNER]);
    let (menu, _) = menu_order(&values, Some(GUNNER), &tiles(admitted));
    assert_same(&menu, &runtime);
}

#[test]
fn a_missing_roster_key_means_the_same_thing_to_the_grid_and_the_runtime() {
    let editor = RosterEditor::new(RosterUse::Cycles);
    assert_eq!(editor.key(), "player_roster");
    let factory = editor.values(None);
    assert_eq!(factory[0], 1, "the grid's missing cycle key is All");
    let grid = roster_of(&factory);
    assert_eq!(grid, OrderedFighterRoster::all());
    let view = TrainingModpackMenu::from_snapshot(&SettingsSnapshot::factory());
    assert_eq!(view.player_roster, grid, "missing player_roster");
    assert_eq!(view.cpu_roster, grid, "missing cpu_roster");

    // A malformed value is the same factory, on both readers and both roles.
    let mut snapshot = SettingsSnapshot::factory();
    snapshot
        .set_kept("player_roster", serde_json::json!("no"))
        .unwrap();
    snapshot
        .set_kept("cpu_roster", serde_json::json!([1u8]))
        .unwrap();
    let view = TrainingModpackMenu::from_snapshot(&snapshot);
    assert_eq!(view.player_roster, grid);
    assert_eq!(view.cpu_roster, grid);
    assert_eq!(
        editor.values(snapshot.extra().get("player_roster")),
        factory
    );
    assert_eq!(editor.values(snapshot.extra().get("cpu_roster")), factory);

    // A present key is that key. The two roles can differ, and both readers agree on each.
    let explicit = ranks(&[FOX, KIRBY]);
    snapshot
        .set_kept("player_roster", serde_json::json!(explicit.clone()))
        .unwrap();
    snapshot
        .set_kept("cpu_roster", RosterEditor::value_of(&all_values()))
        .unwrap();
    let view = TrainingModpackMenu::from_snapshot(&snapshot);
    assert_eq!(view.player_roster, roster_of(&explicit));
    assert_eq!(view.cpu_roster, roster_of(&all_values()));
    assert_eq!(
        roster_of(&editor.values(snapshot.extra().get("player_roster"))),
        view.player_roster
    );
    assert_eq!(
        roster_of(&editor.values(snapshot.extra().get("cpu_roster"))),
        view.cpu_roster
    );
}
