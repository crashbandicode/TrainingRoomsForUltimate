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

/// Pins every kind of `group` R's tree holds; true when all of them are pinned (now or by an earlier keep). A group not
/// wholly pinned keeps none of its new pins.
pub(crate) unsafe fn pin(r: Option<usize>, role: usize, group: Members) -> bool {
    let (Some(r), Some((base, manager))) = (r, manager()) else { return false; };
    let mut pins = PINS[role].lock();
    let before = pins.len();
    for &kind in &group.kinds()[..group.count()] {
        if pins.iter().any(|(k, _)| *k == kind) { continue; }
        let Some(handle) = (pins.len() < PIN_CAPACITY).then(|| take(base, manager, r, kind)).flatten() else {
            while pins.len() > before { release(base, manager, pins.pop().unwrap_or_default().1); }
            return false;
        };
        pins.push((kind, handle));
    }
    true
}

pub(crate) fn held(role: usize) -> bool { !PINS[role].lock().is_empty() }

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
