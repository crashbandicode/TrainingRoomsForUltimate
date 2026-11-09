//! What the menu worker asks of the game, handed to the game (lead ruling 2026-11-02: the worker reads no game state, so an action that
//! reads or writes the game runs where the game is). One atomic per request, no allocation. The item request is taken on Player 1's frame
//! (`lobby::frame`), with the roster's other per-frame steps, not on the Training layout's tick: that tick runs on the game's task workers
//! (hot-slots hook-thread table 2026-11-09), and the item engine takes the source lock (`character_sources::STATE`) that the menu worker's
//! roster re-plan holds, the rc.6 freeze class (`tests/lock_order.rs`).

use std::sync::atomic::{AtomicI32, Ordering};

const NONE: i32 = -1;

/// The item of the Give item grid to give Player 1 (`ITEM_NAMES` index), or `NONE`.
static GIVE_ITEM: AtomicI32 = AtomicI32::new(NONE);

/// The player pressed A on a tile of the Give item grid (the menu worker).
pub fn give_item(item: u8) {
    GIVE_ITEM.store(i32::from(item), Ordering::Release);
}

/// The item the worker asked for, once (Player 1's frame queues it: `character_items::request`).
pub fn take_give_item() -> Option<usize> {
    usize::try_from(GIVE_ITEM.swap(NONE, Ordering::AcqRel)).ok()
}
