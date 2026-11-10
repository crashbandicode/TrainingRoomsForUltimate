// SPDX-License-Identifier: MIT
// Copyright (c) 2026 intpa

//! [`Event`]: any thread sets it, one thread waits for it, and the setter never waits. A set is an atomic swap and, only when
//! the waiter may be asleep, a wake through the futex (on the Switch the kernel's address arbiter: a supervisor call that takes
//! no lock of this process). It replaces `std::thread::park`/`unpark` for a thread that game hooks wake: std's parker on the
//! Skyline target is pthread's, whose `unpark` takes the parker's mutex, which the parked thread holds around its
//! condition-variable wait (and again after every wake), so a hook could wait for a plugin thread.
//!
//! Contract: one thread waits; any number set.

use core::sync::atomic::{AtomicU32, Ordering::{Acquire, Release}};
use core::time::Duration;

use crate::word::Futex;

const CLEAR: u32 = 0;
const SET: u32 = 1;

pub struct Event {
    word: AtomicU32,
}

impl Default for Event {
    fn default() -> Self {
        Self::new()
    }
}

impl Event {
    pub const fn new() -> Self {
        Self { word: AtomicU32::new(CLEAR) }
    }

    /// Any thread: set the event. The waiter's next [`wait`](Self::wait) returns at once, or a sleeping waiter wakes. Never
    /// waits. Writes made before it are seen by the waiter after its `wait` returns.
    pub fn set(&self) {
        // The waiter sleeps only while the word is CLEAR, so a set that finds it SET has nobody to wake.
        if self.word.swap(SET, Release) == CLEAR {
            <crate::Sys as Futex>::wake(&self.word, 1);
        }
    }

    /// The waiter (one thread): consume a set, or sleep until the event is set or `timeout` passes (`None`: no timeout). It may
    /// also return without a set, as `park` may: the caller re-checks its work either way.
    pub fn wait(&self, timeout: Option<Duration>) {
        if self.word.swap(CLEAR, Acquire) == SET {
            return;
        }
        sleep(&self.word, CLEAR, timeout);
        self.word.swap(CLEAR, Acquire);
    }
}

#[cfg(all(target_os = "switch", target_arch = "aarch64"))]
fn sleep(word: &AtomicU32, expected: u32, timeout: Option<Duration>) {
    let timeout_ns = timeout.map_or(-1, |t| i64::try_from(t.as_nanos()).unwrap_or(i64::MAX));
    crate::horizon::wait_timeout(word, expected, timeout_ns);
}

#[cfg(not(all(target_os = "switch", target_arch = "aarch64")))]
fn sleep(word: &AtomicU32, expected: u32, timeout: Option<Duration>) {
    crate::host::wait_timeout(word, expected, timeout);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
    use std::sync::Arc;
    use std::time::Instant;

    #[test]
    fn a_set_before_the_wait_returns_it_at_once_and_is_consumed() {
        let event = Event::new();
        event.set();
        event.set();
        let start = Instant::now();
        event.wait(Some(Duration::from_secs(10)));
        assert!(start.elapsed() < Duration::from_secs(5), "a pending set returns at once");
        let start = Instant::now();
        event.wait(Some(Duration::from_millis(30)));
        assert!(start.elapsed() >= Duration::from_millis(25), "two sets before one wait are one set");
    }

    #[test]
    fn a_wait_with_a_timeout_returns_after_it() {
        let event = Event::new();
        let start = Instant::now();
        event.wait(Some(Duration::from_millis(40)));
        let waited = start.elapsed();
        assert!(waited >= Duration::from_millis(35) && waited < Duration::from_secs(5), "{waited:?}");
    }

    #[test]
    fn a_set_from_another_thread_wakes_a_sleeping_waiter() {
        let event = Arc::new(Event::new());
        let setter = {
            let event = Arc::clone(&event);
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(50));
                event.set();
            })
        };
        let start = Instant::now();
        event.wait(None);
        assert!(start.elapsed() < Duration::from_secs(5));
        setter.join().unwrap();
    }

    /// Many setters racing one waiter: every published count is seen without a wait running out its timeout while a newer
    /// count was there (a lost wake would leave the waiter asleep for the full timeout with work pending).
    #[test]
    fn racing_setters_never_lose_a_wake() {
        const PER_SETTER: u64 = 20_000;
        const SETTERS: u64 = 3;
        let event = Arc::new(Event::new());
        let count = Arc::new(AtomicU64::new(0));
        let setters: Vec<_> = (0..SETTERS)
            .map(|_| {
                let (event, count) = (Arc::clone(&event), Arc::clone(&count));
                std::thread::spawn(move || {
                    for _ in 0..PER_SETTER {
                        count.fetch_add(1, SeqCst);
                        event.set();
                    }
                })
            })
            .collect();
        let mut seen = 0;
        while seen < PER_SETTER * SETTERS {
            let before = count.load(SeqCst);
            if before == seen {
                let start = Instant::now();
                event.wait(Some(Duration::from_secs(3)));
                if start.elapsed() >= Duration::from_secs(3) {
                    assert_eq!(count.load(SeqCst), before, "a wait slept its full timeout while a newer count was set: a lost wake");
                }
            }
            seen = count.load(SeqCst);
        }
        for setter in setters {
            setter.join().unwrap();
        }
    }
}
