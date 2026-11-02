//! The held item of a save state (SPEC-state-04 section 5.1, draft 9): the snapshot is taken and restored by package 6's item engine
//! (`roster::character_items`, "the fork's item engine"), which also loads the resources of the fighter an item needs, so a saved
//! Daisy turnip comes back when Daisy is not in the match. This module only converts between the engine's snapshot and the save
//! record's `HeldItem` (the same fields, kept apart so the save file format does not depend on the engine).

use smash::app::BattleObjectModuleAccessor;

use crate::engine::items::character_items::HeldItemSnapshot;
use crate::model::state::held_item::HeldItem;
use crate::runtime::roster::character_items;

fn to_record(snapshot: HeldItemSnapshot) -> HeldItem {
    match snapshot {
        HeldItemSnapshot::Empty => HeldItem::Empty,
        HeldItemSnapshot::Unsupported => HeldItem::Unsupported,
        HeldItemSnapshot::Item { choice, variation, lifetime, lifetime_max, fuse, fuse_max } => {
            HeldItem::Item { choice, variation, lifetime, lifetime_max, fuse, fuse_max }
        }
    }
}

fn from_record(item: &HeldItem) -> HeldItemSnapshot {
    match *item {
        HeldItem::Empty => HeldItemSnapshot::Empty,
        HeldItem::Unsupported => HeldItemSnapshot::Unsupported,
        HeldItem::Item { choice, variation, lifetime, lifetime_max, fuse, fuse_max } => {
            HeldItemSnapshot::Item { choice, variation, lifetime, lifetime_max, fuse, fuse_max }
        }
    }
}

/// The player's held item as a save-record value.
///
/// # Safety
/// `accessor` is the player's live module accessor.
pub unsafe fn snapshot(accessor: usize) -> HeldItem {
    to_record(character_items::snapshot(accessor as *mut BattleObjectModuleAccessor))
}

/// Restore a saved item onto the player's hand (asynchronous when the item needs another fighter's resources: the engine finishes it
/// from the player's frame callback).
///
/// # Safety
/// `accessor` is the player's live module accessor.
pub unsafe fn restore(accessor: usize, item: &HeldItem) {
    character_items::restore(accessor as *mut BattleObjectModuleAccessor, from_record(item));
}
