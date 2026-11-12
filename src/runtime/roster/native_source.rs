//! Native source-owner operations shared by copy abilities and character items.
//! Callers establish slot ownership and retain leases. This module neither picks
//! arbitrary spare entries nor changes native fighter quantities/active counts.
use super::{aligned, native_fighter_install, pointer_chain, read, setting, Forms, BASE};
use std::sync::atomic::Ordering;
use smash::app::{self, lua_bind::*};
use {crate::engine::swap::character_forms::typed_record_kind, crate::engine::swap::trio::TrioFighter};

pub use crate::engine::swap::source_lifetime::NativeIdentity as ReadySource;

/// Reacquire the actual native entry. Returned accessors must be consumed in the
/// current native callback, never saved and dereferenced by a later request.
pub unsafe fn entry(slot: u32) -> Option<*const u8> {
    if slot >= 8 || BASE.load(Ordering::Acquire) == 0 { return None; }
    let symbol = setting(&crate::runtime::compat::common::FIGHTER_MANAGER_ADDR);
    if symbol == 0 { return None; }
    let manager = std::ptr::read_volatile(symbol as *const *mut app::FighterManager);
    if !aligned(manager.cast::<u8>()) { return None; }
    let current = FighterManager::get_fighter_entry(manager, app::FighterEntryID(slot as i32))
        as *const u8;
    (aligned(current) && read::<u32>(current, 0xe0) == slot).then_some(current)
}

pub unsafe fn training_cpu_quantity() -> Option<u32> {
    if !crate::runtime::compat::common::is_native_training_mode() || BASE.load(Ordering::Acquire) == 0
        || !crate::runtime::roster::cpu_quantity_guarded() {
        return None;
    }
    let root = std::ptr::read_volatile(
        (BASE.load(Ordering::Acquire) + 0x52c3420) as *const *const u8);
    let menu = pointer_chain(root, &[8, 0, 0xb38, 0, 0x210, 8, 0x80])?;
    Some(read(menu, 0xb40))
}

/// Exact waiting shape for a new 5/6 admission: manager count 4, primaries 0/4/5/6,
/// no backups, CPU rows 4/5/6 flagged present. Local wireless witnessed it at
/// runtime; Quickplay waiting is built by the same PracticeScene setup (13.0.5
/// `0x1790020`, reached from both waiting controllers without a mode branch), so
/// the same live check admits or refuses it. Refuse every other layout. Reads only.
/// The selection table is consulted only after the manager shape matches, because
/// those records can stay stale when native setup does not rebuild them. Selection
/// record +0 is an opaque word, not a fighter kind, and is not read here.
pub unsafe fn exact_waiting_layout(current: *const u8, slot: u32) -> bool {
    exact_waiting_layout_with(current, slot, crate::engine::mii::backend::Layout::Ldn,
        crate::engine::mii::capacity::Extras::NONE)
}

/// The same live check with a value snapshot of registered created entries.
/// No registry/transaction lock is taken here. Callers capture extras before G;
/// native roots, count, backups and every row are freshly checked in this call.
pub unsafe fn exact_waiting_layout_with(current: *const u8, slot: u32,
    layout: crate::engine::mii::backend::Layout,
    extras: crate::engine::mii::capacity::Extras) -> bool {
    if !matches!(slot, 5 | 6) || !aligned(current) || read::<u32>(current, 0xe0) != slot {
        return false;
    }
    let base = BASE.load(Ordering::Acquire);
    let symbol = setting(&crate::runtime::compat::common::FIGHTER_MANAGER_ADDR);
    if symbol == 0 || base == 0 { return false; }
    let manager = std::ptr::read_volatile(symbol as *const *const u8);
    if !aligned(manager) { return false; }
    let internal = read::<*const u8>(manager, 0);
    if !aligned(internal) { return false; }
    let primary = read::<[usize;8]>(internal,0x20);
    let backup = read::<[usize;8]>(internal,0x60);
    if !layout.waiting_map(&primary,&backup,read(internal,0xa0),extras)
        || primary[slot as usize] != current as usize { return false; }
    // Rows may be stale outside this exact live owner map.
    let record = |owner: usize| (base + 0x52c5730 + owner * 0x260) as *const u8;
    let fallback = |owner: usize| {
        let row = record(owner);
        read::<u8>(row, 0xa) == 1
            && read::<u8>(row, 0x79) == 1
            && read::<u8>(row, 0xa7) == owner as u8
    };
    fallback(5) && fallback(6)
        && read::<u8>(record(0), 0xa) != 2
        && read::<u8>(record(4), 0xa) != 2
        && [1usize, 2, 3, 7].iter().all(|owner| read::<u8>(record(*owner), 0xa) == 2)
}

/// Current archive owner; its costume is inherited through the preserved native
/// descriptor. No unverified costume-field offset is written into that descriptor.
pub(super) unsafe fn resource_owner(slot: u32) -> Option<*mut u8> {
    if slot >= 8 || BASE.load(Ordering::Acquire) == 0 { return None; }
    let context = std::ptr::read_volatile(
        (BASE.load(Ordering::Acquire) + 0x5324680) as *const *const u8);
    if !aligned(context) { return None; }
    let owner = read::<*mut u8>(context, 0xe8 + slot as usize * 8);
    aligned(owner).then_some(owner)
}

pub unsafe fn inherited_costume(slot: u32) -> Option<u32> {
    Some(read(resource_owner(slot)?, 0x64))
}

pub(super) fn ordinary_kind(kind: u32) -> bool {
    let forms = Forms::starting(kind);
    !(0x48..=0x4a).contains(&kind) && TrioFighter::from_kind(kind).is_some() && forms.count == 1 && forms.actor_count == 1
}

/// Capability-scoped created Mii readiness. This does not replace the caller's
/// current map/registered-roots/G payload validation. Raw reads only, including
/// the same descriptor/body/loader checks as native helpers; safe with G held.
pub unsafe fn parked_created_ready(current:*const u8,slot:u32,kind:u32,
    destination:crate::engine::mii::ordinary_cpu::CreatedDestination)->bool {
    destination.record()==slot as u16 && slot<8
        && destination.owner().entry==current as usize
        && (0x48..=0x4a).contains(&kind) && aligned(current)
        && read::<u32>(current,0xe0)==slot
        && crate::runtime::mii::mii_native_access::cpu::helper_ready_for_kind(
            BASE.load(Ordering::Acquire),current as usize,slot,kind)
}

/// Completion of a non-activating native install, including every form in the
/// canonical group. Grouped parked installs still require runtime qualification.
pub unsafe fn parked_ready(current: *const u8, slot: u32, kind: u32) -> bool {
    if matches!(slot,2|3|5|6) && (0x48..=0x4a).contains(&kind) {
        return aligned(current) && read::<u32>(current,0xe0)==slot
            && crate::runtime::mii::mii_native_access::cpu::helper_ready_for_kind(BASE.load(Ordering::Acquire),current as usize,slot,kind);
    }
    if (0x48..=0x4a).contains(&kind) || TrioFighter::from_kind(kind).is_none() || !aligned(current)
        || read::<u32>(current, 0xe0) != slot
    { return false; }
    let forms = Forms::starting(kind);
    if read::<u32>(current, 0x14) != forms.count
        || read::<u32>(current, 0x28) != forms.actor_count
        || read::<u64>(current, 0x4150) != u64::from(forms.count)
        || !matches!(read::<u8>(current, 0x5920), 0 | 1)
        || read::<u8>(current, 0x591c) != 14
        || read::<u8>(current, 0x5935) != 0
    { return false; }
    for (index, expected) in forms.kinds.iter().copied().enumerate() {
        if read::<u32>(current, 0x18 + index * 4) != expected { return false; }
        if index >= forms.count as usize { continue; }
        let body = read::<*const u8>(current, 0x4160 + index * 8);
        if !aligned(body)
            || read::<usize>(body, 0) != BASE.load(Ordering::Acquire) + 0x4f80210
            || read::<u32>(body, 0xc) != expected
            || read::<u64>(body, 0x10) != u64::from(slot)
            || read::<u8>(body, 0x3a) != 3
            || !aligned(read::<*const u8>(body, 0x20))
        { return false; }
    }
    true
}

/// A window cell is a body only when `installed` publishes that index
/// (`entry_cells::active_cell`, the game's `b.ls`). An unpublished cell is not read.
unsafe fn published_body(entry: *const u8, installed: u64, index: usize) -> Option<*const u8> {
    let Ok(index) = u8::try_from(index) else { return None; };
    if crate::model::entry_cells::active_cell(index, installed).is_none() {
        return None;
    }
    let body = read::<*const u8>(entry, 0x4160 + usize::from(index) * 8);
    if aligned(body) { Some(body) } else { None }
}

unsafe fn identity(current: *const u8, slot: u32, kind: u32, owner: *mut u8) -> ReadySource {
    let forms = Forms::starting(kind);
    let installed = read::<u64>(current, 0x4150);
    let body_id = published_body(current, installed, 0).map(|body| read::<u32>(body, 8)).unwrap_or(0);
    ReadySource { slot, entry: current as usize, kind,
        costume: read(owner, 0x64), body_id,
        bodies: std::array::from_fn(|i| {
            if i >= forms.count as usize { return (0, 0); }
            match published_body(current, installed, i) {
                Some(body) => (body as usize, read::<u32>(body, 8)),
                None => (0, 0),
            }
        }), descriptor: read(current, 0xf8), resource: owner as usize }
}

unsafe fn typed_ready(requested: u32, kind: u32) -> bool {
    let Some(record_kind) = typed_record_kind(requested, kind) else { return false; };
    let registry = std::ptr::read_volatile(
        (BASE.load(Ordering::Acquire) + 0x52bc3b0) as *const *const u8);
    if !aligned(registry) { return false; }
    let record = registry.add(0x60 + record_kind as usize * 0x38);
    read::<u32>(record, 0) > 0
        && aligned(read::<*const u8>(record, 0x10))
        && aligned(read::<*const u8>(record, 0x18))
}

/// Archive readiness and parsed fighter parameters are separate prerequisites.
/// Optional copy files may all be absent; native copy_setup permits that case.
pub unsafe fn ready(current: *const u8, slot: u32, kind: u32) -> Option<ReadySource> {
    if !parked_ready(current, slot, kind) { return None; }
    resources_ready(current, slot, kind)
}

/// Full parked created-source readiness, after its explicit destination proof.
pub unsafe fn ready_created(current: *const u8, slot: u32, kind: u32,
    destination: crate::engine::mii::ordinary_cpu::CreatedDestination) -> Option<ReadySource> {
    if !parked_created_ready(current, slot, kind, destination) { return None; }
    resources_ready(current, slot, kind)
}

unsafe fn resources_ready(current: *const u8, slot: u32, kind: u32) -> Option<ReadySource> {
    let owner = resource_owner(slot)?;
    let base = BASE.load(Ordering::Acquire);
    let archive_ready: unsafe extern "C" fn(*mut u8) -> bool =
        std::mem::transmute(base + 0x17e2880);
    if read::<u32>(owner, 0x58) != kind || !archive_ready(owner) { return None; }
    let forms = Forms::starting(kind);
    for member in forms.kinds.iter().copied().take(forms.count as usize) {
        if !typed_ready(kind, member) { return None; }
    }
    let lookup: unsafe extern "C" fn(*mut u32, *mut u8, u32, u32) =
        std::mem::transmute(base + 0x17e8890);
    let directory: unsafe extern "C" fn(u32) -> u32 = std::mem::transmute(base + 0x353ea90);
    for file_kind in 0x20..=0x22 {
        let mut file = 0xffffff;
        lookup(&mut file, owner, kind, file_kind);
        if file != 0xffffff && directory(file) == 0xffffff { return None; }
    }
    Some(identity(current, slot, kind, owner))
}

/// Invoke only from this entry's native loader callback, after the caller has
/// acquired the shared install gate and proven this source is not leased.
pub unsafe fn install(current: *mut u8, kind: u32) {
    install_with(current, kind, native_fighter_install::Flags::Parked);
}

unsafe fn install_with(current: *mut u8, kind: u32, flags: native_fighter_install::Flags) {
    let forms = Forms::starting(kind);
    let mut descriptor = native_fighter_install::Descriptor::copy_from(current);
    descriptor.request_group(native_fighter_install::Group {
        count: forms.count,
        kinds: forms.kinds,
        actor_count: forms.actor_count,
    });
    native_fighter_install::install(
        BASE.load(Ordering::Acquire),
        current,
        &descriptor,
        flags,
    );
}
