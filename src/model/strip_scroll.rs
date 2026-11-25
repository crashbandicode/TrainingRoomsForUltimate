//! Merge 7's R3 + D-pad Left / Right scroll of the frame strip.
//! Pure; polled by the chord detector.

use core::sync::atomic::{AtomicU64, Ordering};

use super::chords::{deluxe_rows_take, deluxe_shoulders, logical, Sample};
use super::frame_strip::{back::{Back, COLUMNS}, Tracker};
use super::menu::chord::Chord;

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
    /// One P1 poll. A blocked poll closes the scroll; `may_open` says the strip is drawn and no chord took the poll.
    pub fn poll(&mut self, sample: Sample, blocked: bool, may_open: bool) -> ScrollEvent {
        if self.open && (blocked || sample.held & HOLD_BUTTON == 0) {
            self.clear();
            return ScrollEvent::Closed;
        }
        let deluxe = deluxe_shoulders(Chord(HOLD_BUTTON), sample)
            || (sample.overlay && deluxe_rows_take(Chord(HOLD_BUTTON | STEP_BUTTONS), sample));
        let left = !deluxe && sample.pressed & (1 << logical::DPAD_LEFT) != 0;
        let right = !deluxe && sample.pressed & (1 << logical::DPAD_RIGHT) != 0;
        let pressed = match (left, right) {
            (true, false) => Some(Dir::Back),
            (false, true) => Some(Dir::Forward),
            _ => None,
        };
        if !self.open {
            if let Some(dir) = pressed.filter(|_| !blocked && may_open && sample.held & HOLD_BUTTON != 0) {
                self.open = true;
                self.dir = Some(dir);
                self.held = 0;
                return ScrollEvent::Opened(dir);
            }
            return ScrollEvent::Idle;
        }
        if let Some(dir) = pressed {
            self.dir = Some(dir);
            self.held = 0;
            return ScrollEvent::Step(dir);
        }
        let repeat = self.dir.filter(|dir| {
            let button = match dir {
                Dir::Back => 1 << logical::DPAD_LEFT,
                Dir::Forward => 1 << logical::DPAD_RIGHT,
            };
            !deluxe && sample.held & STEP_BUTTONS == button
        });
        if let Some(dir) = repeat {
            self.held = self.held.saturating_add(1);
            if self.held >= REPEAT_AFTER && (self.held - REPEAT_AFTER).is_multiple_of(REPEAT_EVERY) {
                if self.held == REPEAT_AFTER + REPEAT_EVERY { self.held = REPEAT_AFTER; }
                return ScrollEvent::Step(dir);
            }
            return ScrollEvent::Held;
        }
        self.dir = None;
        self.held = 0;
        ScrollEvent::Held
    }
    /// The logical buttons the game must not see while open.
    pub const fn hidden(self) -> u16 {
        if self.open { HOLD_BUTTON | STEP_BUTTONS } else { 0 }
    }
    /// Session clear or a re-entered poll closes without an event.
    pub fn clear(&mut self) { *self = Self::new(); }
}

/// The view's newest column after one step, or `None` for live.
pub fn stepped(anchor: Option<u32>, dir: Dir, count: u32, kept: u32) -> Option<u32> {
    if kept <= super::frame_strip::WINDOW as u32 || count == 0 { return None; }
    let oldest = count.saturating_sub(kept).saturating_add(super::frame_strip::WINDOW as u32).min(count);
    let from = anchor.unwrap_or(count).clamp(oldest, count);
    match dir {
        Dir::Back => Some(from.saturating_sub(STEP).max(oldest)),
        Dir::Forward => {
            let to = from.saturating_add(STEP);
            if to >= count { None } else { Some(to) }
        }
    }
}

/// High half: published column count. Low half: the selected column, or zero for live.
/// A single word prevents a poll based on the previous timeline racing a reset and restoring its anchor.
pub struct Cursor(AtomicU64);

impl Cursor {
    pub const fn new() -> Self { Self(AtomicU64::new(0)) }
    pub fn anchor(&self) -> u32 { self.0.load(Ordering::Acquire) as u32 }
    pub fn clear(&self) { self.0.store(0, Ordering::Release); }

    /// The tracker lock serializes this producer with `clear`; polls only change the low half.
    pub fn record(&self, back: &mut Back, tracker: &Tracker) {
        let old = (self.0.load(Ordering::Relaxed) >> 32) as u32;
        if old == 0 && back.span().0 != 0 { back.clear(); }
        back.record(tracker);
        let count = back.span().0;
        if count != old {
            self.0.fetch_add(u64::from(count.wrapping_sub(old)) << 32, Ordering::Release);
        }
    }

    pub fn publish_selection(&self, event: ScrollEvent, open: bool) {
        let _ = self.0.fetch_update(Ordering::AcqRel, Ordering::Acquire, |state| {
            let anchor = state as u32;
            let count = (state >> 32) as u32;
            let next = match event {
                _ if !open => 0,
                ScrollEvent::Opened(dir) | ScrollEvent::Step(dir) =>
                    stepped((anchor != 0).then_some(anchor), dir, count, count.min(COLUMNS as u32)).unwrap_or(0),
                _ => return None,
            };
            (next != anchor).then_some((u64::from(count) << 32) | u64::from(next))
        });
    }
}

impl Default for Cursor {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
#[path = "strip_scroll_tests.rs"]
mod tests;
