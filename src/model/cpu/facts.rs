//! What the runtime reads from the game for one fighter body's frame, and what it must do with the
//! answer. The runtime gathers `FrameFacts` before it takes the CPU lock (a game call can re-enter one
//! of this plugin's hooks, so no game call is made while the lock is held), runs `Cpu::frame`, and
//! applies the `FrameOut` after it releases the lock. Raw status values are the game's own; the
//! model classifies them with the resolved constant table.

use super::status::Situation;
use super::tech::HideOut;

/// The facts only the tech specs read (SPEC-cpu-04): the runtime fills them when the status needs them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TechFacts {
    /// `MotionModule::end_frame`, for the hide-tech show-again rule.
    pub end_frame: i32,
    /// The work float `FIGHTER_INSTANCE_WORK_ID_FLOAT_CURSOR_OFFSET_Y` (the primary body only).
    pub cursor_offset: f32,
    /// `DamageModule::damage(accessor, 0)` and the three `common` parameters of Snake's lay-down
    /// lock-out (`laydown_no_action_frame`, `laydown_no_action_frame_max`, `laydown_damage_max`), read
    /// only in the lay-down status.
    pub damage: f32,
    pub laydown_base: f32,
    pub laydown_max: f32,
    pub laydown_damage_max: f32,
}

/// One body's facts for one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameFacts {
    pub status: i32,
    /// `StatusModule::prev_status_kind(0)`.
    pub prev_status: i32,
    pub situation: Situation,
    pub prev_situation: Situation,
    /// The integer motion frame.
    pub motion_frame: i32,
    /// `PostureModule::pos_x` of the player entry and of this body.
    pub player_x: f32,
    pub cpu_x: f32,
    /// The work flag `FIGHTER_INSTANCE_WORK_ID_FLAG_CATCH_CLIFF`.
    pub catch_cliff: bool,
    /// The transition term `CONT_CLIFF_ATTACK` is enabled (a ledge action can be taken).
    pub cont_cliff_attack: bool,
    pub tech: TechFacts,
    /// Gravity speed y is negative, and the fast-fall flag `RESERVE_DIVE` is already set (SPEC-cpu-06).
    pub falling: bool,
    pub reserve_dive: bool,
}

impl Default for FrameFacts {
    fn default() -> Self {
        Self {
            status: 0,
            prev_status: 0,
            situation: Situation::Other,
            prev_situation: Situation::Other,
            motion_frame: 0,
            player_x: 0.0,
            cpu_x: 0.0,
            catch_cliff: false,
            cont_cliff_attack: false,
            tech: TechFacts::default(),
            falling: false,
            reserve_dive: false,
        }
    }
}

/// A playback the CPU behaviours ask the recording spec to start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaybackStart {
    /// The slot, 0..=4.
    pub slot: u8,
    /// A ledge playback (a 5-frame input buffer, 4 when the body is still in `CLIFF_CATCH`).
    pub ledge: bool,
}

impl PlaybackStart {
    /// Bit 8 of the request word marks a ledge playback.
    pub const LEDGE_BIT: u32 = 1 << 8;

    /// The word the input-recording package's inbox (`published::CPU_PLAYBACK_REQUEST`) carries: `1 + slot`
    /// in the low byte (0 means no request) and `LEDGE_BIT` for a ledge playback.
    pub const fn request_word(self) -> u32 {
        (1 + self.slot as u32) | if self.ledge { Self::LEDGE_BIT } else { 0 }
    }

    /// The request a word carries, or `None` for 0 (no request) and for a word that is not one.
    pub const fn from_request_word(word: u32) -> Option<Self> {
        let low = word & 0xff;
        if low == 0 || low > 5 || word & !(0xff | Self::LEDGE_BIT) != 0 {
            return None;
        }
        Some(Self { slot: (low - 1) as u8, ledge: word & Self::LEDGE_BIT != 0 })
    }
}

/// What the runtime applies after one body's frame. Every field is `None` when nothing is asked.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameOut {
    /// `ControlModule::add_clatter_time(accessor, delta, 0)` and the `sys_clatter` effect, together.
    pub clatter: Option<f32>,
    /// Write `FIGHTER_INSTANCE_WORK_ID_FLAG_DISABLE_ESCAPE_PENALTY` with this value.
    pub stale_flag: Option<bool>,
    /// `StatusModule::change_status_request_from_script(accessor, status, true)`; the raw status.
    pub request_status: Option<i32>,
    /// Start an input playback.
    pub playback: Option<PlaybackStart>,
    /// `StatusModule::change_status_force(accessor, status, true)`: the missed-tech getup.
    pub force_status: Option<i32>,
    /// The hidden tech animation (primary body only).
    pub hide: HideOut,
    /// Set `RESERVE_DIVE` and show the fast-fall flash (SPEC-cpu-06).
    pub fast_fall: bool,
}
