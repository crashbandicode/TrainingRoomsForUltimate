//! The dependency boundary of the named-macro layer: what the carried files call that another stream owns and that has not merged.
//! Each item says what it waits for; `install_report` is what the install line prints about it. A boundary, not a workaround: it
//! closes when the owner merges and the provider replaces the body (docs/implementation-packages.md package 6A and 5).

/// The fork's `training_mod_sync` helpers the carried layer uses (the lock crate is not kept, decision B9).
pub(crate) mod sync {
    use std::sync::atomic::{AtomicBool, Ordering};

    pub(crate) use crate::runtime::compat::sync::read;
    pub(crate) use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

    pub(crate) fn lock_read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
        lock.read()
    }

    pub(crate) fn lock_write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
        lock.write()
    }

    pub(crate) fn assign(flag: &AtomicBool, value: bool) {
        flag.store(value, Ordering::Release);
    }
}

/// The mapped input record and its button set as the carried layer names them.
pub(crate) mod input {
    pub(crate) use crate::model::input_delay::MappedInputs;
}

/// Worker A's roster cycle (`character_roster` and its Mii children): not merged, so every call answers "unavailable". The named
/// layer then plays on the settled CPU only (no fighter or exact-Mii request) and says so in the install line.
pub(crate) mod roster {
    pub(crate) fn busy() -> bool {
        false
    }

    pub(crate) fn waiting_drafts_ready() -> bool {
        true
    }

    pub(crate) fn cpu_reload_ready() -> bool {
        false
    }

    pub(crate) fn request_cpu_kind(_kind: u32) -> bool {
        false
    }

    pub(crate) fn current_mii_configuration(_role: usize) -> Option<crate::engine::mii::lifetime::Configuration> {
        None
    }

    pub(crate) mod mii_replay {
        use crate::engine::mii::replay::ReplayMiis;

        /// A committed replay capture's exact Mii metadata (the cycle remembers it for exact reloads).
        pub(crate) fn committed(_miis: ReplayMiis) {}
    }
}

/// The Mii editor bridge (package 6B, carried and not declared until the roster cycle lands).
pub(crate) mod editor_bridge {
    /// A saved-Mii catalog was read (the bridge keeps its last view for the editor).
    pub(crate) fn catalog_read(_catalog: &crate::engine::mii::inventory::Snapshot) {}
}

/// `Macro Source`: where the CPU's recordings come from (the settings row `macro_source`, option order Numbered Slots, Named Library).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MacroSource(pub(crate) u8);

impl MacroSource {
    pub(crate) const NUMBERED: MacroSource = MacroSource(0);
    pub(crate) const NAMED: MacroSource = MacroSource(1);
}

/// What the install line says about this layer's dependency boundary.
pub(crate) fn install_report() -> &'static str {
    "roster=unavailable_until_cycle_lands menu_app=unavailable_until_pkg5_screens"
}
