//! The input callers' one pending mapping and controller, stored as four scalar
//! native TLS values. This is deliberately NOT a stack: a second note replaces
//! the first, exactly as the former per-thread Cells did (lead 2026-11-02).
//! The delay queue/reset/owner state is separate in InputAdapter.
//
//! Reserve on the boot thread before installing any caller hook. Keys remain
//! reserved for the plugin lifetime; no thread pointers or heap boxes are kept.

pub use super::scalar_tls::Tls;
use super::scalar_tls::Slots;

/// One key for each word of (out, slot, holder), plus the separate controller.
/// Private keys cannot be observed until all four allocations succeed.
pub struct Pending {
    slots: Slots<4>,
}

impl Pending {
    pub fn reserve<T: Tls>(tls: &T) -> Result<Self, T::Error> {
        Ok(Self { slots: Slots::reserve(tls)? })
    }

    /// Boot rollback only, before any hook can use this set.
    pub fn release(self, tls: &impl Tls) { self.slots.release(tls); }

    pub fn note(&self, tls: &impl Tls, out: usize, slot: i64, holder: usize) {
        for (index, value) in [out, slot as usize, holder].into_iter().enumerate() {
            self.slots.set(tls, index, value);
        }
    }

    /// An unnoted/no-call pass returns zeros. Taking the tuple does not clear
    /// the controller; early-return and nested-helper behavior stays unchanged.
    pub fn take(&self, tls: &impl Tls) -> (usize, i64, usize) {
        let values = [0, 1, 2].map(|index| self.slots.get(tls, index));
        for index in 0..3 { self.slots.set(tls, index, 0); }
        (values[0], values[1] as i64, values[2])
    }

    pub fn set_controller(&self, tls: &impl Tls, controller: usize) {
        self.slots.set(tls, 3, controller);
    }

    pub fn controller(&self, tls: &impl Tls) -> usize {
        self.slots.get(tls, 3)
    }

    pub fn take_controller(&self, tls: &impl Tls) -> usize {
        let controller = self.controller(tls);
        self.set_controller(tls, 0);
        controller
    }
}
