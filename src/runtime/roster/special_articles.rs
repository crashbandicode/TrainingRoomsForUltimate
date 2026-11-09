//! PT-TRAINER-PARK, native side of `swap_model::special_articles`. This is the only caller of the PT article-pool
//! builder `0x3a6270` outside native's window constructor (`0x658db8`) and its in-place rebuild `0x65a650`. It is
//! also the only caller of the pool release `0x3a6b80` on a holder whose window stays alive (a detached PT group).
//! The holder passed in has been validated by the caller (`valid_special_object`: vtable `main+0x4f626e8`,
//! `+0x230` = owner). No bank lock is held across a native call.
//!
//! Lines:
//! - `[GroupSwap] articles retired owner= holder= pool=a->b why=`;
//! - `[GroupSwap] articles rebuilt owner= holder= pool=0->n slot= why=`;
//! - `[GroupSwap] articles NOT rebuilt owner= holder= why= reason=`;
//! - `[GroupSwap] articles deactivated owner= holder= pool= why=` (inbox 02:00Z, the exit's tick);
//! - `[GroupSwap] articles retired owner= holder= pool=a->0 why= stage=destroy gone=` (the last stage).
//!
//! Inbox 02:00Z: a detach's retirement runs in stages (`model::RetireStage`): [`deactivate_saved`] on the exit's tick,
//! [`release_sub_pools`] and [`destroy_saved`] on later ones. Together they do what `0x3a6b80` does in one call.

use crate::engine::seams::native_fields as fields;
use crate::engine::swap::swap_model::special_articles::{self as model, ArticleStep};
use std::sync::atomic::{AtomicU8, Ordering};

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
/// `holder+0x18`: the articles pass 1 deactivates; `holder+0x20..`: the objects pass 2 destroys; `holder+0x228`: the
/// table the build recorded (`0x3a6cb8` clears it).
const ARTICLES: usize = 0x18;
const OBJECTS: usize = 0x20;
const TABLE: usize = 0x228;
/// The 0x238-byte holder class every pool `0x3a6270` fills (`entry+0x5a88`, and an article module's `+0x240`,
/// constructed at `0x33b98d4`).
const HOLDER_VTABLE: usize = 0x4f626e8;
/// `0x3ac560(id)`: BattleObjectManager's object of `id`, or null when its slot no longer holds that id (the slot's `+8`
/// is compared with the whole id).
pub(crate) const OBJECT_LOOKUP: usize = 0x3ac560;
pub(crate) const OBJECT_LOOKUP_WORDS: [u32; 4] = [0x531c7c08, 0x7100111f, 0x54000768, 0x90020869];
/// `0x3aeb50(id)`: destroy by id (the object's own `vt+0x50`, then its slot freed). It reads the slot without a null check.
pub(crate) const OBJECT_DESTROY: usize = 0x3aeb50;
pub(crate) const OBJECT_DESTROY_WORDS: [u32; 4] = [0xa9bd57f6, 0xa9014ff4, 0xa9027bfd, 0x910083fd];
/// Pass 2's one-time static (`0x3a6c34-0x3a6c58`): guard `0x52b7100`, object `0x52b70f0` built by `0x3a9ab0(obj, 0)`,
/// then its vtable `0x4f62078`; `__cxa_guard_acquire/release` are `0x39c06b0`/`0x39c06c0`.
pub(crate) const STATIC_INIT: usize = 0x3a9ab0;
pub(crate) const STATIC_INIT_WORDS: [u32; 4] = [0xb0025dc8, 0x91332108, 0xa9000408, 0xb4000041];
const STATIC_GUARD: usize = 0x52b7100;
const STATIC_OBJECT: usize = 0x52b70f0;
const STATIC_VTABLE: usize = 0x4f62078;
pub(crate) const GUARD_ACQUIRE: usize = 0x39c06b0;
pub(crate) const GUARD_RELEASE: usize = 0x39c06c0;
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

/// The article count as both passes of `0x3a6b80` read it (`and x8, x8, #0xffffffff`).
unsafe fn pool_len(holder: usize) -> usize {
    (pool_count(holder) & 0xffff_ffff) as usize
}

/// Pass 1 of `0x3a6b80` (`0x3a6ba0-0x3a6c04`) as native runs it: each article at `holder+0x18[i]` deactivated
/// (`vt+0xd0` true: `vt+0x20(a, 1)`; else `a+8 -> +0x98 -> vt+0x48(m, 1)`). Nothing is destroyed.
unsafe fn deactivate_articles(holder: usize) {
    for i in 0..pool_len(holder) {
        let Some(list) = fields::child(holder, ARTICLES) else { return; };
        let Some(article) = fields::child(list, i * 8) else { continue; };
        let Some(vtable) = fields::child(article, 0) else { continue; };
        let live: unsafe extern "C" fn(usize) -> u32 = std::mem::transmute(fields::read_unchecked::<usize>(vtable, 0xd0));
        if live(article) & 1 != 0 {
            let remove: unsafe extern "C" fn(usize, u32) = std::mem::transmute(fields::read_unchecked::<usize>(vtable, 0x20));
            remove(article, 1);
        } else if let Some(module) = fields::child(article, 8).and_then(|accessor| fields::child(accessor, 0x98)) {
            let Some(table) = fields::child(module, 0) else { continue; };
            let remove: unsafe extern "C" fn(usize, u32) = std::mem::transmute(fields::read_unchecked::<usize>(table, 0x48));
            remove(module, 1);
        }
    }
}

/// The battle object id a pool object names (`vt+0x30`, as pass 2 reads it at `0x3a6c64` and `0x3a6c90`).
unsafe fn object_id(object: usize) -> Option<u32> {
    let vtable = fields::child(object, 0)?;
    let id: unsafe extern "C" fn(usize) -> u32 = std::mem::transmute(fields::read_unchecked::<usize>(vtable, 0x30));
    Some(id(object))
}

unsafe fn lookup(base: usize, id: u32) -> usize {
    let lookup: unsafe extern "C" fn(u32) -> usize = std::mem::transmute(base + OBJECT_LOOKUP);
    lookup(id)
}

/// Pass 2's one-time static (`0x3a6c88`, `0x3a6c34-0x3a6c58`), built as native builds it.
unsafe fn static_init(base: usize) {
    let guard = base + STATIC_GUARD;
    // `ldarb w8, [x20]; tbz w8, #0`
    if (*(guard as *const AtomicU8)).load(Ordering::Acquire) & 1 != 0 { return; }
    let acquire: unsafe extern "C" fn(usize) -> u32 = std::mem::transmute(base + GUARD_ACQUIRE);
    if acquire(guard) == 0 { return; }
    let init: unsafe extern "C" fn(usize, usize) = std::mem::transmute(base + STATIC_INIT);
    init(base + STATIC_OBJECT, 0);
    fields::write_unchecked(base + STATIC_OBJECT, 0, base + STATIC_VTABLE);
    let release: unsafe extern "C" fn(usize) = std::mem::transmute(base + GUARD_RELEASE);
    release(guard);
}

/// `model::RetireStage::Deactivate`, on the exit's tick: pass 1 only, when the model retires this window. The trainer
/// leaves the world at once (`0x33a1c90`: `bo+0x61` cleared, its modules off); the pool keeps its count until
/// [`destroy_saved`]. Returns whether it did.
///
/// # Safety
/// As for [`retire_saved`].
pub(crate) unsafe fn deactivate_saved(owner: u32, flags: u8, holder: usize, valid: bool, why: &str) -> bool {
    let count = if valid { pool_count(holder) } else { 0 };
    if model::on_detach(flags, valid, count) != Some(ArticleStep::Retire) { return false; }
    deactivate_articles(holder);
    trace!("[GroupSwap] articles deactivated owner={} holder={:#x} pool={} why={}", owner, holder, count, why);
    true
}

/// `model::RetireStage::SubPools`, a later tick: the deactivated trainer's own article pools
/// (`[[bo+0x20]+0x98]+0x240`, which its teardown releases first through `0x3d3a90`) through `0x3a6b80`. Only a pool
/// of the holder class goes; anything else stays for the teardown, as native leaves it. Returns `(pools, articles)`.
///
/// # Safety
/// `holder` is a pool [`deactivate_saved`] deactivated and nothing has destroyed since.
pub(crate) unsafe fn release_sub_pools(base: usize, holder: usize) -> (usize, u64) {
    let mut out = (0, 0);
    for i in 0..pool_len(holder) {
        let Some(object) = fields::child(holder, OBJECTS + i * 8) else { continue; };
        let Some(id) = object_id(object) else { continue; };
        let battle = lookup(base, id);
        if battle == 0 { continue; }
        let Some(pool) = fields::child(battle, 0x20)
            .and_then(|accessor| fields::child(accessor, 0x98))
            .and_then(|module| fields::child(module, 0x240))
            .filter(|pool| fields::read::<usize>(*pool, 0) == Some(base + HOLDER_VTABLE)) else { continue; };
        let count = pool_count(pool);
        if count == 0 { continue; }
        release(base, pool);
        out = (out.0 + 1, out.1 + count);
    }
    out
}

/// `model::RetireStage::Destroy`, a later tick: pass 2 of `0x3a6b80` (`0x3a6c08-0x3a6cb8`) as native runs it: each
/// object destroyed by id, its slot cleared, then `+0x10 = 0`, `+0x228 = 0`. The one difference: an id `0x3ac560` no
/// longer finds (freed since pass 1) is not handed to `0x3aeb50`, which would read the null slot. Returns how many.
///
/// # Safety
/// As for [`release_sub_pools`].
pub(crate) unsafe fn destroy_saved(base: usize, owner: u32, holder: usize, why: &str) -> usize {
    let count = pool_count(holder);
    let destroy: unsafe extern "C" fn(u32) = std::mem::transmute(base + OBJECT_DESTROY);
    let mut gone = 0;
    for i in 0..pool_len(holder) {
        let slot = holder + OBJECTS + i * 8;
        let Some(object) = fields::child(slot, 0) else { continue; };
        let Some(id) = object_id(object) else { continue; };
        let found = lookup(base, id);
        if found == 0 || fields::read::<u8>(found, 0x3a).unwrap_or(0) <= 3 { static_init(base); }
        if found == 0 {
            gone += 1;
        } else {
            destroy(object_id(object).unwrap_or(id));
        }
        fields::write_unchecked(slot, 0, 0usize);
    }
    fields::write_unchecked(holder, POOL_COUNT, 0u64);
    fields::write_unchecked(holder, TABLE, 0usize);
    trace!("[GroupSwap] articles retired owner={} holder={:#x} pool={}->{} why={} stage=destroy gone={}",
        owner, holder, count, pool_count(holder), why, gone);
    gone
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
