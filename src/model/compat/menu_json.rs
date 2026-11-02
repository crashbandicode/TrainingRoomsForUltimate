//! The fork's menu file as the carried tests load it: `{"menu": {...}, "defaults_menu": {...}}` into the typed view.
//!
//! The fork's `menu_json::parse_menu_file` (ours-new, not carried: it is bound to the fork's whole menu struct, its roster and
//! Mii-preference sections) reads the same envelope into every field. This stand-in reads it into the view of `compat::menu`, so
//! the carried tests of the choice deserializers (`deserialize_slots`, `deserialize_bank_members`, `deserialize_ready_fighters`
//! and the per-key defaults) still run against a menu file shaped like the fork's. The clean room's own reader of this file is
//! `model::settings::parse` (package 1); this module is only the carried code's expectation. Written for this repository.

use serde::Deserialize;

use super::menu::{TrainingModpackMenu, BASE_MENU};

const fn base_menu() -> TrainingModpackMenu {
    BASE_MENU
}

/// The two snapshots of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct SavedMenus {
    #[serde(default = "base_menu")]
    pub menu: TrainingModpackMenu,
    #[serde(default = "base_menu")]
    pub defaults_menu: TrainingModpackMenu,
}

/// What `parse_menu_file` returns: the fork's value also holds the roster and preference sections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadedMenuFile {
    pub settings: SavedMenus,
}

pub fn parse_menu_file(contents: &str) -> Result<LoadedMenuFile, serde_json::Error> {
    Ok(LoadedMenuFile { settings: serde_json::from_str(contents)? })
}
