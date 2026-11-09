//! Lobby Items (Phase 2 N11, `docs/ROADMAP.md`; design: `artifacts/phase23/LANE-SURFACE-REPORT.md` "P-N11"). Switch only.
//!
//! The waiting room has two item sources, and each gets one inline hook on its own native decision:
//! - the per-frame spawner's switch test (`lobby_items.spawn_switch`, `main+0x15d9a80`, `cbz w8` after `ldrb w8,[c,#0x30]`);
//! - the timed generator's lottery verdict (`lobby_items.drop_lot`, `main+0x15499b4`, `tbz w0,#0` after `bl 0x15dc450`), which
//!   the spawner's switch doesn't gate (#62, `artifacts/phase23/DESIGN-RC6-62-QP-ITEMS.md`).
//!
//! In a qualified waiting-room practice scene with the row Off each callback hands the native a zero, and the native takes its own
//! off (or "lottery declined") path. Nothing is written to the game and nothing is restored; Training, matches, online matches and
//! every other scene see the context untouched. Items already on the stage are removed once per Off edge through the native
//! removal, from Player 1's frame ([`frame`]), never inside the item manager's update and never as a sweep.

use std::sync::atomic::{AtomicBool, Ordering};

use skyline::hooks::InlineCtx;

use crate::model::hook_stats::{HookId, HookScope};
use crate::model::lobby_items::{collect, suppress, Removal};
use crate::model::sites_pkg3 as s;
use crate::runtime::state_natives as n;

/// The rows passed and the hook is installed.
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// Player 1's frame is the only caller (one game thread), so the lock is never contended.
static REMOVAL: hos_lock::Mutex<Removal> = hos_lock::Mutex::new(Removal::new());

/// The seam rows this feature compares before it hooks: its own five, and the practice observer's item-gate rows the removal reads.
const ROWS: &[&str] = &[
    "lobby_items.spawn_switch",
    "lobby_items.switch_load",
    "lobby_items.spawner_thunk",
    "lobby_items.drop_lot",
    "lobby_items.drop_gate",
    "practice_stage.item_manager",
    "practice_stage.item_predicate",
    "practice_stage.item_predicate+0x40",
    "practice_stage.item_list_init",
    "practice_stage.item_list_init+0x40",
];

/// The item natives of the save states the removal calls (`sites_pkg3` rows; `state_natives` refuses any not granted).
const NATIVES: [usize; 4] = [
    s::STATE_ITEM_ACTIVE_COUNT_OFFSET,
    s::STATE_ITEM_ACTIVE_ITEM_OFFSET,
    s::STATE_ITEM_OBJECT_ID_OFFSET,
    s::STATE_ITEM_REMOVE_OFFSET,
];

/// A qualified waiting-room practice scene (the lobby stage cycle's own predicate: LDN, or Quickplay once verified; melee 0x0d).
fn waiting() -> bool {
    crate::runtime::lobby::game_mode::current()
        .is_some_and(|mode| mode.initialized && super::lobby_stage::qualified_waiting(mode.mode, mode.melee))
}

#[skyline::hook(offset = 0x15d9a80, inline)]
unsafe fn spawn_switch(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::LobbyItems);
    if suppress(waiting(), crate::runtime::published::lobby_items()) {
        // The native's `cbz w8` now takes the spawner's own off path (its epilogue) for this frame.
        ctx.registers[8].set_x(0);
    }
}

#[skyline::hook(offset = 0x15499b4, inline)]
unsafe fn drop_lot(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::LobbyItemsDrop);
    if suppress(waiting(), crate::runtime::published::lobby_items()) {
        // The native's `tbz w0,#0` now reads a lost lottery: no item, and the generator re-arms its timer as after any loss.
        ctx.registers[0].set_x(0);
    }
}

/// Once from `lobby::init`, after the practice observer checked its rows: compare every row, then hook.
pub(crate) fn init() {
    let natives = NATIVES.iter().all(|offset| crate::runtime::game::granted(*offset));
    let rows = crate::runtime::seams::seam_install::gate("lobby_items", ROWS, natives);
    if rows && natives {
        skyline::install_hooks!(spawn_switch, drop_lot);
        INSTALLED.store(true, Ordering::Release);
    }
    crate::engine::compat::logging::diagnostic(format_args!(
        "[TrainingRooms] lobby_items installed={} rows={rows} natives={natives}",
        INSTALLED.load(Ordering::Acquire)
    ));
}

/// Player 1's frame (`lobby::frame`): pays the one removal an Off edge owes, when the native item gate and the roster allow it.
///
/// # Safety
/// Called on the game thread from the per-fighter frame callback.
pub(crate) unsafe fn frame() {
    if !INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let _scope = HookScope::enter_hook(HookId::LobbyItemsRemove);
    let waiting = waiting();
    let on = crate::runtime::published::lobby_items();
    let Some(mut removal) = REMOVAL.try_lock() else { return };
    // The gate and the roster are asked only while a removal is owed: no game reads on the other frames.
    let pay = removal.step(waiting, on, || {
        !crate::runtime::roster::character_roster::busy()
            && super::practice_stage::item_gate_ready(crate::runtime::game::text_base())
    });
    drop(removal);
    if !pay {
        return;
    }
    let manager = n::item_manager();
    if manager == 0 {
        return;
    }
    // Collect first: removing shifts the native active list.
    let count = n::item_active_count(manager) as usize;
    let (ids, found) = collect::<64>(count, |index| {
        let item = n::item_active(manager, index as u64);
        if item == 0 { 0 } else { n::item_object_id(item) }
    });
    for id in &ids[..found] {
        n::item_remove(manager, *id);
    }
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!("[TrainingRooms] lobby_items removed={found} active={count}"));
}
