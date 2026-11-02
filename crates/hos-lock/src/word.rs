// SPDX-License-Identifier: MIT
// Copyright (c) 2026 intpa

//! Blocking locks on 32-bit words: the state machines behind `RawMutex` and `RawRwLock`.
//!
//! The waits go through [`Futex`]: sleep while a word holds an expected value, and wake sleepers on a
//! word. On the Switch that is the kernel's address arbiter (`horizon.rs`); on the host a condition
//! variable over one table (`host.rs`). A waiter never spins: on a fixed-priority scheduler a spinning
//! waiter keeps a lower-priority holder on its core from ever running. That livelock is why
//! parking_lot is not used. Its parker on this target is a pure busy loop (`parking_lot_core`
//! `thread_parker/generic.rs`, picked because the Skyline target is neither unix nor windows).
//!
//! The algorithms are the futex mutex and the futex reader-writer lock of the Rust standard library
//! (`library/std/src/sys/sync/{mutex,rwlock}/futex.rs`), without their spin phases and without
//! `downgrade`. A wake that may have woken nobody is treated as such (the Switch arbiter does not
//! report how many threads it woke).
//!
//! Semantics the callers keep from parking_lot:
//! - neither lock is reentrant;
//! - the reader-writer lock prefers writers: a read waits while a writer waits, so a thread must not
//!   take a second read lock on a lock it already reads.
//!
//! Every wait may end spuriously, by a timeout, or because the word changed before the kernel looked.
//! The callers re-check the word in a loop, so any return of [`Futex::wait`] is correct.

use std::sync::atomic::{AtomicU32, Ordering::{Acquire, Relaxed, Release}};

/// The sleep and wake primitive under the locks.
pub trait Futex {
    /// Sleep while `*word == expected`. The comparison and the sleep are atomic with respect to
    /// [`Futex::wake`] on the same word. It may return at any time; the caller re-checks.
    fn wait(word: &AtomicU32, expected: u32);
    /// Wake at most `count` sleepers on `word`; `count <= 0` wakes them all.
    fn wake(word: &AtomicU32, count: i32);
}

const UNLOCKED: u32 = 0;
/// Locked, nobody sleeps on it.
const LOCKED: u32 = 1;
/// Locked, and a waiter may sleep on it: the unlock must wake one.
const CONTENDED: u32 = 2;

/// A mutex word: 0 unlocked, 1 locked, 2 locked with waiters.
#[derive(Debug)]
pub struct MutexWord {
    state: AtomicU32,
}

impl Default for MutexWord {
    fn default() -> Self {
        Self::new()
    }
}

impl MutexWord {
    pub const fn new() -> Self {
        Self { state: AtomicU32::new(UNLOCKED) }
    }

    pub fn try_lock(&self) -> bool {
        self.state.compare_exchange(UNLOCKED, LOCKED, Acquire, Relaxed).is_ok()
    }

    pub fn lock<F: Futex>(&self) {
        if !self.try_lock() {
            self.lock_contended::<F>();
        }
    }

    #[cold]
    fn lock_contended<F: Futex>(&self) {
        // Mark the word contended before sleeping: whoever unlocks next then wakes a sleeper. A thread
        // that takes the lock this way keeps the mark (it cannot know whether others still sleep), which
        // costs at most one wake that finds nobody.
        while self.state.swap(CONTENDED, Acquire) != UNLOCKED {
            F::wait(&self.state, CONTENDED);
        }
    }

    /// # Safety
    /// The caller holds the lock.
    pub unsafe fn unlock<F: Futex>(&self) {
        if self.state.swap(UNLOCKED, Release) == CONTENDED {
            F::wake(&self.state, 1);
        }
    }

    pub fn is_locked(&self) -> bool {
        self.state.load(Relaxed) != UNLOCKED
    }

    /// The raw word (tests and the diag self-test).
    pub fn word(&self) -> &AtomicU32 {
        &self.state
    }
}

const READ_LOCKED: u32 = 1;
const MASK: u32 = (1 << 30) - 1;
const WRITE_LOCKED: u32 = MASK;
const MAX_READERS: u32 = MASK - 1;
const READERS_WAITING: u32 = 1 << 30;
const WRITERS_WAITING: u32 = 1 << 31;

fn is_unlocked(state: u32) -> bool {
    state & MASK == 0
}

fn is_write_locked(state: u32) -> bool {
    state & MASK == WRITE_LOCKED
}

fn has_readers_waiting(state: u32) -> bool {
    state & READERS_WAITING != 0
}

fn has_writers_waiting(state: u32) -> bool {
    state & WRITERS_WAITING != 0
}

/// A read lock can be taken: room for another reader and nobody waiting. Readers wait while a writer
/// waits (writers first), and while readers wait (an unlocking thread is about to wake them and clears
/// the bit first).
fn is_read_lockable(state: u32) -> bool {
    state & MASK < MAX_READERS && !has_readers_waiting(state) && !has_writers_waiting(state)
}

/// A reader-writer lock: `state` holds the reader count (or `WRITE_LOCKED`) and the two waiting bits;
/// writers sleep on `writer_notify`, a counter bumped on every writer wake, so a wake between a
/// writer's check and its sleep is never lost.
#[derive(Debug)]
pub struct RwLockWord {
    state: AtomicU32,
    writer_notify: AtomicU32,
}

impl Default for RwLockWord {
    fn default() -> Self {
        Self::new()
    }
}

impl RwLockWord {
    pub const fn new() -> Self {
        Self { state: AtomicU32::new(0), writer_notify: AtomicU32::new(0) }
    }

    pub fn try_read(&self) -> bool {
        self.state.fetch_update(Acquire, Relaxed, |s| is_read_lockable(s).then(|| s + READ_LOCKED)).is_ok()
    }

    pub fn read<F: Futex>(&self) {
        let state = self.state.load(Relaxed);
        if !is_read_lockable(state) || self.state.compare_exchange_weak(state, state + READ_LOCKED, Acquire, Relaxed).is_err() {
            self.read_contended::<F>();
        }
    }

    /// # Safety
    /// The caller holds a read lock.
    pub unsafe fn read_unlock<F: Futex>(&self) {
        let state = self.state.fetch_sub(READ_LOCKED, Release) - READ_LOCKED;
        // A reader only waits on a read-locked word when a writer waits too.
        debug_assert!(!has_readers_waiting(state) || has_writers_waiting(state));
        if is_unlocked(state) && has_writers_waiting(state) {
            self.wake_writer_or_readers::<F>(state);
        }
    }

    #[cold]
    fn read_contended<F: Futex>(&self) {
        let mut state = self.state.load(Relaxed);
        loop {
            if is_read_lockable(state) {
                match self.state.compare_exchange_weak(state, state + READ_LOCKED, Acquire, Relaxed) {
                    Ok(_) => return,
                    Err(s) => {
                        state = s;
                        continue;
                    }
                }
            }
            assert!(state & MASK != MAX_READERS, "too many read locks on one RwLock");
            if !has_readers_waiting(state) {
                if let Err(s) = self.state.compare_exchange(state, state | READERS_WAITING, Relaxed, Relaxed) {
                    state = s;
                    continue;
                }
            }
            F::wait(&self.state, state | READERS_WAITING);
            state = self.state.load(Relaxed);
        }
    }

    pub fn try_write(&self) -> bool {
        self.state.fetch_update(Acquire, Relaxed, |s| is_unlocked(s).then(|| s + WRITE_LOCKED)).is_ok()
    }

    pub fn write<F: Futex>(&self) {
        if self.state.compare_exchange_weak(0, WRITE_LOCKED, Acquire, Relaxed).is_err() {
            self.write_contended::<F>();
        }
    }

    /// # Safety
    /// The caller holds the write lock.
    pub unsafe fn write_unlock<F: Futex>(&self) {
        let state = self.state.fetch_sub(WRITE_LOCKED, Release) - WRITE_LOCKED;
        debug_assert!(is_unlocked(state));
        if has_writers_waiting(state) || has_readers_waiting(state) {
            self.wake_writer_or_readers::<F>(state);
        }
    }

    #[cold]
    fn write_contended<F: Futex>(&self) {
        let mut state = self.state.load(Relaxed);
        let mut other_writers_waiting = 0;
        loop {
            if is_unlocked(state) {
                // Keep the writers-waiting bit once another writer may sleep: it is ours to wake.
                match self.state.compare_exchange_weak(state, state | WRITE_LOCKED | other_writers_waiting, Acquire, Relaxed) {
                    Ok(_) => return,
                    Err(s) => {
                        state = s;
                        continue;
                    }
                }
            }
            if !has_writers_waiting(state) {
                if let Err(s) = self.state.compare_exchange(state, state | WRITERS_WAITING, Relaxed, Relaxed) {
                    state = s;
                    continue;
                }
            }
            other_writers_waiting = WRITERS_WAITING;
            // Read the counter before re-checking the state, so a wake in between changes the counter
            // and the wait below returns at once.
            let seq = self.writer_notify.load(Acquire);
            state = self.state.load(Relaxed);
            if is_unlocked(state) || !has_writers_waiting(state) {
                continue;
            }
            F::wait(&self.writer_notify, seq);
            state = self.state.load(Relaxed);
        }
    }

    /// The lock is unlocked and someone waits: wake one writer, or else every reader.
    #[cold]
    fn wake_writer_or_readers<F: Futex>(&self, mut state: u32) {
        debug_assert!(is_unlocked(state));
        // Only writers wait: wake one.
        if state == WRITERS_WAITING {
            match self.state.compare_exchange(state, 0, Relaxed, Relaxed) {
                Ok(_) => {
                    self.wake_writer::<F>();
                    return;
                }
                // Readers may wait now too: fall through.
                Err(s) => state = s,
            }
        }
        // Both wait: leave the readers waiting and wake one writer. The arbiter does not say whether a
        // writer was asleep to receive it, so the readers are woken as well; a reader that loses to the
        // writer sleeps again.
        if state == READERS_WAITING + WRITERS_WAITING {
            if self.state.compare_exchange(state, READERS_WAITING, Relaxed, Relaxed).is_err() {
                // Locked again meanwhile: that holder wakes the waiters when it unlocks.
                return;
            }
            self.wake_writer::<F>();
            state = READERS_WAITING;
        }
        if state == READERS_WAITING && self.state.compare_exchange(state, 0, Relaxed, Relaxed).is_ok() {
            F::wake(&self.state, 0);
        }
    }

    fn wake_writer<F: Futex>(&self) {
        self.writer_notify.fetch_add(1, Release);
        F::wake(&self.writer_notify, 1);
    }

    pub fn is_locked(&self) -> bool {
        self.state.load(Relaxed) & MASK != 0
    }

    pub fn is_write_locked(&self) -> bool {
        is_write_locked(self.state.load(Relaxed))
    }

    /// The raw state word (tests).
    pub fn state_bits(&self) -> u32 {
        self.state.load(Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::UnsafeCell;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
    use std::sync::{Arc, Barrier, Condvar, Mutex};
    use std::thread;
    use std::time::Duration;

    /// A host futex: one table lock and condition variable for every word. The value check runs under
    /// the table lock and every wake takes it after the word changed, so no wake is lost; `notify_all`
    /// wakes more threads than asked, which the locks must tolerate (spurious returns).
    struct HostTable {
        generation: Mutex<u64>,
        changed: Condvar,
        waits: AtomicUsize,
        wakes: AtomicUsize,
    }

    static TABLE: HostTable =
        HostTable { generation: Mutex::new(0), changed: Condvar::new(), waits: AtomicUsize::new(0), wakes: AtomicUsize::new(0) };
    /// Make every `SPURIOUS`-th wait return at once without sleeping.
    static SPURIOUS: AtomicUsize = AtomicUsize::new(0);

    struct HostFutex;

    impl Futex for HostFutex {
        fn wait(word: &AtomicU32, expected: u32) {
            let n = TABLE.waits.fetch_add(1, SeqCst) + 1;
            let every = SPURIOUS.load(SeqCst);
            if n.checked_rem(every) == Some(0) {
                return;
            }
            let mut generation = TABLE.generation.lock().unwrap();
            if word.load(SeqCst) != expected {
                return;
            }
            let seen = *generation;
            // A bounded sleep: a timeout is one more kind of early return the locks must survive.
            while *generation == seen {
                let (g, timeout) = TABLE.changed.wait_timeout(generation, Duration::from_millis(50)).unwrap();
                generation = g;
                if timeout.timed_out() {
                    return;
                }
            }
        }

        fn wake(_word: &AtomicU32, _count: i32) {
            TABLE.wakes.fetch_add(1, SeqCst);
            *TABLE.generation.lock().unwrap() += 1;
            TABLE.changed.notify_all();
        }
    }

    /// The tests share the futex statistics: run them one at a time.
    static SERIAL: Mutex<()> = Mutex::new(());

    struct Guarded<T> {
        lock: MutexWord,
        value: UnsafeCell<T>,
    }
    unsafe impl<T: Send> Sync for Guarded<T> {}

    fn hammer_mutex(threads: usize, rounds: usize) -> usize {
        let shared = Arc::new(Guarded { lock: MutexWord::new(), value: UnsafeCell::new(0usize) });
        let start = Arc::new(Barrier::new(threads));
        let handles: Vec<_> = (0..threads)
            .map(|_| {
                let shared = shared.clone();
                let start = start.clone();
                thread::spawn(move || {
                    start.wait();
                    for _ in 0..rounds {
                        shared.lock.lock::<HostFutex>();
                        // A non-atomic read-modify-write with a yield inside: lost updates show a broken lock.
                        let v = unsafe { *shared.value.get() };
                        if v % 7 == 0 {
                            thread::yield_now();
                        }
                        unsafe { *shared.value.get() = v + 1 };
                        unsafe { shared.lock.unlock::<HostFutex>() };
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert!(!shared.lock.is_locked());
        assert_eq!(shared.lock.word().load(SeqCst), UNLOCKED);
        unsafe { *shared.value.get() }
    }

    #[test]
    fn the_mutex_counts_exactly_under_contention_and_sleeps_instead_of_spinning() {
        let _serial = SERIAL.lock().unwrap();
        SPURIOUS.store(0, SeqCst);
        let waits = TABLE.waits.load(SeqCst);
        assert_eq!(hammer_mutex(8, 4000), 8 * 4000);
        assert!(TABLE.waits.load(SeqCst) > waits, "the contended path (a futex wait) was never taken");
    }

    #[test]
    fn the_mutex_survives_spurious_wakeups() {
        let _serial = SERIAL.lock().unwrap();
        SPURIOUS.store(3, SeqCst);
        assert_eq!(hammer_mutex(6, 3000), 6 * 3000);
        SPURIOUS.store(0, SeqCst);
    }

    #[test]
    fn the_mutex_hands_off_to_a_sleeping_waiter() {
        let _serial = SERIAL.lock().unwrap();
        SPURIOUS.store(0, SeqCst);
        let lock = Arc::new(MutexWord::new());
        lock.lock::<HostFutex>();
        assert!(!lock.try_lock());
        let got = Arc::new(AtomicBool::new(false));
        let waiter = {
            let (lock, got) = (lock.clone(), got.clone());
            thread::spawn(move || {
                lock.lock::<HostFutex>();
                got.store(true, SeqCst);
                unsafe { lock.unlock::<HostFutex>() };
            })
        };
        // The waiter marks the word contended and sleeps; the holder's unlock must wake it.
        while lock.word().load(SeqCst) != CONTENDED {
            thread::yield_now();
        }
        assert!(!got.load(SeqCst));
        let wakes = TABLE.wakes.load(SeqCst);
        unsafe { lock.unlock::<HostFutex>() };
        assert!(TABLE.wakes.load(SeqCst) > wakes, "a contended unlock must wake");
        waiter.join().unwrap();
        assert!(got.load(SeqCst));
        assert!(lock.try_lock());
        unsafe { lock.unlock::<HostFutex>() };
        assert_eq!(lock.word().load(SeqCst), UNLOCKED);
    }

    #[test]
    fn an_uncontended_unlock_does_not_wake() {
        let _serial = SERIAL.lock().unwrap();
        let lock = MutexWord::new();
        let wakes = TABLE.wakes.load(SeqCst);
        for _ in 0..100 {
            lock.lock::<HostFutex>();
            unsafe { lock.unlock::<HostFutex>() };
        }
        assert_eq!(TABLE.wakes.load(SeqCst), wakes);
    }

    struct Pair {
        lock: RwLockWord,
        a: UnsafeCell<u64>,
        b: UnsafeCell<u64>,
    }
    unsafe impl Sync for Pair {}

    fn hammer_rwlock(writers: usize, readers: usize, rounds: usize) {
        let pair = Arc::new(Pair { lock: RwLockWord::new(), a: UnsafeCell::new(0), b: UnsafeCell::new(0) });
        let start = Arc::new(Barrier::new(writers + readers));
        let mut handles = Vec::new();
        for _ in 0..writers {
            let (pair, start) = (pair.clone(), start.clone());
            handles.push(thread::spawn(move || {
                start.wait();
                for i in 0..rounds {
                    pair.lock.write::<HostFutex>();
                    unsafe { *pair.a.get() += 1 };
                    if i % 5 == 0 {
                        thread::yield_now();
                    }
                    unsafe { *pair.b.get() += 1 };
                    unsafe { pair.lock.write_unlock::<HostFutex>() };
                }
            }));
        }
        for _ in 0..readers {
            let (pair, start) = (pair.clone(), start.clone());
            handles.push(thread::spawn(move || {
                start.wait();
                for i in 0..rounds {
                    pair.lock.read::<HostFutex>();
                    let (a, b) = unsafe { (*pair.a.get(), *pair.b.get()) };
                    if i % 3 == 0 {
                        thread::yield_now();
                    }
                    assert_eq!(a, b, "a reader saw a half-done write");
                    unsafe { pair.lock.read_unlock::<HostFutex>() };
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(unsafe { *pair.a.get() }, (writers * rounds) as u64);
        assert_eq!(pair.lock.state_bits(), 0, "every waiting bit is cleared at the end");
    }

    #[test]
    fn the_rwlock_keeps_writes_whole_under_contention() {
        let _serial = SERIAL.lock().unwrap();
        SPURIOUS.store(0, SeqCst);
        hammer_rwlock(4, 6, 2000);
    }

    #[test]
    fn the_rwlock_survives_spurious_wakeups() {
        let _serial = SERIAL.lock().unwrap();
        SPURIOUS.store(4, SeqCst);
        hammer_rwlock(3, 5, 1500);
        SPURIOUS.store(0, SeqCst);
    }

    #[test]
    fn readers_share_and_a_writer_excludes() {
        let _serial = SERIAL.lock().unwrap();
        let lock = RwLockWord::new();
        assert!(lock.try_read());
        assert!(lock.try_read());
        assert!(!lock.try_write());
        unsafe { lock.read_unlock::<HostFutex>() };
        unsafe { lock.read_unlock::<HostFutex>() };
        assert!(lock.try_write());
        assert!(lock.is_write_locked());
        assert!(!lock.try_read());
        unsafe { lock.write_unlock::<HostFutex>() };
        assert_eq!(lock.state_bits(), 0);
    }

    #[test]
    fn a_waiting_writer_gets_the_lock_from_the_last_reader_and_new_readers_wait_for_it() {
        let _serial = SERIAL.lock().unwrap();
        SPURIOUS.store(0, SeqCst);
        let lock = Arc::new(RwLockWord::new());
        lock.read::<HostFutex>();
        let wrote = Arc::new(AtomicBool::new(false));
        let writer = {
            let (lock, wrote) = (lock.clone(), wrote.clone());
            thread::spawn(move || {
                lock.write::<HostFutex>();
                wrote.store(true, SeqCst);
                thread::sleep(Duration::from_millis(20));
                unsafe { lock.write_unlock::<HostFutex>() };
            })
        };
        while !has_writers_waiting(lock.state_bits()) {
            thread::yield_now();
        }
        // Writers first: a new reader may not join while the writer waits.
        assert!(!lock.try_read());
        // The last reader's unlock hands the lock to the sleeping writer.
        unsafe { lock.read_unlock::<HostFutex>() };
        writer.join().unwrap();
        assert!(wrote.load(SeqCst));
        lock.read::<HostFutex>();
        unsafe { lock.read_unlock::<HostFutex>() };
        assert_eq!(lock.state_bits(), 0);
    }
}
