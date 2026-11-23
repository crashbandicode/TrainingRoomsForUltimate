//! Sora's traced Aerial Sweep and three-dash Sonic Blade crossing.

use super::{Correction, Outline, Point, RouteFacts, Segment, JUMP, SPECIAL};

// Model and geometry from reference-crossings/MODEL.md section 5 (S-X2).
pub const KIND: i32 = 0x5d;
pub const FALL: i32 = 0xe;
pub const FALL_AERIAL: i32 = 0xf;
const JUMP_AERIAL: i32 = 0xc;
const UP_SWEEP: i32 = 0x1de;
const BLADE_START: i32 = 0x1dd;
const DASH: i32 = 0x1eb;
const DASH_LINK: i32 = 0x1ec;
const DASH_AIM: i32 = 0x1ed;
const BLADE_END: i32 = 0x1ee;
const AIR_SPEED: f32 = 0.96;
const ACCEL: f32 = 0.096;
const G: f32 = 0.064;
const TERMINAL: f32 = 1.44;
const LOWEST_CLEAR: f32 = 8.0;
const BODY: f32 = 8.43;
const JUMP_CLEARANCE: f32 = 17.84;
const MAX_WAIT: u32 = 90;
const JUMP_THINKS: u32 = 72;
const FALL_TO_SWEEP: [u32; 6] = [11, 10, 12, 9, 13, 14];
const EARLY_JUMP_FALL: [u32; 8] = [16, 18, 20, 22, 24, 26, 28, 30];
const LOCK_STEP: f32 = 2.647;
const LOCK_THINKS: f32 = 12.0;

const S_X2_RING: [u32; 44] = [
    0x00000000, 0x0003ffc0, 0x0001ffc0, 0x0001ffc0, 0x0000ffc0, 0x0000ffc0, 0x00007fc0, 0x00003fc0,
    0x00003fc0, 0x00001fc0, 0x00000fc0, 0x000007c0, 0x000007c0, 0x000003c0, 0x000003c0, 0x000001c0,
    0x000000e0, 0x000000fc, 0x0000007c, 0x0000003c, 0x0000001c, 0x0000001c, 0x0000000c, 0x0000000c,
    0x00000004, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000,
];
const JUMP_HEIGHT: [f32; JUMP_THINKS as usize + 1] = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.476, 1.447, 2.706, 4.167, 5.806, 7.597,
    9.521, 11.552, 13.668, 15.844, 18.058, 20.286, 22.506, 24.693, 26.825, 28.877, 30.828, 32.652,
    34.328, 35.917, 37.442, 38.822, 40.058, 41.209, 42.276, 43.261, 44.168, 45.001, 45.761, 46.452,
    47.075, 47.635, 48.134, 48.574, 48.96, 49.292, 49.575, 49.811, 50.002, 50.153, 50.265, 50.342,
    50.386, 50.4, 50.366, 50.265, 50.093, 49.849, 49.532, 49.14, 48.67, 48.123, 47.494, 46.785,
    45.99, 45.11, 44.144, 43.088, 41.941, 40.702, 39.369, 37.939, 36.585,
];
const SWEEP: [Point; 50] = [
    [0.0, 0.0],
    [0.369, -0.189],
    [0.815, -0.422],
    [1.338, -0.701],
    [1.914, -1.024],
    [2.49, -1.392],
    [3.066, -1.805],
    [3.642, -2.262],
    [4.218, -2.765],
    [4.338, -0.909],
    [4.578, 0.907],
    [4.938, 2.684],
    [5.418, 4.42],
    [5.994, 6.116],
    [6.57, 7.772],
    [7.146, 9.388],
    [7.722, 10.965],
    [8.298, 12.501],
    [8.874, 13.997],
    [9.45, 15.453],
    [10.026, 16.869],
    [10.602, 18.245],
    [11.178, 19.582],
    [11.754, 20.878],
    [12.33, 22.134],
    [12.906, 23.35],
    [13.482, 24.526],
    [14.058, 25.662],
    [14.634, 26.758],
    [15.21, 27.815],
    [15.786, 28.831],
    [16.362, 29.807],
    [16.938, 30.743],
    [17.514, 31.639],
    [18.09, 32.495],
    [18.666, 33.312],
    [19.242, 34.088],
    [19.818, 34.824],
    [20.394, 35.52],
    [20.97, 36.176],
    [21.546, 36.792],
    [22.122, 37.369],
    [22.698, 37.905],
    [23.274, 38.401],
    [23.85, 38.857],
    [24.426, 39.273],
    [25.002, 39.65],
    [25.578, 39.986],
    [26.154, 40.282],
    [26.73, 40.538],
];

#[derive(Clone, Copy)]
pub struct SoraBeat {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub player_x: f32,
    pub jumps: u8,
    pub can_jump: bool,
    pub can_rise: bool,
    pub can_blade: bool,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct FlightTiming {
    glide: u32,
    jump: u32,
    sweep: u32,
    angle: BladeAngle,
    finish: FlightFinish,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum FlightFinish {
    Stage,
    Low,
}

impl FlightFinish {
    const fn live_margin(self) -> f32 {
        match self {
            Self::Stage => 1.0,
            Self::Low => 1.5,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum BladeAngle {
    A125,
    A130,
}

impl BladeAngle {
    const ALL: [Self; 2] = [Self::A130, Self::A125];

    const fn delta(self) -> Point {
        // Dash 2 plus its link to dash 3. Both were flown.
        match self {
            Self::A125 => [-18.34, 29.0],
            Self::A130 => [-20.72, 27.28],
        }
    }

    fn stick(self, inward: f32) -> Point {
        let [x, y] = match self {
            Self::A125 => [-69.0, 98.0],
            Self::A130 => [-77.0, 92.0],
        };
        [inward * x / 120.0, y / 120.0]
    }
}

#[derive(Clone, Copy)]
struct FlightSpace<'a> {
    outline: &'a [Segment],
    near: Point,
    far: Point,
    inward: f32,
    bottom: f32,
}

impl FlightSpace<'_> {
    fn ahead(self, position: Point, dx: f32, dy: f32) -> Point {
        [position[0] + dx * self.inward, position[1] + dy]
    }

    fn clear(self, from: Point, to: Point, head: f32) -> bool {
        // The measured model uses the fighter's vertical centerline for S-X2.
        super::aerial_clear(self.outline, [from, to], self.bottom, head, 0.0)
    }

    fn player_on_near_half(self, player_x: f32) -> bool {
        (player_x - self.near[0].midpoint(self.far[0])) * self.inward <= 0.0
    }

    fn next_fall(self, position: Point, vy: f32, vx: f32) -> Point {
        self.ahead(position, vx, (vy - G).max(-TERMINAL))
    }

    fn near_grab(self, point: Point) -> bool {
        (-16.0..=9.6).contains(&((point[0] - self.near[0]) * self.inward))
            && (-22.5..=-8.5).contains(&(point[1] - self.near[1]))
    }

    fn cast_slack(self, cast: Point, angle: BladeAngle, finish: FlightFinish) -> f32 {
        let [dx, dy] = sweep_and_blade(angle);
        let q = self.ahead(cast, dx, dy);
        let out = (q[0] - self.far[0]) * self.inward;
        let depth = self.far[1] - q[1];
        let distance = out.hypot(depth);
        if !distance.is_finite() || distance == 0.0 {
            return f32::NEG_INFINITY;
        }
        let end = LOCK_STEP * LOCK_THINKS * depth / distance - depth;
        let (min_end, max_end, max_distance) = match finish {
            FlightFinish::Stage => (-2.3, 4.0, 35.0),
            // Measured low catches include the lock dash's two cliff checks and its rise.
            // Keep 1.5 units inside the deepest measured end and longest lock.
            FlightFinish::Low => (-24.5, 0.7, 58.0),
        };
        (out - 5.5)
            .min(10.5 - out)
            .min(distance - 27.0)
            .min(max_distance - distance)
            .min(end - min_end)
            .min(max_end - end)
    }

    fn sweep_clear(self, cast: Point, angle: BladeAngle) -> bool {
        let [dx, dy] = SWEEP[49];
        let start = self.ahead(cast, dx, dy);
        let first = self.ahead(start, 6.43, -6.36);
        let second = self.ahead(first, 41.3, 0.0);
        let [dx, dy] = angle.delta();
        let third = self.ahead(second, dx, dy);
        SWEEP.windows(2).all(|pair| {
            let a = self.ahead(cast, pair[0][0], pair[0][1]);
            let b = self.ahead(cast, pair[1][0], pair[1][1]);
            self.clear(a, b, BODY)
        }) && self.clear(start, first, BODY)
            && self.clear(first, second, BODY)
            && self.clear(second, third, BODY)
    }

    fn jump_to_cast(self, press: Point, vy: f32, gap: u32) -> Option<Point> {
        let base = self.next_fall(press, vy, AIR_SPEED);
        let mut last = base;
        for (think, &height) in JUMP_HEIGHT.iter().enumerate().skip(1) {
            let p = self.ahead(base, AIR_SPEED * think as f32, height);
            if !self.clear(last, p, JUMP_CLEARANCE) {
                return None;
            }
            last = p;
        }
        let cast = self.ahead(last, AIR_SPEED * gap as f32, 0.02 - TERMINAL * gap as f32);
        (self.clear(press, base, BODY) && self.clear(last, cast, BODY)).then_some(cast)
    }

    fn fall_clear(self, start: Point, vy: f32, wait: u32, jump: u32) -> Option<Point> {
        let mut p = start;
        let (mut vx, mut vy) = (0.0_f32, vy);
        for n in 0..jump {
            if n >= wait {
                vx = (vx + ACCEL).min(AIR_SPEED);
            }
            let next = self.next_fall(p, vy, vx);
            if self.near_grab(next) || !self.clear(p, next, BODY) {
                return None;
            }
            vy = (vy - G).max(-TERMINAL);
            p = next;
        }
        Some(p)
    }
}

fn sweep_and_blade(angle: BladeAngle) -> Point {
    let [dx, dy] = SWEEP[49];
    let [sx, sy] = angle.delta();
    [dx + 6.43 + 41.3 + sx, dy - 6.36 + sy]
}

fn lost(vy: f32, frames: u32) -> f32 {
    let ramp = ((TERMINAL + vy) / G).floor().clamp(0.0, frames as f32);
    let capped = frames as f32 - ramp;
    (G * (ramp + 1.0) * 0.5 - vy) * ramp + TERMINAL * capped
}

fn solve(space: FlightSpace<'_>, start: Point, vy: f32) -> Option<FlightTiming> {
    find_flight(space, start, vy, &FALL_TO_SWEEP, FlightFinish::Stage)
        .or_else(|| find_flight(space, start, vy, &FALL_TO_SWEEP, FlightFinish::Low))
        .or_else(|| find_flight(space, start, vy, &EARLY_JUMP_FALL, FlightFinish::Low))
}

fn find_flight(
    space: FlightSpace<'_>,
    start: Point,
    vy: f32,
    fall_to_sweep: &[u32],
    finish: FlightFinish,
) -> Option<FlightTiming> {
    let span = (space.far[0] - start[0]) * space.inward;
    let mut best = None;
    for angle in BladeAngle::ALL {
        let [dx, _] = sweep_and_blade(angle);
        for &gap in fall_to_sweep {
            let lo = ((span + 5.5 - dx + 4.32) / AIR_SPEED - (JUMP_THINKS + gap) as f32)
                .ceil()
                .max(11.0) as u32;
            let hi = ((span + 10.5 - dx + 4.32) / AIR_SPEED - (JUMP_THINKS + gap) as f32)
                .floor()
                .clamp(0.0, 300.0) as u32;
            for glide in lo..=hi {
                let drift = AIR_SPEED * (glide + JUMP_THINKS + gap) as f32 - 4.32;
                let max_wait = if finish == FlightFinish::Stage { MAX_WAIT } else { 0 };
                for wait in 0..=max_wait {
                    let dy = -lost(vy, wait + glide) + JUMP_HEIGHT[JUMP_THINKS as usize] + 0.02
                        - TERMINAL * gap as f32;
                    let cast = space.ahead(start, drift, dy);
                    let slack = space.cast_slack(cast, angle, finish);
                    let score = slack - 0.01 * wait as f32;
                    if slack < 1.5 || best.as_ref().is_some_and(|(old, _)| score <= *old) {
                        continue;
                    }
                    let jump = wait + glide - 1;
                    let Some(press) = space.fall_clear(start, vy, wait, jump) else {
                        continue;
                    };
                    let press_speed = (vy - G * jump as f32).max(-TERMINAL);
                    let Some(cast) = space.jump_to_cast(press, press_speed, gap) else {
                        continue;
                    };
                    if space.cast_slack(cast, angle, finish) >= 1.5 && space.sweep_clear(cast, angle) {
                        best = Some((
                            score,
                            FlightTiming {
                                glide: wait,
                                jump,
                                sweep: jump + JUMP_THINKS + gap,
                                angle,
                                finish,
                            },
                        ));
                    }
                }
            }
        }
    }
    best.map(|(_, timing)| timing)
}

pub struct SoraFlight {
    outline: Outline,
    ledges: [Point; 2],
    inward: f32,
    bottom: f32,
    timing: FlightTiming,
    origin: Option<u32>,
    sweep_start: Option<u32>,
}

impl SoraFlight {
    pub fn start(facts: &RouteFacts<'_>, beat: SoraBeat) -> Option<Self> {
        let [left, right] = facts.ledges;
        let from_left = beat.position[0] < left[0].midpoint(right[0]);
        let [near, far] = if from_left {
            [left, right]
        } else {
            [right, left]
        };
        let inward = if from_left { 1.0 } else { -1.0 };
        let bottom = facts.blast[3] + LOWEST_CLEAR;
        let outside = (near[0] - beat.position[0]) * inward;
        let height = beat.position[1] - near[1];
        let space = FlightSpace {
            outline: facts.outline,
            near,
            far,
            inward,
            bottom,
        };
        if !matches!(facts.stage, (0 | 3 | 311, 0))
            || facts.fighter != KIND
            || !(159.0..=161.0).contains(&(right[0] - left[0]))
            || !(139.0..=141.0).contains(&(near[1] - facts.blast[3]))
            || (near[1] - far[1]).abs() > 0.25
            || facts.outline.is_empty()
            || !space.player_on_near_half(beat.player_x)
            || !super::region::contains(&S_X2_RING, outside, height)
            || beat.status != FALL
            || !(-0.01..=0.01).contains(&beat.velocity[0])
            || !(-TERMINAL..=0.0).contains(&beat.velocity[1])
            || beat.jumps != 1
            || !beat.can_blade
            || !beat.can_rise
        {
            return None;
        }
        let timing = solve(space, beat.position, beat.velocity[1])?;
        let mut outline = Outline::new();
        for &line in facts.outline {
            outline.add(line);
        }
        Some(Self {
            outline,
            ledges: [near, far],
            inward,
            bottom,
            timing,
            origin: None,
            sweep_start: None,
        })
    }

    pub fn exit_point(&self) -> Point {
        self.ledges[1]
    }

    pub fn crossing_frames(&self) -> u16 {
        // Sweep press to first grounded think was 169 or 170 frames.
        self.timing.sweep as u16 + 170
    }

    fn space(&self) -> FlightSpace<'_> {
        let [near, far] = self.ledges;
        FlightSpace {
            outline: self.outline.segments(),
            near,
            far,
            inward: self.inward,
            bottom: self.bottom,
        }
    }

    pub fn step(&mut self, beat: SoraBeat) -> Option<Correction> {
        if beat
            .position
            .into_iter()
            .chain(beat.velocity)
            .any(|x| !x.is_finite())
        {
            return None;
        }
        let origin = *self.origin.get_or_insert(beat.frame);
        let frame = beat.frame.checked_sub(origin)?;
        let forward = [self.inward, 0.0];
        let drop = self.space();
        let timing = self.timing;
        if frame <= timing.jump {
            if beat.status != FALL || !drop.player_on_near_half(beat.player_x) || beat.jumps != 1 {
                return None;
            }
            if frame < timing.glide {
                return Some(pad([0.0; 2], 0));
            }
            if frame < timing.jump {
                return Some(pad(forward, 0));
            }
            let gap = timing.sweep - timing.jump - JUMP_THINKS;
            let cast = drop.jump_to_cast(beat.position, beat.velocity[1], gap)?;
            return (beat.can_jump
                && beat.can_rise
                && beat.can_blade
                && (beat.velocity[0] * self.inward - AIR_SPEED).abs() <= 0.12
                && drop.cast_slack(cast, timing.angle, timing.finish) >= timing.finish.live_margin()
                && drop.sweep_clear(cast, timing.angle))
            .then_some(pad(forward, JUMP));
        }

        if frame <= timing.sweep {
            if frame < timing.sweep {
                return matches!(beat.status, FALL | FALL_AERIAL | JUMP_AERIAL)
                    .then_some(pad(forward, 0));
            }
            if !matches!(beat.status, FALL | FALL_AERIAL)
                || !beat.can_rise
                || !beat.can_blade
                || (beat.velocity[0] * self.inward - AIR_SPEED).abs() > 0.12
                || (beat.velocity[1] + TERMINAL).abs() > 0.12
            {
                return None;
            }
            let cast = drop.next_fall(beat.position, beat.velocity[1], AIR_SPEED);
            let angle = BladeAngle::ALL
                .into_iter()
                .filter_map(|angle| {
                    let slack = drop.cast_slack(cast, angle, timing.finish);
                    (slack >= timing.finish.live_margin() && drop.sweep_clear(cast, angle))
                        .then_some((slack, angle))
                })
                .max_by(|a, b| a.0.total_cmp(&b.0))?
                .1;
            self.timing.angle = angle;
            return Some(pad([0.0, 1.0], SPECIAL));
        }

        if frame == timing.sweep + 1 && beat.status == UP_SWEEP {
            self.sweep_start = Some(beat.frame);
            return Some(pad([0.0, 1.0], SPECIAL));
        }
        let age = beat.frame.checked_sub(self.sweep_start?)?;
        let stick = timing.angle.stick(self.inward);
        match (age, beat.status) {
            (1..=47, UP_SWEEP) => Some(pad(forward, 0)),
            (48, UP_SWEEP) | (49..=51, BLADE_START) => Some(pad(forward, SPECIAL)),
            (52..=76, BLADE_START | DASH | DASH_LINK) => Some(pad(forward, 0)),
            (77..=104, DASH_LINK | DASH_AIM | DASH) => Some(blade_pad(stick, age - 77)),
            (105..=130, DASH_LINK | DASH_AIM | DASH) | (131..=161, BLADE_END) => {
                Some(blade_pad(stick, age - 105))
            }
            (162..=179, BLADE_END) => Some(pad([0.0; 2], 0)),
            _ => None,
        }
    }
}

fn blade_pad(stick: Point, age: u32) -> Correction {
    let press = if age.is_multiple_of(4) { SPECIAL } else { 0 };
    pad(stick, press)
}

fn pad(stick: Point, press: u32) -> Correction {
    Correction {
        stick: Some(stick),
        press,
        release: !press,
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::model::edge_juggle::perfect::{Segment, JUMP, SPECIAL};

    const RING: [Segment; 8] = [
        [[-79.99, 0.0], [79.99, 0.0]],
        [[-79.99, 0.0], [-79.99, -8.319]],
        [[-79.99, -8.319], [-64.395, -25.111]],
        [[-64.395, -25.111], [-41.396, -40.099]],
        [[-41.396, -40.099], [41.416, -40.118]],
        [[41.416, -40.118], [64.387, -25.103]],
        [[64.387, -25.103], [79.99, -8.319]],
        [[79.99, -8.319], [79.99, 0.0]],
    ];

    fn facts(stage: (i32, u32), position: Point, player: Point) -> RouteFacts<'static> {
        RouteFacts {
            fighter: 0x5d,
            stage,
            position,
            player,
            player_reach: 15.0,
            ledges: [[-79.99, 0.0], [79.99, 0.0]],
            blast: [-200.0, 200.0, 140.0, -140.0],
            outline: &RING,
            body_clearance: 7.43,
            ledge_distance: 0.0,
            fall_budget: 100.0,
            reach: [0.0; 2],
        }
    }

    fn sample(position: Point) -> SoraBeat {
        SoraBeat {
            frame: 0,
            status: FALL,
            position,
            velocity: [0.0; 2],
            player_x: 0.0,
            jumps: 1,
            can_jump: true,
            can_rise: true,
            can_blade: true,
        }
    }

    #[test]
    fn sora_lower_release_planes_keep_the_traced_jump_and_sweep_schedule() {
        let position = [-89.99, -30.0];
        let beat = sample(position);
        for stage in [(0, 0), (3, 0), (311, 0)] {
            let flight = SoraFlight::start(&facts(stage, position, [beat.player_x, 0.0]), beat)
                .expect("the d30 island was physically flown");
            assert_eq!(
                flight.timing,
                FlightTiming {
                    glide: 17,
                    jump: 67,
                    sweep: 150,
                    angle: BladeAngle::A130,
                    finish: FlightFinish::Stage,
                }
            );
        }
    }

    #[test]
    fn sora_admits_the_widened_default_and_the_measured_lower_far_catch() {
        for (outside, height, vy) in [
            (20.0, -28.0, 0.0),
            (20.0, -28.0, -1.44),
            (10.0, -58.0, -1.44),
            (30.0, -30.0, -1.44),
            (70.0, 34.0, -1.44),
            (14.0, -46.0, -1.44),
        ] {
            for stage in [(3, 0), (0, 0), (311, 0)] {
                let position = [-79.99 - outside, height];
                let beat = SoraBeat {
                    velocity: [0.0, vy],
                    ..sample(position)
                };
                assert!(
                    SoraFlight::start(&facts(stage, position, [0.0; 2]), beat).is_some(),
                    "no S-X2 at {outside} out, {height} high, vy {vy}, on {stage:?}",
                );
            }
        }
    }

    #[test]
    fn sora_does_not_choose_an_unmeasured_end_or_a_far_side_lock_target() {
        let position = [-109.99, 2.0];
        let beat = sample(position);
        assert!(SoraFlight::start(&facts((3, 0), position, [beat.player_x, 0.0]), beat).is_some());
        for (stage, position, player, jumps, can_blade) in [
            ((1, 0), position, [0.0, 0.0], 1, true),
            ((3, 2), position, [0.0, 0.0], 1, true),
            ((3, 0), position, [1.0, 0.0], 1, true),
            ((3, 0), [-109.99, -75.0], [0.0, 0.0], 1, true),
            ((3, 0), position, [0.0, 0.0], 0, true),
            ((3, 0), position, [0.0, 0.0], 1, false),
        ] {
            assert!(SoraFlight::start(
                &facts(stage, position, player),
                SoraBeat {
                    position,
                    player_x: player[0],
                    jumps,
                    can_blade,
                    ..beat
                }
            )
            .is_none());
        }
    }

    #[test]
    fn sora_replays_the_native_sweep_and_blade_input_edges() {
        let start = [-89.99, -30.0];
        let seed = sample(start);
        let mut flight = SoraFlight::start(&facts((3, 0), start, [seed.player_x, 0.0]), seed).unwrap();
        let mut row = |frame, status, position, velocity| {
            flight
                .step(SoraBeat {
                    frame: 5 + frame,
                    status,
                    position,
                    velocity,
                    ..seed
                })
                .unwrap()
        };
        assert_eq!(row(0_u32, FALL, start, [0.0; 2]).stick, Some([0.0; 2]));
        assert_eq!(
            row(17, FALL, [-89.99, -40.176], [0.0, -1.088]).stick,
            Some([1.0, 0.0])
        );
        assert_eq!(row(67, FALL, [-46.31, -111.376], [0.96, -1.44]).press, JUMP);
        assert_eq!(
            row(150, 0xf, [33.37, -90.608], [0.96, -1.44]).press,
            SPECIAL
        );
        assert_eq!(
            row(151, UP_SWEEP, [34.33, -92.048], [0.96, -1.44]).stick,
            Some([0.0, 1.0])
        );
        assert_eq!(
            row(199, UP_SWEEP, [60.4719, -51.7663], [0.0; 2]).press,
            SPECIAL
        );
        assert_eq!(
            row(200, BLADE_START, [61.0479, -51.5101], [0.0; 2]).press,
            SPECIAL
        );
        assert_eq!(row(214, DASH, [67.4799, -57.8715], [0.0; 2]).press, 0);
        assert_eq!(
            row(228, DASH_LINK, [106.8599, -57.8715], [0.0; 2]).stick,
            Some([-77.0 / 120.0, 92.0 / 120.0])
        );
        assert_eq!(row(229, DASH_LINK, [107.8399, -57.8715], [0.0; 2]).press, 0);
        assert_eq!(row(242, DASH, [108.7799, -57.8715], [0.0; 2]).press, 0);
        assert_eq!(
            row(282, BLADE_END, [84.1994, 0.1214], [0.0; 2]).stick,
            Some([-77.0 / 120.0, 92.0 / 120.0])
        );
    }
    #[test]
    fn sora_region_cells_keep_a_clear_way_through_the_ring_from_either_side() {
        assert_eq!(
            S_X2_RING.iter().map(|row| row.count_ones()).sum::<u32>(),
            142
        );
        for (row, &bits) in S_X2_RING.iter().enumerate() {
            for col in (0..30).filter(|col| bits & (1 << col) != 0) {
                let height = 38.0 - row as f32 * 4.0;
                let outside = col as f32 * 4.0 + 2.0;
                for inward in [-1.0, 1.0] {
                    let position = [-(79.99 + outside) * inward, height];
                    for [dx, dy] in [
                        [0.0, 0.0],
                        [-1.99, 0.0],
                        [1.99, 0.0],
                        [0.0, -1.99],
                        [0.0, 1.99],
                    ] {
                        let position = [position[0] + dx, position[1] + dy];
                        for vy in [0.0, -0.72, -1.44] {
                            let beat = SoraBeat {
                                velocity: [0.0, vy],
                                ..sample(position)
                            };
                            assert!(SoraFlight::start(&facts((3, 0), position, [beat.player_x, 0.0]), beat).is_some(),
                                "no safe S-X2 schedule for cell ({row}, {col}) at {position:?}, vy {vy}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn sora_buffer_rows_and_post_blade_pads_match_the_exact_reference() {
        let start = [-89.99, -30.0];
        let seed = sample(start);
        let mut flight = SoraFlight::start(&facts((3, 0), start, [seed.player_x, 0.0]), seed).unwrap();
        flight.step(seed).unwrap();
        for (frame, status, position, velocity, expected) in [
            (150, 0xf, [33.3699, -90.6082], [0.96, -1.44], [2, 0, 120]),
            (151, 0x1de, [34.3179, -92.0482], [0.948, -1.44], [2, 0, 120]),
            (
                152,
                0x1de,
                [34.6871, -92.2370],
                [0.3692, -0.1888],
                [0, 120, 0],
            ),
            (228, 0x1ec, [106.8599, -57.8715], [1.32, 0.0], [2, -77, 92]),
            (236, 0x1ed, [108.7799, -57.8715], [0.0; 2], [2, -77, 92]),
            (
                256,
                0x1ec,
                [89.0186, -32.2620],
                [-0.6592, 1.1436],
                [2, -77, 92],
            ),
            (264, 0x1ed, [88.0598, -30.5986], [0.0; 2], [2, -77, 92]),
            (282, 0x1ee, [84.1994, 0.1214], [-0.1752, 2.56], [0, -77, 92]),
            (312, 0x1ee, [75.5396, 7.9214], [-0.308, -0.9], [2, -77, 92]),
            (313, 0x1ee, [75.2316, 6.9414], [-0.308, -0.98], [0, 0, 0]),
        ] {
            let edit = flight
                .step(SoraBeat {
                    frame,
                    status,
                    position,
                    velocity,
                    ..seed
                })
                .unwrap_or_else(|| {
                    panic!("lost measured pad at frame {frame}, status {status:#x}")
                });
            let [x, y] = edit
                .stick
                .unwrap()
                .map(|coordinate| (coordinate * 120.0).round() as i32);
            assert_eq!(
                [edit.press as i32, x, y],
                expected,
                "frame {frame}, status {status:#x}"
            );
        }
    }
    }
}
