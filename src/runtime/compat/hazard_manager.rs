//! The fork's `hazard_manager::waiting_hazards`: the per-request hazard byte of a stage request (SPEC-state-07 §6.3). The
//! carried files call it for a waiting-room stage entry (`lobby_stage`) and for a Change Stage (`practice_stage_request`).
//! In a waiting room the rule is the carried `waiting_hazards_policy`; its inputs are the scene, the Lobby Practice setting
//! (On: the request comes before the practice UI is ready, so the setting and not `lobby::active()` is what qualifies it)
//! and the Stage Hazards setting. rc.12: a Change Stage in Training takes the Stage Hazards setting too
//! (`stage_hazards::change_stage_byte`); the waiting-room entry never asks in Training.

use crate::engine::compat::menu::OnOff;
use crate::engine::stages::waiting_hazards_policy;
use crate::runtime::compat::menu::MENU;
use crate::runtime::compat::sync::read;
use crate::runtime::lobby::game_mode::Mode;
use crate::runtime::published;

/// `Some(hazards byte)` for a Training Change Stage, or a waiting-room stage request while Lobby Practice is On; `None`
/// otherwise (the native request stands).
pub fn waiting_hazards(mode: Mode) -> Option<u8> {
    let practice = read(&MENU).lobby_practice == OnOff::ON;
    let hazards = published::stage_hazards();
    let waiting = waiting_hazards_policy::waiting_hazards(mode.initialized, mode.mode, mode.melee, practice, hazards);
    crate::model::stage_hazards::change_stage_byte(mode.initialized, mode.mode, mode.melee, waiting, hazards)
}
