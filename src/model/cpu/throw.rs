//! CPU throws, pummels, fast fall, attack angle, crouch and the full-hop jump button (SPEC-cpu-06).
//! Pure rules over the settings, the body's rolled values (`mash::Rolled`) and the facts the runtime
//! reads. One state per fighter body (decision B6). Decision B11: a changed Throw Options row clears
//! the held throw case at once (`Throw::clear_case`); decision B5 / N4: the throw rule yields to a
//! playing recording unless *Settings override playback* is On (the caller checks it).

use super::command::{CmdFacts, CmdOut};
use super::counter::{CounterId, Counters};
use super::ids::{Table, C, UNRESOLVED};
use super::mash::{BufferContext, Mash};
use super::rng::Rng;
use super::settings::CpuSettings;

/// Indices of the Throw Options row.
pub mod throw_option {
    pub const NONE: u8 = 0;
    pub const FORWARD: u8 = 1;
    pub const BACKWARD: u8 = 2;
    pub const UP: u8 = 3;
    pub const DOWN: u8 = 4;
}

/// One body's throw state: the case and the two delays held for this grab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Throw {
    pub case: Option<u8>,
    pub delay: Option<u16>,
    pub pummel: Option<u16>,
}

impl Throw {
    pub const fn new() -> Self {
        Self { case: None, delay: None, pummel: None }
    }

    /// Decision B11: the held throw case goes; step 3 rolls it again from the new set in the same hold.
    pub fn clear_case(&mut self) {
        self.case = None;
    }

    /// The reset bundle's "reset the throw delay".
    pub fn reset_delay(&mut self) {
        self.delay = None;
    }

    /// The throw and pummel rule for a command query (SPEC-cpu-06 §5 "Throws and pummels"). `cat2` is
    /// true for the category-2 query, on which the throw bits are delivered (question 1).
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        t: &Table,
        s: &CpuSettings,
        rng: &mut Rng,
        counters: &mut Counters,
        mash: &mut Mash,
        ctx: BufferContext,
        cat2: bool,
        f: &CmdFacts,
    ) -> CmdOut {
        let mut out = CmdOut::default();
        let holding = t.is(C::StatusCatchWait, f.status) || t.is(C::StatusCatchPull, f.status) || t.is(C::StatusCatchAttack, f.status);
        // 1. Not holding anyone: forget the grab.
        if !holding {
            *self = Self::new();
            counters.get(CounterId::Throw).full_reset();
            counters.get(CounterId::Pummel).full_reset();
            return out;
        }
        // 2. A throw cannot be input now.
        if !f.cont_throw_hi {
            return out;
        }
        // 3. Roll what is not yet held for this grab.
        let delay = *self.delay.get_or_insert_with(|| s.throw_delay.pick(rng));
        let case = match self.case {
            Some(case) => case,
            None => {
                let pick = s.throw_state.pick(rng).map_or(throw_option::NONE, |i| i as u8);
                self.case = Some(pick);
                pick
            }
        };
        let pummel_on = s.pummel_delay.is_set();
        let pummel = if pummel_on { Some(*self.pummel.get_or_insert_with(|| s.pummel_delay.pick(rng))) } else { None };
        // 4. None: neither throws nor pummels.
        if case == throw_option::NONE {
            return out;
        }
        // 5. Waiting out the throw delay: maybe pummel.
        if counters.get(CounterId::Throw).should_delay(delay) {
            let pummel_waits = pummel.is_some_and(|p| counters.get(CounterId::Pummel).should_delay(p));
            if pummel_waits || pummel.is_none() {
                return out;
            }
            if t.is(C::StatusCatchWait, f.status) {
                out.request_status = Some(t.get(C::StatusCatchAttack)).filter(|v| *v != UNRESOLVED);
            }
            return out;
        }
        // 6. The delay elapsed: the throw, and the mash after it (which ignores the triggers).
        if cat2 {
            let flag = match case {
                throw_option::FORWARD => C::FlagThrowF,
                throw_option::BACKWARD => C::FlagThrowB,
                throw_option::UP => C::FlagThrowHi,
                _ => C::FlagThrowLw,
            };
            let v = t.get(flag);
            out.bits = if v == UNRESOLVED { 0 } else { v as u32 };
            mash.external_buffer(s, rng, ctx, &s.mash_state);
        }
        out
    }
}

/// The fast-fall rule (SPEC-cpu-06 §5 "Fast fall", once per frame): `true` when the flag and the
/// flash effect are to be applied now.
#[allow(clippy::too_many_arguments)]
pub fn fast_fall_frame(
    t: &Table,
    s: &CpuSettings,
    rng: &mut Rng,
    counters: &mut Counters,
    mash: &mut Mash,
    airborne: bool,
    falling: bool,
    status: i32,
    reserve_dive: bool,
    playback_running: bool,
) -> bool {
    if !mash.rolled.fast_fall || !airborne {
        return false;
    }
    if !falling {
        // Rolled at the apex, every frame until the fall begins.
        mash.rolled.fast_fall_delay = s.fast_fall_delay.pick(rng);
        counters.get(CounterId::FastFall).full_reset();
        return false;
    }
    if !(t.is(C::StatusFall, status) || t.is(C::StatusAttackAir, status)) || playback_running || reserve_dive {
        return false;
    }
    !counters.get(CounterId::FastFall).should_delay(mash.rolled.fast_fall_delay)
}

/// The value the attack-angle hook returns (`get_stick_dir`): 1.0 up, -1.0 down, `None` the game's.
pub fn attack_angle_value(mash: &Mash, on_ledge: bool, playback_running: bool) -> Option<f32> {
    if on_ledge || playback_running {
        return None;
    }
    match mash.rolled.attack_angle {
        Some(1) => Some(1.0),
        Some(2) => Some(-1.0),
        _ => None,
    }
}

/// Crouch: the six standing and crouching statuses read a full-down stick.
pub fn crouch_applies(t: &Table, s: &CpuSettings, status: i32) -> bool {
    s.crouch
        && [C::StatusWait, C::StatusSquat, C::StatusSquatB, C::StatusSquatF, C::StatusSquatRv, C::StatusSquatWait]
            .iter()
            .any(|c| t.is(*c, status))
}

/// The full-hop jump button answer: held while the jump is in its squat, released never.
pub fn full_hop_button(t: &Table, mash: &Mash, status: i32, on_query: bool, playback_running: bool) -> Option<bool> {
    (mash.rolled.full_hop && t.is(C::StatusJumpSquat, status) && !playback_running).then_some(on_query)
}
