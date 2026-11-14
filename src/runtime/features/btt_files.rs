//! Break the Targets' saved layouts on disk (`artifacts/rc14-eggsercise/SPEC.md` §V2.5 "Saved layouts"): `targets/<stage>.json`,
//! read and written by the writer thread only. Game threads do no file I/O and allocate nothing here: they leave a request in an
//! atomic or a mailbox and take the answer from a published copy (`hos_lock::Published`, read without waiting). The writer
//! thread publishes after every read and every write it completes.
//!
//! * `request_read(stage)`, `take_read()`: the game thread asks for a stage's file and takes the slots once they are read.
//! * `request_write(stage, slots)`: the whole file of a stage is rewritten (newest wins per stage; at most one retry).
//! * `cached()`: the stage and slots as last read or written, for the Saved layouts screen (any thread).

use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::path::Path;

use hos_lock::{Mailbox, Published};

use crate::model::btt::{self, Slots, SLOTS};
use crate::model::notify::DEFAULT_COLOR;
use crate::model::storage::ROOT;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::notices;

/// No stage: no read is waiting, nothing has been read yet.
const NONE: i32 = i32::MIN;
/// The stage whose file the game thread wants read (newest wins).
static READ: AtomicI32 = AtomicI32::new(NONE);
/// Whole-file writes, in order; four is more than a player can ask for in one poll of the writer.
static WRITES: Mailbox<(i32, Slots), 4> = Mailbox::new();
/// What the last read found (stage, slots, whether the file could not be read), and the last generation the game thread took.
static RESULT: Published<(i32, Slots, bool)> = Published::new((NONE, [None; SLOTS], false));
static SEEN: AtomicU32 = AtomicU32::new(0);
/// The stage and slots as just read or written; an unreadable file is empty slots.
static CACHE: Published<(i32, Slots)> = Published::new((NONE, [None; SLOTS]));

/// Ask for `stage`'s file to be read (any thread, no allocation, newest wins).
pub fn request_read(stage: i32) {
    READ.store(stage, Ordering::Release);
}

/// The slots of the file read since the last call: stage, slots, and whether the file could not be read (game thread, no wait).
pub fn take_read() -> Option<(i32, Slots, bool)> {
    RESULT.read_if_newer(&SEEN)
}

/// Ask for `stage`'s file to be rewritten with `slots` (any thread, no allocation).
pub fn request_write(stage: i32, slots: Slots) {
    if !WRITES.post((stage, slots)) {
        diagnostic(format_args!("[Btt] target layouts write dropped stage={stage}: the writer is behind"));
    }
}

/// The stage and slots as last read or written (any thread, no wait).
pub fn cached() -> (i32, Slots) {
    CACHE.read()
}

/// The writer thread, every poll: the writes asked for, then the read asked for.
pub fn service() {
    let root = Path::new(ROOT);
    WRITES.drain(|(stage, slots)| {
        // At most one retry. A failed write keeps the layouts in memory for the session and says so.
        if !(0..2).any(|_| btt::write(root, stage, &slots).is_ok()) {
            diagnostic(format_args!("[Btt] target layouts for stage {stage} not written; kept for this session"));
            notices::post("Break the Targets", "Could not save the target layouts", 120, DEFAULT_COLOR);
        }
        CACHE.publish((stage, slots));
    });
    let stage = READ.swap(NONE, Ordering::AcqRel);
    if stage != NONE {
        let (slots, unreadable) = btt::read(root, stage);
        diagnostic(format_args!("[Btt] target layouts read stage={stage} unreadable={unreadable}"));
        RESULT.publish((stage, slots, unreadable));
        CACHE.publish((stage, slots));
    }
}