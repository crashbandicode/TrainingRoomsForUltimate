//! Boot-reserved one-shot replies. A slot becomes reusable only after BOTH the
//! request job and its ticket return their channel endpoints. Neither side waits.
use std::cell::UnsafeCell;
use std::sync::{atomic::{AtomicU8, Ordering}, mpsc::{self, Receiver, SyncSender, TryRecvError}};

const READER: u8 = 1;
const WRITER: u8 = 2;
struct Slot<T> {
    owners: AtomicU8,
    receiver: UnsafeCell<Option<Receiver<Result<T, ()>>>>,
    sender: UnsafeCell<Option<SyncSender<Result<T, ()>>>>,
}
// SAFETY: claim changes 0 to READER|WRITER before taking either endpoint.
// Each endpoint then has exactly one owner, which writes only its own cell
// before clearing its bit with Release. A new successful Acquire claim can
// occur only after both bits cleared, and observes both endpoint returns via
// the atomic's release sequence. No endpoint cell is read while owned.
unsafe impl<T: Send> Sync for Slot<T> {}

pub struct Pool<T> { slots: Vec<Slot<T>> }
pub struct Ticket<'a, T> {
    slot: &'a Slot<T>,
    receiver: Option<Receiver<Result<T, ()>>>,
}
pub struct Job<'a, T> {
    slot: &'a Slot<T>,
    sender: Option<SyncSender<Result<T, ()>>>,
    completed: bool,
}
impl<T> Pool<T> {
    /// Boot only. Each channel has room for exactly one completion.
    pub fn new(capacity: usize) -> Self {
        let slots = (0..capacity).map(|_| {
            let (sender, receiver) = mpsc::sync_channel(1);
            Slot { owners: AtomicU8::new(0), receiver: UnsafeCell::new(Some(receiver)),
                sender: UnsafeCell::new(Some(sender)) }
        }).collect();
        Self { slots }
    }
    /// Multiple producers may claim; the scan is bounded by boot capacity.
    pub fn claim(&self) -> Option<(Ticket<'_, T>, Job<'_, T>)> {
        for slot in &self.slots {
            if slot.owners.compare_exchange(0, READER | WRITER, Ordering::Acquire, Ordering::Relaxed).is_err() { continue; }
            // SAFETY: this successful claim exclusively owns both endpoint cells.
            let (receiver, sender) = unsafe {
                ((*slot.receiver.get()).take().unwrap(), (*slot.sender.get()).take().unwrap())
            };
            // The previous ticket may have retired without consuming its reply.
            // Its job is finished before this slot can be claimed, so at most one
            // old completion is present and no old sender can race this drain.
            let _ = receiver.try_recv();
            return Some((Ticket { slot, receiver: Some(receiver) },
                Job { slot, sender: Some(sender), completed: false }));
        }
        None
    }
}
impl<T> Ticket<'_, T> {
    pub fn try_recv(&self) -> Result<Result<T, ()>, TryRecvError> {
        self.receiver.as_ref().unwrap().try_recv()
    }
}
impl<T> Job<'_, T> {
    pub fn complete(mut self, value: T) {
        let _ = self.sender.as_ref().unwrap().try_send(Ok(value));
        self.completed = true;
    }
}
impl<T> Drop for Ticket<'_, T> {
    fn drop(&mut self) {
        // SAFETY: only this ticket owns the receiver cell while READER is set.
        unsafe { *self.slot.receiver.get() = self.receiver.take(); }
        self.slot.owners.fetch_and(!READER, Ordering::Release);
    }
}
impl<T> Drop for Job<'_, T> {
    fn drop(&mut self) {
        if !self.completed { let _ = self.sender.as_ref().unwrap().try_send(Err(())); }
        // SAFETY: only this job owns the sender cell while WRITER is set.
        unsafe { *self.slot.sender.get() = self.sender.take(); }
        self.slot.owners.fetch_and(!WRITER, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn abandoned_reader_does_not_recycle_live_writer_or_leak_old_completion() {
        let pool = Pool::new(1);
        let (ticket, job) = pool.claim().unwrap();
        drop(ticket);
        assert!(pool.claim().is_none());
        job.complete(10);
        let (ticket, job) = pool.claim().unwrap();
        assert_eq!(ticket.try_recv(), Err(TryRecvError::Empty));
        job.complete(20);
        assert!(pool.claim().is_none());
        assert_eq!(ticket.try_recv(), Ok(Ok(20)));
        drop(ticket);
        assert!(pool.claim().is_some());
    }
    #[test]
    fn dropped_job_reports_failure_and_receiver_must_retire_before_reuse() {
        let pool = Pool::<u32>::new(1);
        let (ticket, job) = pool.claim().unwrap();
        drop(job);
        assert_eq!(ticket.try_recv(), Ok(Err(())));
        assert!(pool.claim().is_none());
        drop(ticket);
        let (ticket, job) = pool.claim().unwrap();
        job.complete(7);
        assert_eq!(ticket.try_recv(), Ok(Ok(7)));
    }
    #[test]
    fn concurrent_claim_return_and_cross_thread_completion_keep_values_distinct() {
        let pool = Pool::new(4);
        std::thread::scope(|scope| {
            for producer in 0..4u64 {
                let pool = &pool;
                scope.spawn(move || {
                    for n in 0..200 {
                        let (ticket, job) = loop {
                            if let Some(pair) = pool.claim() { break pair; }
                            std::thread::yield_now();
                        };
                        let expected = producer * 1000 + n;
                        std::thread::scope(|scope| { scope.spawn(move || job.complete(expected)); });
                        assert_eq!(ticket.try_recv(), Ok(Ok(expected)));
                    }
                });
            }
        });
    }
}