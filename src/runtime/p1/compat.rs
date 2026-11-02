//! The names the carried `cpu_pilot.rs` takes from the fork's mixed files (`common`, `training`, `replay_preview`,
//! `character_roster`, `training_mod_sync`), answered by the merged runtime. Written for this repository, not carried; the tool
//! rewrites the carried file's paths to this module (`docs/provenance/pkg6-carry-manifest.json`, `modules.root`).
//!
//! Thin on purpose: every answer is one call into another package's public API, so a re-merge of those packages touches only
//! this file. Natives are reached only through `runtime::game::natives` (its capability gates answer "nothing" until their rows
//! passed their guards).

use std::sync::atomic::{AtomicBool, AtomicUsize};

use smash::app::BattleObjectModuleAccessor;

use crate::model::input_delay::Owners;
use crate::runtime::features::recorder;
use crate::runtime::game::{self, natives};
use crate::runtime::{published, scene};

/// `training_mod_sync::read`: one atomic or slot read by reference (the fork's helper crate is not kept, decision B9).
pub mod sync {
    use super::*;

    pub trait Load {
        type Value;
        fn load_now(&self) -> Self::Value;
    }

    impl Load for AtomicBool {
        type Value = bool;
        fn load_now(&self) -> bool {
            self.load(std::sync::atomic::Ordering::Acquire)
        }
    }

    impl Load for AtomicUsize {
        type Value = usize;
        fn load_now(&self) -> usize {
            self.load(std::sync::atomic::Ordering::Acquire)
        }
    }

    pub fn read<T: Load>(source: &T) -> T::Value {
        source.load_now()
    }
}

pub mod common {
    use super::*;

    /// The fork's `FighterId`: P1 is entry 0, the CPU entry 1 (offline Training).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum FighterId {
        Player,
        CPU,
    }

    impl FighterId {
        const fn entry(self) -> i32 {
            match self {
                FighterId::Player => 0,
                FighterId::CPU => 1,
            }
        }
    }

    /// The module accessor of a fighter's first body, or `None` (the natives' `Entry` gate, a missing entry).
    pub fn try_get_module_accessor(id: FighterId) -> Option<*mut BattleObjectModuleAccessor> {
        // SAFETY: `natives` answers `None` until the entry rows passed their guards and reads the live game each call.
        unsafe { natives::module_accessor(id.entry()) }.map(|accessor| accessor as *mut BattleObjectModuleAccessor)
    }

    /// The game's own Training predicate (not the widened one: the pilot's offline scope is native Training).
    pub fn is_native_training_mode() -> bool {
        scene::is_native_training()
    }

    pub fn is_fighter(module: &BattleObjectModuleAccessor) -> bool {
        // SAFETY: `module` is a live accessor the caller holds; the `Category` gate answers false until its rows passed.
        unsafe { natives::is_fighter(module as *const BattleObjectModuleAccessor as usize) }
    }

    pub fn is_grounded(module: &BattleObjectModuleAccessor) -> bool {
        // SAFETY: as `is_fighter`; false until the CPU runtime resolved the constants.
        unsafe { natives::is_grounded(module as *const BattleObjectModuleAccessor as usize) }
    }

    /// The `FighterManager` singleton, read from its game global on every access (`sync::read(&FIGHTER_MANAGER_ADDR)`).
    pub struct ManagerSlot;

    impl super::sync::Load for ManagerSlot {
        type Value = usize;
        fn load_now(&self) -> usize {
            // SAFETY: the slot is a pointer-sized global of `main`, the same one the reset hub reads.
            unsafe { ((game::text_base() + crate::model::lra_reset::FIGHTER_MANAGER_SLOT_OFFSET) as *const usize).read_volatile() }
        }
    }

    pub static FIGHTER_MANAGER_ADDR: ManagerSlot = ManagerSlot;

    pub mod input {
        pub use crate::model::input_delay::MappedInputs;
    }

    pub mod menu {
        use super::*;

        /// The plugin menu (or a modal panel) owns the input.
        pub fn captures_input() -> bool {
            published::input_owners().contains(Owners::MENU)
        }
    }

    /// The fork's `game_mode` (the online/lobby scene probe) belongs to the lobby UI work (worker B); until it lands the pilot
    /// sees no mode, so only native Training scopes it (the online 1v1 scope is package 7's).
    pub mod game_mode {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct Mode {
            pub initialized: bool,
            pub melee: u8,
            pub mode: u32,
        }

        pub fn current() -> Option<Mode> {
            None
        }
    }
}

pub mod training {
    use super::*;

    pub fn recording_or_playback() -> bool {
        recorder::recording_or_playback()
    }

    pub mod ui {
        pub mod menu {
            use super::super::*;

            /// The native (vanilla) pause menu is open.
            pub struct VanillaPause;

            impl sync::Load for VanillaPause {
                type Value = bool;
                fn load_now(&self) -> bool {
                    published::input_owners().contains(Owners::VANILLA_PAUSE)
                }
            }

            pub static VANILLA_MENU_ACTIVE: VanillaPause = VanillaPause;
        }
    }
}

pub mod replay_preview {
    /// The replay editor holds the screen.
    pub fn occupied() -> bool {
        super::recorder::replay_editor_occupied()
    }
}

pub mod character_roster {
    /// A hot swap is replacing a fighter (the roster engine's busy flag, `published::HOT_SWAP_IN_PROGRESS`).
    pub fn busy() -> bool {
        super::published::HOT_SWAP_IN_PROGRESS.load(std::sync::atomic::Ordering::Acquire)
    }
}