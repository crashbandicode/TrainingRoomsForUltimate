use core::sync::atomic::{AtomicU8, Ordering};
use super::input_delay::Owners;

/// One variant per wizard; a new wizard adds its variant here and to `ALL`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { DrillStart, EggZone, TechWizard, TechPlacing, BttPlacing, RecordPlacing }

impl Kind {
    pub const ALL: [Self; 6] = [Self::DrillStart, Self::EggZone, Self::TechWizard, Self::TechPlacing, Self::BttPlacing, Self::RecordPlacing];
}

/// Which wizards are up, read lock-free by the HUD draw. The strip, its card and
/// the helper bar step aside while any is up.
pub struct Active { bits: AtomicU8 }

impl Default for Active {
    fn default() -> Self { Self::new() }
}

impl Active {
    pub const fn new() -> Self { Self { bits: AtomicU8::new(0) } }

    /// The one path every wizard reports through on entry and every exit.
    /// Returns whether that wizard was up before.
    pub fn set_up(&self, kind: Kind, up: bool) -> bool {
        let bit = 1u8 << kind as u8;
        let old = if up {
            self.bits.fetch_or(bit, Ordering::AcqRel)
        } else {
            self.bits.fetch_and(!bit, Ordering::AcqRel)
        };
        old & bit != 0
    }

    pub fn is_up(&self, kind: Kind) -> bool {
        self.bits.load(Ordering::Acquire) & (1u8 << kind as u8) != 0
    }
    pub fn any_up(&self) -> bool { self.bits.load(Ordering::Acquire) != 0 }
    /// The menu or clip editor covers the HUD, or any wizard is up.
    pub fn hides_strip(&self, owners: Owners) -> bool { owners.covers_hud() || self.any_up() }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn record_placing_is_a_shared_wizard_kind() {
        assert_eq!(Kind::ALL.len(), 6);
        assert!(Kind::ALL.contains(&Kind::RecordPlacing));
        let active = Active::new();
        assert!(!active.hides_strip(Owners::NONE));
        assert!(!active.set_up(Kind::RecordPlacing, true));
        assert!(active.is_up(Kind::RecordPlacing));
        assert!(active.hides_strip(Owners::NONE));
        assert!(active.set_up(Kind::RecordPlacing, false));
        assert!(!active.hides_strip(Owners::NONE));
    }

    fn set_up_reports_whether_the_wizard_was_up() {
        for kind in Kind::ALL {
            let active = Active::new();
            assert!(!active.set_up(kind, true), "{kind:?}");
            assert!(active.is_up(kind), "{kind:?}");
            assert!(active.any_up(), "{kind:?}");
            assert!(active.set_up(kind, true), "{kind:?}");
            assert!(active.set_up(kind, false), "{kind:?}");
            assert!(!active.is_up(kind), "{kind:?}");
            assert!(!active.any_up(), "{kind:?}");
            assert!(!active.set_up(kind, false), "{kind:?}");
        }
    }

    fn a_wizard_hides_and_restores_both_hud_renderers() {
        for kind in Kind::ALL {
            let active = Active::new();
            assert!(!active.hides_strip(Owners::NONE), "{kind:?}");
            active.set_up(kind, true);
            assert!(active.hides_strip(Owners::NONE), "{kind:?}");
            active.set_up(kind, false);
            assert!(!active.hides_strip(Owners::NONE), "{kind:?}");
        }
    }

    fn the_strip_stays_hidden_until_the_last_wizard_leaves() {
        let active = Active::new();
        active.set_up(Kind::DrillStart, true);
        active.set_up(Kind::EggZone, true);
        active.set_up(Kind::DrillStart, false);
        assert!(active.is_up(Kind::EggZone));
        assert!(active.hides_strip(Owners::NONE));
        active.set_up(Kind::EggZone, false);
        assert!(!active.hides_strip(Owners::NONE));
    }

    fn menus_and_wizards_hide_the_strip_independently() {
        for bits in 0..64 {
            let owners = Owners::from_bits(bits);
            for kind in Kind::ALL {
                let active = Active::new();
                assert_eq!(active.hides_strip(owners), owners.covers_hud(), "{bits}/{kind:?}");
                let before = owners;
                let covered = owners.covers_hud();
                active.set_up(kind, true);
                assert_eq!(owners, before);
                assert_eq!(owners.covers_hud(), covered);
                assert!(active.hides_strip(owners), "{bits}/{kind:?}");
                active.set_up(kind, false);
                assert_eq!(active.hides_strip(owners), covered, "{bits}/{kind:?}");
            }
        }
    }

    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("set_up_reports_whether_the_wizard_was_up", set_up_reports_whether_the_wizard_was_up as fn()),
            ("a_wizard_hides_and_restores_both_hud_renderers", a_wizard_hides_and_restores_both_hud_renderers as fn()),
            ("the_strip_stays_hidden_until_the_last_wizard_leaves", the_strip_stays_hidden_until_the_last_wizard_leaves as fn()),
            ("menus_and_wizards_hide_the_strip_independently", menus_and_wizards_hide_the_strip_independently as fn()),
        ]);
    }
    }
}
