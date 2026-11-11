//! One drill, and the walk over a sequence of combos (recording redesign, rr-2).
//!
//! A sequence step uses the combo helper's summary: `clean` already means the recorded input events were
//! there, the same kind and direction, in order, within the tolerance, and the move matched. Drift is
//! not part of that. TRUE is not required. A drop returns
//! to combo 1, sets the streak to 0, and leaves best. Make combo records the new combo as used and
//! does not change the drill. The hot five that "used" feeds is the index in rr-1.

use super::combo::{Combo, Helper, Out, Settings, Summary};
use super::frame_strip::Tracker;

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

/// What one graded step does to a sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Advance {
    /// The step was clean and the sequence goes on: the next step, by index, follows in the same exchange.
    Next(usize),
    /// The last step was clean: the streak went up by one and the sequence is back at combo 1.
    Finished,
    /// The step dropped: back at combo 1, the streak is 0 and best is kept.
    Dropped,
    /// An empty sequence drills nothing.
    Idle,
}

/// Where a drilled sequence is: the step, the number of steps, the streak and the best streak. It keeps no ids, so a
/// sequence of any length is walked in full and stepping it never allocates (the frame hook steps it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SequenceWalk {
    len: usize,
    at: usize,
    streak: u32,
    best: u32,
}

impl SequenceWalk {
    /// A walk over `len` steps at combo 1, with no streak.
    pub const fn new(len: usize) -> Self {
        Self::resume(len, 0, 0)
    }

    /// A walk at combo 1 that goes on from a saved streak and best (the sequence file's).
    pub const fn resume(len: usize, streak: u32, best: u32) -> Self {
        Self { len, at: 0, streak, best }
    }

    pub fn cursor(&self) -> usize {
        self.at
    }

    pub fn steps(&self) -> usize {
        self.len
    }

    pub fn streak(&self) -> u32 {
        self.streak
    }

    pub fn best(&self) -> u32 {
        self.best
    }

    /// One combo of the sequence finished. `summary.clean` is the combo helper's grade.
    pub fn note_summary(&mut self, summary: &Summary) -> Advance {
        self.note(summary.clean)
    }

    fn note(&mut self, clean: bool) -> Advance {
        if self.len == 0 {
            return Advance::Idle;
        }
        if !clean {
            self.at = 0;
            self.streak = 0;
            return Advance::Dropped;
        }
        if self.at + 1 >= self.len {
            self.streak = self.streak.saturating_add(1);
            self.best = self.best.max(self.streak);
            self.at = 0;
            Advance::Finished
        } else {
            self.at += 1;
            Advance::Next(self.at)
        }
    }
}

/// A drilled sequence: its id, the ids of its combos that still open, and the walk over them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequenceDrill {
    id: DrillId,
    ids: Vec<String>,
    walk: SequenceWalk,
}

impl SequenceDrill {
    /// A drill of sequence `id` over `present` (its combo ids that still open, in file order), at step 1.
    pub fn new(id: &str, present: &[&str]) -> Self {
        Self::resume(id, present, 0, 0)
    }

    /// The same, going on from the streak and best saved in the sequence file.
    pub fn resume(id: &str, present: &[&str], streak: u32, best: u32) -> Self {
        Self {
            id: DrillId::new(id),
            ids: present.iter().map(|id| (*id).to_owned()).collect(),
            walk: SequenceWalk::resume(present.len(), streak, best),
        }
    }

    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    /// The combo ids, in order.
    pub fn ids(&self) -> &[String] {
        &self.ids
    }

    pub fn walk(&self) -> SequenceWalk {
        self.walk
    }

    /// The combo the helper grades now. None when no combo is left.
    pub fn target(&self) -> Option<&str> {
        (self.walk.cursor() < self.ids.len()).then(|| self.ids[self.walk.cursor()].as_str())
    }

    /// The current step counted from 1, and the number of steps. (0, 0) when no combo is left.
    pub fn step(&self) -> (usize, usize) {
        if self.target().is_some() {
            (self.walk.cursor() + 1, self.walk.steps())
        } else {
            (0, 0)
        }
    }

    pub fn streak(&self) -> u32 {
        self.walk.streak()
    }

    pub fn best(&self) -> u32 {
        self.walk.best()
    }

    /// One graded attempt of the current combo finished. Returns the combo to grade next.
    pub fn note_summary(&mut self, summary: &Summary) -> Option<&str> {
        self.walk.note_summary(summary);
        self.target()
    }
}

/// A drilled sequence on the frame hook: the walk, and what the bar says about the last graded step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SequenceRun {
    walk: SequenceWalk,
    last: Option<Stepped>,
}

/// The last graded step: which combo (0-based), its grade, and what it did to the sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stepped {
    pub step: usize,
    pub summary: Summary,
    pub advance: Advance,
}

impl SequenceRun {
    pub const fn new(walk: SequenceWalk) -> Self {
        Self { walk, last: None }
    }

    pub fn walk(&self) -> SequenceWalk {
        self.walk
    }

    pub fn last(&self) -> Option<Stepped> {
        self.last
    }

    /// The helper's target at the start: combo 1, graded from the first link of an exchange. An empty sequence sets
    /// no target, so it drills nothing.
    pub fn start(&self, helper: &mut Helper, steps: &[Combo]) {
        helper.set_target(steps.first().copied());
    }

    /// One frame, after the strip's tracker stepped: the helper grades the current combo, and a graded step moves
    /// the sequence. A clean step that is not the last one hands the helper the next combo inside the same exchange
    /// (`Helper::continue_with`). A finished run or a drop gives it combo 1 again, from a new exchange.
    pub fn step(&mut self, helper: &mut Helper, steps: &[Combo], tracker: &Tracker, settings: Settings) -> (Out, Option<Stepped>) {
        let out = helper.step(tracker, settings);
        let stepped = self.after(helper, steps, &out);
        (out, stepped)
    }

    /// The strip's session clear: an open step ends as cleared, which is not clean, so the sequence starts over.
    pub fn clear(&mut self, helper: &mut Helper, steps: &[Combo]) -> (Out, Option<Stepped>) {
        let out = helper.clear();
        let stepped = self.after(helper, steps, &out);
        (out, stepped)
    }

    fn after(&mut self, helper: &mut Helper, steps: &[Combo], out: &Out) -> Option<Stepped> {
        let summary = out.resolved?;
        let step = self.walk.cursor();
        let advance = self.walk.note_summary(&summary);
        match advance {
            Advance::Next(next) if next < steps.len() => helper.continue_with(steps[next]),
            Advance::Next(_) | Advance::Finished | Advance::Dropped => helper.restart_with(steps.first().copied()),
            Advance::Idle => {}
        }
        let stepped = Stepped { step, summary, advance };
        self.last = Some(stepped);
        Some(stepped)
    }
}

