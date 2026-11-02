//! The fork's `common` module as the carried game-facing files name it. Each item forwards to the clean room's provider (the shared
//! natives of `runtime::game::natives`, decision D5; the input-owner mask of `runtime::published`).

use smash::app::BattleObjectModuleAccessor;
use smash::lib::lua_const::FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID;

use crate::model::lra_reset::{entry_id_valid, FIGHTER_MANAGER_SLOT_OFFSET};
use crate::runtime::compat::sync::Readable;
use crate::runtime::game::natives;
use crate::runtime::{game, scene, state_natives};

pub(crate) use crate::runtime::lobby::game_mode;

/// The game version the plugin is installed for (the version guard installs nothing on any other build): the tag macros and captures carry.
pub static SMASH_VERSION: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| "13.0.5".to_owned());

/// The fork's `common::consts` (`FighterId`).
pub mod consts {
    pub use crate::runtime::compat::consts::*;
}

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

/// The game's own `is_training_mode()` (native Training only, not the widened scene predicate). `unsafe` like the fork's: the carried
/// callers wrap it.
///
/// # Safety
/// Always sound: the call is guarded and answers false until its site passed.
pub unsafe fn is_native_training_mode() -> bool {
    scene::is_native_training()
}

/// The widened training predicate: native Training, or the waiting room while its practice is active and the plugin's own UI is not
/// busy (`scene::is_training`). `unsafe` like the fork's: the carried callers wrap it.
///
/// # Safety
/// Always sound: it reads the plugin's scene facts.
pub unsafe fn is_training_mode() -> bool {
    scene::is_training()
}

/// The game's ready-go state (the guarded `ui_is_ready_go` call).
pub fn is_ready_go() -> bool {
    game::is_ready_go()
}

/// Whether the accessor belongs to a fighter (not an article, an item or an effect owner).
///
/// # Safety
/// `module` is a live battle object's module accessor.
pub unsafe fn is_fighter(module: &BattleObjectModuleAccessor) -> bool {
    natives::is_fighter(module as *const BattleObjectModuleAccessor as usize)
}

/// `utility::get_category` of the accessor's battle object id.
///
/// # Safety
/// `module` is a live battle object's module accessor.
pub unsafe fn get_category(module: &BattleObjectModuleAccessor) -> i32 {
    state_natives::object_category(module.battle_object_id)
}

/// The fighter entry id of a fighter's accessor (`FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID`).
unsafe fn entry_of(module: &BattleObjectModuleAccessor) -> i32 {
    natives::get_int(module as *const BattleObjectModuleAccessor as usize, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID)
}

/// The game's operator flag of a fighter: `Some(true)` for an AI-operated CPU, `Some(false)` for a controller-operated fighter,
/// `None` when the entry is not one the game knows. `module` is a live fighter's module accessor (it comes from a reference).
pub fn operation_cpu(module: &mut BattleObjectModuleAccessor) -> Option<bool> {
    // SAFETY: the accessor is a live reference; both natives read through it or the entry id and keep nothing.
    unsafe {
        let entry = entry_of(module);
        entry_id_valid(entry).then(|| natives::is_operation_cpu(entry))
    }
}

/// `operation_cpu`, with an unknown entry reading as "not an AI CPU".
pub fn is_operation_cpu(module: &mut BattleObjectModuleAccessor) -> bool {
    operation_cpu(module) == Some(true)
}

/// The module accessor of the battle object with `object_id` (`sv_battle_object::module_accessor`), `None` for an unknown id.
pub fn try_get_object_accessor(object_id: u32) -> Option<*mut BattleObjectModuleAccessor> {
    // SAFETY: a guarded called function; a null answer is an unknown id.
    let accessor = unsafe { state_natives::object_accessor(object_id) };
    (accessor != 0).then_some(accessor as *mut BattleObjectModuleAccessor)
}

/// The fork's `common::menu`: what the plugin's own menu and the native UI own, read from the input-owner mask (SPEC-input-03: one
/// producer per fact, one reader).
pub mod menu {
    use std::sync::atomic::Ordering;

    use crate::model::input_delay::Owners;
    use crate::runtime::published;

    pub use crate::runtime::compat::menu::{mii_preferences, stage_preferences};
    pub use crate::runtime::ui::menu_app::{MENU_RECEIVED_INPUT, QUICK_MENU_APP};

    /// The Replay Window asks the plugin's menu to close by clearing this flag (the editor takes the screen); package 5's menu reads it.
    pub static QUICK_MENU_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    /// Polls the close of the editor keeps P1 neutral for (the plugin menu's own close window).
    pub const MENU_CLOSE_WAIT_FRAMES: u32 = 15;

    /// The stored menu is refreshed after the INI overlay: the one store is the settings file's rows, which `assign(&MENU, ..)`
    /// already wrote back (`compat::menu`), so there is no second copy to refresh.
    pub fn refresh_stored_menu_on_quick_app() {}

    /// Something other than gameplay owns Player 1's input (the plugin's menu, the replay editor, the native waiting-room UI,
    /// the game's pause menu). The stop chord's own bit is not an owner: it only marks a poll the chord consumed.
    pub fn captures_input() -> bool {
        Owners::from_bits(published::input_owners().bits() & !Owners::STOP_CHORD.bits()).any()
    }

    /// The plugin's menu is open (or closed within its 15-frame window).
    pub fn quick_menu_active() -> bool {
        published::input_owners().contains(Owners::MENU)
    }

    /// The menu's session ends (the menu closes without a commit, its input and chord state are forgotten).
    pub fn clear_session_state() {
        crate::runtime::ui::clear_session_state();
    }

    /// The one producer of the waiting-room native UI busy fact: `Owners::NATIVE_UI_BUSY` in the input-owner mask (review R2: the
    /// input-delay stage reads the mask, never a hand-supplied value).
    pub fn set_native_ui_owner(busy: bool) {
        if busy {
            published::INPUT_OWNERS.fetch_or(Owners::NATIVE_UI_BUSY.bits(), Ordering::AcqRel);
        } else {
            published::INPUT_OWNERS.fetch_and(!Owners::NATIVE_UI_BUSY.bits(), Ordering::AcqRel);
        }
    }
}

/// The fork's `common::button_config`: the chord state of the session.
pub mod button_config {
    use crate::model::chords::Request;
    use crate::runtime::features::chords;

    /// Some shortcut request (save, load, record, playback, menu, a cycle) is pending.
    pub fn has_pending_request() -> bool {
        chords::any_pending()
    }

    /// The Player 1 cycle chord passed since the last call (clears it).
    pub fn consume_player_cycle_request() -> bool {
        chords::consume(Request::PlayerCycle)
    }

    /// The CPU cycle chord passed since the last call (clears it).
    pub fn consume_cpu_cycle_request() -> bool {
        chords::consume(Request::CpuCycle)
    }

    /// The chords forget what they were tracking (a native dialog took the controller).
    pub fn clear_session_state() {
        crate::runtime::features::chords::session_clear();
    }
}

/// Whether the body is on the ground (`StatusModule::situation_kind`); false until the CPU runtime resolved the game's constants.
///
/// # Safety
/// `module` is a live battle object's module accessor.
pub unsafe fn is_grounded(module: &BattleObjectModuleAccessor) -> bool {
    natives::is_grounded(module as *const BattleObjectModuleAccessor as usize)
}

/// The module accessor of a practice fighter's first body: the entry the lobby maps the role to (`training::lobby::entry_id`: the
/// waiting room's CPU is entry 4, offline Training's is entry 1), then the shared native.
pub fn try_get_module_accessor(id: crate::runtime::compat::consts::FighterId) -> Option<*mut BattleObjectModuleAccessor> {
    let entry = crate::runtime::lobby::training::lobby::entry_id(id);
    // SAFETY: `natives` answers `None` until the entry rows passed their guards and reads the live game on every call.
    unsafe { natives::module_accessor(entry) }.map(|accessor| accessor as *mut BattleObjectModuleAccessor)
}
/// The `BattleObjectManager` singleton's slot (`lib::Singleton<app::BattleObjectManager>::instance_`), main + this.
const BATTLE_OBJECT_MANAGER_SLOT_OFFSET: usize = 0x52b7a00;
/// `BattleObjectManager::is_active_find_battle_object(manager, object id)` (the `lua_bind` thunk, row `roster_find_battle_object`).
const FIND_BATTLE_OBJECT_OFFSET: usize = 0x1fca0b0;

/// The active battle object with `object_id`, `None` for an unknown or inactive id. The roster cycle starts only when the row
/// `roster_find_battle_object` passed its guard (`roster::CALLED_ROWS`), and this runs only from the cycle.
pub fn try_get_battle_object(object_id: u32) -> Option<&'static smash::app::BattleObject> {
    let base = game::text_base();
    // SAFETY: the slot is a pointer-sized `.bss` word of the main image; the function is the guarded thunk and a null answer is
    // an unknown id.
    unsafe {
        let manager = ((base + BATTLE_OBJECT_MANAGER_SLOT_OFFSET) as *const usize).read_volatile();
        if manager == 0 {
            return None;
        }
        let find: extern "C" fn(usize, u32) -> *const smash::app::BattleObject = std::mem::transmute(base + FIND_BATTLE_OBJECT_OFFSET);
        find(manager, object_id).as_ref()
    }
}

/// The mapped input record the carried files read and write.
pub mod input {
    pub use crate::runtime::compat::input::{Buttons, MappedInputs};
}