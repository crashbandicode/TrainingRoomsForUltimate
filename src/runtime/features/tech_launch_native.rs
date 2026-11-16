//! The one V9 mutation sequence used by both product and diagnostic launchers.
use super::{tech_info_seed::Seed, tech_native};
use crate::model::tech_launch::Vec2;
use crate::runtime::game::natives;
use crate::runtime::state_natives::{self, Vector3f};
#[derive(Clone, Copy)]
pub(super) struct Plan {
    pub start: Vec2,
    pub facing: f32,
    pub percent: f32,
    pub reaction: f32,
    pub seed: Seed,
    pub frames: i32,
    pub reserve_lr: Option<f32>,
    pub ground_to_air: bool,
}
pub(super) enum Moment {
    Placed,
    Reaction,
    Reserve,
    InfoBefore([u8; 0xa0]),
    InfoAfter([u8; 0xa0]),
    Stop {
        stopped: bool,
        damage: bool,
    },
    Initialized,
    Action {
        written: bool,
        value: i32,
        readback: i32,
    },
    Speed(Vec2),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Error {
    Guard,
    Value,
    Observation,
    DamageStop { stopped: bool, damage: bool },
    Status,
}
/// Call only for a freshly resolved, owned P1 on the game thread. Observer never owns mutation.
pub(super) unsafe fn launch(
    accessor: usize,
    plan: Plan,
    mut observe: impl FnMut(Moment) -> Option<()>,
) -> Result<Vec2, Error> {
    if !tech_native::launch_ready(accessor) {
        return Err(Error::Guard);
    }
    let id = |name| tech_native::id(name).ok_or(Error::Value);
    let heights = [
        id("HIT_HEIGHT_LOW")?,
        id("HIT_HEIGHT_CENTER")?,
        id("HIT_HEIGHT_HIGH")?,
    ];
    let air = id("SITUATION_KIND_AIR")?;
    let ground = id("SITUATION_KIND_GROUND")?;
    let fly = id("FIGHTER_STATUS_KIND_DAMAGE_FLY")?;
    let reserve = id("FIGHTER_STATUS_WORK_ID_FLOAT_RESERVE_DAMAGE_LR")?;
    let action = id("FIGHTER_STATUS_DAMAGE_WORK_INT_STOP_RELEASE_ACTION")?;
    let ground_to_air = id("FIGHTER_STATUS_DAMAGE_STOP_RELEASE_ACTION_GROUND_TO_AIR")?;
    if !plan.seed.valid(heights)
        || plan.frames <= 0
        || !plan.reaction.is_finite()
        || plan.reaction <= 0.0
        || !plan.percent.is_finite()
        || plan.percent < 0.0
        || !plan.start.x.is_finite()
        || !plan.start.y.is_finite()
        || !plan.facing.is_finite()
        || plan.reserve_lr.is_some_and(|x| !x.is_finite())
    {
        return Err(Error::Value);
    }
    // The product hold does not manufacture a grounded situation. Refuse before mutation.
    if plan.ground_to_air
        && (natives::situation_kind(accessor) != ground || !natives::is_grounded(accessor))
    {
        return Err(Error::Value);
    }
    state_natives::clear_speed_all(accessor);
    state_natives::init_pos(
        accessor,
        &Vector3f {
            x: plan.start.x,
            y: plan.start.y,
            z: 0.0,
        },
    );
    state_natives::set_lr(accessor, plan.facing);
    state_natives::heal(accessor, -natives::damage(accessor, 0));
    state_natives::add_damage(accessor, plan.percent);
    if !plan.ground_to_air {
        state_natives::set_situation_kind(accessor, air);
    }
    observe(Moment::Placed).ok_or(Error::Observation)?;
    natives::tech_overwrite_reaction(accessor, plan.reaction).ok_or(Error::Guard)?;
    observe(Moment::Reaction).ok_or(Error::Observation)?;
    if let Some(lr) = plan.reserve_lr {
        natives::set_float(accessor, lr, reserve);
    }
    observe(Moment::Reserve).ok_or(Error::Observation)?;
    let before = tech_native::damage_info(accessor).ok_or(Error::Guard)?;
    observe(Moment::InfoBefore(before)).ok_or(Error::Observation)?;
    let module = ((accessor + 0xa8) as *const usize).read();
    if !plan.seed.write((module + 0x80) as *mut u8, heights) {
        return Err(Error::Value);
    }
    observe(Moment::InfoAfter(
        tech_native::damage_info(accessor).ok_or(Error::Guard)?,
    ))
    .ok_or(Error::Observation)?;
    // V8: init must observe the damage stop and choose the native deferred-motion path.
    natives::set_hit_stop_frame(accessor, plan.frames, true).ok_or(Error::Guard)?;
    let (stopped, damage) = (
        natives::is_stop(accessor),
        natives::tech_is_damage(accessor).ok_or(Error::Guard)?,
    );
    observe(Moment::Stop { stopped, damage }).ok_or(Error::Observation)?;
    if !stopped || !damage {
        return Err(Error::DamageStop { stopped, damage });
    }
    natives::change_status_force(accessor, fly, false);
    if natives::status_kind(accessor) != fly {
        return Err(Error::Status);
    }
    observe(Moment::Initialized).ok_or(Error::Observation)?;
    // Native grounded damage init owns the release action; never seed the work value.
    let readback = natives::get_int(accessor, action);
    observe(Moment::Action {
        written: false,
        value: if plan.ground_to_air {
            ground_to_air
        } else {
            -1
        },
        readback,
    })
    .ok_or(Error::Observation)?;
    if plan.ground_to_air && readback != ground_to_air {
        return Err(Error::Status);
    }
    let speed = tech_native::damage_velocity(accessor).ok_or(Error::Value)?;
    observe(Moment::Speed(speed)).ok_or(Error::Observation)?;
    Ok(speed)
}
