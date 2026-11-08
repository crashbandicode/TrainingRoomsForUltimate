// SPDX-License-Identifier: MIT
// Copyright (c) 2026 intpa

//! Feature `owners` (a diagnostic build's): which thread holds which lock, and where it took it.
//!
//! Every acquisition of a [`Mutex`](crate::Mutex) or [`RwLock`](crate::RwLock) (read or write) is entered in one fixed
//! table: the lock word's address, the holding thread and the return address of the call that took it; the release
//! removes it. A watchdog thread reads the table with [`snapshot`] when another thread stops, so it can say who holds
//! what. Atomics only: no lock, no heap. A full table drops the entry and counts it in [`overflow`].

use core::sync::atomic::{AtomicUsize, Ordering};

/// Entries the table holds at once.
pub const SLOTS: usize = 128;

/// One held lock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Held {
    /// The address of the lock's word.
    pub lock: usize,
    /// The holding thread (its TLS pointer on the Switch, an address unique to the thread on the host).
    pub thread: usize,
    /// The return address of the call that took the lock (0 on the host).
    pub pc: usize,
}

struct Slot {
    lock: AtomicUsize,
    thread: AtomicUsize,
    pc: AtomicUsize,
}

static TABLE: [Slot; SLOTS] =
    [const { Slot { lock: AtomicUsize::new(0), thread: AtomicUsize::new(0), pc: AtomicUsize::new(0) } }; SLOTS];
static OVERFLOW: AtomicUsize = AtomicUsize::new(0);

/// The calling thread.
pub fn current_thread() -> usize {
    #[cfg(all(target_os = "switch", target_arch = "aarch64"))]
    {
        let tls: usize;
        // SAFETY: reads the read-only thread pointer register; no memory is touched.
        unsafe { core::arch::asm!("mrs {}, tpidrro_el0", out(reg) tls, options(nomem, nostack, preserves_flags)) };
        tls
    }
    #[cfg(not(all(target_os = "switch", target_arch = "aarch64")))]
    {
        std::thread_local!(static MARK: u8 = const { 0 });
        MARK.with(|mark| mark as *const u8 as usize)
    }
}

/// Enter `lock` as held by this thread, taken at `pc`.
pub fn acquired(lock: usize, pc: usize) {
    let thread = current_thread();
    for slot in &TABLE {
        if slot.lock.compare_exchange(0, usize::MAX, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            slot.thread.store(thread, Ordering::Relaxed);
            slot.pc.store(pc, Ordering::Relaxed);
            slot.lock.store(lock, Ordering::Release);
            return;
        }
    }
    OVERFLOW.fetch_add(1, Ordering::Relaxed);
}

/// Remove this thread's entry for `lock`.
pub fn released(lock: usize) {
    let thread = current_thread();
    for slot in &TABLE {
        if slot.lock.load(Ordering::Acquire) == lock && slot.thread.load(Ordering::Relaxed) == thread {
            slot.lock.store(0, Ordering::Release);
            return;
        }
    }
}

/// The held locks, up to `out.len()` of them; returns how many were written.
pub fn snapshot(out: &mut [Held]) -> usize {
    let mut n = 0;
    for slot in &TABLE {
        if n == out.len() {
            break;
        }
        let lock = slot.lock.load(Ordering::Acquire);
        if lock == 0 || lock == usize::MAX {
            continue;
        }
        out[n] = Held { lock, thread: slot.thread.load(Ordering::Relaxed), pc: slot.pc.load(Ordering::Relaxed) };
        n += 1;
    }
    n
}

/// Acquisitions dropped because the table was full.
pub fn overflow() -> usize {
    OVERFLOW.load(Ordering::Relaxed)
}

/// This call's return address: called from the lock wrappers, which are inlined into the code that takes the lock, it
/// is an address in that code.
#[inline(never)]
pub fn caller() -> usize {
    #[cfg(all(target_os = "switch", target_arch = "aarch64"))]
    {
        let lr: usize;
        // SAFETY: reads the link register at entry (this function is never inlined and calls nothing before the read).
        unsafe { core::arch::asm!("mov {}, x30", out(reg) lr, options(nomem, nostack, preserves_flags)) };
        lr
    }
    #[cfg(not(all(target_os = "switch", target_arch = "aarch64")))]
    {
        0
    }
}
