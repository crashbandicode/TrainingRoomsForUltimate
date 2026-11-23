use super::{cpu::konst, mashout::FloorRay};
use crate::model::cpu::ids::{C, UNRESOLVED};
use crate::model::crosshair::{self, CpuPreview, Origin, PreviewAct};
use crate::runtime::{game::{self, natives}, state_natives as state};

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

pub(super) unsafe fn ground(body: usize, [x, y]: [f32; 2], facing: f32) {
    state::clear_speed_all(body);
    state::init_pos(body, &state::Vector3f { x, y, z: 0.0 });
    state::set_lr(body, facing);
    state::set_situation_kind(body, konst(C::SituationGround));
}

/// Stand on a floor point; the caller has checked the constants and grants.
pub(super) unsafe fn stand(body: usize, at: [f32; 2], facing: f32) {
    ground(body, at, facing);
    state::status_request(body, konst(C::StatusWait));
}

/// Place in the air; the caller has checked the constants and grants and requests the status. `init_pos`, as
/// `ground`: a fighter standing on a platform keeps its ground contact through `set_pos` and is pulled back onto it.
pub(super) unsafe fn lift(body: usize, [x, y]: [f32; 2], facing: f32) {
    state::clear_speed_all(body);
    state::init_pos(body, &state::Vector3f { x, y, z: 0.0 });
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

/// The constants and granted natives a picker's CPU preview reads and writes: the placement set and the body's own
/// position, situation and facing.
fn preview_ready() -> bool {
    use crate::model::{sites_pkg2 as s2, sites_pkg3 as s, sites_pkg4 as s4};
    ![C::StatusWait, C::StatusFall, C::SituationGround, C::SituationAir].map(konst).contains(&UNRESOLVED)
        && [
            s::STATE_CLEAR_SPEED_ALL_OFFSET,
            s::STATE_INIT_POS_OFFSET,
            s::STATE_SET_POS_OFFSET,
            s::STATE_SET_LR_OFFSET,
            s::STATE_SET_SITUATION_KIND_OFFSET,
            s::STATE_STATUS_REQUEST_OFFSET,
            s::STATE_SET_SLEEP_OFFSET,
            s2::POS_X_OFFSET,
            s2::POS_Y_OFFSET,
            s2::SITUATION_KIND_OFFSET,
            s4::POSTURE_LR_OFFSET,
        ]
        .into_iter()
        .all(game::granted)
}

/// One frame of a picker's hold on the CPU's body at its picked start (`model::crosshair::CpuPreview`): placed awake
/// with a Fall request first, then held asleep there, as the drill's own start does. With the body at the point the
/// game draws it there, or its own magnifier with the direction pointer when the point is out of the camera's view;
/// the game's `FLAG_IS_LOUPE` is read back. `Some(asleep)`; `None` when the preview cannot run here.
pub(super) unsafe fn preview(slot: &mut Option<CpuPreview>, body: usize, id: u32, at: [f32; 2]) -> Option<bool> {
    if !preview_ready() {
        *slot = None;
        return None;
    }
    let act = crosshair::preview_step(slot, id, || Origin {
        at: [natives::pos_x(body), natives::pos_y(body)],
        grounded: natives::situation_kind(body) == konst(C::SituationGround),
        facing: natives::lr(body),
    });
    let asleep = match act {
        PreviewAct::Lift => {
            lift(body, at, natives::lr(body));
            fall(body);
            false
        }
        PreviewAct::Hold => {
            hold(body, at);
            true
        }
    };
    let loupe = konst(C::WorkFlagIsLoupe);
    if let Some(preview) = slot {
        preview.magnified = loupe != UNRESOLVED && natives::is_flag(body, loupe);
    }
    Some(asleep)
}

/// Put a previewed body back where its picker found it. A preview only exists where `preview_ready` held.
pub(super) unsafe fn restore(body: usize, origin: Origin) {
    if origin.grounded {
        stand(body, origin.at, origin.facing);
    } else {
        lift(body, origin.at, origin.facing);
        fall(body);
    }
}
