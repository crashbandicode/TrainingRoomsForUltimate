//! CPU ledge options (SPEC-cpu-03): how a CPU hanging on the ledge gets up, after which delay, with
//! which mash afterwards. Pure: the per-frame step reads the body's facts and answers what to request;
//! the transition-term rule answers whether the game's automatic climb is denied. One state per
//! fighter body (decision B6). Decision B11: a changed Ledge Options row clears the held case at once
//! (`Ledge::clear_case`), so the CPU rolls again from the new set in the same hang.

use super::counter::{CounterId, Counters};
use super::facts::{FrameFacts, FrameOut, PlaybackStart};
use super::ids::{Table, C};
use super::mash::Step;
use super::playback::PlaybackState;
use super::settings::{situation, trigger, Weights};
use super::status::Situation;

/// Indices of the Ledge Options row.
pub mod option {
    pub const NEUTRAL: u8 = 0;
    pub const ROLL: u8 = 1;
    pub const JUMP: u8 = 2;
    pub const ATTACK: u8 = 3;
    pub const WAIT: u8 = 4;
    pub const PLAYBACK_FIRST: u8 = 5;
}

/// Motion frames the spec's tuned buffering uses (SPEC-cpu-03 §5 step 3; to be confirmed in Eden,
/// open question 2).
pub const BUFFER_PLAYBACK_FRAME: i32 = 13;
pub const BUFFER_REGULAR_FRAME: i32 = 19;
pub const BUFFER_TETHER_FRAME: i32 = 18;

fn is_playback(case: u8) -> bool {
    case >= option::PLAYBACK_FIRST
}

/// The transition-term hook's question: the body's status, the term asked about and the game's own answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TermQuery {
    pub status: i32,
    pub term: i32,
    pub original: bool,
}

/// One body's ledge state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ledge {
    /// The option held for this grab.
    pub case: Option<u8>,
    /// The delay held for this grab.
    pub delay: Option<u16>,
    /// The playback of a playback case has been started (the climb stays denied until it is).
    pub playback_started: bool,
    /// This grab rolled an empty effective set: the CPU behaves natively (CPU-091: unlike a case not rolled yet).
    pub native_grab: bool,
    /// CPU-091: hanging frames since the held case could act (its delay over); the bound releases the climb after them.
    pub actionable_frames: u16,
}

/// CPU-091 safety bound: TR requests a performing case on its first actionable frame after the delay (at the latest the
/// buffered motion frame 19) and the CPU leaves CliffWait on the next status change. If it still hangs 30 such frames
/// (half a second) later, the native climb is no longer held back, so a fault can never park the CPU on the ledge.
pub const UNREQUESTED_BOUND: u16 = 30;

impl Ledge {
    pub const fn new() -> Self {
        Self { case: None, delay: None, playback_started: false, native_grab: false, actionable_frames: 0 }
    }

    /// The native climb is still held back for a performing case while the CPU hangs, before and after TR's request (the
    /// game's climb is decided in the same frame and would win, p3f-verify-yoga-3), within the bound.
    pub fn awaits_request(&self) -> bool {
        self.case.is_some_and(|case| case <= option::ATTACK) && self.actionable_frames < UNREQUESTED_BOUND
    }

    /// The bound released the native climb for a performing case that never took (the runtime logs it).
    pub fn bound_released(&self) -> bool {
        self.case.is_some_and(|case| case <= option::ATTACK) && self.actionable_frames >= UNREQUESTED_BOUND
    }

    /// Decision B11: drop the held case so the next hanging frame rolls again. The delay and its
    /// counter are not touched.
    pub fn clear_case(&mut self) {
        self.case = None;
        self.playback_started = false;
        self.native_grab = false;
        self.actionable_frames = 0;
    }

    /// The reset bundle's "reset the ledge delay" (the case clears by itself through step 1).
    pub fn reset_delay(&mut self) {
        self.delay = None;
    }

    /// The game's question `q` for this body (`WorkModule::is_enable_transition_term`). SPEC-cpu-03 §4: deny the
    /// native automatic climb while the CPU waits, delays, or waits for its playback to start, and (CPU-091,
    /// DECISIONS B11a) while TR holds a case it performs itself, before and after its request, for at most
    /// `UNREQUESTED_BOUND` frames; otherwise the original answer.
    pub fn transition_term(&self, t: &Table, options: Weights<10>, counters: &Counters, playback: PlaybackState, q: TermQuery) -> bool {
        let TermQuery { status, term, original } = q;
        let on_ledge_wait = t.is(C::StatusCliffWait, status);
        if !original || !on_ledge_wait || options.is_empty() || !t.is(C::TermContCliffClimb, term) {
            return original;
        }
        // CPU-091: no case yet (B11 cleared it at the menu change) holds the climb until the next frame rolls.
        // Another playback yields the feature entirely (its step does not roll), as before.
        let Some(case) = self.case else { return (self.native_grab || playback.running) && original };
        let hold_for_playback = is_playback(case) && (!self.playback_started || playback.owed);
        // While another playback runs the feature yields entirely, except the hold that keeps the CPU
        // on the ledge until its own playback has started.
        let waiting = if playback.running {
            hold_for_playback
        } else {
            case == option::WAIT
                || counters.count(CounterId::Ledge) < u32::from(self.delay.unwrap_or(0))
                || hold_for_playback
                || self.awaits_request()
        };
        !waiting
    }

    /// One frame of the ledge step (SPEC-cpu-03 §5). `playback_running` is an input playback that is
    /// not this feature's own: the feature yields entirely.
    pub fn frame(&mut self, step: Step, f: &FrameFacts) -> FrameOut {
        let Step { t, s, rng, counters, mash, ctx } = step;
        let mut out = FrameOut::default();
        if ctx.playback.running || s.ledge_state.is_empty() {
            return out;
        }
        // 1. Not hanging: forget the grab.
        if f.situation != Situation::Cliff {
            *self = Self::new();
            counters.get(CounterId::Ledge).full_reset();
            return out;
        }
        // 2. Hanging: hold a delay and a case for this grab.
        let delay = *self.delay.get_or_insert_with(|| s.ledge_delay.pick(rng));
        if self.case.is_none() {
            self.case = s.ledge_state.pick(rng).map(|i| i as u8);
            self.native_grab = self.case.is_none();
            if self.native_grab {
                // An empty effective set: the CPU behaves natively this grab, the climb is not denied.
                return out;
            }
        }
        let Some(case) = self.case else { return out };
        // CPU-091 bound: count the frames on which TR could already act on its case without having requested it.
        if counters.count(CounterId::Ledge) >= u32::from(delay) {
            self.actionable_frames = self.actionable_frames.saturating_add(1);
        }
        // 3. The buffered frames.
        let after_catch = t.is(C::StatusCliffCatch, f.prev_status);
        let waiting_status = t.is(C::StatusCliffWait, f.status);
        let buffer_playback_now = delay == 0 && f.motion_frame == BUFFER_PLAYBACK_FRAME;
        let buffer_now = if waiting_status && after_catch {
            delay == 0 && f.motion_frame == BUFFER_REGULAR_FRAME && !f.catch_cliff
        } else if waiting_status {
            delay == 0 && f.motion_frame == BUFFER_TETHER_FRAME && f.catch_cliff
        } else {
            false
        };
        // 4. Cannot act yet.
        if !f.cont_cliff_attack {
            if buffer_playback_now && is_playback(case) && s.ledge_delay.is_set() {
                self.playback_started = true;
                out.playback = Some(PlaybackStart {
                    slot: case - option::PLAYBACK_FIRST,
                    ledge: true,
                });
                return out;
            }
            if !buffer_now {
                return out;
            }
        }
        // 5. Wait keeps hanging.
        if case == option::WAIT {
            return out;
        }
        // 6. The delay counts only from actionability (or from the buffered frame).
        if counters.get(CounterId::Ledge).should_delay(delay) {
            return out;
        }
        // 7. Perform.
        let (status, over) = match case {
            option::NEUTRAL => (Some(C::StatusCliffClimb), Some(situation::LEDGE_NEUTRAL)),
            option::ROLL => (Some(C::StatusCliffEscape), Some(situation::LEDGE_ROLL)),
            option::JUMP => (Some(C::StatusCliffJump1), Some(situation::LEDGE_JUMP)),
            option::ATTACK => (Some(C::StatusCliffAttack), Some(situation::LEDGE_ATTACK)),
            _ => (None, None),
        };
        match status {
            Some(c) => out.request_status = Some(t.get(c)).filter(|v| *v != super::ids::UNRESOLVED),
            None => {
                self.playback_started = true;
                out.playback = Some(PlaybackStart { slot: case - option::PLAYBACK_FIRST, ledge: false });
            }
        }
        // 8. The mash after the getup: the override row of the performed kind if set, else the mash
        // action when the Ledge Option trigger is ticked, else nothing.
        let row = over.map(|i| s.overrides[i]).filter(|r| !r.is_empty());
        let row = row.or_else(|| s.trigger_on(trigger::LEDGE_OPTION).then_some(s.mash_state));
        if let Some(row) = row {
            mash.external_buffer(s, rng, ctx, &row);
        }
        out
    }
}
