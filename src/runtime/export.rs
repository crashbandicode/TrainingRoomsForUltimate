//! The state export v1 (`model::state_export`) for the test-only hardware bridge. Unsubscribed, an exported diag site,
//! the diag line offer, the frame callback and the item mailbox each cost one relaxed load and nothing else; no snapshot
//! is kept for the bridge. Subscribed, the lines of the oracle families go into the reader's ring as TR formats them.
//! Requests run on the reader's thread, never in a hook, and are refused outside Training and waiting-room practice.

use std::sync::atomic::{AtomicI32, AtomicPtr, AtomicU64, AtomicUsize, Ordering};

use crate::model::settings::{Loaded, Snapshot};
use crate::model::state_export::{self as abi, Ring, Table};
use crate::runtime::diag_log::diagnostic;

static RING: AtomicPtr<Ring> = AtomicPtr::new(std::ptr::null_mut());
static ON_FRAME: AtomicUsize = AtomicUsize::new(0);
static LAST_NOTICE: AtomicU64 = AtomicU64::new(0);
static ITEM_REQUEST: AtomicI32 = AtomicI32::new(-1);

const VERSION: &str = env!("CARGO_PKG_VERSION");
static TABLE: Table = Table {
    abi: abi::ABI,
    size: std::mem::size_of::<Table>() as u32,
    version: VERSION.as_ptr(),
    version_len: VERSION.len(),
    subscribe,
    request,
};

#[no_mangle]
pub extern "C" fn training_rooms_state_export_v1() -> *const Table {
    &TABLE
}

/// Whether an exported diag line is formatted at all: the diag build always, the shipped build only for a reader.
#[inline(always)]
pub fn wanted() -> bool {
    cfg!(feature = "diag") || !RING.load(Ordering::Relaxed).is_null()
}

/// `diag_log::enqueue` offers every formatted line here.
pub fn offer(text: &[u8], whole: bool) {
    if RING.load(Ordering::Relaxed).is_null() {
        return;
    }
    let ring = RING.load(Ordering::Acquire);
    if !abi::exported(text) || !abi::notice_changed(text, &LAST_NOTICE) {
        return;
    }
    // SAFETY: a subscribed ring lives as long as the process (the subscribe contract).
    unsafe { (*ring).push(crate::runtime::clock::uptime_ms(), text, !whole) };
}

/// The per-fighter frame (`states::on_fighter_frame`, practice scenes only) for the reader's callback.
pub fn fighter_frame(entry: i32, kind: i32, status: i32, situation: i32, flags: u32, accessor: usize, fighter: usize) {
    if ON_FRAME.load(Ordering::Relaxed) == 0 {
        return;
    }
    let callback = ON_FRAME.load(Ordering::Acquire);
    if callback != 0 {
        // SAFETY: the reader registered this `extern "C"` function and never unloads.
        let callback: abi::OnFrame = unsafe { std::mem::transmute(callback) };
        callback(&abi::FighterFrame { entry, kind, status, situation, flags, reserved: 0, accessor, fighter });
    }
}

extern "C" fn subscribe(ring: *mut Ring, on_frame: Option<abi::OnFrame>) -> i32 {
    // SAFETY: the reader passes its own ring or null.
    if ring.is_null() || unsafe { (*ring).abi } != abi::ABI {
        return abi::BAD_REQUEST;
    }
    ON_FRAME.store(on_frame.map_or(0, |f| f as usize), Ordering::Release);
    RING.store(ring, Ordering::Release);
    diagnostic(format_args!("[TrainingRooms] state_export subscribed abi={} frame_callback={}", abi::ABI, on_frame.is_some()));
    abi::OK
}

extern "C" fn request(kind: u32, data: *const u8, len: usize) -> i32 {
    match kind {
        abi::REQ_SETTLED => return if settled() { abi::OK } else { abi::BUSY },
        abi::REQ_RELOAD_SAVES => {
            crate::runtime::features::states::reload_from_disk();
            return abi::OK;
        }
        _ => {}
    }
    if !crate::model::scene::is_training(crate::runtime::scene::facts()) {
        return abi::REFUSED_SCENE;
    }
    match kind {
        abi::REQ_SET_APPLY if !data.is_null() => {
            // SAFETY: the reader's buffer, live for this call.
            match crate::model::settings_request::accept(unsafe { std::slice::from_raw_parts(data, len) }) {
                Ok(loaded) => apply_loaded(loaded),
                Err(issues) => {
                    diagnostic(format_args!("[TrainingRooms] state_export set refused issues={issues}"));
                    return abi::REFUSED_SETTINGS;
                }
            }
        }
        abi::REQ_SAVE => crate::runtime::features::states::request_save(),
        abi::REQ_LOAD => crate::runtime::features::states::request_load(),
        abi::REQ_ITEM if len <= i32::MAX as usize => queue_item(len as i32),
        _ => return abi::BAD_REQUEST,
    }
    abi::OK
}

/// Every TR writer the bridge's fixture requests can reach is idle with nothing due (states, settings, bound save).
fn settled() -> bool {
    crate::runtime::features::states::settled()
        && crate::runtime::features::writer::settled()
        && crate::model::rr_host::bound_settled()
}

/// The diag reload's file, as boot reads one (recovered when it has issues).
#[cfg(feature = "diag")]
pub fn apply_settings(bytes: &[u8]) {
    apply_loaded(crate::model::settings::parse_bytes(bytes));
}

/// Settings made live as the diag reload does it: the re-roll bits the menu would have set row by row, against the
/// committed settings the menu opens on (`worker::committed_menu`).
fn apply_loaded(loaded: Loaded) {
    let next = loaded.settings.menu.clone();
    let old = crate::runtime::ui::worker::committed_menu().unwrap_or_else(Snapshot::factory);
    let rerolls = crate::model::menu::rerolls_between(&old, &next);
    diagnostic(format_args!("[TrainingRooms] diag_settings_reload rerolls={rerolls:#x} issues={}", loaded.issues.len()));
    crate::runtime::published::publish(&next, rerolls, [None; 2]);
    crate::runtime::compat::menu::apply_kept(&loaded.settings);
    // The published settings are what a menu close keeps now (decision MENU-CLOSE).
    crate::runtime::ui::worker::adopt_published(&loaded.settings);
}

/// An item grant (a choice of the item table) for the game thread (`lobby::frame`), as the Give item screen's A press.
pub fn queue_item(choice: i32) {
    diagnostic(format_args!("[TrainingRooms] diag_item_request choice={choice}"));
    ITEM_REQUEST.store(choice, Ordering::Release);
}

pub fn take_item_request() -> Option<usize> {
    if ITEM_REQUEST.load(Ordering::Relaxed) < 0 {
        return None;
    }
    usize::try_from(ITEM_REQUEST.swap(-1, Ordering::AcqRel)).ok()
}
