//! CPU ledge options (SPEC-cpu-03): how a CPU hanging on the ledge gets up, after which delay, with
//! which mash afterwards. Pure: the per-frame step reads the body's facts and answers what to request;
//! the transition-term rule answers whether the game's automatic climb is denied. One state per
//! fighter body (decision B6). Decision B11: a changed Ledge Options row clears the held case at once
//! (`Ledge::clear_case`), so the CPU rolls again from the new set in the same hang.

use super::counter::{CounterId, Counters};
use super::facts::{FrameFacts, FrameOut, PlaybackStart};
use super::ids::{Table, C};
use super::mash::{BufferContext, Mash};
use super::rng::Rng;
use super::settings::{situation, trigger, CpuSettings, Weights};
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

/// The row with the lobby filter applied: Playback options are removed in a waiting scene while
/// practice is not enabled.
pub fn effective(row: &Weights<10>, lobby_without_practice: bool) -> Weights<10> {
    let mut out = *row;
    if lobby_without_practice {
        for w in &mut out.0[option::PLAYBACK_FIRST as usize..] {
            *w = 0;
        }
    }
    out
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
}

impl Ledge {
    pub const fn new() -> Self {
        Self { case: None, delay: None, playback_started: false }
    }

    /// Decision B11: drop the held case so the next hanging frame rolls again. The delay and its
    /// counter are not touched.
    pub fn clear_case(&mut self) {
        self.case = None;
        self.playback_started = false;
    }

    /// The reset bundle's "reset the ledge delay" (the case clears by itself through step 1).
    pub fn reset_delay(&mut self) {
        self.delay = None;
    }

    /// The game asks whether `term` is enabled for this body (`WorkModule::is_enable_transition_term`)
    /// and the original answered `original`. SPEC-cpu-03 §4: deny the native automatic climb while the
    /// CPU waits, delays, or waits for its playback to start; otherwise the original answer.
    #[allow(clippy::too_many_arguments)]
    pub fn transition_term(
        &self,
        t: &Table,
        s: &CpuSettings,
        counters: &Counters,
        playback_running: bool,
        status: i32,
        term: i32,
        original: bool,
    ) -> bool {
        let on_ledge_wait = t.is(C::StatusCliffWait, status);
        if !original || !on_ledge_wait || s.ledge_state.is_empty() || !t.is(C::TermContCliffClimb, term) {
            return original;
        }
        let Some(case) = self.case else { return original };
        let hold_for_playback = is_playback(case) && !self.playback_started;
        // While another playback runs the feature yields entirely, except the hold that keeps the CPU
        // on the ledge until its own playback has started.
        let waiting = if playback_running {
            hold_for_playback
        } else {
            case == option::WAIT || counters.count(CounterId::Ledge) < u32::from(self.delay.unwrap_or(0)) || hold_for_playback
        };
        !waiting
    }

    /// One frame of the ledge step (SPEC-cpu-03 §5). `playback_running` is an input playback that is
    /// not this feature's own: the feature yields entirely.
    #[allow(clippy::too_many_arguments)]
    pub fn frame(
        &mut self,
        t: &Table,
        s: &CpuSettings,
        rng: &mut Rng,
        counters: &mut Counters,
        mash: &mut Mash,
        ctx: BufferContext,
        f: &FrameFacts,
    ) -> FrameOut {
        let mut out = FrameOut::default();
        if ctx.playback.running || s.ledge_state.is_empty() {
            return out;
        }
        // 1. Not hanging: forget the grab.
        if f.situation != Situation::Cliff {
            self.case = None;
            self.delay = None;
            self.playback_started = false;
            counters.get(CounterId::Ledge).full_reset();
            return out;
        }
        // 2. Hanging: hold a delay and a case for this grab.
        let delay = *self.delay.get_or_insert_with(|| s.ledge_delay.pick(rng));
        if self.case.is_none() {
            let row = effective(&s.ledge_state, ctx.lobby_without_practice);
            self.case = row.pick(rng).map(|i| i as u8);
            if self.case.is_none() {
                // An empty effective set: the CPU behaves natively this grab, the climb is not denied.
                return out;
            }
        }
        let Some(case) = self.case else { return out };
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
