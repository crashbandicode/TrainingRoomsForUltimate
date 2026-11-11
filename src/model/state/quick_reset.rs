//! SPEC-state-06 §6.4: first WAIT poses belong to entry lifetimes, not save slots.
use super::record::SavedState;
use crate::model::chords::bit;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub x: f32,
    pub y: f32,
    pub lr: f32,
    pub percent: f32,
    pub kind: i32,
    pub situation: i32,
    pub stage: i32,
}

impl Pose {
    pub fn record(self) -> SavedState {
        SavedState {
            x: self.x,
            y: self.y,
            lr: self.lr,
            percent: self.percent,
            fighter_kind: self.kind,
            situation_kind: self.situation,
            saved_stage: Some(self.stage),
            steve: None,
            ..SavedState::never_saved()
        }
    }
}

#[derive(Default)]
pub struct QuickReset {
    entries: Option<[usize; 2]>,
    poses: [Option<Pose>; 2],
    held: bool,
    /// The held chord was refused on its first poll (rc.9 C14).
    refused: bool,
}

impl QuickReset {
    pub const fn new() -> Self {
        Self {
            entries: None,
            poses: [None; 2],
            held: false,
            refused: false,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// Observe each role once at WAIT. Non-WAIT frames may invalidate old entries
    /// but cannot fill the fresh pair. Destruction also calls `clear`, including
    /// when a later allocator reuses exactly the same entry addresses.
    pub fn observe(&mut self, entries: [usize; 2], role: usize, waiting: bool, pose: Pose) {
        self.observe_entries(entries);
        if waiting {
            if let Some(slot) = self.poses.get_mut(role) {
                slot.get_or_insert(pose);
            }
        }
    }

    pub fn pair(&self, entries: [usize; 2]) -> Option<[Pose; 2]> {
        (self.entries == Some(entries)).then_some(())?;
        Some([self.poses[0]?, self.poses[1]?])
    }

    fn observe_entries(&mut self, entries: [usize; 2]) {
        if self.entries != Some(entries) {
            self.clear();
            self.entries = Some(entries);
        }
    }

    /// Check the live entry pair before skipping native pose reads. A filled
    /// cache for retired entries must not prevent observing their replacements.
    pub fn needs_pose(&mut self, entries: [usize; 2], role: usize) -> bool {
        self.observe_entries(entries);
        self.poses.get(role).is_some_and(Option::is_none)
    }

    /// L + R + A held on the raw controller.
    pub const fn chord_held(raw: u32) -> bool {
        raw & (bit::L | bit::REAL_DIGITAL_L) != 0 && raw & (bit::R | bit::REAL_DIGITAL_R) != 0 && raw & bit::A != 0
    }

    /// Swallow every eligible held chord; request only its first poll. rc.9 C14: while an AI drives Player 1 (the
    /// standalone CPU pilot, asked once on the chord's first poll, `p1_driven_by_ai`), the chord is refused for as long
    /// as it is held: nothing is swallowed (the pilot writes P1's input), nothing is requested, and the first poll says
    /// why.
    pub fn input(&mut self, raw: u32, eligible: bool, p1_driven_by_ai: impl FnOnce() -> bool) -> QuickInput {
        let held = Self::chord_held(raw);
        let first = eligible && held && !self.held;
        if first {
            self.refused = p1_driven_by_ai();
        } else if !held {
            self.refused = false;
        }
        self.held = held;
        QuickInput {
            consume: eligible && held && !self.refused,
            request: first && !self.refused,
            refused: first && self.refused,
        }
    }
}

/// What one poll does with the Quick Reset chord.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuickInput {
    /// P1's mapped input is neutral this poll.
    pub consume: bool,
    /// The reset is requested (the chord's first poll).
    pub request: bool,
    /// rc.9 C14: an AI drives Player 1, so this press is refused (its first poll): the notice says why.
    pub refused: bool,
}

/// The refusal's notice, worded as Record's (`recording::RecordRefusal::P1DrivenByAi`).
pub const AI_REFUSAL: &str = "P1 is controlled by an AI: Quick Reset is off";

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::{QuickInput, QuickReset};
    use crate::model::chords::bit;

    const CHORD: u32 = bit::L | bit::R | bit::A;

    /// rc.9 C14: with the standalone pilot driving Player 1, Quick Reset leaves P1's input alone and says why, once per
    /// press; without it the chord swallows and requests as before.
    #[test]
    fn quick_reset_is_refused_while_an_ai_drives_p1() {
        let mut q = QuickReset::new();
        let first = q.input(CHORD, true, || true);
        assert_eq!(first, QuickInput { consume: false, request: false, refused: true });
        let held = q.input(CHORD, true, || panic!("asked only on the first poll"));
        assert_eq!(held, QuickInput::default(), "still refused while held, no second notice");
        assert_eq!(q.input(0, true, || false), QuickInput::default());
        let player = q.input(CHORD, true, || false);
        assert_eq!(player, QuickInput { consume: true, request: true, refused: false });
        assert_eq!(q.input(CHORD, true, || true), QuickInput { consume: true, request: false, refused: false });
        // Not eligible (no practice, the setting off): nothing, and the pilot is not asked.
        let mut q = QuickReset::new();
        assert_eq!(q.input(CHORD, false, || panic!("not asked")), QuickInput::default());
    }
    }
}

