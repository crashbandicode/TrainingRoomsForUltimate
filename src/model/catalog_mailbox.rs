//! Bounded ownership transfer from native publication to the UI worker.
//! Endpoints are allocated at boot. The producer never borrows UI-owned state.
// The native side posts: a `nowait_channel` never waits on the worker mid-read (rc.8 hang).
use super::nowait_channel::{self as mpsc, Receiver, SyncSender};

pub struct Publication<T> {
    pub sequence: u32,
    pub epoch: u64,
    pub value: T,
}
pub struct Producer<T>(SyncSender<Publication<T>>);
pub struct Consumer<T> {
    receiver: Receiver<Publication<T>>,
    capacity: usize,
}
pub fn channel<T>(capacity: usize) -> (Producer<T>, Consumer<T>) {
    assert!(capacity > 0);
    let (sender, receiver) = mpsc::sync_channel(capacity);
    (Producer(sender), Consumer { receiver, capacity })
}
impl<T> Producer<T> {
    /// Full or disconnected retains the exact owned value for the native policy.
    pub fn post(&self, publication: Publication<T>) -> Result<(), Publication<T>> {
        self.0.try_send(publication).map_err(|error| match error {
            mpsc::TrySendError::Full(value) | mpsc::TrySendError::Disconnected(value) => value,
        })
    }
}
impl<T> Consumer<T> {
    /// UI worker only. At most capacity values are consumed, even if a producer
    /// posts concurrently. Both opening sequence and captured epoch must match.
    pub fn take(&mut self, sequence: u32, epoch: u64, active: bool) -> Option<T> {
        let mut latest = None;
        for _ in 0..self.capacity {
            let Ok(publication) = self.receiver.try_recv() else { break; };
            if active && publication.sequence == sequence && publication.epoch == epoch {
                latest = Some(publication.value);
            }
        }
        latest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    fn value(sequence: u32, epoch: u64, n: u32) -> Publication<Arc<u32>> {
        Publication { sequence, epoch, value: Arc::new(n) }
    }
    #[test]
    fn full_returns_exact_identity_and_retries_after_worker_drains() {
        let (send, mut receive) = channel(1);
        assert!(send.post(value(2, 8, 10)).is_ok());
        let next = value(2, 8, 20);
        let identity = Arc::clone(&next.value);
        let next = send.post(next).err().unwrap();
        assert!(Arc::ptr_eq(&identity, &next.value));
        assert_eq!(*receive.take(2, 8, true).unwrap(), 10);
        assert!(send.post(next).is_ok());
        assert!(Arc::ptr_eq(&identity, &receive.take(2, 8, true).unwrap()));
        assert!(receive.take(2, 8, true).is_none());
    }
    #[test]
    fn close_reopen_and_retired_epoch_cannot_mount_old_publication() {
        let (send, mut receive) = channel(2);
        assert!(send.post(value(2, 8, 10)).is_ok());
        assert!(receive.take(4, 8, true).is_none()); // same epoch observed before native catches up
        assert!(send.post(value(4, 8, 20)).is_ok());
        assert!(receive.take(4, 9, true).is_none());
        assert!(send.post(value(4, 9, 30)).is_ok());
        assert!(receive.take(4, 9, false).is_none());
        assert!(send.post(value(6, 10, 40)).is_ok());
        assert_eq!(*receive.take(6, 10, true).unwrap(), 40);
    }
    #[test]
    fn newest_current_value_wins_without_stale_value_overwriting_it() {
        let (send, mut receive) = channel(3);
        assert!(send.post(value(4, 9, 10)).is_ok());
        assert!(send.post(value(4, 9, 20)).is_ok());
        assert!(send.post(value(2, 8, 99)).is_ok());
        assert_eq!(*receive.take(4, 9, true).unwrap(), 20);
    }
    #[test]
    fn disconnected_consumer_returns_ownership() {
        let (send, receive) = channel(1);
        drop(receive);
        let returned = send.post(value(2, 8, 10)).err().unwrap();
        assert_eq!(*returned.value, 10);
    }
}