//! The CPU rows of the settings, as one small `Copy` value the hooks read without allocating
//! (`model::settings::Snapshot` keeps its rows in vectors; a hook must not touch them). Built from a
//! snapshot at boot and at every menu commit; the runtime replaces the published copy as a whole.
//! Also the picks: a weighted pick over a row and the pick of a delay row (a checklist or, N3, a range).

use super::rng::Rng;
use crate::model::settings::Snapshot;

/// A weighted row: element `i` is the weight of option `i` (`0` = never).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Weights<const N: usize>(pub [u8; N]);

impl<const N: usize> Default for Weights<N> {
    fn default() -> Self {
        Self([0; N])
    }
}

impl<const N: usize> Weights<N> {
    /// The row's elements (a longer row loses its tail, a shorter one reads as zeros).
    pub fn from_row(row: &[u8]) -> Self {
        let mut out = [0u8; N];
        for (slot, value) in out.iter_mut().zip(row) {
            *slot = *value;
        }
        Self(out)
    }

    /// Nothing selected.
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|w| *w == 0)
    }

    pub fn weight(&self, i: usize) -> u8 {
        self.0.get(i).copied().unwrap_or(0)
    }

    pub fn total(&self) -> u32 {
        self.0.iter().map(|w| u32::from(*w)).sum()
    }

    /// A weighted pick (SPEC-cpu-01 §3): each option counts `weight` times, uniform over the total;
    /// weight 0 never; an empty row picks nothing. No allocation.
    pub fn pick(&self, rng: &mut Rng) -> Option<usize> {
        let total = self.total();
        if total == 0 {
            return None;
        }
        let mut left = rng.below(total);
        for (i, w) in self.0.iter().enumerate() {
            let w = u32::from(*w);
            if left < w {
                return Some(i);
            }
            left -= w;
        }
        None
    }

    /// The only selected option, or the first when several (a single-select row).
    pub fn first(&self) -> Option<usize> {
        self.0.iter().position(|w| *w != 0)
    }
}

/// A delay row as a consumer reads it (SPEC-core-02 §2.8, decision N3): nothing, a weighted checklist
/// of values (`first + i * step`), or a range picked uniformly among its step values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DelayRow {
    Empty,
    Set { weights: Weights<31>, first: u16, step: u16 },
    Range { min: u16, max: u16, step: u16 },
}

impl DelayRow {
    /// A set range counts as a selection (Pummel Delay: a set range turns pummelling on); so does a
    /// ticked checklist, even "0".
    pub fn is_set(&self) -> bool {
        !matches!(self, DelayRow::Empty)
    }

    /// Read a delay row from its checklist and its optional range (`[lo, hi]` option indices).
    pub fn read(checklist: &[u8], range: &[u8], first: u16, step: u16) -> Self {
        if let [lo, hi] = range {
            let (lo, hi) = (u16::from(*lo), u16::from(*hi));
            return DelayRow::Range { min: first + step * lo, max: first + step * hi.max(lo), step };
        }
        let weights = Weights::<31>::from_row(checklist);
        if weights.is_empty() {
            DelayRow::Empty
        } else {
            DelayRow::Set { weights, first, step }
        }
    }

    /// The pick: an empty row is 0; a checklist is a weighted pick; a range is uniform.
    pub fn pick(&self, rng: &mut Rng) -> u16 {
        match self {
            DelayRow::Empty => 0,
            DelayRow::Set { weights, first, step } => {
                weights.pick(rng).map_or(0, |i| first + step * i as u16)
            }
            DelayRow::Range { min, max, step } => {
                let step = (*step).max(1);
                let count = u32::from((*max - *min) / step) + 1;
                *min + rng.below(count) as u16 * step
            }
        }
    }
}

/// The 13 override rows, in the menu's situation order (`menu::options::SITUATIONS`).
pub mod situation {
    pub const LEDGE_NEUTRAL: usize = 0;
    pub const LEDGE_ROLL: usize = 1;
    pub const LEDGE_JUMP: usize = 2;
    pub const LEDGE_ATTACK: usize = 3;
    pub const TECH_ACTION: usize = 4;
    pub const CLATTER: usize = 5;
    pub const TUMBLE: usize = 6;
    pub const HITSTUN: usize = 7;
    pub const PARRY: usize = 8;
    pub const SHIELDSTUN: usize = 9;
    pub const FOOTSTOOL: usize = 10;
    pub const LANDING: usize = 11;
    pub const TRUMP: usize = 12;
    pub const COUNT: usize = 13;
}

/// The setting ids of the overrides, same order as `situation`.
pub const OVERRIDE_IDS: [&str; situation::COUNT] = [
    "ledge_neutral_override",
    "ledge_roll_override",
    "ledge_jump_override",
    "ledge_attack_override",
    "tech_action_override",
    "clatter_override",
    "tumble_override",
    "hitstun_override",
    "parry_override",
    "shieldstun_override",
    "footstool_override",
    "landing_override",
    "trump_override",
];

/// Indices of the Mash Triggers row (`menu::options::MASH_TRIGGERS`).
pub mod trigger {
    pub const HITSTUN: u8 = 0;
    pub const SHIELDSTUN: u8 = 1;
    pub const PARRY: u8 = 2;
    pub const TUMBLE: u8 = 3;
    pub const LANDING: u8 = 4;
    pub const LEDGE_TRUMP: u8 = 5;
    pub const FOOTSTOOL: u8 = 6;
    pub const CLATTER: u8 = 7;
    pub const LEDGE_OPTION: u8 = 8;
    pub const TECH_OPTION: u8 = 9;
    pub const MISTECH_OPTION: u8 = 10;
    pub const GROUNDED: u8 = 11;
    pub const AIRBORNE: u8 = 12;
    pub const DISTANCE_CLOSE: u8 = 13;
    pub const DISTANCE_MID: u8 = 14;
    pub const DISTANCE_FAR: u8 = 15;
    pub const ALWAYS: u8 = 16;
}

/// The strength rows (SDI and Clatter): None, Normal, Medium, High.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strength {
    None,
    Normal,
    Medium,
    High,
}

impl Strength {
    fn from_row(row: &[u8]) -> Self {
        match row.iter().position(|v| *v != 0) {
            Some(1) => Strength::Normal,
            Some(2) => Strength::Medium,
            Some(3) => Strength::High,
            _ => Strength::None,
        }
    }
}

/// Shield Toggles: None, Infinite, Hold, Constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShieldMode {
    None,
    Infinite,
    Hold,
    Constant,
}

impl ShieldMode {
    fn from_row(row: &[u8]) -> Self {
        match row.iter().position(|v| *v != 0) {
            Some(1) => ShieldMode::Infinite,
            Some(2) => ShieldMode::Hold,
            Some(3) => ShieldMode::Constant,
            _ => ShieldMode::None,
        }
    }
}

/// Every CPU row, read once per publish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuSettings {
    pub mash_state: Weights<30>,
    pub follow_up: Weights<30>,
    /// Bit `i` set when Mash Triggers option `i` is ticked.
    pub mash_triggers: u32,
    pub overrides: [Weights<30>; situation::COUNT],
    pub air_dodge_dir: Weights<11>,
    pub di_state: Weights<11>,
    pub sdi_state: Weights<11>,
    pub sdi_strength: Strength,
    pub clatter_strength: Strength,
    pub ledge_state: Weights<10>,
    pub ledge_delay: DelayRow,
    pub tech_state: Weights<4>,
    pub miss_tech_state: Weights<4>,
    pub tech_hide: bool,
    pub shield_state: ShieldMode,
    /// The one selected direction of the Shield Tilt row, as an index of the Direction list.
    pub shield_tilt: Option<u8>,
    pub oos_offset: DelayRow,
    pub reaction_time: DelayRow,
    pub attack_angle: Weights<3>,
    pub throw_state: Weights<5>,
    pub throw_delay: DelayRow,
    pub pummel_delay: DelayRow,
    pub falling_aerials: Weights<2>,
    pub full_hop: Weights<2>,
    pub aerial_delay: DelayRow,
    pub fast_fall: Weights<2>,
    pub fast_fall_delay: DelayRow,
    pub crouch: bool,
    /// The Dodge Staling row: On means the game's staling applies.
    pub stale_dodges: bool,
    pub frame_advantage: bool,
    /// Playback Mash Interrupt.
    pub playback_mash: bool,
    /// Decision N4: Settings override playback (default Off: the recording wins).
    pub override_playback: bool,
}

impl CpuSettings {
    /// Read the rows from a snapshot (a row the snapshot lacks reads as its factory value).
    pub fn from_snapshot(snapshot: &Snapshot) -> Self {
        let row = |id: &str| snapshot.get(id).unwrap_or(&[]);
        let on = |id: &str| row(id).first().is_some_and(|v| *v != 0);
        let delay = |id: &str, range: &str, step: u16| DelayRow::read(row(id), row(range), 0, step);
        let mut overrides = [Weights::<30>::default(); situation::COUNT];
        for (slot, id) in overrides.iter_mut().zip(OVERRIDE_IDS) {
            *slot = Weights::from_row(row(id));
        }
        let triggers = row("mash_triggers").iter().enumerate().fold(0u32, |mask, (i, v)| mask | (u32::from(*v != 0) << i));
        Self {
            mash_state: Weights::from_row(row("mash_state")),
            follow_up: Weights::from_row(row("follow_up")),
            mash_triggers: triggers,
            overrides,
            air_dodge_dir: Weights::from_row(row("air_dodge_dir")),
            di_state: Weights::from_row(row("di_state")),
            sdi_state: Weights::from_row(row("sdi_state")),
            sdi_strength: Strength::from_row(row("sdi_strength")),
            clatter_strength: Strength::from_row(row("clatter_strength")),
            ledge_state: Weights::from_row(row("ledge_state")),
            ledge_delay: delay("ledge_delay", "ledge_delay_range", 10),
            tech_state: Weights::from_row(row("tech_state")),
            miss_tech_state: Weights::from_row(row("miss_tech_state")),
            tech_hide: on("tech_hide"),
            shield_state: ShieldMode::from_row(row("shield_state")),
            shield_tilt: row("shield_tilt").iter().position(|v| *v != 0).map(|i| i as u8),
            oos_offset: delay("oos_offset", "oos_offset_range", 1),
            reaction_time: delay("reaction_time", "reaction_time_range", 1),
            attack_angle: Weights::from_row(row("attack_angle")),
            throw_state: Weights::from_row(row("throw_state")),
            throw_delay: delay("throw_delay", "throw_delay_range", 5),
            pummel_delay: delay("pummel_delay", "pummel_delay_range", 5),
            falling_aerials: Weights::from_row(row("falling_aerials")),
            full_hop: Weights::from_row(row("full_hop")),
            aerial_delay: delay("aerial_delay", "aerial_delay_range", 1),
            fast_fall: Weights::from_row(row("fast_fall")),
            fast_fall_delay: delay("fast_fall_delay", "fast_fall_delay_range", 1),
            crouch: on("crouch"),
            stale_dodges: on("stale_dodges"),
            frame_advantage: on("frame_advantage"),
            playback_mash: on("playback_mash"),
            override_playback: on("settings_override_playback"),
        }
    }

    /// Whether Mash Triggers option `i` (`trigger::*`) is ticked.
    pub fn trigger_on(&self, i: u8) -> bool {
        self.mash_triggers & (1 << i) != 0
    }
}
