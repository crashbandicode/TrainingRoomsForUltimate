//! L+R+A Reset gate (SPEC-state-06 §4): inside the game's Training reset check, clear register x8
//! when the setting is Off so the check fails. CRASH-0900 (operator 2026-11-17 11:40Z): a reset asked for while a
//! practice fighter loads is deferred and fired here once the load settles (`lra_reset::defer_step`).

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use skyline::hooks::InlineCtx;

use crate::model::{lra_reset, sites};
use crate::runtime::alloc::{HookId, HookScope};
use crate::runtime::{published, scene};

/// The deferred reset (game thread only: this hook is its one reader and writer). `u64::MAX`: none waiting.
static PENDING_SINCE: AtomicU64 = AtomicU64::new(u64::MAX);
static PENDING_ABORTS: AtomicU32 = AtomicU32::new(0);
static PENDING_GENERATION: AtomicU32 = AtomicU32::new(0);
static PENDING_NOTIFIED: AtomicBool = AtomicBool::new(false);

#[skyline::hook(offset = sites::RESET_CHECK_OFFSET, inline)]
pub unsafe fn reset_check(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::ResetCheck);
    // No fighter-entry lookup (review R1): the roster's suspending INSTALL token, its abort count, the lifecycle
    // generation (review R3), the menu owner bit and the save-state load flag are atomics; the existing guarded scene
    // predicate and the system tick are the only calls.
    let facts = lra_reset::Facts {
        x8: ctx.registers[8].x(),
        lra_reset_on: published::lra_reset(),
        native_training: scene::is_training(),
        loading: crate::runtime::roster::character_roster::busy(),
        menu_open: published::input_owners().contains(crate::model::input_delay::Owners::MENU),
        save_state_loading: super::states::is_loading(),
        install_aborts: crate::runtime::roster::character_roster::install_aborts(),
        generation: lra_reset::GENERATION.load(Ordering::Acquire),
        now_ms: crate::runtime::clock::uptime_ms(),
    };
    let since = PENDING_SINCE.load(Ordering::Acquire);
    let mut pending = (since != u64::MAX).then(|| lra_reset::Pending {
        since_ms: since,
        aborts: PENDING_ABORTS.load(Ordering::Acquire),
        generation: PENDING_GENERATION.load(Ordering::Acquire),
        notified: PENDING_NOTIFIED.load(Ordering::Acquire),
    });
    let step = lra_reset::defer_step(&mut pending, facts);
    if step.post_notice {
        // A required notice (review R2): Combo Display Off does not clear it. Marked sent only once the mailbox took
        // it, so a full mailbox is retried on the next check.
        let sent = super::notices::post_required(lra_reset::DEFERRED_HEADER, lra_reset::DEFERRED_BODY, 120, crate::model::notify::DEFAULT_COLOR);
        pending.iter_mut().for_each(|p| p.notified = sent);
    }
    match pending {
        Some(p) => {
            PENDING_ABORTS.store(p.aborts, Ordering::Release);
            PENDING_GENERATION.store(p.generation, Ordering::Release);
            PENDING_NOTIFIED.store(p.notified, Ordering::Release);
            PENDING_SINCE.store(p.since_ms, Ordering::Release);
        }
        None => PENDING_SINCE.store(u64::MAX, Ordering::Release),
    }
    if let Some(value) = step.write {
        ctx.registers[8].set_x(value);
    }
}
