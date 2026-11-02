//! What the menu worker asks of the game, handed to the game-synchronized tick (lead ruling 2026-11-02: the worker reads no game state, so
//! an action that reads or writes the game runs where the game is: the layout tick on the UI job). One atomic per request, no allocation.

use std::sync::atomic::{AtomicI32, Ordering};

const NONE: i32 = -1;

/// The item of the Give item grid to give Player 1 (`ITEM_NAMES` index), or `NONE`.
static GIVE_ITEM: AtomicI32 = AtomicI32::new(NONE);

/// The player pressed A on a tile of the Give item grid (the menu worker).
pub fn give_item(item: u8) {
    GIVE_ITEM.store(i32::from(item), Ordering::Release);
}

/// Run what the worker asked for.
///
/// # Safety
/// On the UI job thread, inside the Training layout's tick.
pub unsafe fn run_menu_requests() {
    let item = GIVE_ITEM.swap(NONE, Ordering::AcqRel);
    if let Ok(choice) = usize::try_from(item) {
        // The roster module queues it for the player's callback and says why in a notice when practice is not ready.
        crate::runtime::roster::character_items::request(choice);
    }
}