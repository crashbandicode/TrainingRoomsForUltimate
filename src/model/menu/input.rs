//! The logical buttons the menu reacts to, and the hold-repeat rule of the input adapter
//! (SPEC-core-02 §2.3: a direction held on an analog stick repeats every 21 frames; the d-pad does
//! not repeat). The adapter itself is package 5; the rule is here so it is tested with the model.

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
    ZL,
    ZR,
}

/// Frames a stick direction repeats at.
pub const HOLD_REPEAT_PERIOD: u32 = 21;

/// One stick direction's hold counter: it counts frames held and fires when it exceeds 20, which
/// also restarts it, so the repeat comes every 21 frames. The first press is the adapter's own press
/// edge, not this counter's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HoldRepeat {
    held: u32,
}

impl HoldRepeat {
    /// Advance one input frame; `true` when the held direction repeats on this frame.
    pub fn frame(&mut self, held: bool) -> bool {
        if !held {
            self.held = 0;
            return false;
        }
        self.held += 1;
        if self.held > HOLD_REPEAT_PERIOD - 1 {
            self.held = 0;
            true
        } else {
            false
        }
    }

    /// Every accepted input resets the counters (SPEC-core-02 §2.3).
    pub fn reset(&mut self) {
        self.held = 0;
    }
}
