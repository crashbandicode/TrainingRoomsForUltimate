//! Per-attempt configuration sampling. Auto uses the explicitly approximate Aim
//! planner until native replay/contact/ECB acceptance; it promises no spot hit.
use super::controller::Setup;
use super::picker::Spot;
use super::solver::{contact_frames, Aim, Box2};
use super::surface::Surfaces;
use super::variables::{Di, Mode, Variable};
use super::{add, finite, scale, HitHeight, Launch};
use crate::model::tech_launch::{direction_degrees, FlightParams, DAMAGE_FRAME_MUL};
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
/// Why a draw has no plan; only the diagnostic log shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// A setting or a native value has nothing to offer.
    Values,
    /// No angle has a start for this spot and time.
    Aim,
    /// The start is outside the blast zone or the view, or inside the stage.
    Start,
    /// A manual start's ranges are still being checked: ask again next frame.
    Checking,
}
/// A manual start's check of its ranges, kept from one frame to the next while it runs.
#[derive(Clone, Copy, Debug, Default)]
pub struct Search {
    /// How many values the ranges hold; 0 before the first draw.
    count: u32,
    /// The next value to fly, in the ranges' order.
    next: u32,
    /// The values found to touch, and the one remembered among them.
    found: u32,
    kept: u32,
    /// The spots this attempt refused.
    pub refused: u8,
}
impl Search {
    pub fn running(&self) -> bool {
        self.count != 0
    }
}

/// A call's bounds, an Auto start's worst case: its 73 angles, of up to 60 frames each.
const FLIGHTS: u32 = 73;
const FRAMES: u32 = FLIGHTS * 60;

/// Where a candidate's values come from. Those that decide a manual flight (knockback, angle, start distance,
/// bearing) come from the draw, or are the `index`th value of their ranges in order.
struct Values<'a, D> {
    draw: &'a mut D,
    index: Option<u32>,
    /// How many values those ranges hold.
    count: u32,
    /// Frames flown.
    flown: u32,
}
impl<D: FnMut(u32) -> u32> Values<'_, D> {
    fn pick(&mut self, n: u32) -> u32 {
        self.count = self.count.saturating_mul(n);
        if let Some(index) = &mut self.index {
            let digit = *index % n;
            *index /= n;
            digit
        } else {
            (self.draw)(n)
        }
    }
}
/// The candidates one attempt may draw from the same settings, in one frame, before the drill stops. A draw without a
/// start is drawn again; values that leave none a start would draw for ever, and each candidate is up to 73 passes
/// of the solver on the game frame, so the count is bounded.
pub const CANDIDATES: u8 = 8;
/// One attempt: `candidate` draws a spot and its plan. `Ok(Some(_))` is taken, with which candidate it was;
/// `Ok(None)` is drawn again; `Err` ends the attempt at once (something other than the draw is missing).
/// `Ok(None)` from here: none of the [`CANDIDATES`] has a start.
pub fn draw_attempt<T, E>(mut candidate: impl FnMut() -> Result<Option<T>, E>) -> Result<Option<(T, u8)>, E> {
    for n in 1..=CANDIDATES {
        if let Some(plan) = candidate()? {
            return Ok(Some((plan, n)));
        }
    }
    Ok(None)
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
) -> Result<Plan, Refused> {
    let mut search = Search::default();
    loop {
        match plan_frame(config, &context, draw, &mut search) {
            Err(Refused::Checking) => {}
            result => return result,
        }
    }
}

/// One frame's share of [`plan`].
pub fn plan_frame(
    config: Config,
    context: &Context<'_>,
    draw: &mut impl FnMut(u32) -> u32,
    search: &mut Search,
) -> Result<Plan, Refused> {
    let result = (|| {
        let longest = ((config.speed.lo.max(config.speed.hi) * DAMAGE_FRAME_MUL) as u32)
            .saturating_add(u32::from(super::session::ABANDON));
        let mut candidates = 0u32;
        let mut frames = 0u32;
        if !search.running() {
            let mut values = Values { draw: &mut *draw, index: None, count: 1, flown: 0 };
            let result = candidate(config, context, &mut values);
            candidates += 1;
            frames += values.flown;
            match result {
                Err(Refused::Aim | Refused::Start)
                    if config.start.mode != SettingMode::Auto && values.count > 1 =>
                {
                    search.count = values.count;
                }
                other => return other,
            }
        }
        loop {
            if search.next == search.count {
                if search.found == 0 {
                    return Err(Refused::Aim);
                }
                if candidates != 0 && (candidates + 1 > FLIGHTS || frames.saturating_add(longest) > FRAMES) {
                    return Err(Refused::Checking);
                }
                let mut values = Values { draw: &mut *draw, index: Some(search.kept), count: 1, flown: 0 };
                return candidate(config, context, &mut values);
            }
            if candidates != 0
                && (candidates + 2 > FLIGHTS || frames.saturating_add(longest.saturating_mul(2)) > FRAMES)
            {
                return Err(Refused::Checking);
            }
            let mut values = Values { draw: &mut *draw, index: Some(search.next), count: 1, flown: 0 };
            let ordered = candidate(config, context, &mut values);
            candidates += 1;
            frames += values.flown;
            if ordered.is_ok() {
                search.found += 1;
                if draw(search.found) == 0 {
                    search.kept = search.next;
                }
            }
            search.next += 1;
            let mut values = Values { draw: &mut *draw, index: None, count: 1, flown: 0 };
            let chance = candidate(config, context, &mut values);
            candidates += 1;
            frames += values.flown;
            match chance {
                Err(Refused::Aim | Refused::Start) => {}
                other => return other,
            }
        }
    })();
    if !matches!(result, Err(Refused::Checking)) {
        search.count = 0;
        search.next = 0;
        search.found = 0;
        search.kept = 0;
    }
    result
}

fn candidate(
    config: Config,
    context: &Context<'_>,
    values: &mut Values<'_, impl FnMut(u32) -> u32>,
) -> Result<Plan, Refused> {
    if !context.spot.on_segment() { return Err(Refused::Aim); }
    let multiplier = context
        .damage_multiplier
        .filter(|m| m.is_finite() && *m > 0.)
        .ok_or(Refused::Values)?;
    let target = context.spot.point();
    let knockback = sample(config.speed, 10., &mut |n| values.pick(n)).ok_or(Refused::Values)?;
    let percent = sample(config.damage, 5., &mut *values.draw).ok_or(Refused::Values)?;
    let visible_hit_damage = sample(config.hit_damage, 1., &mut *values.draw).ok_or(Refused::Values)?;
    let hit_damage = base_hit_damage(visible_hit_damage, Some(multiplier)).ok_or(Refused::Values)?;
    let mut time = config.time;
    if config.start.mode == SettingMode::Auto && time.mode == SettingMode::Random {
        if !time.lo.is_finite() || !time.hi.is_finite() {
            return Err(Refused::Values);
        }
        let allowed = contact_frames(knockback);
        time.lo = time.lo.max(*allowed.start());
        time.hi = time.hi.min(*allowed.end());
    }
    let frames = sample(time, 1., &mut *values.draw).ok_or(Refused::Values)? as u16;
    let delay = sample(config.delay, 1., &mut *values.draw).ok_or(Refused::Values)? as u16;
    let angle = sample(config.angle, 5., &mut |n| values.pick(n));
    let start_offset = sample(config.start, 1., &mut |n| values.pick(n));
    let aim = Aim {
        surfaces: context.surfaces,
        spot: context.spot,
        knockback,
        params: context.params,
        blast: context.blast,
        view: context.view,
    };
    let (start, angle, approximate) = if let Some(offset) = start_offset {
        // A fixed distance follows the selected line's outward normal. Random
        // start uses a ring around the point, drawing its bearing each attempt.
        let direction = if config.start.mode == SettingMode::Random {
            direction_degrees(values.pick(360) as f32).ok_or(Refused::Values)?
        } else {
            context.spot.line.normal
        };
        let start = add(target, scale(direction, offset));
        let angle = angle.unwrap_or_else(|| {
            (target.y - start.y)
                .atan2(target.x - start.x)
                .to_degrees()
                .rem_euclid(360.)
        });
        (start, angle, config.angle.mode == SettingMode::Auto)
    } else {
        let solution = aim.solve(angle, frames).ok_or(Refused::Aim)?;
        (solution.start, solution.angle, true)
    };
    if !finite(start)
        || !context.blast.contains(start)
        || context.view.is_some_and(|v| !v.contains(start))
        || super::solver::inside(context.surfaces, start)
    {
        return Err(Refused::Start);
    }
    if start_offset.is_some() {
        let (touch, flown) = aim.contact(start, angle);
        values.flown += u32::from(flown);
        if touch.is_none() {
            return Err(Refused::Aim);
        }
    }
    let face_right = match config.facing {
        1 => false,
        2 => true,
        3 => (values.draw)(2) != 0,
        _ => target.x >= start.x,
    };
    let di = match config.di {
        1 => Di::Player,
        2 => {
            // DirectionGrid's canonical mapping is supplied by the settings
            // decoder; all finite unit-grid values are native stick requests.
            let mut stick = crate::model::tech_settings::di_vector(config.di_direction);
            if target.x > start.x && config.di_direction < 8 {
                stick.x = -stick.x;
            }
            Di::Set(stick)
        }
        3 => Di::Random,
        _ => Di::Off,
    }
    .sample(&mut *values.draw);
    let launch = Launch {
        start,
        target,
        angle,
        knockback,
        percent: percent + hit_damage * multiplier,
        hit_damage,
        height: HitHeight::Middle,
        facing: if face_right { 1. } else { -1. },
        hit_from_behind: false,
        grounded_hit: false,
        di,
    };
    if !launch.valid() {
        return Err(Refused::Values);
    }
    Ok(Plan {
        setup: Setup { launch, delay },
        approximate,
        contact_frames: frames,
        sampled_percent: [percent, visible_hit_damage],
    })
}
