//! Scene policy and fresh native readiness for the three exact Mii backends:
//! Training, local-wireless waiting and Quickplay waiting (the two waiting
//! backends share one PracticeScene shape and one live-shape gate).
pub(super) use crate::engine::mii::backend::Layout;
pub(super) fn current()->Option<Layout> {
    let mode=crate::runtime::lobby::game_mode::current()?;
    Layout::scene(mode.initialized,mode.melee,mode.mode,
        cfg!(feature="waiting-roster-reload") && super::lobby::active())
}
pub(super) fn cpu()->Option<u32> {Some(current()?.visible()[1])}
/// Registered created entries (value copy) that Mii owner checks may accept beside
/// the layout owners. Read outside native G; rechecks under G are physical only.
#[cfg(any(feature="mii-ordinary-player-alpha",feature="mii-ordinary-cpu-alpha"))]
pub(super) fn extras()->crate::engine::mii::capacity::Extras {
    #[cfg(feature="managed-third-source-alpha")]
    { super::native_source_capacity::managed::owned_extras() }
    #[cfg(not(feature="managed-third-source-alpha"))]
    { crate::engine::mii::capacity::Extras::NONE }
}
/// Raw current facts of created entry `slot` for `Extras::admit`. Reads only; safe
/// with native G held (no native call, no Rust lock). Callers have already
/// span-checked FE (0x5a88), E (0x260), D (0x3a0), O (0xb0) and the owner table (0x130).
#[cfg(any(feature="mii-ordinary-player-alpha",feature="mii-ordinary-cpu-alpha"))]
pub(super) unsafe fn extra_facts(table:usize,slot:usize,fe:usize,e:usize,d:usize,o:usize)
    ->crate::engine::mii::capacity::ExtraFacts {
    use crate::runtime::mii::mii_native_access::rd;
    crate::engine::mii::capacity::ExtraFacts {
        entry_owner:rd(fe,0xe0),row_owner:rd(e,0x14),descriptor_selector:rd(fe,4),
        entry_index:rd(fe,0x46),information_index:rd(d,0x72),object_index:rd(o,0x68),
        entry_kind:rd(fe,0x18),object_kind:rd(o,0x58),
        resource:rd(table,slot*8),alias_root:rd(table,0xe8+slot*8),selection:rd(e,0xa),
    }
}
pub(super) unsafe fn ready()->bool {
    let Some(layout)=current() else{return false;};
    if !layout.quantity(super::native_source::training_cpu_quantity()) {return false;}
    if layout==Layout::Training {return true;}
    let Some(helper)=super::native_source::entry(5) else{return false;};
    super::native_source::exact_waiting_layout_with(helper,5,layout,extras())
        && layout.visible().iter().copied().all(|id|super::native_source::entry(id).is_some_and(|entry|
            super::ready_single_mii(entry,id).is_some() || super::ready_forms(entry,id).is_some()))
}
