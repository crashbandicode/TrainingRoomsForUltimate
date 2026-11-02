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
}

impl QuickReset {
    pub const fn new() -> Self {
        Self {
            entries: None,
            poses: [None; 2],
            held: false,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// Observe each role once at WAIT. Non-WAIT frames may invalidate old entries
    /// but cannot fill the fresh pair. Destruction also calls `clear`, including
    /// when a later allocator reuses exactly the same entry addresses.
    pub fn observe(&mut self, entries: [usize; 2], role: usize, waiting: bool, pose: Pose) {
        if self.entries != Some(entries) {
            self.clear();
            self.entries = Some(entries);
        }
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

    pub fn needs_pose(&self, role: usize) -> bool {
        self.poses.get(role).is_some_and(Option::is_none)
    }

    /// Swallow every eligible held chord; request only its first poll.
    pub fn input(&mut self, raw: u32, eligible: bool) -> (bool, bool) {
        let held = raw & (bit::L | bit::REAL_DIGITAL_L) != 0
            && raw & (bit::R | bit::REAL_DIGITAL_R) != 0
            && raw & bit::A != 0;
        let request = eligible && held && !self.held;
        self.held = held;
        (eligible && held, request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const ENTRIES: [usize; 2] = [8, 16];
    const POSE: Pose = Pose {
        x: 4.0,
        y: 1.0,
        lr: -1.0,
        percent: 17.0,
        kind: 7,
        situation: 0,
        stage: 8,
    };

    #[test]
    fn first_wait_is_kept_until_either_entry_lifetime_ends() {
        let mut q = QuickReset::new();
        q.observe(ENTRIES, 0, false, POSE);
        q.observe(ENTRIES, 1, true, POSE);
        assert_eq!(q.pair(ENTRIES), None);
        q.observe(ENTRIES, 0, true, POSE);
        q.observe(
            ENTRIES,
            0,
            true,
            Pose {
                x: 99.0,
                percent: 90.0,
                ..POSE
            },
        );
        assert_eq!(q.pair(ENTRIES), Some([POSE; 2]));
        let replacement = [8, 24];
        assert_eq!(q.pair(replacement), None);
        q.observe(replacement, 1, true, POSE);
        assert_eq!(q.pair(replacement), None);
        q.clear();
        assert_eq!(
            q.pair(ENTRIES),
            None,
            "same addresses do not revive a retired pair"
        );
    }

    #[test]
    fn analog_and_digital_shoulders_share_one_held_chord_latch() {
        let mut q = QuickReset::new();
        let digital = bit::REAL_DIGITAL_L | bit::REAL_DIGITAL_R | bit::A;
        assert_eq!(q.input(bit::L | bit::R | bit::A, true), (true, true));
        assert_eq!(q.input(digital, true), (true, false));
        assert_eq!(q.input(0, true), (false, false));
        assert_eq!(q.input(digital, true), (true, true));
        assert_eq!(q.input(digital, false), (false, false));
    }
}
