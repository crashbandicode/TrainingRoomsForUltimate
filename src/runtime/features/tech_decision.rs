//! Read-only exact-decision snapshot. No calibration or grading policy lives here.
use super::tech_native;
use crate::model::cpu::hash::hash40;
use crate::model::tech_launch::{ContactReceipt, DecisionSnapshot};
use crate::model::{sites_pkg2 as p2, sites_pkg3 as p3, sites_tech_launch as sites};
use crate::runtime::{
    game::{self, natives},
    state_natives,
};

fn pointer(value: usize) -> bool {
    value >= 0x1000 && value & 7 == 0 && value >> 47 == 0
}
/// Export thunks at2001260/20013c0/20013d0 load accessor+48 then slots178/228/230.
/// Owned relocated Control vtable and all three concrete64-byte windows are pinned.
unsafe fn control(accessor: usize) -> Option<usize> {
    if !pointer(accessor) {
        return None;
    }
    let module = ((accessor + 0x48) as *const usize).read();
    if !pointer(module) {
        return None;
    }
    let vptr = (module as *const usize).read();
    let base = game::text_base();
    if vptr != base + 0x4f6bd90 {
        return None;
    }
    for (slot, target) in [
        (0x178, sites::STICK_X_OFFSET),
        (0x228, sites::GUARD_COUNT_OFFSET),
        (0x230, sites::GUARD_PREVIOUS_OFFSET),
    ] {
        if !game::granted(target) || ((vptr + slot) as *const usize).read() != base + target {
            return None;
        }
    }
    pointer(((module + 0x110) as *const usize).read()).then_some(module)
}

/// Existing guarded status accessor; caller first proves a fresh owned player.
pub(super) unsafe fn entered_status(accessor: usize) -> Option<i32> {
    game::granted(p2::STATUS_KIND_OFFSET).then(|| natives::status_kind(accessor))
}

pub(super) unsafe fn snapshot(
    accessor: usize,
    attempt: u64,
    contact: ContactReceipt,
) -> DecisionSnapshot {
    let source_status =
        game::granted(p2::STATUS_KIND_OFFSET).then(|| natives::status_kind(accessor));
    let damage_fall = source_status
        .zip(tech_native::id("FIGHTER_STATUS_KIND_DAMAGE_FALL"))
        .map(|(actual, fall)| actual == fall);
    let reaction = if game::granted(p2::WORK_GET_FLOAT_OFFSET) {
        tech_native::id("FIGHTER_INSTANCE_WORK_ID_FLOAT_DAMAGE_REACTION_FRAME")
            .map(|id| natives::get_float(accessor, id))
    } else {
        None
    };
    let params = game::granted(p3::STATE_GET_PARAM_INT_OFFSET);
    let window_base = params.then(|| {
        state_natives::get_param_int(accessor, hash40("common"), hash40("passive_trigger_frame"))
    });
    let window_multiplier = game::granted(p2::WORK_GET_PARAM_FLOAT_OFFSET)
        .then(|| natives::get_param_float(accessor, hash40("passive_trigger_frame_mul"), 0));
    let lockout =
        params.then(|| state_natives::get_param_int(accessor, hash40("common"), 0x14459066dd));
    let module = control(accessor);
    let button = tech_native::id("CONTROL_PAD_BUTTON_GUARD");
    let guard_count = module
        .zip(button)
        .and_then(|(m, b)| natives::tech_guard_count(m, b));
    let previous_guard_gap = module
        .zip(button)
        .and_then(|(m, b)| natives::tech_guard_previous(m, b));
    let stick_x = module.and_then(|m| natives::tech_stick_x(m));
    DecisionSnapshot {
        attempt,
        contact,
        source_status,
        entered_status: None,
        guard_count,
        previous_guard_gap,
        guard_event: None,
        window_base,
        window_multiplier,
        lockout,
        reaction,
        damage_fall,
        stick_x,
        press_frame_offset: None,
    }
}
