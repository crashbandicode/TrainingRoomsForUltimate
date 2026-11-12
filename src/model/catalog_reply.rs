//! A startup-allocated reply channel whose sole sender travels with a job.
//!
//! Returning the sender alongside the result permits reuse without keeping a
//! spare sender alive while work runs. Dropped jobs report failure explicitly,
//! return the endpoint, and preserve subsequent fallback requests.
// The native side polls the replies: a `nowait_channel` never waits on a worker mid-send (rc.8 hang).
use super::nowait_channel::{self as mpsc, Receiver, SyncSender, TryRecvError};

pub struct Reply<T> {
    pub value: Result<T, ()>,
    pub next: SyncSender<Reply<T>>,
}

pub struct Replies<T> {
    receiver: Receiver<Reply<T>>,
    sender: Option<SyncSender<Reply<T>>>,
}

impl<T> Default for Replies<T> {
    fn default() -> Self { Self::new() }
}

impl<T> Replies<T> {
    /// Startup only: this is the only channel allocation.
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::sync_channel(1);
        Self { receiver, sender: Some(sender) }
    }

    /// Move the only endpoint to the job; no clone remains to hide failure.
    pub fn begin(&mut self) -> Option<SyncSender<Reply<T>>> { self.sender.take() }

    /// Queue-full leaves the endpoint available for the policy's bounded retry.
    pub fn rejected(&mut self, sender: SyncSender<Reply<T>>) {
        assert!(self.sender.is_none());
        self.sender = Some(sender);
    }

    pub fn poll(&mut self) -> Result<Result<T, ()>, TryRecvError> {
        let reply = self.receiver.try_recv()?;
        self.sender = Some(reply.next);
        Ok(reply.value)
    }
}

/// Queued job ownership. If a worker unwinds or drops its queue, the existing
/// request policy receives failure and can reuse the returned endpoint later.
pub struct Job<T>(Option<SyncSender<Reply<T>>>);
impl<T> Job<T> {
    pub fn new(sender: SyncSender<Reply<T>>) -> Self { Self(Some(sender)) }
    pub fn rejected(mut self) -> SyncSender<Reply<T>> { self.0.take().unwrap() }
    pub fn complete(mut self, value: T) { self.send(Ok(value)); }
    fn send(&mut self, value: Result<T, ()>) {
        if let Some(sender) = self.0.take() {
            let next = sender.clone();
            let _ = sender.try_send(Reply { value, next });
        }
    }
}
impl<T> Drop for Job<T> {
    fn drop(&mut self) { self.send(Err(())); }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn repeated_flights_reuse_endpoint_and_deliver_once() {
        let mut replies = Replies::new();
        for value in 0..20 {
            let sender = replies.begin().unwrap();
            assert!(replies.begin().is_none());
            assert!(matches!(replies.poll(), Err(TryRecvError::Empty)));
            Job::new(sender).complete(value);
            assert_eq!(replies.poll(), Ok(Ok(value)));
            assert!(matches!(replies.poll(), Err(TryRecvError::Empty)));
        }
    }

    fn lost_job_reports_failure_and_allows_later_fallback() {
        let mut replies = Replies::<u32>::new();
        drop(Job::new(replies.begin().unwrap()));
        assert_eq!(replies.poll(), Ok(Err(())));
        Job::new(replies.begin().unwrap()).complete(9);
        assert_eq!(replies.poll(), Ok(Ok(9)));
    }

    fn full_queue_returns_endpoint_before_retry() {
        let mut replies = Replies::new();
        let sender = replies.begin().unwrap();
        replies.rejected(Job::new(sender).rejected());
        Job::new(replies.begin().unwrap()).complete(7);
        assert_eq!(replies.poll(), Ok(Ok(7)));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("repeated_flights_reuse_endpoint_and_deliver_once", repeated_flights_reuse_endpoint_and_deliver_once as fn()),
            ("lost_job_reports_failure_and_allows_later_fallback", lost_job_reports_failure_and_allows_later_fallback as fn()),
            ("full_queue_returns_endpoint_before_retry", full_queue_returns_endpoint_before_retry as fn()),
        ]);
    }
    }
}
