//! Measured multi-jump crossing for Jigglypuff.

use super::{blocked, fly_launch, Correction, Outline, Point, RouteFacts, ATTACK, GUARD, JUMP, SPECIAL};

const PUFF: i32 = 0xc;
const HEAD: f32 = 9.5;
const HALF: f32 = 2.0;
const GRAVITY: f32 = 0.053;
const FALL: f32 = 0.98;
const AIR_SPEED: f32 = 1.332;
const DRIFT: f32 = 0.13;
const JUMP_DRIFT: f32 = 0.114;
const JUMP_VX: f32 = 0.914;
const JUMP_GAP: u32 = 30;
const TURN_BRAKE: u32 = 8;
const TURN_OUTSIDE: f32 = 8.0;

// FLY-L, the 189-cell ring baked by reference-crossings/g1/g1_sim.py.
const RING_CELLS: [u32; 44] = [
    0x00000000, 0x0fffffc0, 0x03ffffc0, 0x01ffffc0, 0x00ffffc0, 0x003fffc0,
    0x001fffc0, 0x000fffc0, 0x0003ffc0, 0x0001ffc0, 0x00007fc0, 0x00003fc0,
    0x00001fc0, 0x000007c0, 0x000003c0, 0x000001c0, 0x00000040, 0x00000020,
    0x0000001c, 0x00000004, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000,
];

/// The live shape must still match the measured profile.
pub fn puff_stage(stage: (i32, u32)) -> bool {
    stage == (3, 0)
}

fn falling(vy: f32) -> f32 {
    (vy - GRAVITY).max(-FALL)
}

/// Puff stays in native FLY, not JUMP_AERIAL, during its multijumps (fd-4, seq 121241).
pub fn puff_active_status(status: i32, standard_airborne: bool) -> bool {
    status == 0xd || standard_airborne
}

#[derive(Clone, Copy)]
pub struct FlyState {
    pub position: Point,
    pub velocity: Point,
    pub remaining: u8,
    pub can_jump: bool,
    pub frame: u32,
}

pub struct PuffFlight {
    hull: Outline,
    near: Point,
    far: Point,
    bottom: f32,
    flat: f32,
    inward: f32,
    descended: bool,
    jump: Option<(u32, f32)>,
    turn_frame: Option<u32>,
}

impl PuffFlight {
    pub fn start(facts: &RouteFacts<'_>, state: FlyState) -> Option<Self> {
        if facts.fighter != PUFF
            || state.remaining != 5
            || !state.velocity.into_iter().all(f32::is_finite)
            || state.velocity[0].abs() > DRIFT
            || !(-FALL..=0.0).contains(&state.velocity[1])
            || !puff_stage(facts.stage)
            || !(facts.body_clearance > 0.0 && facts.body_clearance <= HEAD)
        {
            return None;
        }
        let [left, right] = facts.ledges;
        let [x, y] = state.position;
        let [near, far] = if x < left[0] { [left, right] } else { [right, left] };
        let inward = (far[0] - near[0]).signum();
        let outside = (near[0] - x) * inward;
        let relative_y = y - near[1];
        let bottom = facts.blast[3];
        let flat = facts.outline.iter().flatten().map(|point| point[1]).reduce(f32::min)?;
        facts.crossing_band()?;
        if ((right[0] - left[0]) * 0.5 - 79.99).abs() > 0.5
            || (right[1] - left[1]).abs() > 0.5
            || (flat - near[1] + 40.1).abs() > 1.0
            || (bottom - near[1] + 140.0).abs() > 1.0
        {
            return None;
        }
        if !super::region::contains(&RING_CELLS, outside, relative_y) {
            return None;
        }
        let mut hull = Outline::new();
        for &line in facts.outline {
            hull.add(line);
        }
        Some(Self { hull, near, far, bottom, flat, inward, descended: false, jump: None, turn_frame: None })
    }

    pub fn exit_point(&self) -> Point {
        [self.far[0] + self.inward * 5.0, self.far[1] - 10.0]
    }

    fn clear(&self, from: Point, to: Point) -> bool {
        to[1] > self.bottom + 6.0 && [-HALF, 0.0, HALF].into_iter().all(|dx| {
            let segment = [[from[0] + dx, from[1]], [to[0] + dx, to[1]]];
            !self.hull.segments().iter().any(|&line| blocked(segment, line, HEAD + 1.0))
        })
    }

    fn near_catch(&self, [x, y]: Point) -> bool {
        let into_stage = (x - self.near[0]) * self.inward;
        (-18.0..=12.0).contains(&into_stage) && (-26.0..=-6.0).contains(&(y - self.near[1]))
    }

    fn glide_clear(&self, mut pos: Point, [mut vx, mut vy]: Point) -> bool {
        for _ in 0..160 {
            vx = (vx + DRIFT).min(AIR_SPEED);
            vy = falling(vy);
            let next = [pos[0] + self.inward * vx, pos[1] + vy];
            if !self.clear(pos, next) || self.near_catch(next) {
                return false;
            }
            if next[1] <= self.flat - HEAD - 2.0 {
                return true;
            }
            pos = next;
        }
        false
    }

    fn ceiling(&self, x: f32) -> f32 {
        if (x - self.far[0]) * self.inward >= HALF + 1.0 {
            return f32::INFINITY;
        }
        if (x - self.near[0].midpoint(self.far[0])) * self.inward <= 0.0 {
            return self.flat - HEAD - 2.0;
        }
        let mut roof = f32::INFINITY;
        for dx in [-HALF, 0.0, HALF] {
            let probe = x + dx;
            for &line in self.hull.segments() {
                let [left, right] = super::ordered_x(line);
                if (left..=right).contains(&probe) {
                    roof = roof.min(super::line_height(line, probe)[0]);
                }
            }
        }
        roof - HEAD - 2.0
    }

    fn jump_clears(&self, state: FlyState, mut vy: f32) -> bool {
        let [x, y] = state.position;
        let pre_launch = [
            x + self.inward * (state.velocity[0] * self.inward + JUMP_DRIFT).min(AIR_SPEED),
            y + falling(state.velocity[1]),
        ];
        if !self.clear(state.position, pre_launch) {
            return false;
        }
        let mut pos = pre_launch;
        let mut vx = JUMP_VX;
        while vy > 0.0 {
            let next = [pos[0] + self.inward * vx, pos[1] + vy];
            // An inward jump must finish rising before the far-side turn.
            // A moving Puff cannot brake in time while waiting for that jump's apex.
            if (next[0] - self.far[0]) * self.inward >= TURN_OUTSIDE
                || next[1] > self.ceiling(next[0])
                || !self.clear(pos, next)
            {
                return false;
            }
            pos = next;
            vy -= GRAVITY;
            vx = (vx + JUMP_DRIFT).min(AIR_SPEED);
        }
        true
    }

    fn pad(steer: f32, jump: bool) -> Correction {
        Correction {
            stick: Some([steer, 0.0]),
            press: if jump { JUMP } else { 0 },
            release: JUMP | SPECIAL | ATTACK | GUARD,
        }
    }

    pub fn step(&mut self, state: FlyState) -> Option<Correction> {
        let [x, y] = state.position;
        let [vx, vy] = state.velocity;
        if ![x, y, vx, vy].into_iter().all(f32::is_finite)
            || state.remaining > 5
            || !self.clear(state.position, state.position)
        {
            return None;
        }
        if let Some((frame, steer)) = self.jump {
            if state.frame.saturating_sub(frame) < 2 {
                return Some(Self::pad(steer, true));
            }
        }
        let next_y = y + falling(vy);
        if !self.descended {
            if self.near_catch(state.position) {
                return None;
            }
            if next_y > self.flat - HEAD - 2.0 {
                let steer = if self.glide_clear(state.position, [vx * self.inward, vy]) {
                    self.inward
                } else {
                    0.0
                };
                return Some(Self::pad(steer, false));
            }
            self.descended = true;
        }
        let launch = vy <= 0.0
            && state.can_jump
            && state.remaining > 0
            && self.jump.is_none_or(|(frame, _)| state.frame.saturating_sub(frame) >= JUMP_GAP);
        let outside_far = (x - self.far[0]) * self.inward;
        let (steer, jump) = if let Some(turn) = self.turn_frame {
            if state.frame.saturating_sub(turn) < 2 + TURN_BRAKE {
                (self.inward, false)
            } else {
                (0.0, launch && y < self.far[1] - 23.0)
            }
        } else if outside_far >= TURN_OUTSIDE {
            if launch {
                self.turn_frame = Some(state.frame);
                (-self.inward, true)
            } else {
                (0.0, false)
            }
        } else {
            let jump = launch && state.remaining > 1 && outside_far < -4.0
                && fly_launch(PUFF, state.remaining)
                    .is_some_and(|rise| self.jump_clears(state, rise - GRAVITY));
            (self.inward, jump)
        };
        if jump {
            self.jump = Some((state.frame, steer));
        }
        Some(Self::pad(steer, jump))
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::model::edge_juggle::perfect::Segment;

    const LEFT: [f32; 2] = [-79.986, 0.0];
    const RIGHT: [f32; 2] = [79.989, 0.0];
    const RING: [Segment; 8] = [
        [LEFT, [-79.986, -8.319]],
        [[-79.986, -8.319], [-64.395, -25.111]],
        [[-64.395, -25.111], [-41.396, -40.099]],
        [[-41.396, -40.099], [41.416, -40.118]],
        [[41.416, -40.118], [64.387, -25.103]],
        [[64.387, -25.103], [79.989, -8.319]],
        [[79.989, -8.319], RIGHT],
        [RIGHT, LEFT],
    ];

    fn facts(position: [f32; 2]) -> RouteFacts<'static> {
        RouteFacts {
            fighter: 0xc,
            stage: (3, 0),
            position,
            player: [-75.0, 0.0],
            player_reach: 9.0,
            ledges: [LEFT, RIGHT],
            blast: [-230.0, 230.0, 185.0, -140.0],
            outline: &RING,
            body_clearance: 9.5,
            ledge_distance: 10.0,
            fall_budget: 0.98,
            reach: [130.0, 20.0],
        }
    }

    fn live(position: [f32; 2], velocity: [f32; 2]) -> FlyState {
        FlyState { position, velocity, remaining: 5, can_jump: true, frame: 0 }
    }

    #[test]
    fn puff_steers_through_native_fly_status() {
        assert!(puff_active_status(0xd, false));
        assert!(puff_active_status(0xe, true));
        assert!(!puff_active_status(0x4b, false)); // damage fly
        assert!(!puff_active_status(0x1de, false)); // another move
    }

    #[test]
    fn puff_requires_the_measured_region_and_all_five_jumps() {
        let start = live([-100.0, -28.0], [0.0; 2]);
        let safe = facts(start.position);
        assert!(PuffFlight::start(&safe, start).is_some());
        assert!(PuffFlight::start(&safe, FlyState { remaining: 4, ..start }).is_none());
        assert!(PuffFlight::start(&safe, FlyState { velocity: [1.0, 0.0], ..start }).is_none());
        assert!(PuffFlight::start(&RouteFacts { stage: (4, 0), ..safe }, start).is_none());
        let too_low = live([-130.0, -70.0], [0.0; 2]);
        assert!(PuffFlight::start(&facts(too_low.position), too_low).is_none());
    }

    #[test]
    fn puff_fly_l_admits_only_the_measured_ring() {
        let inside = live([-100.0, -28.0], [0.0; 2]);
        assert!(PuffFlight::start(&facts(inside.position), inside).is_some());
        let old_low_cell = live([-93.986, -66.0], [0.0; 2]);
        assert!(PuffFlight::start(&facts(old_low_cell.position), old_low_cell).is_none());
    }

    #[test]
    fn puff_fly_l_turn_has_a_two_frame_press_before_braking() {
        let start = live([-100.0, -28.0], [0.0; 2]);
        let mut flight = PuffFlight::start(&facts(start.position), start).unwrap();
        flight.descended = true;
        let turn = FlyState {
            frame: 197,
            remaining: 3,
            ..live([88.47, -45.4], [1.332, -0.98])
        };
        let reverse = flight.step(turn).unwrap();
        assert_eq!((reverse.stick, reverse.press), (Some([-1.0, 0.0]), JUMP));
        assert_eq!(flight.step(turn).unwrap(), reverse);
        let before_launch = FlyState {
            frame: 198,
            position: [89.69, -46.38],
            ..turn
        };
        assert_eq!(flight.step(before_launch).unwrap(), reverse);

        let launched = FlyState {
            frame: 199,
            position: [88.66, -44.96],
            velocity: [-0.914, 1.417],
            remaining: 2,
            ..turn
        };
        let braking = flight.step(launched).unwrap();
        assert_eq!((braking.stick, braking.press), (Some([1.0, 0.0]), 0));
    }

    fn verify_flight(start: Point, far: Point, direction: f32, vy: f32) {
        let mut state = live(start, [0.0, vy]);
        let mut flight = PuffFlight::start(&facts(start), state).unwrap();
        let mut first_press = false;
        let mut caught = false;
        let mut presses = Vec::new();
        for frame in 0..330 {
            state.frame = frame;
            let edit = flight.step(state).unwrap_or_else(|| {
                panic!("start {start:?} off route at {:?}, frame {frame}", state.position)
            });
            let steer = edit.stick.unwrap()[0];
            let press = edit.press & JUMP != 0;
            if press && !first_press {
                presses.push((frame, steer, state.position));
            }
            let [vx, vy] = &mut state.velocity;
            if press && first_press {
                *vx = steer * JUMP_VX;
                *vy = fly_launch(PUFF, state.remaining).unwrap() - GRAVITY;
                state.remaining -= 1;
                first_press = false;
            } else {
                let accel = if press || *vy > 0.0 { JUMP_DRIFT } else { DRIFT };
                *vx = if steer == 0.0 {
                    vx.signum() * (vx.abs() - 0.0375).max(0.0)
                } else {
                    (*vx + steer * accel).clamp(-AIR_SPEED, AIR_SPEED)
                };
                *vy = falling(*vy);
                first_press = press;
            }
            state.position[0] += *vx;
            state.position[1] += *vy;
            let out_far = (state.position[0] - far[0]) * direction;
            if *vy < 0.0 && (4.5..=8.0).contains(&out_far)
                && (-23.5..=-8.0).contains(&state.position[1])
            {
                caught = true;
                break;
            }
        }
        assert!(caught, "start {start:?} at vy {vy} reached {:?} at frame {} (presses {presses:?})", state.position, state.frame);
        assert!((1..=2).contains(&state.remaining), "start {start:?} overspent jumps");
    }

    #[test]
    fn puff_crosses_the_measured_fd_routes_in_both_directions() {
        for [out, y] in [[20.0, -28.0], [40.0, 20.0], [90.0, 30.0], [26.0, -14.0]] {
            verify_flight([LEFT[0] - out, y], RIGHT, 1.0, 0.0);
            verify_flight([RIGHT[0] + out, y], LEFT, -1.0, 0.0);
        }
    }

    #[test]
    fn puff_fly_l_baked_cell_centers_keep_a_jump_in_reserve() {
        for (row, mask) in RING_CELLS.into_iter().enumerate() {
            for cell in 0..30 {
                if mask & (1 << cell) != 0 {
                    let start = [LEFT[0] - (4 * cell + 2) as f32, 38.0 - 4.0 * row as f32];
                    for vy in [0.0, -0.49, -0.98] {
                        verify_flight(start, RIGHT, 1.0, vy);
                    }
                }
            }
        }
    }

    #[test]
    fn puff_fly_l_baked_cell_edge_midpoints_keep_a_jump_in_reserve() {
        for (row, mask) in RING_CELLS.into_iter().enumerate() {
            for cell in 0..30 {
                if mask & (1 << cell) == 0 {
                    continue;
                }
                let center = [LEFT[0] - (4 * cell + 2) as f32, 38.0 - 4.0 * row as f32];
                for [dx, dy] in [[-1.95, 0.0], [1.95, 0.0], [0.0, -1.95], [0.0, 1.95]] {
                    let start = [center[0] + dx, center[1] + dy];
                    for vy in [0.0, -0.49, -0.98] {
                        verify_flight(start, RIGHT, 1.0, vy);
                    }
                }
            }
        }
    }

    #[test]
    fn puff_avoids_a_near_ledge_snap_during_descent() {
        let start = live([-106.0, 25.0], [0.0; 2]);
        let mut flight = PuffFlight::start(&facts(start.position), start).unwrap();
        assert_eq!(flight.step(start).unwrap().stick, Some([0.0, 0.0]));
        let in_catch = live([-90.0, -19.0], [0.0, -0.98]);
        assert!(flight.step(in_catch).is_none());
        let below_lip = live([-100.0, -28.0], [0.0, -0.98]);
        assert_eq!(flight.step(below_lip).unwrap().stick, Some([1.0, 0.0]));
    }
    }
}
