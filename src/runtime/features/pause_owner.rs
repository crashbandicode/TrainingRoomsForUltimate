//! The native pause as an input owner (SPEC-input-03 §5.1, review R2 of package 1): two inline hooks at the
//! begin and the end of the Training scene's `StatePause` (`model::sites_pause` has the facts and the
//! guards). The hooks only call `input_delay::pause_begin` / `pause_end`, which run the adapter that the
//! host tests drive (`tests/input_pause.rs`): the begin sets the `VANILLA_PAUSE` bit of
//! `published::INPUT_OWNERS` and empties the delay queue at once, the end releases the bit.
//! NOT YET SWITCH-COMPILED.

use skyline::hooks::InlineCtx;

use crate::model::sites_pause;
use crate::runtime::alloc::{HookId, HookScope};
use crate::runtime::features::input_delay;

/// `StatePause::enter`, just before the game's pause-begin routine.
#[skyline::hook(offset = sites_pause::PAUSE_ENTER_OFFSET, inline)]
pub unsafe fn pause_enter(_ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::PauseEnter);
    input_delay::pause_begin();
}

/// `StatePause::exit`'s epilogue, after the game cleared its pause flags.
#[skyline::hook(offset = sites_pause::PAUSE_EXIT_OFFSET, inline)]
pub unsafe fn pause_exit(_ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::PauseExit);
    input_delay::pause_end();
}
