//! Value-only capacity contract for cycle preloading. Counts and backend limits
//! live here; the slot order is the single `mii::capacity` contract that Mii
//! owner checks also use. Native ownership must still qualify every entry.
use crate::engine::mii::backend::Layout;

pub use crate::engine::mii::capacity::{capacity_index, extra_slots, Extras, ALL_EXTRA_SLOTS, BASE_HELPERS, EXTRA_SLOTS, MAX_TARGETS};
pub const ABSENT_SLOT: u32 = 8;

/// Potential entries, not permission to create or dereference them. Training
/// extensions and solo waiting extensions require separate opt-in features.
pub fn slots(layout: Layout, third: bool, ldn_third: bool, extended: bool)
    -> [u32; MAX_TARGETS] {
    slots_with_waiting(layout, third, ldn_third, extended, false)
}

pub fn slots_with_waiting(layout: Layout, third: bool, ldn_third: bool, extended: bool, waiting: bool)
    -> [u32; MAX_TARGETS] {
    let mut slots = [ABSENT_SLOT; MAX_TARGETS];
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
    usize::from(requested).min(supported_count(layout, third, ldn_third, extended))
}

pub fn target_count_with_waiting(requested: u8, layout: Layout, third: bool, ldn_third: bool,
    extended: bool, waiting: bool) -> usize {
    usize::from(requested).min(slots_with_waiting(layout,third,ldn_third,extended,waiting)
        .iter().filter(|slot| **slot != ABSENT_SLOT).count())
}

/// Count parked alternatives eligible for the same-entry reuse route. The
/// active fighter is baseline; canonical original restoration is not a cache hit.
/// Its runtime caller is the native body bank (`native-body-bank-alpha`); the host policy tests
/// (`src/cycle_preload_policy.rs`) check it too.
pub fn reusable_extras(kinds: &[u32], active: u32, canonical_original: Option<u32>) -> u8 {
    kinds.iter().filter(|&&kind| kind != active && Some(kind) != canonical_original).count() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_training_can_use_every_extra_and_waiting_seats_stay_reserved() {
        assert_eq!(slots(Layout::Training,true,true,true),[2,3,7,6,5]);
        assert_eq!(slots(Layout::Training,true,true,false),[2,3,7,8,8]);
        assert_eq!(slots(Layout::Ldn,true,true,true),[5,6,7,8,8]);
        assert_eq!(slots(Layout::Ldn,true,false,true),[5,6,8,8,8]);
        assert_eq!(slots(Layout::Quickplay,true,true,true),[5,6,8,8,8]);
        assert_eq!(slots(Layout::Training,false,true,true),[2,3,8,8,8]);
    }
    #[test]
    fn every_requested_budget_obeys_the_backend_limit() {
        for count in 0..=u8::MAX {
            assert_eq!(target_count(count,Layout::Training,true,true,true),usize::from(count).min(MAX_TARGETS));
            assert_eq!(target_count(count,Layout::Ldn,true,true,true),usize::from(count).min(3));
            assert_eq!(target_count(count,Layout::Quickplay,true,true,true),usize::from(count).min(2));
        }
        for (i, slot) in EXTRA_SLOTS.iter().enumerate() {assert_eq!(capacity_index(*slot),Some(i));}
        assert_eq!(capacity_index(4),None);
        assert_eq!(capacity_index(8),None);
    }
    #[test]
    fn waiting_capacity_is_opt_in_and_uses_its_own_plan() {
        for layout in [Layout::Ldn, Layout::Quickplay] {
            assert_eq!(slots_with_waiting(layout,true,true,true,true),[5,6,7,3,2]);
            assert_eq!(slots_with_waiting(layout,false,true,true,true),[5,6,8,8,8]);
            for count in 0..=u8::MAX {
                assert_eq!(target_count_with_waiting(count,layout,true,true,true,true),usize::from(count).min(MAX_TARGETS));
            }
        }
    }
}
