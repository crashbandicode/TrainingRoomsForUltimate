// SPDX-License-Identifier: MIT
// Copyright (c) 2026 intpa

//! The Horizon kernel's address arbiter as the lock words' [`Futex`](crate::word::Futex).
//!
//! The calls (numbers, registers and meaning from the kernel the Eden tests run on: Eden
//! `src/core/hle/kernel/svc.h` ids, `svc.cpp` `SvcWrap_WaitForAddress64` / `SvcWrap_SignalToAddress64` register order,
//! `svc/svc_address_arbiter.cpp` and `k_address_arbiter.cpp` semantics, `svc_types.h` enums, `svc_results.h` results):
//! - `svc 0x34` WaitForAddress(x0 address, w1 ArbitrationType, w2 value, x3 timeout_ns) -> w0 result. With
//!   `WaitIfEqual` (2) the kernel reads the 4-byte-aligned word under its scheduler lock and sleeps only if it still
//!   equals `value`; otherwise it returns `InvalidState` (0xFA01). A negative timeout waits forever; 0 returns
//!   `TimedOut` (0xEA01) at once.
//! - `svc 0x35` SignalToAddress(x0 address, w1 SignalType, w2 value, w3 count) -> w0 result. With `Signal` (0) it ends
//!   the wait of up to `count` threads sleeping on the address (`count <= 0`: all). It does not report how many.
//!
//! Every non-success return (a mismatch, a timeout, `TerminationRequested` when the process ends) only makes the
//! caller re-check its word, which the state machines do in a loop. [`self_test`] checks the result codes at run time.

use core::fmt;
use core::sync::atomic::AtomicU32;

use crate::word::Futex;

pub struct Arbiter;

const WAIT_IF_EQUAL: u64 = 2;
const SIGNAL: u64 = 0;
/// A negative timeout: no timeout.
const FOREVER: i64 = -1;

/// `svc 0x34`; returns the result code.
///
/// # Safety
/// `word` is a live, 4-byte-aligned word of this process.
unsafe fn wait_for_address(word: *const u32, arbitration: u64, value: u32, timeout_ns: i64) -> u32 {
    let result: u64;
    // SAFETY: the kernel reads the word and may block this thread; x0–x7 are declared clobbered as for any supervisor
    // call. Not `nomem`: the wait orders memory like any lock operation.
    unsafe {
        core::arch::asm!(
            "svc #0x34",
            inout("x0") word as u64 => result,
            inout("x1") arbitration => _,
            inout("x2") u64::from(value) => _,
            inout("x3") timeout_ns as u64 => _,
            lateout("x4") _,
            lateout("x5") _,
            lateout("x6") _,
            lateout("x7") _,
            options(nostack),
        );
    }
    result as u32
}

/// `svc 0x35`; returns the result code.
///
/// # Safety
/// `word` is a live, 4-byte-aligned word of this process.
unsafe fn signal_to_address(word: *const u32, signal: u64, value: u32, count: i32) -> u32 {
    let result: u64;
    // SAFETY: as `wait_for_address`; the kernel only uses the address to find its sleepers.
    unsafe {
        core::arch::asm!(
            "svc #0x35",
            inout("x0") word as u64 => result,
            inout("x1") signal => _,
            inout("x2") u64::from(value) => _,
            inout("x3") i64::from(count) as u64 => _,
            lateout("x4") _,
            lateout("x5") _,
            lateout("x6") _,
            lateout("x7") _,
            options(nostack),
        );
    }
    result as u32
}

impl Futex for Arbiter {
    fn wait(word: &AtomicU32, expected: u32) {
        // SAFETY: an `AtomicU32` is a live, aligned word.
        let _ = unsafe { wait_for_address(word.as_ptr(), WAIT_IF_EQUAL, expected, FOREVER) };
    }

    fn wake(word: &AtomicU32, count: i32) {
        // SAFETY: as above.
        let _ = unsafe { signal_to_address(word.as_ptr(), SIGNAL, 0, count) };
    }
}

/// What the arbiter answered to three calls that cannot block: a wait on a mismatched value (`InvalidState`
/// 0xFA01), a wait with a zero timeout (`TimedOut` 0xEA01) and a signal with no sleeper (0).
#[derive(Clone, Copy, Debug)]
pub struct SelfTest {
    pub mismatch: u32,
    pub timeout: u32,
    pub signal: u32,
}

impl SelfTest {
    pub fn ok(&self) -> bool {
        self.mismatch == 0xFA01 && self.timeout == 0xEA01 && self.signal == 0
    }
}

impl fmt::Display for SelfTest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mismatch={:#x} timeout={:#x} signal={:#x} ok={}", self.mismatch, self.timeout, self.signal, self.ok())
    }
}

/// Run the three non-blocking calls on a local word.
pub fn self_test() -> SelfTest {
    let word = AtomicU32::new(7);
    // SAFETY: a live local word.
    unsafe {
        SelfTest {
            mismatch: wait_for_address(word.as_ptr(), WAIT_IF_EQUAL, 8, FOREVER),
            timeout: wait_for_address(word.as_ptr(), WAIT_IF_EQUAL, 7, 0),
            signal: signal_to_address(word.as_ptr(), SIGNAL, 0, 1),
        }
    }
}
