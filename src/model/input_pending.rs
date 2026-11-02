//! The input callers' one pending mapping and controller, stored as four scalar
//! native TLS values. This is deliberately NOT a stack: a second note replaces
//! the first, exactly as the former per-thread Cells did (lead 2026-11-02).
//! The delay queue/reset/owner state is separate in InputAdapter.
//
//! Reserve on the boot thread before installing any caller hook. Keys remain
//! reserved for the plugin lifetime; no thread pointers or heap boxes are kept.

/// The SDK boundary. Successful keys start at zero on existing and future
/// threads. Reads/writes address the current thread and cannot allocate.
pub trait Tls {
    type Error;
    fn allocate(&self) -> Result<u32, Self::Error>;
    fn free(&self, key: u32);
    fn get(&self, key: u32) -> usize;
    fn set(&self, key: u32, value: usize);
}

/// One key for each word of (out, slot, holder), plus the separate controller.
/// Private keys cannot be observed until all four allocations succeed.
pub struct Pending {
    keys: [u32; 4],
}

impl Pending {
    pub fn reserve<T: Tls>(tls: &T) -> Result<Self, T::Error> {
        let mut keys = [0; 4];
        for index in 0..keys.len() {
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

    /// Boot rollback only, before any hook can use this set.
    pub fn release(self, tls: &impl Tls) {
        for key in self.keys.into_iter().rev() { tls.free(key); }
    }

    pub fn note(&self, tls: &impl Tls, out: usize, slot: i64, holder: usize) {
        for (key, value) in self.keys[..3].iter().zip([out, slot as usize, holder]) {
            tls.set(*key, value);
        }
    }

    /// An unnoted/no-call pass returns zeros. Taking the tuple does not clear
    /// the controller; early-return and nested-helper behavior stays unchanged.
    pub fn take(&self, tls: &impl Tls) -> (usize, i64, usize) {
        let values = [self.keys[0], self.keys[1], self.keys[2]].map(|key| tls.get(key));
        for &key in &self.keys[..3] { tls.set(key, 0); }
        (values[0], values[1] as i64, values[2])
    }

    pub fn set_controller(&self, tls: &impl Tls, controller: usize) {
        tls.set(self.keys[3], controller);
    }

    pub fn controller(&self, tls: &impl Tls) -> usize {
        tls.get(self.keys[3])
    }

    pub fn take_controller(&self, tls: &impl Tls) -> usize {
        let controller = self.controller(tls);
        self.set_controller(tls, 0);
        controller
    }
}
