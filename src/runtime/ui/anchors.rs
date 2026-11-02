//! The two UI anchors (SPEC-core-05 §4.1): inline hooks in the UI manager on the instruction before each
//! holder layout's virtual Calculate, so the tick runs after the holder's own animation update and before
//! its Calculate. The per-frame pass runs on the UI job thread (holder in x23); the manager insert runs on
//! the main thread for a newly registered layout (holder in x21).

use skyline::hooks::InlineCtx;

use crate::model::sites_ui::{UI_PASS_CALCULATE_OFFSET, UI_REGISTER_CALCULATE_OFFSET};
use crate::runtime::ui::{registry, tick};

pub fn install() {
    skyline::install_hook!(ui_pass_calculate);
    skyline::install_hook!(ui_register_calculate);
}

#[skyline::hook(offset = UI_PASS_CALCULATE_OFFSET, inline)]
unsafe fn ui_pass_calculate(ctx: &mut InlineCtx) {
    if let Some(layout) = registry::layout_of(ctx.registers[23].x() as *const u8) {
        tick::tick(layout, false);
    }
}

#[skyline::hook(offset = UI_REGISTER_CALCULATE_OFFSET, inline)]
unsafe fn ui_register_calculate(ctx: &mut InlineCtx) {
    if let Some(layout) = registry::layout_of(ctx.registers[21].x() as *const u8) {
        tick::tick(layout, true);
    }
}