//! M3 Set observer (frozen v1 interface; OPUS-DESIGN-REVIEW-sora116-refreeze.md). FighterControl vtable +0x90
//! (main+0x4f85e70) → main+0x3f81a0, installed only from Settle to Entering of a 113/116 attempt by the existing
//! owner site (`interposer::service`). Unqualified calls touch x16/x17 only and branch to the original. Qualified
//! calls (LR == the verified common+0x111f80 word, x0 == the armed P1 Control, w1 == 0) read record 0 before and
//! after ONE original call and publish `{generation, seq, tag, old, operand, new}` to an 8-entry ring.
//! No lock, heap, formatting or I/O on the call path; no stack reads.
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use crate::model::mashout::clatter_write::{ClatterWrite, Poison, Slot};
use crate::runtime::game;

const TABLE: usize = 0x4f85de0;
const SET_SLOT: usize = 0x4f85e70;
const SET_ORIGINAL: usize = 0x3f81a0;
const COMMON_SET_RETURN: usize = 0x111f80;
pub const RING: usize = 8;

static ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// C9: common base + 0x111f80 while the verified common module is mapped; 0 otherwise (never equals an LR).
static COMMON_LR: AtomicUsize = AtomicUsize::new(0);
/// The armed P1 Control; 0 when no attempt owns the observer.
static P1_CONTROL: AtomicUsize = AtomicUsize::new(0);
static GENERATION: AtomicU32 = AtomicU32::new(0);
static TAG: AtomicU32 = AtomicU32::new(0);
static WANT: AtomicBool = AtomicBool::new(false);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static LOST: AtomicBool = AtomicBool::new(false);
static IN_CALL: AtomicBool = AtomicBool::new(false);
static POISON: AtomicU32 = AtomicU32::new(0);
static COUNT: AtomicU32 = AtomicU32::new(0);
/// Per entry: [generation << 32 | seq (published last, Release)], [tag], [old << 32 | operand], [new].
static ENTRIES: [[AtomicU64; 4]; RING] = [const { [const { AtomicU64::new(0) }; 4] }; RING];

unsafe fn slot() -> &'static AtomicUsize {
    &*((game::text_base() + SET_SLOT) as *const AtomicUsize)
}

fn wrapper() -> usize {
    set_wrapper as *const () as usize
}

/// C9: called from the common-module load callback with the module's base.
pub fn publish_common(base: usize) {
    COMMON_LR.store(base + COMMON_SET_RETURN, Ordering::Release);
}

/// C9: called synchronously on every scene change before any re-publication.
pub fn retract_common() {
    COMMON_LR.store(0, Ordering::Release);
}

/// Settle → Entering of a 113/116 attempt: request the install for this generation (the owner site performs it).
pub fn request(generation: u32, tag: u16, p1_control: usize) {
    if crate::model::mashout::clatter_write::slot_for(tag) != Some(Slot::Set) {
        return;
    }
    COUNT.store(0, Ordering::Release);
    POISON.store(0, Ordering::Release);
    GENERATION.store(generation, Ordering::Release);
    TAG.store(u32::from(tag), Ordering::Release);
    P1_CONTROL.store(p1_control, Ordering::Release);
    WANT.store(true, Ordering::Release);
}

/// Every exit (Result, Retry, Choose, fail, Off, scene change, body/CPU change): the gate closes now; the owner
/// site restores the slot on its next call (O4 two-phase rule).
pub fn release() {
    WANT.store(false, Ordering::Release);
    P1_CONTROL.store(0, Ordering::Release);
}

pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire) && !LOST.load(Ordering::Acquire)
}

/// Owner site (main thread, outside hooks): install over the exact original or restore TR's own value only (O8).
pub fn service(scene_active: bool) {
    let original = game::text_base() + SET_ORIGINAL;
    let found = unsafe { slot() };
    let wanted = WANT.load(Ordering::Acquire) && scene_active && COMMON_LR.load(Ordering::Acquire) != 0;
    if !wanted || LOST.load(Ordering::Acquire) {
        if INSTALLED.swap(false, Ordering::AcqRel)
            && (found.compare_exchange(wrapper(), original, Ordering::AcqRel, Ordering::Acquire).is_err()
                || found.load(Ordering::Acquire) != original)
        {
            LOST.store(true, Ordering::Release);
        }
        return;
    }
    if INSTALLED.load(Ordering::Acquire) {
        if found.load(Ordering::Acquire) != wrapper() {
            LOST.store(true, Ordering::Release);
            POISON.compare_exchange(0, Poison::Lost as u32 + 1, Ordering::AcqRel, Ordering::Acquire).ok();
        }
        return;
    }
    ORIGINAL.store(original, Ordering::Release);
    if found.compare_exchange(original, wrapper(), Ordering::AcqRel, Ordering::Acquire).is_err()
        || found.load(Ordering::Acquire) != wrapper()
    {
        let _ = found.compare_exchange(wrapper(), original, Ordering::AcqRel, Ordering::Acquire);
        LOST.store(true, Ordering::Release);
        return;
    }
    INSTALLED.store(true, Ordering::Release);
}

/// Table identity used by guards and readers: the concrete FighterControl table.
pub fn table() -> usize {
    game::text_base() + TABLE
}

// Unqualified: x16/x17 only, then the original through an acquire load. Qualified: tail-branch to `qualified`
// with x0/w1/s0 and LR untouched, so the original's caller sees an ordinary call.
#[unsafe(naked)]
unsafe extern "C" fn set_wrapper(_control: usize, _index: i32, _operand: f32) {
    core::arch::naked_asm!(
        "adrp x16, {lr}",
        "add x16, x16, :lo12:{lr}",
        "ldar x16, [x16]",
        "cmp x16, x30",
        "b.ne 1f",
        "adrp x16, {p1}",
        "add x16, x16, :lo12:{p1}",
        "ldar x16, [x16]",
        "cmp x16, x0",
        "b.ne 1f",
        "cbnz w1, 1f",
        "b {qualified}",
        "1:",
        "adrp x16, {original}",
        "add x16, x16, :lo12:{original}",
        "ldar x16, [x16]",
        "br x16",
        lr = sym COMMON_LR,
        p1 = sym P1_CONTROL,
        original = sym ORIGINAL,
        qualified = sym qualified,
    )
}

unsafe fn record0(control: usize) -> *const f32 {
    *((control + 0x18) as *const *const f32)
}

unsafe extern "C" fn qualified(control: usize, index: i32, operand: f32) {
    let original: unsafe extern "C" fn(usize, i32, f32) =
        core::mem::transmute(ORIGINAL.load(Ordering::Acquire));
    if IN_CALL.swap(true, Ordering::AcqRel) {
        POISON.compare_exchange(0, Poison::Nested as u32 + 1, Ordering::AcqRel, Ordering::Acquire).ok();
        original(control, index, operand);
        return;
    }
    let old = record0(control).read_volatile();
    original(control, index, operand);
    let new = record0(control).read_volatile();
    let seq = COUNT.fetch_add(1, Ordering::AcqRel) + 1;
    if seq as usize > RING {
        POISON.compare_exchange(0, Poison::Overflow as u32 + 1, Ordering::AcqRel, Ordering::Acquire).ok();
    } else {
        let entry = &ENTRIES[seq as usize - 1];
        entry[1].store(u64::from(TAG.load(Ordering::Acquire)), Ordering::Relaxed);
        entry[2].store((u64::from(old.to_bits()) << 32) | u64::from(operand.to_bits()), Ordering::Relaxed);
        entry[3].store(u64::from(new.to_bits()), Ordering::Relaxed);
        entry[0].store((u64::from(GENERATION.load(Ordering::Acquire)) << 32) | u64::from(seq), Ordering::Release);
    }
    IN_CALL.store(false, Ordering::Release);
}

/// Consumer (main thread): the published event `seq` (1-based) if complete, a poison, or nothing yet.
pub fn poll(seq: u32) -> Result<Option<ClatterWrite>, Poison> {
    match POISON.load(Ordering::Acquire) {
        0 => {}
        p => return Err(poison_from(p - 1)),
    }
    if seq == 0 || seq as usize > RING {
        return Err(Poison::Overflow);
    }
    let entry = &ENTRIES[seq as usize - 1];
    let head = entry[0].load(Ordering::Acquire);
    if head as u32 != seq {
        return Ok(None);
    }
    let pair = entry[2].load(Ordering::Relaxed);
    Ok(Some(ClatterWrite {
        generation: (head >> 32) as u32,
        seq,
        slot: Slot::Set,
        tag: entry[1].load(Ordering::Relaxed) as u16,
        old: (pair >> 32) as u32,
        operand: pair as u32,
        new: entry[3].load(Ordering::Relaxed) as u32,
        frame: 0,
    }))
}

fn poison_from(code: u32) -> Poison {
    match code {
        0 => Poison::Overflow,
        1 => Poison::NewMismatch,
        2 => Poison::Nested,
        3 => Poison::WrongSlot,
        4 => Poison::Stale,
        5 => Poison::Lost,
        _ => Poison::SequenceGap,
    }
}

/// A new attempt reuses slots; entries of an older generation are stale to its Stream and refused there.
pub fn reset_ring() {
    for entry in &ENTRIES {
        entry[0].store(0, Ordering::Release);
    }
    COUNT.store(0, Ordering::Release);
}
