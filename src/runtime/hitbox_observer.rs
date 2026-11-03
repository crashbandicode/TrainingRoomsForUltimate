//! Bounded diagnostic observation of our synchronous req_follow call.
use crate::engine::seams::native_seams::rows_hitbox_observer;
use crate::runtime::diag_log;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use skyline::hooks::{getRegionAddress, InlineCtx, Region};
#[path = "../model/hitbox_observation.rs"]
mod policy;

static BASE: AtomicUsize = AtomicUsize::new(0);
static READY: AtomicBool = AtomicBool::new(false);
static FAILED: policy::FailureBudget = policy::FailureBudget::new();
static POOL: policy::PoolBudget = policy::PoolBudget::new();
static CAPTURE: policy::Capture = policy::Capture::new();
const LIMIT: usize = policy::FAILED_LIMIT;

pub(crate) struct Request {
    thread: usize,
}
impl Request {
    pub(crate) fn begin() -> Self {
        let thread = unsafe { skyline::nn::os::GetCurrentThread() as usize };
        let active = READY.load(Ordering::Acquire)
            && diag_log::async_writer_ready()
            && (FAILED.available() || POOL.available())
            && CAPTURE.begin(thread);
        if active {
            if let Some(ordinal) = POOL.reserve() {
                CAPTURE.observe_pool(thread, ordinal);
            }
        }
        Self {
            thread: if active { thread } else { 0 },
        }
    }

    pub(crate) fn finish(mut self, handle: u32) {
        let snapshot = CAPTURE.finish(self.thread);
        self.thread = 0;
        if let Some(snapshot) = &snapshot {
            log_pool(snapshot, Some(handle));
        }
        if handle != 0 {
            return;
        }
        let Some(snapshot) = snapshot else {
            return;
        };
        let Some(ordinal) = FAILED.record(handle) else {
            return;
        };
        // Existing asynchronous diagnostic queue only; no startup filesystem fallback.
        if diag_log::async_writer_ready() {
            diag_log::diagnostic(format_args!(
                "[HitboxConstructor] failure={} outcome={:?} object={:#x} frame={:#x} helper={:?} returned={:?}",
                ordinal, snapshot.outcome, snapshot.object, snapshot.frame,
                snapshot.helper, snapshot.returned));
            diag_log::diagnostic(format_args!(
                "[HitboxHelper] failure={} reason={:?} demand={:?} registers={:x?}",
                ordinal, snapshot.helper_reason, snapshot.demand, snapshot.helper_registers
            ));
        }
    }
}
impl Drop for Request {
    fn drop(&mut self) {
        if let Some(snapshot) = CAPTURE.finish(self.thread) {
            log_pool(&snapshot, None);
        }
    }
}

// Capture::finish has released ownership before this asynchronous formatter.
fn log_pool(snapshot: &policy::Snapshot, handle: Option<u32>) {
    if let Some(pool) = &snapshot.pool {
        if diag_log::async_writer_ready() {
            diag_log::diagnostic(format_args!(
                "[HitboxPool] n={} stage=AfterHelper status={} mgr={:#x} total={} free={} handle={:?} outcome={:?}",
                pool.ordinal, pool.status as usize, pool.manager, pool.total,
                pool.available, handle, snapshot.outcome
            ));
        }
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
    // Permit is checked before the injected reader can run. x19 is a manager
    // only on this exact caller's normal paths; early resource failures are denied.
    CAPTURE.pool_after(
        thread,
        ctx.sp.x() as usize,
        relative(ctx.registers[30].x()),
        ctx.registers[0].w(),
        ctx.registers[19].x() as usize,
        |address| unsafe { core::ptr::read_volatile(address as *const u32) },
    );
    CAPTURE.helper_return(
        thread,
        ctx.sp.x() as usize,
        relative(ctx.registers[30].x()),
        ctx.registers[0].w(),
    );
    CAPTURE.helper_registers(
        thread,
        ctx.sp.x() as usize,
        relative(ctx.registers[30].x()),
        [
            ctx.registers[8].x() as usize,
            ctx.registers[9].x() as usize,
            ctx.registers[19].x() as usize,
        ],
    );
}
#[skyline::hook(offset = 0xa5020, inline)]
unsafe fn helper_demand(ctx: &mut InlineCtx) {
    let Some(thread) = owner() else {
        return;
    };
    let _scope = crate::model::hook_stats::HookScope::enter();
    crate::runtime::threads::headroom("hitbox_observer_helper_demand");
    CAPTURE.demand_rejected(
        thread,
        policy::Demand {
            requested: ctx.registers[8].w() as i32,
            available: ctx.registers[9].w() as i32,
            output: ctx.registers[1].x() as usize,
            manager: ctx.registers[19].x() as usize,
            sp: ctx.sp.x() as usize,
            caller: relative(ctx.registers[30].x()),
        },
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
        (0x89850, 116, 0xa91708f8481d3b39u64),
        (0x356cec0, 3248, 0x4d169258c892d75bu64),
        (0xa4f60, 596, 0x5cac3e4803f744c9u64),
        (0x89080, 668, 0x358149e0d6c282ecu64),
        (0x93660, 44, 0x803692d6bb48c168u64),
    ];
    let flow = ranges.iter().all(|&(offset, len, expected)| {
        bytes(offset, len).is_some_and(|data| {
            data.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
                (hash ^ *byte as u64).wrapping_mul(0x100000001b3)
            }) == expected
        })
    });
    let labels: [&str; 9] = core::array::from_fn(|i| rows_hitbox_observer::ROWS[i].label);
    let usable =
        crate::runtime::seams::seam_install::gate("hitbox_observer", &labels, exact && flow);
    if !exact || !flow || !usable {
        diag_log::diagnostic(format_args!(
            "[HitboxConstructor] observer refused windows={exact} flow={flow}"
        ));
        return;
    }
    BASE.store(base, Ordering::Release);
    skyline::install_hooks!(
        constructor,
        helper_return,
        helper_demand,
        final_phase,
        constructor_return
    );
    READY.store(true, Ordering::Release);
    diag_log::diagnostic(format_args!(
        "[HitboxConstructor] observer installed failed_limit={LIMIT} pool_limit=80"
    ));
}
