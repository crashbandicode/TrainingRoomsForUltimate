//! Q15 diagnostic-only native constructor observation; never constructs or borrows a texture.
//! The inline entry preserves every native register and resumes the original prologue.

use core::sync::atomic::{AtomicUsize, Ordering};
use skyline::hooks::{getRegionAddress, InlineCtx, Region};
use crate::engine::seams::native_seams::rows_q15;
use crate::runtime::diag_log;

#[path = "../../model/mii/bust_context.rs"]
mod policy;

static BASE: AtomicUsize = AtomicUsize::new(0);
static SEEN: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];

#[skyline::hook(offset = 0x377c0d0, inline)]
unsafe fn constructor_context(ctx: &mut InlineCtx) {
    let base = BASE.load(Ordering::Acquire);
    let Some(caller) = policy::caller(base, ctx.registers[30].x() as usize) else { return; };
    if !diag_log::async_writer_ready() { return; }
    // Saturation avoids wraparound re-admission even in an arbitrarily long diagnostic session.
    let Ok(previous) = SEEN[caller].fetch_update(Ordering::Relaxed, Ordering::Relaxed,
        |n| policy::within_limit(n).then_some(n.saturating_add(1))) else { return; };
    let _scope = crate::model::hook_stats::HookScope::enter();
    crate::runtime::threads::headroom("q15_constructor_context");
    let thread = skyline::nn::os::GetCurrentThread() as usize;
    // r19/r27 are opaque caller tokens, NOT dereferenced or advertised as validated scenes.
    // Record both so static caller dataflow can be checked against the observed native route.
    diag_log::diagnostic(format_args!(
        "[Q15Context] caller={:#x} ordinal={} thread={thread:#x} sp={:#x} r19={:#x} r27={:#x} pair={:#x} bytes={:#x} len={} copy={}",
        policy::RETURNS[caller] - 4, previous + 1, ctx.sp.x(), ctx.registers[19].x(),
        ctx.registers[27].x(), ctx.registers[0].x(), ctx.registers[1].x(),
        ctx.registers[2].w(), ctx.registers[3].w()));
}

pub(crate) fn init() {
    let base = unsafe { getRegionAddress(Region::Text) as usize };
    let end = unsafe { getRegionAddress(Region::Rodata) as usize };
    let mut exact = [false; 3];
    for (i, row) in rows_q15::ROWS.iter().enumerate() {
        if let Some(start) = base.checked_add(row.offset)
            .filter(|&start| start >= base && start.checked_add(64).is_some_and(|last| last <= end)) {
            // Boot only, before installing any observer. Exact full windows, not just BL words.
            exact[i] = unsafe { core::slice::from_raw_parts(start as *const u8, 64) } == row.guard;
        }
    }
    let legacy = policy::admitted(exact) && rows_q15::ROWS[0].offset == policy::CONSTRUCTOR;
    let labels = [rows_q15::ROWS[0].label, rows_q15::ROWS[1].label, rows_q15::ROWS[2].label];
    let usable = crate::runtime::seams::seam_install::gate("q15_bust_context", &labels, legacy);
    if !legacy || !usable {
        diag_log::diagnostic(format_args!("[Q15Context] observer refused guards={exact:?}"));
        return;
    }
    BASE.store(base, Ordering::Release);
    skyline::install_hooks!(constructor_context);
    diag_log::diagnostic(format_args!("[Q15Context] observer installed limit_per_caller={}", policy::LIMIT_PER_CALLER));
}
