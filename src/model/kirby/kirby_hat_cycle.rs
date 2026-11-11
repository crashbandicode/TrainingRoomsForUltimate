//! Value-only Kirby shortcut state. Native loading commits only after acceptance.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HatCycle {
    last: Option<u32>,
}

impl HatCycle {
    pub const fn new() -> Self {
        Self { last: None }
    }

    pub fn target(&self, choices: &[u32], equipped: Option<u32>) -> Option<u32> {
        let first = *choices.first()?;
        let Some(index) = self
            .last
            .and_then(|last| choices.iter().position(|k| *k == last))
        else {
            return Some(first);
        };
        if equipped == self.last {
            Some(choices[(index + 1) % choices.len()])
        } else {
            // Losing the hat, including a KO, restores it without advancing.
            self.last
        }
    }

    pub fn commit(&mut self, kind: u32) {
        self.last = Some(kind);
    }
}

/// An equipped hat is distinct from the shortcut cursor: losing a hat must stay
/// lost, while the next explicit shortcut can still reapply the last selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HatSnapshot {
    pub cycle: HatCycle,
    pub equipped: Option<u32>,
}

pub struct HatContinuation {
    choice: crate::engine::swap::character_memory::WaitingChoice<HatSnapshot>,
}

impl Default for HatContinuation {
    fn default() -> Self { Self::new() }
}

impl HatContinuation {
    pub const fn new() -> Self {
        Self { choice: crate::engine::swap::character_memory::WaitingChoice::empty() }
    }

    pub fn scene_changed(&mut self, same_lobby: bool) {
        if same_lobby { self.choice.scene_changed(); } else { self.clear(); }
    }

    pub fn clear(&mut self) { self.choice.clear(); }

    pub fn pending(&self) -> bool { self.choice.restore_target().is_some() }

    /// Called only on a stable live Kirby, never on a transitional CSS body.
    pub fn observe(&mut self, snapshot: HatSnapshot) {
        if !self.pending() { self.choice.commit(snapshot); }
    }

    /// One attempt per return, consumed only after the new body and UI are ready.
    pub fn take_restore(&mut self) -> Option<HatSnapshot> {
        let snapshot = self.choice.restore_target()?;
        self.choice.manual_request();
        Some(snapshot)
    }
}

/// A claimed shortcut owns Special until its release. A held chord never repeats.
#[derive(Clone, Copy, Debug, Default)]
pub struct ShortcutLatch {
    held: bool,
    owned_special: bool,
}

impl ShortcutLatch {
    pub const fn new() -> Self {
        Self {
            held: false,
            owned_special: false,
        }
    }

    /// Lets the input hook avoid game-object lookups on ordinary input frames.
    pub fn would_request(&self, special: bool, neutral: bool, left: bool, right: bool) -> bool {
        special && neutral && (left != right) && !self.held
    }

    /// Returns (new role request, suppress Special). Role 0 is P1, 1 is CPU.
    pub fn sample(
        &mut self,
        special: bool,
        neutral: bool,
        left_click: bool,
        right_click: bool,
        available: [bool; 2],
    ) -> (Option<usize>, bool) {
        if !special {
            self.owned_special = false;
        }
        let chord = special && (left_click || right_click);
        let edge = chord && !self.held;
        self.held = chord;
        let role = match (left_click, right_click) {
            (true, false) => Some(0),
            (false, true) => Some(1),
            _ => None,
        };
        let request = role.filter(|role| edge && neutral && available[*role]);
        if request.is_some() {
            self.owned_special = true;
        }
        (request, self.owned_special)
    }
}

/// (left, right) clicks for [`ShortcutLatch::sample`] from the controller operating the
/// CPU under native Behavior=Control. Its CPU hat chord is P1's (Special plus right stick
/// click); Special plus left stick click, its own fighter, is an alias. Both clicks hold
/// the chord without a request, as for P1. It never requests P1's hat.
pub const fn operator_clicks(left_click: bool, right_click: bool) -> (bool, bool) {
    (left_click && right_click, left_click || right_click)
}

/// The role a Kirby frame callback serves: the role whose Kirby body is the callback's own body. The callback
/// runs once per Kirby with that Kirby's body (the fork's `SITE_KIRBY_OPFF` call), so each role is reached by
/// its own body; any other body serves no role.
pub fn hat_role<T: PartialEq>(body: T, roles: [Option<T>; 2]) -> Option<usize> {
    roles.iter().position(|role| role.as_ref() == Some(&body))
}

/// The game frames a pending hat request may take before it ends with "Kirby hat could not be loaded".
pub const REQUEST_FRAMES: u16 = 900;

/// HAT-REQUEST-END R1 (operator option A, 2026-11-05): one game frame of a pending request, counted whether or not its
/// role's Kirby was served this frame (`held`: a save state is restoring it). The new age, and whether it ends now.
pub const fn request_frame(age: u16, held: bool) -> (u16, bool) {
    if held {
        return (age, false);
    }
    let age = age.saturating_add(1);
    (age, age > REQUEST_FRAMES)
}

/// HAT-REQUEST-END R2: committing a fighter other than Kirby (kind 6) to a role ends that role's pending request.
pub const fn commit_ends_request(kind: u32) -> bool {
    kind != 6
}

