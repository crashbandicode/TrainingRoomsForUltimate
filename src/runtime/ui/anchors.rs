//! The two UI anchors (SPEC-core-05 §4.1): inline hooks in the UI manager on the instruction before each
//! holder layout's virtual Calculate, so the tick runs after the holder's own animation update and before
//! its Calculate. The per-frame pass runs on the UI job thread (holder in x23); the manager insert runs on
//! the main thread for a newly registered layout (holder in x21).

use skyline::hooks::InlineCtx;

use crate::model::hook_stats::{HookId, HookScope};
use crate::model::sites_ui::{UI_PASS_CALCULATE_OFFSET, UI_REGISTER_CALCULATE_OFFSET};
use crate::runtime::ui::{registry, tick};

pub fn install() {
    skyline::install_hook!(ui_pass_calculate);
    skyline::install_hook!(ui_register_calculate);
}

#[skyline::hook(offset = UI_PASS_CALCULATE_OFFSET, inline)]
unsafe fn ui_pass_calculate(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::UiPassCalculate);
    if let Some(layout) = registry::layout_of(ctx.registers[23].x() as *const u8) {
        #[cfg(feature = "diag")]
        context_reference(ctx, layout, false);
        tick::tick(layout, false);
    }
}

#[skyline::hook(offset = UI_REGISTER_CALCULATE_OFFSET, inline)]
unsafe fn ui_register_calculate(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::UiRegisterCalculate);
    if let Some(layout) = registry::layout_of(ctx.registers[21].x() as *const u8) {
        #[cfg(feature = "diag")]
        context_reference(ctx, layout, true);
        tick::tick(layout, true);
    }
}
/// Same-boot reference for Q15 constructor observations. Identifiers are opaque;
/// equality with a constructor sample must be established from the retained log.
/// Requires the async-writer readiness accessor supplied by Slim 23ec705.
#[cfg(feature = "diag")]
unsafe fn context_reference(ctx: &InlineCtx, layout: *mut skyline::nn::ui2d::Layout, registration: bool) {
    use core::sync::atomic::{AtomicUsize, Ordering};
    static SEEN: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];
    let Some(name) = registry::layout_name(layout) else { return };
    let name = match name {
        b"info_training" => "info_training",
        b"info_waiting" => "info_waiting",
        _ => return,
    };
    if !crate::runtime::diag_log::async_writer_ready() { return; }
    let Ok(previous) = SEEN[usize::from(registration)].fetch_update(Ordering::Relaxed, Ordering::Relaxed,
        |n| (n < 8).then_some(n.saturating_add(1))) else { return; };
    crate::runtime::threads::headroom("q15_ui_context");
    let thread = skyline::nn::os::GetCurrentThread() as usize;
    crate::runtime::diag_log::diagnostic(format_args!(
        "[Q15UiContext] registration={registration} ordinal={} name={name} thread={thread:#x} sp={:#x} layout={:#x} holder={:#x} draw_info={:#x}",
        previous + 1, ctx.sp.x(), layout as usize,
        ctx.registers[if registration { 21 } else { 23 }].x(), ctx.registers[19].x()));
}
