//! `cycle_preload.ini` as a boot cache of `settings.json` (decision D7 and the lead's ruling of 2026-11-02); written for this
//! repository, not carried.
//!
//! The fork kept the 13 `cycle_preload_*` rows in two places and let the INI overlay the menu at startup. Here `settings.json`
//! (through the typed view of `compat::menu`) is the only source of the active choice. The INI is regenerated from it: written when
//! it is missing, stale, unreadable or rejected, left alone when it already says the same, and never read back to change the
//! active choice. A first boot therefore writes the player's imported (or the factory) settings, not the engine's own defaults, and
//! a rejected or failed cache cannot turn the preload on or off by itself. The carried reader and the transactional writer
//! (`store_ini`: temporary file, rename, `.previous` recovery) do the file work.

use super::cycle_preload_config::{parse_unset, settings_from_menu, store_ini, Settings, MAX_BYTES};
use crate::engine::compat::menu::TrainingModpackMenu;

/// What the cache did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cache {
    /// The file already said the active choice.
    Unchanged,
    /// The file was missing, stale or rejected and now says the active choice.
    Written,
    /// The file could not be written (the reason); the active choice is unaffected and the next boot tries again.
    Failed(&'static str),
}

/// The active choice and what happened to the cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Synced {
    pub active: Settings,
    pub cache: Cache,
}

/// The choice `settings.json` asks for. Rows that do not read as one choice each fail closed (preload off), the carried
/// `Settings::FAIL_CLOSED`, never the engine default.
pub fn desired(menu: &TrainingModpackMenu) -> Settings {
    settings_from_menu(menu).unwrap_or(Settings::FAIL_CLOSED)
}

/// Boot, and a menu apply: make the INI say what the settings say.
pub fn sync(path: &str, menu: &TrainingModpackMenu) -> Synced {
    let active = desired(menu);
    // Never invoke the legacy default-creating reader: even the first write must reflect JSON.
    // Invalid menu rows fail closed without touching the cache.
    let cache = if settings_from_menu(menu).is_none() || cached(path) == Some(active) {
        Cache::Unchanged
    } else {
        match store_ini(path, active) {
            Ok(()) => Cache::Written,
            Err(reason) => Cache::Failed(reason),
        }
    };
    Synced { active, cache }
}

fn cached(path: &str) -> Option<Settings> {
    use std::io::Read;
    let file = std::fs::File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut text = String::new();
    file.take((MAX_BYTES + 1) as u64).read_to_string(&mut text).ok()?;
    if text.len() > MAX_BYTES {
        return None;
    }
    let (settings, unset) = parse_unset(&text).ok()?;
    (!unset.any()).then_some(settings)
}

