//! The save-state rows of the settings as one small `Copy` value the per-frame code reads without
//! allocating (`model::settings::Snapshot` keeps rows in vectors). Built from a snapshot at boot and at
//! every menu commit; the runtime replaces the published copy as a whole (the same contract as
//! `model::cpu::settings::CpuSettings`).

use crate::model::cpu::settings::Weights;
use crate::model::settings::Snapshot;

/// Mirroring (SPEC-state-01 §2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mirroring {
    #[default]
    None,
    Alternate,
    Random,
}

/// How a load treats a role's damage (Save Dmg, SPEC-state-01 §5.3 step 8a).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DamageMode {
    /// Leave the respawn damage (0%).
    #[default]
    Default,
    /// The saved percent.
    SaveState,
    /// Uniform between the range's two ends.
    Random,
}

/// A set of buffs: bit `i` is option `i` of the menu's 18-entry list (SPEC-state-03 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Buffs(pub u32);

impl Buffs {
    pub const ACCELERATE: u32 = 0;
    pub const OOMPH: u32 = 1;
    pub const PSYCHE: u32 = 2;
    pub const BOUNCE: u32 = 3;
    pub const ARSENE: u32 = 4;
    pub const BREATHING: u32 = 5;
    pub const LIMIT: u32 = 6;
    pub const KO: u32 = 7;
    pub const WING: u32 = 8;
    pub const MONAD_JUMP: u32 = 9;
    pub const MONAD_SPEED: u32 = 10;
    pub const MONAD_SHIELD: u32 = 11;
    pub const MONAD_BUSTER: u32 = 12;
    pub const MONAD_SMASH: u32 = 13;
    pub const POWER_DRAGON: u32 = 14;
    pub const WAFT_MINI: u32 = 15;
    pub const WAFT_HALF: u32 = 16;
    pub const WAFT_FULL: u32 = 17;
    pub const COUNT: usize = 18;

    pub fn from_row(row: &[u8]) -> Self {
        Self(row.iter().take(Self::COUNT).enumerate().fold(0, |bits, (i, v)| if *v != 0 { bits | 1 << i } else { bits }))
    }

    pub fn has(&self, option: u32) -> bool {
        option < 32 && self.0 & (1 << option) != 0
    }

    pub fn any(&self) -> bool {
        self.0 != 0
    }
}

/// The state rows, as a hook reads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateSettings {
    pub enable: bool,
    /// Save State Slot, `0..SLOTS`.
    pub slot: u8,
    pub randomize: Weights<5>,
    pub mirroring: Mirroring,
    pub autoload: bool,
    pub respawn_invincibility: bool,
    pub damage_player: DamageMode,
    pub limits_player: (u8, u8),
    pub damage_cpu: DamageMode,
    pub limits_cpu: (u8, u8),
    pub buffs: Buffs,
}

impl Default for StateSettings {
    fn default() -> Self {
        StateSettings {
            enable: true,
            slot: 0,
            randomize: Weights::default(),
            mirroring: Mirroring::None,
            autoload: false,
            respawn_invincibility: false,
            damage_player: DamageMode::Default,
            limits_player: (0, 150),
            damage_cpu: DamageMode::Default,
            limits_cpu: (0, 150),
            buffs: Buffs(0),
        }
    }
}

fn first(row: Option<&[u8]>) -> Option<usize> {
    row?.iter().position(|v| *v != 0)
}

fn on(row: Option<&[u8]>, default: bool) -> bool {
    // `OnOff` is `[On, Off]`.
    match first(row) {
        Some(0) => true,
        Some(_) => false,
        None => default,
    }
}

fn damage(row: Option<&[u8]>) -> DamageMode {
    match first(row) {
        Some(1) => DamageMode::SaveState,
        Some(2) => DamageMode::Random,
        _ => DamageMode::Default,
    }
}

fn limits(row: Option<&[u8]>) -> (u8, u8) {
    match row {
        Some(&[lower, upper]) => (lower.min(150), upper.min(150)),
        _ => (0, 150),
    }
}

impl StateSettings {
    pub fn from_snapshot(snapshot: &Snapshot) -> Self {
        let row = |id: &str| snapshot.get(id);
        StateSettings {
            enable: on(row("save_state_enable"), true),
            slot: first(row("save_state_slot")).map_or(0, |i| i.min(4) as u8),
            randomize: Weights::from_row(row("randomize_slots").unwrap_or(&[])),
            mirroring: match first(row("save_state_mirroring")) {
                Some(1) => Mirroring::Alternate,
                Some(2) => Mirroring::Random,
                _ => Mirroring::None,
            },
            autoload: on(row("save_state_autoload"), false),
            respawn_invincibility: on(row("respawn_invincibility"), false),
            damage_player: damage(row("save_damage_player")),
            limits_player: limits(row("save_damage_limits_player")),
            damage_cpu: damage(row("save_damage_cpu")),
            limits_cpu: limits(row("save_damage_limits_cpu")),
            buffs: Buffs::from_row(row("buff_state").unwrap_or(&[])),
        }
    }
}
