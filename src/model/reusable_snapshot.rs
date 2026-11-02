//! One boot-allocated immutable snapshot. Reuse waits for readers by admission,
//! never by blocking, copying the payload, or allocating another backing store.
use std::ops::Deref;
use std::sync::Arc;

pub struct Storage<T> { value: Arc<Option<T>> }
pub struct Snapshot<T> { value: Arc<Option<T>> }
impl<T> Default for Storage<T> { fn default() -> Self { Self::new() } }
impl<T> Storage<T> {
    /// Startup only: reserves the payload and its reference count together.
    pub fn new() -> Self { Self { value: Arc::new(None) } }
    pub fn available(&self) -> bool { Arc::strong_count(&self.value) == 1 }
    /// No allocation. A surviving reader keeps the previous value immutable.
    pub fn publish(&mut self, value: T) -> Result<Snapshot<T>, T> {
        let Some(slot) = Arc::get_mut(&mut self.value) else { return Err(value); };
        *slot = Some(value);
        Ok(Snapshot { value: Arc::clone(&self.value) })
    }
}
impl<T> Clone for Snapshot<T> {
    fn clone(&self) -> Self { Self { value: Arc::clone(&self.value) } }
}
impl<T> Deref for Snapshot<T> {
    type Target = T;
    fn deref(&self) -> &T { self.value.as_ref().as_ref().expect("published snapshot") }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retired_transaction_cannot_overwrite_a_surviving_reader() {
        let mut storage = Storage::new();
        let transaction = storage.publish([7; 128]).unwrap();
        let reader = transaction.clone();
        let address = (&*reader) as *const [i32; 128];
        drop(transaction);
        assert!(!storage.available());
        assert_eq!(storage.publish([9; 128]).err(), Some([9; 128]));
        assert_eq!(*reader, [7; 128]);
        drop(reader);
        assert!(storage.available());
        let next = storage.publish([9; 128]).unwrap();
        assert_eq!((&*next) as *const [i32; 128], address);
        assert_eq!(*next, [9; 128]);
    }
}
