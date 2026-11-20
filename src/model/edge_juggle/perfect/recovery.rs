//! Player-grade recovery decisions for Guard the Edge. Pure: the runtime applies pad edits only.

use super::{
    AirPhysics, Correction, Plan, Point, Route, SpecialReach, ATTACK, GUARD, SPECIAL,
};

/// Where the edge-guarder is against the fighter's near ledge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Far,
    Mid,
    Ledge,
    Hang,
    OffHigh,
    OffLow,
}

#[derive(Clone, Copy)]
pub struct Guarder {
    pub position: Point,
    pub hanging: bool,
    pub airborne: bool,
    /// Horizontal threat radius: body height and the live attack volume.
    pub reach: f32,
}

pub fn zone(guard: Guarder, ledge: Point, cpu: Point) -> Zone {
    let inward = (ledge[0] - cpu[0]).signum();
    let depth = (guard.position[0] - ledge[0]) * inward;
    let reach = guard.reach;
    if !(depth.is_finite() && guard.position[1].is_finite() && reach.is_finite() && reach > 0.0) {
        return Zone::Far;
    }
    if guard.hanging && depth.abs() <= reach {
        Zone::Hang
    } else if depth < 0.0 && guard.airborne {
        if guard.position[1] >= cpu[1] {
            Zone::OffHigh
        } else {
            Zone::OffLow
        }
    } else if depth <= reach {
        Zone::Ledge
    } else if depth <= reach * 3.0 {
        Zone::Mid
    } else {
        Zone::Far
    }
}

/// Near, High, LateLow and Opposite against each zone, in `Zone` order.
const ZONE_WEIGHTS: [[u8; 4]; 6] = [
    [8, 4, 2, 0],
    [5, 3, 5, 1],
    [2, 3, 6, 5],
    [0, 8, 1, 3],
    [3, 0, 8, 3],
    [4, 8, 0, 1],
];

/// Roll one feasible route, weighted against the guarder and the last route.
/// A zero zone weight removes a route only while another feasible one remains.
pub fn mix(options: [Option<Plan>; 4], zone: Zone, last: Option<Route>, draw: u32) -> Option<Plan> {
    let weight = |plan: Plan| {
        let column = match plan.route {
            Route::Near => 0,
            Route::High => 1,
            Route::LateLow => 2,
            Route::Opposite => 3,
        };
        let weight = u32::from(plan.weight) * u32::from(ZONE_WEIGHTS[zone as usize][column]);
        if Some(plan.route) == last {
            weight.div_ceil(2)
        } else {
            weight
        }
    };
    let total: u32 = options.iter().flatten().map(|&plan| weight(plan)).sum();
    if total == 0 {
        return options.into_iter().flatten().max_by_key(|plan| plan.weight);
    }
    let mut draw = ((u64::from(draw) * u64::from(total)) >> 32) as u32;
    for plan in options.into_iter().flatten() {
        let weight = weight(plan);
        if draw < weight {
            return Some(plan);
        }
        draw -= weight;
    }
    None
}

/// What the fighter still has for the return: the jump, then the up special.
#[derive(Clone, Copy)]
pub struct Reach {
    pub air: AirPhysics,
    pub jump: bool,
    pub special: Option<SpecialReach>,
}

const REACH_FRAMES: u8 = 90;
/// The stock up-special table is the stock CPU's own estimate: timing trusts this share of it.
const TRUST: f32 = 0.85;

impl Reach {
    pub fn trusted(air: AirPhysics, jump: bool, special: Option<SpecialReach>) -> Self {
        let special = special.map(|reach| SpecialReach {
            horizontal: reach.horizontal * TRUST,
            rise: reach.rise * TRUST,
        });
        Self { air, jump, special }
    }

    /// Drift at the ledge and act on frame `delay`: the jump when one is left,
    /// then the up special from the first frame its envelope holds the ledge.
    pub fn returns(self, from: Point, speed: Point, ledge: Point, delay: u8, bottom: f32) -> bool {
        if !self.air.valid()
            || !from
                .into_iter()
                .chain(speed)
                .chain(ledge)
                .chain([bottom])
                .all(f32::is_finite)
        {
            return false;
        }
        let air = self.air;
        let direction = (ledge[0] - from[0]).signum();
        let mut dx = (ledge[0] - from[0]).abs();
        let mut y = from[1];
        let [mut vx, mut vy] = [speed[0] * direction, speed[1]];
        for frame in 0..=REACH_FRAMES {
            if y <= bottom {
                return false;
            }
            let at = [ledge[0] - dx * direction, y];
            if frame >= delay && self.special.is_some_and(|special| special.reaches(at, ledge)) {
                return true;
            }
            vy = if frame == delay && self.jump {
                air.jump_speed
            } else {
                (vy - air.gravity).max(-air.terminal_speed)
            };
            vx = (vx + air.horizontal_acceleration).min(air.horizontal_speed);
            // Stay in the ledge column while the jump or up special gains height.
            dx = (dx - vx).max(0.0);
            y += vy;
            if dx == 0.0 {
                if y >= ledge[1] {
                    return true;
                }
                if vy <= 0.0 && (!self.jump || frame >= delay)
                    && self.special.is_none_or(|special| y + special.rise < ledge[1])
                {
                    return false;
                }
            }
        }
        false
    }

    /// The last frame the fighter may wait: the point of no return.
    pub fn deadline(self, from: Point, speed: Point, ledge: Point, bottom: f32) -> Option<u8> {
        (0..=REACH_FRAMES)
            .take_while(|&delay| self.returns(from, speed, ledge, delay, bottom))
            .last()
    }

    fn jump_rise(self) -> f32 {
        if self.jump {
            self.air.jump_speed.powi(2) / (2.0 * self.air.gravity)
        } else {
            0.0
        }
    }
}

/// The frame after the plan on which a held jump or special is spent.
pub fn planned_delay(route: Route, zone: Zone, deadline: Option<u8>, draw: u32) -> u8 {
    let Some(deadline) = deadline else { return 0 };
    match (route, zone) {
        (Route::LateLow, _) => deadline.saturating_sub(3),
        (Route::Near, Zone::Far) | (Route::High | Route::Opposite, _) => 0,
        (Route::Near, _) => (draw % (u32::from(deadline) / 3 + 1)) as u8,
    }
}

#[derive(Clone, Copy)]
pub struct HoldFacts {
    /// The attempt's game frame: several think calls can share one.
    pub frame: u32,
    pub buttons: u32,
    /// The held resource: `JUMP` or `SPECIAL`.
    pub button: u32,
    pub planned: u8,
    /// Spending it next frame still returns.
    pub can_wait: bool,
    /// The guarder's live attack covers the ledge.
    pub hot: bool,
}

/// A stock jump or up special held back until its planned frame.
#[derive(Default)]
pub struct Hold {
    start: Option<u32>,
    pending: bool,
    last: Option<(u32, Correction)>,
}

impl Hold {
    pub fn waiting(&self) -> bool {
        self.pending
    }

    pub fn step(&mut self, facts: HoldFacts) -> Correction {
        if let Some((_, out)) = self.last.filter(|(frame, _)| *frame == facts.frame) {
            // Another think on the same game frame repeats the pad.
            return out;
        }
        let age = facts.frame.saturating_sub(*self.start.get_or_insert(facts.frame));
        let mut out = Correction::default();
        if facts.buttons & (GUARD | ATTACK) != 0 {
            // A stock defensive answer passes and keeps the resource.
            self.pending = false;
        } else if facts.can_wait && (age < u32::from(facts.planned) || facts.hot) {
            // The up special never goes before a jump held in reserve.
            out.release = facts.buttons & (facts.button | SPECIAL);
            self.pending |= out.release != 0;
        } else if std::mem::take(&mut self.pending) {
            out.press = facts.button;
        }
        self.last = Some((facts.frame, out));
        out
    }
}

/// The guarder's attack at the ledge: dangerous from the frame it can be seen
/// until its hit volume has come and gone. Its end lag is the time to arrive.
#[derive(Default)]
pub struct LedgeWatch {
    attack: Option<u64>,
    since: u32,
    struck: bool,
}

impl LedgeWatch {
    pub fn hot(&mut self, attack: Option<u64>, volume: bool, frame: u32, reaction: u8) -> bool {
        if self.attack != attack {
            self.attack = attack;
            self.since = frame;
            self.struck = false;
        }
        self.struck |= volume;
        attack.is_some()
            && (volume || !self.struck)
            && frame.saturating_sub(self.since) >= u32::from(reaction)
    }
}

/// The plain return fails for distance while the climb is still affordable.
pub fn short(reach: Reach, from: Point, speed: Point, ledge: Point, bottom: f32) -> bool {
    let lift = reach.jump_rise() + reach.special.map_or(0.0, |special| special.rise);
    !reach.returns(from, speed, ledge, 0, bottom) && ledge[1] - from[1] <= lift
}

/// Trace-measured displacement of a row's side and down specials.
#[derive(Clone, Copy, Default)]
pub struct Measured {
    pub run: f32,
    pub rise: f32,
}

#[derive(Clone, Copy)]
pub struct Crossing {
    /// Horizontal distance to the exit point beyond the far ledge.
    pub span: f32,
    /// Foot heights that clear the underside and the blast zone.
    pub band: Point,
    pub foot: f32,
    /// Height of the far ledge.
    pub ledge: f32,
}

/// Spare climb left by an under-stage crossing as a fraction, or `None` when
/// jumps, glide, the measured specials and the up special do not cover it.
/// An aimed up special is shared between level travel and the final climb.
pub fn crossing_margin(
    air: AirPhysics,
    jumps: u8,
    special: Option<SpecialReach>,
    aimed: bool,
    measured: Measured,
    crossing: Crossing,
) -> Option<f32> {
    let Crossing {
        span,
        band: [bottom, top],
        foot,
        ledge,
    } = crossing;
    if !air.valid()
        || ![span, bottom, top, foot, ledge, measured.run, measured.rise]
            .into_iter()
            .all(f32::is_finite)
        || special.is_some_and(|special| !special.valid())
        || !(bottom < top && top <= ledge && foot >= bottom && span > 0.0)
    {
        return None;
    }
    let rise = air.jump_speed.powi(2) / (2.0 * air.gravity);
    let coast = air.terminal_speed.powi(2) / (2.0 * air.gravity);
    let down = if rise <= coast {
        (2.0 * rise / air.gravity).sqrt()
    } else {
        air.terminal_speed / air.gravity + (rise - coast) / air.terminal_speed
    };
    // One jump from the hold line returns to it after this much level travel.
    let hop = (air.jump_speed / air.gravity + down) * air.horizontal_speed;
    let under = if rise <= top - bottom { jumps } else { 0 };
    let [length, height] = special.map_or([0.0; 2], |reach| [reach.horizontal, reach.rise]);
    // The crossing's entry waits for the foot to reach the corridor.
    let start = (foot.min(top) + measured.rise).min(top);
    let mut best = None;
    for spent in 0..=under {
        for share in 0..=4u8 {
            let level = if aimed { f32::from(share) * 0.25 } else { 0.0 };
            let glide = span - measured.run - f32::from(spent) * hop - level * length;
            let exit = start - glide.max(0.0) * air.terminal_speed / air.horizontal_speed;
            let lift = height * (1.0 - level) + f32::from(jumps - spent) * rise;
            let spare = lift - (ledge - exit);
            if exit >= bottom && spare >= 0.0 {
                let margin = (spare / (ledge - bottom)).min(1.0);
                best = Some(best.map_or(margin, |known: f32| known.max(margin)));
            }
        }
    }
    best
}

/// Level travel an aimed up special still has after keeping the climb to the ledge.
pub fn level_run(special: SpecialReach, climb: f32) -> f32 {
    if special.valid() && climb.is_finite() {
        special.horizontal * (1.0 - (climb / special.rise).clamp(0.0, 1.0))
    } else {
        0.0
    }
}

/// A fighter row offered to the roll as its route, weighted by its spare climb.
pub fn niche_plan(route: Route, margin: f32, exit: Point) -> Plan {
    Plan {
        route,
        waypoints: [exit; 3],
        count: 1,
        weight: 1 + (margin.clamp(0.0, 1.0) * 15.0) as u16,
    }
}

/// The point whose far side is the stage: DI "away" from it bends at the ledge.
pub fn di_anchor(cpu: Point, ledge: Point) -> Point {
    [2.0 * cpu[0] - ledge[0], cpu[1]]
}

#[cfg(test)]
lib_unit_tests! {
    #[path = "../../../../tests/common/edge_juggle_recovery.rs"]
    mod tests;
}
