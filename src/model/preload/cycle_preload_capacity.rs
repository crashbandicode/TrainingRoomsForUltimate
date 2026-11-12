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

#[cfg(test)]
mod callback_tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::swap::source_lifetime::SourceBook;

    #[test]
    fn count_promotion_dispatch_accepts_only_current_owned_training_four() {
        let mut book = SourceBook::<1, 1>::new();
        let current = CallbackIdentity {layout: Layout::Training, epoch: book.epoch(), entry: 0x4000};
        assert!(source_callback(4, true, Some(current), Some(current)));
        assert!(!source_callback(4, false, Some(current), Some(current)));
        assert!(!source_callback(4, true, Some(current), None));
        assert!(!source_callback(4, true, Some(current), Some(CallbackIdentity {entry: 0x5000, ..current})));
        for layout in [Layout::Ldn, Layout::Quickplay] {
            let waiting = CallbackIdentity {layout, ..current};
            assert!(!source_callback(4, true, Some(waiting), Some(waiting)));
            assert!(!source_callback(4, true, Some(waiting), Some(current)));
        }
        book.depart_scene().unwrap();
        let next = CallbackIdentity {epoch: book.epoch(), ..current};
        assert!(!source_callback(4, true, Some(next), Some(current)));
        assert!(!source_callback(4, true, None, Some(current)));
        assert!(!source_callback(0, true, Some(current), Some(current)));
        let absent = CallbackIdentity {entry: 0, ..current};
        assert!(!source_callback(4, true, Some(absent), Some(absent)));
        for slot in [2, 3, 5, 6] { assert!(source_callback(slot, false, None, None)); }
        assert!(source_callback(7, true, None, None));
        assert!(!source_callback(7, false, None, None));
    }
    }
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

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    fn every_requested_budget_obeys_the_backend_limit() {
        assert_eq!(supported_count(Layout::Training,true,true,true), 6);
        assert_eq!(MAX_TARGETS, 5, "renderer demand must not extend the saved preload menu");
        for count in 0..=u8::MAX {
            assert_eq!(target_count(count,Layout::Training,true,true,true),usize::from(count).min(MAX_TARGETS));
            assert_eq!(target_count(count,Layout::Ldn,true,true,true),usize::from(count).min(3));
            assert_eq!(target_count(count,Layout::Quickplay,true,true,true),usize::from(count).min(2));
        }
        for (i, slot) in EXTRA_SLOTS.iter().enumerate() {assert_eq!(capacity_index(*slot),Some(i));}
        assert_eq!(capacity_index(4),Some(3));
        assert_eq!(capacity_index(8),None);
    }
    fn waiting_capacity_is_opt_in_and_uses_its_own_plan() {
        for layout in [Layout::Ldn, Layout::Quickplay] {
            assert_eq!(slots_with_waiting(layout,true,true,true,true),[5,6,7,3,2,8]);
            assert_eq!(slots_with_waiting(layout,false,true,true,true),[5,6,8,8,8,8]);
            for count in 0..=u8::MAX {
                assert_eq!(target_count_with_waiting(count,layout,true,true,true,true),usize::from(count).min(5));
            }
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("every_requested_budget_obeys_the_backend_limit", every_requested_budget_obeys_the_backend_limit as fn()),
            ("waiting_capacity_is_opt_in_and_uses_its_own_plan", waiting_capacity_is_opt_in_and_uses_its_own_plan as fn()),
        ]);
    }
    }
}
