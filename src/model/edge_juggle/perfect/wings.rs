//! Native finishing moves for Pit, Dark Pit and Meta Knight's measured crossings.

use super::{aerial_clear, fly_launch, niche::pad, Correction, Outline, Point, RouteFacts, JUMP, SPECIAL};

#[derive(Clone, Copy)]
enum Move {
    PowerOfFlight,
    ShuttleLoop,
}

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
    movement: Move,
    started: u32,
}

impl Finisher {
    fn start(movement: Move, far: Point, inward: f32, entry: Entry) -> Option<Self> {
        if !entry.can_special
            || entry.remaining != 0
            || !matches!(inward, -1.0 | 1.0)
            || !far.into_iter().chain(entry.position).chain(entry.velocity).all(f32::is_finite)
        {
            return None;
        }
        let outside = (entry.position[0] - far[0]) * inward;
        let height = entry.position[1] - far[1];
        let reaches = match movement {
            Move::PowerOfFlight => {
                // The fourteen start rows carry 8.9 times the entering horizontal speed.
                let after_start = outside + FLIGHT_CARRY * entry.velocity[0] * inward;
                entry.facing == inward
                    && (5.0..=8.5).contains(&after_start)
                    && (-120.0..=-40.0).contains(&height)
            }
            Move::ShuttleLoop => {
                // Only turn-jump-apex entries were measured at this height.
                entry.facing == -inward
                    && (0.0..=0.11).contains(&entry.velocity[1])
                    && (LOOP_MIN_HEIGHT..=LOOP_MAX_HEIGHT).contains(&height)
            }
        };
        reaches.then_some(Self { movement, started: entry.frame })
    }

    fn step(&self, frame: u32, status: i32) -> Option<Correction> {
        let age = frame.checked_sub(self.started)?;
        match (self.movement, age, status) {
            (_, 0, 0xd..=0xf) => Some(pad([0.0, 1.0], SPECIAL)),
            (Move::PowerOfFlight, 1..=67, 0x1de | 0x1e7) => Some(pad([0.0, 1.0], 0)),
            (Move::ShuttleLoop, 1..=63, 0x1de | 0x1e8) => Some(pad([0.0, 0.0], 0)),
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

struct Profile {
    movement: Move,
    gravity: f32,
    fall: f32,
    speed: f32,
    brake: f32,
    head: f32,
    jump_vx: f32,
    gap: u32,
    jumps: u8,
    catch: [f32; 3],
    ring: [u32; 11],
}

const META: Profile = Profile {
    movement: Move::ShuttleLoop,
    gravity: 0.11,
    fall: 1.66,
    speed: 1.04,
    brake: 0.012,
    head: 9.0,
    jump_vx: 0.862,
    gap: 26,
    jumps: 5,
    catch: [-17.0, -27.0, -8.0],
    // 32 cells from metaknight-ring.txt. The retained cells all leave room
    // for the backward fifth jump and the first Shuttle Loop catch.
    ring: [0, 0xfc0, 0x7c0, 0x7c0, 0x3c0, 0x3c0, 0x1c0, 0xc0, 0xc0, 0x40, 0],
};
const PIT: Profile = Profile {
    movement: Move::PowerOfFlight,
    gravity: 0.081,
    fall: 1.48,
    speed: 0.935,
    brake: 0.008,
    head: 16.5,
    jump_vx: 0.81,
    gap: 31,
    jumps: 3,
    catch: [-18.0, -27.0, -9.0],
    ring: [0, 0x1fc0, 0xfc0, 0x7c0, 0x7c0, 0x3c0, 0x3c0, 0x1c0, 0xc0, 0xc0, 0x40],
};

const FLIGHT_CARRY: f32 = 8.9;
const HALF: f32 = 2.0;
const DRIFT: f32 = 0.075;
const JUMP_DRIFT: f32 = 0.062;
const META_DRIFT_CHECK_FRAMES: u32 = 10;
const META_CLEAR_DRIFT: f32 = 0.70;
const LOOP_MIN_HEIGHT: f32 = -77.68;
const LOOP_MAX_HEIGHT: f32 = -75.0;
pub const META_KNIGHT: i32 = 0x1d;

enum Phase {
    Descend,
    Bounce,
    Turned,
    Finish(Finisher),
}

enum DriftCheck {
    Waiting,
    SteeringSince(u32),
    Ready,
}

pub struct WingsFlight {
    profile: &'static Profile,
    fighter: i32,
    hull: Outline,
    near: Point,
    far: Point,
    bottom: f32,
    flat: f32,
    inward: f32,
    phase: Phase,
    jump: Option<(u32, f32)>,
    drift_check: DriftCheck,
}

impl WingsFlight {
    pub fn start(facts: &RouteFacts<'_>, live: Sample) -> Option<Self> {
        let profile = match facts.fighter {
            META_KNIGHT => &META,
            0x1e | 0x1f => &PIT,
            _ => return None,
        };
        let entry = live.entry;
        if !super::is_crossing_stage(facts.stage)
            || live.status != 0xe
            || entry.remaining != profile.jumps
            || !entry.can_special
            || !entry.position.into_iter().chain(entry.velocity).all(f32::is_finite)
            || entry.velocity[0] != 0.0
            || !(-profile.fall..=0.0).contains(&entry.velocity[1])
            || !(facts.body_clearance > 0.0 && facts.body_clearance <= profile.head)
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
            || !super::region::contains(&profile.ring, outside, height)
        {
            return None;
        }
        let mut hull = Outline::new();
        for &line in facts.outline {
            hull.add(line);
        }
        Some(Self {
            profile, fighter: facts.fighter, hull, near, far, bottom, flat, inward,
            phase: Phase::Descend, jump: None,
            drift_check: if matches!(profile.movement, Move::ShuttleLoop) {
                DriftCheck::Waiting
            } else {
                DriftCheck::Ready
            },
        })
    }

    pub fn checking_drift(&self) -> bool {
        !matches!(self.drift_check, DriftCheck::Ready)
    }

    pub fn exit_point(&self) -> Point {
        self.far
    }

    pub const fn crossing_frames(&self) -> u16 {
        330
    }

    fn fall(&self, vy: f32) -> f32 {
        (vy - self.profile.gravity).max(-self.profile.fall)
    }

    fn clear(&self, from: Point, to: Point) -> bool {
        aerial_clear(self.hull.segments(), [from, to], self.bottom + 6.0, self.profile.head + 1.0, HALF)
    }

    fn near_catch(&self, [x, y]: Point) -> bool {
        let [inbound, low, top] = self.profile.catch;
        (inbound..=12.0).contains(&((x - self.near[0]) * self.inward))
            && (low..=top).contains(&(y - self.near[1]))
    }

    fn descent_target(&self) -> f32 {
        self.flat - self.profile.head - 2.0
    }

    fn can_glide(&self, mut pos: Point, [vx, mut vy]: Point) -> bool {
        let mut forward = vx * self.inward;
        for _ in 0..160 {
            forward = (forward + DRIFT).min(self.profile.speed);
            vy = self.fall(vy);
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
        if roof.is_finite() { roof - self.profile.head - 2.0 } else { flat }
    }

    fn jump_clears(&self, entry: Entry, launch: f32, steer: f32) -> bool {
        let [x, y] = entry.position;
        let [vx, vy] = entry.velocity;
        let carry_x = (vx + steer * JUMP_DRIFT).clamp(-self.profile.speed, self.profile.speed);
        let mut pos = [x + carry_x, y + self.fall(vy)];
        if !self.clear(entry.position, pos) {
            return false;
        }
        let mut rise = launch - self.profile.gravity;
        let mut drift = steer * self.profile.jump_vx;
        while rise > 0.0 {
            let next = [pos[0] + drift, pos[1] + rise];
            if !self.clear(pos, next) || (steer == self.inward && next[1] > self.ceiling(next[0])) {
                return false;
            }
            pos = next;
            rise -= self.profile.gravity;
            // The inward bounce keeps steering; the turn releases after the launch.
            drift = if steer == self.inward {
                (drift + steer * JUMP_DRIFT).clamp(-self.profile.speed, self.profile.speed)
            } else {
                drift.signum() * (drift.abs() - self.profile.brake).max(0.0)
            };
        }
        true
    }

    fn start_finish(&mut self, live: Sample) -> Option<Correction> {
        let flight = Finisher::start(self.profile.movement, self.far, self.inward, live.entry)?;
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
            || entry.remaining > self.profile.jumps
            || !self.clear(entry.position, entry.position)
        {
            return None;
        }
        if let Some((frame, steer)) = self.jump {
            if entry.frame.saturating_sub(frame) < 2 {
                return Some(pad([steer, 0.0], JUMP));
            }
        }
        let [x, y] = entry.position;
        let [vx, vy] = entry.velocity;
        if matches!(self.phase, Phase::Descend) {
            if self.near_catch(entry.position) {
                return None;
            }
            let descending = y + self.fall(vy) > self.descent_target();
            let glide = descending && self.can_glide(entry.position, entry.velocity);
            if self.checking_drift() {
                // A cut Shuttle Loop can leave Fall capped at 0.520 or 0.624.
                // The clean Fall reaches 0.750 on its tenth inward step.
                if live.status != 0xe || entry.remaining != self.profile.jumps || !entry.can_special {
                    return None;
                }
                if vx * self.inward >= META_CLEAR_DRIFT {
                    self.drift_check = DriftCheck::Ready;
                } else {
                    match self.drift_check {
                        _ if !descending => return None,
                        DriftCheck::SteeringSince(first)
                            if entry.frame.saturating_sub(first) >= META_DRIFT_CHECK_FRAMES => return None,
                        DriftCheck::Waiting if glide => self.drift_check = DriftCheck::SteeringSince(entry.frame),
                        _ => {}
                    }
                }
            }
            if descending {
                let steer = if glide { self.inward } else { 0.0 };
                return Some(pad([steer, 0.0], 0));
            }
            self.phase = Phase::Bounce;
        }
        let outside = (x - self.far[0]) * self.inward;
        let power = matches!(self.profile.movement, Move::PowerOfFlight);
        if power && outside + FLIGHT_CARRY * vx * self.inward >= 5.0 {
            return self.start_finish(live);
        }
        if matches!(self.phase, Phase::Turned) {
            return if vy <= self.profile.gravity {
                self.start_finish(live)
            } else {
                Some(pad([0.0, 0.0], 0))
            };
        }
        let launch = (live.can_jump && vy <= 0.0
            && self.jump.is_none_or(|(frame, _)| entry.frame.saturating_sub(frame) >= self.profile.gap))
            .then(|| fly_launch(self.fighter, entry.remaining))
            .flatten();
        if !power && outside >= 6.0 {
            if let Some(launch) = launch.filter(|_| entry.remaining == 1) {
                let mut speed = launch - self.profile.gravity;
                let mut rise = 0.0;
                while speed > 0.0 {
                    rise += speed;
                    speed -= self.profile.gravity;
                }
                let apex = y + self.fall(vy) + rise - self.far[1];
                if apex < LOOP_MIN_HEIGHT {
                    return None;
                }
                if y + rise <= self.far[1] - 73.5 && self.jump_clears(entry, launch, -self.inward) {
                    self.phase = Phase::Turned;
                    self.jump = Some((entry.frame, -self.inward));
                    return Some(pad([-self.inward, 0.0], JUMP));
                }
            }
            return Some(pad([0.0, 0.0], 0));
        }
        let bounce = (power || entry.remaining > 1) && outside < -4.0
            && launch.is_some_and(|launch| self.jump_clears(entry, launch, self.inward));
        if bounce {
            self.jump = Some((entry.frame, self.inward));
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
                body_clearance: if fighter == 0x1d { 9.0 } else { 16.5 },
                ledge_distance: 15.0,
                fall_budget: if fighter == 0x1d { 1.66 } else { 1.48 },
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
        fn wing_routes_start_in_their_baked_cells_with_the_measured_jump_reserve() {
            for (fighter, jumps) in [(0x1d, 5), (0x1e, 3), (0x1f, 3)] {
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

        #[test]
        fn meta_knight_uses_his_measured_region_and_backward_turn_before_shuttle_loop() {
            for stage in [(3, 0), (0, 0), (311, 0)] {
                for side in [-1.0, 1.0] {
                    let lip = if side < 0.0 { LEFT } else { RIGHT };
                    let live = falling(lip[0] + side * 30.0, 6.0, 5);
                    let route = facts(0x1d, live.entry.position, stage);
                    assert!(WingsFlight::start(&route, live).is_some(),
                        "normal stage {stage:?}, side {side}");
                    let mut spent = live;
                    spent.entry.remaining = 4;
                    assert!(WingsFlight::start(&route, spent).is_none());
                    let wrong_height = falling(lip[0] + side * 30.0, -2.0, 5);
                    assert!(WingsFlight::start(&route, wrong_height).is_none());
                }
            }
        }

        #[test]
        fn meta_knight_does_not_commit_with_halved_fall_drift() {
            // Product FD attempts 14 (fg1-pp-fd-8) and 21 (fg1-pp-fd-10)
            // released at the same point and produced the same starting pads.
            let start = falling(-109.9856, 5.8902593, 5);
            let route = facts(0x1d, start.entry.position, (3, 0));
            for (x, vx, expected) in [(-105.860603, 0.75, true), (-106.330620, 0.52, false)] {
                let mut flight = WingsFlight::start(&route, start).unwrap();
                assert_eq!(flight.step(start), Some(pad([0.0, 0.0], 0)));
                let mut first_drift = start;
                first_drift.entry.position[1] = 2.0402594;
                first_drift.entry.velocity[1] = -0.88;
                first_drift.entry.frame = 8;
                assert_eq!(flight.step(first_drift), Some(pad([1.0, 0.0], 0)));
                assert!(flight.checking_drift(), "the initial drift alone cannot select a route");
                let mut live = first_drift;
                live.entry.position = [x, -12.179741];
                live.entry.velocity = [vx, -1.66];
                live.entry.frame = 18;
                assert_eq!(flight.step(live).is_some(), expected, "live fall drift {vx}");
                assert_eq!(flight.checking_drift(), !expected);
            }
        }

        #[test]
        fn meta_knight_requires_drift_past_the_retained_shuttle_loop_cap() {
            let start = falling(-109.9856, 5.8902593, 5);
            let route = facts(0x1d, start.entry.position, (3, 0));
            let mut flight = WingsFlight::start(&route, start).unwrap();
            let mut live = start;
            live.entry.position[1] = 2.0402594;
            live.entry.velocity[1] = -0.88;
            live.entry.frame = 8;
            assert_eq!(flight.step(live), Some(pad([1.0, 0.0], 0)));
            // A clean Fall and a Fall capped by the prior special still
            // look the same after eight inward pads (fg1-pp-fd-8).
            live.entry.position = [-107.2856, -8.859741];
            live.entry.velocity = [0.6, -1.66];
            live.entry.frame = 16;
            assert!(flight.step(live).is_some());
            assert!(flight.checking_drift(), "0.600 cannot distinguish the two falls");
            live.entry.frame = 18;
            live.entry.position = [-106.037605, -12.179741];
            live.entry.velocity = [0.624001, -1.66];
            assert_eq!(flight.step(live), None);
        }

        #[test]
        fn meta_knight_only_spends_the_turn_jump_for_a_reachable_apex() {
            let start = falling(-109.9856, 5.8902593, 5);
            let route = facts(0x1d, start.entry.position, (3, 0));
            for (position, vx, expected) in [
                ([90.848, -87.2], 0.992, Some(pad([-1.0, 0.0], JUMP))),
                ([86.237, -110.02], 1.04, None),
            ] {
                let mut flight = WingsFlight::start(&route, start).unwrap();
                flight.phase = Phase::Bounce;
                let mut live = start;
                live.entry.position = position;
                live.entry.velocity = [vx, -1.66];
                live.entry.remaining = 1;
                live.entry.frame = 223;
                assert_eq!(flight.step(live), expected, "live turn from {position:?}");
            }
        }

        fn run_to_finish(fighter: i32, mut live: Sample) -> (u32, Entry) {
            let route = facts(fighter, live.entry.position, (3, 0));
            let mut flight = WingsFlight::start(&route, live).unwrap();
            let start = live.entry;
            let profile = flight.profile;
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
                    live.entry.velocity = [steer * profile.jump_vx,
                        fly_launch(fighter, live.entry.remaining).unwrap() - profile.gravity];
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
                        vx.signum() * (vx.abs() - profile.brake).max(0.0)
                    } else {
                        (*vx + steer * accel).clamp(-profile.speed, profile.speed)
                    };
                    *vy = (*vy - profile.gravity).max(-profile.fall);
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
            for (fighter, profile) in [(0x1d, &META), (0x1e, &PIT)] {
                for (row, mask) in profile.ring.into_iter().enumerate() {
                    for col in 0..30 {
                        if mask & (1 << col) == 0 {
                            continue;
                        }
                        let out = 4.0 * col as f32 + 2.0;
                        let y = 38.0 - 4.0 * row as f32;
                        for [dx, dy] in [[0.0, 0.0], [-1.95, 0.0], [1.95, 0.0], [0.0, -1.95], [0.0, 1.95]] {
                            for speed in [0.0, -profile.fall * 0.5, -profile.fall] {
                                let mut live = falling(LEFT[0] - out + dx, y + dy, profile.jumps);
                                live.entry.velocity[1] = speed;
                                let (frame, entry) = run_to_finish(fighter, live);
                                assert!(frame < 330 && entry.remaining == 0);
                            }
                        }
                    }
                }
                let mirrored = falling(RIGHT[0] + 30.0, 6.0, profile.jumps);
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
            let flight = Finisher::start(Move::PowerOfFlight, far, 1.0, entry).unwrap();
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
            assert!(Finisher::start(Move::PowerOfFlight, [-far[0], far[1]], -1.0, mirrored).is_some());
            let fast = Entry { velocity: [1.3, -1.48], ..entry };
            let late = Entry { position: [entry.position[0], -124.0], ..entry };
            assert!(Finisher::start(Move::PowerOfFlight, far, 1.0, fast).is_none());
            assert!(Finisher::start(Move::PowerOfFlight, far, 1.0, late).is_none());
        }

        #[test]
        fn meta_knight_finishes_shuttle_loop_only_after_the_turn_jump_apex() {
            // Battlefield reference metaknight-bf-in3-1, departure seq 191007.
            let far = [79.989, 0.0];
            let entry = Entry {
                position: [80.90212, -76.14011],
                velocity: [-0.694, 0.1000],
                remaining: 0,
                facing: -1.0,
                can_special: true,
                frame: 242,
            };
            let flight = Finisher::start(Move::ShuttleLoop, far, 1.0, entry).unwrap();
            assert_eq!(flight.step(242, 0xd), Some(pad([0.0, 1.0], SPECIAL)));
            assert_eq!(flight.step(243, 0x1de), Some(pad([0.0, 0.0], 0)));
            assert_eq!(flight.step(249, 0x1e8), Some(pad([0.0, 0.0], 0)));
            assert!(flight.step(256, 0x75).is_none());
            for invalid in [
                Entry { facing: 1.0, ..entry },
                Entry { remaining: 1, ..entry },
                Entry { velocity: [-0.7, 0.23], ..entry },
                Entry { can_special: false, ..entry },
                Entry { position: [80.0, -81.0], ..entry },
            ] {
                assert!(Finisher::start(Move::ShuttleLoop, far, 1.0, invalid).is_none());
            }
        }
    }
}
