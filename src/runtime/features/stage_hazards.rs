//! Stage Hazards (SPEC-state-07): inline hook at the `cbz` of the stage-loading routine; writes the
//! game's hazard rule byte just before the game reads it. NOT YET SWITCH-COMPILED.

use std::sync::atomic::{AtomicUsize, Ordering};

use skyline::hooks::InlineCtx;

use crate::model::stage_hazards::{self, HOOK_SITE_OFFSET};
use crate::runtime::alloc::HookScope;
use crate::runtime::{published, scene};

/// Absolute address of the rule byte, set at boot from the guarded instruction pair.
static FLAG: AtomicUsize = AtomicUsize::new(0);

pub fn set_flag_address(address: usize) {
    FLAG.store(address, Ordering::Release);
}

#[skyline::hook(offset = HOOK_SITE_OFFSET, inline)]
pub unsafe fn hazard_rule(_ctx: &mut InlineCtx) {
    let _scope = HookScope::enter();
    let flag = FLAG.load(Ordering::Acquire);
    if flag == 0 {
        return;
    }
    if let Some(byte) = stage_hazards::rule_byte(published::stage_hazards(), scene::is_native_training()) {
        // SAFETY: `flag` was derived from the 64-byte-guarded `adrp`/`add` pair and equals the
        // pinned 13.0.5 address (`boot` checks both) inside the game's `.bss`.
        (flag as *mut u8).write_volatile(byte);
    }
}
