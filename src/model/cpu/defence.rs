//! CPU defensive behaviour (SPEC-cpu-02): DI, SDI, air-dodge direction, clatter mashing, dodge
//! staling. Pure: each rule takes the body's state, the settings and the plugin's own generator, and
//! answers what the hook writes. One state per fighter body (decision B6).
//!
//! The Direction list and its angles are the spec's §2 table. "In" and "Out" are relative to the
//! opponent on every row and on both sides (decision B7): an angle `a` is mirrored to `π − a` when the
//! player stands to the CPU's right; Left and Right are absolute; Neutral is the stick at rest.

use core::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use super::rng::Rng;
use super::settings::{CpuSettings, Strength};

/// Indices of the Direction option set.
pub mod direction {
    pub const OUT: u8 = 0;
    pub const UP_OUT: u8 = 1;
    pub const UP: u8 = 2;
    pub const UP_IN: u8 = 3;
    pub const IN: u8 = 4;
    pub const DOWN_IN: u8 = 5;
    pub const DOWN: u8 = 6;
    pub const DOWN_OUT: u8 = 7;
    pub const NEUTRAL: u8 = 8;
    pub const LEFT: u8 = 9;
    pub const RIGHT: u8 = 10;
}

/// The angle of a direction before mirroring, in radians; `None` for Neutral (the stick at rest).
pub fn angle(dir: u8) -> Option<f32> {
    Some(match dir {
        direction::OUT | direction::RIGHT => 0.0,
        direction::UP_OUT => FRAC_PI_4,
        direction::UP => FRAC_PI_2,
        direction::UP_IN => 3.0 * FRAC_PI_4,
        direction::IN | direction::LEFT => PI,
        direction::DOWN_IN => 5.0 * FRAC_PI_4,
        direction::DOWN => 3.0 * FRAC_PI_2,
        direction::DOWN_OUT => 7.0 * FRAC_PI_4,
        _ => return None,
    })
}

/// The unit vector of a direction before mirroring: the eight compass directions are exact, so no
/// trigonometry is needed (a `cos` or `sin` call links the whole math library into the image).
fn unit_vector(dir: u8) -> Option<(f32, f32)> {
    const H: f32 = core::f32::consts::FRAC_1_SQRT_2;
    Some(match dir {
        direction::OUT | direction::RIGHT => (1.0, 0.0),
        direction::UP_OUT => (H, H),
        direction::UP => (0.0, 1.0),
        direction::UP_IN => (-H, H),
        direction::IN | direction::LEFT => (-1.0, 0.0),
        direction::DOWN_IN => (-H, -H),
        direction::DOWN => (0.0, -1.0),
        direction::DOWN_OUT => (H, -H),
        _ => return None,
    })
}

/// The stick vector of a direction for a CPU at `cpu_x` with the player at `player_x`: `(cos a, sin a)`
/// of the direction's angle, mirrored (`a` becomes `π − a`, the x sign flips) when the player is to the
/// CPU's right and the direction is not Left or Right; `(0, 0)` for Neutral.
pub fn stick_vector(dir: u8, player_x: f32, cpu_x: f32) -> (f32, f32) {
    let Some((x, y)) = unit_vector(dir) else { return (0.0, 0.0) };
    if player_x > cpu_x && dir != direction::LEFT && dir != direction::RIGHT {
        (-x, y)
    } else {
        (x, y)
    }
}

/// SDI: one input every `period` frames; `None` for Strength None (never).
pub const fn sdi_period(strength: Strength) -> Option<u32> {
    match strength {
        Strength::None => None,
        Strength::Normal => Some(8),
        Strength::Medium => Some(6),
        Strength::High => Some(4),
    }
}

/// Clatter: one simulated input every `period` frames (the code's rates 8 / 5 / 2, SPEC-cpu-02 §2).
pub const fn clatter_period(strength: Strength) -> Option<u32> {
    match strength {
        Strength::None => None,
        Strength::Normal => Some(8),
        Strength::Medium => Some(5),
        Strength::High => Some(2),
    }
}

/// The clatter step before the first start call sets it (SPEC-cpu-02 §5).
pub const CLATTER_STEP_DEFAULT: f32 = 8.0;

/// What the SDI hook answers for a CPU body in Training.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SdiAnswer {
    /// Report an SDI input this hitlag frame: write the vector into the out parameter and return 1.
    Input(f32, f32),
    /// Return 0. The hook never calls the original for a CPU body (decision N9: SDI not set means no
    /// SDI; the game's own CPU SDI is never used).
    NoInput,
    /// A playback has possession of the CPU and *Settings override playback* is Off (decisions B5 /
    /// N4): the override is skipped and the hook calls the original.
    Native,
}

/// One body's defensive state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Defence {
    /// The DI case held for the whole hitstun episode.
    pub di_case: Option<u8>,
    /// The SDI direction rolled by the reset bundle or by a row change, held until the next of either.
    pub sdi_dir: Option<u8>,
    pub sdi_counter: u32,
    /// The air-dodge direction held for the aerial hitstun episode.
    pub air_dodge_case: Option<u8>,
    pub clatter_counter: u32,
    pub clatter_step: f32,
    /// The staling flag value last written, `None` until written (so a fresh body is written once).
    pub stale_written: Option<bool>,
}

impl Defence {
    pub const fn new() -> Self {
        Self {
            di_case: None,
            sdi_dir: None,
            sdi_counter: 0,
            air_dodge_case: None,
            clatter_counter: 0,
            clatter_step: CLATTER_STEP_DEFAULT,
            stale_written: None,
        }
    }

    // ---- DI ------------------------------------------------------------------------------------

    /// The game is about to correct a damage vector for this body: the DI vector to write into the
    /// two stick floats, or `None` to write nothing (an empty row leaves the native CPU's DI). The
    /// case is rolled on the first correction while none is held and kept while the body stays in
    /// hitstun; mirroring uses the positions at this moment.
    pub fn di_correction(&mut self, s: &CpuSettings, rng: &mut Rng, player_x: f32, cpu_x: f32) -> Option<(f32, f32)> {
        if s.di_state.is_empty() {
            return None;
        }
        if self.di_case.is_none() {
            self.di_case = s.di_state.pick(rng).map(|i| i as u8);
        }
        self.di_case.map(|dir| stick_vector(dir, player_x, cpu_x))
    }

    /// Once per frame: the DI case clears whenever the body is not in hitstun.
    pub fn di_frame(&mut self, in_hitstun: bool) {
        if !in_hitstun {
            self.di_case = None;
        }
    }

    // ---- SDI -----------------------------------------------------------------------------------

    /// Roll the SDI direction from the row and restart the period counter (the reset bundle, and the
    /// next CPU frame after the row changed; decision Q18 A).
    pub fn sdi_roll(&mut self, s: &CpuSettings, rng: &mut Rng) {
        self.sdi_dir = s.sdi_state.pick(rng).map(|i| i as u8);
        self.sdi_counter = 0;
    }

    /// One hitlag query of the SDI hook for this body.
    pub fn sdi_query(&mut self, s: &CpuSettings, player_x: f32, cpu_x: f32) -> SdiAnswer {
        let (Some(period), Some(dir)) = (sdi_period(s.sdi_strength), self.sdi_dir) else {
            return SdiAnswer::NoInput;
        };
        self.sdi_counter = (self.sdi_counter + 1) % period;
        if self.sdi_counter != period - 1 || dir == direction::NEUTRAL {
            return SdiAnswer::NoInput;
        }
        let (x, y) = stick_vector(dir, player_x, cpu_x);
        SdiAnswer::Input(x, y)
    }

    // ---- air-dodge direction -------------------------------------------------------------------

    /// One stick read of the air-dodge override. `airborne_in_hitstun` is the body's state now. One
    /// direction is rolled per aerial hitstun episode and every x and y read of it returns the same
    /// vector; `None` leaves the game's stick (not airborne in hitstun, or an empty row).
    pub fn air_dodge_stick(
        &mut self,
        s: &CpuSettings,
        rng: &mut Rng,
        airborne_in_hitstun: bool,
        player_x: f32,
        cpu_x: f32,
    ) -> Option<(f32, f32)> {
        if !airborne_in_hitstun {
            self.air_dodge_case = None;
            return None;
        }
        if s.air_dodge_dir.is_empty() {
            return None;
        }
        if self.air_dodge_case.is_none() {
            self.air_dodge_case = s.air_dodge_dir.pick(rng).map(|i| i as u8);
        }
        self.air_dodge_case.map(|dir| stick_vector(dir, player_x, cpu_x))
    }

    // ---- clatter -------------------------------------------------------------------------------

    /// The game started a clatter situation for this body: remember its manual recovery rate as the
    /// step (the original is always called by the hook).
    pub fn clatter_start(&mut self, manual_recovery_rate: f32) {
        self.clatter_step = manual_recovery_rate;
    }

    /// Once per frame while the body is in a clatter status: the clatter time to add (negative) on an
    /// input frame, with the `sys_clatter` effect on the same frame. Outside clatter the counter is
    /// left as it is (the caller does not call this).
    pub fn clatter_frame(&mut self, s: &CpuSettings) -> Option<f32> {
        let period = clatter_period(s.clatter_strength)?;
        self.clatter_counter = (self.clatter_counter + 1) % period;
        (self.clatter_counter == period - 1).then_some(-self.clatter_step)
    }

    // ---- dodge staling -------------------------------------------------------------------------

    /// The value `FIGHTER_INSTANCE_WORK_ID_FLAG_DISABLE_ESCAPE_PENALTY` must have: the staling row On
    /// means the game's penalty applies, so the flag is false.
    pub const fn stale_flag_wanted(s: &CpuSettings) -> bool {
        !s.stale_dodges
    }

    /// The write this frame needs, if any: the first frame of a body and any change of the row.
    pub fn stale_write(&mut self, s: &CpuSettings) -> Option<bool> {
        let wanted = Self::stale_flag_wanted(s);
        if self.stale_written == Some(wanted) {
            return None;
        }
        self.stale_written = Some(wanted);
        Some(wanted)
    }

    /// Forget what was written (a save-state load or a new session may have reset the flag).
    pub fn stale_forget(&mut self) {
        self.stale_written = None;
    }
}

impl Default for Defence {
    fn default() -> Self {
        Self::new()
    }
}
