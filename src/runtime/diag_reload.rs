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
    let spawned = crate::runtime::threads::spawn(crate::model::thread_stacks::ThreadId::DiagReload, || loop {
        std::thread::sleep(Duration::from_secs(1));
        poll();
    });
    if spawned.is_err() {
        diagnostic(format_args!("[TrainingRooms] diag_settings_reload thread unavailable"));
    }
}

/// `diag-item.req`: one line, a choice of the item table (`ITEM_NAMES`). The Give item screen is not built yet, so Eden asks the item
/// engine for a grant through this file; the game thread takes the request (`lobby::frame`, `take_item_request`) and calls
/// `character_items::request` exactly as the screen's A press will.
const ITEM_NAME: &str = "diag-item.req";

static ITEM_REQUEST: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

/// The item choice Eden asked for, once.
pub fn take_item_request() -> Option<usize> {
    usize::try_from(ITEM_REQUEST.swap(-1, std::sync::atomic::Ordering::AcqRel)).ok()
}

/// A request file's text, once: read, then removed.
fn request(name: &str) -> Option<String> {
    let path = FixedPath::<128>::in_root(name)?;
    let bytes = fs::read(path.as_str()).ok()?;
    let _ = fs::remove_file(path.as_str());
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn poll_item_request() {
    let Some(text) = request(ITEM_NAME) else { return };
    let choice = text.trim().parse::<i32>().unwrap_or(-1);
    diagnostic(format_args!("[TrainingRooms] diag_item_request choice={choice}"));
    ITEM_REQUEST.store(choice, std::sync::atomic::Ordering::Release);
}

/// `diag-egg.req`: two numbers, `dx dy`. The game thread moves the current Eggs-ercise egg to Player 1's position plus
/// that offset, floating (`hitbox::eggs_pass`), so an Eden row can put it where a move's hitbox will be.
static EGG_REQUEST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);

/// The egg offset Eden asked for, once.
pub fn take_egg_request() -> Option<[f32; 2]> {
    let bits = EGG_REQUEST.swap(u64::MAX, std::sync::atomic::Ordering::AcqRel);
    (bits != u64::MAX).then(|| [bits as u32, (bits >> 32) as u32].map(f32::from_bits))
}

fn poll_egg_request() {
    let Some(text) = request("diag-egg.req") else { return };
    let mut words = text.split_whitespace().map(|w| w.parse::<f32>().unwrap_or(0.0));
    let [dx, dy] = [words.next(), words.next()].map(Option::unwrap_or_default);
    diagnostic(format_args!("[TrainingRooms] diag_egg_request dx={dx} dy={dy}"));
    let bits = u64::from(dx.to_bits()) | u64::from(dy.to_bits()) << 32;
    EGG_REQUEST.store(bits, std::sync::atomic::Ordering::Release);
}

/// F9 research (package 6): the native pause menu object (`x22` of package 4's stale-moves site), and `diag-pausedump.req` (one
/// word, a tag) which writes its first `PAUSE_DUMP` bytes to `pausemenu-<tag>.bin`, so Eden can diff the object around a change of
/// No. of CPUs and find the field and the moment the count changes. Diag only; a read of a live game object from this thread.
static PAUSE_MENU: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
const PAUSE_DUMP: usize = 0x1000;

pub fn note_pause_menu(menu: usize) {
    if menu != 0 && menu % 8 == 0 && PAUSE_MENU.swap(menu, std::sync::atomic::Ordering::Relaxed) != menu {
        diagnostic(format_args!("[TrainingRooms] diag_pause_menu object=0x{menu:x}"));
    }
}

fn poll_pause_dump() {
    let Some(text) = request("diag-pausedump.req") else { return };
    let tag: String = text.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(24).collect();
    let menu = PAUSE_MENU.load(std::sync::atomic::Ordering::Relaxed);
    if menu == 0 {
        diagnostic(format_args!("[TrainingRooms] diag_pause_dump tag={tag} object=none"));
        return;
    }
    // SAFETY: diag only; the pause menu object of the current Training scene, read as bytes.
    let dump = unsafe { std::slice::from_raw_parts(menu as *const u8, PAUSE_DUMP) }.to_vec();
    let name = format!("pausemenu-{tag}.bin");
    let written = FixedPath::<128>::in_root(&name).map(|p| fs::write(p.as_str(), &dump).is_ok()).unwrap_or(false);
    diagnostic(format_args!("[TrainingRooms] diag_pause_dump tag={tag} object=0x{menu:x} written={written}"));
}

fn poll() {
    poll_item_request();
    poll_egg_request();
    poll_pause_dump();
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
    // The published settings are what a menu close keeps now (decision MENU-CLOSE).
    crate::runtime::ui::worker::adopt_published(&loaded.settings);
}
