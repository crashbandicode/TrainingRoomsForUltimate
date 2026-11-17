//! TrainingRooms state export v1: the table a test-only reader (ssbu-hw-bridge) subscribes through, and the line
//! ring the reader owns (TR keeps no buffer). The same layout is in the bridge's `src/abi.rs`; both pin it in a host test.
//! Producers take a ticket with one `fetch_add` and publish the slot with its sequence word, so a hook never waits on the
//! reader's thread; the reader validates the sequence and counts a lapped slot as lost.

use core::sync::atomic::{fence, AtomicU64, Ordering};

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

#[repr(C)]
pub struct Slot {
    pub seq: AtomicU64,
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
    /// Producer side.
    ///
    /// # Safety
    /// `slots` points at `slot_count` slots that live as long as the process.
    pub unsafe fn push(&self, uptime_ms: u64, text: &[u8], truncated: bool) {
        let count = u64::from(self.slot_count);
        if count == 0 || self.slots.is_null() {
            return;
        }
        let ticket = self.head.fetch_add(1, Ordering::AcqRel);
        let slot = self.slots.add((ticket % count) as usize);
        (*slot).seq.store(2 * ticket + 1, Ordering::Release);
        fence(Ordering::Release);
        let len = text.len().min(SLOT_BYTES);
        core::ptr::copy_nonoverlapping(text.as_ptr(), core::ptr::addr_of_mut!((*slot).bytes).cast::<u8>(), len);
        core::ptr::write_volatile(core::ptr::addr_of_mut!((*slot).frame), self.frame.load(Ordering::Relaxed));
        core::ptr::write_volatile(core::ptr::addr_of_mut!((*slot).uptime_ms), uptime_ms);
        core::ptr::write_volatile(core::ptr::addr_of_mut!((*slot).len), len as u32);
        core::ptr::write_volatile(core::ptr::addr_of_mut!((*slot).truncated), u32::from(truncated || len < text.len()));
        (*slot).seq.store(2 * ticket + 2, Ordering::Release);
    }

    /// Reader side: the line of ticket `*next` into `out`, or why not.
    ///
    /// # Safety
    /// As `push`.
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
        let slot = self.slots.add((*next % count) as usize);
        let want = 2 * *next + 2;
        let seq = (*slot).seq.load(Ordering::Acquire);
        if seq < want {
            return Pop::Empty;
        }
        let len = (core::ptr::read_volatile(core::ptr::addr_of!((*slot).len)) as usize).min(SLOT_BYTES);
        core::ptr::copy_nonoverlapping(core::ptr::addr_of!((*slot).bytes).cast::<u8>(), out.as_mut_ptr(), len);
        let frame = core::ptr::read_volatile(core::ptr::addr_of!((*slot).frame));
        let uptime_ms = core::ptr::read_volatile(core::ptr::addr_of!((*slot).uptime_ms));
        let truncated = core::ptr::read_volatile(core::ptr::addr_of!((*slot).truncated)) != 0;
        fence(Ordering::Acquire);
        let again = (*slot).seq.load(Ordering::Relaxed);
        *next += 1;
        if seq != want || again != want {
            return Pop::Lost(1);
        }
        Pop::Line { len, frame, uptime_ms, truncated }
    }
}

#[cfg(test)]
#[path = "state_export_tests.rs"]
mod tests;
