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
const JUMP_VX: f32 = 0.8;

// FLY-5 on the common ring in reference-crossings/gen_regions.py.
const RING_CELLS: [u32; 44] = [
    0x00000000, 0x1ffffffc, 0x1ffffffc, 0x1ffffffc, 0x1ffffffc, 0x1ffffffc,
    0x1ffffffc, 0x1ffffffc, 0x1ffffffc, 0x07fffffc, 0x03fffffc, 0x00fffffc,
    0x007ffffc, 0x003ffffc, 0x001ffffc, 0x0007fffc, 0x0003fffc, 0x0000fffc,
    0x00007ffc, 0x00003ffc, 0x00000ffc, 0x000007fc, 0x000003fc, 0x000001fc,
    0x0000007c, 0x0000001c, 0x0000000c, 0x00000004, 0x00000004, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000,
];

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
    going: bool,
    jump_frame: Option<u32>,
}

impl PuffFlight {
    pub fn start(facts: &RouteFacts<'_>, state: FlyState) -> Option<Self> {
        if state.remaining != 5
            || !state.velocity.into_iter().all(f32::is_finite)
            || state.velocity[0].abs() > DRIFT
            || !(-FALL..=0.0).contains(&state.velocity[1])
            || !matches!(facts.stage, (0 | 3 | 311, 0))
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
        if !x.is_finite() || !y.is_finite() || outside <= 0.0 || relative_y > 40.0
            || ((right[0] - left[0]) * 0.5 - 79.99).abs() > 0.5
            || (right[1] - left[1]).abs() > 0.5
            || (flat - near[1] + 40.1).abs() > 1.0
            || (bottom - near[1] + 140.0).abs() > 1.0
        {
            return None;
        }
        let column = (outside / 4.0) as u32;
        let row = ((40.0 - relative_y) / 4.0) as usize;
        if RING_CELLS.get(row)? & 1u32.checked_shl(column)? == 0 {
            return None;
        }
        let mut hull = Outline::new();
        for &line in facts.outline {
            hull.add(line);
        }
        Some(Self { hull, near, far, bottom, flat, inward, going: false, jump_frame: None })
    }

    pub fn exit_point(&self) -> Point {
        [self.far[0] + self.inward * 5.0, self.far[1] - 10.0]
    }

    fn clear(&self, from: Point, to: Point) -> bool {
        to[1] > self.bottom + 4.0 && [-HALF, 0.0, HALF].into_iter().all(|dx| {
            let segment = [[from[0] + dx, from[1]], [to[0] + dx, to[1]]];
            !self.hull.segments().iter().any(|&line| blocked(segment, line, HEAD + 1.0))
        })
    }

    /// Wait until drifting can take the feet past the near slope without touching it.
    fn glide_clear(&self, mut pos: Point, [mut vx, mut vy]: Point) -> bool {
        let mid = self.near[0].midpoint(self.far[0]);
        for _ in 0..140 {
            vx = (vx + DRIFT).min(AIR_SPEED);
            vy = (vy - GRAVITY).max(-FALL);
            let next = [pos[0] + self.inward * vx, pos[1] + vy];
            if !self.clear(pos, next) {
                return false;
            }
            if (next[0] - mid) * self.inward >= 0.0 {
                return true;
            }
            pos = next;
        }
        true
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

    fn jump_clears(&self, mut pos: Point, mut vy: f32) -> bool {
        let mut vx = JUMP_VX;
        while vy > 0.0 {
            vx = (vx + JUMP_DRIFT).min(AIR_SPEED);
            let next = [pos[0] + self.inward * vx, pos[1] + vy];
            if next[1] > self.ceiling(next[0]) || !self.clear(pos, next) {
                return false;
            }
            pos = next;
            vy -= GRAVITY;
        }
        true
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
        let next_jump = fly_launch(PUFF, state.remaining).map(|speed| speed - GRAVITY);
        let launch = vy <= 0.0 && state.can_jump && next_jump.is_some();
        let (steer, want_jump) = if (x - self.far[0]) * self.inward >= HALF + 1.0 {
            let target = self.exit_point();
            let brake = if vy > 0.0 { JUMP_DRIFT } else { DRIFT };
            let ahead = x + vx * vx.abs() / (2.0 * brake);
            let dx = target[0] - ahead;
            let steer = if dx.abs() > 0.5 { dx.signum() } else { 0.0 };
            (steer, launch && y < target[1] + 2.0)
        } else {
            if !self.going {
                self.going = self.glide_clear(state.position, [vx * self.inward, vy]);
            }
            let steer = if self.going { self.inward } else { 0.0 };
            let jump = self.going && launch
                && next_jump.is_some_and(|rise| self.jump_clears(state.position, rise));
            (steer, jump)
        };
        let held = self.jump_frame == Some(state.frame);
        let fresh = want_jump && self.jump_frame.is_none_or(|last| last.saturating_add(1) < state.frame);
        if fresh {
            self.jump_frame = Some(state.frame);
        }
        Some(Correction {
            stick: Some([steer, 0.0]),
            press: if held || fresh { JUMP } else { 0 },
            release: JUMP | SPECIAL | ATTACK | GUARD,
        })
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
    fn puff_crosses_the_ring_with_the_measured_jump_budget() {
        for (start, maximum_frames, expected_jumps) in [
            ([-100.0, -28.0], 184, 4),
            ([-91.986, 30.0], 260, 4),
            ([-93.986, -66.0], 188, 5),
        ] {
            let mut state = live(start, [0.0; 2]);
            let mut flight = PuffFlight::start(&facts(start), state).unwrap();
            let [hang_x, hang_y] = flight.exit_point();
            for frame in 0..maximum_frames {
                state.frame = frame;
                let edit = flight.step(state).expect("fits the stage and blast margins");
                let x = edit.stick.unwrap()[0];
                let [vx, vy] = &mut state.velocity;
                if edit.press & JUMP != 0 {
                    let first_step = if state.position[0] >= RIGHT[0] + 3.0 { 0.0 } else { JUMP_DRIFT };
                    *vx = x * (JUMP_VX + first_step);
                    *vy = fly_launch(PUFF, state.remaining).unwrap() - GRAVITY;
                    state.remaining -= 1;
                } else {
                    let drift = if *vy > 0.0 { JUMP_DRIFT } else { DRIFT };
                    *vx = if x == 0.0 {
                        vx.signum() * (vx.abs() - drift).max(0.0)
                    } else {
                        (*vx + x * drift).clamp(-AIR_SPEED, AIR_SPEED)
                    };
                    *vy = (*vy - GRAVITY).max(-FALL);
                }
                state.position[0] += *vx;
                state.position[1] += *vy;
                if state.position[1] >= hang_y && (state.position[0] - hang_x).abs() <= 4.0 {
                    break;
                }
            }
            assert!(
                state.position[1] >= hang_y && (state.position[0] - hang_x).abs() <= 4.0,
                "{start:?} reached only {:?} at frame {}", state.position, state.frame
            );
            assert_eq!(5 - state.remaining, expected_jumps, "{start:?}");
        }
    }

    #[test]
    fn puff_stays_below_the_live_hull_and_steers_at_far_hang_height() {
        let start = live([-90.0, 30.0], [0.0; 2]);
        let mut flight = PuffFlight::start(&facts(start.position), start).unwrap();
        let first = flight.step(start).unwrap();
        assert_eq!(first.stick, Some([0.0, 0.0]));
        assert_eq!(first.press, 0);

        let glide = FlyState { frame: 60, ..live([-90.0, -33.0], [0.0, -0.98]) };
        assert_eq!(flight.step(glide).unwrap().stick, Some([1.0, 0.0]));
        let under = FlyState { frame: 100, ..live([-30.0, -74.0], [1.332, -0.98]) };
        assert_eq!(flight.step(under).unwrap().press, 0);
        let room = FlyState { frame: 105, position: [-30.0, -78.5], ..under };
        let jump = flight.step(room).unwrap();
        assert_eq!(jump.press, crate::model::edge_juggle::perfect::JUMP);
        assert_eq!(jump.stick, Some([1.0, 0.0]));
        assert_eq!(flight.step(room).unwrap(), jump);
        let rising = FlyState { frame: 106, remaining: 4, velocity: [0.914, 1.597], ..room };
        assert_eq!(flight.step(rising).unwrap().press, 0);

        let beside = FlyState {
            frame: 180,
            remaining: 2,
            ..live([83.2, -22.0], [0.0, -0.98])
        };
        let climb = flight.step(beside).unwrap();
        assert_eq!(climb.stick, Some([1.0, 0.0]));
        assert_eq!(climb.press, crate::model::edge_juggle::perfect::JUMP);
    }
    }
}
