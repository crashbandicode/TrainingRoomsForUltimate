//! rc.13, (a′) proper (wa-fixes-logs/a-prime-re.md; probe wa-aprime-probe-9ad09558-2): a group kept across its owner's
//! target-first Reload keeps its fighter module. Loader state 3 releases every node of R's NRO tree (`0x17e48a0` →
//! `0x22b6f60`), so the kept bodies' module would live only on a helper seat's reference, and that seat's next Reload
//! unloads it under them. Before the Reload each kept kind's module gets one reference of ours ([`pin`], the way
//! `0x17e4900`'s found path takes one); once the new bank is Ready, `0x17e4900(R, kind)` gives R its node again and our
//! reference goes ([`settle`]). Nothing is pinned, and no group is kept, unless every `module_pins.*` row passed.
//! Game thread only, never under the bank lock.

use crate::engine::compat::fixed::FixedList;
use crate::engine::seams::native_fields as fields;
use crate::engine::swap::body_bank::{Members, PIN_CAPACITY};
use hos_lock::Mutex;
use std::mem::transmute;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) const SEAMS: &[&str] = &["module_pins.nro_request", "module_pins.module_release", "module_pins.mutex_lock",
    "module_pins.mutex_unlock", "module_pins.tree_clear", "module_pins.found_path"];
const HANDLE: u64 = 0xff_ffff_ffff;
/// One `(kind, module handle)` per pinned kind: a role's kept window and groups hold at most this many kinds (rc.14
/// a-full-2: derived from the group cap; a literal 9 for three groups before).
type Pins = FixedList<(u32, u64), PIN_CAPACITY>;
/// `main` once the rows passed; 0 pins nothing.
static BASE: AtomicUsize = AtomicUsize::new(0);
static PINS: [Mutex<Pins>; 2] = [Mutex::new(Pins::new((0, 0))), Mutex::new(Pins::new((0, 0)))];

pub(crate) fn install(base: usize, rows: bool) {
    BASE.store(if rows { base } else { 0 }, Ordering::Release);
}

/// `main` and the module manager `**(main+0x5327cd0)` (lock `+0x60`, record map root `+0x40`), once the rows passed.
unsafe fn manager() -> Option<(usize, usize)> {
    let base = Some(BASE.load(Ordering::Acquire)).filter(|base| *base != 0)?;
    Some((base, fields::child(fields::child(base, 0x5327cd0)?, 0)?))
}

/// The node of a libc++ tree rooted at `*(at)` whose 40-bit key is `key` (`left` +0, `right` +8, key +0x20).
unsafe fn find(at: usize, key: u64) -> Option<usize> {
    let mut node = fields::child(at, 0);
    for _ in 0..64 {
        let current = node?;
        let k = fields::read::<u64>(current, 0x20)? & HANDLE;
        if k == key { return Some(current); }
        node = fields::child(current, if k < key { 8 } else { 0 });
    }
    None
}

/// One reference on the module of R's node for `kind` (record node+0x28, refs `+0x134`), taken under the manager lock
/// (`0x39c1a40` / `0x39c1a50`); the module's handle.
unsafe fn take(base: usize, manager: usize, r: usize, kind: u32) -> Option<u64> {
    let handle = fields::read::<u64>(find(r + 0x4b8, u64::from(kind) * 0x2a)?, 0x28)? & HANDLE;
    let lock: unsafe extern "C" fn(usize) = transmute(base + 0x39c1a40);
    lock(manager + 0x60);
    let at = find(manager + 0x40, handle).and_then(|node| fields::child(node, 0x28));
    let refs = at.and_then(|at| fields::read::<i32>(at, 0x134)).filter(|n| *n > 0);
    if let (Some(at), Some(refs)) = (at, refs) { fields::write_unchecked(at, 0x134, refs + 1); }
    let unlock: unsafe extern "C" fn(usize) = transmute(base + 0x39c1a50);
    unlock(manager + 0x60);
    refs.map(|_| handle)
}

unsafe fn release(base: usize, manager: usize, handle: u64) {
    let release: unsafe extern "C" fn(usize, *const u64) = transmute(base + 0x22b6f60);
    release(manager, &handle);
}

/// Pins every kind of `group` R's tree holds (`body_bank::pin_kinds`: its members, then its object kind); true when all of
/// its members are pinned (now or by an earlier keep). A group not wholly pinned keeps none of its new pins. The object
/// kind is pinned whenever R has its node (a window's holder runs code from it; rc.14 a-full-2, boot a2-99b9083f-y1); a
/// detached group's holder without one keeps its own reference (`hold`), so its absence refuses nothing.
pub(crate) unsafe fn pin(r: Option<usize>, role: usize, group: Members) -> bool {
    let (Some(r), Some((base, manager))) = (r, manager()) else { return false; };
    let mut pins = PINS[role].lock();
    let before = pins.len();
    for (i, &kind) in crate::engine::swap::body_bank::pin_kinds(group).iter().enumerate() {
        if pins.iter().any(|(k, _)| *k == kind) { continue; }
        let object = i >= group.count();
        let Some(handle) = (pins.len() < PIN_CAPACITY).then(|| take(base, manager, r, kind)).flatten() else {
            if object { continue; }
            while pins.len() > before { release(base, manager, pins.pop().unwrap_or_default().1); }
            return false;
        };
        pins.push((kind, handle));
    }
    true
}

pub(crate) fn held(role: usize) -> bool { !PINS[role].lock().is_empty() }

/// rc.14 mem-headroom (diag census): how many kinds' modules this role's kept groups pin.
#[cfg(feature = "diag")]
pub(crate) fn count(role: usize) -> usize { PINS[role].lock().len() }

/// rc.14 a-full-2 (boot a2-99b9083f-y1, integration-3's smoke2: `0x33c23f8` read the vtable of an Aegis holder's article
/// in the unloaded `0x75` module): the bank gives a group's object-kind token back right after its holder step, which drops
/// R's node for that kind's NRO while the holder still runs code from it. Before that, the group takes one reference of
/// its own on the module of R's node for `kind` ([`take`]); it keeps it until the holder is destroyed ([`drop_hold`]) or
/// the holder becomes R's window ([`hand_to`]). The module's handle, or 0 when R holds no node for `kind` or the rows did
/// not pass.
pub(crate) unsafe fn hold(r: usize, kind: u32) -> u64 {
    let Some((base, manager)) = manager().filter(|_| r != 0) else { return 0; };
    take(base, manager, r, kind).unwrap_or(0)
}

/// The holder [`hold`] kept alive is destroyed: our reference goes.
pub(crate) unsafe fn drop_hold(handle: u64) {
    if handle == 0 { return; }
    if let Some((base, manager)) = manager() { release(base, manager, handle); }
}

/// The holder became `r`'s window: `r` gets its node for `kind` again (`0x17e4900`, as [`settle`] gives a kept kind back),
/// unless it still has one, then our reference goes. Native's next Reload then destroys the holder (state 2) before it
/// releases `r`'s tree (state 3). Whether `r` asked again.
pub(crate) unsafe fn hand_to(r: usize, kind: u32, handle: u64) -> bool {
    let Some((base, manager)) = manager().filter(|_| handle != 0) else { return false; };
    let asked = r != 0 && find(r + 0x4b8, u64::from(kind) * 0x2a).is_none();
    if asked {
        let request: unsafe extern "C" fn(usize, u32) = transmute(base + 0x17e4900);
        request(r, kind);
    }
    release(base, manager, handle);
    asked
}

/// The role's new bank is Ready on `r` (`Some`): R gets each pinned kind's node again (`0x17e4900`), then the pins go.
/// `None` (the bank is retired or its entry destroyed): the pins go alone.
pub(crate) unsafe fn settle(role: usize, r: Option<usize>) {
    let pins = std::mem::replace(&mut *PINS[role].lock(), Pins::new((0, 0)));
    let Some((base, manager)) = manager().filter(|_| !pins.is_empty()) else { return; };
    let request: unsafe extern "C" fn(usize, u32) = transmute(base + 0x17e4900);
    for &(kind, handle) in pins.iter() {
        if let Some(r) = r { request(r, kind); }
        release(base, manager, handle);
    }
    let (re_requested, pins) = (r.is_some(), &pins[..]);
    crate::engine::compat::logging::diagnostic(format_args!(
        "[GroupSwap] module pins released role={role} re_requested={re_requested} pins={pins:x?}"));
}
