//! One drill, and the walk over a sequence of combos (recording redesign, rr-2).
//!
//! A sequence step uses the combo helper's summary: `clean` already means the recorded input events were
//! there, the same kind and direction, in order, within the tolerance, and the move matched. Drift is
//! not part of that. TRUE is not required. A drop returns
//! to combo 1, sets the streak to 0, and leaves best. Make combo records the new combo as used and
//! does not change the drill. The hot five that "used" feeds is the index in rr-1.

use super::combo::Summary;

/// Ids the recording index uses, cut to 48 bytes on a character boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrillId {
    bytes: [u8; 48],
    len: u8,
}

impl DrillId {
    pub const EMPTY: DrillId = DrillId { bytes: [0; 48], len: 0 };

    pub fn new(text: &str) -> DrillId {
        let mut id = DrillId::EMPTY;
        for c in text.chars() {
            let next = usize::from(id.len) + c.len_utf8();
            if next > id.bytes.len() {
                break;
            }
            c.encode_utf8(&mut id.bytes[usize::from(id.len)..next]);
            id.len = next as u8;
        }
        id
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or("")
    }
}

/// What the one drill is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Combo,
    Sequence,
}

/// The helper drills one thing. Selecting either kind replaces the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OneDrill {
    kind: Option<Kind>,
    id: DrillId,
    /// The combo Make combo last produced. It is not the drill.
    made: Option<DrillId>,
}

impl OneDrill {
    pub const fn new() -> Self {
        Self { kind: None, id: DrillId::EMPTY, made: None }
    }

    pub fn select(&mut self, kind: Kind, id: &str) {
        self.kind = Some(kind);
        self.id = DrillId::new(id);
    }

    pub fn current(&self) -> Option<(Kind, &str)> {
        self.kind.map(|kind| (kind, self.id.as_str()))
    }

    /// Mark `id` used. The drill id stays.
    pub fn make_combo(&mut self, id: &str) -> DrillId {
        let made = DrillId::new(id);
        self.made = Some(made);
        made
    }

    pub fn made(&self) -> Option<&str> {
        self.made.as_ref().map(DrillId::as_str)
    }
}

impl Default for OneDrill {
    fn default() -> Self {
        Self::new()
    }
}

/// Combos in one sequence, and where the drill is in that list.
pub const STEPS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SequenceWalk {
    ids: [DrillId; STEPS],
    len: usize,
    at: usize,
    streak: u16,
    best: u16,
}

impl SequenceWalk {
    pub fn new(ids: &[&str]) -> Self {
        let mut walk = Self { ids: [DrillId::EMPTY; STEPS], len: 0, at: 0, streak: 0, best: 0 };
        for id in ids.iter().take(STEPS) {
            walk.ids[walk.len] = DrillId::new(id);
            walk.len += 1;
        }
        walk
    }

    pub fn cursor(&self) -> usize {
        self.at
    }

    pub fn steps(&self) -> usize {
        self.len
    }

    pub fn current(&self) -> Option<&str> {
        (self.at < self.len).then(|| self.ids[self.at].as_str())
    }

    pub fn streak(&self) -> u16 {
        self.streak
    }

    pub fn best(&self) -> u16 {
        self.best
    }

    /// One combo of the sequence finished. `summary.clean` is the combo helper's grade.
    pub fn note_summary(&mut self, summary: &Summary) {
        self.note(summary.clean);
    }

    fn note(&mut self, clean: bool) {
        if self.len == 0 {
            return;
        }
        if !clean {
            self.at = 0;
            self.streak = 0;
            return;
        }
        if self.at + 1 >= self.len {
            self.streak = self.streak.saturating_add(1);
            self.best = self.best.max(self.streak);
            self.at = 0;
        } else {
            self.at += 1;
        }
    }
}

/// A drilled sequence: its id and the walk over the combos that still open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SequenceDrill {
    id: DrillId,
    walk: SequenceWalk,
}

impl SequenceDrill {
    /// A drill of sequence `id` over `present` (its combo ids that still open, in file order), at step 1.
    pub fn new(id: &str, present: &[&str]) -> Self {
        Self { id: DrillId::new(id), walk: SequenceWalk::new(present) }
    }

    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    /// The combo the helper grades now. None when no combo is left.
    pub fn target(&self) -> Option<&str> {
        self.walk.current()
    }

    /// The current step counted from 1, and the number of steps. (0, 0) when no combo is left.
    pub fn step(&self) -> (usize, usize) {
        if self.target().is_some() {
            (self.walk.cursor() + 1, self.walk.steps())
        } else {
            (0, 0)
        }
    }

    pub fn streak(&self) -> u16 {
        self.walk.streak()
    }

    pub fn best(&self) -> u16 {
        self.walk.best()
    }

    /// One graded attempt of the current combo finished. Returns the combo to grade next.
    pub fn note_summary(&mut self, summary: &Summary) -> Option<&str> {
        self.walk.note_summary(summary);
        self.target()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::combo::{Outcome, Summary};

    fn pass() -> Summary {
        Summary { outcome: Outcome::Completed, graded: 1, true_links: 0, total: 1, clean: true, fault: None }
    }

    fn missed() -> Summary {
        Summary { outcome: Outcome::Stopped, graded: 1, true_links: 0, total: 1, clean: false, fault: None }
    }

    #[test]
    fn a_drop_restarts_the_sequence_and_keeps_best() {
        let mut walk = SequenceWalk::new(&["combo-1", "combo-2"]);
        walk.note_summary(&pass());
        walk.note_summary(&pass());
        walk.note_summary(&pass());
        walk.note_summary(&pass());
        assert_eq!((walk.cursor(), walk.streak(), walk.best()), (0, 2, 2));
        walk.note_summary(&pass());
        assert_eq!(walk.cursor(), 1);
        assert_eq!(walk.current(), Some("combo-2"));
        walk.note_summary(&missed());
        assert_eq!(walk.cursor(), 0);
        assert_eq!(walk.current(), Some("combo-1"));
        assert_eq!((walk.streak(), walk.best()), (0, 2), "streak 0, best kept");
        walk.note_summary(&pass());
        walk.note_summary(&pass());
        assert_eq!((walk.streak(), walk.best()), (1, 2), "best = max(best, streak) after a finished run");
    }

    #[test]
    fn one_drill_and_make_combo_leaves_it() {
        let mut drill = OneDrill::new();
        drill.select(Kind::Sequence, "seq-1");
        drill.select(Kind::Combo, "combo-a");
        assert_eq!(drill.current(), Some((Kind::Combo, "combo-a")), "a combo replaces the sequence");
        drill.select(Kind::Sequence, "seq-2");
        assert_eq!(drill.current(), Some((Kind::Sequence, "seq-2")), "a sequence replaces the combo");
        let made = drill.make_combo("combo-new");
        assert_eq!(made.as_str(), "combo-new");
        assert_eq!(drill.made(), Some("combo-new"));
        assert_eq!(drill.current(), Some((Kind::Sequence, "seq-2")), "Make combo leaves the drill");
    }
}
