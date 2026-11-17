//! L+R+A Reset gate (SPEC-state-06 §4): inside the game's Training reset check, clear register x8
//! when the setting is Off so the check fails. CRASH-0900: also while a practice fighter loads, with a
//! notice (never silent).

use std::sync::atomic::{AtomicBool, Ordering};

use skyline::hooks::InlineCtx;

use crate::model::{lra_reset, sites};
use crate::runtime::alloc::{HookId, HookScope};
use crate::runtime::{published, scene};

/// The previous check refused a requested reset because a fighter loads (one notice per refusal).
static REFUSED: AtomicBool = AtomicBool::new(false);

#[skyline::hook(offset = sites::RESET_CHECK_OFFSET, inline)]
pub unsafe fn reset_check(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::ResetCheck);
    // CRASH-0900: no native reset while a practice fighter loads (lock-free: one atomic and two
    // guarded byte reads; the game thread, inside the game's own reset check). Outside native
    // Training `x8_write` leaves the register alone whatever this reads.
    let loading = fighter_loading();
    let was_refused = REFUSED.load(Ordering::Acquire);
    let (write, refused, notice) = lra_reset::gate(ctx.registers[8].x(), published::lra_reset(), scene::is_training(), loading, was_refused);
    REFUSED.store(refused, Ordering::Release);
    if notice {
        // The notice mailbox: allocation-free, any thread (`notices::post`).
        super::notices::post(lra_reset::REFUSED_HEADER, lra_reset::REFUSED_BODY, 120, crate::model::notify::DEFAULT_COLOR);
    }
    if let Some(value) = write {
        ctx.registers[8].set_x(value);
    }
}

/// The roster's install suspends practice, or entry 0 / 1 has a loader away from rest.
unsafe fn fighter_loading() -> bool {
    if crate::runtime::roster::character_roster::busy() {
        return true;
    }
    [0u32, 1].into_iter().any(|slot| {
        crate::runtime::roster::character_roster::practice_entry(slot).is_some_and(|entry| {
            let state = crate::engine::seams::native_fields::read::<u8>(entry as usize, lra_reset::LOADER_STATE_FIELD);
            !lra_reset::loader_resting(state)
        })
    })
}
