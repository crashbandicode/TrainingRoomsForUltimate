//! K15 root fix, native side of `swap_model::copy_packages`. This is the only caller of the Kirby
//! copy package acquirer `0x17efb40` and its release `0x17f0290` outside native's own load
//! completion and construction tails.
//!
//! Per role, the bank holds what native's completion would take for every member set the bank may
//! register (the attached window and each group with bodies or keys). The roster is the one
//! `0x17e4f50` would write at that registration. Each holding is released with the identical
//! roster bytes, member and costume. A registration is admitted only once G68's own readiness
//! (vtable `+0x30`, as `0x17eb140(G)` asks it) is true. No lock is held across a native call.
//!
//! Lines: `[CopyPkg] sync owner={} reason={} sets={:x?} acquired={} released={} held={} keys={:x?}`,
//! `[CopyPkg] loading owner={} keys={:x?}`, `[CopyPkg] forget owner={} held={} reason={}`.

use crate::engine::seams::native_fields as fields;
use crate::engine::compat::fixed::DebugIter;
use std::sync::Mutex;
use crate::engine::swap::swap_model::copy_packages::{self as model, Acquisition};
use crate::engine::swap::swap_model::copy_registry::{Record, Roster, ENTRIES, SUBS};

macro_rules! trace {
    ($($arg:tt)*) => { crate::engine::compat::logging::diagnostic(format_args!($($arg)*)) };
}

const ACQUIRE: usize = 0x17efb40;
const RELEASE: usize = 0x17f0290;
/// `*(*(main+0x52b94f8))` is the fighter manager: `+0x20 + 8i` its entries (`0x17e5580–0x17e5594`).
const MANAGER_SLOT: usize = 0x52b94f8;
const OWNER_TABLE: usize = 0x5324680;
const G68_SLOT: usize = 0x68;
const REGION_WORD: usize = 0x523d00c;

static HELD: [Mutex<Vec<Acquisition>>; 2] = [Mutex::new(Vec::new()), Mutex::new(Vec::new())];

/// Both native routines consume the roster synchronously; callees receive only kind/costume
/// or a key, never its address. Keep the value receipt and reconstruct identical bytes for release.
/// Owned-ELF evidence: docs/provenance/pkg6-copy-roster-lifetime.md.
unsafe fn call(function: unsafe extern "C" fn(usize, *const u8, u32, u32), g68: usize, acquisition: &Acquisition) {
    let bytes = model::roster_bytes(&acquisition.roster);
    function(g68, bytes.as_ptr(), acquisition.member, acquisition.costume);
}

unsafe fn g68(base: usize) -> Option<usize> {
    fields::child(fields::child(base, OWNER_TABLE)?, G68_SLOT)
}

/// The roster `0x17e4f50` writes now: per manager slot `i`, `src = E_i+0x70` while
/// `E_i+0x5935 != 0 && E_i+0x591c <= 4`, else `E_i` (`0x17e5594–0x17e57a0`).
unsafe fn roster_now(base: usize) -> Roster {
    let mut roster = [Record::DEFAULT; ENTRIES];
    let Some(manager) = fields::child(base, MANAGER_SLOT).and_then(|slot| fields::child(slot, 0)) else { return roster; };
    for (i, record) in roster.iter_mut().enumerate() {
        let Some(entry) = fields::child(manager, 0x20 + i * 8) else { continue; };
        let staged = fields::read::<u8>(entry, 0x5935).unwrap_or(0) != 0 && fields::read::<u8>(entry, 0x591c).unwrap_or(u8::MAX) <= 4;
        let src = if staged { entry + 0x70 } else { entry };
        let kinds: [u32; SUBS] = std::array::from_fn(|j| fields::read::<u32>(src, 0x18 + 4 * j).unwrap_or(u32::MAX));
        *record = Record { entry: fields::read::<i32>(entry, 0xe0).unwrap_or(-1), kinds,
            costume: u32::from(fields::read::<u8>(src, 0x41).unwrap_or(0)) };
    }
    roster
}

/// What the bank should hold for `sets` (the window and every group with bodies or keys).
unsafe fn wanted(base: usize, owner: u32, r: usize, sets: &[[u32; SUBS]]) -> impl Iterator<Item = Acquisition> + Clone + '_ {
    let roster = roster_now(base);
    let costume = fields::read::<u32>(r, 0x64).unwrap_or(0);
    let region = fields::read::<u32>(base, REGION_WORD).unwrap_or(0);
    sets.iter().flat_map(move |set| model::acquisitions_iter(&roster, owner, *set, costume, region))
}

/// Bring the role's holdings to `sets`. Acquisitions come first, then releases, so a key two sets
/// share never drops to zero in between. Returns whether anything changed.
///
/// # Safety
/// `base` is the main image base; `r` is the role's proved resource owner in this callback; no
/// bank lock is held.
pub(crate) unsafe fn sync(base: usize, role: usize, owner: u32, r: usize, sets: &[[u32; SUBS]], reason: &str) -> bool {
    let Some(g68) = g68(base) else { return false; };
    let wanted = wanted(base, owner, r, sets);
    let plan = {
        let held = HELD[role].lock().unwrap_or_else(|p| p.into_inner());
        model::sync_iter(held.iter().copied(), wanted)
    };
    if plan.acquire.is_empty() && plan.release.is_empty() { return false; }
    let acquire: unsafe extern "C" fn(usize, *const u8, u32, u32) = std::mem::transmute(base + ACQUIRE);
    let release: unsafe extern "C" fn(usize, *const u8, u32, u32) = std::mem::transmute(base + RELEASE);
    let mut fresh = Vec::with_capacity(plan.acquire.len());
    for acquisition in plan.acquire {
        call(acquire, g68, &acquisition);
        fresh.push(acquisition);
    }
    let released: Vec<Acquisition> = {
        let mut held = HELD[role].lock().unwrap_or_else(|p| p.into_inner());
        plan.release.iter().map(|i| held.remove(*i)).collect()
    };
    for gone in &released {
        call(release, g68, gone);
    }
    {
        let mut held = HELD[role].lock().unwrap_or_else(|p| p.into_inner());
        let added = fresh.len();
        held.extend(fresh);
        trace!("[CopyPkg] sync owner={} reason={} sets={:x?} acquired={} released={} held={}", owner, reason, sets, added,
            released.len(), held.len());
        trace!("[CopyPkg] held owner={} acquisitions={} keys={:x?}", owner, held.len(),
            DebugIter(model::referenced_keys(held.iter().copied())));
    }
    true
}

/// Sync for `sets`, then whether every package the bank holds is resident: G68's own readiness
/// (vtable `+0x30`). Vacuously true when the bank holds nothing.
///
/// # Safety
/// As for [`sync`].
pub(crate) unsafe fn admit(base: usize, role: usize, owner: u32, r: usize, sets: &[[u32; SUBS]]) -> bool {
    sync(base, role, owner, r, sets, "admission");
    if HELD[role].lock().unwrap_or_else(|p| p.into_inner()).is_empty() { return true; }
    let Some(g68) = g68(base) else { return false; };
    let Some(ready) = fields::child(g68, 0).and_then(|vtable| fields::read::<usize>(vtable, 0x30)).filter(|f| *f != 0) else { return false; };
    let ready: unsafe extern "C" fn(usize) -> bool = std::mem::transmute(ready);
    let resident = ready(g68);
    if !resident {
        let held = HELD[role].lock().unwrap_or_else(|p| p.into_inner());
        trace!("[CopyPkg] loading owner={} keys={:x?}", owner, DebugIter(model::referenced_keys(held.iter().copied())));
    }
    resident
}

/// Drop the role's holdings without native calls: the scene is ending and its loop resets every
/// holder, G68 included (`0x134efb4`, FEASIBILITY §4.2).
pub(crate) fn forget(role: usize, owner: u32, reason: &str) {
    let mut held = HELD[role].lock().unwrap_or_else(|p| p.into_inner());
    if !held.is_empty() {
        trace!("[CopyPkg] forget owner={} held={} reason={}", owner, held.len(), reason);
        held.clear();
    }
}
