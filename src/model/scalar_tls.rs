//! Boot-reserved scalar TLS. No per-thread boxes, thread registry, or lazy initialization.

/// The SDK boundary. Successful keys start at zero on existing and future
/// threads. Reads/writes address the current thread and cannot allocate.
pub trait Tls {
    type Error;
    fn allocate(&self) -> Result<u32, Self::Error>;
    fn free(&self, key: u32);
    fn get(&self, key: u32) -> usize;
    fn set(&self, key: u32, value: usize);
}


/// A complete reservation, unpublished until every key is valid.
pub struct Slots<const N: usize> { keys: [u32; N] }
impl<const N: usize> Slots<N> {
    pub fn reserve<T: Tls>(tls: &T) -> Result<Self, T::Error> {
        let mut keys = [0; N];
        for index in 0..N {
            match tls.allocate() {
                Ok(key) => keys[index] = key,
                Err(error) => {
                    for &key in keys[..index].iter().rev() { tls.free(key); }
                    return Err(error);
                }
            }
        }
        Ok(Self { keys })
    }
    /// Only before publication; published keys live for the process.
    pub fn release(self, tls: &impl Tls) {
        for key in self.keys.into_iter().rev() { tls.free(key); }
    }
    pub fn get(&self, tls: &impl Tls, index: usize) -> usize { tls.get(self.keys[index]) }
    pub fn set(&self, tls: &impl Tls, index: usize, value: usize) { tls.set(self.keys[index], value); }
}

/// Allocation-free bootstrap publication. Reads before successful reservation
/// return unavailable without touching the SDK, including allocator reentry
/// while keys are being reserved. The boot caller must refuse hooks on failure.
pub struct Bank<const N: usize> { slots: std::sync::OnceLock<Slots<N>> }
impl<const N: usize> Default for Bank<N> { fn default() -> Self { Self::new() } }
impl<const N: usize> Bank<N> {
    pub const fn new() -> Self { Self { slots: std::sync::OnceLock::new() } }
    pub fn init<T: Tls>(&self, tls: &T) -> Result<(), T::Error> {
        if self.slots.get().is_some() { return Ok(()); }
        let slots = Slots::reserve(tls)?;
        if let Err(unpublished) = self.slots.set(slots) { unpublished.release(tls); }
        Ok(())
    }
    pub fn cell<'a, T: Tls>(&'a self, tls: &'a T, index: usize, default: usize) -> Option<Scalar<'a, T, N>> {
        self.slots.get().map(|slots| Scalar { slots, tls, index, default })
    }
}

/// Zero-initialized native values encode any scalar default using XOR.
/// In particular CURRENT starts as Other on all existing and future threads.
pub struct Scalar<'a, T, const N: usize> {
    slots: &'a Slots<N>, tls: &'a T, index: usize, default: usize,
}
impl<T: Tls, const N: usize> Scalar<'_, T, N> {
    pub fn get(&self) -> usize { self.slots.get(self.tls, self.index) ^ self.default }
    pub fn set(&self, value: usize) { self.slots.set(self.tls, self.index, value ^ self.default); }
    pub fn replace(&self, value: usize) -> usize {
        let old = self.get();
        self.set(value);
        old
    }
}
