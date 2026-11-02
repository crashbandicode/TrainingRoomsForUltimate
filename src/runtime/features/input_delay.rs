//! Input Delay hooks (SPEC-input-03): P1's final mapped record is replaced by the delayed one after
//! the game finished mapping it. Two inline hooks inside the game's final input mapping, not a hook
//! on its entry: `mapping_begin` (after the prologue and the player-index checks) notes the out record
//! and the player index for this thread, and `mapping_end` (the one epilogue) applies the delay to that
//! record. The reasons and the checked facts are in `tests/input_mapping_sites.rs`.
//! Compiled for the Switch with `cargo skyline build`.

use std::cell::Cell;

use parking_lot::Mutex;
use skyline::hooks::InlineCtx;

use crate::model::input_delay::{self, Clear, DelayQueue, MappedInputs};
use crate::model::sites;
use crate::runtime::alloc::HookScope;
use crate::runtime::{game, published};

static QUEUE: Mutex<DelayQueue> = Mutex::new(DelayQueue::new());

thread_local! {
    /// The out record and player index of the mapping call in progress on this thread, noted by
    /// `mapping_begin` and taken by `mapping_end`. Const-initialised, no destructor: no allocation.
    static IN_PROGRESS: Cell<(usize, i64)> = const { Cell::new((0, 0)) };
}

/// At `mov x19, x2`: x2 is the out record, x23 the sign-extended player index.
#[skyline::hook(offset = sites::INPUT_MAPPING_BEGIN_OFFSET, inline)]
pub unsafe fn mapping_begin(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter();
    let note = (ctx.registers[2].x() as usize, ctx.registers[23].x() as i64);
    let _ = IN_PROGRESS.try_with(|cell| cell.set(note));
}

/// At the epilogue's first instruction: the record is complete.
#[skyline::hook(offset = sites::INPUT_MAPPING_END_OFFSET, inline)]
pub unsafe fn mapping_end(_ctx: &mut InlineCtx) {
    let _scope = HookScope::enter();
    // A path that never passed `mapping_begin` (player index -1) has no note.
    let (out, player_idx) = IN_PROGRESS.try_with(|cell| cell.replace((0, 0))).unwrap_or((0, 0));
    if out == 0 || !input_delay::applies(player_idx as i32, game::is_training_mode()) {
        return;
    }
    let out = out as *mut MappedInputs;
    let mut queue = QUEUE.lock();
    *out = queue.poll(published::input_delay(), published::input_owners(), *out);
}

/// Empty the queue: the reset hub (decision B8) and, from package 3, every save-state load. Returns
/// how many queued records were dropped.
pub fn clear(why: Clear) -> usize {
    QUEUE.lock().clear(why)
}
