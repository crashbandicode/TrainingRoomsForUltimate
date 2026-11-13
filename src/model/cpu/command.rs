//! The mash run (SPEC-cpu-01 §5 steps 1, 3, 4) and the throw, aerial, fast-fall, angle and crouch rules
//! of SPEC-cpu-06: what the CPU "presses" on each category-1 / category-2 command query. Pure: the
//! runtime gathers `CmdFacts`, calls `Cpu::command_flags` and ORs the answer into the game's result.
//! Decision B8 (question 1 of SPEC-cpu-01): the action is rolled once per situation entry, not on every
//! frame of the situation.

use super::counter::{CounterId, Counters};
use super::ids::{Table, C, UNRESOLVED};
use super::mash::{Action, BufferContext, Buffered, Mash};
use super::rng::Rng;
use super::settings::{situation, trigger, CpuSettings, Weights};
use super::status::{self, Situation};

/// What the runtime reads for one command query of a CPU body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CmdFacts {
    pub status: i32,
    pub situation: Situation,
    pub motion_frame: i32,
    /// The motion kind is `dash`.
    pub motion_is_dash: bool,
    /// The motion kind is `just_shield_off` (a parry).
    pub in_parry: bool,
    pub invalid_catch_frame: i32,
    pub cont_dash: bool,
    pub cont_catch_dash: bool,
    pub cont_attack_air: bool,
    pub cont_throw_hi: bool,
    /// The fighter kind.
    pub kind: i32,
    /// Gravity speed y is negative.
    pub falling: bool,
    /// 3-D distance to the player entry; infinity without one.
    pub distance: f32,
    pub prev_status: i32,
}

impl Default for CmdFacts {
    fn default() -> Self {
        Self {
            status: 0,
            situation: Situation::Other,
            motion_frame: 0,
            motion_is_dash: false,
            in_parry: false,
            invalid_catch_frame: 0,
            cont_dash: false,
            cont_catch_dash: false,
            cont_attack_air: false,
            cont_throw_hi: false,
            kind: UNRESOLVED,
            falling: false,
            distance: f32::INFINITY,
            prev_status: 0,
        }
    }
}

/// What a command query answers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CmdOut {
    /// Command flag values to OR into the game's answer (the category the query asked for).
    pub bits: u32,
    /// `change_status_request_from_script`: a dash, a dash attack, or a pummel.
    pub request_status: Option<i32>,
    /// Playback Mash Interrupt: stop the running playback.
    pub stop_playback: bool,
}

/// Which trigger situation last buffered a mash (so a persisting situation does not roll again).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situ {
    Tech,
    Clatter,
    Tumble,
    Hitstun,
    Parry,
    Footstool,
    Trump,
    Landing,
    Generic,
}

fn flag(t: &Table, c: C) -> u32 {
    let v = t.get(c);
    if v == UNRESOLVED {
        0
    } else {
        v as u32
    }
}

fn row_for(s: &CpuSettings, over: usize, trigger_on: bool) -> Option<Weights<30>> {
    let o = s.overrides[over];
    if !o.is_empty() {
        Some(o)
    } else if trigger_on && !s.mash_state.is_empty() {
        Some(s.mash_state)
    } else {
        None
    }
}

/// The situation that applies now and the row to pick from (SPEC-cpu-01 §5 step 1, in priority order).
pub fn pick_situation(t: &Table, s: &CpuSettings, f: &CmdFacts) -> Option<(Situ, Weights<30>)> {
    let st = f.status;
    let try_row = |situ: Situ, over: usize, trig: u8| row_for(s, over, s.trigger_on(trig)).map(|r| (situ, r));
    if status::in_tech(t, st) {
        return try_row(Situ::Tech, situation::TECH_ACTION, trigger::TECH_OPTION);
    }
    if status::in_clatter(t, st) {
        return try_row(Situ::Clatter, situation::CLATTER, trigger::CLATTER);
    }
    if status::in_tumble(t, st) {
        return try_row(Situ::Tumble, situation::TUMBLE, trigger::TUMBLE);
    }
    if status::in_hitstun(t, st) {
        return try_row(Situ::Hitstun, situation::HITSTUN, trigger::HITSTUN);
    }
    if f.in_parry {
        return try_row(Situ::Parry, situation::PARRY, trigger::PARRY);
    }
    if status::in_footstool(t, st) {
        return try_row(Situ::Footstool, situation::FOOTSTOOL, trigger::FOOTSTOOL);
    }
    if status::in_ledge_trump(t, st) {
        return try_row(Situ::Trump, situation::TRUMP, trigger::LEDGE_TRUMP);
    }
    if status::in_landing(t, st) {
        return try_row(Situ::Landing, situation::LANDING, trigger::LANDING);
    }
    let generic = (s.trigger_on(trigger::GROUNDED) && f.situation == Situation::Ground)
        || (s.trigger_on(trigger::AIRBORNE) && f.situation == Situation::Air)
        || (s.trigger_on(trigger::DISTANCE_CLOSE) && f.distance < 16.0)
        || (s.trigger_on(trigger::DISTANCE_MID) && f.distance < 37.0)
        || (s.trigger_on(trigger::DISTANCE_FAR) && f.distance < 64.0)
        || s.trigger_on(trigger::ALWAYS);
    (generic && !s.mash_state.is_empty()).then_some((Situ::Generic, s.mash_state))
}

/// The character-specific completion table (SPEC-cpu-01 §4): statuses that count as performing `expected`
/// for one fighter. Data-driven: a row per (kind, expected status, accepted statuses).
fn character_completes(t: &Table, f: &CmdFacts, expected: C) -> bool {
    let is = |c: C| t.is(c, f.status);
    let kind = |c: C| t.is(c, f.kind);
    match expected {
        C::StatusSpecialHi if kind(C::KindKoopa) => is(C::KoopaSpecialHiG) || is(C::KoopaSpecialHiA),
        C::StatusSpecialN if kind(C::KindTrail) => {
            is(C::TrailSpecialN1) || is(C::TrailSpecialN1Shoot) || is(C::TrailSpecialN1End) || is(C::TrailSpecialN2) || is(C::TrailSpecialN3)
        }
        C::StatusAttackAir if kind(C::KindTrail) => is(C::TrailAttackAirN) || is(C::TrailAttackAirF),
        C::StatusSpecialS if kind(C::KindKrool) => {
            is(C::KroolSpecialSThrow) || is(C::KroolSpecialSCatch) || is(C::KroolSpecialSFailure) || is(C::KroolSpecialSGet)
        }
        // Kirby (fork fix, required): every copied neutral-special status completes a Neutral Special.
        C::StatusSpecialN if kind(C::KindKirby) => t.below(C::KirbyMiiSpecialN, C::KirbyNum, f.status),
        _ => false,
    }
}

/// The aerial command kind of an aerial action.
pub fn aerial_kind(a: Action) -> Option<C> {
    Some(match a {
        Action::NAIR => C::AirKindN,
        Action::FAIR => C::AirKindF,
        Action::BAIR => C::AirKindB,
        Action::UAIR => C::AirKindHi,
        Action::DAIR => C::AirKindLw,
        _ => return None,
    })
}

impl Mash {
    /// Step 1: choose what to buffer. Returns `StopPlayback` through `CmdOut` when Playback Mash
    /// Interrupt asks for it. Skipped while a save state loads.
    pub fn check_buffer(
        &mut self,
        t: &Table,
        s: &CpuSettings,
        rng: &mut Rng,
        ctx: BufferContext,
        f: &CmdFacts,
        save_loading: bool,
    ) -> CmdOut {
        let mut out = CmdOut::default();
        if save_loading {
            return out;
        }
        // Beginning a dash attack: a new command could cancel it into a roll or a grab.
        let dash_attack_begins = t.is(C::StatusDash, f.status)
            || (t.is(C::StatusAttackDash, f.status) && (f.cont_catch_dash || f.motion_frame <= 2));
        if dash_attack_begins && self.queue.current() == Some(Action::DASH_ATTACK) {
            return out;
        }
        let Some((situ, row)) = pick_situation(t, s, f) else {
            self.latch = None;
            return out;
        };
        if self.latch == Some(situ) {
            return out;
        }
        self.latch = Some(situ);
        if let Buffered::StopPlayback = self.external_buffer(s, rng, ctx, &row) {
            out.stop_playback = true;
            // The mash triggers again on a later frame, once the playback has stopped.
            self.latch = None;
        }
        out
    }

    /// Steps 3 and 4: the dash latch, then the current action's command bits and its completion.
    pub fn perform(&mut self, t: &Table, s: &CpuSettings, counters: &mut Counters, f: &CmdFacts) -> CmdOut {
        let mut out = CmdOut::default();
        let in_dash = t.is(C::StatusDash, f.status);
        if !(self.queue.current() == Some(Action::DASH_ATTACK) && in_dash) {
            self.transitioning_dash = false;
        }
        let Some(action) = self.queue.current() else {
            // The angle only applies to mash-driven attacks (decision B8): it holds while the body is still in the
            // status the attack completed in (the status reads it after this query), and goes once it leaves.
            if self.rolled.angle_status != Some(f.status) {
                self.rolled.attack_angle = None;
                self.rolled.angle_status = None;
            }
            return out;
        };
        let grounded = f.situation == Situation::Ground;
        let airborne = f.situation == Situation::Air;
        let (bits, expected): (u32, Option<C>) = match action {
            Action::AIR_DODGE if airborne => (flag(t, C::FlagAirEscape), Some(C::StatusEscapeAir)),
            Action::AIR_DODGE => (flag(t, C::FlagAirEscape), Some(C::StatusGuardOn)),
            Action::SHIELD if grounded => (flag(t, C::FlagAirEscape), Some(C::StatusGuardOn)),
            Action::SHIELD => return out, // airborne: nothing, and it stays queued
            Action::JUMP => {
                let expected = if grounded {
                    C::StatusJumpSquat
                } else if airborne {
                    C::StatusJumpAerial
                } else {
                    C::StatusJump
                };
                (flag(t, C::FlagJumpButton), Some(expected))
            }
            Action::SPOT_DODGE => (flag(t, C::FlagEscape), Some(C::StatusEscape)),
            Action::ROLL_F => (flag(t, C::FlagEscapeF), Some(C::StatusEscapeF)),
            Action::ROLL_B => (flag(t, C::FlagEscapeB), Some(C::StatusEscapeB)),
            Action::DASH => {
                if f.cont_dash && !in_dash {
                    out.request_status = Some(t.get(C::StatusDash)).filter(|v| *v != UNRESOLVED);
                }
                (0, Some(C::StatusDash))
            }
            Action::DASH_ATTACK => {
                if !in_dash && !t.is(C::StatusAttackDash, f.status) {
                    if f.cont_dash {
                        out.request_status = Some(t.get(C::StatusDash)).filter(|v| *v != UNRESOLVED);
                    }
                    return out;
                }
                if in_dash && f.motion_frame == 0 && f.motion_is_dash {
                    if self.transitioning_dash {
                        out.request_status = Some(t.get(C::StatusAttackDash)).filter(|v| *v != UNRESOLVED);
                    } else {
                        self.transitioning_dash = true;
                    }
                }
                (0, Some(C::StatusAttackDash))
            }
            a if a.is_aerial() => {
                let mut bits = 0;
                if grounded {
                    bits |= flag(t, C::FlagJumpButton);
                    if self.rolled.full_hop {
                        return CmdOut { bits, ..out };
                    }
                }
                let in_air_attack = t.is(C::StatusAttackAir, f.status);
                let waiting_fall = self.rolled.falling_aerial && !f.falling;
                let delay = self.rolled.aerial_delay;
                let waiting_delay = delay != 0 && !in_air_attack && (!f.cont_attack_air || counters.get(CounterId::Aerial).should_delay(delay));
                if waiting_fall || waiting_delay {
                    return CmdOut { bits, ..out };
                }
                if delay != 0 {
                    counters.get(CounterId::Aerial).full_reset();
                }
                self.queued_aerial = Some(a);
                bits |= flag(t, C::FlagAttackN);
                (bits, Some(C::StatusAttackAir))
            }
            Action::NEUTRAL_B => (flag(t, C::FlagSpecialN), Some(C::StatusSpecialN)),
            Action::SIDE_B => (flag(t, C::FlagSpecialS), Some(C::StatusSpecialS)),
            Action::UP_B => (flag(t, C::FlagSpecialHi), Some(C::StatusSpecialHi)),
            Action::DOWN_B => (flag(t, C::FlagSpecialLw), Some(C::StatusSpecialLw)),
            Action::U_SMASH => (flag(t, C::FlagAttackHi4), Some(C::StatusAttackHi4Start)),
            Action::F_SMASH => (flag(t, C::FlagAttackS4), Some(C::StatusAttackS4Start)),
            Action::D_SMASH => (flag(t, C::FlagAttackLw4), Some(C::StatusAttackLw4Start)),
            Action::GRAB => (if f.invalid_catch_frame != 0 { 0 } else { flag(t, C::FlagCatch) }, Some(C::StatusCatch)),
            Action::JAB => (if grounded { flag(t, C::FlagAttackN) } else { 0 }, Some(C::StatusAttack)),
            Action::F_TILT => (flag(t, C::FlagAttackS3), Some(C::StatusAttackS3)),
            Action::U_TILT => (flag(t, C::FlagAttackHi3), Some(C::StatusAttackHi3)),
            Action::D_TILT => (flag(t, C::FlagAttackLw3), Some(C::StatusAttackLw3)),
            _ => return out, // a playback action produces no bits
        };
        // Completion: the expected status, or the character-specific table.
        if let Some(expected) = expected {
            if t.is(expected, f.status) || character_completes(t, f, expected) {
                self.complete();
                self.rolled.angle_status = self.rolled.attack_angle.map(|_| f.status);
                let _ = s;
            }
        }
        // Never interrupt a dash with command bits.
        if in_dash {
            return out;
        }
        out.bits = bits;
        out
    }
}
