//! M3 Set observer (event interface v1.1; OPUS-DESIGN-REVIEW-sora116-refreeze.md, rulings 20:49Z/22:38Z).
//! FighterControl vtable +0x90 (main+0x4f85e70) → main+0x3f81a0. `arm` runs at Settle on the owner thread and
//! returns only after the physical CAS and readback; every exit closes the gate and restores first.
//! Unqualified calls touch x16/x17 only and branch to the original. Qualified calls (LR == the verified
//! common+0x111f80 word, x0 == the armed P1 Control, w1 == 0) read record 0 before and after ONE original call and
//! publish `{generation, seq, tag, old, operand, new}` to an 8-entry ring. No lock, heap, formatting or I/O on the
//! call path; no stack reads.
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use crate::model::mashout::clatter::{slot_for, ClatterWrite, Poison, Poll, Slot};
use crate::runtime::game;

const TABLE: usize = 0x4f85de0;
const SET_SLOT: usize = 0x4f85e70;
const SET_ORIGINAL: usize = 0x3f81a0;
const COMMON_SET_RETURN: usize = 0x111f80;
/// Add instance (tag 117; egg117-consumer/C1-S0-STATIC.md): +0x98 → main+0x3f81c0, called from common+0xc5b2c..b38
/// through main+0x20010d0 (`ldr x2,[x8,#0x98]; br x2`), so LR == common+0xc5b38.
const ADD_SLOT: usize = 0x4f85e78;
const ADD_ORIGINAL: usize = 0x3f81c0;
const COMMON_ADD_RETURN: usize = 0xc5b38;

/// (vtable slot, original, wrapper) of one instance; Set and Add are exclusive per scenario (C4).
fn layout(slot: Slot) -> (usize, usize, usize) {
    match slot {
        Slot::Set => (SET_SLOT, SET_ORIGINAL, set_wrapper as *const () as usize),
        Slot::Add => (ADD_SLOT, ADD_ORIGINAL, add_wrapper as *const () as usize),
    }
}
/// lua2cpp_common 13.0.5 build id (NRO header +0x40), nro:13f5177a7e86f0150f1a22aa9e611eacd010397c.
const COMMON_BUILD_ID: [u8; 20] = [
    0x13, 0xf5, 0x17, 0x7a, 0x7e, 0x86, 0xf0, 0x15, 0x0f, 0x1a, 0x22, 0xaa, 0x9e, 0x61, 0x1e, 0xac, 0xd0, 0x10,
    0x39, 0x7c,
];
pub const RING: usize = 8;

/// The proven Start slot (interposer.rs): C5 requires the Set slot's page to carry its permission.
const START_SLOT: usize = 0x4f85e58;

/// v1.1 (b): every refusal maps to "Mash out unavailable" (MASH-NATIVE-SLOT).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    NotOwnerThread,
    WrongPhase,
    AlreadyArmed,
    ForeignSlot,
    PageCheck,
    CommonUnverified,
    VtableMismatch,
}

/// O2: the thread that runs the owner site (`interposer::service`); `arm` must run on it.
static OWNER_THREAD: AtomicUsize = AtomicUsize::new(0);

fn current_thread() -> usize {
    // SAFETY: Horizon's current-thread query has no preconditions.
    unsafe { skyline::nn::os::GetCurrentThread() as usize }
}

#[repr(C)]
#[derive(Default)]
struct MemoryInfo { address: u64, size: u64, kind: u32, attributes: u32, permission: u32, ipc: u32, device: u32, padding: u32 }

/// C5: Horizon QueryMemory (pinned ABI reused from mii_ordinary_native::span): the region's permission bits.
unsafe fn permission(address: usize) -> Option<u32> {
    let mut info = MemoryInfo::default();
    let result: u64;
    core::arch::asm!("svc 0x6", inout("x0") &mut info as *mut MemoryInfo as u64 => result,
        lateout("x1") _, in("x2") address as u64, clobber_abi("C"), options(nostack));
    (result as u32 == 0 && info.address <= address as u64
        && info.address.checked_add(info.size).is_some_and(|end| address as u64 + 8 <= end))
        .then_some(info.permission)
}

static ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// C9: common base + 0x111f80 (Set) / + 0xc5b38 (Add) while the verified common module is mapped; 0 otherwise
/// (never equals an LR).
static COMMON_LR: AtomicUsize = AtomicUsize::new(0);
static COMMON_ADD_LR: AtomicUsize = AtomicUsize::new(0);
/// The installed instance's vtable offset (0x90 / 0x98), so the owner site restores exactly what it installed.
static INSTALLED_SLOT: AtomicU32 = AtomicU32::new(0);
static COMMON_BASE: AtomicUsize = AtomicUsize::new(0);
static COMMON_GENERATION: AtomicU32 = AtomicU32::new(0);
static ARMED_COMMON_GENERATION: AtomicU32 = AtomicU32::new(0);
/// The armed P1 Control; 0 when no attempt owns the observer.
static P1_CONTROL: AtomicUsize = AtomicUsize::new(0);
static GENERATION: AtomicU32 = AtomicU32::new(0);
static TAG: AtomicU32 = AtomicU32::new(0);
static ARMED: AtomicBool = AtomicBool::new(false);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static LOST: AtomicBool = AtomicBool::new(false);
static IN_CALL: AtomicBool = AtomicBool::new(false);
/// 0 = none, else Poison as u32 + 1. Latched until the next arm.
static POISON: AtomicU32 = AtomicU32::new(0);
static COUNT: AtomicU32 = AtomicU32::new(0);
/// Owner-site frame counter: the `frame` of each published write (G2 timing only).
static FRAME: AtomicU32 = AtomicU32::new(0);
/// v1.2: the L2CValue number tag; `d_max` lives at [caller x29 − 0xc0] (tag at +0, f32 at +8) at Set entry,
/// written at common+0x111e64 from get_param_float(damage_ice_frame_max) (V2-FINDING.md).
const L2C_NUMBER: u32 = 3;
const D_MAX_SLOT: usize = 0xc0;
/// Per entry: [generation << 32 | seq (published last, Release)], [d_max << 32 | tag], [old << 32 | operand],
/// [frame << 32 | new].
static ENTRIES: [[AtomicU64; 4]; RING] = [const { [const { AtomicU64::new(0) }; 4] }; RING];

fn poison(why: Poison) {
    let _ = POISON.compare_exchange(0, why as u32 + 1, Ordering::AcqRel, Ordering::Acquire);
}

unsafe fn slot_at(offset: usize) -> &'static AtomicUsize {
    &*((game::text_base() + offset) as *const AtomicUsize)
}

/// C9 (a)/(d): called from the common-module load callback with the module's base. A load while armed poisons the
/// attempt; the words are re-published only after zeroing, with a new common generation.
pub fn publish_common(base: usize) {
    COMMON_LR.store(0, Ordering::Release);
    COMMON_ADD_LR.store(0, Ordering::Release);
    if ARMED.load(Ordering::Acquire) {
        poison(Poison::Lost);
    }
    // SAFETY: the loader just mapped this module; the NRO header (and its build id at +0x40) is readable text.
    let id = unsafe { core::ptr::read_unaligned((base + 0x40) as *const [u8; 20]) };
    if id != COMMON_BUILD_ID {
        return;
    }
    COMMON_BASE.store(base, Ordering::Release);
    COMMON_GENERATION.fetch_add(1, Ordering::AcqRel);
    COMMON_LR.store(base + COMMON_SET_RETURN, Ordering::Release);
    COMMON_ADD_LR.store(base + COMMON_ADD_RETURN, Ordering::Release);
}

/// C9 (b): every scope loss zeroes the words FIRST, then closes the gate (restore at the owner site, O4 order).
pub fn retract_common() {
    COMMON_LR.store(0, Ordering::Release);
    COMMON_ADD_LR.store(0, Ordering::Release);
    release();
}

static SCENE_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Owner site, every frame: the falling edge of an active scene (scene change, waiting-room exit, loading) retracts
/// the common word before anything else runs. A common load published during loading survives until the next exit.
pub fn scene(active: bool) {
    if SCENE_ACTIVE.swap(active, Ordering::AcqRel) && !active {
        retract_common();
    }
}

/// Settle on the owner thread (O5): install the tag's one instance (113/116 → Set +0x90, 117 → Add +0x98) over the
/// exact original and read it back before returning the generation. Only the concrete FighterControl table.
pub fn arm(generation: u32, tag: u16, p1_control: usize) -> Result<u32, Refusal> {
    let owner = OWNER_THREAD.load(Ordering::Acquire);
    if owner == 0 || owner != current_thread() {
        return Err(Refusal::NotOwnerThread);
    }
    let Some(kind) = slot_for(tag) else {
        return Err(Refusal::WrongPhase);
    };
    let (slot_offset, original_offset, wrapper) = layout(kind);
    let (word, ret) = match kind {
        Slot::Set => (&COMMON_LR, COMMON_SET_RETURN),
        Slot::Add => (&COMMON_ADD_LR, COMMON_ADD_RETURN),
    };
    if ARMED.load(Ordering::Acquire) {
        return Err(Refusal::AlreadyArmed);
    }
    if LOST.load(Ordering::Acquire) {
        return Err(Refusal::ForeignSlot);
    }
    if p1_control == 0 || unsafe { *(p1_control as *const usize) } != game::text_base() + TABLE {
        return Err(Refusal::VtableMismatch);
    }
    // C9 (c): the module at the published base still carries the verified build id and generation.
    let base = COMMON_BASE.load(Ordering::Acquire);
    let common_generation = COMMON_GENERATION.load(Ordering::Acquire);
    if word.load(Ordering::Acquire) != base + ret
        || base == 0
        || unsafe { core::ptr::read_unaligned((base + 0x40) as *const [u8; 20]) } != COMMON_BUILD_ID
    {
        return Err(Refusal::CommonUnverified);
    }
    let page = unsafe { permission(game::text_base() + slot_offset) };
    if page.is_none() || page != unsafe { permission(game::text_base() + START_SLOT) } {
        return Err(Refusal::PageCheck);
    }
    let original = game::text_base() + original_offset;
    let found = unsafe { slot_at(slot_offset) };
    if found.load(Ordering::Acquire) != original {
        LOST.store(true, Ordering::Release);
        return Err(Refusal::ForeignSlot);
    }
    for entry in &ENTRIES {
        entry[0].store(0, Ordering::Release);
    }
    COUNT.store(0, Ordering::Release);
    POISON.store(0, Ordering::Release);
    GENERATION.store(generation, Ordering::Release);
    TAG.store(u32::from(tag), Ordering::Release);
    ARMED_COMMON_GENERATION.store(common_generation, Ordering::Release);
    ORIGINAL.store(original, Ordering::Release);
    INSTALLED_SLOT.store(kind as u32, Ordering::Release);
    if found.compare_exchange(original, wrapper, Ordering::AcqRel, Ordering::Acquire).is_err()
        || found.load(Ordering::Acquire) != wrapper
    {
        let _ = found.compare_exchange(wrapper, original, Ordering::AcqRel, Ordering::Acquire);
        LOST.store(true, Ordering::Release);
        return Err(Refusal::ForeignSlot);
    }
    INSTALLED.store(true, Ordering::Release);
    ARMED.store(true, Ordering::Release);
    P1_CONTROL.store(p1_control, Ordering::Release);
    Ok(generation)
}

/// Every exit (Result, Retry, Choose, fail, Off, scene change, body/CPU change): the gate closes now and, on the
/// owner thread, the slot is restored in the same call; elsewhere the owner site restores it (O4 backstop).
pub fn release() {
    P1_CONTROL.store(0, Ordering::Release);
    ARMED.store(false, Ordering::Release);
    // L2: the owner site does not run on the main menu, so on the owner thread the slot is restored now, not at the
    // next Training frame. Off the owner thread only the gate closes (O3); service() restores.
    let owner = OWNER_THREAD.load(Ordering::Acquire);
    if owner != 0 && owner == current_thread() {
        restore();
    }
}

/// Owner thread only: put the original back over TR's own wrapper (O8); a foreign value latches LOST.
fn restore() {
    let kind = if INSTALLED_SLOT.load(Ordering::Acquire) == Slot::Add as u32 { Slot::Add } else { Slot::Set };
    let (slot_offset, original_offset, wrapper) = layout(kind);
    let original = game::text_base() + original_offset;
    let found = unsafe { slot_at(slot_offset) };
    if INSTALLED.swap(false, Ordering::AcqRel)
        && (found.compare_exchange(wrapper, original, Ordering::AcqRel, Ordering::Acquire).is_err()
            || found.load(Ordering::Acquire) != original)
    {
        LOST.store(true, Ordering::Release);
    }
}

/// Owner site (main thread, outside hooks): restore TR's own value only (O8); verify an armed install each frame.
pub fn service() {
    OWNER_THREAD.store(current_thread(), Ordering::Release);
    FRAME.fetch_add(1, Ordering::AcqRel);
    let kind = if INSTALLED_SLOT.load(Ordering::Acquire) == Slot::Add as u32 { Slot::Add } else { Slot::Set };
    let (slot_offset, _, wrapper) = layout(kind);
    let word = if kind == Slot::Add { &COMMON_ADD_LR } else { &COMMON_LR };
    let found = unsafe { slot_at(slot_offset) };
    if ARMED.load(Ordering::Acquire) {
        if found.load(Ordering::Acquire) != wrapper
            || COMMON_GENERATION.load(Ordering::Acquire) != ARMED_COMMON_GENERATION.load(Ordering::Acquire)
            || word.load(Ordering::Acquire) == 0
        {
            poison(Poison::Lost);
            release();
        } else {
            return;
        }
    }
    restore();
}

// The Add instance (117): the same x16/x17-only unqualified path against its own C9 word; no d_max (Add's operand is
// the native delta, v1.2 publishes 0).
#[unsafe(naked)]
unsafe extern "C" fn add_wrapper(_control: usize, _index: i32, _operand: f32) {
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
        lr = sym COMMON_ADD_LR,
        p1 = sym P1_CONTROL,
        original = sym ORIGINAL,
        qualified = sym qualified_add,
    )
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
        // v1.2: the caller's frame pointer (PLT and the lua_bind wrapper are frame-less) for the d_max read.
        "mov x2, x29",
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

/// v1.2 `d_max` bits from the qualified caller's live frame; anything but an L2C number publishes NaN, which the
/// consumer refuses as AuxInvalid.
unsafe fn d_max(fp: usize) -> u32 {
    let slot = fp - D_MAX_SLOT;
    if (slot as *const u32).read_volatile() == L2C_NUMBER {
        ((slot + 8) as *const u32).read_volatile()
    } else {
        f32::NAN.to_bits()
    }
}

unsafe extern "C" fn qualified(control: usize, index: i32, fp: usize, operand: f32) {
    observe(control, index, operand, Some(fp));
}

unsafe extern "C" fn qualified_add(control: usize, index: i32, operand: f32) {
    observe(control, index, operand, None);
}

/// One qualified call: record 0 before and after ONE original call, published to the ring (d_max only for Set).
unsafe fn observe(control: usize, index: i32, operand: f32, fp: Option<usize>) {
    let original: unsafe extern "C" fn(usize, i32, f32) = core::mem::transmute(ORIGINAL.load(Ordering::Acquire));
    if IN_CALL.swap(true, Ordering::AcqRel) {
        poison(Poison::Nested);
        original(control, index, operand);
        return;
    }
    let cap = fp.map_or(0, |fp| d_max(fp));
    let old = record0(control).read_volatile();
    original(control, index, operand);
    let new = record0(control).read_volatile();
    let seq = COUNT.fetch_add(1, Ordering::AcqRel) + 1;
    if seq as usize > RING {
        poison(Poison::Overflow);
    } else {
        let entry = &ENTRIES[seq as usize - 1];
        entry[1].store(u64::from(TAG.load(Ordering::Acquire)) | (u64::from(cap) << 32), Ordering::Relaxed);
        entry[2].store((u64::from(old.to_bits()) << 32) | u64::from(operand.to_bits()), Ordering::Relaxed);
        entry[3].store(u64::from(new.to_bits()) | (u64::from(FRAME.load(Ordering::Acquire)) << 32), Ordering::Relaxed);
        entry[0].store((u64::from(GENERATION.load(Ordering::Acquire)) << 32) | u64::from(seq), Ordering::Release);
    }
    IN_CALL.store(false, Ordering::Release);
}

/// Consumer (owner thread): the published event `seq` (1-based) of `generation`, a poison, or nothing yet.
pub fn poll(generation: u32, seq: u32) -> Poll {
    match POISON.load(Ordering::Acquire) {
        0 => {}
        p => return Poll::Poisoned(poison_from(p - 1)),
    }
    if generation != GENERATION.load(Ordering::Acquire) {
        // v1.1 Stale: polling another generation is refused without poisoning the current attempt.
        return Poll::Empty;
    }
    if seq == 0 || seq as usize > RING {
        return if COUNT.load(Ordering::Acquire) >= seq { Poll::Poisoned(Poison::Overflow) } else { Poll::Empty };
    }
    let entry = &ENTRIES[seq as usize - 1];
    let head = entry[0].load(Ordering::Acquire);
    if head as u32 != seq || (head >> 32) as u32 != generation {
        return Poll::Empty;
    }
    let (meta, pair, last) = (entry[1].load(Ordering::Relaxed), entry[2].load(Ordering::Relaxed), entry[3].load(Ordering::Relaxed));
    Poll::Event(ClatterWrite {
        generation,
        seq,
        slot: if INSTALLED_SLOT.load(Ordering::Acquire) == Slot::Add as u32 { Slot::Add } else { Slot::Set },
        tag: meta as u16,
        old: (pair >> 32) as u32,
        operand: pair as u32,
        new: last as u32,
        frame: (last >> 32) as u32,
        d_max: (meta >> 32) as u32,
    })
}

fn poison_from(code: u32) -> Poison {
    match code {
        0 => Poison::Overflow,
        1 => Poison::NewMismatch,
        2 => Poison::Nested,
        3 => Poison::WrongSlot,
        4 => Poison::Stale,
        6 => Poison::AuxInvalid,
        _ => Poison::Lost,
    }
}
