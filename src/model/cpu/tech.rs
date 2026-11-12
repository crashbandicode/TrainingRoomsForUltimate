//! CPU tech, missed-tech options and hidden tech animations (SPEC-cpu-04). Pure: the status-change
//! rule (the script hook), the motion substitution, the missed-tech getup, the hide-tech lifecycle,
//! the quake rule and the fixed-camera table answer from the settings and the body's facts.
//! One state per fighter body (decision B6); the hidden-body record is the primary CPU body's only.
//! Decisions B5 / N4 (the recording wins by default) and B8 (Mistech Options no longer depend on Tech
//! Options) apply.

use super::facts::FrameFacts;
use super::hash::hash40;
use super::ids::{Table, C, UNRESOLVED};
use super::rng::Rng;
use super::settings::CpuSettings;

/// Indices of the Tech Options row.
pub mod tech_option {
    pub const NO_TECH: usize = 0;
    pub const ROLL_F: usize = 1;
    pub const ROLL_B: usize = 2;
    pub const IN_PLACE: usize = 3;
}

/// Indices of the Mistech Options row.
pub mod mistech_option {
    pub const GETUP: usize = 0;
    pub const ATTACK: usize = 1;
    pub const ROLL_F: usize = 2;
}

/// A roll direction relative to the body's facing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Roll {
    Forward,
    Backward,
}

/// The hashes of the animations the motion hook substitutes (`phx::Hash40` of the names).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionHashes {
    pub passive_stand_f: u64,
    pub passive_stand_b: u64,
    pub down_forward_u: u64,
    pub down_back_u: u64,
    pub down_forward_d: u64,
    pub down_back_d: u64,
}

impl MotionHashes {
    pub fn new() -> Self {
        Self {
            passive_stand_f: hash40("passive_stand_f"),
            passive_stand_b: hash40("passive_stand_b"),
            down_forward_u: hash40("down_forward_u"),
            down_back_u: hash40("down_back_u"),
            down_forward_d: hash40("down_forward_d"),
            down_back_d: hash40("down_back_d"),
        }
    }
}

impl Default for MotionHashes {
    fn default() -> Self {
        Self::new()
    }
}

/// What the script's status change asks and what the game allows (SPEC-cpu-04 §3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TechRequest {
    /// The status the script is about to change to.
    pub requested: i32,
    /// `StatusModule::prev_status_kind(accessor, 1)`: the second-previous status.
    pub second_prev: i32,
    /// The transition terms `PASSIVE`, `PASSIVE_WALL`, `PASSIVE_CEIL` are enabled.
    pub passive: bool,
    pub passive_wall: bool,
    pub passive_ceil: bool,
    /// The script's own "is passive enabled" query.
    pub script_passive_enabled: bool,
}

/// The answer of the script status-change hook.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TechChange {
    /// Leave the request unchanged (a missed tech, or nothing to decide).
    Unchanged,
    /// Request this status instead, with the second argument true.
    Status(i32),
}

/// What a body's frame asks for the hidden tech animation (all `None` / false when nothing).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HideOut {
    /// `WorkModule::set_float(cursor offset)`.
    pub cursor: Option<f32>,
    /// `VisibilityModule::set_whole`.
    pub model_visible: Option<bool>,
    /// `EffectModule::set_visible_kind(.., false)` for the five tech effects.
    pub hide_effects: bool,
}

/// The primary body's hidden record: the cursor offset to write back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hidden {
    pub entry: u8,
    pub saved_cursor: f32,
}

/// The cursor offset that moves the player's cursor off screen.
pub const HIDDEN_CURSOR_OFFSET: f32 = 800.0;
/// Motion frame from which the model is hidden, and the last frames that show it again.
pub const HIDE_FROM_FRAME: i32 = 6;
pub const SHOW_LAST_FRAMES: i32 = 5;

/// One body's tech state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tech {
    /// The roll direction held by the last tech roll pick.
    pub tech_roll: Option<Roll>,
    /// The roll direction held by the last missed-tech roll pick.
    pub miss_roll: Option<Roll>,
}

impl Tech {
    pub const fn new() -> Self {
        Self { tech_roll: None, miss_roll: None }
    }

    /// The script is about to change this body's status (SPEC-cpu-04 §5 "Tech"). Rolls only when one
    /// of the three situations applies. Returns the change and whether a tech happened (the caller then
    /// buffers the Tech Option mash).
    pub fn status_change(&mut self, t: &Table, s: &CpuSettings, rng: &mut Rng, r: &TechRequest) -> (TechChange, bool) {
        let ground = t.is(C::StatusDown, r.requested) || t.is(C::StatusDamageFlyReflectD, r.requested);
        let wall = t.is(C::StatusStopWall, r.requested) || t.is(C::StatusDamageFlyReflectLr, r.requested);
        let ceiling = t.is(C::StatusStopCeil, r.requested) || t.is(C::StatusDamageFlyReflectU, r.requested);
        if s.tech_state.is_empty() || !(ground || wall || ceiling) {
            return (TechChange::Unchanged, false);
        }
        let pick = s.tech_state.pick(rng);
        let status = |c: C| Some(t.get(c)).filter(|v| *v != UNRESOLVED);
        if ground {
            // A tech needs the PASSIVE term, the script's own check, and not Ganondorf's aerial side
            // special slam (the second-previous status says which of his two it was).
            let can = r.passive && r.script_passive_enabled && !t.is(C::ClatterCatchedAirFallGanon, r.second_prev);
            if !can {
                return (TechChange::Unchanged, false);
            }
            let change = match pick {
                Some(tech_option::IN_PLACE) => status(C::StatusPassive),
                Some(tech_option::ROLL_F) => {
                    self.tech_roll = Some(Roll::Forward);
                    status(C::StatusPassiveFb)
                }
                Some(tech_option::ROLL_B) => {
                    self.tech_roll = Some(Roll::Backward);
                    status(C::StatusPassiveFb)
                }
                _ => None,
            };
            return change.map_or((TechChange::Unchanged, false), |v| (TechChange::Status(v), true));
        }
        if pick == Some(tech_option::NO_TECH) || pick.is_none() {
            return (TechChange::Unchanged, false);
        }
        let change = if wall {
            match pick {
                Some(tech_option::IN_PLACE) if r.passive_wall && r.script_passive_enabled => status(C::StatusPassiveWall),
                Some(tech_option::ROLL_F) if r.passive_wall && r.script_passive_enabled => status(C::StatusPassiveWallJump),
                _ => None,
            }
        } else if r.passive_ceil && r.script_passive_enabled {
            status(C::StatusPassiveCeil)
        } else {
            None
        };
        change.map_or((TechChange::Unchanged, false), |v| (TechChange::Status(v), true))
    }

    /// The motion hook (SPEC-cpu-04 §5 "Motion"): the animation to play instead of `hash`, following
    /// the held roll directions. Tech animations follow Tech Options, getup animations Mistech Options.
    pub fn substitute_motion(&self, m: &MotionHashes, s: &CpuSettings, hash: u64) -> u64 {
        if !s.tech_state.is_empty() && (hash == m.passive_stand_f || hash == m.passive_stand_b) {
            return match self.tech_roll {
                Some(Roll::Forward) => m.passive_stand_f,
                Some(Roll::Backward) => m.passive_stand_b,
                None => hash,
            };
        }
        if !s.miss_tech_state.is_empty() {
            let up = hash == m.down_forward_u || hash == m.down_back_u;
            let down = hash == m.down_forward_d || hash == m.down_back_d;
            if up || down {
                return match (self.miss_roll, up) {
                    (Some(Roll::Forward), true) => m.down_forward_u,
                    (Some(Roll::Backward), true) => m.down_back_u,
                    (Some(Roll::Forward), false) => m.down_forward_d,
                    (Some(Roll::Backward), false) => m.down_back_d,
                    (None, _) => hash,
                };
            }
        }
        hash
    }

    /// One frame of the missed-tech rule: the status to force, if any. `lay_down` is the lock-out
    /// state (see `laydown_lockout`) as "the counter says wait". The caller resets the lay-down counter
    /// when the body is in none of the wait statuses.
    pub fn mistech(&mut self, t: &Table, s: &CpuSettings, rng: &mut Rng, status: i32, lay_down_waiting: bool) -> Option<i32> {
        if s.miss_tech_state.is_empty() {
            return None;
        }
        let down_wait = t.is(C::StatusDownWait, status) || t.is(C::StatusDownWaitContinue, status);
        let lay_down = t.is(C::StatusLayDown, status);
        let slip = t.is(C::StatusSlipWait, status);
        if !(down_wait || slip || (lay_down && !lay_down_waiting)) {
            return None;
        }
        let pick = s.miss_tech_state.pick(rng)?;
        let chosen = if slip {
            match pick {
                mistech_option::GETUP => C::StatusSlipStand,
                mistech_option::ATTACK => C::StatusSlipStandAttack,
                mistech_option::ROLL_F => C::StatusSlipStandF,
                _ => C::StatusSlipStandB,
            }
        } else {
            match pick {
                mistech_option::GETUP => C::StatusDownStand,
                mistech_option::ATTACK => C::StatusDownStandAttack,
                mistech_option::ROLL_F => {
                    self.miss_roll = Some(Roll::Forward);
                    C::StatusDownStandFb
                }
                _ => {
                    self.miss_roll = Some(Roll::Backward);
                    C::StatusDownStandFb
                }
            }
        };
        Some(t.get(chosen)).filter(|v| *v != UNRESOLVED)
    }

    /// Whether the body is in a status the mistech rule waits in (the lay-down counter resets in any
    /// other status).
    pub fn in_wait_status(t: &Table, status: i32) -> bool {
        t.is(C::StatusLayDown, status)
    }
}

/// The lock-out of Snake's lay-down in whole frames: `min(base + damage / damage_max * (max - base),
/// max)`, truncated (SPEC-cpu-04 §5 "Mistech").
pub fn laydown_lockout(base: f32, max: f32, damage_max: f32, damage: f32) -> u16 {
    let ratio = if damage_max > 0.0 { damage / damage_max } else { 0.0 };
    let frames = (base + ratio * (max - base)).min(max);
    if frames <= 0.0 {
        0
    } else {
        frames as u16
    }
}

/// The hide-tech frame rule for the primary body (SPEC-cpu-04 §5 "Hide tech"). `hidden` is the record
/// of the body hidden last frame; `f` gives the status, the motion frame, the end frame and the cursor
/// offset. Returns what to apply and the new record.
pub fn hide_frame(t: &Table, hide_on: bool, entry: u8, f: &FrameFacts, hidden: Option<Hidden>) -> (HideOut, Option<Hidden>) {
    let (status, motion_frame, end_frame, cursor_offset) = (f.status, f.motion_frame, f.tech.end_frame, f.tech.cursor_offset);
    let in_status = t.is(C::StatusDown, status) || t.is(C::StatusPassive, status) || t.is(C::StatusPassiveFb, status);
    let mut out = HideOut::default();
    if hide_on && in_status {
        let record = hidden.unwrap_or(Hidden { entry, saved_cursor: cursor_offset });
        out.cursor = Some(HIDDEN_CURSOR_OFFSET);
        if motion_frame >= HIDE_FROM_FRAME {
            out.model_visible = Some(false);
            out.hide_effects = true;
        }
        if end_frame - motion_frame <= SHOW_LAST_FRAMES {
            out.model_visible = Some(true);
        }
        return (out, Some(record));
    }
    // Restore: the toggle is Off, or the status is none of the three, or practice ended.
    if let Some(record) = hidden {
        out.model_visible = Some(true);
        out.cursor = Some(record.saved_cursor);
    }
    (out, None)
}

/// The miss-tech camera shake rule (SPEC-cpu-04 §4): suppressed for a CPU body in status `DOWN` while
/// Hide Tech Animations is On.
pub fn quake_suppressed(t: &Table, s: &CpuSettings, status: i32) -> bool {
    s.tech_hide && t.is(C::StatusDown, status)
}

/// The stages whose fixed camera the hide-tech rule re-centres (SPEC-cpu-04 §5 "Camera").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageClass {
    /// Final Destination, Battlefield, Small Battlefield, Town and City, Pokemon Stadium 2, and every
    /// Omega and Battlefield form of every stage.
    Standard,
    /// Smashville.
    Smashville,
    /// Hollow Bastion itself (its Omega and Battlefield forms are `Standard`).
    HollowBastion,
    /// No value: the game's own centre stays.
    Other,
}

/// The fixed-camera centre for a stage, `None` when the game's value stays.
pub const fn camera_centre(class: StageClass) -> Option<[f32; 3]> {
    match class {
        StageClass::Standard => Some([0.0, 50.0, 230.0]),
        StageClass::Smashville => Some([0.0, 35.0, 350.0]),
        StageClass::HollowBastion => Some([0.0, 50.0, 210.0]),
        StageClass::Other => None,
    }
}

/// The native stage ids the camera rule names (the game's own `StageID` numbers: the index into the form table at
/// `main+0x454a7e8`, whose record holds the stage's name hash; `tests/cpu_sites.rs` checks each against its name).
pub mod stage {
    pub const BATTLEFIELD: i32 = 0;
    pub const FINAL_DESTINATION: i32 = 3;
    pub const SMASHVILLE: i32 = 95;
    pub const POKEMON_STADIUM_2: i32 = 107;
    pub const TOWN_AND_CITY: i32 = 257;
    pub const SMALL_BATTLEFIELD: i32 = 347;
    pub const HOLLOW_BASTION: i32 = 361;
    /// Records in the form table (ids `0..STAGE_COUNT`).
    pub const STAGE_COUNT: i32 = 364;
}

/// The camera class of a native stage id. `form` is the record's form field: 0 the stage itself, 1 its Omega form,
/// 2 its Battlefield form (read from the game's table, so no list of forms is written down here); `None` for an
/// id outside the table.
pub const fn stage_class(stage_id: i32, form: Option<u32>) -> StageClass {
    let Some(form) = form else { return StageClass::Other };
    if form != 0 {
        return StageClass::Standard;
    }
    match stage_id {
        stage::BATTLEFIELD | stage::FINAL_DESTINATION | stage::SMALL_BATTLEFIELD | stage::TOWN_AND_CITY | stage::POKEMON_STADIUM_2 => {
            StageClass::Standard
        }
        stage::SMASHVILLE => StageClass::Smashville,
        stage::HOLLOW_BASTION => StageClass::HollowBastion,
        _ => StageClass::Other,
    }
}

/// The camera mode that is the Training fixed camera.
pub const CAMERA_MODE_FIXED: i32 = 4;

/// The centre to put in the fixed camera: the stage's value while Hide Tech Animations is On, the stage default the
/// game set last when it is Off (`None`: the game's value stays).
pub fn fixed_camera_centre(s: &CpuSettings, class: StageClass, stage_default: Option<[f32; 3]>) -> Option<[f32; 3]> {
    if s.tech_hide {
        camera_centre(class)
    } else {
        stage_default
    }
}
/// Whether the script's requested status is one of the three tech situations (ground, wall, ceiling):
/// only then does the status-change hook read the terms and ask the script's own check.
pub fn is_tech_situation(t: &Table, requested: i32) -> bool {
    [
        C::StatusDown,
        C::StatusDamageFlyReflectD,
        C::StatusStopWall,
        C::StatusDamageFlyReflectLr,
        C::StatusStopCeil,
        C::StatusDamageFlyReflectU,
    ]
    .iter()
    .any(|c| t.is(*c, requested))
}
