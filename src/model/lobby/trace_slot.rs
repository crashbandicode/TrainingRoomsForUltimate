//! The newest-state slot and the transactional JSON write behind the Mii editor bridge's trace (lead ruling 2026-11-02 on
//! `mii-editor-last.json`, review R1/R5): a game callback only copies its latest state into a slot (no allocation, no file I/O); a
//! writer thread takes the slot and writes the file through `model::store::write_replace`, so a reader or an interrupted write never
//! sees a partial file and a failed write leaves the old file in place. A newer state replaces an older unwritten one (coalesced).
//! Written for this repository, not carried.

use std::io;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;

/// The newest unwritten state of one kind.
pub struct LatestSlot<T> {
    value: Mutex<Option<T>>,
}

impl<T> LatestSlot<T> {
    pub const fn new() -> Self {
        Self { value: Mutex::new(None) }
    }

    /// Store `value`, replacing an unwritten older one.
    pub fn put(&self, value: T) {
        if let Ok(mut slot) = self.value.lock() {
            *slot = Some(value);
        }
    }

    /// Take the newest state, if any.
    pub fn take(&self) -> Option<T> {
        self.value.lock().ok().and_then(|mut slot| slot.take())
    }

    /// Give a state back after a failed write, unless a newer one arrived meanwhile.
    pub fn restore(&self, value: T) {
        if let Ok(mut slot) = self.value.lock() {
            slot.get_or_insert(value);
        }
    }
}

impl<T> Default for LatestSlot<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// The newest unwritten state of a large `Copy` kind, kept off every stack (stream/stack, 2026-11-02). `LatestSlot<T>` moves
/// its `T` by value through `put`, `take` and `restore`; for the Mii editor's 11 KB trace that gave the writer thread's body a
/// 59,712 B frame on its 64 KiB stack and the first SDK call under it ran off the bottom (Eden s4-07, u4b-02). Here the state
/// lives in two heap buffers made once by `prepare`: `put` copies the producer's state into the pending buffer in place, and
/// the writer swaps that buffer for its own and serialises it by reference. Only box pointers move; after `prepare` nothing
/// allocates.
pub struct BoxedSlot<T: Copy> {
    inner: Mutex<Buffers<T>>,
}

struct Buffers<T> {
    /// The buffer `put` writes into.
    pending: Option<Box<T>>,
    /// The buffer the writer swaps in when it takes `pending`.
    spare: Option<Box<T>>,
    /// `pending` holds a state not yet written.
    dirty: bool,
}

/// A buffer the writer took; give it back with `BoxedSlot::finish`.
pub struct Taken<T>(Box<T>);

impl<T> std::ops::Deref for Taken<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Copy> BoxedSlot<T> {
    pub const fn new() -> Self {
        Self { inner: Mutex::new(Buffers { pending: None, spare: None, dirty: false }) }
    }

    /// Make the two buffers, each a copy of `empty` written straight to the heap (not through a stack temporary). Once.
    pub fn prepare(&self, empty: &T) {
        let Ok(mut inner) = self.inner.lock() else { return };
        if inner.pending.is_none() {
            inner.pending = Some(boxed_copy(empty));
        }
        if inner.spare.is_none() {
            inner.spare = Some(boxed_copy(empty));
        }
    }

    /// Store `value` as the newest state, in place (an unwritten older one is replaced). Before `prepare`, dropped.
    pub fn put(&self, value: &T) -> bool {
        let Ok(mut inner) = self.inner.lock() else { return false };
        let Some(pending) = inner.pending.as_deref_mut() else { return false };
        // SAFETY: two distinct live `T`s (the producer's and this slot's buffer); `T: Copy`, so a byte copy is a copy.
        unsafe { std::ptr::copy_nonoverlapping(value as *const T, pending as *mut T, 1) };
        inner.dirty = true;
        true
    }

    /// The newest unwritten state, swapped out of the slot (the spare buffer becomes the pending one).
    pub fn take(&self) -> Option<Taken<T>> {
        let mut inner = self.inner.lock().ok()?;
        if !inner.dirty || inner.spare.is_none() {
            return None;
        }
        inner.dirty = false;
        let full = inner.pending.take()?;
        inner.pending = inner.spare.take();
        Some(Taken(full))
    }

    /// Return a taken buffer. After a failed write its state becomes pending again, unless a newer one arrived meanwhile.
    pub fn finish(&self, taken: Taken<T>, written: bool) {
        let Ok(mut inner) = self.inner.lock() else { return };
        if !written && !inner.dirty {
            inner.spare = inner.pending.take();
            inner.pending = Some(taken.0);
            inner.dirty = true;
        } else {
            inner.spare = Some(taken.0);
        }
    }
}

impl<T: Copy> Default for BoxedSlot<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A heap copy of `value` made without a stack temporary of `T`.
fn boxed_copy<T: Copy>(value: &T) -> Box<T> {
    let mut boxed = Box::<T>::new_uninit();
    // SAFETY: the box has room for one `T`; `T: Copy` so copying its bytes makes a valid `T`.
    unsafe {
        std::ptr::copy_nonoverlapping(value as *const T, boxed.as_mut_ptr(), 1);
        boxed.assume_init()
    }
}

/// [`flush_with`] for a [`BoxedSlot`]: the state is written from its heap buffer by reference.
pub fn flush_boxed<T: Copy>(slot: &BoxedSlot<T>, write: impl FnOnce(&T) -> io::Result<()>) -> io::Result<bool> {
    let Some(taken) = slot.take() else { return Ok(false) };
    let result = write(&taken);
    slot.finish(taken, result.is_ok());
    result.map(|()| true)
}

/// `value` as pretty JSON, written to `path` transactionally.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    crate::model::store::write_replace(path, &bytes)
}

/// One writer-thread turn: write the newest state of `slot` to `path`. `Ok(false)` when the slot was empty; on a failed write the
/// state goes back into the slot (unless a newer one is there) and the error is returned, with the old file untouched.
pub fn flush<T: Serialize>(slot: &LatestSlot<T>, path: &Path) -> io::Result<bool> {
    flush_with(slot, |value| write_json(path, value))
}

/// [`flush`] with the writer given as a function (the carried bridge's `write_trace` logs its event line and then calls
/// [`write_json`]).
pub fn flush_with<T>(slot: &LatestSlot<T>, write: impl FnOnce(&T) -> io::Result<()>) -> io::Result<bool> {
    let Some(value) = slot.take() else { return Ok(false) };
    match write(&value) {
        Ok(()) => Ok(true),
        Err(error) => {
            slot.restore(value);
            Err(error)
        }
    }
}

