// SPDX-License-Identifier: MIT
// Copyright (c) 2026 intpa

//! `Mutex` and `RwLock` for Skyline plugins: [`lock_api`] raw locks on one 32-bit word that wait in the Horizon
//! kernel's address arbiter. No spin, no heap, `const` constructors; the API is parking_lot's, so code moves over by
//! changing its import. See README.md for why (parking_lot's parker spins forever on this target).
//!
//! On the Switch (`target_os = "switch"`) the waits are `svc 0x34`/`0x35` ([`horizon`]); everywhere else a
//! table-and-condvar futex ([`host`]), so the same state machines ([`word`]) run in host tests.

#![forbid(unsafe_op_in_unsafe_fn)]

pub mod event;
pub mod mailbox;
#[cfg(feature = "owners")]
pub mod owners;
pub mod snapshot;
pub mod triple;
pub mod word;

pub use event::Event;
pub use mailbox::Mailbox;
pub use snapshot::Published;
pub use triple::TripleBuffer;

#[cfg(all(target_os = "switch", target_arch = "aarch64"))]
pub mod horizon;
#[cfg(all(target_os = "switch", target_arch = "aarch64"))]
use horizon::Arbiter as Sys;

#[cfg(not(all(target_os = "switch", target_arch = "aarch64")))]
pub mod host;
#[cfg(not(all(target_os = "switch", target_arch = "aarch64")))]
use host::HostFutex as Sys;

use word::{MutexWord, RwLockWord};

/// Feature `owners`: enter an acquisition of the lock at `lock` in the held-lock table.
#[inline(always)]
fn took(lock: *const u8) {
    #[cfg(feature = "owners")]
    owners::acquired(lock as usize, owners::caller());
    #[cfg(not(feature = "owners"))]
    let _ = lock;
}

/// Feature `owners`: remove this thread's entry for the lock at `lock`.
#[inline(always)]
fn left(lock: *const u8) {
    #[cfg(feature = "owners")]
    owners::released(lock as usize);
    #[cfg(not(feature = "owners"))]
    let _ = lock;
}

/// A mutual-exclusion lock word (0 unlocked, 1 locked, 2 locked with sleepers).
pub struct RawMutex(MutexWord);

// SAFETY: `MutexWord` excludes (word.rs, tests). The guard is not `Send`, as with parking_lot's default.
unsafe impl lock_api::RawMutex for RawMutex {
    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: Self = RawMutex(MutexWord::new());
    type GuardMarker = lock_api::GuardNoSend;

    #[inline]
    fn lock(&self) {
        self.0.lock::<Sys>();
        took(self as *const Self as *const u8);
    }

    #[inline]
    fn try_lock(&self) -> bool {
        let taken = self.0.try_lock();
        if taken {
            took(self as *const Self as *const u8);
        }
        taken
    }

    #[inline]
    unsafe fn unlock(&self) {
        left(self as *const Self as *const u8);
        // SAFETY: lock_api's contract: the caller holds the lock.
        unsafe { self.0.unlock::<Sys>() }
    }

    #[inline]
    fn is_locked(&self) -> bool {
        self.0.is_locked()
    }
}

/// A reader-writer lock: writers first, not reentrant (a thread must not take a second read lock on a lock it
/// already reads while a writer may be waiting; parking_lot's `read` has the same rule).
pub struct RawRwLock(RwLockWord);

// SAFETY: `RwLockWord` is a reader-writer lock (word.rs, tests).
unsafe impl lock_api::RawRwLock for RawRwLock {
    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: Self = RawRwLock(RwLockWord::new());
    type GuardMarker = lock_api::GuardNoSend;

    #[inline]
    fn lock_shared(&self) {
        self.0.read::<Sys>();
        took(self as *const Self as *const u8);
    }

    #[inline]
    fn try_lock_shared(&self) -> bool {
        let taken = self.0.try_read();
        if taken {
            took(self as *const Self as *const u8);
        }
        taken
    }

    #[inline]
    unsafe fn unlock_shared(&self) {
        left(self as *const Self as *const u8);
        // SAFETY: the caller holds a read lock.
        unsafe { self.0.read_unlock::<Sys>() }
    }

    #[inline]
    fn lock_exclusive(&self) {
        self.0.write::<Sys>();
        took(self as *const Self as *const u8);
    }

    #[inline]
    fn try_lock_exclusive(&self) -> bool {
        let taken = self.0.try_write();
        if taken {
            took(self as *const Self as *const u8);
        }
        taken
    }

    #[inline]
    unsafe fn unlock_exclusive(&self) {
        left(self as *const Self as *const u8);
        // SAFETY: the caller holds the write lock.
        unsafe { self.0.write_unlock::<Sys>() }
    }

    #[inline]
    fn is_locked(&self) -> bool {
        self.0.is_locked()
    }

    #[inline]
    fn is_locked_exclusive(&self) -> bool {
        self.0.is_write_locked()
    }
}

pub type Mutex<T> = lock_api::Mutex<RawMutex, T>;
pub type MutexGuard<'a, T> = lock_api::MutexGuard<'a, RawMutex, T>;
pub type MappedMutexGuard<'a, T> = lock_api::MappedMutexGuard<'a, RawMutex, T>;
pub type RwLock<T> = lock_api::RwLock<RawRwLock, T>;
pub type RwLockReadGuard<'a, T> = lock_api::RwLockReadGuard<'a, RawRwLock, T>;
pub type RwLockWriteGuard<'a, T> = lock_api::RwLockWriteGuard<'a, RawRwLock, T>;
pub type MappedRwLockReadGuard<'a, T> = lock_api::MappedRwLockReadGuard<'a, RawRwLock, T>;
pub type MappedRwLockWriteGuard<'a, T> = lock_api::MappedRwLockWriteGuard<'a, RawRwLock, T>;
