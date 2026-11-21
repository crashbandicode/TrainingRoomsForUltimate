//! Native finishing moves for Pit and Dark Pit's measured crossings.

use super::{aerial_clear, fly_launch, niche::pad, Correction, Outline, Point, RouteFacts, JUMP, SPECIAL};

#[derive(Clone, Copy)]
pub struct Entry {
    pub position: Point,
    pub velocity: Point,
    pub remaining: u8,
    pub facing: f32,
    pub can_special: bool,
    pub frame: u32,
}

struct Finisher {
    started: u32,
}

impl Finisher {
    fn start(far: Point, inward: f32, entry: Entry) -> Option<Self> {
        if !entry.can_special
            || entry.remaining != 0
            || !matches!(inward, -1.0 | 1.0)
            || !far.into_iter().chain(entry.position).chain(entry.velocity).all(f32::is_finite)
        {
            return None;
        }
        let outside = (entry.position[0] - far[0]) * inward;
        let height = entry.position[1] - far[1];
        // The fourteen start rows carry 8.9 times the entering horizontal speed.
        let after_start = outside + FINISH_CARRY * entry.velocity[0] * inward;
        (entry.facing == inward
            && (5.0..=8.5).contains(&after_start)
            && (-120.0..=-40.0).contains(&height))
            .then_some(Self { started: entry.frame })
    }

    fn step(&self, frame: u32, status: i32) -> Option<Correction> {
        let age = frame.checked_sub(self.started)?;
        match (age, status) {
            (0, 0xd..=0xf) => Some(pad([0.0, 1.0], SPECIAL)),
            (1..=67, 0x1de | 0x1e7) => Some(pad([0.0, 1.0], 0)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Sample {
    pub entry: Entry,
    pub status: i32,
    pub can_jump: bool,
}

const GRAVITY: f32 = 0.081;
const FALL_SPEED: f32 = 1.48;
const DRIFT_SPEED: f32 = 0.935;
const BODY_HEIGHT: f32 = 16.5;
const JUMP_X_SPEED: f32 = 0.81;
const JUMP_GAP: u32 = 31;
const JUMPS: u8 = 3;
const NEAR_CATCH: [f32; 3] = [-18.0, -27.0, -9.0];
const PIT_CROSS_RING: [u32; 11] = [0, 0x1fc0, 0xfc0, 0x7c0, 0x7c0, 0x3c0, 0x3c0, 0x1c0, 0xc0, 0xc0, 0x40];
const FINISH_CARRY: f32 = 8.9;

const HALF: f32 = 2.0;
const DRIFT: f32 = 0.075;
const JUMP_DRIFT: f32 = 0.062;

enum Phase {
    Descend,
    Bounce,
    Finish(Finisher),
}

pub struct WingsFlight {
    fighter: i32,
    hull: Outline,
    near: Point,
    far: Point,
    bottom: f32,
    flat: f32,
    inward: f32,
    phase: Phase,
    jump_frame: Option<u32>,
}

impl WingsFlight {
    pub fn start(facts: &RouteFacts<'_>, live: Sample) -> Option<Self> {
        let entry = live.entry;
        if !matches!(facts.fighter, 0x1e | 0x1f)
            || !super::is_crossing_stage(facts.stage)
            || live.status != 0xe
            || entry.remaining != JUMPS
            || !entry.can_special
            || !entry.position.into_iter().chain(entry.velocity).all(f32::is_finite)
            || entry.velocity[0] != 0.0
            || !(-FALL_SPEED..=0.0).contains(&entry.velocity[1])
            || !(facts.body_clearance > 0.0 && facts.body_clearance <= BODY_HEIGHT)
        {
            return None;
        }
        facts.crossing_band()?;
        let [left, right] = facts.ledges;
        let [near, far] = if entry.position[0] < left[0] { [left, right] } else { [right, left] };
        let inward = (far[0] - near[0]).signum();
        let outside = (near[0] - entry.position[0]) * inward;
        let height = entry.position[1] - near[1];
        let bottom = facts.blast[3];
        let flat = facts.outline.iter().flatten().map(|p| p[1]).reduce(f32::min)?;
        if entry.facing != inward
            || ((right[0] - left[0]) * 0.5 - 79.99).abs() > 0.5
            || (right[1] - left[1]).abs() > 0.5
            || (flat - near[1] + 40.1).abs() > 1.0
            || (bottom - near[1] + 140.0).abs() > 1.0
            || !super::region::contains(&PIT_CROSS_RING, outside, height)
        {
            return None;
        }
        let mut hull = Outline::new();
        for &line in facts.outline {
            hull.add(line);
        }
        Some(Self {
            fighter: facts.fighter, hull, near, far, bottom, flat, inward,
            phase: Phase::Descend, jump_frame: None,
        })
    }

    pub fn exit_point(&self) -> Point {
        self.far
    }

    pub const fn crossing_frames(&self) -> u16 {
        330
    }

    fn fall(vy: f32) -> f32 {
        (vy - GRAVITY).max(-FALL_SPEED)
    }

    fn clear(&self, from: Point, to: Point) -> bool {
        aerial_clear(self.hull.segments(), [from, to], self.bottom + 6.0, BODY_HEIGHT + 1.0, HALF)
    }

    fn near_catch(&self, [x, y]: Point) -> bool {
        let [inbound, low, top] = NEAR_CATCH;
        (inbound..=12.0).contains(&((x - self.near[0]) * self.inward))
            && (low..=top).contains(&(y - self.near[1]))
    }

    fn descent_target(&self) -> f32 {
        self.flat - BODY_HEIGHT - 2.0
    }

    fn can_glide(&self, mut pos: Point, [vx, mut vy]: Point) -> bool {
        let mut forward = vx * self.inward;
        for _ in 0..160 {
            forward = (forward + DRIFT).min(DRIFT_SPEED);
            vy = Self::fall(vy);
            let next = [pos[0] + self.inward * forward, pos[1] + vy];
            if self.near_catch(next) || !self.clear(pos, next) {
                return false;
            }
            if next[1] <= self.descent_target() {
                return true;
            }
            pos = next;
        }
        false
    }

    fn ceiling(&self, x: f32) -> f32 {
        let flat = self.descent_target();
        if (x - self.near[0].midpoint(self.far[0])) * self.inward <= 0.0 {
            return flat;
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
        if roof.is_finite() { roof - BODY_HEIGHT - 2.0 } else { flat }
    }

    fn jump_clears(&self, entry: Entry, launch: f32) -> bool {
        let [x, y] = entry.position;
        let [vx, vy] = entry.velocity;
        let steer = self.inward;
        let carry_x = (vx + steer * JUMP_DRIFT).clamp(-DRIFT_SPEED, DRIFT_SPEED);
        let mut pos = [x + carry_x, y + Self::fall(vy)];
        if !self.clear(entry.position, pos) {
            return false;
        }
        let mut rise = launch - GRAVITY;
        let mut drift = steer * JUMP_X_SPEED;
        while rise > 0.0 {
            let next = [pos[0] + drift, pos[1] + rise];
            if !self.clear(pos, next) || next[1] > self.ceiling(next[0]) {
                return false;
            }
            pos = next;
            rise -= GRAVITY;
            drift = (drift + steer * JUMP_DRIFT).clamp(-DRIFT_SPEED, DRIFT_SPEED);
        }
        true
    }

    fn start_finish(&mut self, live: Sample) -> Option<Correction> {
        let flight = Finisher::start(self.far, self.inward, live.entry)?;
        let edit = flight.step(live.entry.frame, live.status)?;
        self.phase = Phase::Finish(flight);
        Some(edit)
    }

    pub fn step(&mut self, live: Sample) -> Option<Correction> {
        let entry = live.entry;
        if let Phase::Finish(flight) = &self.phase {
            return flight.step(entry.frame, live.status);
        }
        if !matches!(live.status, 0xd..=0xf)
            || !entry.position.into_iter().chain(entry.velocity).all(f32::is_finite)
            || entry.remaining > JUMPS
            || !self.clear(entry.position, entry.position)
        {
            return None;
        }
        if let Some(frame) = self.jump_frame {
            if entry.frame.saturating_sub(frame) < 2 {
                return Some(pad([self.inward, 0.0], JUMP));
            }
        }
        let [x, y] = entry.position;
        let [vx, vy] = entry.velocity;
        if matches!(self.phase, Phase::Descend) {
            if self.near_catch(entry.position) {
                return None;
            }
            if y + Self::fall(vy) > self.descent_target() {
                let steer = if self.can_glide(entry.position, entry.velocity) { self.inward } else { 0.0 };
                return Some(pad([steer, 0.0], 0));
            }
            self.phase = Phase::Bounce;
        }
        let outside = (x - self.far[0]) * self.inward;
        if outside + FINISH_CARRY * vx * self.inward >= 5.0 {
            return self.start_finish(live);
        }
        let launch = (live.can_jump && vy <= 0.0
            && self.jump_frame.is_none_or(|frame| entry.frame.saturating_sub(frame) >= JUMP_GAP))
            .then(|| fly_launch(self.fighter, entry.remaining))
            .flatten();
        let bounce = outside < -4.0
            && launch.is_some_and(|launch| self.jump_clears(entry, launch));
        if bounce {
            self.jump_frame = Some(entry.frame);
        }
        Some(pad([self.inward, 0.0], if bounce { JUMP } else { 0 }))
    }
}

#[cfg(test)]
lib_unit_tests! {
    mod tests {
        use super::*;
        use crate::model::edge_juggle::perfect::Segment;

        const LEFT: Point = [-79.986, 0.0];
        const RIGHT: Point = [79.989, 0.0];
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

        fn facts(fighter: i32, position: Point, stage: (i32, u32)) -> RouteFacts<'static> {
            RouteFacts {
                fighter,
                stage,
                position,
                player: [-75.0, 0.0],
                player_reach: 6.0,
                ledges: [LEFT, RIGHT],
                blast: [-230.0, 230.0, 185.0, -140.0],
                outline: &RING,
                body_clearance: 16.5,
                ledge_distance: 15.0,
                fall_budget: 1.48,
                reach: [90.0, 105.0],
            }
        }

        fn falling(x: f32, y: f32, remaining: u8) -> Sample {
            Sample {
                entry: Entry {
                    position: [x, y],
                    velocity: [0.0, 0.0],
                    facing: if x < 0.0 { 1.0 } else { -1.0 },
                    remaining,
                    can_special: true,
                    frame: 0,
                },
                status: 0xe,
                can_jump: true,
            }
        }

        #[test]
        fn pit_and_dark_pit_start_in_their_baked_cells_with_the_measured_jump_reserve() {
            for (fighter, jumps) in [(0x1e, 3), (0x1f, 3)] {
                for stage in [(3, 0), (0, 0)] {
                    for [x, y] in [[LEFT[0] - 30.0, 6.0], [RIGHT[0] + 26.0, 34.0]] {
                        let mut live = falling(x, y, jumps);
                        let route = facts(fighter, live.entry.position, stage);
                        let mut flight = WingsFlight::start(&route, live).expect("reference cell");
                        assert!(flight.step(live).is_some());
                        live.entry.remaining -= 1;
                        assert!(WingsFlight::start(&route, live).is_none());
                    }
                    let live = falling(LEFT[0] - 20.0, -28.0, jumps);
                    assert!(WingsFlight::start(&facts(fighter, live.entry.position, stage), live).is_none());
                }
                let live = falling(LEFT[0] - 30.0, 6.0, jumps);
                for stage in [(0, 1), (3, 1), (311, 1), (4, 0)] {
                    assert!(WingsFlight::start(&facts(fighter, live.entry.position, stage), live).is_none());
                }
            }
        }

        #[test]
        fn pit_and_dark_pit_admit_both_waiting_room_confirm_starts_but_never_a_stage_form() {
            for fighter in [0x1e, 0x1f] {
                for (outside, y) in [(26.0, 34.0), (30.0, 6.0)] {
                    let live = falling(LEFT[0] - outside, y, 3);
                    assert!(WingsFlight::start(&facts(fighter, live.entry.position, (311, 0)), live).is_some());
                    assert!(WingsFlight::start(&facts(fighter, live.entry.position, (311, 1)), live).is_none());
                }
            }
        }

        fn run_to_finish(fighter: i32, mut live: Sample) -> (u32, Entry) {
            let route = facts(fighter, live.entry.position, (3, 0));
            let mut flight = WingsFlight::start(&route, live).unwrap();
            let start = live.entry;
                let mut pressed_last = false;
            let mut jump_facing = None;
            for frame in 0..400 {
                live.entry.frame = frame;
                let pad = flight.step(live).unwrap_or_else(|| {
                    panic!("kind {fighter:#x}, start {:?} with {:?}, frame {frame}, at {:?} with {:?}, jumps {}, facing {}",
                        start.position, start.velocity, live.entry.position, live.entry.velocity,
                        live.entry.remaining, live.entry.facing)
                });
                if pad.press & SPECIAL != 0 {
                    return (frame, live.entry);
                }
                let [steer, _] = pad.stick.unwrap();
                let press = pad.press & JUMP != 0;
                if press && pressed_last {
                    live.entry.velocity = [steer * JUMP_X_SPEED,
                        fly_launch(fighter, live.entry.remaining).unwrap() - GRAVITY];
                    live.entry.remaining -= 1;
                    live.status = 0xd;
                    jump_facing = Some(steer);
                    pressed_last = false;
                } else {
                    if let Some(facing) = jump_facing.take() {
                        live.entry.facing = facing;
                    }
                    let [vx, vy] = &mut live.entry.velocity;
                    let accel = if press || *vy > 0.0 { JUMP_DRIFT } else { DRIFT };
                    *vx = if steer == 0.0 {
                        vx.signum() * (vx.abs() - 0.008).max(0.0)
                    } else {
                        (*vx + steer * accel).clamp(-DRIFT_SPEED, DRIFT_SPEED)
                    };
                    *vy = (*vy - GRAVITY).max(-FALL_SPEED);
                    pressed_last = press;
                }
                for axis in 0..2 {
                    live.entry.position[axis] += live.entry.velocity[axis];
                }
            }
            panic!("the authored move did not fire for {fighter:#x} from {:?}", start.position);
        }

        #[test]
        fn wing_regions_reach_the_measured_finisher_window() {
                for fighter in [0x1e, 0x1f] {
                for (row, mask) in PIT_CROSS_RING.into_iter().enumerate() {
                    for col in 0..30 {
                        if mask & (1 << col) == 0 {
                            continue;
                        }
                        let out = 4.0 * col as f32 + 2.0;
                        let y = 38.0 - 4.0 * row as f32;
                        for [dx, dy] in [[0.0, 0.0], [-1.95, 0.0], [1.95, 0.0], [0.0, -1.95], [0.0, 1.95]] {
                            for speed in [0.0, -FALL_SPEED * 0.5, -FALL_SPEED] {
                                let mut live = falling(LEFT[0] - out + dx, y + dy, JUMPS);
                                live.entry.velocity[1] = speed;
                                let (frame, entry) = run_to_finish(fighter, live);
                                assert!(frame < 330 && entry.remaining == 0);
                            }
                        }
                    }
                }
                let mirrored = falling(RIGHT[0] + 30.0, 6.0, JUMPS);
                run_to_finish(fighter, mirrored);
            }
        }

        #[test]
        fn pit_finisher_uses_carried_x_to_fly_straight_up() {
            // Battlefield reference pit-bf-in3-1, departure seq 124910.
            let far = [79.989, 0.0];
            let entry = Entry {
                position: [76.94695, -109.23608],
                velocity: [0.935, -1.48],
                remaining: 0,
                facing: 1.0,
                can_special: true,
                frame: 237,
            };
            let flight = Finisher::start(far, 1.0, entry).unwrap();
            assert_eq!(flight.step(237, 0xf), Some(pad([0.0, 1.0], SPECIAL)));
            assert_eq!(flight.step(238, 0x1de), Some(pad([0.0, 1.0], 0)));
            assert_eq!(flight.step(252, 0x1e7), Some(pad([0.0, 1.0], 0)));
            assert!(flight.step(289, 0x75).is_none());
            let mirrored = Entry {
                position: [-entry.position[0], entry.position[1]],
                velocity: [-entry.velocity[0], entry.velocity[1]],
                facing: -1.0,
                ..entry
            };
            assert!(Finisher::start([-far[0], far[1]], -1.0, mirrored).is_some());
            let fast = Entry { velocity: [1.3, -1.48], ..entry };
            let late = Entry { position: [entry.position[0], -124.0], ..entry };
            assert!(Finisher::start(far, 1.0, fast).is_none());
            assert!(Finisher::start(far, 1.0, late).is_none());
        }

    }
}
