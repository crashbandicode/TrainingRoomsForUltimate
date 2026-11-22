use super::{cpu::konst, mashout::FloorRay};
use crate::model::cpu::ids::C;
use crate::runtime::{game::natives, state_natives as state};

/// The floor point a fighter stands on for a crosshair at `at`. With `platforms`, the first floor at or below the
/// crosshair's own height (one unit of slack above it), so a crosshair moved down past a platform finds the floor
/// under that platform; without, the main floor only, cast from the camera's top. `None`: no floor there, or the
/// floor reads are not granted.
pub(super) unsafe fn floor_under(body: usize, [x, y]: [f32; 2], platforms: bool) -> Option<[f32; 2]> {
    let [_, _, top, bottom] = natives::camera_limit_range()?;
    let ray = FloorRay::new(body)?;
    let top = if platforms { (y + 1.0).min(top) } else { top };
    let floor = ray.floor(x, [top, bottom], platforms)?;
    Some([x, floor.height(x)?])
}

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
