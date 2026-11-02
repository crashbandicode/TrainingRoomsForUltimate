//! The typed menu view behind the carried code's `MENU` global (decision D1).
//!
//! The settings live as rows in the clean room (`model::settings::Settings`); package 5's commit path publishes them and
//! calls [`apply`] with the committed settings (so does boot, once they are loaded). [`apply`] parses the typed view (the rows and
//! the kept keys of the menu object) and the top-level kept keys the carried engines read (`stage_preferences`,
//! `mii_preferences`) once, on the committing thread; a hook or tick reads the stored
//! copy with `read(&MENU)` and allocates nothing. Before the first [`apply`] the factory values are what a reader gets.
//!
//! The carried engines also change the menu: the roster cycle records the fighter it swapped to, the preload INI overlays its keys.
//! `assign(&MENU, menu)` and the guard of `lock_write(&MENU)` store the changed typed view and write the rows and kept keys that
//! differ back into the settings the menu opens on (`ui::worker::adopt_typed`), which the writer thread persists.

use std::ops::{Deref, DerefMut};

use parking_lot::{Mutex, RwLock};

use crate::engine::compat::menu::{TrainingModpackMenu, BASE_MENU};
use crate::engine::mii::selection::Preferences as MiiPreferences;
use crate::engine::stages::stage_menu::Preferences as StagePreferences;
use crate::model::settings::Settings;
use crate::runtime::compat::sync::{Assignable, Lockable, Readable};

struct Current {
    menu: TrainingModpackMenu,
    defaults: TrainingModpackMenu,
    stage: StagePreferences,
    mii: MiiPreferences,
}

static CURRENT: RwLock<Option<Current>> = RwLock::new(None);

/// The carried code's `MENU`: `read(&MENU)` is the current typed menu.
pub struct Menu;
pub static MENU: Menu = Menu;

/// The carried code's `DEFAULTS_MENU`: the player's saved defaults, typed.
pub struct DefaultsMenu;
pub static DEFAULTS_MENU: DefaultsMenu = DefaultsMenu;

impl Readable for Menu {
    type Value = TrainingModpackMenu;
    fn read_value(&self) -> TrainingModpackMenu {
        CURRENT.read().as_ref().map_or(BASE_MENU, |current| current.menu)
    }
}

impl Readable for DefaultsMenu {
    type Value = TrainingModpackMenu;
    fn read_value(&self) -> TrainingModpackMenu {
        CURRENT.read().as_ref().map_or(BASE_MENU, |current| current.defaults)
    }
}

/// Typed views waiting to be written back into the settings (menu, saved defaults): no allocation to leave one here, so a game
/// callback can store; the menu worker does the rest (`flush_writeback`).
static PENDING: Mutex<[Option<TrainingModpackMenu>; 2]> = Mutex::new([None, None]);

/// Write the waiting typed views back into the settings (called by the menu worker's thread, after `request_writeback`).
pub fn flush_writeback() {
    let [menu, defaults] = std::mem::take(&mut *PENDING.lock());
    for (is_defaults, typed) in [(false, menu), (true, defaults)] {
        if let Some(typed) = typed {
            crate::runtime::ui::worker::adopt_typed(is_defaults, &typed.rows(), &typed.kept_values());
        }
    }
}

/// Store a typed view (`defaults` selects the saved defaults); what changed is written back into the settings by the menu worker.
fn store(defaults: bool, menu: TrainingModpackMenu) {
    let changed = {
        let mut guard = CURRENT.write();
        let current = guard.get_or_insert_with(|| Current {
            menu: BASE_MENU,
            defaults: BASE_MENU,
            stage: StagePreferences::default(),
            mii: MiiPreferences::default(),
        });
        let slot = if defaults { &mut current.defaults } else { &mut current.menu };
        let changed = *slot != menu;
        *slot = menu;
        changed
    };
    if changed {
        PENDING.lock()[usize::from(defaults)] = Some(menu);
        crate::runtime::ui::worker::request_writeback();
    }
}

impl Assignable for Menu {
    type Value = TrainingModpackMenu;
    fn assign_value(&self, value: TrainingModpackMenu) {
        store(false, value);
    }
}

impl Assignable for DefaultsMenu {
    type Value = TrainingModpackMenu;
    fn assign_value(&self, value: TrainingModpackMenu) {
        store(true, value);
    }
}

/// An edit in place: a copy of the typed menu that is stored (and written back) when the guard drops.
pub struct MenuGuard {
    menu: TrainingModpackMenu,
}

impl Deref for MenuGuard {
    type Target = TrainingModpackMenu;
    fn deref(&self) -> &TrainingModpackMenu {
        &self.menu
    }
}

impl DerefMut for MenuGuard {
    fn deref_mut(&mut self) -> &mut TrainingModpackMenu {
        &mut self.menu
    }
}

impl Drop for MenuGuard {
    fn drop(&mut self) {
        store(false, self.menu);
    }
}

impl Lockable for Menu {
    type Guard = MenuGuard;
    fn lock_write(&'static self) -> MenuGuard {
        MenuGuard { menu: MENU.read_value() }
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
        defaults: TrainingModpackMenu::from_snapshot(&settings.defaults),
        stage: kept(settings, "stage_preferences"),
        mii: kept(settings, "mii_preferences"),
    };
    *CURRENT.write() = Some(next);
}

/// The committed settings after the menu closed: `apply`, then the roster cycle compares the menu it had with the committed one (a
/// changed roster, hat order or pick re-plans the cycle and may queue a swap; an unrelated change does nothing).
pub fn apply_commit(settings: &Settings) {
    let previous = MENU.read_value();
    apply(settings);
    crate::runtime::roster::character_roster::settings_changed(&previous, None);
}

/// The stage preferences (direct stage, rotation forms) of the Lobby Stages screen.
pub fn stage_preferences() -> StagePreferences {
    CURRENT.read().as_ref().map_or_else(StagePreferences::default, |current| current.stage)
}

/// The saved Mii picks per role and class.
pub fn mii_preferences() -> MiiPreferences {
    CURRENT.read().as_ref().map_or_else(MiiPreferences::default, |current| current.mii.clone())
}
