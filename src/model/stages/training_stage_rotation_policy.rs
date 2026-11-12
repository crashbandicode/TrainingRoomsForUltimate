//! Ordinary Training entry rotation: pure policy over the shared stage cycle.
//! No native reads or writes. The adapter reports observed events; this module
//! decides when one genuine Training entry consumes one rotation step.

use crate::engine::stages::lobby_stages::{native_stage, LobbyStageOrder, Selection};
use crate::engine::stages::stage_menu::Preferences;

use crate::engine::stages::lobby_stage_cycle::StageCycle;

/// The ordinary Training mode word. Melee 0x20 is only required at the loader:
/// at the character-select transition the melee byte still holds the previous
/// scene's value (retained diagnostic, TRAINING-ROTATION185 §2).
pub const TRAINING_MODE: u32 = 0x04040000;
pub const TRAINING_MELEE: u8 = 0x20;

/// Exactly the generic loader's Training call: the return inside the loader,
/// resource slot 0 and the index-0 practice descriptor the native setter fills.
pub fn qualified_loader(
    initialized: bool, mode: u32, melee: u8, caller: usize, slot: u32, descriptor: usize, base: usize,
) -> bool {
    initialized && mode == TRAINING_MODE && melee == TRAINING_MELEE && caller == 0x14f57c4
        && slot == 0 && base != 0 && descriptor == base.wrapping_add(0x52c55d0)
}

/// What the loader descriptor should carry. `selection` is `None` after a direct
/// change completed natively; that stage is reapplied but never moves the cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Applied {
    pub selection: Option<Selection>,
    pub native: u32,
    pub hash: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    Prepared(Selection, u32),
    Suppressed,
    Disabled,
    Refused(&'static str),
}

pub struct TrainingRotation {
    /// Training state-context token of the live sequence; compared, never dereferenced.
    owner: Option<usize>,
    /// Owner whose successful product reenter must not count as a new entry.
    suppress: Option<usize>,
    cycle: StageCycle<Selection>,
    /// Last confirmed Training stage: this entry's resolution or a completed direct change.
    current: Option<Applied>,
}

impl Default for TrainingRotation {
    fn default() -> Self { Self::new() }
}

impl TrainingRotation {
    pub const fn new() -> Self {
        Self { owner: None, suppress: None, cycle: StageCycle::new(), current: None }
    }

    /// A Training sequence context was constructed. Every owner-bound value
    /// belongs to a dead sequence, even when the address is reused. The
    /// rotation cursor continues across Training visits.
    pub fn sequence_initialized(&mut self) {
        self.owner = None;
        self.suppress = None;
        self.cycle.cancel_preload();
        self.current = None;
    }

    /// Age-zero state-5 call of a structurally verified Training sequence.
    pub fn entry(
        &mut self, owner: usize, enabled: bool, order: LobbyStageOrder, prefs: &Preferences,
    ) -> Entry {
        if self.owner != Some(owner) {
            self.sequence_initialized();
            self.owner = Some(owner);
        }
        // One-shot: the next age-zero call of this owner consumes it, whatever the setting.
        let suppressed = self.suppress.take().is_some();
        if !enabled {
            self.cycle.cancel_preload();
            self.current = None;
            return Entry::Disabled;
        }
        if suppressed {
            return Entry::Suppressed;
        }
        // A genuinely new entry never inherits a reservation an earlier entry
        // prepared and abandoned; it resolves the current preferences.
        self.cycle.cancel_preload();
        self.current = None;
        match self.cycle.prepare_preferences(true, order, prefs, false) {
            Ok(Some(selection)) => match native_stage(selection) {
                Ok(native) => Entry::Prepared(selection, native),
                Err(why) => {
                    self.cycle.cancel_preload();
                    Entry::Refused(why)
                }
            },
            Ok(None) => Entry::Refused("no Training rotation stages selected"),
            Err(why) => Entry::Refused(why),
        }
    }

    /// The product reenter succeeded for `owner`; its next age-zero call is a return.
    pub fn reenter_requested(&mut self, owner: usize) {
        if self.owner == Some(owner) {
            self.suppress = Some(owner);
        }
    }

    /// Qualified Training loader call. `hash_of` reads the catalog row of a
    /// native id. A pending reservation is consumed once; later calls reapply.
    pub fn apply(
        &mut self, enabled: bool, hash_of: impl FnOnce(u32) -> Option<u64>,
    ) -> Result<Option<Applied>, &'static str> {
        if !enabled {
            self.cycle.cancel_preload();
            self.current = None;
            return Ok(None);
        }
        if self.owner.is_none() {
            return Ok(None);
        }
        let Some(selection) = self.cycle.pending() else { return Ok(self.current); };
        let native = native_stage(selection)?;
        let hash = hash_of(native).ok_or("stage catalog row unavailable")?;
        let _ = self.cycle.enter_resolved()?;
        let applied = Applied { selection: Some(selection), native, hash };
        self.current = Some(applied);
        Ok(Some(applied))
    }

    /// A direct practice change completed natively in this Training sequence.
    pub fn direct_completed(&mut self, native: u32, hash: u64) {
        if self.owner.is_some() {
            self.current = Some(Applied { selection: None, native, hash });
        }
    }

    /// A published direct change departed before completion: the live stage is unknown.
    pub fn direct_uncertain(&mut self) {
        self.current = None;
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    
    

    const BF: Selection = Selection::normal(0);
    const FD: Selection = Selection::normal(3);
    const OWNER: usize = 0x2a00_1000;

    /// Independently recorded 40-bit package hashes (catalog rows and retained logs).
    fn hash_of(native: u32) -> Option<u64> {
        match native {
            0 | 1 => Some(0x18dcbe2bc4),
            3 => Some(0x100f8e9a50),
            96 | 97 => Some(0x1b9fc8fb46),
            308 => Some(0x151e3f8e1e),
            311 => Some(0x165a48f367),
            _ => None,
        }
    }

    fn order() -> LobbyStageOrder { serde_json::from_str("[0,0,1,2]").unwrap() }

    fn applied(selection: Selection, native: u32) -> Applied {
        Applied { selection: Some(selection), native, hash: hash_of(native).unwrap() }
    }

    fn disabled_setting_prepares_nothing_writes_nothing_and_keeps_the_cursor() {
        let prefs = Preferences::default();
        let mut rotation = TrainingRotation::new();
        assert_eq!(rotation.entry(OWNER, false, order(), &prefs), Entry::Disabled);
        assert_eq!(rotation.apply(false, hash_of), Ok(None));
        assert_eq!(rotation.apply(true, hash_of), Ok(None)); // No reservation, no snapshot.
        // Enabling later starts at the first stage: nothing was consumed while Off.
        assert_eq!(rotation.entry(OWNER, true, order(), &prefs), Entry::Prepared(BF, 0));
    }

    fn direct_completion_replaces_the_reapplied_stage_and_uncertainty_clears_it() {
        let prefs = Preferences::default();
        let mut rotation = TrainingRotation::new();
        assert_eq!(rotation.entry(OWNER, true, order(), &prefs), Entry::Prepared(BF, 0));
        assert_eq!(rotation.apply(true, hash_of), Ok(Some(applied(BF, 0))));
        // Live direct change to Smashville Omega completed natively.
        rotation.direct_completed(96, hash_of(96).unwrap());
        rotation.reenter_requested(OWNER);
        assert_eq!(rotation.entry(OWNER, true, order(), &prefs), Entry::Suppressed);
        assert_eq!(rotation.apply(true, hash_of),
            Ok(Some(Applied { selection: None, native: 96, hash: 0x1b9fc8fb46 })));
        // A published direct change that departed unfinished leaves the stage unknown.
        rotation.direct_uncertain();
        rotation.reenter_requested(OWNER);
        assert_eq!(rotation.entry(OWNER, true, order(), &prefs), Entry::Suppressed);
        assert_eq!(rotation.apply(true, hash_of), Ok(None));
        // Neither direct event moved the rotation cursor.
        assert_eq!(rotation.entry(OWNER, true, order(), &prefs), Entry::Prepared(FD, 3));
        // Direct completion before any entry of a sequence is not adopted.
        let mut fresh = TrainingRotation::new();
        fresh.direct_completed(96, 0x1b9fc8fb46);
        assert_eq!(fresh.apply(true, hash_of), Ok(None));
    }

    fn qualified_loader_rejects_other_callers_slots_descriptors_and_scenes() {
        let base = 0x7100_0000_0000usize;
        let descriptor = base + 0x52c55d0;
        assert!(qualified_loader(true, 0x04040000, 0x20, 0x14f57c4, 0, descriptor, base));
        for caller in [0x26074f4, 0x2335cf0, 0x23684b0, 0x2369208, 0x236ca48, 0x237cce0, 0x14f57c0, 0] {
            assert!(!qualified_loader(true, 0x04040000, 0x20, caller, 0, descriptor, base));
        }
        assert!(!qualified_loader(true, 0x04040000, 0x20, 0x14f57c4, 2, descriptor, base));
        assert!(!qualified_loader(true, 0x04040000, 0x20, 0x14f57c4, 0, descriptor + 0xb0, base));
        assert!(!qualified_loader(true, 0x04040000, 0x20, 0x14f57c4, 0, descriptor, 0));
        for (mode, melee) in [(0x04040000, 0x1c), (0x04040000, 0x00), (0x04050000, 0x1c),
            (0x02010000, 0x0d), (0x07010100, 0x0d), (0x01000000, 0x10)] {
            assert!(!qualified_loader(true, mode, melee, 0x14f57c4, 0, descriptor, base));
        }
        assert!(!qualified_loader(false, 0x04040000, 0x20, 0x14f57c4, 0, descriptor, base));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("disabled_setting_prepares_nothing_writes_nothing_and_keeps_the_cursor", disabled_setting_prepares_nothing_writes_nothing_and_keeps_the_cursor as fn()),
            ("direct_completion_replaces_the_reapplied_stage_and_uncertainty_clears_it", direct_completion_replaces_the_reapplied_stage_and_uncertainty_clears_it as fn()),
            ("qualified_loader_rejects_other_callers_slots_descriptors_and_scenes", qualified_loader_rejects_other_callers_slots_descriptors_and_scenes as fn()),
        ]);
    }
    }
}
