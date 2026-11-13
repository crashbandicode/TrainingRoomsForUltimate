//! The fighter on-init wrapper (SPEC-state-06 §5.1): the fighter vtable's `on_init` slot is replaced
//! by a wrapper that calls the game's own on-init and then, for the CPU-operated fighter in native
//! Training and not during a hot swap, runs the reset hub.
//!
//! Facts (read from the owned 13.0.5 main; the `on_init` site's guard covers the words): the
//! function takes the fighter object in x0 and one more argument in x1; the fighter's entry id is
//! the `i32` at fighter+0x10. The CPU test is the game's own
//! `FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(manager, entry))`.
//! The wrapper forwards x0..x7 unchanged and returns x0, so any arguments the game passes reach it.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::model::input_delay::Clear;
use crate::model::lra_reset;
use crate::model::sites;
use crate::runtime::alloc::{HookId, HookScope};
use crate::runtime::{game, hub, published, scene};

static ORIGINAL: AtomicUsize = AtomicUsize::new(0);

type OnInit = extern "C" fn(usize, usize, usize, usize, usize, usize, usize, usize) -> usize;

/// The fighter is CPU-operated, by the game's own accessors; false for anything unresolvable.
pub(crate) unsafe fn cpu_operated(fighter: usize) -> bool {
    if fighter == 0 {
        return false;
    }
    let entry = ((fighter + lra_reset::FIGHTER_ENTRY_ID_FIELD) as *const i32).read();
    if !lra_reset::entry_id_valid(entry) {
        return false;
    }
    game::natives::is_operation_cpu(entry)
}

extern "C" fn on_init(a0: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize, a7: usize) -> usize {
    // SAFETY: `ORIGINAL` holds the guarded original on-init address (set by `install`).
    let original: OnInit = unsafe { std::mem::transmute(ORIGINAL.load(Ordering::Acquire)) };
    let result = original(a0, a1, a2, a3, a4, a5, a6, a7);
    let _scope = HookScope::enter_hook(HookId::ResetHub);
    // SPEC-core-06: the hitbox visualizer drops this fighter's sphere records (no game call; its effect module may reuse the numbers).
    crate::runtime::features::hitbox::forget_fighter(a0);
    // SPEC-input-06: the frame strip forgets what it showed (a reset, a fighter replaced), as on a save-state load.
    crate::runtime::features::input_display::fighter_initialised();
    // Package 6: the waiting-room practice engine records the entry the game just initialised.
    unsafe { crate::runtime::lobby::observe(a0) };
    // SAFETY: `a0` is the fighter object the game just initialised.
    let cpu = unsafe { cpu_operated(a0) };
    let hot_swap = published::HOT_SWAP_IN_PROGRESS.load(Ordering::Acquire);
    if lra_reset::hub_should_run(cpu, scene::is_native_training(), hot_swap) {
        hub::run(Clear::TrainingReset);
    }
    result
}

/// Replace the vtable slot with the wrapper. Only after both the slot and the original passed their
/// guards (`boot`).
pub fn install() {
    let base = game::text_base();
    ORIGINAL.store(base + sites::ON_INIT_OFFSET, Ordering::Release);
    let slot = (base + sites::ON_INIT_SLOT_OFFSET) as *mut usize;
    // SAFETY: the slot is a writable `.data` pointer that held the original's address.
    unsafe { slot.write_volatile(on_init as *const () as usize) };
}
