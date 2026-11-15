//! rc.14 custom stages (Stage Builder), runtime side, entirely through the game's own memory and its own reads (the
//! primary 2026-11-15 ~05:18Z; boot cs-proof-2 showed that TR may never open the save filesystem itself).
//!
//! - **The list.** The stage save object `H = *(*(*(main+0x5314510))+0x298)` keeps every saved stage's 0x198-byte
//!   header at `H+0x18/+0x20` (the full records `H+0/+8` inside Stage Builder only; probe cs-probe-y1). On MainThread,
//!   only while no save operation runs (`H+0x68` null), `service` copies id and name of at most 64 into a fixed
//!   buffer; the menu worker turns it into the list and publishes it with the kept order.
//! - **The data.** Outside Stage Builder no record is in memory, so TR reads one exactly as local wireless does
//!   (`0x2336560`/`0x23367c0`): queue the id (`0x17cfd40`), begin (`0x17cfeb0`), let the game's frames run the storage,
//!   then, when the game's idle condition holds, `0x17d0120` on the real `H` and the rule writes, then the cleanup
//!   (`0x17cfbc0`). One read at a time, started only in Training and the waiting-room practice (`ReadMachine`); a
//!   storage the game replaced is dropped without cleanup. A Change Stage pick waits for its read; Training Stage
//!   Rotation and the waiting rooms read ahead the stage their next entry will take.
//! - **The rule.** `RuleOwner` (unchanged): TR writes the fields the game's own load writes, frees only what it took
//!   out, and gives the rule back when the practice scene ends.
//!
//! Guards (approved fail-safe): the `custom_stages` seam rows, stage table row 325, the game's idle condition, the rule
//! index ≤ 2. On any failure nothing is called; the caller keeps a normal stage and posts the approved notice.
//! Design and evidence: `artifacts/rc14-custom-stages/DESIGN.md`.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use hos_lock::Mutex;
use skyline::hooks::{getRegionAddress, Region};

use crate::engine::stages::custom_stages::{self as model, CustomStage, Lists, Observed, Published, ReadAction, ReadMachine,
    RuleOwner, RulePlan, RuleSlot};

const LOADER: usize = 0x17d0120;
const QUEUE: usize = 0x17cfd40;
const BEGIN: usize = 0x17cfeb0;
const CLEANUP: usize = 0x17cfbc0;
const FREE: usize = 0x392eb40;
/// The active rule index (0..=2; the game aborts above 2).
const RULE_INDEX: usize = 0x530a968;
/// Rule `r`: `+ r * 0x40`: u16 +0, u32 +4, u16 +8, data vector +0x10, extra vector +0x28.
const RULE: usize = 0x530a818;
/// Rule `r`'s custom stage id: `+ r * 0x30`.
const RULE_ID: usize = 0x530a8fc;
/// The stage save object `H = *(*(*(main+0x5314510))+0x298)` and the busy word `[[main+0x593b218]+0x50]`.
const HOLDER_ROOT: usize = 0x5314510;
const BUSY_ROOT: usize = 0x593b218;
/// `H`'s fields: full records, headers, the storage, a read's records.
const H_RECORDS: usize = 0x00;
const H_HEADERS: usize = 0x18;
const H_STORAGE: usize = 0x68;
const H_READ: usize = 0x70;
const HEADER_LEN: usize = 0x198;
/// The game's own limit on saved stages (`0x1cadbc0` refuses a new one past 99 headers).
const GAME_MAX: usize = 99;
const LIST_MAX: usize = 64;
const NAME_AT: usize = 0x110;
const NAME_UNITS: usize = 22;
/// The stage table row of kind 325 (`SP_Edit`): word 0, and the package hash at +24.
const STAGE_TABLE: usize = 0x454a7e8;
const SP_EDIT_WORD: u64 = 0x79_0000_0145;
const SP_EDIT_HASH: u64 = 0x14_d051_407d;

/// The seam rows the custom stages rely on (`native_seams::rows_custom_stages`, group `custom_stages`, authoritative).
const SEAMS: &[&str] = &["custom_stages.loader", "custom_stages.free", "custom_stages.game_load_holder",
    "custom_stages.game_load_idle", "custom_stages.game_load_call", "custom_stages.game_load_scalars",
    "custom_stages.game_load_vectors", "custom_stages.game_load_vectors+0x40", "custom_stages.queue",
    "custom_stages.begin", "custom_stages.cleanup", "custom_stages.lw_request", "custom_stages.lw_cleanup",
    "custom_stages.header_list", "custom_stages.record_list", "custom_stages.setter_save_byte"];

static VERIFIED: AtomicBool = AtomicBool::new(false);
/// The save's list and the kept order (menu worker → hooks). The worker holds it only to swap the `Arc`.
static PUBLISHED: Mutex<Option<Arc<Published>>> = Mutex::new(None);
/// TR's hold on the rule (MainThread only).
static OWNER: Mutex<RuleOwner> = Mutex::new(RuleOwner::new());
/// The read through the game's save object (MainThread only).
static READ: Mutex<ReadMachine> = Mutex::new(ReadMachine::new());
/// The loaded data of one stage, built by the game's loader right after its read (the save object is cleaned up at
/// once, so nothing of TR's stays in it): the rule fields and the data vector (a game allocation TR owns until the
/// rule takes it, or frees with the game's free). MainThread only.
static PREPARED: Mutex<Option<RuleSlot>> = Mutex::new(None);
/// The game mode word the rule load was made in: the departure restores only once the scene has changed.
static LOADED_MODE: AtomicU32 = AtomicU32::new(0);
/// The game mode word the last scene-change check saw.
static LAST_MODE: AtomicU32 = AtomicU32::new(0);
/// The Direct custom stage the missing notice was posted for (once per stage per boot).
static NOTICED: AtomicU32 = AtomicU32::new(u32::MAX);
/// A Change Stage pick waiting for its read (0: none); it wins over the read-ahead.
static PICK: AtomicU32 = AtomicU32::new(0);
/// Frames since the last read-ahead choice and list snapshot (MainThread).
static FRAMES: AtomicU32 = AtomicU32::new(0);
/// A scene changed: take the list again at the next in-scope frame.
static SNAP_WANTED: AtomicBool = AtomicBool::new(true);

#[derive(Clone, Copy)]
struct Listed {
    id: u32,
    name: [u16; NAME_UNITS],
}

/// The game's list as MainThread last copied it (fixed size: no heap in hooks).
struct Snapshot {
    count: usize,
    entries: [Listed; LIST_MAX],
}

static SNAPSHOT: Mutex<Snapshot> =
    Mutex::new(Snapshot { count: 0, entries: [Listed { id: 0, name: [0; NAME_UNITS] }; LIST_MAX] });
/// Bumped when a snapshot differs from the one before; the worker takes the list when it is not what it took.
static SNAP_SEQ: AtomicU32 = AtomicU32::new(0);
static TAKEN_SEQ: AtomicU32 = AtomicU32::new(0);

fn log(args: std::fmt::Arguments<'_>) {
    crate::engine::compat::logging::diagnostic(args);
}

fn base() -> usize {
    // SAFETY: the main module's text base; a constant for the process.
    unsafe { getRegionAddress(Region::Text) as usize }
}

/// Boot: the rows decide (authoritative group), and the stage table's row 325 must be `SP_Edit`.
pub(crate) fn init() {
    let base = base();
    let rows = crate::runtime::seams::seam_install::gate("custom_stages", SEAMS, true);
    let row = base + STAGE_TABLE + model::CUSTOM_STAGE_KIND as usize * 72;
    // SAFETY: the stage table is static data of the main image (read-only, mapped for the process).
    let (word, hash) = unsafe {
        (crate::engine::seams::native_fields::read_unchecked::<u64>(row, 0),
         crate::engine::seams::native_fields::read_unchecked::<u64>(row, 24) & 0xff_ffff_ffff)
    };
    let table = word == SP_EDIT_WORD && hash == SP_EDIT_HASH;
    let verified = rows && table;
    VERIFIED.store(verified, Ordering::Release);
    log(format_args!("[CustomStages] verified={verified} rows={rows} table={table}"));
}

#[repr(C)]
#[derive(Default)]
struct MemoryInfo { address: u64, size: u64, kind: u32, attributes: u32, permission: u32, ipc: u32, device: u32, padding: u32 }

/// Horizon QueryMemory (the pinned SVC ABI of `practice_stage_request::span`): `[address, address + size)` is mapped
/// readable. Mapping is not lifetime: the game's objects are read on MainThread, between its own updates.
unsafe fn readable(address: usize, size: usize) -> bool {
    let Some(end) = address.checked_add(size) else { return false };
    if address < 0x10000 || address & 3 != 0 { return false; }
    let mut info = MemoryInfo::default();
    let result: u64;
    core::arch::asm!("svc 0x6", inout("x0") &mut info as *mut MemoryInfo as u64 => result,
        lateout("x1") _, in("x2") address as u64, clobber_abi("C"), options(nostack));
    result as u32 == 0 && info.permission & 1 == 1 && info.address <= address as u64
        && info.address.checked_add(info.size).is_some_and(|e| end as u64 <= e)
}

unsafe fn pointer_at(at: usize) -> Option<usize> {
    (at & 7 == 0 && readable(at, 8)).then(|| crate::engine::seams::native_fields::read_unchecked::<usize>(at, 0))
}

/// The stage save object `H` (`custom_stages.game_load_holder`).
unsafe fn holder(base: usize) -> Option<usize> {
    let root = pointer_at(base + HOLDER_ROOT).filter(|p| *p != 0)?;
    let manager = pointer_at(root).filter(|p| *p != 0)?;
    let slot = pointer_at(manager + 0x298).filter(|p| *p != 0)?;
    pointer_at(slot).filter(|p| *p != 0)
}

/// What `H` shows now: its storage, and the game's own idle condition (`custom_stages.game_load_idle`: the storage null,
/// or its state 0 and the busy word 0).
unsafe fn observe(base: usize, in_scope: bool) -> Option<(usize, Observed)> {
    let h = holder(base)?;
    let storage = pointer_at(h + H_STORAGE)?;
    let idle = if storage == 0 {
        true
    } else {
        if !readable(storage + 0x448, 4) {
            return None;
        }
        let state = crate::engine::seams::native_fields::read_unchecked::<u32>(storage, 0x448);
        let busy_root = pointer_at(base + BUSY_ROOT).filter(|p| *p != 0)?;
        let busy_owner = pointer_at(busy_root).filter(|p| *p != 0)?;
        if !readable(busy_owner + 0x50, 8) {
            return None;
        }
        state == 0 && crate::engine::seams::native_fields::load_acquire_u64(busy_owner + 0x50) == 0
    };
    Some((h, Observed { storage, idle, in_scope }))
}

/// The game's list into the fixed buffer (`custom_stages.header_list`): headers `H+0x18`, else records `H+0`. Only
/// while no save operation runs. Returns whether the list changed.
unsafe fn snapshot(h: usize) -> Option<bool> {
    let (mut begin, mut end) = (pointer_at(h + H_HEADERS)?, pointer_at(h + H_HEADERS + 8)?);
    let mut stride = HEADER_LEN;
    if begin == end {
        (begin, end) = (pointer_at(h + H_RECORDS)?, pointer_at(h + H_RECORDS + 8)?);
        stride = model::STAGE_FILE_LEN;
    }
    if end < begin || (end - begin) % stride != 0 || (end - begin) / stride > GAME_MAX {
        return None;
    }
    let count = (end - begin) / stride;
    let mut snap = SNAPSHOT.try_lock()?;
    let mut changed = snap.count != count.min(LIST_MAX);
    let mut n = 0;
    for i in 0..count.min(LIST_MAX) {
        let at = begin + i * stride;
        if !readable(at, NAME_AT + NAME_UNITS * 2) {
            return None;
        }
        let id: u32 = crate::engine::seams::native_fields::read_unchecked(at, 0);
        let mut name = [0u16; NAME_UNITS];
        for (k, unit) in name.iter_mut().enumerate() {
            *unit = crate::engine::seams::native_fields::read_unchecked(at, NAME_AT + k * 2);
        }
        if id == 0 {
            continue;
        }
        changed |= snap.entries[n].id != id || snap.entries[n].name != name;
        snap.entries[n] = Listed { id, name };
        n += 1;
    }
    changed |= snap.count != n;
    snap.count = n;
    Some(changed)
}

// ---- menu worker side ----

/// The save's Stage Builder stages in the game's order, from the game's own list (MainThread's last snapshot). `None`
/// before the first snapshot: no custom tile shows and nothing is pruned.
pub(crate) fn read_list() -> Option<Vec<CustomStage>> {
    let seq = SNAP_SEQ.load(Ordering::Acquire);
    if seq == 0 {
        return None;
    }
    let snap = SNAPSHOT.lock();
    let list = snap.entries[..snap.count].iter()
        .map(|e| CustomStage { id: e.id, name: model::name_from_units(e.id, &e.name) }).collect();
    drop(snap);
    TAKEN_SEQ.store(seq, Ordering::Release);
    Some(list)
}

/// The list and the order the hooks build the stage cycles' lists from.
pub(crate) fn publish(published: Published) {
    log(format_args!("[CustomStages] published customs={} order={:?} direct={:?}",
        published.customs.len(), published.order.entries, published.order.direct_custom));
    *PUBLISHED.lock() = Some(Arc::new(published));
}

/// What the worker published last (the menu's custom tiles); `None` before the game's list was first read.
pub(crate) fn published() -> Option<Arc<Published>> {
    PUBLISHED.lock().clone()
}

/// The worker's pass: a snapshot it has not taken yet.
pub(crate) fn refresh_wanted() -> bool {
    SNAP_SEQ.load(Ordering::Acquire) != TAKEN_SEQ.load(Ordering::Acquire)
}

/// Once per game frame (`lobby::update`, inside the input hooks: atomics only): a new scene asks for a new snapshot.
pub(crate) fn frame() {
    let Some(mode) = crate::runtime::lobby::game_mode::current() else { return };
    if LAST_MODE.swap(mode.mode, Ordering::AcqRel) != mode.mode {
        SNAP_WANTED.store(true, Ordering::Release);
    }
}

// ---- MainThread side ----

/// Every in-scope practice frame (Training, the waiting-room practice: `practice_stage_request::update`): the list
/// snapshot, the read-ahead choice and one step of the read.
pub(crate) fn service(in_training: bool) {
    if !VERIFIED.load(Ordering::Acquire) {
        return;
    }
    let base = base();
    let frames = FRAMES.fetch_add(1, Ordering::Relaxed);
    // SAFETY: MainThread, inside the StageManager's own update; pointer reads behind QueryMemory checks.
    let Some((h, o)) = (unsafe { observe(base, true) }) else { return };
    if (SNAP_WANTED.load(Ordering::Acquire) || frames % 300 == 0) && o.storage == 0 {
        // SAFETY: as above; no game operation runs on H (its storage is null).
        if let Some(changed) = unsafe { snapshot(h) } {
            SNAP_WANTED.store(false, Ordering::Release);
            if changed || SNAP_SEQ.load(Ordering::Acquire) == 0 {
                SNAP_SEQ.fetch_add(1, Ordering::AcqRel);
                crate::runtime::ui::worker::wake();
            }
        }
    }
    let Some(mut read) = READ.try_lock() else { return };
    if frames % 60 == 0 || PICK.load(Ordering::Acquire) != 0 {
        let pick = PICK.load(Ordering::Acquire);
        let ahead = if pick != 0 { Some(pick) } else { read_ahead(in_training) };
        read.want(ahead);
        if let Some(id) = read.take_stale() {
            // SAFETY: MainThread; the data TR built for `id` and never gave to the rule.
            unsafe { free_prepared(base, id) };
        }
    }
    match read.step(o) {
        ReadAction::None => {}
        ReadAction::Start(id) => {
            // SAFETY: the game's own read request, in local wireless's order (`custom_stages.queue`, `.begin`,
            // `.lw_request`), only with no game operation running on H (ReadMachine).
            let storage = unsafe {
                let queue: extern "C" fn(usize, u32) = std::mem::transmute(base + QUEUE);
                let begin: extern "C" fn(usize) = std::mem::transmute(base + BEGIN);
                queue(h, id);
                begin(h);
                pointer_at(h + H_STORAGE).unwrap_or(0)
            };
            read.started(id, storage);
            log(format_args!("[CustomStages] read start id={id} storage=0x{storage:x}"));
        }
        ReadAction::Cleanup => {
            // SAFETY: the storage is TR's own read and idle (ReadMachine); `custom_stages.cleanup`, `.lw_cleanup`.
            unsafe {
                let cleanup: extern "C" fn(usize) = std::mem::transmute(base + CLEANUP);
                cleanup(h + H_STORAGE);
            }
            log(format_args!("[CustomStages] read cleanup (unused)"));
        }
    }
    if let Some(id) = read.wanted().filter(|id| read.ready(*id, o)) {
        // SAFETY: TR's own finished read, the storage idle (ReadMachine::ready).
        unsafe { prepare_now(base, h, id, &mut read) };
    }
}

/// The game's loader over TR's finished read, at once (`custom_stages.loader`): the stage's rule fields and data vector
/// are kept by TR, and the save object is cleaned up in the game's order (`custom_stages.cleanup`, `.lw_cleanup`).
unsafe fn prepare_now(base: usize, h: usize, id: u32, read: &mut ReadMachine) {
    let cleanup: extern "C" fn(usize) = std::mem::transmute(base + CLEANUP);
    if !read_holds(h, id) {
        // The game's read finished without this record (a deleted stage).
        cleanup(h + H_STORAGE);
        read.spoiled();
        log(format_args!("[CustomStages] read id={id} found no record"));
        return;
    }
    let Some(mut prepared) = PREPARED.try_lock() else { return };
    // The loader's stage-entry argument: scratch (it sets +0x18, the music; TR leaves the rule's entry as it is).
    let mut entry = [0u64; 4];
    let mut out = [0u64; 8];
    let loader: extern "C" fn(usize, u32, *mut u64, *mut u64) = std::mem::transmute(base + LOADER);
    loader(h, id, entry.as_mut_ptr(), out.as_mut_ptr());
    cleanup(h + H_STORAGE);
    read.consumed();
    read.have(Some(id));
    let loaded = RuleSlot {
        head: out[0] as u16, word: (out[0] >> 32) as u32, tail: out[1] as u16,
        data: [out[2], out[3], out[4]], extra: [0; 3], id,
    };
    if let Some(old) = prepared.replace(loaded) {
        if old.data[0] != 0 {
            // SAFETY: the data vector of a stage TR built and never gave to the rule; the game's own free.
            let free: extern "C" fn(u64) = std::mem::transmute(base + FREE);
            free(old.data[0]);
        }
    }
    log(format_args!("[CustomStages] read id={id} prepared head=0x{:04x} data=0x{:x}+{}",
        loaded.head, loaded.data[0], loaded.data[1].wrapping_sub(loaded.data[0])));
}

/// The data read ahead for `id` outlived its use: the stage was deleted, or another stage (or none) is next
/// (`ReadMachine::take_stale`). Freed with the game's free, as a replaced slot is in `prepare_now`.
unsafe fn free_prepared(base: usize, id: u32) {
    let Some(mut prepared) = PREPARED.try_lock() else { return };
    let Some(slot) = prepared.filter(|slot| slot.id == id) else { return };
    *prepared = None;
    if slot.data[0] != 0 {
        let free: extern "C" fn(u64) = std::mem::transmute(base + FREE);
        free(slot.data[0]);
    }
    log(format_args!("[CustomStages] read-ahead freed id={id} data=0x{:x} (no longer next)", slot.data[0]));
}

/// The custom stage the next entry will take: Training Stage Rotation's next entry in Training, the waiting room's
/// next entry otherwise.
fn read_ahead(in_training: bool) -> Option<u32> {
    let next = if in_training {
        crate::runtime::lobby::training_stage_rotation::peek_next()
            .or_else(|| crate::runtime::lobby::lobby_stage::peek_next())
    } else {
        crate::runtime::lobby::lobby_stage::peek_next()
    };
    next.and_then(|s| model::decode(s.stage))
}

/// Change Stage picked custom stage `id` (0: the pick is settled): its read starts at the next in-scope frame.
pub(crate) fn pick(id: u32) {
    if PICK.swap(id, Ordering::AcqRel) != id {
        PICK_FRAMES.store(0, Ordering::Release);
    }
}

/// Frames a Change Stage pick has waited for its read; past `PICK_FRAMES_MAX` it fails (the read could not start).
static PICK_FRAMES: AtomicU32 = AtomicU32::new(0);
const PICK_FRAMES_MAX: u32 = 900;

/// Where a Change Stage pick's read is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Readiness {
    Ready,
    Waiting,
    Failed,
}

/// MainThread: the read of `id` is done (the load may run), still running, or failed.
pub(crate) fn readiness(id: u32) -> Readiness {
    if PREPARED.try_lock().is_some_and(|p| p.is_some_and(|slot| slot.id == id)) {
        return Readiness::Ready;
    }
    let Some(read) = READ.try_lock() else { return Readiness::Waiting };
    let failed = read.wanted() == Some(id) && read.failed();
    if failed || PICK_FRAMES.fetch_add(1, Ordering::AcqRel) > PICK_FRAMES_MAX {
        Readiness::Failed
    } else {
        Readiness::Waiting
    }
}

/// `lists` without the Direct notice (the read-ahead's look at the next entry).
pub(crate) fn lists_quiet(legacy: crate::engine::stages::lobby_stages::LobbyStageOrder,
    prefs: &crate::engine::stages::stage_menu::Preferences) -> Result<Lists, &'static str> {
    let published = PUBLISHED.try_lock().and_then(|p| p.clone());
    model::lists(legacy, prefs, published.as_deref())
}

/// The stage cycles' lists (`custom_stages::lists`) with what the worker published; hooks never wait for it.
pub(crate) fn lists(legacy: crate::engine::stages::lobby_stages::LobbyStageOrder,
    prefs: &crate::engine::stages::stage_menu::Preferences) -> Result<Lists, &'static str> {
    let published = PUBLISHED.try_lock().and_then(|p| p.clone());
    let lists = model::lists(legacy, prefs, published.as_deref())?;
    if let Some(name) = lists.missing.as_deref().filter(|_| lists.direct_mode) {
        // Once per deleted Direct stage per boot: the lists are built at every entry.
        let id = published.as_deref().and_then(|p| p.order.direct_custom).unwrap_or(0);
        if NOTICED.swap(id, Ordering::AcqRel) != id {
            missing(name);
        }
    }
    Ok(lists)
}

/// The approved notice for a custom stage that is gone or cannot be loaded: the caller goes to (or stays on) a normal
/// stage.
fn missing(name: &str) {
    crate::runtime::lobby::training::ui::notifications::clear_notification("Practice Stage");
    // restriction: custom-stage-missing
    crate::runtime::lobby::training::ui::notifications::notification("Practice Stage".into(), model::missing_notice(name).into(), 240);
}

/// The reason Change Stage gives when a custom stage was not loaded: the approved notice is already up (`refused`), so
/// the request's own "Stage unchanged" notice is left out (`practice_stage_request`).
pub(crate) const NOTICE_POSTED: &str = "custom stage not loaded";

/// A custom stage TR was about to go to cannot be loaded: the approved notice, with the stage's name when known.
pub(crate) fn refused(id: u32, why: &str) {
    let name = PUBLISHED.try_lock().and_then(|p| p.clone())
        .and_then(|p| model::name_of(id, &p.customs).map(str::to_owned).or_else(|| p.order.names.get(&id).cloned()))
        .unwrap_or_else(|| format!("stage {id}"));
    log(format_args!("[CustomStages] load not done id={id} why={why}"));
    missing(&name);
}

unsafe fn read_slot(base: usize, index: u32) -> RuleSlot {
    use crate::engine::seams::native_fields::read_unchecked as rd;
    let rule = base + RULE + index as usize * 0x40;
    RuleSlot {
        head: rd(rule, 0), word: rd(rule, 4), tail: rd(rule, 8),
        data: [rd(rule, 0x10), rd(rule, 0x18), rd(rule, 0x20)],
        extra: [rd(rule, 0x28), rd(rule, 0x30), rd(rule, 0x38)],
        id: rd(base + RULE_ID + index as usize * 0x30, 0),
    }
}

unsafe fn apply_plan(base: usize, plan: RulePlan) {
    use crate::engine::seams::native_fields::write_unchecked as wr;
    if let Some((index, slot)) = plan.write {
        let rule = base + RULE + index as usize * 0x40;
        wr(rule, 0, slot.head);
        wr(rule, 4, slot.word);
        wr(rule, 8, slot.tail);
        for (i, v) in slot.data.iter().chain(&slot.extra).enumerate() {
            wr(rule, 0x10 + i * 8, *v);
        }
        wr(base + RULE_ID + index as usize * 0x30, 0, slot.id);
    }
    // SAFETY: the game heap's free (`custom_stages.free`); every pointer is a game allocation TR took out of the rule.
    let free: extern "C" fn(u64) = std::mem::transmute(base + FREE);
    for p in plan.freed() {
        free(p);
    }
}

/// Whether the read's records `H+0x70/+0x78` hold the record of `id` (what the loader will search).
unsafe fn read_holds(h: usize, id: u32) -> bool {
    let (Some(begin), Some(end)) = (pointer_at(h + H_READ), pointer_at(h + H_READ + 8)) else { return false };
    if begin == 0 || end < begin || (end - begin) % model::STAGE_FILE_LEN != 0 {
        return false;
    }
    (0..(end - begin) / model::STAGE_FILE_LEN).any(|i| {
        let at = begin + i * model::STAGE_FILE_LEN;
        readable(at, 4) && crate::engine::seams::native_fields::read_unchecked::<u32>(at, 0) == id
    })
}

/// MainThread, just before TR asks for stage 325: custom stage `id` goes into the active rule, from the data TR built
/// with the game's loader right after the game's read (`prepare_now`). Returns the byte a stage-325 descriptor carries
/// at +0x98 (the setter's copy of rule byte 0x819). Not read yet: nothing is written, the read is wanted for the next
/// time, and the caller keeps a normal stage.
pub(crate) unsafe fn load(id: u32) -> Result<u8, &'static str> {
    if !VERIFIED.load(Ordering::Acquire) {
        return Err("custom stages need the 13.0.5 game");
    }
    let base = base();
    let index: u32 = crate::engine::seams::native_fields::read_unchecked(base + RULE_INDEX, 0);
    if index > 2 {
        return Err("rule index out of range");
    }
    let mut owner = OWNER.try_lock().ok_or("custom stage load busy")?;
    if let Some(held) = owner.index().filter(|held| *held != index) {
        // The active rule changed since TR's load: give that rule back first.
        let plan = owner.depart(read_slot(base, held));
        apply_plan(base, plan);
    }
    let current = read_slot(base, index);
    if owner.holds(index, current, id) {
        return Ok((current.head >> 8) as u8);
    }
    let mut prepared = PREPARED.try_lock().ok_or("custom stage data busy")?;
    let Some(loaded) = prepared.filter(|slot| slot.id == id) else {
        drop(prepared);
        if let Some(mut read) = READ.try_lock() {
            read.want(Some(id));
        }
        return Err("custom stage not read yet");
    };
    *prepared = None;
    drop(prepared);
    if let Some(mut read) = READ.try_lock() {
        read.have(None);
    }
    let plan = owner.load(index, current, loaded);
    log(format_args!("[CustomStages] load id={id} rule={index} head=0x{:04x} data=0x{:x}+{} free={}",
        loaded.head, loaded.data[0], loaded.data[1].wrapping_sub(loaded.data[0]), plan.freed().count()));
    apply_plan(base, plan);
    if let Some(mode) = crate::runtime::lobby::game_mode::current() {
        LOADED_MODE.store(mode.mode, Ordering::Release);
    }
    Ok((loaded.head >> 8) as u8)
}
/// The byte a stage-325 descriptor carries at +0x98 for the active rule as it is (Training Stage Rotation's reapply).
pub(crate) unsafe fn save_byte() -> Option<u8> {
    let base = base();
    let index: u32 = crate::engine::seams::native_fields::read_unchecked(base + RULE_INDEX, 0);
    (VERIFIED.load(Ordering::Acquire) && index <= 2).then(|| (read_slot(base, index).head >> 8) as u8)
}

/// MainThread, at the practice scene's departure (`practice_stage_request::depart`): once the scene has changed, the rule
/// gets back what it held before TR's first load (or, when the game wrote it since, TR frees what it kept).
pub(crate) fn depart() {
    let Some(mut owner) = OWNER.try_lock() else { return };
    let Some(index) = owner.index() else { return };
    let Some(mode) = crate::runtime::lobby::game_mode::current() else { return };
    if mode.mode == LOADED_MODE.load(Ordering::Acquire) {
        // Still the scene the load was made for (its scene-change notice can come after the load).
        return;
    }
    let base = base();
    // SAFETY: the rule is a static of the main image; MainThread is the game's only writer of it in these scenes.
    unsafe {
        let plan = owner.depart(read_slot(base, index));
        log(format_args!("[CustomStages] depart rule={index} restored={} free={}", plan.write.is_some(), plan.freed().count()));
        apply_plan(base, plan);
    }
}
