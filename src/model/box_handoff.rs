//! [`BoxHandoff`]: one producer thread hands heap values (`Box<T>`, or a clear) to a consumer that installs them into storage it
//! owns, and gets the replaced values back to drop; neither side ever waits, and the consumer never allocates or frees (the lock-free
//! runtime, `artifacts/phase23/TASK-OPUS-rc6-lockfree.md`: the menu worker builds a drilled sequence's plan, the game frame reads it).
//!
//! - **Post** (the producer): first drop what the consumer retired ([`reap`](BoxHandoff::reap)), then swap the new value into
//!   `posted`; a value still posted there was never installed, so the producer drops it. The newest post always wins.
//! - **Install** (the consumer, one at a time: the caller serialises it under the lock that guards its storage): swap `posted` out,
//!   replace the storage's value with it, and store the replaced value in a free retire slot. Only the consumer fills a retire slot
//!   and only the producer empties one, so a slot seen empty stays empty until the consumer fills it.
//! - **Two retire slots.** Each post reaps both before it swaps, so from the start of one reap to the start of the next the
//!   consumer installs at most two values (the one still posted when the reap began, and the one its post swaps in). An install
//!   finds both slots full only while a post's reap is running; it then installs nothing and is counted
//!   ([`take_stalls`](BoxHandoff::take_stalls)), and its next call takes that post's value instead. The consumer never drops a
//!   value itself.
//! - `T` is not zero-sized: the clear marker is `NonNull::dangling`, an address no allocation of a sized `T` has.
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering::{AcqRel, Acquire, Relaxed, Release}};

pub struct BoxHandoff<T> {
    /// Null: nothing posted; [`CLEAR`](Self::clear_marker): a clear posted; otherwise a `Box<T>` the producer leaked.
    posted: AtomicPtr<T>,
    retired: [AtomicPtr<T>; 2],
    stalls: AtomicU32,
}

// SAFETY: a `Box<T>` moves between the two threads through the atomic pointers; each pointer has one owner at a time (the posted
// slot until one swap takes it out, a retire slot until the producer's swap empties it).
unsafe impl<T: Send> Sync for BoxHandoff<T> {}
unsafe impl<T: Send> Send for BoxHandoff<T> {}

impl<T> Default for BoxHandoff<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> BoxHandoff<T> {
    pub const fn new() -> Self {
        Self {
            posted: AtomicPtr::new(core::ptr::null_mut()),
            retired: [AtomicPtr::new(core::ptr::null_mut()), AtomicPtr::new(core::ptr::null_mut())],
            stalls: AtomicU32::new(0),
        }
    }

    /// The posted value "clear": an address no allocation of `T` has (aligned, never dereferenced).
    fn clear_marker() -> *mut T {
        const { assert!(core::mem::size_of::<T>() != 0, "BoxHandoff needs a sized T") };
        core::ptr::NonNull::<T>::dangling().as_ptr()
    }

    /// Drop a pointer this side owns (null and the clear marker own nothing).
    fn free(&self, pointer: *mut T) {
        if !pointer.is_null() && pointer != Self::clear_marker() {
            // SAFETY: a non-marker pointer came from `Box::into_raw`, and the swap that returned it made this side its only owner.
            drop(unsafe { Box::from_raw(pointer) });
        }
    }

    /// The producer: drop what the consumer retired. Never waits.
    pub fn reap(&self) {
        for slot in &self.retired {
            self.free(slot.swap(core::ptr::null_mut(), AcqRel));
        }
    }

    /// The producer: hand over `value` (`None` clears the consumer's storage). Never waits.
    pub fn post(&self, value: Option<Box<T>>) {
        self.reap();
        let new = value.map_or(Self::clear_marker(), Box::into_raw);
        self.free(self.posted.swap(new, AcqRel));
    }

    /// The consumer: is a value posted that [`install`](Self::install) would take?
    pub fn pending(&self) -> bool {
        !self.posted.load(Acquire).is_null()
    }

    /// The consumer (one at a time): install the posted value, if any, into `current`; the replaced value goes back to the producer.
    /// Returns whether `current` changed. Never waits, allocates or frees.
    pub fn install(&self, current: &mut Option<Box<T>>) -> bool {
        if !self.pending() {
            return false;
        }
        let slot = match current {
            Some(_) => match self.retired.iter().find(|slot| slot.load(Acquire).is_null()) {
                Some(slot) => Some(slot),
                None => {
                    self.stalls.fetch_add(1, Relaxed);
                    return false;
                }
            },
            None => None,
        };
        let taken = self.posted.swap(core::ptr::null_mut(), AcqRel);
        let new = if taken == Self::clear_marker() {
            None
        } else {
            // SAFETY: not null (only this side empties `posted`, and it saw it full), not the marker: a `Box::into_raw` pointer
            // the swap made this side's own.
            Some(unsafe { Box::from_raw(taken) })
        };
        if let (Some(old), Some(slot)) = (core::mem::replace(current, new), slot) {
            slot.store(Box::into_raw(old), Release);
        }
        true
    }

    /// Installs that found no free retire slot since the last call.
    pub fn take_stalls(&self) -> u32 {
        self.stalls.swap(0, Relaxed)
    }
}

impl<T> Drop for BoxHandoff<T> {
    fn drop(&mut self) {
        self.reap();
        self.free(self.posted.swap(core::ptr::null_mut(), AcqRel));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
    use std::sync::Arc;

    /// A value that counts its drops and records the thread that dropped it.
    struct Counted {
        id: u64,
        drops: Arc<AtomicUsize>,
        consumer_dropped: Arc<AtomicBool>,
        consumer: Arc<ConsumerThread>,
    }

    /// The consumer thread's id, for the drop check.
    struct ConsumerThread(std::sync::Mutex<Option<std::thread::ThreadId>>);

    impl Drop for Counted {
        fn drop(&mut self) {
            self.drops.fetch_add(1, SeqCst);
            if *self.consumer.0.lock().unwrap() == Some(std::thread::current().id()) {
                self.consumer_dropped.store(true, SeqCst);
            }
        }
    }

    #[test]
    fn the_newest_post_is_installed_and_every_value_is_dropped_by_the_producer_once() {
        let handoff: BoxHandoff<u64> = BoxHandoff::new();
        let mut current = None;
        assert!(!handoff.install(&mut current));
        handoff.post(Some(Box::new(1)));
        handoff.post(Some(Box::new(2)));
        assert!(handoff.install(&mut current));
        assert_eq!(current.as_deref(), Some(&2), "the newest post wins");
        handoff.post(None);
        assert!(handoff.install(&mut current));
        assert_eq!(current, None, "a clear installs None");
        assert!(!handoff.install(&mut current), "nothing left posted");
        handoff.post(Some(Box::new(3)));
        assert!(handoff.install(&mut current));
        handoff.post(Some(Box::new(4)));
        assert!(handoff.install(&mut current));
        handoff.post(Some(Box::new(5)));
        assert!(handoff.install(&mut current));
        assert_eq!(current.as_deref(), Some(&5));
        assert_eq!(handoff.take_stalls(), 0);
    }

    /// Without a reap the consumer fills both retire slots and then waits (counted), and never drops a value itself.
    #[test]
    fn a_full_retire_stalls_the_install_and_never_drops_on_the_consumer() {
        let handoff: BoxHandoff<u64> = BoxHandoff::new();
        let mut current = Some(Box::new(0));
        for n in 1..=2 {
            handoff.posted.store(Box::into_raw(Box::new(n)), SeqCst);
            assert!(handoff.install(&mut current));
        }
        handoff.posted.store(Box::into_raw(Box::new(3)), SeqCst);
        assert!(!handoff.install(&mut current), "both retire slots full");
        assert_eq!(handoff.take_stalls(), 1);
        assert_eq!(current.as_deref(), Some(&2));
        handoff.reap();
        assert!(handoff.install(&mut current));
        assert_eq!(current.as_deref(), Some(&3));
    }

    /// A producer posting as fast as it can against a consumer installing as fast as it can: the consumer sees increasing ids
    /// only, ends on the last post, never drops a value, and every value but the installed one is dropped exactly once.
    #[test]
    fn a_racing_producer_and_consumer_never_lose_double_drop_or_drop_on_the_consumer() {
        const TOTAL: u64 = 50_000;
        let handoff: Arc<BoxHandoff<Counted>> = Arc::new(BoxHandoff::new());
        let drops = Arc::new(AtomicUsize::new(0));
        let consumer_dropped = Arc::new(AtomicBool::new(false));
        let consumer_id = Arc::new(ConsumerThread(std::sync::Mutex::new(None)));
        let done = Arc::new(AtomicBool::new(false));
        let consumer = {
            let (handoff, done, consumer_id) = (Arc::clone(&handoff), Arc::clone(&done), Arc::clone(&consumer_id));
            std::thread::spawn(move || {
                *consumer_id.0.lock().unwrap() = Some(std::thread::current().id());
                let mut current: Option<Box<Counted>> = None;
                let mut last = 0u64;
                loop {
                    let finished = done.load(SeqCst);
                    if handoff.install(&mut current) {
                        let id = current.as_ref().map_or(0, |c| c.id);
                        assert!(id > last, "out of order: {id} after {last}");
                        last = id;
                    } else if finished && !handoff.pending() {
                        break;
                    }
                }
                (last, current)
            })
        };
        for id in 1..=TOTAL {
            handoff.post(Some(Box::new(Counted {
                id,
                drops: Arc::clone(&drops),
                consumer_dropped: Arc::clone(&consumer_dropped),
                consumer: Arc::clone(&consumer_id),
            })));
        }
        done.store(true, SeqCst);
        let (last, current) = consumer.join().unwrap();
        assert_eq!(last, TOTAL, "the last post is installed");
        assert!(!consumer_dropped.load(SeqCst), "the consumer dropped a value");
        handoff.reap();
        assert_eq!(drops.load(SeqCst), (TOTAL - 1) as usize, "every value but the installed one dropped once");
        drop(current);
        assert_eq!(drops.load(SeqCst), TOTAL as usize);
    }
}
