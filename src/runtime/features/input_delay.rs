//! Input Delay hooks (SPEC-input-03): P1's final mapped record is replaced by the delayed one after
//! the game finished mapping it. The hooks are in the two callers of the game's final input mapping,
//! not in the function (whose inner points belong to the CPU pilot plugin): a `note` hook just before
//! each call notes the out record and the slot for this thread, an `apply` hook right after the call
//! returns applies the delay to that record, after everything the function and the pilot did. The
//! reasons and the checked facts are in `tests/input_mapping_callers.rs`.

use std::cell::Cell;

use parking_lot::Mutex;
use skyline::hooks::InlineCtx;

use crate::model::input_adapter::InputAdapter;
use crate::model::input_delay::{self, Clear, MappedInputs};
use crate::model::sites;
use crate::runtime::alloc::{HookId, HookScope};
#[cfg(feature = "diag")]
use crate::runtime::diag_log::diagnostic;
use crate::runtime::{published, scene};

/// The delay queue, the owner-mask rules and the pause trace (`model::input_adapter`): the hooks only
/// call its methods, so the host tests drive the same code.
static ADAPTER: Mutex<InputAdapter> = Mutex::new(InputAdapter::new());

thread_local! {
    /// The out record and slot of the mapping call in progress on this thread, noted before the call
    /// and taken after it. Const-initialised, no destructor: no allocation.
    static IN_PROGRESS: Cell<(usize, i64)> = const { Cell::new((0, 0)) };
}

fn note(out: usize, slot: i64) {
    let _ = IN_PROGRESS.try_with(|cell| cell.set((out, slot)));
}

/// Caller A, before the call: x20 is the slot, x23 + x22 the slot's out record.
#[skyline::hook(offset = sites::INPUT_CALLER_A_NOTE_OFFSET, inline)]
pub unsafe fn caller_a_note(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter(HookId::InputCallerANote);
    note(ctx.registers[23].x().wrapping_add(ctx.registers[22].x()) as usize, ctx.registers[20].x() as i64);
}

/// Caller B, before the call: x8 is the out record, x2 the slot (a 32-bit index).
#[skyline::hook(offset = sites::INPUT_CALLER_B_NOTE_OFFSET, inline)]
pub unsafe fn caller_b_note(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter(HookId::InputCallerBNote);
    note(ctx.registers[8].x() as usize, i64::from(ctx.registers[2].w() as i32));
}

fn apply() {
    // A pass that made no call has no note.
    let (out, slot) = IN_PROGRESS.try_with(|cell| cell.replace((0, 0))).unwrap_or((0, 0));
    if out == 0 || !input_delay::applies(slot as i32, scene::is_training()) {
        return;
    }
    let out = out as *mut MappedInputs;
    let mut adapter = ADAPTER.lock();
    // SAFETY: `out` is the 8-byte record the game just wrote through the call.
    let (record, check) = adapter.poll(&published::INPUT_OWNERS, published::input_delay(), unsafe { *out });
    // SAFETY: as above.
    unsafe { *out = record };
    #[cfg(feature = "diag")]
    if let Some(check) = check {
        diagnostic(format_args!("[TrainingRooms] pause_owner resume_checked delay={} leaked={}", check.delay, check.leaked));
    }
    #[cfg(not(feature = "diag"))]
    let _ = check;
}

/// Caller A, at the loop increment the call returns to.
#[skyline::hook(offset = sites::INPUT_CALLER_A_APPLY_OFFSET, inline)]
pub unsafe fn caller_a_apply(_ctx: &mut InlineCtx) {
    let _scope = HookScope::enter(HookId::InputCallerAApply);
    apply();
}

/// Caller B, the instruction after the call.
#[skyline::hook(offset = sites::INPUT_CALLER_B_APPLY_OFFSET, inline)]
pub unsafe fn caller_b_apply(_ctx: &mut InlineCtx) {
    let _scope = HookScope::enter(HookId::InputCallerBApply);
    apply();
}

/// Empty the queue: the reset hub (decision B8) and, from package 3, every save-state load. Returns
/// how many queued records were dropped.
pub fn clear(why: Clear) -> usize {
    ADAPTER.lock().clear(why)
}

/// The game's pause began (the hook at `StatePause`'s enter): take over P1's input and empty the queue.
pub fn pause_begin() {
    let dropped = ADAPTER.lock().pause_begin(&published::INPUT_OWNERS);
    #[cfg(feature = "diag")]
    diagnostic(format_args!(
        "[TrainingRooms] pause_owner begin queue_dropped={dropped} owners={:#04x}",
        published::INPUT_OWNERS.load(std::sync::atomic::Ordering::Acquire)
    ));
    #[cfg(not(feature = "diag"))]
    let _ = dropped;
}

/// The game's pause ended (the hook at `StatePause`'s exit): release P1's input.
pub fn pause_end() {
    let report = ADAPTER.lock().pause_end(&published::INPUT_OWNERS, published::input_delay());
    #[cfg(feature = "diag")]
    diagnostic(format_args!(
        "[TrainingRooms] pause_owner end queued_at_begin={} owned_polls={} owned_non_neutral={} owners={:#04x}",
        report.queued_at_begin,
        report.owned_polls,
        report.owned_non_neutral,
        published::INPUT_OWNERS.load(std::sync::atomic::Ordering::Acquire)
    ));
    #[cfg(not(feature = "diag"))]
    let _ = report;
}
