// SPDX-License-Identifier: MIT
// Copyright (c) 2026 intpa

//! The public types through lock_api, on the host futex: statics with `const` init, contention, handoff between
//! threads, writers first.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use hos_lock::{Mutex, RwLock};

static COUNTER: Mutex<u64> = Mutex::new(0);
static TABLE: RwLock<[u32; 2]> = RwLock::new([0, 0]);

#[test]
fn a_static_mutex_counts_exactly_across_threads() {
    let start = Arc::new(Barrier::new(8));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let start = start.clone();
            thread::spawn(move || {
                start.wait();
                for i in 0..5000u32 {
                    let mut guard = COUNTER.lock();
                    let v = *guard;
                    if i % 97 == 0 {
                        thread::yield_now();
                    }
                    *guard = v + 1;
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    assert_eq!(*COUNTER.lock(), 40_000);
    assert!(!COUNTER.is_locked());
}

#[test]
fn try_lock_fails_while_held_and_succeeds_after() {
    let lock = Mutex::new(1);
    let guard = lock.lock();
    assert!(lock.try_lock().is_none());
    drop(guard);
    assert_eq!(*lock.try_lock().unwrap(), 1);
}

#[test]
fn a_blocked_locker_runs_once_the_holder_unlocks() {
    let lock = Arc::new(Mutex::new(Vec::<u8>::new()));
    let guard = lock.lock();
    let entered = Arc::new(AtomicBool::new(false));
    let waiter = {
        let (lock, entered) = (lock.clone(), entered.clone());
        thread::spawn(move || {
            lock.lock().push(2);
            entered.store(true, SeqCst);
        })
    };
    thread::sleep(Duration::from_millis(30));
    assert!(!entered.load(SeqCst));
    drop(guard);
    waiter.join().unwrap();
    assert_eq!(*lock.lock(), vec![2]);
}

#[test]
fn a_static_rwlock_keeps_writes_whole_with_many_readers() {
    let start = Arc::new(Barrier::new(10));
    let torn = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    for _ in 0..3 {
        let start = start.clone();
        handles.push(thread::spawn(move || {
            start.wait();
            for i in 0..3000u32 {
                let mut w = TABLE.write();
                w[0] += 1;
                if i % 13 == 0 {
                    thread::yield_now();
                }
                w[1] += 1;
            }
        }));
    }
    for _ in 0..7 {
        let (start, torn) = (start.clone(), torn.clone());
        handles.push(thread::spawn(move || {
            start.wait();
            for _ in 0..3000 {
                let r = TABLE.read();
                if r[0] != r[1] {
                    torn.fetch_add(1, SeqCst);
                }
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    assert_eq!(torn.load(SeqCst), 0);
    assert_eq!(*TABLE.read(), [9000, 9000]);
    assert!(!TABLE.is_locked());
}

#[test]
fn readers_share_and_a_waiting_writer_keeps_new_readers_out() {
    let lock = Arc::new(RwLock::new(0u32));
    let first = lock.read();
    let second = lock.try_read().expect("readers share");
    let writer = {
        let lock = lock.clone();
        thread::spawn(move || {
            *lock.write() += 1;
        })
    };
    // Once the writer waits, a new reader is refused (writers first).
    let mut refused = false;
    for _ in 0..200 {
        match lock.try_read() {
            Some(g) => drop(g),
            None => {
                refused = true;
                break;
            }
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(refused, "a reader joined while a writer waited");
    drop(first);
    drop(second);
    writer.join().unwrap();
    assert_eq!(*lock.read(), 1);
}
