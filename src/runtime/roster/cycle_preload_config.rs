//! Runtime publication of the settings.json preload choice.
//!
//! Boot and menu publication call the model's transactional INI cache sync on
//! their own threads. Native callbacks only read the packed atomic snapshot.
use std::sync::atomic::{AtomicU32, Ordering};
use crate::engine::compat::menu::TrainingModpackMenu;
use crate::engine::preload::boot_cache::{self, Cache};
use crate::engine::preload::cycle_preload_config::{pack, unpack, Settings, PATH};

static CACHE: AtomicU32 = AtomicU32::new(0);

/// Called once from roster boot, after settings.json has been published.
pub fn init() {
    let menu = crate::runtime::compat::sync::read(&crate::runtime::compat::menu::MENU);
    publish(&menu, "loaded");
    #[cfg(feature = "native-body-bank-alpha")]
    crate::runtime::roster::character_roster::body_bank::report_group_swap(&snapshot());
}

/// No locks, files, allocations, or menu edits on the callback path.
pub fn snapshot() -> Settings {
    unpack(CACHE.load(Ordering::Acquire))
}

/// Publishing thread only, before the new menu becomes live and outside hook locks.
/// A failed INI write never rolls back the player's settings.json choice.
pub(crate) fn publish_applied(menu: &mut TrainingModpackMenu) {
    publish(menu, "applied");
}

fn publish(menu: &TrainingModpackMenu, event: &str) {
    let synced = boot_cache::sync(PATH, menu);
    if let Cache::Failed(reason) = synced.cache {
        crate::engine::compat::logging::log_warn!(
            "[CyclePreload] {reason}; settings.json choice remains active; cache retries on next publish or boot"
        );
    }
    let s = synced.active;
    store(s);
    crate::engine::compat::logging::log_info!(
        "[CyclePreload] {} slots={} fighters={} hats={} measurements={} bank_members={} group_swap={} ready_fighters={} ic_pair_bank={} original_bank={} solo_popo_groups={} ic_pair_groups={} native_groups={} pair_groups={} mii_groups={}",
        event, s.slots, s.fighters, s.hats, s.measurements, s.bank_members,
        s.group_swap, s.ready_fighters, s.ic_pair_bank, s.original_bank,
        s.solo_popo_groups, s.ic_pair_groups, s.native_groups, s.pair_groups, s.mii_groups
    );
}

fn store(settings: Settings) {
    let value = pack(settings);
    if CACHE.swap(value, Ordering::AcqRel) != value {
        crate::runtime::roster::cycle_preload::settings_changed();
    }
}
