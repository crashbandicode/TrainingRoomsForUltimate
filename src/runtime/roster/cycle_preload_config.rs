//! Startup cache for the cycle-preload candidate.
//!
//! `init` reads `sd:/ultimate/TrainingRooms/cycle_preload.ini` once.
//! `snapshot` copies that cache and does not touch the file. Menu apply writes
//! the INI only when the committed choice changed. Slot count 0 is off: do not
//! preload while measurements can still record the Off control. The same atomic
//! word carries `bank_members` (0 off, else a native bank size per role); read it
//! through `snapshot().bank_members`, never from the file or the menu.

use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use std::sync::Once;

use training_mod_consts::cycle_preload_config::{
    adopt_unset, apply_settings, load_ini_unset, pack, publish_decision, startup_settings,
    store_ini, unpack, IniLoad, Publish, Unset, PATH,
};
use crate::engine::compat::menu::TrainingModpackMenu;
use {crate::runtime::p1::compat::sync::assign, crate::runtime::p1::compat::sync::read};

use training_mod_consts::cycle_preload_config::Settings;

static CACHE: AtomicU32 = AtomicU32::new(0);
static LOADED: Once = Once::new();
/// Optional keys the startup INI omitted (`Unset::to_bits`), consumed by the
/// first reconcile that sees the saved menu.
static UNSET: AtomicU16 = AtomicU16::new(0);

/// Load the INI once, overlay both stored menus, and refresh the quick menu.
///
/// Call this from `initialize_training_modpack` after the menu loader joins.
/// Do not call it while that loader holds the quick-menu lock, and do not call
/// it on a frame callback. A call on either side of `menu::load_from_file` is
/// safe when the two calls do not overlap: the menu loader overlays the INI
/// again after it assigns JSON.
pub fn init() {
    reconcile_loaded_menu();
    crate::runtime::p1::compat::common::menu::refresh_stored_menu_on_quick_app();
}

/// Cached choice. Before `init` or a menu-loader reconcile, this is closed
/// (slots 0, no sources, no samples). Afterward, slots 0 still means do not
/// preload while measurements can still record the Off control.
pub fn snapshot() -> Settings {
    unpack(CACHE.load(Ordering::Acquire))
}

/// One startup read, then copy the cache onto the menu JSON already in memory.
pub(crate) fn reconcile_loaded_menu() {
    load_cache();
    let mut settings = snapshot();
    if !settings.selectable() {
        return;
    }
    let mut menu = read(&training_mod_consts::MENU);
    let mut defaults = read(&training_mod_consts::DEFAULTS_MENU);
    // Keys an older INI omitted take the saved menu choice once, and the INI is
    // written with them: a menu Off is honoured exactly as after a menu apply.
    // Keys the INI carries keep winning.
    let unset = Unset::from_bits(UNSET.swap(0, Ordering::AcqRel));
    if unset.any() {
        let adopted = adopt_unset(settings, unset, &menu);
        if adopted != settings {
            match store_ini(PATH, adopted) {
                Ok(()) => {
                    log::info!(
                        "[CyclePreload] adopted saved menu choices for keys missing from cycle_preload.ini: bank_members={} group_swap={} ready_fighters={} ic_pair_bank={} original_bank={} solo_popo_groups={} ic_pair_groups={} native_groups={} pair_groups={} mii_groups={}",
                        adopted.bank_members,
                        adopted.group_swap,
                        adopted.ready_fighters,
                        adopted.ic_pair_bank,
                        adopted.original_bank,
                        adopted.solo_popo_groups,
                        adopted.ic_pair_groups,
                        adopted.native_groups,
                        adopted.pair_groups,
                        adopted.mii_groups
                    );
                    settings = adopted;
                    store(adopted);
                }
                Err(reason) => log::warn!(
                    "[CyclePreload] {reason}; keeping the built-in choices for keys missing from cycle_preload.ini"
                ),
            }
        }
    }
    apply_settings(&mut menu, settings);
    apply_settings(&mut defaults, settings);
    assign(&training_mod_consts::MENU, menu);
    assign(&training_mod_consts::DEFAULTS_MENU, defaults);
}

/// Persist a normal menu apply. Does not lock the quick menu; the caller may
/// already hold that lock. Does not read the INI again after startup.
pub(crate) fn publish_applied(menu: &mut TrainingModpackMenu) {
    load_cache();
    let previous = snapshot();
    match publish_decision(previous, menu) {
        Publish::Keep(settings) => {
            apply_settings(menu, settings);
            store(settings);
        }
        Publish::Store(settings) => match store_ini(PATH, settings) {
            Ok(()) => {
                apply_settings(menu, settings);
                store(settings);
                log::info!(
                    "[CyclePreload] applied slots={} fighters={} hats={} measurements={} bank_members={} group_swap={} ready_fighters={} ic_pair_bank={} original_bank={} solo_popo_groups={} ic_pair_groups={} native_groups={} pair_groups={} mii_groups={}",
                    settings.slots,
                    settings.fighters,
                    settings.hats,
                    settings.measurements,
                    settings.bank_members,
                    settings.group_swap,
                    settings.ready_fighters,
                    settings.ic_pair_bank,
                    settings.original_bank,
                    settings.solo_popo_groups,
                    settings.ic_pair_groups,
                    settings.native_groups,
                    settings.pair_groups,
                    settings.mii_groups
                );
            }
            Err(reason) => {
                let fallback = if previous.selectable() {
                    previous
                } else {
                    Settings::FAIL_CLOSED
                };
                log::warn!(
                    "[CyclePreload] {reason}; keeping slots={} bank_members={}",
                    fallback.slots,
                    fallback.bank_members
                );
                apply_settings(menu, fallback);
                store(fallback);
            }
        },
    }
}

fn load_cache() {
    LOADED.call_once(|| {
        let (load, unset) = load_ini_unset(PATH);
        let settings = startup_settings(load);
        UNSET.store(unset.to_bits(), Ordering::Release);
        match load {
            IniLoad::Parsed(settings) => {
                log::info!(
                    "[CyclePreload] loaded slots={} fighters={} hats={} measurements={} bank_members={} group_swap={} ready_fighters={} ic_pair_bank={} original_bank={} solo_popo_groups={} ic_pair_groups={} native_groups={} pair_groups={} mii_groups={}",
                    settings.slots,
                    settings.fighters,
                    settings.hats,
                    settings.measurements,
                    settings.bank_members,
                    settings.group_swap,
                    settings.ready_fighters,
                    settings.ic_pair_bank,
                    settings.original_bank,
                    settings.solo_popo_groups,
                    settings.ic_pair_groups,
                    settings.native_groups,
                    settings.pair_groups,
                    settings.mii_groups
                );
            }
            IniLoad::Created(settings) => {
                log::info!(
                    "[CyclePreload] wrote default cycle_preload.ini; slots={} bank_members={} group_swap={} ready_fighters={} ic_pair_bank={} original_bank={} solo_popo_groups={} ic_pair_groups={} native_groups={} pair_groups={} mii_groups={}",
                    settings.slots,
                    settings.bank_members,
                    settings.group_swap,
                    settings.ready_fighters,
                    settings.ic_pair_bank,
                    settings.original_bank,
                    settings.solo_popo_groups,
                    settings.ic_pair_groups,
                    settings.native_groups,
                    settings.pair_groups,
                    settings.mii_groups
                );
            }
            IniLoad::Unsaved(reason) => {
                log::error!(
                    "[CyclePreload] ERROR {reason}: {PATH} is missing and its default could not be saved; this boot runs the built-in defaults unsaved and the next menu apply or boot tries again"
                );
                crate::engine::compat::logging::diagnostic(format_args!(
                    "[CyclePreload] ERROR default_not_saved reason=\"{reason}\" path={PATH} running=defaults slots={} bank_members={} group_swap={}",
                    settings.slots, settings.bank_members, settings.group_swap
                ));
            }
            IniLoad::Rejected(reason) => {
                log::error!(
                    "[CyclePreload] ERROR {reason}: {PATH} is left unchanged; this boot fails closed (slots 0, bank_members 0, every group off) until the file is fixed or a menu apply replaces it"
                );
                crate::engine::compat::logging::diagnostic(format_args!(
                    "[CyclePreload] ERROR rejected reason=\"{reason}\" path={PATH} running=fail_closed slots=0 bank_members=0 group_swap=false"
                ));
            }
        };
        store(settings);
        // After the load, so the line carries the settings this boot applies (E8).
        #[cfg(feature = "native-body-bank-alpha")]
        crate::runtime::roster::character_roster::body_bank::report_group_swap(&settings);
    });
}

fn store(settings: Settings) {
    let settings = if settings.selectable() {
        settings
    } else {
        Settings::CLOSED
    };
    let value = pack(settings);
    if CACHE.swap(value, Ordering::AcqRel) != value {
        crate::runtime::roster::cycle_preload::settings_changed();
    }
}
