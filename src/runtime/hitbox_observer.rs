//! Diagnostic register-only observation of our synchronous req_follow call.
use crate::engine::seams::native_seams::rows_hitbox_observer;
use crate::runtime::diag_log;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use skyline::hooks::{getRegionAddress, InlineCtx, Region};
#[path = "../model/hitbox_observation.rs"]
mod policy;

static BASE: AtomicUsize = AtomicUsize::new(0);
static READY: AtomicBool = AtomicBool::new(false);
static FAILED: AtomicUsize = AtomicUsize::new(0);
static CAPTURE: policy::Capture = policy::Capture::new();
const LIMIT: usize = 8;

pub(crate) struct Request {
    thread: usize,
}
impl Request {
    pub(crate) fn begin() -> Self {
        let thread = unsafe { skyline::nn::os::GetCurrentThread() as usize };
        let active = READY.load(Ordering::Acquire)
            && diag_log::async_writer_ready()
            && FAILED.load(Ordering::Relaxed) < LIMIT
            && CAPTURE.begin(thread);
        Self {
            thread: if active { thread } else { 0 },
        }
    }

    pub(crate) fn finish(mut self, handle: u32) {
        let snapshot = CAPTURE.finish(self.thread);
        self.thread = 0;
        if handle != 0 {
            return;
        }
        let Some(snapshot) = snapshot else {
            return;
        };
        let Ok(ordinal) = FAILED.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
            (n < LIMIT).then_some(n.saturating_add(1))
        }) else {
            return;
        };
        // Existing asynchronous diagnostic queue only; no startup filesystem fallback.
        if diag_log::async_writer_ready() {
            diag_log::diagnostic(format_args!(
                "[HitboxConstructor] failure={} outcome={:?} object={:#x} frame={:#x} helper={:?} returned={:?}",
                ordinal + 1, snapshot.outcome, snapshot.object, snapshot.frame,
                snapshot.helper, snapshot.returned));
        }
    }
}
impl Drop for Request {
    fn drop(&mut self) {
        let _ = CAPTURE.finish(self.thread);
    }
}

fn owner() -> Option<usize> {
    let thread = unsafe { skyline::nn::os::GetCurrentThread() as usize };
    CAPTURE.owns(thread).then_some(thread)
}
fn relative(address: u64) -> usize {
    (address as usize)
        .checked_sub(BASE.load(Ordering::Acquire))
        .unwrap_or(usize::MAX)
}

#[skyline::hook(offset = 0x356cec0, inline)]
unsafe fn constructor(ctx: &mut InlineCtx) {
    let Some(thread) = owner() else {
        return;
    };
    let _scope = crate::model::hook_stats::HookScope::enter();
    crate::runtime::threads::headroom("hitbox_observer_constructor");
    CAPTURE.constructor(
        thread,
        ctx.sp.x() as usize,
        ctx.registers[0].x() as usize,
        relative(ctx.registers[30].x()),
    );
}
#[skyline::hook(offset = 0xa50d8, inline)]
unsafe fn helper_return(ctx: &mut InlineCtx) {
    let Some(thread) = owner() else {
        return;
    };
    let _scope = crate::model::hook_stats::HookScope::enter();
    crate::runtime::threads::headroom("hitbox_observer_helper_return");
    CAPTURE.helper_return(
        thread,
        ctx.sp.x() as usize,
        relative(ctx.registers[30].x()),
        ctx.registers[0].w(),
    );
}
#[skyline::hook(offset = 0x356d970, inline)]
unsafe fn final_phase(ctx: &mut InlineCtx) {
    let Some(thread) = owner() else {
        return;
    };
    let _scope = crate::model::hook_stats::HookScope::enter();
    crate::runtime::threads::headroom("hitbox_observer_final_phase");
    CAPTURE.final_phase(thread, ctx.sp.x() as usize, ctx.registers[19].x() as usize);
}
#[skyline::hook(offset = 0x356cfb8, inline)]
unsafe fn constructor_return(ctx: &mut InlineCtx) {
    let Some(thread) = owner() else {
        return;
    };
    let _scope = crate::model::hook_stats::HookScope::enter();
    crate::runtime::threads::headroom("hitbox_observer_constructor_return");
    CAPTURE.constructor_return(thread, ctx.sp.x() as usize, ctx.registers[0].w());
}

pub(crate) fn init() {
    let base = unsafe { getRegionAddress(Region::Text) as usize };
    let end = unsafe { getRegionAddress(Region::Rodata) as usize };
    let bytes = |offset: usize, len: usize| -> Option<&'static [u8]> {
        let start = base
            .checked_add(offset)
            .filter(|&s| base != 0 && s >= base && s.checked_add(len).is_some_and(|e| e <= end))?;
        Some(unsafe { core::slice::from_raw_parts(start as *const u8, len) })
    };
    let exact = rows_hitbox_observer::ROWS
        .iter()
        .all(|row| bytes(row.offset, 64) == Some(row.guard.as_slice()));
    let ranges = [
        (0x356cec0, 3248, 0x4d169258c892d75bu64),
        (0xa4f60, 596, 0x5cac3e4803f744c9u64),
    ];
    let flow = ranges.iter().all(|&(offset, len, expected)| {
        bytes(offset, len).is_some_and(|data| {
            data.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
                (hash ^ *byte as u64).wrapping_mul(0x100000001b3)
            }) == expected
        })
    });
    let labels: [&str; 6] = core::array::from_fn(|i| rows_hitbox_observer::ROWS[i].label);
    let usable =
        crate::runtime::seams::seam_install::gate("hitbox_observer", &labels, exact && flow);
    if !exact || !flow || !usable {
        diag_log::diagnostic(format_args!(
            "[HitboxConstructor] observer refused windows={exact} flow={flow}"
        ));
        return;
    }
    BASE.store(base, Ordering::Release);
    skyline::install_hooks!(constructor, helper_return, final_phase, constructor_return);
    READY.store(true, Ordering::Release);
    diag_log::diagnostic(format_args!(
        "[HitboxConstructor] observer installed failed_limit={LIMIT}"
    ));
}
