//! The typed menu view behind the carried code's `MENU` global (decision D1).
//!
//! The settings live as rows in the clean room (`model::settings::Settings`); `published::publish` turns the rows into the typed
//! view (`published::Live::menu`, read wait-free: boot, every commit, the diag reload: the one path), and [`apply_kept`] adds what a
//! `Snapshot` does not carry (the saved defaults and the kept keys `stage_preferences`, `mii_preferences`) once, on the committing thread; a hook or tick reads
//! the stored copy with `read(&MENU)` and allocates nothing. Before the first publish the factory values are what a reader gets.
//!
//! The carried engines also change the menu: the roster cycle records the fighter it swapped to, the preload INI overlays its keys.
//! `assign(&MENU, menu)` and the guard of `lock_write(&MENU)` store the changed typed view and write the rows and kept keys that
//! differ back into the settings the menu opens on (`ui::worker::adopt_typed`), which the writer thread persists.

use std::ops::{Deref, DerefMut};

use hos_lock::{Mutex, RwLock};

use crate::engine::compat::menu::{TrainingModpackMenu, BASE_MENU};
use crate::engine::mii::selection::Preferences as MiiPreferences;
use crate::engine::stages::stage_menu::Preferences as StagePreferences;
use crate::model::settings::Settings;
use crate::runtime::compat::sync::{Assignable, Lockable, Readable};

/// What a `Snapshot` does not carry: the saved defaults and the kept keys. Written by the publishing thread (`apply_kept`) as one
/// assignment (computed before the lock is taken), read by hooks for a copy.
struct Kept {
    defaults: TrainingModpackMenu,
    stage: StagePreferences,
    mii: MiiPreferences,
}

static KEPT: RwLock<Option<Kept>> = RwLock::new(None);

/// A game-side edit of the typed menu (`assign(&MENU, ..)`, the guard of `lock_write(&MENU)`): the value and the publish generation it
/// was made against. It is what readers get until the next publish, which already carries it (the worker writes it back into the
/// settings before it commits, `ui::worker`). Taken by game threads only.
static EDITED: Mutex<Option<(u32, TrainingModpackMenu)>> = Mutex::new(None);

/// The carried code's `MENU`: `read(&MENU)` is the current typed menu.
pub struct Menu;
pub static MENU: Menu = Menu;

/// The carried code's `DEFAULTS_MENU`: the player's saved defaults, typed.
pub struct DefaultsMenu;
pub static DEFAULTS_MENU: DefaultsMenu = DefaultsMenu;

/// The typed menu now: a game-side edit made since the last publish, else the published rows (`published::Live`), else the factory
/// values. The published value is read wait-free (stream/sync).
fn current_menu() -> TrainingModpackMenu {
    let generation = crate::runtime::published::live_generation();
    if let Some((at, menu)) = *EDITED.lock() {
        if at == generation {
            return menu;
        }
    }
    crate::runtime::published::live().map_or(BASE_MENU, |live| live.menu)
}

impl Readable for Menu {
    type Value = TrainingModpackMenu;
    fn read_value(&self) -> TrainingModpackMenu {
        current_menu()
    }
}

impl Readable for DefaultsMenu {
    type Value = TrainingModpackMenu;
    fn read_value(&self) -> TrainingModpackMenu {
        KEPT.read().as_ref().map_or(BASE_MENU, |kept| kept.defaults)
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
    let changed = if defaults {
        let mut kept = KEPT.write();
        let kept = kept.get_or_insert_with(|| Kept { defaults: BASE_MENU, stage: StagePreferences::default(), mii: MiiPreferences::default() });
        let changed = kept.defaults != menu;
        kept.defaults = menu;
        changed
    } else {
        let changed = current_menu() != menu;
        *EDITED.lock() = Some((crate::runtime::published::live_generation(), menu));
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

/// The typed menu a publish replaces: the view readers had (a game-side edit or the published rows), or None before the first publish
/// of the run (boot). Read wait-free on the publishing thread.
pub(crate) fn view_before_publish() -> Option<TrainingModpackMenu> {
    (crate::runtime::published::live_generation() != 0).then(current_menu)
}

/// A publish's first step for the typed menu, on the publishing thread (boot, the menu worker, the diag reload), before the new value
/// goes live and outside every lock a hook takes: the rows as published become the rows that go live. (Package 6's preload engine
/// canonicalises them here when a menu was already live: `cycle_preload_config::publish_applied`.)
pub(crate) fn before_publish(published: TrainingModpackMenu, previous: Option<TrainingModpackMenu>) -> TrainingModpackMenu {
    let _ = previous;
    published
}

/// A publish's last step for the typed menu, on the publishing thread after the new value went live: rows the engines changed are
/// written back into the settings the menu opens on. (Package 6's roster re-plan runs here when a live menu changed:
/// `character_roster::settings_changed(&previous, None)`; the first publish of a run has nothing to compare with.)
pub(crate) fn after_publish(published: TrainingModpackMenu, next: TrainingModpackMenu, previous: Option<TrainingModpackMenu>) {
    if next != published {
        PENDING.lock()[0] = Some(next);
        crate::runtime::ui::worker::request_writeback();
    }
    let _ = previous;
}
/// The settings-only part of the typed view: the saved defaults and the two kept keys (`stage_preferences`, `mii_preferences`), which
/// a `Snapshot` does not carry. Called after the publish at boot, at every commit and by the diag reload (never in a hook). The values
/// are computed first; the lock is held for the assignment only. The menu rows reach `MENU` through `published::Live` (one path).
pub fn apply_kept(settings: &Settings) {
    let kept = Kept {
        defaults: TrainingModpackMenu::from_snapshot(&settings.defaults),
        stage: kept(settings, "stage_preferences"),
        mii: kept(settings, "mii_preferences"),
    };
    let old = KEPT.write().replace(kept);
    drop(old);
}

/// The stage preferences (direct stage, rotation forms) of the Lobby Stages screen.
pub fn stage_preferences() -> StagePreferences {
    KEPT.read().as_ref().map_or_else(StagePreferences::default, |kept| kept.stage)
}

/// The saved Mii picks per role and class.
pub fn mii_preferences() -> MiiPreferences {
    KEPT.read().as_ref().map_or_else(MiiPreferences::default, |kept| kept.mii.clone())
}