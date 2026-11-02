//! Layout lookups over the game's own UI structures (SPEC-core-05 §4.1): a holder's layout and the
//! layout's name. Reads only; never writes game memory. The registry walk of the earlier fork's
//! `layout_registry.rs` (ours-new, carried) is kept for `find`, which later steps use for the HUD rule.

use std::ffi::CStr;
use std::sync::atomic::{AtomicUsize, Ordering};

use skyline::nn::ui2d::Layout;

/// A holder's `nn::ui2d::Layout*` (`main+0x375e260`'s `ldr x0, [x23, #0x458]`).
const HOLDER_LAYOUT: usize = 0x458;
/// The manager's vector of live holders: `[begin, end)` at +0x0 / +0x8, in priority order.
const MANAGER_HOLDERS: usize = 0;

/// Absolute address of the manager-global slot (`main+0x593ba90`), decoded at install.
static MANAGER_SLOT: AtomicUsize = AtomicUsize::new(0);

pub fn set_manager_slot(slot: usize) {
    MANAGER_SLOT.store(slot, Ordering::Release);
}

/// The game loads the manager as `**slot`, exactly like its own `adrp`/`ldr`/`ldr` sequence.
unsafe fn manager() -> Option<*const u8> {
    let slot = MANAGER_SLOT.load(Ordering::Acquire) as *const *const *const u8;
    if slot.is_null() {
        return None;
    }
    let instance = *slot;
    if instance.is_null() {
        return None;
    }
    let manager = *instance;
    (!manager.is_null()).then_some(manager)
}

/// Live holders in the manager's priority order (empty before the manager exists).
pub unsafe fn holders() -> &'static [*const u8] {
    let Some(manager) = manager() else { return &[] };
    let vector = manager.add(MANAGER_HOLDERS) as *const *const *const u8;
    let (begin, end) = (*vector, *vector.add(1));
    if begin.is_null() || end < begin {
        return &[];
    }
    std::slice::from_raw_parts(begin, end.offset_from(begin) as usize)
}

/// A holder's layout; `None` for a null holder or layout.
pub unsafe fn layout_of(holder: *const u8) -> Option<*mut Layout> {
    if holder.is_null() {
        return None;
    }
    let layout = *(holder.add(HOLDER_LAYOUT) as *const *mut Layout);
    (!layout.is_null()).then_some(layout)
}

/// `Layout::layout_name` as bytes (compared against ASCII names; never converted to a string).
pub unsafe fn layout_name<'a>(layout: *mut Layout) -> Option<&'a [u8]> {
    let name = (*layout).layout_name;
    (!name.is_null()).then(|| CStr::from_ptr(name).to_bytes())
}

/// The first live layout named `name`, in the manager's priority order.
pub unsafe fn find(name: &[u8]) -> Option<*mut Layout> {
    holders().iter().filter_map(|holder| layout_of(*holder)).find(|layout| layout_name(*layout) == Some(name))
}