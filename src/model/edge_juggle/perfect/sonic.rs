//! Sonic's measured Spin Dash crossing, double jump, and turned Spring.

use super::{niche::pad, region, Correction, Point, RouteFacts, JUMP, SPECIAL};
use crate::model::cpu::ids::UNRESOLVED;

pub const KIND: i32 = 0x29;
pub const ROW: u8 = 23;
pub const WIZARD_OFFSET: Point = [26.0, 34.0];
const FALL: i32 = 0xe;
const FALL_AERIAL: i32 = 0xf;
const AERIAL_JUMP: i32 = 0xc;
const SIDE: i32 = 0x1dd;
const CHARGE: i32 = 0x1e8;
const SPIN: i32 = 0x1ea;
const SPRING: i32 = 0x1de;
const PROPEL: i32 = 0x1ef;
const FORWARD: Point = [1.0, 0.0];
const UP_BACK: Point = [-0.5, 104.0 / 120.0];

type Pose = [f64; 2];
const LEFT_X: f64 = -79.986;
const RIGHT_X: f64 = 79.989;
const RIGHT_BOTTOM: Pose = [41.416, -40.118];
const RIGHT_KNEE: Pose = [64.387, -25.103];
const TURN_X: f64 = 65.4;
const PREPARE: u16 = 17;
const FLOOR: f64 = -134.0;
const MIN_CORNER: f64 = 5.8;
const MIN_SPARE: f64 = 3.0;
const FULL_SPIN_STEP: Pose = [2.0, -2.4];

const REGION: [u32; 44] = [
    0, 0xfc0, 0xfc0, 0x7c0, 0x3c0, 0x1c0, 0xc0, 0x40, 0x40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
const HULL: [Pose; 6] = [
    [LEFT_X, -8.319],
    [-64.395, -25.111],
    [-41.396, -40.099],
    RIGHT_BOTTOM,
    RIGHT_KNEE,
    [RIGHT_X, -8.319],
];

// Free Spring deltas from the exact 990288d6 stock flight (fg2-m-xps-6).
const SPRING_DELTAS: [Pose; 51] = [
    [0.000000000, 10.500000000],
    [0.000000000, 0.000000000],
    [0.000000000, 0.000000000],
    [0.035995483, 6.790000916],
    [0.072006226, 5.790000916],
    [0.108001709, 4.790000916],
    [0.143997192, 3.790000916],
    [0.179992676, 2.790000916],
    [0.216003418, 2.699996948],
    [0.251998901, 2.610000610],
    [0.287994385, 2.519996643],
    [0.324005127, 2.430000305],
    [0.360000610, 2.339996338],
    [0.396003723, 2.250000000],
    [0.431999207, 2.160003662],
    [0.468002319, 2.069999695],
    [0.503997803, 1.980003357],
    [0.540000916, 1.889999390],
    [0.575996399, 1.800003052],
    [0.603996277, 1.709999084],
    [0.603996277, 1.620002747],
    [0.603996277, 1.529998779],
    [0.603996277, 1.440002441],
    [0.603996277, 1.349998474],
    [0.603996277, 1.260002136],
    [0.603996277, 1.169998169],
    [0.603996277, 1.079994202],
    [0.603996277, 0.990005493],
    [0.603996277, 0.899993896],
    [0.603996277, 0.809997559],
    [0.603996277, 0.720001221],
    [0.603996277, 0.630004883],
    [0.603996277, 0.539993286],
    [0.603996277, 0.449996948],
    [0.603996277, 0.360000610],
    [0.603996277, 0.270004272],
    [0.603996277, 0.179992676],
    [0.603996277, 0.089996338],
    [0.603996277, 0.000000000],
    [0.603996277, -0.089996338],
    [0.603996277, -0.180007935],
    [0.603996277, -0.270004272],
    [0.603996277, -0.360000610],
    [0.603996277, -0.449996948],
    [0.603996277, -0.539993286],
    [0.603996277, -0.630004883],
    [0.603996277, -0.720001221],
    [0.603996277, -0.809997559],
    [0.603996277, -0.899993896],
    [0.603996277, -0.990005493],
    [0.603996277, -1.080001831],
];

#[derive(Clone, Copy)]
pub struct SonicBeat {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub facing: f32,
    pub jumps: u8,
    pub can_spindash: bool,
    pub can_jump: bool,
    pub can_spring: bool,
    pub spring_reach_valid: bool,
}

#[derive(Clone, Copy)]
struct Timing {
    side: u16,
    dash: u16,
    aerial: u16,
}

impl Timing {
    fn jump(self) -> u16 {
        self.side + PREPARE + self.dash - 1
    }

    fn spring(self) -> u16 {
        self.jump() + self.aerial
    }
}

struct Plan {
    timing: Timing,
    arrival: u16,
    spin: Pose,
    jump: Pose,
    spring: Pose,
}

pub struct SonicFlight {
    near: Point,
    far: Point,
    release: Pose,
    downward: f64,
    plan: Plan,
    origin: Option<u32>,
    last_pad: Option<(u32, Correction)>,
    last_spin: Option<(u32, Pose, bool)>,
}

fn add([a, b]: Pose, [x, y]: Pose) -> Pose {
    [a + x, b + y]
}

fn underside(x: f64) -> Option<f64> {
    HULL.windows(2).find_map(|span| {
        let [a, b] = [span[0], span[1]];
        (a[0] <= x && x <= b[0]).then(|| a[1] + (b[1] - a[1]) * (x - a[0]) / (b[0] - a[0]))
    })
}

fn drop_after(ticks: u16, downward: f64) -> f64 {
    (1..=ticks)
        .map(|i| (downward + 0.09 * f64::from(i)).min(1.65))
        .sum()
}

fn spin_delta(step: u16) -> Pose {
    let i = f64::from(step);
    [(1.72 + 0.06 * i).min(2.0), (4.0 - 0.65 * i).max(-2.4)]
}

fn jump_delta(step: u16) -> Pose {
    let i = f64::from(step);
    let x = if step == 44 {
        1.198
    } else {
        (1.0 + 0.05 * i).min(1.208)
    };
    [x, 2.554_98 - 0.09 * i]
}

fn simulate_spin(x: f64, frames: u16) -> (Pose, f64) {
    let mut p = [x - 0.9, 0.0];
    let mut corner: f64 = 99.0;
    for step in 1..=frames {
        p = add(p, spin_delta(step));
        if p[0] < 0.0 {
            if let Some(bottom) = underside(p[0] + 1.5) {
                corner = corner.min(bottom - p[1]);
            }
        }
    }
    (p, corner)
}

fn simulate_jump(from: Pose, frames: u16) -> Pose {
    (1..=frames).fold(from, |p, i| add(p, jump_delta(i)))
}

fn spring_contact([x, y]: Pose, delta: Pose, riding: &mut bool) -> Pose {
    let [nx, ny] = add([x, y], delta);
    let gap = if nx < TURN_X { 6.6 } else { 7.5 };
    let meets_slope = nx > RIGHT_BOTTOM[0]
        && delta[1] > 0.0
        && underside(nx).is_some_and(|bottom| ny > bottom - gap);
    if !meets_slope {
        *riding = false;
        return [nx, ny];
    }
    if *riding {
        // Unit vectors of the two traced far-side ride surfaces.
        let slope = if x < TURN_X {
            [0.837044712366466, 0.547134489407622]
        } else {
            [0.680846017518227, 0.732426583644784]
        };
        return add([x, y], [delta[1] * slope[0], delta[1] * slope[1]]);
    }
    *riding = true;
    let slope_x = |a: Pose, b: Pose, gap| a[0] + (ny + gap - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
    let on_low = slope_x(RIGHT_BOTTOM, RIGHT_KNEE, 6.6);
    let on_face = if on_low < TURN_X {
        on_low
    } else {
        slope_x(RIGHT_KNEE, [RIGHT_X, -8.319], 7.5)
    };
    [nx.max(on_face), ny]
}

fn simulate_spring(mut from: Pose) -> Option<(u16, f64)> {
    if from[0] < 43.3 || underside(from[0])? - (from[1] + 10.5) < 7.6 {
        return None;
    }
    let mut riding = false;
    let mut catch = None;
    let mut spare: f64 = -99.0;
    for (i, delta) in SPRING_DELTAS.into_iter().enumerate() {
        from = spring_contact(from, delta, &mut riding);
        if (RIGHT_X - 3.9..=RIGHT_X + 16.0).contains(&from[0]) {
            spare = spare.max(from[1] + 23.24);
            if (-23.24..=-10.0).contains(&from[1]) {
                catch.get_or_insert(i as u16 + 1);
            }
        }
    }
    (spare >= MIN_SPARE).then_some((catch?, spare))
}

fn choose(release: Pose, out: f64, downward: f64) -> Option<Plan> {
    let central_dash = (58.0 + ((out - 44.0) / 2.0).ceil()) as u16;
    let dash_lines = [central_dash - 1, central_dash, central_dash + 1]
        .map(|dash| (dash, simulate_spin(release[0], dash)));
    let gravity_offset = drop_after(PREPARE, 0.0);
    let mut best = None;
    let mut best_margin = f64::NEG_INFINITY;
    // Same launch search as the reference bake. The common ring interval limits
    // this loop to the small set of waits that can clear both corners.
    for side in 0..=40 {
        let spin_y = release[1] - drop_after(side + PREPARE, downward);
        let band_y = spin_y + gravity_offset - 1.2 * (out - 44.0);
        if band_y > 30.0 {
            continue;
        }
        if band_y < 15.0 {
            break;
        }
        for (dash, (jump, corner_at_zero)) in dash_lines {
            if jump[1] + spin_y - 0.9 < FLOOR || corner_at_zero - spin_y < MIN_CORNER {
                continue;
            }
            for aerial in 41..=44 {
                let (mut arrival, mut spare) = (0, f64::INFINITY);
                let rise = simulate_jump(jump, aerial);
                // Align all modelled pre-rolls from the shared reference driver.
                for offset in [0.0, 0.09, 0.27, 0.54, 0.9] {
                    let Some((flight, extra)) = simulate_spring(add(rise, [0.0, spin_y - offset]))
                    else {
                        spare = f64::NEG_INFINITY;
                        break;
                    };
                    arrival = arrival.max(flight);
                    spare = spare.min(extra);
                }
                let margin = (spare - MIN_SPARE).min(corner_at_zero - spin_y - MIN_CORNER);
                if margin > best_margin {
                    let timing = Timing { side, dash, aerial };
                    best_margin = margin;
                    best = Some(Plan {
                        timing,
                        arrival: timing.spring() + 1 + arrival,
                        spin: [release[0] - 0.9, spin_y],
                        jump: add(jump, [0.0, spin_y]),
                        spring: add(rise, [0.0, spin_y]),
                    });
                }
            }
        }
    }
    best
}

fn below_underside([x, y]: Pose) -> bool {
    y >= FLOOR && underside(x + 1.5).is_none_or(|bottom| bottom - y >= MIN_CORNER)
}

fn resume_spin(plan: &Plan, age: u16, mut jump: Pose) -> Option<Plan> {
    let old = plan.timing;
    let mut best = None;
    let mut best_spare = f64::NEG_INFINITY;
    // Resume only the measured full-speed dash and jump profiles. The prior
    // dash horizon bounds the search and the blast floor ends it early.
    for spin in 0..old.dash {
        jump = add(jump, FULL_SPIN_STEP);
        if !below_underside(jump) {
            break;
        }
        let mut spring = jump;
        for aerial in 1..=44 {
            spring = add(spring, jump_delta(aerial));
            if !below_underside(spring) {
                break;
            }
            if aerial < 41 {
                continue;
            }
            let Some((flight, spare)) = simulate_spring(spring) else {
                continue;
            };
            if spare > best_spare {
                let dash = age.checked_add(spin)?.checked_sub(old.side + PREPARE - 1)?;
                let timing = Timing { dash, aerial, ..old };
                best_spare = spare;
                best = Some(Plan {
                    timing,
                    arrival: timing.spring() + 1 + flight,
                    spin: plan.spin,
                    jump,
                    spring,
                });
            }
        }
    }
    best
}

fn relative(near: Point, p: Point) -> Pose {
    [
        LEFT_X + f64::from(p[0]) - f64::from(near[0]),
        f64::from(p[1]) - f64::from(near[1]),
    ]
}

fn guarder_above_lip(near: Point, player: Point) -> bool {
    // The dash passes through the near ledge's hang point. A hit there
    // interrupts the measured Spin Dash timing and displaces its jump.
    player.into_iter().all(f32::is_finite) && player[1] >= near[1]
}

/// Row 23's guarder stands: grounded, and in Wait or in the edge-teeter wait the game puts
/// a fighter in at the lip (operator ruling 2026-11-22; f3b-sonic-b-3, 55 of 55 placements).
pub fn guarder_standing(grounded: bool, status: i32, wait: i32, teeter_wait: i32) -> bool {
    grounded
        && [wait, teeter_wait]
            .into_iter()
            .any(|standing| standing != UNRESOLVED && status == standing)
}

fn on_line(actual: Pose, expected: Pose) -> bool {
    actual
        .into_iter()
        .zip(expected)
        .all(|(a, b)| (a - b).abs() <= 0.26)
}

impl SonicFlight {
    pub fn start(facts: &RouteFacts<'_>, velocity: Point, beat: SonicBeat) -> Option<Self> {
        facts.crossing_band()?;
        let [near, far] = facts.ledges;
        let out = near[0] - beat.position[0];
        if facts.fighter != KIND
            || !guarder_above_lip(near, facts.player)
            || facts.position != beat.position
            || beat.status != FALL
            || beat.facing != 1.0
            || beat.jumps != 1
            || !beat.can_spindash
            || !beat.can_jump
            || !beat.can_spring
            || velocity[0] != 0.0
            || !(-0.27..=0.0).contains(&velocity[1])
            || !region::contains(&REGION, out, beat.position[1] - near[1])
            || !region::has_common_underside(facts.ledges, facts.blast[3], facts.outline)
        {
            return None;
        }
        let release = relative(near, beat.position);
        let downward = f64::from(-velocity[1]);
        Some(Self {
            near,
            far,
            release,
            downward,
            plan: choose(release, f64::from(out), downward)?,
            origin: None,
            last_pad: None,
            last_spin: None,
        })
    }

    pub fn exit_point(&self) -> Point {
        self.far
    }

    pub fn crossing_frames(&self) -> u16 {
        self.plan.arrival
    }

    fn age(&self, frame: u32) -> u32 {
        frame.wrapping_sub(self.origin.unwrap_or(frame))
    }

    pub fn side_press(&self, frame: u32) -> bool {
        self.age(frame) == u32::from(self.plan.timing.side)
    }

    pub fn guarder_clear(&self, player: Point) -> bool {
        guarder_above_lip(self.near, player)
    }

    pub fn jump_press(&self, frame: u32) -> bool {
        self.age(frame) == u32::from(self.plan.timing.jump())
    }

    pub fn spring_press(&self, frame: u32) -> bool {
        self.age(frame) == u32::from(self.plan.timing.spring())
    }

    fn steady_spin(&mut self, beat: SonicBeat, actual: Pose) -> bool {
        if beat.status != SPIN {
            self.last_spin = None;
            return false;
        }
        let last = self.last_spin;
        let on_step = last.is_some_and(|(frame, position, _)| {
            beat.frame == frame.wrapping_add(1)
                && on_line(actual, add(position, FULL_SPIN_STEP))
        });
        self.last_spin = Some((beat.frame, actual, on_step));
        on_step && last.is_some_and(|(_, _, was_on_step)| was_on_step)
    }

    pub fn step(&mut self, beat: SonicBeat) -> Option<Correction> {
        let actual = relative(self.near, beat.position);
        if !actual.into_iter().all(f64::is_finite) || actual[1] <= FLOOR {
            return None;
        }
        if let Some((_, correction)) = self.last_pad.filter(|&(frame, _)| frame == beat.frame) {
            return Some(correction);
        }
        let age = u16::try_from(self.age(beat.frame)).ok()?;
        let timing = self.plan.timing;
        let steady_spin = age <= timing.jump() && self.steady_spin(beat, actual);
        if age == timing.jump()
            && !on_line(add(actual, spin_delta(timing.dash)), self.plan.jump)
        {
            if !steady_spin
                || beat.facing != 1.0
                || beat.jumps != 1
                || !beat.can_jump
                || !beat.spring_reach_valid
            {
                return None;
            }
            self.plan = resume_spin(&self.plan, age, actual)?;
        }
        let timing = self.plan.timing;
        let (side, jump, spring) = (timing.side, timing.jump(), timing.spring());
        let correction = if age <= side {
            let expected = add(self.release, [0.0, -drop_after(age, self.downward)]);
            if beat.status != FALL
                || beat.facing != 1.0
                || beat.jumps != 1
                || !on_line(actual, expected)
            {
                return None;
            }
            if age == side {
                beat.can_spindash.then(|| pad(FORWARD, SPECIAL))?
            } else {
                pad([0.0; 2], 0)
            }
        } else if age == side + 1 {
            (beat.status == SIDE && beat.facing == 1.0).then(|| pad(FORWARD, SPECIAL))?
        } else if age < jump {
            if age == side + PREPARE && !on_line(actual, self.plan.spin) {
                return None;
            }
            (matches!(beat.status, SIDE | CHARGE | SPIN) && beat.facing == 1.0)
                .then(|| pad(FORWARD, 0))?
        } else if age == jump {
            let after = add(actual, spin_delta(timing.dash));
            (beat.status == SPIN
                && beat.can_jump
                && beat.jumps == 1
                && beat.facing == 1.0
                && on_line(after, self.plan.jump)
                && simulate_spring(simulate_jump(after, timing.aerial)).is_some())
            .then(|| pad(FORWARD, JUMP))?
        } else if age == jump + 1 {
            (beat.status == AERIAL_JUMP && beat.jumps == 0).then(|| pad(FORWARD, JUMP))?
        } else if age < spring {
            matches!(beat.status, AERIAL_JUMP | FALL_AERIAL).then(|| pad(FORWARD, 0))?
        } else if age == spring {
            let after = add(actual, jump_delta(timing.aerial));
            (matches!(beat.status, AERIAL_JUMP | FALL_AERIAL)
                && beat.can_spring
                && beat.facing == 1.0
                && beat.jumps == 0
                && on_line(after, self.plan.spring)
                && simulate_spring(after).is_some())
            .then(|| pad(UP_BACK, SPECIAL))?
        } else if age == spring + 1 {
            (beat.status == SPRING && beat.facing == -1.0 && on_line(actual, self.plan.spring))
                .then(|| pad(UP_BACK, SPECIAL))?
        } else {
            (age <= spring + 52 && matches!(beat.status, SPRING | PROPEL) && beat.facing == -1.0)
                .then(|| pad(FORWARD, 0))?
        };
        self.origin.get_or_insert(beat.frame);
        self.last_pad = Some((beat.frame, correction));
        Some(correction)
    }
}

#[cfg(test)]
#[path = "sonic_tests.rs"]
mod tests;
