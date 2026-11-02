//! Input Delay (SPEC-input-03): P1's final mapped input is held back by 0 to 30 polls before the
//! input display and the recorder see it. A fixed ring of 31 eight-byte records in static data:
//! stepping it allocates nothing, so the hook can run it (playbook: no heap in hooks).
//!
//! The queue advances once per P1 poll of the game's final input mapping, not once per frame
//! (SPEC-input-03 §5.3, open question 1).

/// The largest delay in frames (the menu row has 31 options, `0..=30`).
pub const MAX_DELAY: u8 = 30;
/// Slots in the ring: a poll pushes one record before the oldest are dropped, so `MAX_DELAY + 1`.
const SLOTS: usize = MAX_DELAY as usize + 1;

/// The game's final mapped input record (SPEC-input-01 §3.1, 8 bytes). The all-zero record is
/// neutral: no buttons, both sticks centred.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct MappedInputs {
    pub buttons: u32,
    pub lstick_x: i8,
    pub lstick_y: i8,
    pub rstick_x: i8,
    pub rstick_y: i8,
}

const _: () = assert!(core::mem::size_of::<MappedInputs>() == 8);

impl MappedInputs {
    /// The record the fighter reads while the queue is still filling.
    pub const NEUTRAL: MappedInputs = MappedInputs { buttons: 0, lstick_x: 0, lstick_y: 0, rstick_x: 0, rstick_y: 0 };
}

/// What owns P1's input for a poll. Anything other than gameplay empties the queue and makes that
/// poll neutral (SPEC-input-03 §5.1: "the queue is emptied whenever something other than gameplay
/// owns P1's input, and on session clear").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Owners(u8);

impl Owners {
    /// Gameplay owns the input.
    pub const NONE: Owners = Owners(0);
    /// The plugin's menu or a modal panel captured the input, or closed within the last 15 frames.
    pub const MENU: Owners = Owners(1);
    /// The named stop chord was consumed this poll.
    pub const STOP_CHORD: Owners = Owners(2);
    /// The native waiting-room UI became busy.
    pub const NATIVE_UI_BUSY: Owners = Owners(4);
    /// The replay editor owns the poll.
    pub const REPLAY_EDITOR: Owners = Owners(8);
    /// The game's own pause menu is open (SPEC-input-03 question 3, recommended).
    pub const VANILLA_PAUSE: Owners = Owners(16);

    pub const fn with(self, other: Owners) -> Owners {
        Owners(self.0 | other.0)
    }

    pub const fn contains(self, other: Owners) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn any(self) -> bool {
        self.0 != 0
    }
}

/// Why the queue was emptied outside a poll.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clear {
    /// Scene change, hot swap or a lobby-practice change (SPEC-input-03 §5.1 "session clear").
    Session,
    /// A save state was loaded (decision B8, SPEC-input-03 question 2): a load repositions the
    /// fighters, so presses queued before it are never what the player meant.
    SaveStateLoad,
    /// Training was reset with L+R+A or by the game (decision B8; the reset hub's last step).
    TrainingReset,
}

/// The delay queue.
#[derive(Clone, Copy, Debug)]
pub struct DelayQueue {
    ring: [MappedInputs; SLOTS],
    /// Index of the oldest record.
    head: usize,
    len: usize,
}

impl Default for DelayQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl DelayQueue {
    pub const fn new() -> Self {
        Self { ring: [MappedInputs::NEUTRAL; SLOTS], head: 0, len: 0 }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Empty the queue; the next `delay` polls output neutral.
    pub fn clear(&mut self, _why: Clear) {
        self.head = 0;
        self.len = 0;
    }

    fn push_newest(&mut self, record: MappedInputs) {
        let at = (self.head + self.len) % SLOTS;
        self.ring[at] = record;
        self.len += 1;
    }

    fn pop_oldest(&mut self) {
        self.head = (self.head + 1) % SLOTS;
        self.len -= 1;
    }

    /// One P1 poll with delay `delay` (clamped to `MAX_DELAY`) and the record the game just mapped.
    /// Returns the record the game should see (SPEC-input-03 §5).
    ///
    /// 1. `actual` is the poll's record.
    /// 2. Fewer than `delay` queued: neutral. Otherwise the oldest queued record, if any.
    /// 3. `actual` is queued as the newest.
    /// 4. The oldest records are dropped until at most `delay` remain.
    pub fn step(&mut self, delay: u8, actual: MappedInputs) -> MappedInputs {
        let delay = usize::from(delay.min(MAX_DELAY));
        let out = if self.len < delay {
            MappedInputs::NEUTRAL
        } else if self.len > 0 {
            self.ring[self.head]
        } else {
            actual
        };
        self.push_newest(actual);
        while self.len > delay {
            self.pop_oldest();
        }
        out
    }

    /// A poll while something other than gameplay owns P1's input: the queue is emptied and the
    /// poll outputs neutral (SPEC-input-03 §5.1). Otherwise `step`.
    pub fn poll(&mut self, delay: u8, owners: Owners, actual: MappedInputs) -> MappedInputs {
        if owners.any() {
            self.clear(Clear::Session);
            return MappedInputs::NEUTRAL;
        }
        self.step(delay, actual)
    }
}

/// The stages of one P1 poll of the final input mapping, in order (SPEC-input-03 §5.2, SPEC-input-01
/// §5.1): the menu and the chords read the raw controller before the delay, so they are never
/// delayed; the input display and the recorder run after it and see delayed input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollStage {
    MenuInput,
    ChordDetection,
    InputDelay,
    InputDisplay,
    Recorder,
}

pub const POLL_ORDER: [PollStage; 5] =
    [PollStage::MenuInput, PollStage::ChordDetection, PollStage::InputDelay, PollStage::InputDisplay, PollStage::Recorder];

/// Whether the delay stage acts on a poll: only P1's (`player_idx == 0`) while Training (or an
/// active practice session) is live (SPEC-input-03 §4).
pub const fn applies(player_idx: i32, training_live: bool) -> bool {
    player_idx == 0 && training_live
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(n: u32) -> MappedInputs {
        MappedInputs { buttons: n, ..MappedInputs::NEUTRAL }
    }

    #[test]
    fn two_polls_late() {
        let mut q = DelayQueue::new();
        let out: Vec<_> = (1..=5).map(|n| q.step(2, rec(n)).buttons).collect();
        assert_eq!(out, [0, 0, 1, 2, 3]);
    }

    #[test]
    fn an_owner_empties_the_queue() {
        let mut q = DelayQueue::new();
        q.step(3, rec(1));
        assert_eq!(q.poll(3, Owners::MENU, rec(2)), MappedInputs::NEUTRAL);
        assert!(q.is_empty());
    }
}