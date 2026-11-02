//! PT-TRAINER-PARK, native side of `swap_model::special_articles`. This is the only caller of the PT article-pool
//! builder `0x3a6270` outside native's window constructor (`0x658db8`) and its in-place rebuild `0x65a650`. It is
//! also the only caller of the pool release `0x3a6b80` on a holder whose window stays alive (a detached PT group).
//! The holder passed in has been validated by the caller (`valid_special_object`: vtable `main+0x4f626e8`,
//! `+0x230` = owner). No bank lock is held across a native call.
//!
//! Lines:
//! - `[GroupSwap] articles retired owner= holder= pool=a->b why=`;
//! - `[GroupSwap] articles rebuilt owner= holder= pool=0->n slot= why=`;
//! - `[GroupSwap] articles NOT rebuilt owner= holder= why= reason=`.

use crate::engine::seams::native_fields as fields;
use crate::engine::swap::swap_model::special_articles::{self as model, ArticleStep};

macro_rules! trace {
    ($($arg:tt)*) => { crate::engine::compat::logging::diagnostic(format_args!($($arg)*)) };
}

/// `0x3a6270(holder, table, 0, -1, slot, 0)`: fills the holder's article pool from `table` (`0x658db8`, `0x65a748`).
pub(crate) const POOL_BUILD: usize = 0x3a6270;
/// Its first six words (`str x1,[x0,#0x228]; str w4,[x0,#0x230]; strb w5,[x0,#0x234]; cbz x1,…; stp …; stp …`).
pub(crate) const POOL_BUILD_WORDS: [u32; 6] = [0xf9011401, 0xb9023004, 0x3908d005, 0xb4000f01, 0xa9bc5ffc, 0xa90157f6];
/// `0x3a6b80(holder)`: removes every article and leaves `+0x10 = 0`, `+0x228 = 0` (`0x3a6cb4`).
const POOL_RELEASE: usize = 0x3a6b80;
/// The PT trainer table the build and `0x65a650` pass (`adrp 0x52b9000; add #0x488`).
const PT_ARTICLE_TABLE: usize = 0x52b9488;
/// The table's lazy-init guards the build tests (`ldarb`, bit 0) before using it (`0x65a6c0–0x65a6e0`).
const PT_TABLE_GUARDS: [usize; 2] = [0x52b9458, 0x52b9480];
/// `holder+0x10`: the pool's article count.
const POOL_COUNT: usize = 0x10;
/// `entry+0x5a88/+0x5a90`: the holder and the claimed trainer; `entry+0x591f` the window flags; `entry+0xe0` the
/// entry id the build passes as the pool's slot.
const HOLDER: usize = 0x5a88;
const CLAIMED: usize = 0x5a90;
const FLAGS: usize = 0x591f;
const SLOT: usize = 0xe0;

unsafe fn pool_count(holder: usize) -> u64 {
    if holder == 0 { 0 } else { fields::read::<u64>(holder, POOL_COUNT).unwrap_or(0) }
}

unsafe fn release(base: usize, holder: usize) {
    let release: unsafe extern "C" fn(usize) = std::mem::transmute(base + POOL_RELEASE);
    release(holder);
}

/// A window that already left the entry (`flags`, `holder` from its saved record): retire its pool if the model
/// says so. Returns whether it did. The caller then clears the saved `+0x5a90` record.
///
/// # Safety
/// `base` is the main image base; `holder` is the saved `+0x5a88`, validated by the caller when `valid`.
pub(crate) unsafe fn retire_saved(base: usize, owner: u32, flags: u8, holder: usize, valid: bool, why: &str) -> bool {
    let count = if valid { pool_count(holder) } else { 0 };
    if model::on_detach(flags, valid, count) != Some(ArticleStep::Retire) { return false; }
    release(base, holder);
    trace!("[GroupSwap] articles retired owner={} holder={:#x} pool={}->{} why={}", owner, holder, count, pool_count(holder), why);
    true
}

/// The entry's current window (a failed warm attach about to be replaced): retire its pool as native's destroyers
/// do, `+0x5a90 = 0` first. Returns whether it did.
///
/// # Safety
/// As for [`retire_saved`]; `entry` is the owner's proved fighter-manager entry.
pub(crate) unsafe fn retire_current(base: usize, owner: u32, entry: usize, valid: bool, why: &str) -> bool {
    let flags = fields::read::<u8>(entry, FLAGS).unwrap_or(0);
    let holder = fields::read::<usize>(entry, HOLDER).unwrap_or(0);
    let count = if valid { pool_count(holder) } else { 0 };
    if model::on_detach(flags, valid, count) != Some(ArticleStep::Retire) { return false; }
    fields::write_unchecked(entry, CLAIMED, 0usize);
    release(base, holder);
    trace!("[GroupSwap] articles retired owner={} holder={:#x} pool={}->{} why={}", owner, holder, count, pool_count(holder), why);
    true
}

/// The entry's current window was just restored (warm attach, or the rollback's restore): rebuild its pool with the
/// build's own arguments if the model says so. The activation that follows claims the trainer into `+0x5a90`.
///
/// # Safety
/// As for [`retire_current`].
pub(crate) unsafe fn rebuild_current(base: usize, owner: u32, entry: usize, valid: bool, why: &str) -> bool {
    let flags = fields::read::<u8>(entry, FLAGS).unwrap_or(0);
    let holder = fields::read::<usize>(entry, HOLDER).unwrap_or(0);
    let count = if valid { pool_count(holder) } else { 0 };
    if model::on_attach(flags, valid, count) != Some(ArticleStep::Rebuild) { return false; }
    // The build initialized the table before it built this holder; a clear guard would mean that never happened.
    if !PT_TABLE_GUARDS.iter().all(|guard| fields::read::<u8>(base, *guard).is_some_and(|g| g & 1 != 0)) {
        trace!("[GroupSwap] articles NOT rebuilt owner={} holder={:#x} why={} reason=trainer table not initialized", owner, holder, why);
        return false;
    }
    let slot = fields::read::<u32>(entry, SLOT).unwrap_or(u32::MAX);
    let build: unsafe extern "C" fn(usize, usize, u32, u64, u32, u32) = std::mem::transmute(base + POOL_BUILD);
    // `mov w3, #-1` in both native callers: x3 = 0x0000_0000_ffff_ffff (`0x3a6270` moves the full x3 on).
    build(holder, base + PT_ARTICLE_TABLE, 0, u64::from(u32::MAX), slot, 0);
    trace!("[GroupSwap] articles rebuilt owner={} holder={:#x} pool=0->{} slot={} why={}", owner, holder, pool_count(holder), slot, why);
    true
}
