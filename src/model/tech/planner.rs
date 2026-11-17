//! Per-attempt configuration sampling. Auto uses the explicitly approximate Aim
//! planner until native replay/contact/ECB acceptance; it promises no spot hit.
use super::controller::Setup;
use super::picker::Spot;
use super::solver::{Aim, Box2};
use super::surface::Surfaces;
use super::variables::{Di, Mode, Variable};
use super::{add, finite, scale, HitHeight, Launch};
use crate::model::tech_launch::{direction_degrees, FlightParams};
use crate::model::tech_settings::{Config, Mode as SettingMode, Numeric};

pub struct Context<'a> {
    pub surfaces: &'a Surfaces,
    pub spot: Spot,
    pub params: FlightParams,
    pub blast: Box2,
    pub view: Option<Box2>,
    /// Fresh native percent multiplier; absence never supplies a default.
    pub damage_multiplier: Option<f32>,
}
pub struct Plan {
    pub setup: Setup,
    pub approximate: bool,
    pub contact_frames: u16,
    /// Sampled [pre-hit percent, visible added percent], retained for fresh retries.
    pub sampled_percent: [f32; 2],
}
/// UI Hit damage is the visible added percent; the core accepts base damage.
pub fn base_hit_damage(visible: f32, multiplier: Option<f32>) -> Option<f32> {
    let multiplier = multiplier.filter(|m| m.is_finite() && *m > 0.)?;
    if !visible.is_finite() || !(1. ..=30.).contains(&visible) {
        return None;
    }
    let base = visible / multiplier;
    (base.is_finite() && base > 0.).then_some(base)
}
fn sample(n: Numeric, step: f32, draw: &mut impl FnMut(u32) -> u32) -> Option<f32> {
    Variable {
        mode: match n.mode {
            SettingMode::Set => Mode::Set,
            SettingMode::Random => Mode::Random,
            SettingMode::Auto => Mode::Auto,
        },
        low: n.lo,
        high: n.hi,
        step,
    }
    .sample(draw)
}
pub fn plan(
    config: Config,
    context: Context<'_>,
    draw: &mut impl FnMut(u32) -> u32,
) -> Option<Plan> {
    let multiplier = context
        .damage_multiplier
        .filter(|m| m.is_finite() && *m > 0.)?;
    let knockback = sample(config.speed, 10., draw)?;
    let percent = sample(config.damage, 5., draw)?;
    let visible_hit_damage = sample(config.hit_damage, 1., draw)?;
    let hit_damage = base_hit_damage(visible_hit_damage, Some(multiplier))?;
    let frames = sample(config.time, 1., draw)? as u16;
    let delay = sample(config.delay, 1., draw)? as u16;
    let angle = sample(config.angle, 5., draw);
    let start_offset = sample(config.start, 1., draw);
    let (start, angle, approximate) = if let Some(offset) = start_offset {
        // A fixed distance follows the selected line's outward normal. Random
        // start uses a ring around the point, drawing its bearing each attempt.
        let direction = if config.start.mode == SettingMode::Random {
            direction_degrees(draw(360) as f32)?
        } else {
            context.spot.line.normal
        };
        let start = add(context.spot.point, scale(direction, offset));
        let angle = angle.unwrap_or_else(|| {
            (context.spot.point.y - start.y)
                .atan2(context.spot.point.x - start.x)
                .to_degrees()
                .rem_euclid(360.)
        });
        (start, angle, config.angle.mode == SettingMode::Auto)
    } else {
        let solution = Aim {
            surfaces: context.surfaces,
            line: context.spot.line,
            target: context.spot.point,
            knockback,
            params: context.params,
            blast: context.blast,
            view: context.view,
        }
        .solve(angle, frames)?;
        (solution.start, solution.angle, true)
    };
    if !finite(start)
        || !context.blast.contains(start)
        || context.view.is_some_and(|v| !v.contains(start))
        || super::solver::inside(context.surfaces, start)
    {
        return None;
    }
    let facing = match config.facing {
        1 => -1.,
        2 => 1.,
        3 => {
            if draw(2) == 0 {
                -1.
            } else {
                1.
            }
        }
        _ => {
            if context.spot.point.x >= start.x {
                1.
            } else {
                -1.
            }
        }
    };
    let di = match config.di {
        1 => Di::Player,
        2 => {
            // DirectionGrid's canonical mapping is supplied by the settings
            // decoder; all finite unit-grid values are native stick requests.
            let mut stick = crate::model::tech_settings::di_vector(config.di_direction);
            if context.spot.point.x > start.x && config.di_direction < 8 {
                stick.x = -stick.x;
            }
            Di::Set(stick)
        }
        3 => Di::Random,
        _ => Di::Off,
    }
    .sample(draw);
    let launch = Launch {
        start,
        target: context.spot.point,
        angle,
        knockback,
        percent: percent + hit_damage * multiplier,
        hit_damage,
        height: HitHeight::Middle,
        facing,
        hit_from_behind: false,
        grounded_hit: false,
        di,
    };
    launch.valid().then_some(Plan {
        setup: Setup { launch, delay },
        approximate,
        contact_frames: frames,
        sampled_percent: [percent, visible_hit_damage],
    })
}
