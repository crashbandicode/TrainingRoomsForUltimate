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

