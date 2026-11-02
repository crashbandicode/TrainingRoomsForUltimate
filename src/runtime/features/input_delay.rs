//! Input Delay hook (SPEC-input-03): after the game maps P1's input, the record is replaced by the
//! delayed one. NOT YET SWITCH-COMPILED. The function's signature is the one SPEC-input-01 §4 gives
//! (mapping table, player index, out record, controller struct, bool); confirm the return type
//! (void) on the first Switch build.

use std::ffi::c_void;

use parking_lot::Mutex;

use crate::model::input_delay::{self, Clear, DelayQueue, MappedInputs};
use crate::model::sites;
use crate::runtime::alloc::HookScope;
use crate::runtime::{game, published};

static QUEUE: Mutex<DelayQueue> = Mutex::new(DelayQueue::new());

#[skyline::hook(offset = sites::FINAL_INPUT_MAPPING_OFFSET)]
pub unsafe fn final_input_mapping(
    mappings: *mut c_void,
    player_idx: i32,
    out: *mut MappedInputs,
    controller: *mut c_void,
    flag: bool,
) {
    call_original!(mappings, player_idx, out, controller, flag);
    let _scope = HookScope::enter();
    if out.is_null() || !input_delay::applies(player_idx, game::is_training_mode()) {
        return;
    }
    let mut queue = QUEUE.lock();
    *out = queue.poll(published::input_delay(), published::input_owners(), *out);
}

/// Empty the queue: the reset hub (decision B8) and, from package 3, every save-state load. Returns
/// how many queued records were dropped.
pub fn clear(why: Clear) -> usize {
    QUEUE.lock().clear(why)
}
