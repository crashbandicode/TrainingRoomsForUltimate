//! The input-delay adapter (SPEC-input-03 §4, §5.1): everything the Switch hooks do with the delay queue
//! and the input-owner mask, as plain code the host tests drive. The runtime's hooks hold an
//! `InputAdapter` behind a lock and call exactly these methods:
//!
//! - `poll`: the P1 mapping poll (`caller_*_apply`), reading the owner mask itself;
//! - `pause_flag` (with `pause_begin` / `pause_end` under it): the ownership producer for the native
//!   Training menu, fed by the game's own layout state that SPEC-core-03 section 2.4 names (the help
//!   footer pane `L_staying_help` of `info_training` left its rest position), sampled by the UI tick;
//! - `clear`: the reset hub's and the save-state load's queue clear.
//!
//! The owner mask is the atomic `published::INPUT_OWNERS` (other packages set their own bits on it).
//! `pause_begin` sets the pause bit AND empties the queue in the same call, so the queue is empty from
//! the moment the pause begins even if no mapping poll runs while paused; `pause_end` releases the bit.
//! Both run under the same lock as `poll`, so a poll is wholly before or wholly after either.
//!
//! The adapter also keeps a small trace for the `diag` build's Eden pause lane: how many polls the pause
//! owned and whether any of them produced input (it must not), and, after a resume, whether any of the
//! first `delay` gameplay polls produced input (they must all be neutral: the queue was emptied).

use core::sync::atomic::{AtomicU8, Ordering};

use super::input_delay::{Clear, DelayQueue, MappedInputs, Owners, Sticks};

/// Set `owner`'s bit in the mask.
pub fn take(mask: &AtomicU8, owner: Owners) {
    mask.fetch_or(owner.bits(), Ordering::AcqRel);
}

/// Clear `owner`'s bit in the mask.
pub fn release(mask: &AtomicU8, owner: Owners) {
    mask.fetch_and(!owner.bits(), Ordering::AcqRel);
}

/// The owners in the mask now.
pub fn owners(mask: &AtomicU8) -> Owners {
    Owners::from_bits(mask.load(Ordering::Acquire))
}

/// An edge of the pause flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseEdge {
    /// The menu opened; the queue held this many records and was emptied.
    Begin { dropped: usize },
    /// The menu closed.
    End(PauseReport),
}

/// What a pause owned, reported when it ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PauseReport {
    /// Records the queue held when the pause began.
    pub queued_at_begin: usize,
    /// Mapping polls while the pause owned the input.
    pub owned_polls: u32,
    /// Of those, polls that gave the game anything but neutral (must be 0).
    pub owned_non_neutral: u32,
}

/// The first `delay` gameplay polls after a resume, checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResumeCheck {
    pub delay: u8,
    /// Of the first `delay` polls after the resume, how many produced a non-neutral record (must be 0).
    pub leaked: u32,
}

/// The queue, the owner-mask rules and the pause trace.
#[derive(Clone, Copy, Debug)]
pub struct InputAdapter {
    queue: DelayQueue,
    report: PauseReport,
    paused: bool,
    resume_left: u8,
    resume_delay: u8,
    resume_leaked: u32,
    /// Native layout identity only, never dereferenced. Session retirement
    /// invalidates observations that began before it, even if an address is reused.
    pause_layout: usize,
    pause_epoch: u32,
}

impl Default for InputAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl InputAdapter {
    pub const fn new() -> Self {
        Self { queue: DelayQueue::new(), report: PauseReport { queued_at_begin: 0, owned_polls: 0, owned_non_neutral: 0 }, paused: false, resume_left: 0, resume_delay: 0, resume_leaked: 0, pause_layout: 0, pause_epoch: 0 }
    }

    /// Records in the queue.
    pub const fn queued(&self) -> usize {
        self.queue.len()
    }

    /// One P1 poll: `actual` is the record the game mapped; returns what the game gets. Owners other
    /// than gameplay empty the queue and give neutral (`DelayQueue::poll`). The second value is the
    /// resume check, once, when the first `delay` polls after a resume are done.
    pub fn poll(&mut self, mask: &AtomicU8, delay: u8, actual: MappedInputs) -> (MappedInputs, Option<ResumeCheck>) {
        let (out, _, check) = self.poll_with(mask, delay, actual, Sticks::of_record(actual));
        (out, check)
    }

    /// `poll`, with the poll's physical sticks carried through the delay and returned with the record.
    pub fn poll_with(&mut self, mask: &AtomicU8, delay: u8, actual: MappedInputs, sticks: Sticks)
        -> (MappedInputs, Sticks, Option<ResumeCheck>) {
        let owners = owners(mask);
        let (out, sticks) = self.queue.poll_with(delay, owners, actual, sticks);
        if owners.contains(Owners::VANILLA_PAUSE) {
            self.report.owned_polls += 1;
            self.report.owned_non_neutral += u32::from(out != MappedInputs::NEUTRAL);
        }
        let mut check = None;
        if self.resume_left > 0 && !owners.any() {
            self.resume_leaked += u32::from(out != MappedInputs::NEUTRAL);
            self.resume_left -= 1;
            if self.resume_left == 0 {
                check = Some(ResumeCheck { delay: self.resume_delay, leaked: self.resume_leaked });
            }
        }
        (out, sticks, check)
    }

    /// The pause flag as the UI tick samples it: an edge begins or ends the pause (a repeated sample
    /// changes nothing). `delay` is the Input Delay setting now (for the resume check).
    pub fn pause_flag(&mut self, mask: &AtomicU8, now: bool, delay: u8) -> Option<PauseEdge> {
        match (self.paused, now) {
            (false, true) => Some(PauseEdge::Begin { dropped: self.pause_begin(mask) }),
            (true, false) => Some(PauseEdge::End(self.pause_end(mask, delay))),
            _ => None,
        }
    }

    pub fn pause_epoch(&self) -> u32 { self.pause_epoch }

    /// Feed only a live native layout callback. Unknown/missing help is not an
    /// invented resume; a new registration explicitly retires its predecessor.
    /// A session-retired observation cannot reacquire ownership afterward.
    pub fn native_pause_sample(&mut self, mask: &AtomicU8, epoch: u32, layout: usize,
        registration: bool, active: Option<bool>, delay: u8) -> Option<[Option<PauseEdge>; 2]> {
        if epoch != self.pause_epoch || layout == 0 { return None; }
        let retired = if registration || self.pause_layout != layout {
            self.pause_epoch = self.pause_epoch.wrapping_add(1);
            self.pause_layout = layout;
            self.pause_flag(mask, false, delay)
        } else { None };
        let observed = active.and_then(|on| self.pause_flag(mask, on, delay));
        Some([retired, observed])
    }

    /// Authoritative scene/session retirement; no native pointer is touched.
    pub fn retire_native_pause(&mut self, mask: &AtomicU8, delay: u8) -> Option<PauseEdge> {
        self.pause_epoch = self.pause_epoch.wrapping_add(1);
        self.pause_layout = 0;
        self.queue.clear(Clear::NativePause);
        self.pause_flag(mask, false, delay)
    }

    /// An inactive old layout may retire only its own observation, never a new one.
    pub fn retire_pause_layout(&mut self, mask: &AtomicU8, epoch: u32, layout: usize, delay: u8) -> bool {
        if epoch != self.pause_epoch || self.pause_layout != layout { return false; }
        self.retire_native_pause(mask, delay);
        true
    }

    /// The game's pause began: take over P1's input and empty the queue now. Returns how many records
    /// it dropped.
    pub fn pause_begin(&mut self, mask: &AtomicU8) -> usize {
        take(mask, Owners::VANILLA_PAUSE);
        self.paused = true;
        let dropped = self.queue.clear(Clear::NativePause);
        self.report = PauseReport { queued_at_begin: dropped, owned_polls: 0, owned_non_neutral: 0 };
        self.resume_left = 0;
        dropped
    }

    /// The game's pause ended: release the input (`delay` is the setting now, for the resume check).
    pub fn pause_end(&mut self, mask: &AtomicU8, delay: u8) -> PauseReport {
        release(mask, Owners::VANILLA_PAUSE);
        self.paused = false;
        self.queue.clear(Clear::NativePause);
        self.resume_delay = delay;
        self.resume_left = delay;
        self.resume_leaked = 0;
        self.report
    }

    /// Empty the queue outside a poll (reset hub, save-state load). Returns how many records it held.
    pub fn clear(&mut self, why: Clear) -> usize {
        self.queue.clear(why)
    }
}
