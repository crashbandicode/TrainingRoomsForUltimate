//! Value-only capacity contract for cycle preloading. Counts and backend limits
//! live here; the slot order is the single `mii::capacity` contract that Mii
//! owner checks also use. Native ownership must still qualify every entry.
use crate::engine::mii::backend::Layout;

pub use crate::engine::mii::capacity::{capacity_index, extra_slots, Extras, ALL_EXTRA_SLOTS, BASE_HELPERS, EXTRA_SLOTS, MAX_TARGETS, SOURCE_SLOTS};
pub const ABSENT_SLOT: u32 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallbackIdentity {
    pub layout: Layout,
    pub epoch: crate::engine::swap::source_lifetime::SceneEpoch,
    pub entry: usize,
}

/// Slot four is a visible native CPU in waiting. Only a live creation receipt
/// matching the current Training scene and callback may dispatch it as a source.
pub fn source_callback(slot: u32, third: bool, current: Option<CallbackIdentity>,
    registered: Option<CallbackIdentity>) -> bool {
    matches!(slot, 2 | 3 | 5 | 6) || (third && slot == 7)
        || (third && slot == 4 && current.is_some_and(|identity|
            identity.layout == Layout::Training && identity.entry != 0
                && registered == Some(identity)))
}


/// Potential entries, not permission to create or dereference them. Training
/// extensions and solo waiting extensions require separate opt-in features.
pub fn slots(layout: Layout, third: bool, ldn_third: bool, extended: bool)
    -> [u32; SOURCE_SLOTS] {
    slots_with_waiting(layout, third, ldn_third, extended, false)
}

pub fn slots_with_waiting(layout: Layout, third: bool, ldn_third: bool, extended: bool, waiting: bool)
    -> [u32; SOURCE_SLOTS] {
    let mut slots = [ABSENT_SLOT; SOURCE_SLOTS];
    slots[..BASE_HELPERS].copy_from_slice(&layout.helpers());
    let extra_count = if !third { 0 } else {
        match layout {
            Layout::Training => if extended { EXTRA_SLOTS.len() } else { 1 },
            Layout::Ldn => if waiting { extra_slots(layout).len() } else { usize::from(ldn_third) },
            Layout::Quickplay => if waiting { extra_slots(layout).len() } else { 0 },
        }
    };
    for (target, slot) in slots[BASE_HELPERS..].iter_mut().zip(extra_slots(layout).iter().take(extra_count)) {
        *target = *slot as u32;
    }
    slots
}

pub fn supported_count(layout: Layout, third: bool, ldn_third: bool, extended: bool) -> usize {
    slots(layout, third, ldn_third, extended).iter().filter(|slot| **slot != ABSENT_SLOT).count()
}

pub fn target_count(requested: u8, layout: Layout, third: bool, ldn_third: bool, extended: bool) -> usize {
    usize::from(requested).min(MAX_TARGETS).min(supported_count(layout, third, ldn_third, extended))
}

pub fn target_count_with_waiting(requested: u8, layout: Layout, third: bool, ldn_third: bool,
    extended: bool, waiting: bool) -> usize {
    usize::from(requested).min(MAX_TARGETS).min(slots_with_waiting(layout,third,ldn_third,extended,waiting)
        .iter().filter(|slot| **slot != ABSENT_SLOT).count())
}

/// Count parked alternatives eligible for the same-entry reuse route. The
/// active fighter is baseline; canonical original restoration is not a cache hit.
/// Its runtime caller is the native body bank (`native-body-bank-alpha`); the host policy tests
/// (`src/cycle_preload_policy.rs`) check it too.
pub fn reusable_extras(kinds: &[u32], active: u32, canonical_original: Option<u32>) -> u8 {
    kinds.iter().filter(|&&kind| kind != active && Some(kind) != canonical_original).count() as u8
}

