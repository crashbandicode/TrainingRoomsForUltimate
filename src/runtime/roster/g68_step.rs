//! rc.7 F1, the D-KIRBY1 root fix: the bank owns the G68 step of the per-kind releases it causes
//! (`swap_model::request_bit`, `Policy::Owned`; the mechanism in LANE-OPUS-RC6-PRELOAD-PRIORITY-REPORT 12:55Z).
//!
//! `0x17e8610(R, k)` calls `0x17f0290(G68, R+0xb0, k, sel)` at `0x17e87ac` while `R+0x4c` bit 2 is set. That
//! hands back the Kirby-copy package `(k, costume)` of every Kirby record in the roster. Native pairs the step
//! with the load completion `0x17e8440`, which took those keys for the header members. The bank's keys come
//! from `0x17e64d0`, which takes no G68 key, so the step of a release the bank causes would take another
//! owner's reference: beside a live Kirby that copies `k`, the copy's package (D-KIRBY1's K1 hang).
//!
//! A release the bank causes is armed here, by thread, `R` and kind, right before its `0x17e8610`:
//! `r_handle::execute` for every ledger release, and the construction tail `0x60dec4` for a bank-built
//! body (a background body always; a Phase B body unless a completion took its group's keys under that
//! build's own lease, `request_bit::tail_takes_g68_step`). The hook then passes kind −1 to `0x17f0290`,
//! which returns at once (`0x17f02a4`), and `0x17e8610` still returns R's own keys (`w20` is untouched).
//!
//! Hook rules: atomics only. No lock, no allocation and no log: the hook runs on the game thread and the
//! task worker.

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use skyline::hooks::InlineCtx;

/// The `bl 0x17f0290` inside `0x17e8610`: `x0` G68, `x1` `R+0xb0`, `w2` the kind (`w20`), `w3` the costume
/// selector; `x19` is `R`.
pub(crate) const G68_STEP: usize = 0x17e87ac;
/// `bl 0x17f0290; mov w8, #0x2a; mul w21, w20, w8` (owned 13.0.5 words).
pub(crate) const G68_STEP_WORDS: [u32; 3] = [0x94001eb9, 0x52800548, 0x1b087e95];

/// Per owner (`body+0x10`): `R+0x4c` when its bank build's request-bit lease began (`r_handle::request_bit`), with
/// bit 8 set; 0 while it has none. INSTALL serializes builds, so one lease per owner at a time.
const OWNERS: usize = 8;
static LEASE: [AtomicU32; OWNERS] = [const { AtomicU32::new(0) }; OWNERS];
const LEASE_HELD: u32 = 0x100;

pub(crate) fn lease_began(owner: u32, before: u8) {
    if let Some(slot) = LEASE.get(owner as usize) { slot.store(LEASE_HELD | u32::from(before), Ordering::Release); }
}

pub(crate) fn lease_ended(owner: u32) {
    if let Some(slot) = LEASE.get(owner as usize) { slot.store(0, Ordering::Release); }
}

/// `R+0x4c` when `owner`'s bank build began, while the build runs.
pub(crate) fn lease_before(owner: u32) -> Option<u8> {
    let value = LEASE.get(owner as usize)?.load(Ordering::Acquire);
    (value & LEASE_HELD != 0).then_some(value as u8)
}

/// One slot per thread that may run a release at once (the game thread and the task workers).
const SLOTS: usize = 8;
static THREAD: [AtomicUsize; SLOTS] = [const { AtomicUsize::new(0) }; SLOTS];
static OWNER_R: [AtomicUsize; SLOTS] = [const { AtomicUsize::new(0) }; SLOTS];
static KIND: [AtomicU32; SLOTS] = [const { AtomicU32::new(u32::MAX) }; SLOTS];
static ENABLED: AtomicBool = AtomicBool::new(false);
/// G68 steps the hook took out (reported by the bank's lines).
static SKIPPED: AtomicU32 = AtomicU32::new(0);
/// Arms that found no slot; the release then kept its step (reported, never expected).
static UNARMED: AtomicU32 = AtomicU32::new(0);

fn current_thread() -> usize { unsafe { skyline::nn::os::GetCurrentThread() as usize } }

/// F1 is on: the step hook and the tail hook are installed (`native_body_bank::install`).
pub(crate) fn enabled() -> bool { ENABLED.load(Ordering::Acquire) }

pub(crate) fn enable() { ENABLED.store(true, Ordering::Release); }

/// `(skipped, unarmed)` since boot.
pub(crate) fn counts() -> (u32, u32) { (SKIPPED.load(Ordering::Relaxed), UNARMED.load(Ordering::Relaxed)) }

/// The counts the last [`changed`] call returned (`skipped << 32 | unarmed`).
static REPORTED: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// `(skipped, unarmed)` when they changed since the last call (the caller logs them), else `None`.
pub(crate) fn changed() -> Option<(u32, u32)> {
    let (skipped, unarmed) = counts();
    let now = u64::from(skipped) << 32 | u64::from(unarmed);
    (REPORTED.swap(now, Ordering::AcqRel) != now).then_some((skipped, unarmed))
}

/// Arm the G68 step of this thread's next `0x17e8610(r, kind)`. False when F1 is off or no slot was free.
pub(crate) fn arm(r: usize, kind: u32) -> bool {
    if !enabled() { return false; }
    let thread = current_thread();
    let slot = (0..SLOTS).find(|&i| THREAD[i].load(Ordering::Acquire) == thread).or_else(|| {
        (0..SLOTS).find(|&i| THREAD[i].compare_exchange(0, thread, Ordering::AcqRel, Ordering::Acquire).is_ok())
    });
    match slot {
        Some(i) => {
            OWNER_R[i].store(r, Ordering::Release);
            KIND[i].store(kind, Ordering::Release);
            true
        }
        None => {
            UNARMED.fetch_add(1, Ordering::Relaxed);
            false
        }
    }
}

/// Clear this thread's slot: after the armed call returned, or for a tail that is not the bank's.
pub(crate) fn disarm() {
    let thread = current_thread();
    for i in 0..SLOTS {
        if THREAD[i].load(Ordering::Acquire) == thread {
            OWNER_R[i].store(0, Ordering::Release);
            KIND[i].store(u32::MAX, Ordering::Release);
            THREAD[i].store(0, Ordering::Release);
        }
    }
}

/// The step of an armed release: kind −1 for `0x17f0290`. Any other release keeps native's step.
#[skyline::hook(offset = 0x17e87ac, inline)]
unsafe fn g68_step(ctx: &mut InlineCtx) {
    let r = ctx.registers[19].x() as usize;
    let kind = ctx.registers[20].w();
    let thread = current_thread();
    for i in 0..SLOTS {
        if THREAD[i].load(Ordering::Acquire) == thread && OWNER_R[i].load(Ordering::Acquire) == r
            && KIND[i].load(Ordering::Acquire) == kind {
            ctx.registers[2].set_x(u64::from(u32::MAX));
            SKIPPED.fetch_add(1, Ordering::Relaxed);
            return;
        }
    }
}

pub(crate) fn install_hook() {
    skyline::install_hooks!(g68_step);
}
