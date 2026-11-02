//! Diag build only (Eden tests): replace the live settings while the game runs. The menu is package 5; until
//! it exists a test changes a setting by writing `sd:/ultimate/TrainingRooms/diag-settings-next.json` (a whole
//! settings file; write it to another name and rename it). A one-second poll thread reads it, deletes it,
//! publishes the new snapshot with the re-roll bits the menu would have set row by row
//! (`menu::rerolls_between`) and logs one `diag_settings_reload` line. Never part of the shipped NRO.

use std::fs;
use std::sync::Mutex;
use std::time::Duration;

use crate::model::menu;
use crate::model::settings::{self, Snapshot};
use crate::model::storage::FixedPath;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::published;

const NAME: &str = "diag-settings-next.json";

/// The settings last published (what a reload is compared with).
static CURRENT: Mutex<Option<Snapshot>> = Mutex::new(None);

/// `published::publish` calls this with every snapshot it makes live.
pub fn remember(snapshot: &Snapshot) {
    *CURRENT.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(snapshot.clone());
}

/// Start the poll thread (boot, after the first publish).
pub fn start() {
    let spawned = std::thread::Builder::new()
        .name("tr-diag-reload".into())
        .stack_size(settings::LOADER_STACK_BYTES)
        .spawn(|| loop {
            std::thread::sleep(Duration::from_secs(1));
            poll();
        });
    if spawned.is_err() {
        diagnostic(format_args!("[TrainingRooms] diag_settings_reload thread unavailable"));
    }
}

fn poll() {
    let Some(path) = FixedPath::<128>::in_root(NAME) else { return };
    let Ok(bytes) = fs::read(path.as_str()) else { return };
    let _ = fs::remove_file(path.as_str());
    let loaded = settings::parse_bytes(&bytes);
    let next = loaded.settings.menu.clone();
    let old = CURRENT.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone().unwrap_or_else(Snapshot::factory);
    let rerolls = menu::rerolls_between(&old, &next);
    diagnostic(format_args!("[TrainingRooms] diag_settings_reload rerolls={rerolls:#x} issues={}", loaded.issues.len()));
    published::publish(&next, rerolls);
    crate::runtime::compat::menu::apply_kept(&loaded.settings);
}
