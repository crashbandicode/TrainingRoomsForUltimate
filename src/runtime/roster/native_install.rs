//! Serialize asynchronous native fighter installs without treating resource-only
//! loading as a suspension of the visible practice fighters.
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstallToken(u64);

pub struct InstallGate {
    active: AtomicU64,
    next: AtomicU64,
}

impl InstallGate {
    pub const fn new() -> Self {
        Self { active: AtomicU64::new(0), next: AtomicU64::new(2) }
    }

    pub fn active(&self) -> bool {
        self.active.load(Ordering::Acquire) != 0
    }

    #[cfg(any(test, feature = "mii-ordinary-player-alpha"))]
    pub fn owns(&self, token: InstallToken) -> bool {
        self.active.load(Ordering::Acquire) == token.0
    }

    pub fn suspends_practice(&self) -> bool {
        self.active.load(Ordering::Acquire) & 1 != 0
    }

    /// The caller retains the token until native completion or destruction.
    /// Cancelling plugin intent or leaving a scene does not end a native install.
    pub fn try_begin(&self, suspend_practice: bool) -> Option<InstallToken> {
        if self.active() { return None; }
        let mut generation = self.next.fetch_add(2, Ordering::Relaxed);
        // Zero is reserved for idle, including at integer wraparound.
        if generation == 0 { generation = self.next.fetch_add(2, Ordering::Relaxed); }
        let token = generation | u64::from(suspend_practice);
        self.active.compare_exchange(0, token, Ordering::AcqRel, Ordering::Acquire)
            .ok().map(|_| InstallToken(token))
    }

    /// A delayed completion cannot unlock a later install, even at a reused
    /// native entry address. No native pointer participates in this token.
    pub fn finish(&self, token: InstallToken) -> bool {
        self.active.compare_exchange(token.0, 0, Ordering::AcqRel, Ordering::Acquire).is_ok()
    }
}

