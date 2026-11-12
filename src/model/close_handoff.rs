//! Boot-allocated close intent transport. A current opening retains its owned
//! intent until close; a producer that observes Full retains ownership to retry.
//! The native side polls it every pre-service tick, so it is a `nowait_channel`:
//! std's `try_recv` spun on the menu worker's in-flight `try_send` (rc.8 hang).
use super::nowait_channel::{self as mpsc, Receiver, SyncSender};

pub struct Intent<T> {
    pub opening: u32,
    pub epoch: u64,
    pub value: T,
}
pub struct Producer<T>(SyncSender<Intent<T>>);
pub struct Consumer<T> {
    receiver: Receiver<Intent<T>>,
    held: Option<Intent<T>>,
    capacity: usize,
}
pub fn channel<T>(capacity: usize) -> (Producer<T>, Consumer<T>) {
    assert!(capacity > 0);
    let (sender, receiver) = mpsc::sync_channel(capacity);
    (Producer(sender), Consumer { receiver, held: None, capacity })
}
impl<T> Producer<T> {
    pub fn post(&self, intent: Intent<T>) -> Result<(), Intent<T>> {
        self.0.try_send(intent).map_err(|error| match error {
            mpsc::TrySendError::Full(value) | mpsc::TrySendError::Disconnected(value) => value,
        })
    }
}
impl<T> Consumer<T> {
    /// Native only. Inspect at most the held value plus the channel capacity.
    /// Sequence numbers advance at each open/close. As with the menu sequence,
    /// fewer than half the u32 sequence space may elapse while an intent lives.
    /// A future opening is retained, including when UI advances during a sample.
    pub fn observe(&mut self, sequence: u32, epoch: u64, open: bool) -> Option<T> {
        let opening = if open { sequence } else { sequence.wrapping_sub(1) };
        for _ in 0..=self.capacity {
            if self.held.is_none() {
                self.held = self.receiver.try_recv().ok();
            }
            let held = self.held.as_ref()?;
            let age = opening.wrapping_sub(held.opening);
            if age >= (1 << 31) { return None; } // not yet observed opening
            if age != 0 || held.epoch != epoch {
                self.held = None; // proven retired opening or its retired epoch
                continue;
            }
            if open { return None; }
            return self.held.take().map(|intent| intent.value);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use std::sync::Arc;
    fn intent(opening: u32, epoch: u64, value: Arc<u8>) -> Intent<Arc<u8>> {
        Intent { opening, epoch, value }
    }
    fn full_returns_identity_and_current_is_held_until_close() {
        let (send, mut receive) = channel(1);
        let value = Arc::new(7);
        assert!(send.post(intent(1, 8, Arc::clone(&value))).is_ok());
        let retry = send.post(intent(3, 9, Arc::clone(&value))).err().unwrap();
        assert!(Arc::ptr_eq(&retry.value, &value));
        assert!(receive.observe(1, 8, true).is_none());
        assert!(send.post(retry).is_ok());
        assert!(Arc::ptr_eq(&receive.observe(2, 8, false).unwrap(), &value));
        assert!(receive.observe(2, 8, false).is_none()); // future retained
        assert!(receive.observe(3, 9, true).is_none());
        assert!(Arc::ptr_eq(&receive.observe(4, 9, false).unwrap(), &value));
        assert!(receive.observe(4, 9, false).is_none());
    }
    fn coalesced_reopen_and_epoch_retirement_cancel_only_stale_intents() {
        let (send, mut receive) = channel(2);
        assert!(send.post(intent(1, 8, Arc::new(1))).is_ok());
        assert!(send.post(intent(3, 9, Arc::new(2))).is_ok());
        assert!(receive.observe(3, 9, true).is_none());
        assert_eq!(*receive.observe(4, 9, false).unwrap(), 2);
        assert!(send.post(intent(5, 9, Arc::new(3))).is_ok());
        assert!(receive.observe(6, 10, false).is_none());
    }
    fn wrap_and_disconnection_preserve_ownership() {
        let (send, mut receive) = channel(1);
        assert!(send.post(intent(u32::MAX, 8, Arc::new(4))).is_ok());
        assert_eq!(*receive.observe(0, 8, false).unwrap(), 4);
        drop(receive);
        let value = Arc::new(5);
        let returned = send.post(intent(1, 9, Arc::clone(&value))).err().unwrap();
        assert!(Arc::ptr_eq(&returned.value, &value));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("full_returns_identity_and_current_is_held_until_close", full_returns_identity_and_current_is_held_until_close as fn()),
            ("coalesced_reopen_and_epoch_retirement_cancel_only_stale_intents", coalesced_reopen_and_epoch_retirement_cancel_only_stale_intents as fn()),
            ("wrap_and_disconnection_preserve_ownership", wrap_and_disconnection_preserve_ownership as fn()),
        ]);
    }
    }
}
