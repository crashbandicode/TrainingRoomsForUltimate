//! The typed menu view behind the carried code's `MENU` global (decision D1).
//!
//! The settings live as rows in the clean room (`model::settings::Settings`); package 5's commit path publishes them and
//! calls [`apply`] with the committed settings (so does boot, once they are loaded). [`apply`] parses the typed view (the rows and
//! the kept keys of the menu object) and the top-level kept keys the carried engines read (`stage_preferences`,
//! `mii_preferences`) once, on the committing thread; a hook or tick reads the stored
//! copy with `read(&MENU)` and allocates nothing. Before the first [`apply`] the factory values are what a reader gets.

use parking_lot::RwLock;

use crate::engine::compat::menu::{TrainingModpackMenu, BASE_MENU};
use crate::engine::mii::selection::Preferences as MiiPreferences;
use crate::engine::stages::stage_menu::Preferences as StagePreferences;
use crate::model::settings::Settings;
use crate::runtime::compat::sync::Readable;

struct Current {
    menu: TrainingModpackMenu,
    stage: StagePreferences,
    mii: MiiPreferences,
}

static CURRENT: RwLock<Option<Current>> = RwLock::new(None);

/// The carried code's `MENU`: `read(&MENU)` is the current typed menu.
pub struct Menu;
pub static MENU: Menu = Menu;

impl Readable for Menu {
    type Value = TrainingModpackMenu;
    fn read_value(&self) -> TrainingModpackMenu {
        CURRENT.read().as_ref().map_or(BASE_MENU, |current| current.menu)
    }
}

/// A kept key of the settings file as a typed value; a missing or malformed one is the type's default (the fork's rule for these
/// two keys: a bad stored preference never blocks the menu).
fn kept<T: serde::de::DeserializeOwned + Default>(settings: &Settings, key: &str) -> T {
    settings.extra.get(key).and_then(|value| serde_json::from_value(value.clone()).ok()).unwrap_or_default()
}

/// Publish the committed settings to the carried engines (called after they are published, from the commit path and boot).
pub fn apply(settings: &Settings) {
    let next = Current {
        menu: TrainingModpackMenu::from_snapshot(&settings.menu),
        stage: kept(settings, "stage_preferences"),
        mii: kept(settings, "mii_preferences"),
    };
    *CURRENT.write() = Some(next);
}

/// The stage preferences (direct stage, rotation forms) of the Lobby Stages screen.
pub fn stage_preferences() -> StagePreferences {
    CURRENT.read().as_ref().map_or_else(StagePreferences::default, |current| current.stage)
}

/// The saved Mii picks per role and class.
pub fn mii_preferences() -> MiiPreferences {
    CURRENT.read().as_ref().map_or_else(MiiPreferences::default, |current| current.mii.clone())
}
