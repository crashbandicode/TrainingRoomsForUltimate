//! The logical buttons the menu reacts to and the adapter's steady and quickening hold-repeat rules.

/// Logical buttons, already mapped from the controller (SPEC-core-03 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    X,
    Y,
    L,
    R,
    /// A tap: ZL pressed and released with nothing else pressed while it was held (the adapter sends it on the
    /// release). Held, ZL is the section-jump modifier instead.
    ZL,
    ZR,
    /// ZL held + Up: the previous section of a page, or one window up in a popup list.
    JumpUp,
    /// ZL held + Down: the next section of a page, or one window down in a popup list.
    JumpDown,
}

/// Frames a held direction repeats at: the first repeat of any hold, and every repeat at the steady pace.
pub const HOLD_REPEAT_PERIOD: u32 = 21;
/// A quickening hold: from this many frames held, this interval.
pub const HOLD_QUICKEN: [(u32, u32); 3] = [(22, 8), (60, 5), (120, 4)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pace { Steady, Quickening }

/// One direction's held time and time since the last repeat. The first press is the adapter's edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HoldRepeat {
    held: u32,
    since: u32,
}

impl HoldRepeat {
    pub const fn new() -> HoldRepeat {
        HoldRepeat { held: 0, since: 0 }
    }

    /// Advance one input frame; a repeat due while not ready waits for the first ready frame.
    pub fn frame(&mut self, held: bool, pace: Pace, ready: bool) -> bool {
        if !held {
            self.reset();
            return false;
        }
        self.held = self.held.saturating_add(1);
        self.since = self.since.saturating_add(1);
        let interval = match pace {
            Pace::Steady => HOLD_REPEAT_PERIOD,
            Pace::Quickening => HOLD_QUICKEN.iter().rev().find(|(from, _)| self.held >= *from)
                .map_or(HOLD_REPEAT_PERIOD, |&(_, interval)| interval),
        };
        let fire = ready && self.since >= interval;
        if fire { self.since = 0; }
        fire
    }

    /// Every accepted input resets the counters (SPEC-core-02 §2.3).
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}
