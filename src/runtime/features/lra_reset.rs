//! L+R+A Reset gate (SPEC-state-06 §4): inside the game's Training reset check, clear register x8
//! when the setting is Off so the check fails. NOT YET SWITCH-COMPILED.

use skyline::hooks::InlineCtx;

use crate::model::{lra_reset, sites};
use crate::runtime::alloc::{HookId, HookScope};
use crate::runtime::{published, scene};

#[skyline::hook(offset = sites::RESET_CHECK_OFFSET, inline)]
pub unsafe fn reset_check(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter(HookId::ResetCheck);
    if let Some(value) = lra_reset::x8_write(published::lra_reset(), scene::is_training()) {
        ctx.registers[8].set_x(value);
    }
}
