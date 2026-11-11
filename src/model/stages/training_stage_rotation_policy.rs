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

