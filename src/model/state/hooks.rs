//! The predicates the save-state hooks answer from (SPEC-state-01 §4.1): which parameters a death
//! overrides, when sounds, quakes and effects are suppressed. Pure; the hooks read two flags the
//! runtime publishes (`loading`, `killing`) and the settings.

use super::settings::StateSettings;

/// The `common` parameters of a death and the one `param_mball` parameter a load overrides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeathParam {
    DeadRebirthWaitFrame,
    RebirthMoveFrame,
    RebirthWaitFrame,
    RebirthInvincibleFrame,
    RebirthInvincibleAddFrame,
    RebirthMoveFrameTrainer,
    /// `param_mball` `change_fly_frame`.
    ChangeFlyFrame,
}

/// Whether a death is instant: every death in practice is, unless Respawn on platform is On (N2),
/// in which case only a load's deaths are. With Save States and Auto-load on death both On a death loads a
/// state, so it is instant whatever the toggle says (the row is greyed then). With Save States Off nothing
/// loads, and the toggle decides (operator 2026-11-13, TR5).
pub fn instant_respawn(settings: &StateSettings, loading: bool) -> bool {
    loading || (settings.enable && settings.autoload) || !settings.respawn_invincibility
}

/// The value a parameter read is answered with, or `None` for the game's own. `training` is the widened
/// practice predicate (`runtime::scene::is_training`: native Training or an active waiting-room practice;
/// the hook is inert elsewhere). Save States Off leaves the load's parameters alone, not the respawn row's.
pub fn param_override(param: DeathParam, settings: &StateSettings, loading: bool, training: bool) -> Option<i32> {
    if !training {
        return None;
    }
    match param {
        DeathParam::DeadRebirthWaitFrame if instant_respawn(settings, loading) => Some(1),
        DeathParam::RebirthMoveFrame
        | DeathParam::RebirthWaitFrame
        | DeathParam::RebirthInvincibleFrame
        | DeathParam::RebirthInvincibleAddFrame
            if instant_respawn(settings, loading) =>
        {
            Some(0)
        }
        DeathParam::RebirthMoveFrameTrainer | DeathParam::ChangeFlyFrame if loading => Some(0),
        _ => None,
    }
}

/// A requested camera quake is replaced by none while killing.
pub fn quake_suppressed(killing: bool) -> bool {
    killing
}

/// A sound hash is replaced by `se_silent` while killing (death explosion, misfoot).
pub fn sound_silenced(killing: bool) -> bool {
    killing
}

/// An effect's size is forced to 0 while a load is in progress and the fighter is not being buffed
/// (throw explosions, the Ice Climbers' squall).
pub fn effect_hidden(loading: bool, fighter_buffing: bool) -> bool {
    loading && !fighter_buffing
}

/// `sys_score_aura` follow requests are made size 0 while a load is in progress.
pub fn score_aura_hidden(loading: bool) -> bool {
    loading
}
