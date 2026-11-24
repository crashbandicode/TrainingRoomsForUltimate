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
/// The scene tuple cannot show a CSS visit (the melee byte keeps its previous value), but a new room's native body can.
pub struct WaitingSession<T> {
    choices: [WaitingChoice<T>; 2],
    /// The native body each role's lobby started from.
    origins: [Option<T>; 2],
    /// The next observation is the role's first in a new waiting room of the same lobby.
    returning: [bool; 2],
    scene: Option<(bool, u8, u32)>,
    player_departed: bool,
}

impl<T: Copy + PartialEq> WaitingSession<T> {
    pub const fn empty() -> Self {
        Self {
            choices: [WaitingChoice::empty(); 2],
            origins: [None; 2],
            returning: [false; 2],
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
        for role in 0..self.choices.len() {
            if same_lobby {
                self.choices[role].scene_changed();
                self.returning[role] = self.origins[role].is_some();
            } else {
                self.choices[role].clear();
                self.origins[role] = None;
                self.returning[role] = false;
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

    fn forget(&mut self) {
        for choice in &mut self.choices {
            choice.clear();
        }
        self.origins = [None; 2];
        self.returning = [false; 2];
        self.player_departed = false;
    }

    /// Returns whether this is a fresh lobby: a departed waiting entry or a returning
    /// native body different from the lobby's origin, which only a new CSS pick makes.
    /// Either case forgets BOTH roles.
    pub fn observe(&mut self, role: usize, fighter: T) -> bool {
        let returned = core::mem::take(&mut self.returning[role]);
        let fresh = self.player_departed || (returned && self.origins[role] != Some(fighter));
        if fresh { self.forget(); }
        if self.origins[role].is_none() { self.origins[role] = Some(fighter); }
        self.choices[role].observe(fighter);
        fresh
    }

    /// Check a returning body before its exact value is known; a mismatch forgets both roles.
    /// A match leaves the return pending for the later exact observation and never sets an origin.
    pub fn arrived(&mut self, role: usize, same: impl FnOnce(&T) -> bool) -> bool {
        if !self.returning[role] { return false; }
        let Some(origin) = self.origins[role].as_ref() else { return false; };
        if same(origin) { return false; }
        self.forget();
        true
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

    fn qp_waiting(session: &mut WaitingSession<i32>) {
        session.scene_changed(true, 0x0d, 0x07010101);
    }

    fn qp_match_and_return(session: &mut WaitingSession<i32>) {
        session.scene_changed(true, 0x0e, 0x07010101);
        qp_waiting(session);
    }

    fn a_new_pick_after_a_match_forgets_both_roles() {
        let mut session = WaitingSession::empty();
        qp_waiting(&mut session);
        assert!(!session.observe(0, 10));
        assert!(!session.observe(1, 10));
        qp_match_and_return(&mut session);
        assert!(session.observe(0, 30));
        assert_eq!(session.restore_target(0), None);
        assert_eq!(session.restore_target(1), None);
        assert!(!session.observe(1, 30));
        assert!(!session.restoration_pending([true, true]));

        let mut session = WaitingSession::empty();
        qp_waiting(&mut session);
        assert!(!session.observe(0, 10));
        assert!(!session.observe(1, 10));
        session.commit(0, 20);
        session.commit(1, 6);
        qp_match_and_return(&mut session);
        assert_eq!(session.restore_target(0), Some(20));
        assert!(session.observe(0, 30));
        assert_eq!(session.restore_target(0), None);
        assert_eq!(session.restore_target(1), None);
    }

    fn the_cpu_seen_first_with_the_new_pick_forgets_the_player_too() {
        let mut session = WaitingSession::empty();
        qp_waiting(&mut session);
        assert!(!session.observe(0, 10));
        assert!(!session.observe(1, 10));
        session.commit(0, 20);
        session.commit(1, 6);
        qp_match_and_return(&mut session);
        assert!(session.observe(1, 30));
        assert_eq!(session.restore_target(0), None);
        assert!(!session.observe(0, 30));
    }

    fn the_new_pick_is_the_lobbys_fighter_from_then_on() {
        let mut session = WaitingSession::empty();
        qp_waiting(&mut session);
        assert!(!session.observe(0, 10));
        assert!(!session.observe(1, 10));
        qp_match_and_return(&mut session);
        assert!(session.observe(0, 30));
        assert!(!session.observe(1, 30));
        session.commit(0, 40);
        qp_match_and_return(&mut session);
        assert!(!session.observe(0, 30));
        assert_eq!(session.restore_target(0), Some(40));
        assert!(!session.observe(1, 30));
        qp_match_and_return(&mut session);
        assert!(session.observe(0, 10));
        assert_eq!(session.restore_target(0), None);
        assert_eq!(session.restore_target(1), None);
    }

    fn a_rematch_still_brings_back_both_practice_choices() {
        let mut session = WaitingSession::empty();
        qp_waiting(&mut session);
        assert!(!session.observe(0, 10));
        assert!(!session.observe(1, 10));
        session.commit(0, 20);
        session.commit(1, 6);
        for _ in 0..2 {
            qp_match_and_return(&mut session);
            assert!(!session.observe(0, 10));
            assert!(!session.observe(1, 10));
            assert_eq!(session.restore_target(0), Some(20));
            assert_eq!(session.restore_target(1), Some(6));
            session.commit(0, 20);
            session.commit(1, 6);
        }
    }

    fn a_room_recreated_on_the_same_scene_still_forgets_through_its_departed_entry() {
        let mut session = WaitingSession::empty();
        session.scene_changed(true, 0x0d, 0x02010000);
        assert!(!session.observe(0, 10));
        assert!(!session.observe(1, 10));
        session.commit(0, 20);
        session.player_departed();
        assert!(session.observe(0, 10));
        assert_eq!(session.restore_target(0), None);
        assert!(!session.observe(1, 10));
        session.commit(1, 6);
        session.scene_changed(true, 0x01, 0x02010000);
        session.scene_changed(true, 0x0d, 0x02010000);
        assert!(!session.observe(1, 10));
        assert_eq!(session.restore_target(1), Some(6));
    }

    fn a_body_not_yet_known_exactly_decides_on_its_fighter() {
        let mut session = WaitingSession::empty();
        qp_waiting(&mut session);
        assert!(!session.arrived(0, |origin| *origin == 30));
        assert!(!session.observe(0, 10));
        assert!(!session.observe(1, 10));
        session.commit(0, 20);
        qp_match_and_return(&mut session);
        assert!(!session.arrived(0, |origin| *origin == 10));
        assert_eq!(session.restore_target(0), Some(20));
        assert!(!session.observe(0, 10));
        assert_eq!(session.restore_target(0), Some(20));

        let mut session = WaitingSession::empty();
        qp_waiting(&mut session);
        assert!(!session.observe(0, 10));
        assert!(!session.observe(1, 10));
        session.commit(0, 20);
        qp_match_and_return(&mut session);
        assert!(session.arrived(0, |origin| *origin == 30));
        assert_eq!(session.restore_target(0), None);
        assert_eq!(session.restore_target(1), None);
        assert!(!session.observe(0, 31));

        let mut session = WaitingSession::empty();
        qp_waiting(&mut session);
        assert!(!session.observe(0, 10));
        qp_match_and_return(&mut session);
        assert!(!session.arrived(1, |origin| *origin == 30));
    }

    fn a_change_of_body_inside_one_waiting_room_is_not_a_new_pick() {
        let mut session = WaitingSession::empty();
        qp_waiting(&mut session);
        assert!(!session.observe(0, 10));
        assert!(!session.observe(0, 20));
        session.commit(0, 20);
        qp_match_and_return(&mut session);
        assert!(!session.observe(0, 10));
        assert_eq!(session.restore_target(0), Some(20));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("disabled_player_restore_does_not_block_cpu_hat_continuation", disabled_player_restore_does_not_block_cpu_hat_continuation as fn()),
            ("a_new_pick_after_a_match_forgets_both_roles", a_new_pick_after_a_match_forgets_both_roles as fn()),
            ("the_cpu_seen_first_with_the_new_pick_forgets_the_player_too", the_cpu_seen_first_with_the_new_pick_forgets_the_player_too as fn()),
            ("the_new_pick_is_the_lobbys_fighter_from_then_on", the_new_pick_is_the_lobbys_fighter_from_then_on as fn()),
            ("a_rematch_still_brings_back_both_practice_choices", a_rematch_still_brings_back_both_practice_choices as fn()),
            ("a_room_recreated_on_the_same_scene_still_forgets_through_its_departed_entry", a_room_recreated_on_the_same_scene_still_forgets_through_its_departed_entry as fn()),
            ("a_body_not_yet_known_exactly_decides_on_its_fighter", a_body_not_yet_known_exactly_decides_on_its_fighter as fn()),
            ("a_change_of_body_inside_one_waiting_room_is_not_a_new_pick", a_change_of_body_inside_one_waiting_room_is_not_a_new_pick as fn()),
        ]);
    }
    }
}
