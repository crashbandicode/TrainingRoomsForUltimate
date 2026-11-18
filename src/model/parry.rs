//! Parry practice (`artifacts/rc15-parry/DESIGN.md` rev 2): the parry attempt detector. One attempt is one hit P1 takes
//! from the CPU while a parry pass plays; the result is one of Tech's own grades (`tech::session::Grade`), so the set,
//! streak, best, labels and summary are Tech's unchanged. Pure: the runtime samples P1 once per game frame.
//!
//! Native facts (13.0.5, `FACTS.md` §4, trace-1): entering GUARD_OFF keeps the shield up as a just-shield for
//! `FIGHTER_STATUS_GUARD_ON_WORK_INT_JUST_FRAME` frames from the entry frame; a hit in that time is a GUARD_DAMAGE with
//! `FIGHTER_STATUS_GUARD_ON_WORK_FLAG_JUST_SHIELD` on (a parry); a hit on a held shield is a GUARD_DAMAGE with it off.

use super::frame_strip_plan::Text;
use super::tech::session::{Grade, LATE};

/// Frames a release may come before or after the hit and still be graded early or late (Tech's `LATE`; operator Q4).
pub const BOUND: u32 = LATE as u32;

/// What the runtime read from P1 on one game frame. `just_shield` is read only during GUARD_DAMAGE and `just_frame` only
/// on GUARD_OFF frames (`None`: the read is unavailable); `armed`: a parry pass plays or ended at most 60 frames ago.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Frame {
    pub stop: bool,
    pub gained: bool,
    pub guard_damage: bool,
    pub guard_off: bool,
    pub just_shield: Option<bool>,
    pub just_frame: Option<i32>,
    pub shield_held: bool,
    pub armed: bool,
}

/// One graded attempt: the grade, the hit frame, and the parry window (first frame, length) when there was one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attempt {
    pub grade: Grade,
    pub hit: u32,
    pub window: Option<(u32, u16)>,
}

/// The detector's state between frames; `pending` is a hit on a held shield waiting up to `BOUND` frames for its release.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Detector {
    /// The strip frame of the last step: a smaller one means the strip restarted (a load), and the old timeline goes.
    last: u32,
    was: (bool, bool, bool),
    release: Option<u32>,
    guard_off: Option<(u32, Option<i32>)>,
    pending: Option<(u32, Option<(u32, u16)>)>,
}

fn frames(n: u32) -> u16 {
    u16::try_from(n.max(1)).unwrap_or(u16::MAX)
}

impl Detector {
    pub const fn new() -> Self {
        Self { last: 0, was: (false, false, false), release: None, guard_off: None, pending: None }
    }

    /// An attempt is still open: the next pass waits for it.
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }

    /// One game frame, `now` being the strip's own frame number for it (`Tracker::newest`), so the window and the hit
    /// are drawn on the strip's frames (tr-review-astra-2, finding 2). At most two attempts end on one frame: an open
    /// late attempt cut by the next hit, and that hit.
    pub fn step(&mut self, f: &Frame, now: u32) -> [Option<Attempt>; 2] {
        if now < self.last {
            // tr-review-astra-2, 703e3f8d finding 2: a load restarted the strip; no release, window or open attempt of
            // the old timeline may grade a hit on the new one.
            *self = Self::new();
        }
        self.last = now;
        let mut out = [None; 2];
        let released = self.was.1 && !f.shield_held;
        if released {
            self.release = Some(now);
        }
        if f.guard_off && !self.was.2 {
            self.guard_off = Some((now, f.just_frame));
        }
        if let Some((h, window)) = self.pending.filter(|(h, _)| released || now.wrapping_sub(*h) >= BOUND) {
            let grade = if released { Grade::Late(frames(now.wrapping_sub(h))) } else { Grade::Missed };
            out[0] = Some(Attempt { grade, hit: h, window });
            self.pending = None;
        }
        if f.armed && f.stop && !self.was.0 && (f.guard_damage || f.gained) {
            if let Some((hit, window)) = self.pending.take() {
                out[0] = Some(Attempt { grade: Grade::Missed, hit, window });
            }
            let recent = self.release.filter(|r| now.wrapping_sub(*r) <= BOUND);
            let window = self.guard_off.and_then(|(g, w)| Some((g, u16::try_from(w?).ok()?)));
            let grade = match (f.guard_damage, f.just_shield, recent, self.guard_off) {
                (true, Some(true), _, _) => Some(Grade::Perfect(recent.map(|r| frames(now.wrapping_sub(r))))),
                (true, Some(false), _, _) => {
                    self.pending = Some((now, window));
                    None
                }
                (true, None, _, _) | (false, _, Some(_), Some((_, None))) => Some(Grade::Unavailable),
                (false, _, Some(r), Some((g, Some(w)))) if g >= r => Some(Grade::Early(frames(now.saturating_sub(g + w.max(1) as u32)))),
                _ => Some(Grade::Missed),
            };
            out[1] = grade.map(|grade| Attempt { grade, hit: now, window });
        }
        self.was = (f.stop, f.shield_held, f.guard_off);
        out
    }
}

/// The strip's view of the practice: the newest attempts first, and the SET readout's value and line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct View {
    pub last: [Option<Attempt>; 4],
    pub set: Text,
    pub set_line: Text,
}

impl View {
    pub const EMPTY: View = View { last: [None; 4], set: Text::EMPTY, set_line: Text::EMPTY };
}

#[cfg(test)]
#[path = "parry_tests.rs"]
mod tests;
