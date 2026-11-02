//! The fork's `common` module as the carried game-facing files name it. Each item forwards to the clean room's provider.

use crate::model::lra_reset::FIGHTER_MANAGER_SLOT_OFFSET;
use crate::runtime::compat::sync::Readable;
use crate::runtime::{game, scene};

/// The `FighterManager` singleton's slot address (`main + FIGHTER_MANAGER_SLOT_OFFSET`); `read(&FIGHTER_MANAGER_ADDR)` is the
/// address, the manager pointer is the pointer stored there.
pub struct FighterManagerAddr;
pub static FIGHTER_MANAGER_ADDR: FighterManagerAddr = FighterManagerAddr;

impl Readable for FighterManagerAddr {
    type Value = usize;
    fn read_value(&self) -> usize {
        game::text_base() + FIGHTER_MANAGER_SLOT_OFFSET
    }
}

/// The game's own `is_training_mode()` (native Training only, not the widened scene predicate).
pub fn is_native_training_mode() -> bool {
    scene::is_native_training()
}

/// The game's ready-go state (the guarded `ui_is_ready_go` call).
pub fn is_ready_go() -> bool {
    game::is_ready_go()
}

/// The fork's `common::menu`: what the plugin's own menu and the native UI own, read from the input-owner mask (SPEC-input-03: one
/// producer per fact, one reader).
pub mod menu {
    use crate::model::input_delay::Owners;
    use crate::runtime::published;

    pub use crate::runtime::compat::menu::{mii_preferences, stage_preferences};

    /// Something other than gameplay owns Player 1's input (the plugin's menu, the replay editor, the native waiting-room UI,
    /// the game's pause menu). The stop chord's own bit is not an owner: it only marks a poll the chord consumed.
    pub fn captures_input() -> bool {
        Owners::from_bits(published::input_owners().bits() & !Owners::STOP_CHORD.bits()).any()
    }

    /// The plugin's menu is open (or closed within its 15-frame window).
    pub fn quick_menu_active() -> bool {
        published::input_owners().contains(Owners::MENU)
    }
}
