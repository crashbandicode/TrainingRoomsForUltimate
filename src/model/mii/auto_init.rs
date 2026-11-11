//! One automatic catalog read per visible-entry lifetime. Values only; no IO or native access.
use crate::engine::mii::inventory::MiiClass;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub generations: [u64; 2],
    pub opening: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Waiting,
    Reading,
    Finished,
}

pub struct Initialization {
    token: Option<Token>,
    ready: u8,
    phase: Phase,
}

impl Initialization {
    pub const fn empty() -> Self {
        Self {
            token: None,
            ready: 0,
            phase: Phase::Waiting,
        }
    }

    pub fn pending(&self) -> bool {
        self.phase != Phase::Finished
    }

    /// `eligible` includes both known originals, the current role's native readiness,
    /// and the existing practice exclusions. A callback supplies only its own role.
    pub fn observe(&mut self, token: Token, role: u32, eligible: bool) -> Option<Token> {
        if role > 1 || self.phase != Phase::Waiting {
            return None;
        }
        if self.token.is_some_and(|previous| previous != token) {
            return None;
        }
        self.token = Some(token);
        let bit = 1 << role;
        if eligible {
            self.ready |= bit;
        } else {
            self.ready &= !bit;
        }
        if self.ready != 3 {
            return None;
        }
        self.phase = Phase::Reading;
        Some(token)
    }

    /// Consume this read exactly once, including incomplete or stale results.
    /// An old completion cannot consume a new lifetime's pending read.
    pub fn finish(&mut self, token: Token, current: Token, complete: bool) -> bool {
        if self.phase != Phase::Reading || self.token != Some(token) {
            return false;
        }
        self.phase = Phase::Finished;
        token == current && complete
    }

    /// An explicit menu refresh supersedes automatic work for this lifetime.
    pub fn cancel(&mut self) {
        self.phase = Phase::Finished;
    }

    /// Scene change and visible-entry destruction both start a fresh lifetime.
    pub fn reset(&mut self) {
        *self = Self::empty();
    }

    pub fn reading(&self) -> Option<Token> {
        if self.phase == Phase::Reading { self.token } else { None }
    }
}

/// A mixed Player roster waits only while its native qualification can still finish.
/// Explicit Mii/original-Mii protection is independent of this temporary wait.
pub fn player_qualifying(
    native_training: bool,
    cpu_quantity: Option<u32>,
    automatic: bool,
    capture: bool,
) -> bool {
    // Same admission as `parked_entries::training_quantity_open`. This file is also compiled by path,
    // where that module is not in scope.
    native_training && matches!(cpu_quantity, Some(1..=3)) && (automatic || capture)
}

/// Until capture succeeds, consume Mii-related input without creating a request.
/// Plain ordinary-only routing remains available while the catalog is loading.
pub fn requires_owner(original: Option<u32>, roster_has_mii: bool, direct: Option<u32>) -> bool {
    roster_has_mii
        || original.and_then(MiiClass::from_kind).is_some()
        || direct.and_then(MiiClass::from_kind).is_some()
}

/// A continuation or an early manual choice replaces the visible CSS-origin body,
/// and origin capture accepts only that body. Hold the replacement while
/// the role has no owner for the current generation, the visible body is still the
/// immutable origin, and qualification can still finish: the automatic read is
/// pending or the role's capture is armed, and the backend is before ready-go or
/// exactly ready (the read's own readiness gate). Otherwise the continuation proceeds
/// unchanged; an owner applies its own rules.
pub fn holds_origin_replacement(
    qualified: bool,
    origin_body: bool,
    automatic: bool,
    capture: bool,
    ready_go: bool,
    backend_ready: bool,
) -> bool {
    !qualified && origin_body && (automatic || capture) && (!ready_go || backend_ready)
}

