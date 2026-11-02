//! Package 6, lobby side (worker B): the waiting-room practice engine's game-facing files (carried by `tools/carry_pkg6.py`), the
//! scene observation they share (`game_mode`) and the stage rotation. Switch only.
//!
//! The functions at the bottom are the call sites the fork's mixed code made (decision D5): `init` once from boot after the roster
//! cycle, `update` once per game frame, `observe` from the fighter init path and `frame` from the per-fighter frame callback.

pub(crate) mod automation_helper;
pub(crate) mod game_mode;
pub(crate) mod lobby_stage_probe;
#[cfg(feature = "practice-stage")]
pub(crate) mod practice_stage_observer;
pub(crate) mod practice_stage_request;
pub(crate) mod training;
pub(crate) mod training_stage_rotation;

use smash::app::BattleObjectModuleAccessor;

use crate::model::lra_reset::{entry_id_valid, FIGHTER_ENTRY_ID_FIELD};
use crate::runtime::game;

/// Start the lobby's files, in the order the fork's `main` did: the practice engine (it reads the game mode fingerprint), the stage
/// probe (it starts the rotation), the practice stage request and, in a diag build, its observer. Called once from boot, after the
/// roster cycle's own `init` and before the install line.
pub(crate) fn init() {
    training::lobby::init();
    lobby_stage_probe::init();
    practice_stage_request::init(game::text_base());
    #[cfg(feature = "practice-stage")]
    practice_stage_observer::init();
}

/// Once per game frame, before the input processing (the input poll's note for Player 1).
pub(crate) fn update() {
    training::lobby::update();
}

/// A fighter's on-init has run (the reset hub's wrapper): the practice engine records the entry.
///
/// # Safety
/// `fighter` is the fighter object the game just initialised.
pub(crate) unsafe fn observe(fighter: usize) {
    let accessor = game::natives::fighter_accessor(fighter);
    if accessor != 0 {
        training::lobby::observe_fighter(&mut *(accessor as *mut BattleObjectModuleAccessor));
    }
}

/// A fighter's frame has run: Player 1's body refreshes the practice entry lease (once per game frame).
///
/// # Safety
/// `fighter` is the fighter object the game just ran a frame of.
pub(crate) unsafe fn frame(fighter: usize) {
    let entry = ((fighter + FIGHTER_ENTRY_ID_FIELD) as *const i32).read();
    if entry_id_valid(entry) && entry == 0 {
        training::practice::refresh();
        // The roster cycle's once-per-game-frame steps run on Player 1's body (each returns at once when nothing is pending): the
        // source leases, the Kirby hat requests, the item engine (a granted or restored item) and the preload planner.
        let accessor = game::natives::fighter_accessor(fighter);
        if accessor != 0 {
            let body = accessor as *mut BattleObjectModuleAccessor;
            crate::runtime::roster::character_roster::sources::on_frame(body);
            crate::runtime::roster::kirby_hats::on_frame(body);
            crate::runtime::roster::character_items::on_frame(body);
            crate::runtime::roster::cycle_preload::on_frame(&mut *body);
        }
    }
    // Diag build: the character-item table's constants as the game resolves them (once).
    #[cfg(feature = "diag")]
    training::character_specific::items::dump_once();
}
