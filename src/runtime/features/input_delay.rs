//! Input Delay hooks (SPEC-input-03): P1's final mapped record is replaced by the delayed one after
//! the game finished mapping it. The hooks are in the two callers of the game's final input mapping,
//! not in the function (whose inner points belong to the CPU pilot plugin): a `note` hook just before
//! each call notes the out record and the slot for this thread, an `apply` hook right after the call
//! returns applies the delay to that record, after everything the function and the pilot did. The
//! reasons and the checked facts are in `tests/input_mapping_callers.rs`.
//!
//! The queue, the owner-mask rules and the pause trace are `model::input_adapter::InputAdapter` (the host
//! tests drive the same code); the hooks only call its methods under one lock.

use hos_lock::Mutex;
use skyline::hooks::InlineCtx;

use crate::model::input_adapter::InputAdapter;
use crate::model::input_delay::{self, Clear, MappedInputs};
use crate::model::sites;
use crate::runtime::alloc::{HookId, HookScope};
#[cfg(feature = "diag")]
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::{chords, input_display, recorder};
use crate::runtime::{published, scene, ui};

static ADAPTER: Mutex<InputAdapter> = Mutex::new(InputAdapter::new());

// Switch std uses OS TLS boxes even for const/no-destructor thread_local!
// values. nn::os scalar slots instead live inside each SDK ThreadType. The
// exact 13.0.5 SDK leaves get/set as five-instruction leaf functions; allocation
// clears the slot in existing threads, and CreateThread zeros future slots.
// See eden/runs/ui-input-contract; no hard-coded ThreadType offsets are used.
use crate::model::input_pending::Pending;
use std::sync::OnceLock;

static PENDING: OnceLock<Pending> = OnceLock::new();
use crate::runtime::native_tls::NativeTls;

/// Boot only, before the four input hooks are installed. Failure leaves no keys.
pub fn init() -> Result<(), u32> {
    if PENDING.get().is_some() { return Ok(()); }
    let pending = Pending::reserve(&NativeTls)?;
    if let Err(unpublished) = PENDING.set(pending) { unpublished.release(&NativeTls); }
    Ok(())
}

/// `state` is the fourth argument of the call (x3): the controller holder, a struct whose field at +0x10 is
/// the player's Controller. Package 5's menu keeps the holder itself, package 4 reads the Controller from it.
fn note(out: usize, slot: i64, state: usize) {
    let Some(pending) = PENDING.get() else { return };
    pending.note(&NativeTls, out, slot, state);
    let controller = if state == 0 { 0 } else { unsafe { ((state + 0x10) as *const usize).read() } };
    // Package 6: the waiting-room practice engine's once-per-frame step, before this frame's input processing (Player 1's poll).
    if slot == 0 {
        crate::runtime::lobby::update();
    }
    pending.set_controller(&NativeTls, controller);
    // Chords read the raw controller before the delay, so they are never delayed (SPEC-input-04 §5).
    if slot == 0 && controller != 0 && scene::is_training() {
        unsafe { chords::on_poll(controller) };
    }
}

/// Caller A, before the call: x20 is the slot, x23 + x22 the slot's out record.
#[skyline::hook(offset = sites::INPUT_CALLER_A_NOTE_OFFSET, inline)]
pub unsafe fn caller_a_note(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::InputCallerANote);
    note(ctx.registers[23].x().wrapping_add(ctx.registers[22].x()) as usize, ctx.registers[20].x() as i64, ctx.registers[3].x() as usize);
}

/// Caller B, before the call: x8 is the out record, x2 the slot (a 32-bit index).
#[skyline::hook(offset = sites::INPUT_CALLER_B_NOTE_OFFSET, inline)]
pub unsafe fn caller_b_note(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::InputCallerBNote);
    note(ctx.registers[8].x() as usize, i64::from(ctx.registers[2].w() as i32), ctx.registers[3].x() as usize);
}

fn apply() {
    // A pass that made no call has no note.
    let Some(pending) = PENDING.get() else { return };
    let (out, slot, holder) = pending.take(&NativeTls);
    // Package 6: the controller operating the CPU (native Behavior = Control) reaches the roster cycle's operator handlers.
    if out != 0 && slot == 1 && scene::is_training() {
        let controller = pending.controller(&NativeTls);
        // SAFETY: `out` is the 8-byte record the game just wrote through the call; `controller` is the live Controller or 0.
        unsafe { crate::runtime::roster::handle_operator_input(out as *mut MappedInputs, controller) };
    }
    if out == 0 || !input_delay::applies(slot as i32, scene::is_training()) {
        return;
    }
    // The menu reads P1 before the delay (SPEC-core-03 §4.1 sub-handler order); it sets the MENU owner,
    // which makes the poll below output neutral.
    ui::input::on_p1_poll(holder);
    // Package 6: the roster's cycle chords and the Kirby hat shortcut see P1's record before the delay (and may consume the chord).
    if slot == 0 {
        let controller = pending.controller(&NativeTls);
        // SAFETY: `out` is the 8-byte record the game just wrote through the call; `controller` is the live Controller or 0.
        if unsafe { crate::runtime::features::states::on_quick_reset_input(controller, out as *mut MappedInputs) } {
            pending.take_controller(&NativeTls);
            return;
        }
        unsafe { crate::runtime::roster::handle_p1_input(out as *mut MappedInputs, controller) };
    }
    let out = out as *mut MappedInputs;
    let mut adapter = ADAPTER.lock();
    // SAFETY: `out` is the 8-byte record the game just wrote through the call.
    let (record, check) = adapter.poll(&published::INPUT_OWNERS, published::input_delay(), unsafe { *out });
    // SAFETY: as above.
    unsafe { *out = record };
    drop(adapter);
    // Package 6: the CPU-as-P1 fallback writes its pad into P1's record after the delay (nothing while the pilot is present).
    unsafe { crate::runtime::p1::input(slot as i32, out) };
    #[cfg(feature = "diag")]
    if let Some(check) = check {
        diagnostic(format_args!("[TrainingRooms] pause_owner resume_checked delay={} leaked={}", check.delay, check.leaked));
    }
    #[cfg(not(feature = "diag"))]
    let _ = check;
    // After the delay: the input display and the recorder see the delayed record (SPEC-input-03 §5.2).
    if slot == 0 {
        let controller = pending.take_controller(&NativeTls);
        // The display first (it logs what P1 pressed), then the recorder (which may neutralise out).
        unsafe {
            input_display::on_p1_poll(controller, out);
            // The Replay Window's own controls (nothing unless its editor is in the foreground).
            crate::runtime::macros::on_p1_input(controller, out);
            recorder::on_p1_poll(out);
        }
    }
}

/// Caller A, at the loop increment the call returns to.
#[skyline::hook(offset = sites::INPUT_CALLER_A_APPLY_OFFSET, inline)]
pub unsafe fn caller_a_apply(_ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::InputCallerAApply);
    apply();
}

/// Caller B, the instruction after the call.
#[skyline::hook(offset = sites::INPUT_CALLER_B_APPLY_OFFSET, inline)]
pub unsafe fn caller_b_apply(_ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::InputCallerBApply);
    apply();
}

/// Empty the queue: the reset hub (decision B8) and, from package 3, every save-state load. Returns
/// how many queued records were dropped.
pub fn clear(why: Clear) -> usize {
    ADAPTER.lock().clear(why)
}

/// The game's Training menu flag as the UI tick samples it (`ui::state::PAUSE_ACTIVE`, SPEC-core-03 section
/// 2.4): an edge takes over P1's input and empties the queue (begin) or releases it (end).
pub fn pause_flag(now: bool) {
    use crate::model::input_adapter::PauseEdge;
    let edge = ADAPTER.lock().pause_flag(&published::INPUT_OWNERS, now, published::input_delay());
    #[cfg(feature = "diag")]
    match edge {
        Some(PauseEdge::Begin { dropped }) => diagnostic(format_args!(
            "[TrainingRooms] pause_owner begin queue_dropped={dropped} owners={:#04x}",
            published::INPUT_OWNERS.load(std::sync::atomic::Ordering::Acquire)
        )),
        Some(PauseEdge::End(report)) => diagnostic(format_args!(
            "[TrainingRooms] pause_owner end queued_at_begin={} owned_polls={} owned_non_neutral={} owners={:#04x}",
            report.queued_at_begin,
            report.owned_polls,
            report.owned_non_neutral,
            published::INPUT_OWNERS.load(std::sync::atomic::Ordering::Acquire)
        )),
        None => {}
    }
    #[cfg(not(feature = "diag"))]
    let _ = (edge, PauseEdge::Begin { dropped: 0 });
}
