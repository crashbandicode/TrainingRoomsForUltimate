//! The one-time import conversion of the retired *Item on load* setting (decision N6, SPEC-state-04
//! §5.2 steps 1-2 and §7, SPEC-core-08 §4): each older save without an item snapshot gets one written
//! from the old `character_item` setting, once.
//!
//! The setting is a one-hot array of 17 options: `None`, Player 1st to 8th Var., CPU 1st to 8th Var. A
//! Player entry names the variation of the *player record's* fighter kind; a CPU entry names the
//! variation of the kind of the *CPU record in the same slot*; either way the snapshot goes to the
//! player record (only the player's fighter ever received the item). The table is SPEC-state-04 §2.1;
//! `choice` is the index into the fork's 38-item list (`held_item::ITEM_NAMES`).

use serde_json::Value;

use super::file::SaveFile;
use super::held_item::HeldItem;

/// One row of §2.1: the `(choice, variation)` of each variation index, for a fighter kind.
struct Entry {
    kind: i32,
    items: &'static [(u8, i32)],
}

/// SPEC-state-04 §2.1. Kinds are the game's `FIGHTER_KIND_*` values.
static TABLE: [Entry; 18] = [
    Entry { kind: 0x54, items: &[(0, 0)] },                                                   // Banjo & Kazooie
    Entry { kind: 0x3a, items: &[(1, 0)] },                                                   // Bowser Jr.
    Entry { kind: 0x0e, items: &[(2, 0), (3, 5), (4, 6), (5, 7), (6, 0), (7, 0)] },           // Daisy
    Entry { kind: 0x27, items: &[(8, 0), (9, 0)] },                                           // Diddy Kong
    Entry { kind: 0x45, items: &[(10, 0)] },                                                  // King K. Rool
    Entry { kind: 0x02, items: &[(11, 0), (12, 0)] },                                         // Link
    Entry { kind: 0x31, items: &[(13, 0)] },                                                  // Mega Man
    Entry { kind: 0x37, items: &[(14, 0), (15, 0), (16, 0), (17, 0), (18, 0), (19, 0), (20, 0), (21, 0)] }, // Pac-Man
    Entry { kind: 0x0d, items: &[(22, 0), (23, 5), (24, 6), (25, 7), (26, 0), (27, 0)] },     // Peach
    Entry { kind: 0x44, items: &[(28, 0)] },                                                  // Richter
    Entry { kind: 0x2d, items: &[(29, 0), (29, 1), (29, 2), (29, 3), (29, 4), (29, 5), (29, 6), (29, 7)] }, // R.O.B.
    Entry { kind: 0x38, items: &[(30, 0), (31, 0)] },                                         // Robin
    Entry { kind: 0x10, items: &[(32, 0)] },                                                  // Sheik
    Entry { kind: 0x43, items: &[(33, 0)] },                                                  // Simon
    Entry { kind: 0x22, items: &[(34, 0)] },                                                  // Snake
    Entry { kind: 0x2e, items: &[(35, 0)] },                                                  // Toon Link
    Entry { kind: 0x30, items: &[(36, 0)] },                                                  // Villager
    Entry { kind: 0x17, items: &[(37, 0)] },                                                  // Young Link
];

/// Options of the old setting.
pub const SETTING_OPTIONS: usize = 17;

/// The index of the selected option of the old setting's array (`0` is None), or `None` when the key
/// is absent, not an array, or nothing is set. The first set element wins (a one-hot row).
pub fn setting_index(value: &Value) -> Option<usize> {
    let items = value.as_array()?;
    items.iter().take(SETTING_OPTIONS).position(|v| v.as_u64().is_some_and(|n| n != 0)).filter(|i| *i != 0)
}

/// The entry a fighter kind has for a variation, if any.
pub fn item_for(kind: i32, variation: usize) -> Option<(u8, i32)> {
    TABLE.iter().find(|e| e.kind == kind).and_then(|e| e.items.get(variation)).copied()
}

/// The snapshot the old setting means for one slot: `index` is the option (1..=16), the kinds are the
/// slot's two records. A never-saved player cannot receive an item, even when the CPU supplies its kind.
/// The lifetime and fuse fields are -1/-1 so the restore skips their write-back
/// (the fork writes them back only when the item's maxima match the saved maxima).
pub fn snapshot_for(index: usize, player_kind: i32, cpu_kind: i32) -> Option<HeldItem> {
    if player_kind < 0 {
        return None;
    }
    let (kind, variation) = match index {
        1..=8 => (player_kind, index - 1),
        9..=16 => (cpu_kind, index - 9),
        _ => return None,
    };
    if kind < 0 {
        return None;
    }
    let (choice, variation) = item_for(kind, variation)?;
    Some(HeldItem::Item { choice, variation, lifetime: -1, lifetime_max: -1, fuse: -1, fuse_max: -1 })
}

/// Convert a (copied) save file in place. Returns how many player records got a snapshot. A record with
/// any snapshot (even `Empty` / `Unsupported`), a never-saved record, a setting of None, and every
/// CPU-role record are left alone.
pub fn convert_file(file: &mut SaveFile, index: usize) -> usize {
    let mut converted = 0;
    for slot in 0..file.player.len() {
        let player = &file.player[slot];
        if player.held_item.is_some() {
            continue;
        }
        if let Some(item) = snapshot_for(index, player.fighter_kind, file.cpu[slot].fighter_kind) {
            file.player[slot].held_item = Some(item);
            converted += 1;
        }
    }
    converted
}

/// The same conversion on a parsed `save_states.json` (the import changes the copy's JSON without
/// rewriting the rest of the fork's file through the typed reader). Returns how many player records
/// got a snapshot; records with any `held_item` that is not `null`, never-saved records, and every
/// CPU-role record are left alone.
pub fn convert_json(value: &mut Value, index: usize) -> usize {
    let cpu_kinds: Vec<i64> = value
        .get("cpu")
        .and_then(Value::as_array)
        .map(|records| records.iter().map(|r| r.get("fighter_kind").and_then(Value::as_i64).unwrap_or(-1)).collect())
        .unwrap_or_default();
    let Some(players) = value.get_mut("player").and_then(Value::as_array_mut) else { return 0 };
    let mut converted = 0;
    for (slot, record) in players.iter_mut().enumerate() {
        let Some(object) = record.as_object_mut() else { continue };
        if object.get("held_item").is_some_and(|item| !item.is_null()) {
            continue;
        }
        let kind = object.get("fighter_kind").and_then(Value::as_i64).unwrap_or(-1);
        let cpu_kind = cpu_kinds.get(slot).copied().unwrap_or(-1);
        let (Ok(kind), Ok(cpu_kind)) = (i32::try_from(kind), i32::try_from(cpu_kind)) else { continue };
        if let Some(HeldItem::Item { choice, variation, lifetime, lifetime_max, fuse, fuse_max }) = snapshot_for(index, kind, cpu_kind) {
            object.insert(
                "held_item".to_owned(),
                serde_json::json!({
                    "state": "item", "choice": choice, "variation": variation, "lifetime": lifetime,
                    "lifetime_max": lifetime_max, "fuse": fuse, "fuse_max": fuse_max
                }),
            );
            converted += 1;
        }
    }
    converted
}
