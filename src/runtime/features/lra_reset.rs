//! L+R+A Reset gate (SPEC-state-06 §4): inside the game's Training reset check, clear register x8
//! when the setting is Off so the check fails. CRASH-0900: also while a practice fighter loads, with a
//! notice (never silent).

use std::sync::atomic::{AtomicBool, Ordering};

use skyline::hooks::InlineCtx;

use crate::model::{lra_reset, sites};
use crate::runtime::alloc::{HookId, HookScope};
use crate::runtime::{game, published, scene};

/// The previous check refused a requested reset because a fighter loads, and its notice was posted (one notice
/// per refusal; a post the full mailbox dropped is tried again on the next check).
static REFUSED: AtomicBool = AtomicBool::new(false);

#[skyline::hook(offset = sites::RESET_CHECK_OFFSET, inline)]
pub unsafe fn reset_check(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::ResetCheck);
    // CRASH-0900: no native reset while a practice fighter loads (lock-free: one atomic, and only with the guarded
    // entry lookup two guarded reads per entry; the game thread, inside the game's own reset check). Outside native
    // Training `x8_write` leaves the register alone whatever this reads.
    let loading = fighter_loading();
    let was_refused = REFUSED.load(Ordering::Acquire);
    let (write, refused, notice) = lra_reset::gate(ctx.registers[8].x(), published::lra_reset(), scene::is_training(), loading, was_refused);
    let mut latched = refused;
    if notice {
        // A required notice (review R2): Combo Display Off does not clear it. The mailbox is allocation-free.
        // restriction: character-swap-busy (the same load refuses a pick with this text)
        let body = crate::engine::mii::ordinary_cpu::SWAP_BUSY_NOTICE;
        latched = super::notices::post_required(lra_reset::REFUSED_HEADER, body, 120, crate::model::notify::DEFAULT_COLOR);
    }
    REFUSED.store(latched, Ordering::Release);
    if let Some(value) = write {
        ctx.registers[8].set_x(value);
    }
}

/// The roster's suspending install, or (only with the guarded entry lookup, review R1) entry 0 / 1 whose loader is
/// away from rest (`lra_reset::fighter_loading`).
unsafe fn fighter_loading() -> bool {
    lra_reset::fighter_loading(
        crate::runtime::roster::character_roster::busy(),
        game::entry_lookup_usable(),
        |slot| unsafe { loader_state(slot) },
    )
}

/// Entry `slot`'s loader byte through the guarded lookup (`game::natives::fighter_entry`, which answers 0 unless
/// `NativeGroup::Entry` was granted): `None` no live entry for that slot, else the `+0x591c` byte (`None` inside when
/// unreadable).
unsafe fn loader_state(slot: u32) -> Option<Option<u8>> {
    let entry = game::natives::fighter_entry(slot as i32);
    if entry == 0 || crate::engine::seams::native_fields::read::<u32>(entry, 0xe0) != Some(slot) {
        return None;
    }
    Some(crate::engine::seams::native_fields::read::<u8>(entry, lra_reset::LOADER_STATE_FIELD))
}
