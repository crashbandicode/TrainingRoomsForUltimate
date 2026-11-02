//! Reads of the running game: the text base, the version, the 64 bytes at a site, and the one game
//! function every feature calls (the Training-mode predicate).
//!
//! NOT YET SWITCH-COMPILED.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use skyline::hooks::{getRegionAddress, Region};
use skyline::nn::oe::{DisplayVersion, GetDisplayVersion};

use crate::model::guard::GUARD_BYTES;
use crate::model::sites;
use crate::model::version::BUILD_ID_SCAN_BYTES;

static BASE: AtomicUsize = AtomicUsize::new(0);
static TRAINING_PREDICATE_USABLE: AtomicBool = AtomicBool::new(false);

/// The start of `main`'s `.text` (offsets in the site table are relative to it).
pub fn text_base() -> usize {
    let cached = BASE.load(Ordering::Relaxed);
    if cached != 0 {
        return cached;
    }
    // SAFETY: Skyline's region lookup has no preconditions.
    let base = unsafe { getRegionAddress(Region::Text) } as usize;
    BASE.store(base, Ordering::Relaxed);
    base
}

/// `nn::oe::DisplayVersion.name` (16 bytes, NUL padded).
pub fn display_version() -> [u8; 16] {
    let mut version = DisplayVersion { name: [0; 16] };
    // SAFETY: the function writes at most the 16-byte name into the struct we own.
    unsafe { GetDisplayVersion(&mut version) };
    version.name
}

/// The last page of `.rodata`, where 13.0.5 keeps the GNU build-id note (`.data` follows it).
pub fn rodata_tail() -> &'static [u8] {
    // SAFETY: the regions are mapped by the loader; the tail is the page before `.data`, clipped to
    // `.rodata`'s start so a layout with a short `.rodata` cannot read before it.
    unsafe {
        let rodata = getRegionAddress(Region::Rodata) as usize;
        let data = getRegionAddress(Region::Data) as usize;
        let start = data.saturating_sub(BUILD_ID_SCAN_BYTES).max(rodata);
        std::slice::from_raw_parts(start as *const u8, data - start)
    }
}

/// The 64 bytes now at `main+offset`.
pub fn read_window(offset: usize) -> [u8; GUARD_BYTES] {
    let mut out = [0u8; GUARD_BYTES];
    // SAFETY: only called for offsets of the site table, which lie inside `main`'s mapped image.
    unsafe { std::ptr::copy_nonoverlapping((text_base() + offset) as *const u8, out.as_mut_ptr(), GUARD_BYTES) };
    out
}

/// The pointer-sized value now at `main+offset` (a vtable slot), as an offset from `.text`.
pub fn read_slot_target(offset: usize) -> u64 {
    let base = text_base();
    // SAFETY: only called for the vtable slot row, inside `main`'s mapped `.data`.
    let value = unsafe { ((base + offset) as *const u64).read_volatile() };
    value.wrapping_sub(base as u64)
}

/// Record that the Training-mode predicate passed its 64-byte guard (it is a called function:
/// other plugins' hooks on it are fine).
pub fn allow_training_predicate() {
    TRAINING_PREDICATE_USABLE.store(true, Ordering::Release);
}

/// The game's `app::smashball::is_training_mode()`; false when its guard did not pass.
pub fn is_training_mode() -> bool {
    if !TRAINING_PREDICATE_USABLE.load(Ordering::Acquire) {
        return false;
    }
    // SAFETY: the site was guarded against the 13.0.5 bytes; the function takes no arguments and
    // returns a bool.
    let predicate: extern "C" fn() -> bool =
        unsafe { std::mem::transmute(text_base() + sites::IS_TRAINING_MODE_OFFSET) };
    predicate()
}
