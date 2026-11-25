//! Merge 7's R3 + D-pad Left / Right scroll of the frame strip.
//! Pure; polled by the chord detector.

use super::chords::{logical, Sample};

/// Columns moved by one step.
pub const STEP: u32 = 15;
/// Held polls before the first repeat.
pub const REPEAT_AFTER: u8 = 18;
/// Held polls between repeats.
pub const REPEAT_EVERY: u8 = 6;
/// The R3 hold button.
pub const HOLD_BUTTON: u16 = 1 << logical::STICK_R;
/// The D-pad Left and Right step buttons.
pub const STEP_BUTTONS: u16 = (1 << logical::DPAD_LEFT) | (1 << logical::DPAD_RIGHT);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir { Back, Forward }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollEvent { #[default] Idle, Opened(Dir), Held, Step(Dir), Closed }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scroll { open: bool, dir: Option<Dir>, held: u8 }

impl Scroll {
    pub const fn new() -> Self { Self { open: false, dir: None, held: 0 } }
    pub const fn is_open(self) -> bool { self.open }
    pub fn poll(&mut self, _: Sample, _: bool, _: bool) -> ScrollEvent { ScrollEvent::Idle }
    pub const fn hidden(self) -> u16 { 0 }
    pub fn clear(&mut self) { *self = Self::new(); }
}

pub fn stepped(_: Option<u32>, _: Dir, _: u32, _: u32) -> Option<u32> { None }

#[cfg(test)]
#[path = "strip_scroll_tests.rs"]
mod tests;
