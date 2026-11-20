//! Read-only exact-decision snapshot. No calibration or grading policy lives here.
use super::tech_native;
use crate::model::cpu::hash::hash40;
use crate::model::sha256::Sha256;
use crate::model::tech::session::{timed_guard_would_pass, PASSIVE_SPEED_LIMIT};
use crate::model::tech_launch::{ContactReceipt, DecisionSnapshot};
use crate::model::{
    sites_pkg2 as p2, sites_pkg3 as p3, sites_tech_launch as sites,
    sites_tech_surfaces::{DAMAGE_STOP_OFFSET, PASSIVE_GROUND_OFFSET},
};
use crate::runtime::diag_log::diagnostic;
use crate::runtime::{
    game::{self, natives},
    state_natives,
};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[path = "tech_speed_guards.rs"]
mod speed_guards;

const SPEED_GETTER: usize = 0x6d8cc0;
static MAIN_SPEED_STOCK: AtomicBool = AtomicBool::new(false);
static COMMON_SPEED_BASE: AtomicUsize = AtomicUsize::new(0);

unsafe fn stock(base: usize, windows: &[speed_guards::Window]) -> bool {
    windows.iter().all(|&(offset, len, high, low)| {
        let bytes = std::slice::from_raw_parts((base + offset) as *const u8, len);
        let mut digest = Sha256::new();
        digest.update(bytes);
        let actual = digest.finish();
        let verified = actual[..16] == high.to_be_bytes() && actual[16..] == low.to_be_bytes();
        if !verified {
            diagnostic(format_args!("[TechPractice] speed_guard mismatch offset={offset:#x}"));
        }
        verified
    })
}

/// Non-hook initialization. Optional windows can disable only the extra notice.
pub(super) unsafe fn init_passive_speed() {
    let base = game::text_base();
    let verified = base != 0
        && [
            DAMAGE_STOP_OFFSET,
            PASSIVE_GROUND_OFFSET,
            p2::WORK_IS_FLAG_OFFSET,
            p2::WORK_GET_PARAM_FLOAT_OFFSET,
            p2::SITUATION_KIND_OFFSET,
            p2::TRANSITION_TERM_HOOK_OFFSET,
            sites::GET_ENERGY_OFFSET,
            sites::SPEED_X_OFFSET,
            sites::SPEED_Y_OFFSET,
        ].into_iter().all(game::granted)
        && stock(base, speed_guards::MAIN);
    MAIN_SPEED_STOCK.store(verified, Ordering::Release);
}

/// The existing loader verified the first 64 bytes of is_enable_passive.
pub(super) unsafe fn verify_passive_speed_rule(address: usize) {
    let base = address - 0xed2a0;
    let verified = stock(base, speed_guards::COMMON);
    COMMON_SPEED_BASE.store(if verified { base } else { 0 }, Ordering::Release);
}

/// Only the inner floor path necessarily tested passive eligibility before the
/// miss. Both native paths to REFLECT_D share the same final call instruction.
pub(super) unsafe fn floor_speed_route(index: usize, frame: usize, return_pc: usize) -> bool {
    let base = COMMON_SPEED_BASE.load(Ordering::Acquire);
    if base == 0 {
        return false;
    }
    match (index, return_pc.checked_sub(base)) {
        (2, Some(0xfd7d8)) => true, // DOWN in sub_AirChkDown
        (3, Some(0xfb7ec)) if pointer(frame) && frame & 15 == 0 => {
            // Guarded reflector prologue stores the caller LR at [x29 + 8].
            ((frame + 8) as *const usize).read() == base + 0xfd748
        }
        _ => false,
    }
}

/// The same two f32 squares and reciprocal-square-root refinement as
/// main+2277a10. A plain sqrt can differ by ULPs at the inclusive limit.
fn passive_length(x: f32, y: f32) -> f32 {
    let squared = x * x + y * y;
    if squared == 0.0 {
        return 0.0;
    }
    let length;
    unsafe {
        core::arch::asm!(
            "frsqrte s1, s0",
            "fmul s2, s0, s1",
            "frsqrts s2, s2, s1",
            "fmul s1, s1, s2",
            "fmul s2, s0, s1",
            "frsqrts s2, s2, s1",
            "fmul s1, s1, s2",
            "fmul s0, s0, s1",
            inout("v0") squared => length, out("v1") _, out("v2") _,
            options(pure, nomem, nostack, preserves_flags)
        );
    }
    length
}

/// The guarded ordinary DamageFly floor route reads the same energy at the
/// passive check and pre-status seam, before DOWN or REFLECT_D mutates it.
/// The caller has revalidated the live owned fighter on this game-thread pass.
pub(super) unsafe fn passive_floor_speed(accessor: usize, s: DecisionSnapshot) -> Option<f32> {
    if !MAIN_SPEED_STOCK.load(Ordering::Acquire)
        || s.contact.passive_enabled || s.source_status != Some(0x49)
        || !timed_guard_would_pass(s)
    {
        return None;
    }
    // Stock common+ed2e0 early flag and the in-place floor tech prerequisites.
    if natives::is_flag(accessor, 0x2000006c)
        || natives::situation_kind(accessor) != 0
        || !natives::is_enable_transition_term(accessor, 0x1e000013)
        || natives::get_param_float(accessor, hash40("common"), hash40("invalid_passive_speed"))
            != PASSIVE_SPEED_LIMIT
    {
        return None;
    }
    if natives::tech_in_damage_stop(accessor) != Some(false)
        || super::tech_surfaces::passive_floor_touch(accessor) != Some(true)
    {
        return None;
    }
    // Both axis exports dispatch slot +0x20, like stock get_speed_length.
    // The verified DAMAGE implementation selects the alternate sum when active.
    let energy = natives::tech_energy(accessor, 4)?;
    if !pointer(energy) {
        return None;
    }
    let vptr = (energy as *const usize).read();
    if !pointer(vptr) || ((vptr + 0x20) as *const usize).read() != game::text_base() + SPEED_GETTER {
        return None;
    }
    let (x, y) = natives::tech_speed(energy)?;
    let speed = passive_length(x, y);
    speed.is_finite().then_some(speed)
}

fn pointer(value: usize) -> bool {
    value >= 0x1000 && value & 7 == 0 && value >> 47 == 0
}
/// Export thunks at2001260/20013c0/20013d0 load accessor+48 then slots178/228/230.
/// Fighter constructor address point and every concrete reader window are pinned.
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
    if vptr != base + sites::decision::CONTROL_VTABLE {
        return None;
    }
    for (slot, target) in [
        (0x178, sites::STICK_X_OFFSET),
        (0x228, sites::GUARD_COUNT_OFFSET),
        (0x230, sites::GUARD_PREVIOUS_OFFSET),
        (0x220, sites::TRIGGER_WORD_OFFSET),
        (0x250, sites::GUARD_TRIGGER_OFFSET),
    ] {
        if !game::granted(target) || ((vptr + slot) as *const usize).read() != base + target {
            return None;
        }
    }
    pointer(((module + 0x110) as *const usize).read()).then_some(module)
}

/// Native positive-edge mask, not a shoulder sample or a count decrease.
pub(super) unsafe fn guard_event(accessor: usize) -> Option<bool> {
    let module = control(accessor)?;
    let button = tech_native::id("CONTROL_PAD_BUTTON_GUARD").filter(|b| (0..18).contains(b))?;
    natives::tech_guard_trigger(module, button)
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
    let button = tech_native::id("CONTROL_PAD_BUTTON_GUARD").filter(|b| (0..18).contains(b));
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
        guard_event: module.zip(button).and_then(|(m, b)| natives::tech_guard_trigger(m, b)),
        window_base,
        window_multiplier,
        lockout,
        reaction,
        damage_fall,
        stick_x,
        // Native update3f9554/6c stores zero on the press and increments only otherwise.
        passive_speed: None,
        press_frame_offset: guard_count.map(|_| 0),
    }
}
