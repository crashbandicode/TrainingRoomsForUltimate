//! Native input ownership, independent of injected panes, workers and textures.
//! Every native pointer is resolved and consumed within its live Calculate callback.
use std::sync::atomic::Ordering;
use skyline::nn::ui2d::Layout;
use crate::runtime::{features::input_delay, ui::{pane, registry, state}};

/// Sample native pause panes only during their live layout callback.
///
/// # Safety
/// `layout` must be non-null and reference an initialized live layout whose
/// name and pane tree remain readable for this call, without concurrent native
/// mutation/destruction. The caller must report each new layout lifetime with
/// `registration = true`, including when its address is reused.
pub unsafe fn observe(layout: *mut Layout, registration: bool) {
    let epoch = input_delay::native_pause_epoch();
    let Some(name) = registry::layout_name(layout) else { return };
    let waiting = name == b"info_waiting";
    if name != b"info_training" && !waiting { return; }
    let active = if waiting { crate::runtime::lobby::training::lobby::requested() }
        else { crate::runtime::game::is_training_mode() };
    if !active {
        input_delay::retire_pause_layout(epoch, layout as usize);
        return;
    }
    let help = pane::find((*layout).root_pane,
        if waiting { b"option_menu_setting" } else { b"L_staying_help" });
    if waiting {
        // Waiting busy has its existing, distinct owner. Retire a previous
        // Training owner before invoking its possibly reentrant session cleanup.
        if !input_delay::native_pause_sample(epoch, layout as usize, registration, Some(false)) { return; }
        let busy = help.is_null() || pane::pos_x(help) != -540.0
            || crate::runtime::lobby::game_mode::popup_busy().unwrap_or(true);
        crate::runtime::lobby::training::lobby::set_native_ui_busy(busy);
        state::PAUSE_ACTIVE.store(busy, Ordering::Release);
    } else {
        let paused = (!help.is_null()).then(|| (pane::pos_y(help) + 80.0).abs() > 0.5);
        input_delay::native_pause_sample(epoch, layout as usize, registration, paused);
    }
}
