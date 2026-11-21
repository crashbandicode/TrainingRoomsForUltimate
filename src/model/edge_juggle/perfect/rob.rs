//! Per-think burner steering for the measured R.O.B. crossing.

const HOVER_GRAVITY: f32 = 0.065;
const CATCH_Y: f32 = -24.0;

fn thrust(vy: f32) -> f32 {
    vy + if vy < 1.2 { 0.14 } else { 0.002 }
}

struct BurnerPilot {
    far: f32,
}

impl BurnerPilot {
    fn input(
        &self,
        [x, y]: super::Point,
        [vx, vy]: super::Point,
        ceiling: impl Fn(f32) -> Option<f32>,
    ) -> Option<bool> {
        if (self.far - 16.5..=self.far + 9.9).contains(&x) && vy > 0.0 && y + vy >= CATCH_Y {
            return None;
        }
        let thrust_vy = thrust(vy) - HOVER_GRAVITY;
        if x >= self.far - 6.0 {
            return Some(ceiling(x + vx).is_none_or(|under| y + thrust_vy <= under - 12.5));
        }
        let apex = y + thrust_vy + thrust_vy.max(0.0).powi(2) / (2.0 * HOVER_GRAVITY);
        let limit = [0.0, 2.0, 4.0, 6.0]
            .map(|distance| BurnerFlight::hover_line(x + distance, &ceiling))
            .into_iter()
            .fold(f32::INFINITY, f32::min);
        Some(
            apex <= limit - 0.5 && vy < 0.9 && (y + vy - HOVER_GRAVITY < limit - 6.0 || vy < -0.45),
        )
    }
}

#[derive(Clone, Copy)]
pub struct BurnerState {
    pub frame: u32,
    pub status: i32,
    pub position: super::Point,
    pub velocity: super::Point,
    pub full_fuel: bool,
    pub facing: f32,
    pub jumps: u8,
    pub can_start: bool,
}

use super::{
    aerial_clear, line_height, niche::pad, ordered_x, Correction, Outline, Point, RouteFacts,
    SPECIAL,
};

pub const KIND: i32 = 0x2d;
pub const ROW: u8 = 7;
const FALL: i32 = 0xe;
const TAKEOFF: i32 = 0x1de;
const HOVER: i32 = 0x1e3;
const NEAR_X: f32 = -79.986;
const HULL_HALF_WIDTH: f32 = 3.0;
const HEAD: f32 = 17.0;
const PRESS_Y: f32 = -99.0;
const GRAVITY: f32 = 0.09;
const FALL_SPEED: f32 = 1.6;
const AIR_ACCEL: f32 = 0.085;
const AIR_SPEED: f32 = 1.134;
const HOVER_BUDGET: u8 = 100;

// RB-H, the 188-cell region in reference-crossings/g1/robot-ring.txt.
const BURNER_REGION: [u32; 44] = [
    0, 0x1f_ffc0, 0x0f_ffc0, 0x07_ffc0, 0x07_ffc0, 0x03_ffc0, 0x01_ffc0, 0xffc0, 0xffc0, 0x7fc0,
    0x3fc0, 0x3fc0, 0x1fc0, 0xfc0, 0x7c0, 0x7c0, 0x3c0, 0x1e0, 0xfc, 0xfc, 0x7c, 0x7c, 0x3c, 0x1c,
    0xc, 0xc, 0x4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub fn full_fuel(fuel: f32) -> bool {
    // Native WorkModule float 0x4e reads 160.0 at a fresh takeoff.
    fuel == 160.0
}

fn falling(vy: f32) -> f32 {
    (vy - GRAVITY).max(-FALL_SPEED)
}

enum Phase {
    Descend,
    Takeoff(u32),
    Flying,
    Catch(u32),
}

pub struct BurnerFlight {
    outline: Outline,
    near: Point,
    far: Point,
    direction: f32,
    bottom: f32,
    pilot: BurnerPilot,
    phase: Phase,
    fuel_used: u8,
    last_hold: Option<u32>,
}

impl BurnerFlight {
    pub fn start(facts: &RouteFacts<'_>, state: BurnerState) -> Option<Self> {
        let [left, right] = facts.ledges;
        let [x, y] = state.position;
        let (near, far, direction) = if x < left[0] {
            (left, right, 1.0)
        } else {
            (right, left, -1.0)
        };
        if facts.fighter != KIND
            || !super::is_crossing_stage(facts.stage)
            || state.status != FALL
            || !state.full_fuel
            || state.facing != direction
            || state.jumps != 1
            || !state.can_start
            || state.velocity[0] != 0.0
            || !(-FALL_SPEED..=0.0).contains(&state.velocity[1])
            || !(0.0 < facts.body_clearance && facts.body_clearance <= HEAD)
            || !super::region::contains(&BURNER_REGION, (near[0] - x) * direction, y - near[1])
        {
            return None;
        }
        facts.crossing_band()?;
        let span = (far[0] - near[0]).abs();
        let bottom = facts.blast[3] - near[1];
        let flat = facts
            .outline
            .iter()
            .flatten()
            .map(|point| point[1])
            .reduce(f32::min)?;
        if (span - 159.975).abs() > 1.0
            || (far[1] - near[1]).abs() > 0.5
            || (flat - near[1] + 40.118).abs() > 1.0
            || (bottom + 140.0).abs() > 1.0
        {
            return None;
        }
        let mut flight = Self {
            outline: Outline::new(),
            near,
            far,
            direction,
            bottom,
            pilot: BurnerPilot { far: NEAR_X + span },
            phase: Phase::Descend,
            fuel_used: 0,
            last_hold: None,
        };
        for &line in facts.outline {
            flight
                .outline
                .add(line.map(|point| flight.normalize(point)));
        }
        Some(flight)
    }

    pub fn exit_point(&self) -> Point {
        self.far
    }

    pub fn crossing_frames(&self) -> u16 {
        // The latest retained start reached the far ledge at reference frame 281.
        300
    }

    pub fn needs_fuel_sample(&self) -> bool {
        matches!(self.phase, Phase::Descend)
    }

    fn normalize(&self, [x, y]: Point) -> Point {
        [
            NEAR_X + (x - self.near[0]) * self.direction,
            y - self.near[1],
        ]
    }

    fn underneath(&self, x: f32) -> Option<f32> {
        self.outline
            .segments()
            .iter()
            .filter_map(|&line| {
                let [left, right] = ordered_x(line);
                (left..=right).contains(&x).then(|| line_height(line, x)[0])
            })
            .reduce(f32::min)
    }

    fn near_catch([x, y]: Point) -> bool {
        (NEAR_X - 18.5..=NEAR_X + 12.0).contains(&x) && (-26.0..=-8.0).contains(&y)
    }

    fn forward_descent_is_clear(&self, mut point: Point, mut vx: f32, mut vy: f32) -> bool {
        for _ in 0..160 {
            vx = (vx + AIR_ACCEL).min(AIR_SPEED);
            vy = falling(vy);
            let next = [point[0] + vx, point[1] + vy];
            if Self::near_catch(next)
                || !aerial_clear(
                    self.outline.segments(),
                    [point, next],
                    self.bottom + 6.0,
                    HEAD + 2.0,
                    HULL_HALF_WIDTH,
                )
            {
                return false;
            }
            if next[1] <= PRESS_Y {
                return true;
            }
            point = next;
        }
        false
    }

    fn fly(&mut self, frame: u32, position: Point, velocity: Point) -> Option<Correction> {
        let [x, y] = position;
        if x > self.pilot.far + 12.0 || y > Self::hover_line(x, |x| self.underneath(x)) + 0.6 {
            return None;
        }
        let Some(thrust) = self.pilot.input(position, velocity, |x| self.underneath(x)) else {
            self.phase = Phase::Catch(frame);
            return Some(pad([0.0; 2], 0));
        };
        if thrust && self.last_hold != Some(frame) {
            if self.fuel_used == HOVER_BUDGET {
                return None;
            }
            self.last_hold = Some(frame);
            self.fuel_used += 1;
        }
        Some(pad([self.direction, 0.0], if thrust { SPECIAL } else { 0 }))
    }

    fn hover_line(x: f32, ceiling: impl Fn(f32) -> Option<f32>) -> f32 {
        let gap = 15.0 - 2.0 * ((x - 41.416) / 22.971).clamp(0.0, 1.0);
        ceiling(x).map_or(CATCH_Y + 2.0, |under| under - gap)
    }

    pub fn step(&mut self, state: BurnerState) -> Option<Correction> {
        let BurnerState {
            frame,
            status,
            position,
            velocity,
            full_fuel,
            facing,
            jumps,
            can_start,
        } = state;
        let point = self.normalize(position);
        let [vx, vy] = [velocity[0] * self.direction, velocity[1]];
        if !point.into_iter().chain([vx, vy]).all(f32::is_finite) || point[1] <= self.bottom + 6.0 {
            return None;
        }
        let forward = [self.direction, 0.0];
        let special = pad([0.0, 1.0], SPECIAL);
        match self.phase {
            Phase::Descend
                if status == FALL
                    && full_fuel
                    && facing == self.direction
                    && jumps == 1
                    && !Self::near_catch(point) =>
            {
                let next_y = point[1] + falling(vy);
                if next_y <= PRESS_Y {
                    if !can_start {
                        return None;
                    }
                    self.phase = Phase::Takeoff(frame);
                    Some(special)
                } else {
                    let stick = if self.forward_descent_is_clear(point, vx, vy) {
                        forward
                    } else {
                        [0.0; 2]
                    };
                    Some(pad(stick, 0))
                }
            }
            Phase::Takeoff(since) if status == FALL && frame == since => Some(special),
            Phase::Takeoff(since) if status == TAKEOFF && frame.saturating_sub(since) <= 25 => {
                Some(pad(forward, 0))
            }
            Phase::Takeoff(_) | Phase::Flying if status == HOVER => {
                self.phase = Phase::Flying;
                self.fly(frame, point, [vx, vy])
            }
            Phase::Catch(since) if status == HOVER && frame.saturating_sub(since) <= 2 => {
                Some(pad([0.0; 2], 0))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
lib_unit_tests! {
    mod tests {
        use super::*;

        const LEFT: super::super::Point = [-79.986, 0.0];
        const RIGHT: super::super::Point = [79.989, 0.0];
        const RING: [super::super::Segment; 8] = [
            [LEFT, [-79.986, -8.319]],
            [[-79.986, -8.319], [-64.395, -25.111]],
            [[-64.395, -25.111], [-41.396, -40.099]],
            [[-41.396, -40.099], [41.416, -40.118]],
            [[41.416, -40.118], [64.387, -25.103]],
            [[64.387, -25.103], [79.989, -8.319]],
            [[79.989, -8.319], RIGHT],
            [RIGHT, LEFT],
        ];

        fn facts(stage: (i32, u32), position: super::super::Point) -> super::super::RouteFacts<'static> {
            super::super::RouteFacts {
                fighter: 0x2d,
                stage,
                position,
                player: LEFT,
                player_reach: 5.0,
                ledges: [LEFT, RIGHT],
                blast: [-230.0, 230.0, 185.0, -140.0],
                outline: &RING,
                body_clearance: 17.0,
                ledge_distance: 17.0,
                fall_budget: 1.6,
                reach: [150.0, 130.0],
            }
        }

        #[test]
        fn rob_requires_the_stock_full_tank_before_takeoff() {
            assert!(full_fuel(f32::from_bits(0x4320_0000)));
            for fuel in [137.0, 65.5, 159.75, -0.5, 0.0, 161.0, f32::NAN, f32::INFINITY] {
                assert!(!full_fuel(fuel), "unflown starting fuel {fuel}");
            }
        }

        #[test]
        fn rob_starts_only_with_a_full_burner_and_the_reference_facing_and_jump_reserve() {
            let live = BurnerState {
                frame: 0,
                status: 0xe,
                position: [-119.986, 20.0],
                velocity: [0.0, 0.0],
                facing: 1.0,
                jumps: 1,
                full_fuel: true,
                can_start: true,
            };
            let fd = facts((3, 0), live.position);
            assert!(BurnerFlight::start(&fd, live).is_some());
            assert!(BurnerFlight::start(&fd, BurnerState { full_fuel: false, ..live }).is_none());
            assert!(BurnerFlight::start(&fd, BurnerState { status: 0xc, ..live }).is_none());
            let outside = BurnerState { position: [-149.986, -70.0], ..live };
            assert!(BurnerFlight::start(&facts((3, 0), outside.position), outside).is_none());
            assert!(BurnerFlight::start(&fd, BurnerState { facing: -1.0, ..live }).is_none());
            assert!(BurnerFlight::start(&fd, BurnerState { jumps: 0, ..live }).is_none());
            for stage in [(0, 1), (311, 1), (3, 1), (89, 0)] {
                assert!(BurnerFlight::start(&facts(stage, live.position), live).is_none());
            }
        }

        #[test]
        fn rob_accepts_the_two_reference_cells_on_each_normal_crossing_stage() {
            for stage in [(3, 0), (0, 0), (311, 0)] {
                for right in [false, true] {
                    let (near, direction) = if right { (RIGHT, -1.0) } else { (LEFT, 1.0) };
                    for (outside, y) in [(40.0, 20.0), (60.0, 30.0)] {
                        let live = BurnerState {
                            frame: 0, status: FALL, position: [near[0] - direction * outside, y],
                            velocity: [0.0, -0.8], full_fuel: true, can_start: true,
                            facing: direction, jumps: 1,
                        };
                        let facts = facts(stage, live.position);
                        let route = BurnerFlight::start(&facts, live)
                            .unwrap_or_else(|| panic!("missing {stage:?} {right} {outside} {y}"));
                        assert_eq!(route.direction, direction);
                    }
                }
            }
        }

        #[test]
        fn rob_takeoff_has_one_up_b_frame_then_drives_the_native_hover() {
            use super::super::SPECIAL;

            let live = BurnerState {
                frame: 0,
                status: 0xe,
                position: [-119.986, 20.0],
                velocity: [0.0, 0.0],
                facing: 1.0,
                jumps: 1,
                full_fuel: true,
                can_start: true,
            };
            let mut flight = BurnerFlight::start(&facts((3, 0), live.position), live).unwrap();
            // fusion-g1-proof-b3-yoga-1 / robot-x2-in2-1, exact before-pad samples.
            for (frame, status, position, velocity, sticks, buttons) in [
                (81, 0xe, [-48.74691, -96.16995], [1.134, -1.6], [1.0, 0.0], 0),
                (82, 0xe, [-47.61291, -97.76995], [1.134, -1.6], [0.0, 1.0], SPECIAL),
                (83, 0x1de, [-46.49391, -99.36995], [1.119, -1.6], [1.0, 0.0], 0),
                (107, 0x1e3, [-24.137_9, -74.72796], [0.759, 1.366], [1.0, 0.0], 0),
                (220, 0x1e3, [55.04913, -51.47501], [0.7, 0.321], [1.0, 0.0], SPECIAL),
                (256, 0x1e3, [80.24908, -25.24703], [0.7, 1.203], [1.0, 0.0], SPECIAL),
                (257, 0x1e3, [80.94907, -24.10703], [0.7, 1.14], [0.0, 0.0], 0),
            ] {
                let input = BurnerState {
                    frame, status, position, velocity,
                    full_fuel: status == 0xe,
                    can_start: status == 0xe,
                    ..live
                };
                let output = flight.step(input).unwrap_or_else(|| panic!("lost control on frame {frame}"));
                assert_eq!((output.stick, output.press), (Some(sticks), buttons), "frame {frame}");
                assert_eq!(flight.step(input), Some(output));
            }
        }

        fn simulate(mut state: BurnerState) -> Result<Point, (u32, Point)> {
            let mut flight = BurnerFlight::start(&facts((3, 0), state.position), state)
                .expect("caller selected an in-region release");
            let mut pressed = None;
            for frame in 0..340 {
                state.frame = frame;
                let correction = flight.step(state).ok_or((frame, state.position))?;
                let [steer, up] = correction.stick.expect("route always owns the stick");
                let special = correction.press == SPECIAL;
                if let Some(start) = pressed {
                    let age = frame - start;
                    let vx = state.velocity[0];
                    let vy = state.velocity[1];
                    if age == 1 {
                        state.velocity[1] = (vy + 0.94).max(-0.06);
                        state.velocity[0] = if vx > 0.7 { vx - 0.015 } else { vx };
                    } else if age <= 24 {
                        state.velocity[1] = thrust(vy);
                        state.velocity[0] = hover_drift(vx);
                    } else {
                        if steer == 0.0 {
                            return Ok(state.position);
                        }
                        let boosted = if special { thrust(vy) } else { vy };
                        state.velocity[1] = (boosted - HOVER_GRAVITY).max(-1.5);
                        state.velocity[0] = hover_drift(vx);
                    }
                    state.status = if age < 24 { TAKEOFF } else { HOVER };
                } else {
                    state.velocity[1] = falling(state.velocity[1]);
                    state.velocity[0] = if steer > 0.0 {
                        (state.velocity[0] + AIR_ACCEL).min(AIR_SPEED)
                    } else {
                        (state.velocity[0] - 0.015).max(0.0)
                    };
                    if special && up == 1.0 {
                        pressed = Some(frame);
                        state.status = TAKEOFF;
                        state.can_start = false;
                        state.full_fuel = false;
                    }
                }
                for (axis, delta) in state.position.iter_mut().zip(state.velocity) {
                    *axis += delta;
                }
            }
            Err((state.frame, state.position))
        }

        fn hover_drift(vx: f32) -> f32 {
            if vx < 0.7 { (vx + 0.068).min(0.7) } else { (vx - 0.015).max(0.7) }
        }

        #[test]
        fn rob_baked_ring_reaches_the_far_catch_with_reserve_fuel() {
            let mut failures = Vec::new();
            for (row, mask) in BURNER_REGION.into_iter().enumerate() {
                for column in 0..30 {
                    if mask & (1 << column) == 0 {
                        continue;
                    }
                    for [dx, dy] in [[2.0, 2.0], [0.05, 2.0], [3.95, 2.0], [2.0, 0.05], [2.0, 3.95]] {
                        for fall in [0.0, -0.8, -1.6] {
                            let start = [LEFT[0] - (column as f32 * 4.0 + dx), 40.0 - row as f32 * 4.0 - dy];
                            let state = BurnerState {
                                frame: 0, status: FALL, position: start, velocity: [0.0, fall],
                                facing: 1.0, jumps: 1, full_fuel: true, can_start: true,
                            };
                            let result = simulate(state);
                            if !result.is_ok_and(|[x, y]| (RIGHT[0] - 16.5..=RIGHT[0] + 9.9).contains(&x) && y >= -25.5)
                            {
                                failures.push((row, column, start, fall, result));
                            }
                        }
                    }
                }
            }
            assert!(failures.is_empty(), "{} failures, first {:?}", failures.len(), &failures[..failures.len().min(12)]);
        }

        #[test]
        fn robo_burner_keeps_the_native_f32_takeoff_plateau() {
            let mut vy = -0.06;
            for _ in 0..9 {
                vy = thrust(vy);
            }
            assert_eq!(vy.to_bits(), 1.2f32.to_bits() - 1);
            assert!((thrust(vy) - 1.34).abs() < 1.0e-6);
        }

        #[test]
        fn rob_hover_feathers_at_the_measured_rows_then_releases_at_far_catch() {
            // Final Destination, fusion-g1-proof-b3-yoga-1 / robot-x2-in2-1.
            // Sequence numbers are the before-pad samples.
            let pilot = BurnerPilot { far: 79.989 };
            let hull = |x: f32| {
                if x < 41.416 {
                    Some(-40.118)
                } else if x < 64.387 {
                    Some(-40.118 + (x - 41.416) * (15.015 / 22.971))
                } else if x <= 79.989 {
                    Some(-25.103 + (x - 64.387) * (16.784 / 15.602))
                } else {
                    None
                }
            };
            for (seq, position, velocity, expected) in [
                (33481, [-24.137_95, -74.72796], [0.759, 1.366], Some(false)),
                (33707, [55.049137, -51.475006], [0.7, 0.321], Some(true)),
                (33775, [78.84908, -27.716028], [0.7, 1.191], Some(true)),
                (33779, [80.24908, -25.24703], [0.7, 1.203], Some(true)),
                (33781, [80.94907, -24.10703], [0.7, 1.14], None),
            ] {
                assert_eq!(pilot.input(position, velocity, hull), expected, "seq {seq}");
            }
        }
    }
}
