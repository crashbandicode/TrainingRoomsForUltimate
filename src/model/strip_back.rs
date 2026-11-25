//! Merge 7 frame-strip scrollback history, written on the game thread's frame step and read by the HUD while scrolled.
//! Pure, with fixed storage.

use super::Tracker;

/// Columns kept in history.
pub const COLUMNS: usize = 600;
/// Presses kept in history.
pub const PRESSES: usize = 128;
/// Direction marks, spot marks and grade tags kept in each history ring.
pub const NOTES: usize = 64;
/// Links kept in history.
pub const LINKS: usize = 16;

pub struct Back;

impl Back {
    pub const fn new() -> Self { Self }
    pub fn clear(&mut self) {}
    pub fn span(&self) -> (u32, u32) { (0, 0) }
    pub fn record(&mut self, _: &Tracker) {}
    pub fn view(&self, _: &mut Tracker, _: u32) -> Option<u32> { None }
}

#[cfg(test)]
#[path = "strip_back_tests.rs"]
mod tests;
