//! The typed menu view behind the carried code's `MENU` global (decision D1).
//!
//! The settings live as rows in the clean room (`model::settings::Settings`); `published::publish` turns the rows into the typed
//! view (`published::Live::menu`, read wait-free: boot, every commit, the diag reload: the one path), and [`apply_kept`] adds what a
//! `Snapshot` does not carry (the kept keys `stage_preferences`, `mii_preferences`) once, on the committing thread; a hook or tick reads
//! the stored copy with `read(&MENU)` and allocates nothing. Before the first publish the factory values are what a reader gets.
//!
//! The carried engines also change the menu: the roster cycle records the fighter it swapped to, the preload INI overlays its keys.
//! `assign(&MENU, menu)` and the guard of `lock_write(&MENU)` store the changed typed view and write the rows and kept keys that
//! differ back into the settings the menu opens on (`ui::worker::adopt_typed`), which the writer thread persists.

use std::ops::{Deref, DerefMut};

use hos_lock::Mutex;

use crate::engine::compat::menu::{TrainingModpackMenu, BASE_MENU};
use crate::engine::mii::selection::Preferences as MiiPreferences;
use crate::engine::stages::stage_menu::Preferences as StagePreferences;
use crate::model::settings::Settings;
use crate::runtime::compat::sync::{Assignable, Lockable, Readable};

/// What a `Snapshot` does not carry: the kept keys. Published by the publishing path (`apply_kept`, plugin threads only), read by
/// hooks and plugin threads as a copy that never waits. It was an `RwLock` that hooks read blocking while the menu worker held it
/// (its preload planner read it in place): the rc.6 freeze class (`rc6-qpf3`, `tests/lock_order.rs`).
#[derive(Clone, Copy)]
struct Kept {
    stage: StagePreferences,
    mii: MiiPreferences,
}

static KEPT: hos_lock::Published<Option<Kept>> = hos_lock::Published::new(None);

/// A game-side edit of the typed menu (`assign(&MENU, ..)`, the guard of `lock_write(&MENU)`): the value and the publish generation it
/// was made against. It is what readers get until the next publish, which already carries it (the worker writes it back into the
/// settings before it commits, `ui::worker`). Taken by game threads only.
static EDITED: Mutex<Option<(u32, TrainingModpackMenu)>> = Mutex::new(None);

/// The game-side edit as plugin threads see it (stream/sync slice 3: a plugin thread never takes `EDITED`, which game threads
/// take): a wait-free triple buffer whose one writer is `store` (serialised by `EDITED`) and whose one reader is a plugin thread
/// under `published::PUBLISHING` (the publishing path, and `read(&MENU)` on a plugin thread: [`plugin_view`]).
static EDIT_HANDOFF: hos_lock::TripleBuffer<Option<(u32, TrainingModpackMenu)>> = hos_lock::TripleBuffer::new(None);

/// The carried code's `MENU`: `read(&MENU)` is the current typed menu.
pub struct Menu;
pub static MENU: Menu = Menu;

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

/// The typed menu on a plugin thread: the view [`current_menu`] gives, read the way the publishing path reads it (the edit handoff and
/// the live menu, under `published::PUBLISHING`, which only plugin threads take), never through `EDITED`. rc.6 QP freeze (`rc6-qpf3`,
/// 2026-11-09): the menu worker took `EDITED` here through `read(&MENU)` (named macros, the lobby flag, the roster), the game's
/// MainThread preempted it on core 0 and spun in a task join, and the layout task on another core waited in its hook for `EDITED`.
fn plugin_view() -> TrainingModpackMenu {
    let _turn = crate::runtime::published::PUBLISHING.lock();
    let generation = crate::runtime::published::live_generation();
    match EDIT_HANDOFF.read().0 {
        Some((at, menu)) if at == generation => menu,
        _ => crate::runtime::published::live().map_or(BASE_MENU, |live| live.menu),
    }
}

impl Readable for Menu {
    type Value = TrainingModpackMenu;
    fn read_value(&self) -> TrainingModpackMenu {
        // Game threads (hooks) take `EDITED`; plugin threads never do (stream/sync slice 3).
        if crate::runtime::threads::on_plugin_thread() {
            plugin_view()
        } else {
            current_menu()
        }
    }
}

/// A game-side typed menu edit waiting for the menu worker. Saved defaults
/// belong exclusively to the menu model's current/default snapshot pair.
static PENDING: Mutex<Option<TrainingModpackMenu>> = Mutex::new(None);

pub fn flush_writeback() {
    let menu = PENDING.lock().take();
    if let Some(menu) = menu {
        crate::runtime::ui::worker::adopt_typed(false, &menu.rows(), &menu.kept_values());
    }
}

/// Store a game-side edit, then ask the existing worker to persist its changed rows.
fn store(menu: TrainingModpackMenu) {
    let changed = current_menu() != menu;
    let edit = Some((crate::runtime::published::live_generation(), menu));
    {
        let mut edited = EDITED.lock();
        *edited = edit;
        EDIT_HANDOFF.write(edit);
    }
    if changed {
        *PENDING.lock() = Some(menu);
        crate::runtime::ui::worker::request_writeback();
    }
}
impl Assignable for Menu {
    type Value = TrainingModpackMenu;
    fn assign_value(&self, value: TrainingModpackMenu) {
        store(value);
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
        store(self.menu);
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
    crate::engine::compat::menu::kept_or_default(&settings.extra, key)
}

/// The typed menu a publish replaces: the view readers had (a game-side edit or the published rows), or None before the first publish
/// of the run (boot). Read wait-free on the publishing thread.
pub(crate) fn view_before_publish() -> Option<TrainingModpackMenu> {
    let generation = crate::runtime::published::live_generation();
    if generation == 0 {
        return None;
    }
    // The game-side edit, from the handoff (never `EDITED`): valid while no publish has happened since it was made.
    let (edit, _) = EDIT_HANDOFF.read();
    Some(match edit {
        Some((at, menu)) if at == generation => menu,
        _ => crate::runtime::published::live().map_or(BASE_MENU, |live| live.menu),
    })
}

/// A publish's first step for the typed menu, on the publishing thread (boot, the menu worker, the diag reload), before the new value
/// goes live and outside every lock a hook takes: the rows as published become the rows that go live. Package 6 syncs its preload
/// cache here when a menu was already live; cache write failure never changes the published choice.
pub(crate) fn before_publish(published: TrainingModpackMenu, previous: Option<TrainingModpackMenu>) -> TrainingModpackMenu {
    let mut next = published;
    if crate::runtime::roster::running() && previous.is_some() {
        crate::runtime::roster::cycle_preload_config::publish_applied(&mut next);
    }
    next
}

/// A publish's last step for the typed menu, on the publishing thread after the new value went live: rows the engines changed are
/// written back into the settings the menu opens on. When a live menu changed, package 6's roster re-plan is only *noted* here: the
/// menu worker runs it ([`run_roster_change`], a thread, never a game hook, so its allocations stay out of the hook counters), so a
/// publish does only the publish (stream/stack: run here, the planner sat under boot's frame on Skyline's 64 KiB thread). The first
/// publish of a run has nothing to compare with.
pub(crate) fn after_publish(published: TrainingModpackMenu, next: TrainingModpackMenu, previous: Option<TrainingModpackMenu>) {
    if next != published {
        *PENDING.lock() = Some(next);
        crate::runtime::ui::worker::request_writeback();
    }
    if let Some(previous) = previous.filter(|previous| *previous != next) {
        note_roster_change(previous);
    }
}

/// The roster's pending re-plan: the menu as it was before the earliest publish the roster has not seen yet (several publishes
/// before its next frame are compared once, earliest against live).
static ROSTER_CHANGE: Mutex<Option<TrainingModpackMenu>> = Mutex::new(None);
static ROSTER_CHANGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn note_roster_change(previous: TrainingModpackMenu) {
    {
        let mut pending = ROSTER_CHANGE.lock();
        if pending.is_none() {
            *pending = Some(previous);
        }
        ROSTER_CHANGED.store(true, std::sync::atomic::Ordering::Release);
    }
    crate::runtime::ui::worker::wake();
}

/// The menu worker's step (`ui::worker::run`, after each wake): the roster re-plans for a noted settings change, once per change
/// (one atomic load when nothing changed).
pub(crate) fn run_roster_change() {
    if !ROSTER_CHANGED.swap(false, std::sync::atomic::Ordering::AcqRel) {
        return;
    }
    let previous = ROSTER_CHANGE.lock().take();
    if let Some(previous) = previous {
        crate::runtime::roster::character_roster::settings_changed(&previous, None);
    }
}
/// The settings-only part of the typed view: the two kept keys (`stage_preferences`, `mii_preferences`), which
/// a `Snapshot` does not carry. Called after the publish at boot, at every commit and by the diag reload (never in a hook): the
/// values are computed first, then published (`KEPT`). The menu rows reach `MENU` through `published::Live` (one path).
pub fn apply_kept(settings: &Settings) {
    KEPT.publish(Some(Kept {
        stage: kept(settings, "stage_preferences"),
        mii: crate::engine::compat::menu::mii_preferences_of(settings.extra.get("mii_preferences")),
    }));
}

/// The stage preferences (direct stage, rotation forms) of the Lobby Stages screen. Never waits.
pub fn stage_preferences() -> StagePreferences {
    KEPT.read().map_or_else(StagePreferences::default, |kept| kept.stage)
}

/// The saved Mii picks per role and class: a copy (no heap, never waits; the preload planner reads them on every planning tick).
pub fn mii_preferences() -> MiiPreferences {
    KEPT.read().map_or_else(MiiPreferences::default, |kept| kept.mii)
}
