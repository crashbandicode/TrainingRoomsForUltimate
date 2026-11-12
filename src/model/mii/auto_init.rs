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

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    const FIRST: Token = Token {
        generations: [7, 11],
        opening: 13,
    };

    fn readiness_precedes_arming_and_mii_input_never_falls_through_unqualified() {
        let mut init = Initialization::empty();
        assert_eq!(init.observe(FIRST, 0, false), None); // Original/readiness unavailable.
        assert_eq!(init.observe(FIRST, 1, true), None);
        assert_eq!(init.observe(FIRST, 0, false), None); // INSTALL or another action owns practice.
        assert_eq!(init.observe(FIRST, 4, true), None); // Not a visible Training role.
        assert!(!init.finish(FIRST, FIRST, true));
        assert!(requires_owner(Some(0x49), false, None));
        assert!(requires_owner(Some(9), true, None));
        assert!(requires_owner(Some(9), false, Some(0x48)));
        assert!(!requires_owner(Some(9), false, Some(0x0f)));
        assert!(!requires_owner(None, false, None));
        let mixed_player = |native, quantity, automatic, capture| {
            requires_owner(
                Some(9),
                player_qualifying(native, quantity, automatic, capture),
                None,
            )
        };
        assert!(mixed_player(true, Some(1), init.pending(), false));
        assert!(!mixed_player(false, Some(1), init.pending(), false)); // Waiting scope.
        assert!(mixed_player(true, Some(2), init.pending(), false));
        assert!(mixed_player(true, Some(3), init.pending(), false));
        assert!(!mixed_player(true, Some(4), init.pending(), false)); // Outside 1..=3.
        assert!(!mixed_player(true, None, init.pending(), true)); // Unknown native quantity.
        assert_eq!(init.observe(FIRST, 0, true), Some(FIRST));
        assert!(init.finish(FIRST, FIRST, true));
        assert!(mixed_player(true, Some(1), init.pending(), true)); // Catalog done; role capture armed.
        assert!(!mixed_player(true, Some(1), init.pending(), false)); // Refused/completed capture.
        init.reset();
        init.observe(FIRST, 0, true);
        init.observe(FIRST, 1, true);
        assert!(!init.finish(FIRST, FIRST, false)); // Incomplete catalog consumes the attempt.
        assert!(!mixed_player(true, Some(1), init.pending(), false));
        assert!(requires_owner(Some(0x49), false, None)); // Immutable Mii origin remains protected.
        assert!(requires_owner(Some(9), false, Some(0x48))); // Direct Mii remains refused.
    }

    const ORIGIN: u32 = 0x0e;
    const REMEMBERED: u32 = 6;
    const RETURN: Token = Token {
        generations: [18, 18],
        opening: 19,
    };

    type Hold = fn(bool, bool, bool, bool, bool, bool) -> bool;

    /// One waiting lifetime after a match, frame by frame in callback order: auto-init,
    /// the role's capture, then the roster's continuation consult. Capture qualifies
    /// only the CSS-origin body; an unheld continuation replaces it at once. Ready-go is
    /// frame 3; `menu` cancels the automatic read at frame 4 and arms capture itself.
    /// Returns (frame qualified, frame replaced, capture still armed).
    fn after_match(
        hold: Hold,
        complete: bool,
        ready: bool,
        menu: bool,
    ) -> (Option<u32>, Option<u32>, bool) {
        let mut init = Initialization::empty();
        init.reset();
        let (mut body, mut capture, mut reading) = (ORIGIN, false, None);
        let (mut qualified_at, mut replaced_at) = (None, None);
        for frame in 0..12 {
            let go = frame >= 3;
            if menu && frame == 4 {
                init.cancel();
                capture = true;
            }
            if let Some(token) = reading.take() {
                capture |= init.finish(token, RETURN, complete);
            } else if go && ready {
                init.observe(RETURN, 0, true);
                reading = init.observe(RETURN, 1, true);
            }
            if capture && body == ORIGIN {
                capture = false;
                qualified_at = Some(frame);
            }
            let (qualified, origin) = (qualified_at.is_some(), body == ORIGIN);
            let automatic = init.pending();
            let held = hold(qualified, origin, automatic, capture, go, ready);
            if origin && !held {
                body = REMEMBERED;
                replaced_at = Some(frame);
            }
        }
        (qualified_at, replaced_at, capture)
    }

    fn match_return_qualifies_the_css_origin_before_its_continuation() {
        let unheld: Hold = |_, _, _, _, _, _| false;
        // Counterexample (C99 return): the continuation replaces the CSS body first,
        // so the armed capture can never qualify and stays armed.
        let old = after_match(unheld, true, true, false);
        assert_eq!(old, (None, Some(0), true));
        // Held: the delivered read arms capture, the origin qualifies, then the
        // remembered choice continues in the same frame's later consult.
        let held = (Some(4), Some(4), false);
        assert_eq!(after_match(holds_origin_replacement, true, true, false), held);
        // A menu that supersedes the read arms capture itself; same order.
        assert_eq!(after_match(holds_origin_replacement, true, true, true), held);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("readiness_precedes_arming_and_mii_input_never_falls_through_unqualified", readiness_precedes_arming_and_mii_input_never_falls_through_unqualified as fn()),
            ("match_return_qualifies_the_css_origin_before_its_continuation", match_return_qualifies_the_css_origin_before_its_continuation as fn()),
        ]);
    }
    }
}
