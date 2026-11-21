//! Power Pellet under the stage, then double jump into two Pac-Jump bounces.
//! Flight rows and entry cells come from reference-crossings/g2/pacman.md.

use super::{aerial_clear, niche::pad, Correction, Outline, Point, RouteFacts, JUMP, SPECIAL};

pub const KIND: i32 = 0x37;
const FALL: i32 = 0xe;
const AIR_JUMP: i32 = 0xc;
const CHARGE: i32 = 0x1dd;
const HOLD: i32 = 0x1e6;
const STILL: i32 = 0x1e7;
const TRAIL: i32 = 0x1e8;
const DASH: i32 = 0x1ea;
const END: i32 = 0x1e9;
const PAC_JUMP: i32 = 0x1de;
const RISE: i32 = 0x1ec;
const BOUNCE_FALL: i32 = 0x1ed;
const SPECIAL_FALL: i32 = 0x10;

const GRAVITY: f32 = 0.072;
const TERMINAL: f32 = 1.35;
const AIR_SPEED: f32 = 1.092;
const ACCEL: f32 = 0.08;
const BRAKE: f32 = 0.015;
const CHARGE_ROWS: u32 = 30;
const STILL_ROWS: u32 = 4;
const END_START: u32 = 80;
const END_ROWS: u32 = 35;
const JUMP_ROWS: u32 = 31;
const JUMP_SPEED: f32 = 2.179_951_4; // sqrt(2 * gravity * aerial jump height) minus gravity / 2
const JUMP_BODY: f32 = 13.84;
const BODY_HALF: f32 = 0.0; // Source trajectory uses the center and measured status-specific ECB heights.
const PELLET_BODY: f32 = 11.54;

#[rustfmt::skip]
const TRAIL_AT_REST: [Point; 17] = [
    [0.9799, 0.0151],
    [1.1598, 0.0179],
    [1.3399, 0.0207],
    [1.5198, 0.0234],
    [1.6998, 0.0262],
    [1.8798, 0.029],
    [2.0597, 0.0318],
    [2.24, 0.0139],
    [2.4199, 0.015],
    [2.6, 0.0161],
    [2.78, 0.0002],
    [2.96, 0.0004],
    [3.0, 0.0002],
    [2.9679, 0.4379],
    [2.9678, 0.4378],
    [2.9984, 0.0982],
    [6.4257, 0.2103],
];

#[rustfmt::skip]
const TRAIL_11: [Point; 17] = [
    [0.9799, 0.0131],
    [1.1599, 0.0155],
    [1.3399, 0.0179],
    [1.5199, 0.0203],
    [1.6999, 0.0227],
    [1.8798, 0.0251],
    [2.0598, 0.0275],
    [2.24, 0.0121],
    [2.42, 0.0131],
    [2.6, 0.014],
    [2.78, 0.0002],
    [2.96, 0.0003],
    [3.0, 0.0002],
    [2.9683, 0.4351],
    [2.9683, 0.4351],
    [2.9983, 0.0998],
    [6.4695, 0.2154],
];

#[rustfmt::skip]
const TRAIL_FAST: [Point; 17] = [
    [0.98, 0.0],
    [1.16, 0.0],
    [1.34, 0.0],
    [1.52, 0.0],
    [1.7, 0.0],
    [1.88, 0.0],
    [2.06, 0.0],
    [2.24, 0.0],
    [2.42, 0.0],
    [2.6, 0.0],
    [2.78, 0.0],
    [2.96, 0.0],
    [3.0, 0.0],
    [2.9686, 0.433],
    [2.9686, 0.433],
    [2.9983, 0.0998],
    [6.4318, 0.2141],
];

#[rustfmt::skip]
const DASH_STEPS: [Point; 28] = [
    [3.0683, 0.0805],
    [3.0384, 0.0596],
    [3.0084, 0.0391],
    [2.9784, 0.0186],
    [2.9483, -0.0015],
    [2.9184, -0.0215],
    [2.8884, -0.0413],
    [2.8584, -0.0608],
    [2.8283, -0.0802],
    [2.7984, -0.0994],
    [2.7685, -0.1183],
    [2.7384, -0.137],
    [2.7085, -0.1555],
    [2.6785, -0.1738],
    [2.6486, -0.1919],
    [2.6187, -0.2096],
    [2.5887, -0.2273],
    [2.5589, -0.2447],
    [2.5291, -0.2618],
    [2.4991, -0.2787],
    [2.4694, -0.2954],
    [2.4396, -0.3119],
    [2.4098, -0.328],
    [2.3801, -0.344],
    [2.3504, -0.3597],
    [2.3207, -0.3751],
    [2.2912, -0.3904],
    [2.2615, -0.4053],
];

// Two measured-model seed programs for each admitted cell, near rest and terminal fall.
#[rustfmt::skip]
const SEEDS: [[i8; 10]; 38] = [
    [22, 34, 36, 19, 16, 12, 26, 20, 15, 12],
    [26, 34, 32, 23, 16, 14, 24, 22, 17, 13],
    [30, 34, 28, 27, 16, 14, 20, 26, 17, 13],
    [34, 34, 24, 31, 15, 12, 16, 30, 16, 12],
    [38, 34, 22, 33, 17, 13, 12, 34, 16, 14],
    [22, 30, 32, 20, 15, 12, 24, 19, 16, 12],
    [26, 30, 30, 22, 17, 13, 20, 23, 16, 14],
    [30, 30, 26, 26, 17, 13, 16, 27, 16, 14],
    [34, 30, 22, 30, 16, 12, 12, 31, 15, 12],
    [38, 30, 18, 34, 16, 14, 10, 33, 17, 13],
    [22, 26, 30, 19, 16, 12, 20, 20, 15, 12],
    [26, 26, 26, 23, 16, 14, 18, 22, 17, 13],
    [30, 26, 22, 27, 16, 14, 14, 26, 17, 13],
    [34, 26, 18, 31, 15, 12, 10, 30, 16, 12],
    [38, 26, 16, 33, 17, 13, 6, 34, 16, 14],
    [22, 22, 26, 20, 15, 12, 18, 19, 16, 12],
    [26, 22, 24, 22, 17, 13, 14, 23, 16, 14],
    [30, 22, 20, 26, 17, 13, 10, 27, 16, 14],
    [34, 22, 16, 30, 16, 12, 6, 31, 15, 12],
    [38, 22, 12, 34, 16, 14, 4, 33, 17, 13],
    [22, 18, 24, 19, 16, 12, 14, 20, 15, 12],
    [26, 18, 20, 23, 16, 14, 12, 22, 17, 13],
    [30, 18, 16, 27, 16, 14, 8, 26, 17, 13],
    [34, 18, 12, 31, 15, 12, 4, 30, 16, 12],
    [38, 18, 10, 33, 17, 13, 0, 34, 16, 14],
    [22, 14, 20, 20, 15, 12, 12, 19, 16, 12],
    [26, 14, 18, 22, 17, 13, 8, 23, 16, 14],
    [30, 14, 14, 26, 17, 13, 4, 27, 16, 14],
    [34, 14, 10, 30, 16, 12, 0, 31, 15, 12],
    [22, 10, 18, 19, 16, 12, 8, 20, 15, 12],
    [26, 10, 14, 23, 16, 14, 6, 22, 17, 13],
    [30, 10, 10, 27, 16, 14, 2, 26, 17, 13],
    [22, 6, 14, 20, 15, 12, 6, 19, 16, 12],
    [26, 6, 12, 22, 17, 13, 2, 23, 16, 14],
    [30, 6, 8, 26, 17, 13, 0, 25, 18, 12],
    [22, 2, 12, 19, 16, 12, 2, 20, 15, 12],
    [26, 2, 8, 23, 16, 14, 0, 22, 17, 13],
    [22, -2, 8, 20, 15, 12, 0, 19, 16, 12],
];

#[rustfmt::skip]
const JUMP_PROGRAMS: [[u16; 3]; 109] = [
    [3155, 1, 16],
    [4575, 1, 15],
    [5390, 2, 17],
    [5715, 1, 14],
    [6350, 1, 13],
    [6505, 1, 12],
    [6625, 2, 16],
    [6765, 1, 11],
    [7370, 1, 10],
    [7498, 3, 17],
    [7755, 2, 15],
    [8325, 1, 9],
    [8390, 2, 14],
    [8545, 2, 13],
    [8638, 3, 16],
    [8693, 4, 17],
    [8805, 2, 12],
    [9410, 2, 11],
    [9576, 3, 15],
    [9625, 1, 8],
    [9738, 4, 16],
    [10007, 3, 14],
    [10114, 3, 13],
    [10365, 2, 10],
    [10512, 3, 12],
    [10668, 4, 15],
    [10853, 5, 16],
    [11099, 4, 14],
    [11120, 1, 7],
    [11206, 4, 13],
    [11257, 3, 11],
    [11604, 4, 12],
    [11630, 2, 9],
    [11760, 5, 15],
    [11983, 6, 16],
    [12191, 5, 14],
    [12298, 5, 13],
    [12343, 3, 10],
    [12349, 4, 11],
    [12696, 5, 12],
    [12852, 6, 15],
    [12995, 2, 8],
    [13128, 7, 16],
    [13283, 6, 14],
    [13390, 6, 13],
    [13413, 4, 10],
    [13441, 5, 11],
    [13578, 3, 9],
    [13788, 6, 12],
    [13944, 7, 15],
    [14288, 8, 16],
    [14375, 7, 14],
    [14468, 5, 10],
    [14482, 7, 13],
    [14533, 6, 11],
    [14583, 4, 9],
    [14880, 7, 12],
    [15036, 8, 15],
    [15463, 9, 16],
    [15467, 8, 14],
    [15508, 6, 10],
    [15573, 5, 9],
    [15574, 8, 13],
    [15623, 7, 11],
    [15972, 8, 12],
    [16128, 9, 15],
    [16533, 7, 10],
    [16559, 9, 14],
    [16653, 10, 16],
    [16666, 9, 13],
    [16698, 8, 11],
    [17064, 9, 12],
    [17223, 10, 15],
    [17543, 8, 10],
    [17651, 10, 14],
    [17758, 9, 11],
    [17758, 10, 13],
    [18156, 10, 12],
    [18333, 11, 15],
    [18538, 9, 10],
    [18743, 11, 14],
    [18803, 10, 11],
    [18850, 11, 13],
    [19248, 11, 12],
    [19458, 12, 15],
    [19833, 11, 11],
    [19835, 12, 14],
    [19942, 12, 13],
    [20328, 12, 12],
    [20598, 13, 15],
    [20848, 12, 11],
    [20927, 13, 14],
    [21034, 13, 13],
    [21393, 13, 12],
    [21753, 14, 15],
    [21848, 13, 11],
    [22019, 14, 14],
    [22126, 14, 13],
    [22443, 14, 12],
    [22923, 15, 15],
    [23111, 15, 14],
    [23218, 15, 13],
    [23478, 15, 12],
    [24203, 16, 14],
    [24303, 16, 13],
    [24498, 16, 12],
    [25373, 17, 13],
    [25503, 17, 12],
    [26493, 18, 12],
];

#[derive(Clone, Copy)]
pub struct PacBeat {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub jumps: u8,
    pub side: bool,
    pub up: bool,
}

#[derive(Clone, Copy)]
struct DriftPlan {
    hold: u32,
    drift: u32,
    jump: JumpProgram,
}

impl DriftPlan {
    fn from_seed([hold, drift, forward, back]: [i8; 4]) -> Self {
        Self {
            hold: hold as u32,
            drift: drift as u32,
            jump: JumpProgram {
                forward: forward as u32,
                back: back as u32,
            },
        }
    }
}

#[derive(Clone, Copy)]
struct JumpProgram {
    forward: u32,
    back: u32,
}

impl JumpProgram {
    fn stick(self, row: u32) -> f32 {
        if row < self.forward {
            1.0
        } else if row < self.forward + self.back {
            -1.0
        } else {
            0.0
        }
    }

    fn project(self, flight: &PacFlight, mut at: Point) -> Option<Point> {
        let mut vx = 0.0;
        for row in 0..JUMP_ROWS {
            let stick = if row + 1 == JUMP_ROWS {
                0.0
            } else {
                self.stick(row)
            };
            vx = if row == 0 {
                0.9 + ACCEL
            } else {
                drift_x(vx, stick)
            };
            if !flight.advance(&mut at, [vx, JUMP_SPEED - GRAVITY * row as f32], JUMP_BODY) {
                return None;
            }
        }
        at[1] -= 0.37;
        (vx.abs() <= 0.12 && flight.reaches_second_bounce(at)).then_some(at)
    }
}

#[derive(Clone, Copy)]
enum Phase {
    Approach(Option<u32>),
    Pellet(u32),
    Jump(u32),
    Bounce(u32),
}

pub struct PacFlight {
    hull: Outline,
    near: Point,
    far: Point,
    bottom: f32,
    direction: f32,
    plan: DriftPlan,
    phase: Phase,
}

fn drift_x(vx: f32, stick: f32) -> f32 {
    if stick == 0.0 {
        vx.signum() * (vx.abs() - BRAKE).max(0.0)
    } else {
        (vx + ACCEL * stick).clamp(-AIR_SPEED, AIR_SPEED)
    }
}

impl PacFlight {
    pub fn start(facts: &RouteFacts<'_>, velocity: Point, facing: f32) -> Option<Self> {
        if !super::is_crossing_stage(facts.stage)
            || facts.fighter != KIND
            || !(0.0 < facts.body_clearance && facts.body_clearance <= JUMP_BODY)
            || !(-TERMINAL..=0.0).contains(&velocity[1])
            || !(-0.005..=0.005).contains(&velocity[0])
            || !facts.position.into_iter().all(f32::is_finite)
        {
            return None;
        }
        facts.crossing_band()?;
        let [left, right] = facts.ledges;
        let [near, far] = if facts.position[0] < left[0] {
            [left, right]
        } else {
            [right, left]
        };
        let direction = (far[0] - near[0]).signum();
        if !(0.99..=1.01).contains(&(facing * direction))
            || ((right[0] - left[0]) * 0.5 - 79.99).abs() > 0.5
            || (right[1] - left[1]).abs() > 0.5
            || (facts.blast[3] - near[1] + 140.0).abs() > 1.0
        {
            return None;
        }
        let [x, y] = facts.position;
        let outside = (near[0] - x) * direction;
        let seed = SEEDS.iter().find(|seed| {
            let [out, height, ..] = **seed;
            let [out, height] = [out, height].map(f32::from);
            (out - 2.0..out + 2.0).contains(&outside)
                && (height - 2.0..height + 2.0).contains(&(y - near[1]))
        })?;
        let floor = facts
            .outline
            .iter()
            .flatten()
            .map(|p| p[1])
            .reduce(f32::min)?;
        if (floor - near[1] + 40.1).abs() > 1.0 {
            return None;
        }
        let mut hull = Outline::new();
        for &line in facts.outline {
            hull.add(line);
        }
        let [_, _, hold, drift, forward, back, fall_hold, fall_drift, fall_forward, fall_back] =
            *seed;
        let mut plans = [
            DriftPlan::from_seed([hold, drift, forward, back]),
            DriftPlan::from_seed([fall_hold, fall_drift, fall_forward, fall_back]),
        ];
        if velocity[1] < -TERMINAL * 0.5 {
            plans.swap(0, 1);
        }
        let mut flight = Self {
            hull,
            near,
            far,
            direction,
            bottom: facts.blast[3],
            plan: plans[0],
            phase: Phase::Approach(None),
        };
        for seed in plans {
            for adjust in [0, -2, 2, -4, 4, -6, 6, -8, 8] {
                let Some(hold) = seed.hold.checked_add_signed(adjust) else {
                    continue;
                };
                let plan = DriftPlan { hold, ..seed };
                let Some((press, speed)) = flight.approach(facts.position, velocity, plan) else {
                    continue;
                };
                if let Some(launch) = flight.pellet_to_jump(press, speed, plan.drift) {
                    if let Some(jump) = flight.jump_program(launch, plan.jump) {
                        flight.plan = DriftPlan { jump, ..plan };
                        return Some(flight);
                    }
                }
            }
        }
        None
    }

    pub fn exit_point(&self) -> Point {
        [self.far[0] + self.direction * 1.78, self.far[1] - 11.125]
    }

    pub fn crossing_frames(&self) -> u16 {
        (self.plan.hold + self.plan.drift + END_START + END_ROWS + JUMP_ROWS + 80) as u16
    }

    fn advance(&self, at: &mut Point, [dx, dy]: Point, height: f32) -> bool {
        let next = [at[0] + dx * self.direction, at[1] + dy];
        let clear = aerial_clear(
            self.hull.segments(),
            [*at, next],
            self.bottom + 4.0,
            height,
            BODY_HALF,
        );
        *at = next;
        clear
    }

    fn near_ledge_grab(&self, at: Point) -> bool {
        let outside = (self.near[0] - at[0]) * self.direction;
        let below = self.near[1] - at[1];
        (-0.5..=14.5).contains(&outside) && (7.5..=19.5).contains(&below)
    }

    fn approach(
        &self,
        mut at: Point,
        [world_vx, mut vy]: Point,
        plan: DriftPlan,
    ) -> Option<(Point, Point)> {
        let mut vx = world_vx * self.direction;
        for row in 0..plan.hold + plan.drift {
            vx = drift_x(vx, f32::from(u8::from(row >= plan.hold)));
            vy = (vy - GRAVITY).max(-TERMINAL);
            if !self.advance(&mut at, [vx, vy], 9.93) || self.near_ledge_grab(at) {
                return None;
            }
        }
        Some((at, [vx, vy]))
    }

    fn pellet_to_jump(&self, mut at: Point, [vx, vy]: Point, drift: u32) -> Option<Point> {
        let (trail, class) = match drift {
            0 => (&TRAIL_AT_REST, 0.08),
            11 => (&TRAIL_11, 0.96),
            13.. => (&TRAIL_FAST, AIR_SPEED),
            _ => return None,
        };
        let vx = drift_x(vx, 1.0);
        let vy = (vy - GRAVITY).max(-TERMINAL);
        if (vx - class).abs() > 0.005
            || !self.advance(&mut at, [vx, vy], 9.93)
            || self.near_ledge_grab(at)
        {
            return None;
        }
        let [mut vx, mut vy] = [0.5 * vx, 0.7 * vy];
        for _ in 0..CHARGE_ROWS {
            vx = drift_x(vx, 0.0);
            vy = (vy - GRAVITY).max(-1.08);
            if !self.advance(&mut at, [vx, vy], 9.93) {
                return None;
            }
        }
        for _ in 0..STILL_ROWS {
            vx = drift_x(vx, 0.0);
            if !self.advance(&mut at, [vx, 0.0], 9.93) {
                return None;
            }
        }
        for &step in trail.iter().chain(&DASH_STEPS) {
            if !self.advance(&mut at, step, PELLET_BODY) {
                return None;
            }
        }
        self.finish_dash(at, DASH_STEPS[DASH_STEPS.len() - 1], 0)
    }

    fn finish_dash(&self, mut at: Point, [mut vx, mut vy]: Point, age: u32) -> Option<Point> {
        if age == 0 {
            vx *= 0.4;
            vy *= 0.6;
        }
        for _ in age..END_ROWS {
            vx = drift_x(vx, 0.0);
            vy = (vy - GRAVITY).max(-TERMINAL);
            if !self.advance(&mut at, [vx, vy], 9.93) {
                return None;
            }
        }
        Some(at)
    }

    fn reaches_second_bounce(&self, at: Point) -> bool {
        let outside = (at[0] - self.far[0]) * self.direction;
        let depth = if outside <= 7.0 { 15.77 } else { 14.6 };
        let catch_y = self.far[1] - depth;
        (5.0..=7.5).contains(&outside)
            && at[1] > self.bottom + 4.0
            && at[1] + 64.34 >= catch_y + 2.0
            && at[1] + 42.6 < catch_y
    }

    fn jump_program(&self, at: Point, preferred: JumpProgram) -> Option<JumpProgram> {
        if preferred.project(self, at).is_some() {
            return Some(preferred);
        }
        let wanted = (self.far[0] - at[0]) * self.direction;
        let minimum = (wanted + 5.0) * 1000.0;
        let maximum = (wanted + 7.5) * 1000.0;
        let first = JUMP_PROGRAMS.partition_point(|p| f32::from(p[0]) < minimum);
        JUMP_PROGRAMS[first..]
            .iter()
            .take_while(|p| f32::from(p[0]) <= maximum)
            .find_map(|p| {
                let candidate = JumpProgram {
                    forward: u32::from(p[1]),
                    back: u32::from(p[2]),
                };
                candidate.project(self, at).map(|_| candidate)
            })
    }

    pub fn step(&mut self, beat: PacBeat) -> Option<Correction> {
        if !beat
            .position
            .into_iter()
            .chain(beat.velocity)
            .all(f32::is_finite)
            || beat.position[1] <= self.bottom + 4.0
        {
            return None;
        }
        let [vx, vy] = beat.velocity;
        let forward = [self.direction, 0.0];
        match self.phase {
            Phase::Approach(first) => {
                if beat.status != FALL || beat.jumps != 1 || !(-TERMINAL..=0.0).contains(&vy) {
                    return None;
                }
                let first = first.unwrap_or(beat.frame);
                let age = beat.frame.saturating_sub(first);
                let press = self.plan.hold + self.plan.drift;
                if age < press {
                    self.phase = Phase::Approach(Some(first));
                    return Some(pad(
                        if age < self.plan.hold {
                            [0.0; 2]
                        } else {
                            forward
                        },
                        0,
                    ));
                }
                if age != press || !beat.side {
                    return None;
                }
                let launch =
                    self.pellet_to_jump(beat.position, [vx * self.direction, vy], self.plan.drift)?;
                self.plan.jump = self.jump_program(launch, self.plan.jump)?;
                self.phase = Phase::Pellet(beat.frame);
                Some(pad(forward, SPECIAL))
            }
            Phase::Pellet(first) => {
                let age = beat.frame.saturating_sub(first);
                match beat.status {
                    FALL if age <= 1 => Some(pad(forward, SPECIAL)),
                    CHARGE | HOLD if age <= CHARGE_ROWS => {
                        Some(pad(forward, if age < CHARGE_ROWS { SPECIAL } else { 0 }))
                    }
                    STILL | TRAIL | DASH if age < END_START => Some(pad(forward, 0)),
                    END if (END_START..END_START + END_ROWS).contains(&age) => {
                        let elapsed = age - END_START;
                        if elapsed < END_ROWS - 3 {
                            return Some(pad([0.0; 2], 0));
                        }
                        if beat.jumps != 1 {
                            return None;
                        }
                        let launch =
                            self.finish_dash(beat.position, [vx * self.direction, vy], elapsed)?;
                        self.plan.jump = self.jump_program(launch, self.plan.jump)?;
                        let buttons = if elapsed == END_ROWS - 2 { 0 } else { JUMP };
                        Some(pad(forward, buttons))
                    }
                    AIR_JUMP if age == END_START + END_ROWS && beat.jumps == 0 => {
                        self.plan.jump = self.jump_program(beat.position, self.plan.jump)?;
                        self.phase = Phase::Jump(beat.frame);
                        Some(pad(forward, 0))
                    }
                    _ => None,
                }
            }
            Phase::Jump(first) => {
                let age = beat.frame.saturating_sub(first);
                if beat.status != AIR_JUMP || beat.jumps != 0 {
                    return None;
                }
                if age < JUMP_ROWS - 1 {
                    return Some(pad([self.plan.jump.stick(age) * self.direction, 0.0], 0));
                }
                let next = [
                    beat.position[0] + drift_x(vx, 0.0),
                    beat.position[1] + vy - GRAVITY - 0.37,
                ];
                if age != JUMP_ROWS - 1
                    || !beat.up
                    || vx.abs() > 0.12
                    || !self.reaches_second_bounce(next)
                {
                    return None;
                }
                self.phase = Phase::Bounce(beat.frame);
                Some(pad([0.0, 1.0], SPECIAL))
            }
            Phase::Bounce(first) => {
                let age = beat.frame.saturating_sub(first);
                if age <= 1 && matches!(beat.status, AIR_JUMP | PAC_JUMP) {
                    return Some(pad([0.0, 1.0], SPECIAL));
                }
                (age <= 85 && matches!(beat.status, PAC_JUMP | RISE | BOUNCE_FALL | SPECIAL_FALL))
                    .then(|| pad([0.0; 2], 0))
            }
        }
    }
}

#[cfg(test)]
lib_unit_tests! {
    mod tests {
        use super::*;
        use crate::model::edge_juggle::perfect::{niche::pad, Segment, JUMP, SPECIAL};

        const NEAR: Point = [-79.986, 0.0];
        const FAR: Point = [79.989, 0.0];
        const HULL: [Segment; 8] = [
            [NEAR, [-79.986, -8.319]],
            [[-79.986, -8.319], [-64.395, -25.111]],
            [[-64.395, -25.111], [-41.396, -40.099]],
            [[-41.396, -40.099], [41.416, -40.118]],
            [[41.416, -40.118], [64.387, -25.103]],
            [[64.387, -25.103], [79.989, -8.319]],
            [[79.989, -8.319], FAR],
            [NEAR, FAR],
        ];

        fn route(position: Point, stage: (i32, u32)) -> RouteFacts<'static> {
            RouteFacts {
                fighter: KIND,
                stage,
                position,
                player: [-75.0, 0.0],
                player_reach: 5.0,
                ledges: [NEAR, FAR],
                blast: [-230.0, 230.0, 185.0, -140.0],
                outline: &HULL,
                body_clearance: 12.0,
                ledge_distance: 12.0,
                fall_budget: 1.35,
                reach: [160.0, 100.0],
            }
        }

        #[test]
        fn pacman_rejects_the_outside_start_and_unproven_stage_forms() {
            let position = [-105.986, 14.0];
            for stage in [(0, 0), (3, 0), (311, 0)] {
                assert!(PacFlight::start(&route(position, stage), [0.0; 2], 1.0).is_some());
            }
            for stage in [(311, 2), (3, 1), (0, 2), (13, 0), (89, 0)] {
                assert!(PacFlight::start(&route(position, stage), [0.0; 2], 1.0).is_none());
            }
            assert!(PacFlight::start(&route([-169.986, 18.0], (3, 0)), [0.0; 2], 1.0).is_none());
            assert!(PacFlight::start(&route(position, (3, 0)), [0.05, 0.0], 1.0).is_none());
            assert!(PacFlight::start(&route(position, (3, 0)), [0.0, f32::NAN], 1.0).is_none());
            assert!(PacFlight::start(&route(position, (3, 0)), [0.0; 2], -1.0).is_none());
        }

        #[test]
        fn pacman_checks_the_38_traced_region_cells_in_both_directions() {
            let mut mirrored = 0;
            for [out, height, ..] in SEEDS {
                for (edge, facing) in [(NEAR, 1.0), (FAR, -1.0)] {
                    let center = [edge[0] - facing * f32::from(out), f32::from(height)];
                    for vy in [0.0, -TERMINAL * 0.5, -TERMINAL] {
                        let flight = PacFlight::start(&route(center, (3, 0)), [0.0, vy], facing);
                        assert!(flight.is_some(), "out {out}, height {height}, facing {facing}, vy {vy}");
                        mirrored += 1;
                    }
                }
            }
            assert_eq!(mirrored, 228);
        }

        #[test]
        fn pacman_replays_pellet_buffered_jump_and_pacjump_from_native_frames() {
            // fg2-f-yoga-5, pm-x-26-out-y-plus-14, attempt 5. Each input drives the next row.
            let mut flight = PacFlight::start(&route([-105.986, 14.0], (3, 0)), [0.0; 2], 1.0)
                .expect("the stock reference has a route from this start");
            let rows = [
                (0, 0xe, [-105.986, 14.0], [0.0, 0.0], 1, [0.0, 0.0], 0),
                (18, 0xe, [-105.986, 1.688], [0.0, -1.296], 1, [1.0, 0.0], 0),
                (40, 0xe, [-88.878, -28.012], [1.092, -1.35], 1, [1.0, 0.0], SPECIAL),
                (41, 0x1dd, [-87.786, -29.362], [1.092, -1.35], 1, [1.0, 0.0], SPECIAL),
                (70, 0x1e6, [-78.477, -60.6191], [0.111, -1.08], 1, [1.0, 0.0], 0),
                (75, 0x1e8, [-78.1469, -61.6991], [0.036, 0.0], 1, [1.0, 0.0], 0),
                (92, 0x1ea, [-36.1396, -60.5192], [6.4318, 0.2141], 1, [1.0, 0.0], 0),
                (120, 0x1e9, [38.4521, -65.3935], [2.2615, -0.4041], 1, [0.0, 0.0], 0),
                (152, 0x1e9, [59.4789, -100.6202], [0.4246, -1.35], 1, [1.0, 0.0], JUMP),
                (153, 0x1e9, [59.8885, -101.9702], [0.4096, -1.35], 1, [1.0, 0.0], 0),
                (154, 0x1e9, [60.2831, -103.3202], [0.3946, -1.35], 1, [1.0, 0.0], JUMP),
                (155, 0xc, [60.6695, -104.6702], [0.3864, -1.35], 0, [1.0, 0.0], 0),
                (172, 0xc, [79.0895, -77.4032], [1.092, 1.0279], 0, [-1.0, 0.0], 0),
                (185, 0xc, [86.0055, -70.592], [0.052, 0.0919], 0, [0.0, 1.0], SPECIAL),
                (186, 0x1de, [86.0425, -70.942], [0.037, -0.35], 0, [0.0, 1.0], SPECIAL),
                (239, 0x10, [86.0425, -61.1522], [0.0, -1.35], 0, [0.0, 0.0], 0),
                (244, 0x1de, [86.0425, -70.942], [0.0, -4.3898], 0, [0.0, 0.0], 0),
            ];
            for (frame, status, position, velocity, jumps, stick, buttons) in rows {
                let beat = PacBeat { frame, status, position, velocity, jumps, side: status == 0xe, up: status == 0xc };
                assert_eq!(flight.step(beat), Some(pad(stick, buttons)), "frame {frame}");
            }
        }
    }
}
