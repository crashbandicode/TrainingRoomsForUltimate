use super::cpu::konst;
use crate::model::cpu::ids::C;
use crate::runtime::state_natives as state;

/// Stand on a floor point; the caller has checked the constants and grants.
pub(super) unsafe fn stand(body: usize, [x, y]: [f32; 2], facing: f32) {
    state::clear_speed_all(body);
    state::init_pos(body, &state::Vector3f { x, y, z: 0.0 });
    state::set_lr(body, facing);
    state::set_situation_kind(body, konst(C::SituationGround));
    state::status_request(body, konst(C::StatusWait));
}

/// Place in the air; the caller has checked the constants and grants and requests the status.
pub(super) unsafe fn lift(body: usize, [x, y]: [f32; 2], facing: f32) {
    state::clear_speed_all(body);
    state::set_pos(body, &state::Vector3f { x, y, z: 0.0 });
    state::set_lr(body, facing);
    state::set_situation_kind(body, konst(C::SituationAir));
}

/// Request Fall after the caller has checked the constants and grants.
pub(super) unsafe fn fall(body: usize) {
    state::status_request(body, konst(C::StatusFall));
}

/// Like Tech practice, reapply the hold because MotionModule sleep leaves physics running.
/// The caller has checked the constants and grants.
pub(super) unsafe fn hold(body: usize, [x, y]: [f32; 2]) {
    state::set_sleep(body, true);
    state::clear_speed_all(body);
    state::set_pos(body, &state::Vector3f { x, y, z: 0.0 });
}
