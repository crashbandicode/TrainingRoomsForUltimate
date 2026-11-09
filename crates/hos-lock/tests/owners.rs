// SPDX-License-Identifier: MIT
// Copyright (c) 2026 intpa

//! Feature `owners`: every held lock is in the table with its holder, and leaves it on release.
#![cfg(feature = "owners")]

use hos_lock::owners::{self, Held};
use hos_lock::{Mutex, RwLock};

fn held_by(lock: usize) -> Vec<Held> {
    let mut out = [Held::default(); owners::SLOTS];
    let n = owners::snapshot(&mut out);
    out[..n].iter().copied().filter(|h| h.lock == lock).collect()
}

fn word<T>(lock: &T) -> usize {
    lock as *const T as usize
}

#[test]
fn a_held_mutex_names_its_thread_and_leaves_on_release() {
    static M: Mutex<u32> = Mutex::new(0);
    let addr = word(unsafe { M.raw() });
    assert!(held_by(addr).is_empty());
    let guard = M.lock();
    let held = held_by(addr);
    assert_eq!(held.len(), 1, "the lock is held once");
    assert_eq!(held[0].thread, owners::current_thread());
    drop(guard);
    assert!(held_by(addr).is_empty(), "released");
    let guard = M.try_lock().expect("free");
    assert_eq!(held_by(addr).len(), 1, "try_lock enters it too");
    drop(guard);
    assert!(held_by(addr).is_empty());
}

#[test]
fn readers_are_entered_per_thread_and_the_writer_once() {
    static L: RwLock<u32> = RwLock::new(0);
    let addr = word(unsafe { L.raw() });
    let read = L.read();
    let other = std::thread::spawn(move || {
        let read = L.read();
        let held = held_by(addr);
        drop(read);
        held
    })
    .join()
    .unwrap();
    assert_eq!(other.len(), 2, "two readers: this thread and the other one");
    assert_ne!(other[0].thread, other[1].thread);
    drop(read);
    assert!(held_by(addr).is_empty());
    let write = L.write();
    assert_eq!(held_by(addr).len(), 1);
    drop(write);
    assert!(held_by(addr).is_empty());
}
