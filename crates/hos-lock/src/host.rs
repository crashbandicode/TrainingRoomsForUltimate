// SPDX-License-Identifier: MIT
// Copyright (c) 2026 intpa

//! The host's [`Futex`](crate::word::Futex): one table lock and one condition variable for every word, so the
//! same state machines run (and are tested) off the Switch. The value check runs under the table lock and every
//! wake takes that lock after its word changed, so no wake is lost. `notify_all` wakes more threads than asked,
//! which the state machines treat as spurious returns.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Condvar, Mutex, PoisonError};

use crate::word::Futex;

pub struct HostFutex;

static GENERATION: Mutex<u64> = Mutex::new(0);
static CHANGED: Condvar = Condvar::new();

impl Futex for HostFutex {
    fn wait(word: &AtomicU32, expected: u32) {
        let mut generation = GENERATION.lock().unwrap_or_else(PoisonError::into_inner);
        if word.load(Ordering::SeqCst) != expected {
            return;
        }
        let seen = *generation;
        while *generation == seen {
            generation = CHANGED.wait(generation).unwrap_or_else(PoisonError::into_inner);
        }
    }

    fn wake(_word: &AtomicU32, _count: i32) {
        *GENERATION.lock().unwrap_or_else(PoisonError::into_inner) += 1;
        CHANGED.notify_all();
    }
}
