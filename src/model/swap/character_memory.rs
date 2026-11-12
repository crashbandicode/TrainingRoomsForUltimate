//! Value-only waiting-practice choices, independent of native entry lifetimes.
//! The caller owns lobby boundaries and validates a loaded fighter before commit.

#[derive(Clone, Copy, Debug)]
pub struct WaitingChoice<T> {
    remembered: Option<T>,
    restoring: bool,
}

impl<T: Copy> WaitingChoice<T> {
    pub const fn empty() -> Self {
        Self {
            remembered: None,
            restoring: false,
        }
    }

    /// A fresh lobby has no practice override from its predecessor.
    pub fn clear(&mut self) {
        *self = Self::empty();
    }

    /// Native entries may disappear for a match; only the fighter value survives.
    pub fn scene_changed(&mut self) {
        self.restoring = self.remembered.is_some();
    }

    /// The first ready native body supplies the default for a new lobby.
    /// Observing the CSS body on return must not replace a remembered choice.
    pub fn observe(&mut self, fighter: T) {
        if self.remembered.is_none() {
            self.commit(fighter);
        }
    }

    /// A queued or interrupted replacement is not a completed choice.
    pub fn commit(&mut self, fighter: T) {
        self.remembered = Some(fighter);
        self.restoring = false;
    }

    /// A new explicit request supersedes automatic restoration. Until it actually
    /// completes, a later scene transition still remembers the previous body.
    pub fn manual_request(&mut self) {
        self.restoring = false;
    }

    /// Peeking does not consume restoration while a menu/native action is busy.
    pub fn restore_target(&self) -> Option<T> {
        self.remembered.filter(|_| self.restoring)
    }
}

/// A waiting player's entry can be destroyed before either a match or a room exit.
/// Defer that distinction: a match changes the scene; recreating an LDN room can
/// create new entries with the exact same scene tuple (13.0.5 runtime trace).
pub struct WaitingSession<T> {
    choices: [WaitingChoice<T>; 2],
    scene: Option<(bool, u8, u32)>,
    player_departed: bool,
}

impl<T: Copy> WaitingSession<T> {
    pub const fn empty() -> Self {
        Self {
            choices: [WaitingChoice::empty(); 2],
            scene: None,
            player_departed: false,
        }
    }

    /// Returns the shared continuation decision for other value-only practice state.
    ///
    /// Match excursions retain value-only selections through the shared backend
    /// continuation policy. This never permits practice writes during the match.
    pub fn scene_changed(&mut self, initialized: bool, melee: u8, mode: u32) -> bool {
        let lobby = crate::engine::mii::backend::Layout::continuation;
        let current = lobby(initialized, melee, mode);
        let same_lobby = current.is_some() && self.scene
            .and_then(|(initialized, melee, mode)| lobby(initialized, melee, mode)) == current;
        for choice in &mut self.choices {
            if same_lobby {
                choice.scene_changed();
            } else {
                choice.clear();
            }
        }
        self.scene = Some((initialized, melee, mode));
        self.player_departed = false;
        same_lobby
    }

    /// Caller proves destruction of its current waiting P1 entry, not a body reload,
    /// preview, CPU partner or actual-match entry. Never retain the entry pointer.
    pub fn player_departed(&mut self) {
        self.player_departed = true;
    }

    /// Called for a newly observed, ready waiting body. Returns whether this is a
    /// fresh entry lifetime without a match excursion, which forgets BOTH roles.
    pub fn observe(&mut self, role: usize, fighter: T) -> bool {
        let fresh = self.player_departed;
        if fresh {
            for choice in &mut self.choices {
                choice.clear();
            }
            self.player_departed = false;
        }
        self.choices[role].observe(fighter);
        fresh
    }

    pub fn commit(&mut self, role: usize, fighter: T) {
        self.choices[role].commit(fighter);
    }

    pub fn manual_request(&mut self, role: usize) {
        self.choices[role].manual_request();
    }

    pub fn restore_target(&self, role: usize) -> Option<T> {
        self.choices[role].restore_target()
    }

    /// Disabled roster roles do not service their remembered target. Consumers
    /// such as hats must still work on those roles' current native fighters.
    pub fn restoration_pending(&self, enabled: [bool; 2]) -> bool {
        enabled.iter().copied().enumerate()
            .any(|(role, enabled)| enabled && self.restore_target(role).is_some())
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::WaitingSession;

    fn disabled_player_restore_does_not_block_cpu_hat_continuation() {
        let mut session = WaitingSession::empty();
        session.scene_changed(true, 0x0d, 0x02010000);
        session.observe(0, 10);
        session.observe(1, 6);
        session.scene_changed(true, 0x01, 0x02010000);
        session.scene_changed(true, 0x0d, 0x02010000);
        assert!(session.restoration_pending([false, true]));
        session.commit(1, 6);
        assert!(!session.restoration_pending([false, true]));
        assert!(session.restoration_pending([true, true]));
        session.commit(0, 10);
        assert!(!session.restoration_pending([true, true]));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("disabled_player_restore_does_not_block_cpu_hat_continuation", disabled_player_restore_does_not_block_cpu_hat_continuation as fn()),
        ]);
    }
    }
}
