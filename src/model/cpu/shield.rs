//! CPU shield behaviour (SPEC-cpu-05): holding the shield, the decay override, the number of hits on
//! shield and the reaction time before an out-of-shield action, the out-of-shield decision itself,
//! the shield tilt and the shield damage multiplier. Pure: the button, parameter and stick hooks and
//! the script continuation answer from the settings and the body's state. One state per fighter body
//! (decision B6); the tilt's In / Out are opponent-relative on both sides (decision B7) and a changed
//! Shield Tilt applies at the next CPU frame (decision Q18 A).

use super::counter::{CounterId, Counters};
use super::defence::stick_vector;
use super::ids::{Table, C, UNRESOLVED};
use super::mash::{Action, BufferContext, Mash};
use super::rng::Rng;
use super::settings::{situation, trigger, CpuSettings, ShieldMode};

/// The game's shield damage multiplier is only ever 0.0 (Infinite, in Training) or the value the game
/// booted with; this is the value to write this frame. `cached` is the game's boot value observed on
/// the first look, `None` until then (nothing is written until it is known).
pub fn damage_multiplier(mode: ShieldMode, in_training: bool, cached: Option<f32>) -> Option<f32> {
    if in_training && mode == ShieldMode::Infinite {
        Some(0.0)
    } else {
        cached
    }
}

/// Whether the decay and regeneration parameters are overridden now (SPEC-cpu-05 §5 "Decay override"):
/// Infinite and Constant always; Hold until the first hit on this shield; None never (native decay).
pub const fn decay_override_active(mode: ShieldMode, decay_enabled: bool) -> bool {
    match mode {
        ShieldMode::Infinite | ShieldMode::Constant => true,
        ShieldMode::Hold => !decay_enabled,
        ShieldMode::None => false,
    }
}

/// The values the parameter hook answers while the override is active.
pub const SHIELD_DECAY_OVERRIDE: f32 = 0.0;
pub const SHIELD_RECOVERY_OVERRIDE: f32 = 999.0;

/// What `should_hold` reads from the game and the other features.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HoldFacts {
    pub playback_running: bool,
    pub standby: bool,
    pub save_state_loading: bool,
    pub was_airborne: bool,
    /// The mash queue's current action asks for the shield (`SHIELD`, or `AIR_DODGE` while grounded).
    pub queue_requests_shield: bool,
}

/// "Should hold shield" (SPEC-cpu-05 §5 "Holding the shield").
pub fn should_hold(mode: ShieldMode, f: &HoldFacts) -> bool {
    !f.playback_running
        && !f.standby
        && !f.save_state_loading
        && !f.was_airborne
        && (f.queue_requests_shield || mode != ShieldMode::None)
}

/// Whether the current mash action asks for the shield: `SHIELD`, or `AIR_DODGE` while grounded.
pub fn queue_requests_shield(current: Option<Action>, grounded: bool) -> bool {
    matches!(current, Some(a) if a == Action::SHIELD || (a == Action::AIR_DODGE && grounded))
}

/// The answer of a guard-button query for a CPU body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonAnswer {
    /// No opinion: the original answer (or, for the jump buttons, SPEC-cpu-06's rule).
    Original,
    /// Report the button as held / not held.
    Held(bool),
}

/// The guard-button hook for a CPU body: `on` is the `check_button_on` query, `off` the
/// `check_button_off` one. While the CPU should hold the shield and it is not suspended, `on` reports
/// true and `off` reports false, except when the current mash action needs a native shield drop.
pub fn guard_button(on_query: bool, hold: bool, suspended: bool) -> ButtonAnswer {
    if !hold || suspended {
        return ButtonAnswer::Original;
    }
    ButtonAnswer::Held(on_query)
}

/// What the out-of-shield decision asks (SPEC-cpu-05 §5 "Out of shield").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OosOut {
    /// Change the status directly on the script object: the raw status of the dodge or roll.
    pub escape_status: Option<i32>,
}

/// What the script continuation reads when the body has just left shieldstun.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OosFacts {
    /// The previous status was `GUARD_DAMAGE` (the body left shieldstun this frame).
    pub left_shieldstun: bool,
    /// The body is still shielding.
    pub shielding: bool,
    /// The parry animation is playing.
    pub in_parry: bool,
    /// The transition term `CONT_ESCAPE` is enabled.
    pub cont_escape: bool,
}

/// One body's shield state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Shield {
    /// Hits on shield still to take before an out-of-shield action (the offset).
    pub offset: u16,
    /// Reaction frames rolled on the first frame of the latest shieldstun.
    pub reaction: u16,
    /// The latch "was in shieldstun" (edge detection).
    pub was_in_shieldstun: bool,
    /// The shield is suspended so an out-of-shield action can drop it.
    pub suspended: bool,
    /// Hold: the first hit on this shield has happened, so normal decay applies.
    pub decay_enabled: bool,
    /// The tilt direction held (an index of the Direction list), re-read on a reset and on a row change.
    pub tilt: Option<u8>,
}

impl Shield {
    pub const fn new() -> Self {
        Self { offset: 0, reaction: 0, was_in_shieldstun: false, suspended: false, decay_enabled: false, tilt: None }
    }

    /// Re-read the Shield Tilt row (the reset bundle, and the next CPU frame after the row changed).
    pub fn reread_tilt(&mut self, s: &CpuSettings) {
        self.tilt = s.shield_tilt;
    }

    /// The reset bundle and the session clear: everything back to nothing (the cached damage
    /// multiplier is the caller's and survives).
    pub fn reset_state(&mut self) {
        let tilt = self.tilt;
        *self = Self::new();
        self.tilt = tilt;
    }

    /// The per-frame part (SPEC-cpu-05 §5): the hit offset resets when the body is idle or in hitstun;
    /// the first frame of each shieldstun rolls the reaction delay and takes one hit off the offset; the
    /// Hold decay flag drops whenever the status is not `GUARD`.
    pub fn frame(&mut self, s: &CpuSettings, rng: &mut Rng, idle_or_hitstun: bool, in_shieldstun: bool, status_is_guard: bool) {
        if idle_or_hitstun {
            self.offset = s.oos_offset.pick(rng).saturating_add(1);
        }
        if in_shieldstun {
            if !self.was_in_shieldstun {
                self.was_in_shieldstun = true;
                self.reaction = s.reaction_time.pick(rng);
                self.offset = self.offset.saturating_sub(1);
            }
        } else {
            self.was_in_shieldstun = false;
        }
        if !status_is_guard {
            self.decay_enabled = false;
        }
    }

    /// The script continuation, after the original, for a body that just left shieldstun
    /// (SPEC-cpu-05 §5 "Out of shield"). The Hold flag, the offset and reaction gates, the parry stop,
    /// the shieldstun mash and the current action's effect on the shield all happen here.
    #[allow(clippy::too_many_arguments)]
    pub fn out_of_shield(
        &mut self,
        t: &Table,
        s: &CpuSettings,
        rng: &mut Rng,
        counters: &mut Counters,
        mash: &mut Mash,
        ctx: BufferContext,
        f: &OosFacts,
    ) -> OosOut {
        let mut out = OosOut::default();
        if !f.left_shieldstun {
            return out;
        }
        // 1. Hold: the first hit on this shield has happened.
        if s.shield_state == ShieldMode::Hold {
            self.decay_enabled = true;
        }
        // 2. Out of shield is allowed once the offset is used up.
        if self.offset > 0 {
            return out;
        }
        // 3. No longer shielding: start over.
        if !f.shielding {
            counters.get(CounterId::Reaction).full_reset();
            return out;
        }
        // 4. The reaction time.
        if counters.get(CounterId::Reaction).should_delay(self.reaction) {
            return out;
        }
        // 5. Let a parry resolve.
        if f.in_parry {
            return out;
        }
        // 6. The shieldstun mash.
        if s.trigger_on(trigger::SHIELDSTUN) {
            let row = Some(s.overrides[situation::SHIELDSTUN]).filter(|r| !r.is_empty()).unwrap_or(s.mash_state);
            mash.external_buffer(s, rng, ctx, &row);
        }
        // 7. The current action.
        let holding = s.shield_state != ShieldMode::None;
        let Some(action) = mash.queue.current() else {
            self.suspended = false;
            return out;
        };
        let escape = |c: C| Some(t.get(c)).filter(|v| *v != UNRESOLVED);
        if f.cont_escape && (action == Action::SPOT_DODGE || action == Action::ROLL_F || action == Action::ROLL_B) {
            out.escape_status = escape(match action {
                Action::SPOT_DODGE => C::StatusEscape,
                Action::ROLL_F => C::StatusEscapeF,
                _ => C::StatusEscapeB,
            });
            return out;
        }
        if action == Action::JUMP || action.is_aerial() || action == Action::UP_B || action == Action::U_SMASH {
            // The button-off hook reports the shield released natively, so the game performs it.
            return out;
        }
        if action == Action::AIR_DODGE {
            if holding {
                self.suspended = true;
            }
            return out;
        }
        if action == Action::SHIELD {
            return out;
        }
        // Grab works from the shield (up smash returned above); every other action needs it dropped.
        self.suspended = action != Action::GRAB;
        out
    }

    /// The stick the shield tilt answers for a body whose tilt is held (`None`: the native stick).
    pub fn tilt_stick(&self, player_x: f32, cpu_x: f32) -> Option<(f32, f32)> {
        self.tilt.map(|dir| stick_vector(dir, player_x, cpu_x))
    }
}
