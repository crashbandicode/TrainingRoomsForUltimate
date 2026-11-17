//! TrainingRooms state export v1: the table a test-only reader (ssbu-hw-bridge) subscribes through, and the line
//! ring the reader owns (TR keeps no buffer). The same layout is in the bridge's `src/abi.rs`; both pin it in a host test.
//! Every slot is owned by one side at a time through its state word (`WRITING` by the producer that claimed it with a
//! compare-and-swap, `READING` by the reader that claimed it the same way). A producer that finds its slot owned (a
//! writer one lap away, or the reader copying it) drops its line and counts it: a hook never waits on the reader's thread.
//! A ready line the reader has not taken yet may be overwritten by a later lap; the reader counts it as lost.

use core::sync::atomic::{AtomicU64, Ordering};

pub const ABI: u32 = 1;
/// Bytes of one exported line (the diag ring's stack line).
pub const SLOT_BYTES: usize = 512;
/// `request` kinds and answers.
pub const REQ_SET_APPLY: u32 = 1;
pub const REQ_SAVE: u32 = 2;
pub const REQ_LOAD: u32 = 3;
pub const REQ_ITEM: u32 = 4;
pub const OK: i32 = 0;
pub const REFUSED_SCENE: i32 = -1;
pub const BAD_REQUEST: i32 = -2;
/// A settings file that does not read cleanly: nothing was applied.
pub const REFUSED_SETTINGS: i32 = -3;

/// Slot phases (the low two bits of `Slot::state`; the rest is the line's ticket + 1, 0 for a slot never written).
pub const FREE: u64 = 0;
pub const WRITING: u64 = 1;
pub const READY: u64 = 2;
pub const READING: u64 = 3;

pub const fn state(ticket: u64, phase: u64) -> u64 {
    ((ticket + 1) << 2) | phase
}

#[repr(C)]
pub struct Slot {
    pub state: AtomicU64,
    pub frame: u64,
    pub uptime_ms: u64,
    pub len: u32,
    pub truncated: u32,
    pub bytes: [u8; SLOT_BYTES],
}

#[repr(C)]
pub struct Ring {
    pub abi: u32,
    pub slot_count: u32,
    pub head: AtomicU64,
    /// The reader's game-frame counter; each line carries the value it had when the line was pushed.
    pub frame: AtomicU64,
    /// Lines a producer dropped because its slot was owned.
    pub dropped: AtomicU64,
    pub slots: *mut Slot,
}

/// One fighter frame, passed to the reader's callback from TR's per-fighter frame (Training and waiting-room practice).
#[repr(C)]
pub struct FighterFrame {
    pub entry: i32,
    pub kind: i32,
    pub status: i32,
    pub situation: i32,
    /// `FRAME_*` bits.
    pub flags: u32,
    pub reserved: u32,
    pub accessor: usize,
    pub fighter: usize,
}

/// The body is Nana (the second body of an Ice Climbers entry).
pub const FRAME_NANA: u32 = 1;
/// The entry is Player 1's role; otherwise a CPU's.
pub const FRAME_PLAYER: u32 = 2;

pub type OnFrame = extern "C" fn(*const FighterFrame);

// SAFETY: the table is immutable; `version` names a 'static string.
unsafe impl Sync for Table {}
// SAFETY: a ring is shared by design: atomics order every access to its slots.
unsafe impl Sync for Ring {}
unsafe impl Send for Ring {}

#[repr(C)]
pub struct Table {
    pub abi: u32,
    pub size: u32,
    pub version: *const u8,
    pub version_len: usize,
    /// The ring must live as long as the process; `on_frame` may be null.
    pub subscribe: extern "C" fn(*mut Ring, Option<OnFrame>) -> i32,
    pub request: extern "C" fn(u32, *const u8, usize) -> i32,
}

/// The families TR offers (`[TrainingRooms] state pos` is the reader's own native line).
pub const FAMILIES: [&str; 10] = [
    "[TrainingRooms] cpu_event ",
    "[TrainingRooms] notice_draw ",
    "[TrainingRooms] state ",
    "[TrainingRooms] save_state ",
    "[TrainingRooms] settings.json ",
    "[TrainingRooms] diag_settings_reload ",
    "[TrainingRooms] diag_item_request ",
    "[TrainingRooms] state_export ",
    "[Ui] ",
    "[TrainingRooms] cpu_ids ",
];

pub fn exported(line: &[u8]) -> bool {
    FAMILIES.iter().any(|family| line.starts_with(family.as_bytes())) && !line.starts_with(b"[TrainingRooms] state pos ")
}

/// A notice is drawn every frame it shows; it is exported once per change of its text.
pub fn notice_changed(line: &[u8], last: &AtomicU64) -> bool {
    if !line.starts_with(FAMILIES[1].as_bytes()) {
        return true;
    }
    let hash = line.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3));
    last.swap(hash, Ordering::Relaxed) != hash
}

pub enum Pop {
    Empty,
    Lost(u64),
    Line { len: usize, frame: u64, uptime_ms: u64, truncated: bool },
}

impl Ring {
    /// The slot of `ticket`.
    ///
    /// # Safety
    /// `slots` points at `slot_count` slots that live as long as the process.
    unsafe fn slot(&self, ticket: u64) -> *mut Slot {
        self.slots.add((ticket % u64::from(self.slot_count)) as usize)
    }

    /// Producer side: wait-free; the line is dropped (and counted) when its slot is owned.
    ///
    /// # Safety
    /// As `slot`.
    pub unsafe fn push(&self, uptime_ms: u64, text: &[u8], truncated: bool) -> bool {
        if self.slot_count == 0 || self.slots.is_null() {
            return false;
        }
        let ticket = self.head.fetch_add(1, Ordering::AcqRel);
        let slot = self.slot(ticket);
        let seen = (*slot).state.load(Ordering::Acquire);
        let older = (seen >> 2) <= ticket;
        let claimable = older && matches!(seen & 3, FREE | READY);
        if !claimable
            || (*slot).state.compare_exchange(seen, state(ticket, WRITING), Ordering::Acquire, Ordering::Relaxed).is_err()
        {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return false;
        }
        // Owned: no other producer and no reader touches these fields until the release below (`state` stays shared).
        let len = text.len().min(SLOT_BYTES);
        (*slot).bytes[..len].copy_from_slice(&text[..len]);
        (*slot).frame = self.frame.load(Ordering::Relaxed);
        (*slot).uptime_ms = uptime_ms;
        (*slot).len = len as u32;
        (*slot).truncated = u32::from(truncated || len < text.len());
        (*slot).state.store(state(ticket, READY), Ordering::Release);
        true
    }

    /// Reader side: the line of ticket `*next` into `out`, or why not. A ticket whose producer dropped its line stays
    /// `Empty`; the reader skips it after its own deadline (`skip`).
    ///
    /// # Safety
    /// As `slot`.
    pub unsafe fn pop(&self, next: &mut u64, out: &mut [u8; SLOT_BYTES]) -> Pop {
        let count = u64::from(self.slot_count);
        let head = self.head.load(Ordering::Acquire);
        if count == 0 || *next >= head {
            return Pop::Empty;
        }
        if head - *next > count {
            let lost = head - count - *next;
            *next = head - count;
            return Pop::Lost(lost);
        }
        let slot = self.slot(*next);
        let seen = (*slot).state.load(Ordering::Acquire);
        let ticket = (seen >> 2).wrapping_sub(1);
        if seen >> 2 > *next + 1 {
            *next += 1;
            return Pop::Lost(1);
        }
        if ticket != *next || seen & 3 != READY
            || (*slot).state.compare_exchange(seen, state(*next, READING), Ordering::Acquire, Ordering::Relaxed).is_err()
        {
            return Pop::Empty;
        }
        // Owned by the reader: producers drop rather than write a READING slot.
        let len = ((*slot).len as usize).min(SLOT_BYTES);
        out[..len].copy_from_slice(&(*slot).bytes[..len]);
        let line = Pop::Line { len, frame: (*slot).frame, uptime_ms: (*slot).uptime_ms, truncated: (*slot).truncated != 0 };
        (*slot).state.store(state(*next, FREE), Ordering::Release);
        *next += 1;
        line
    }

    /// The reader gives up on ticket `*next` (its producer dropped the line).
    pub fn skip(&self, next: &mut u64) {
        *next += 1;
    }
}

#[cfg(test)]
#[path = "state_export_tests.rs"]
mod tests;
