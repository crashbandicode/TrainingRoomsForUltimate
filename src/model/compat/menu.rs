//! The typed menu view the carried fork code reads: `OnOff` and a `TrainingModpackMenu` with the fields the carried code uses.
//!
//! The fork keeps one big struct of every setting behind a global (`MENU`). The clean room keeps settings as rows by id
//! (`model::settings::Snapshot`); this view is built from a snapshot (and the kept-as-they-are rows) for the carried engines.
//! Fields are added here when a carried file needs them; each is the fork's field name, i.e. the setting id. Written for this
//! repository (not carried).

/// A two-state setting as the carried code names it: `OnOff::ON` / `OnOff::OFF`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OnOff(u32);

impl OnOff {
    pub const OFF: OnOff = OnOff(0);
    pub const ON: OnOff = OnOff(1);
}

/// The settings the carried hot-swap code reads (grows with each carried layer).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingModpackMenu {
    /// Ice Climbers' Solo Popo preference for Player slot 0 (`solo_popo_original`; factory default On).
    pub solo_popo_original: OnOff,
    /// Solo Popo for the Player's numbered cycle slots (`solo_popo_player_cycle`).
    pub solo_popo_player_cycle: OnOff,
    /// Solo Popo for the CPU (`solo_popo_cpu`).
    pub solo_popo_cpu: OnOff,
}

/// The factory values (`BASE_MENU` in the fork).
pub const BASE_MENU: TrainingModpackMenu =
    TrainingModpackMenu { solo_popo_original: OnOff::ON, solo_popo_player_cycle: OnOff::OFF, solo_popo_cpu: OnOff::OFF };