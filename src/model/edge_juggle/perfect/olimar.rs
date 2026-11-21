//! Zero-Pikmin Winged Pikmin flight measured by the group 3 stock replay.

use super::{niche::pad, Correction, Outline, Point, RouteFacts, JUMP, SPECIAL};

pub const KIND: i32 = 0x2b;
pub const FALL: i32 = 0xe;
pub const JUMP_AERIAL: i32 = 0xc;
pub const FALL_AERIAL: i32 = 0xf;
const WING_START: i32 = 0x1de;
const WING_FLY: i32 = 0x1e5;
const AIR_SPEED: f32 = 0.861;
const AIR_ACCEL: f32 = 0.08;
const AIR_BRAKE: f32 = 0.015;
const GRAVITY: f32 = 0.068;
const TERMINAL: f32 = 1.35;
const JUMP_VY: f32 = 2.1005;
const WING_VY: f32 = 1.732;
const WING_START_ROWS: u32 = 20;
const FLY_SPEED: f32 = 1.65;
const FLY_ACCEL: f32 = 0.04;
const FLY_SINK: f32 = 0.8;
const FLY_LIFT: f32 = 1.1;
const FLY_LIFT_ACCEL: f32 = 0.1;
const CLIMB_STICK: f32 = 85.0 / 120.0;
const MAX_FLY_ROWS: u32 = 104;
const TURN_ROWS: u32 = 24;
const BODY: f32 = 13.0;
const HALF: f32 = 3.0;

// OL_DJ_FLY_P0_RING, measured zero-Pikmin routes. Each seed records the
// drift height, jump column, and jump-to-special duration at the cell center.
const RING: [u32; 44] = [
    0x00000000, 0x0001fffc, 0x0000fffc, 0x00007ffc, 0x00007ffc, 0x00003ffc, 0x00001ffc, 0x00001ffc,
    0x00000ffc, 0x00000ffc, 0x000007fc, 0x000003fc, 0x000003fc, 0x000001fc, 0x000000fc, 0x000000fc,
    0x0000007c, 0x0000003c, 0x0000003c, 0x0000001c, 0x0000000c, 0x0000000c, 0x00000004, 0x00000004,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000,
];
const SEEDS: [[i8; 3]; 175] = [
    [-40, 40, 38],
    [-32, 40, 38],
    [-24, 40, 38],
    [-20, 40, 38],
    [-12, 40, 38],
    [-16, 20, 54],
    [0, 40, 38],
    [4, 40, 38],
    [4, 30, 46],
    [16, 40, 38],
    [20, 30, 46],
    [20, 20, 54],
    [32, 30, 54],
    [32, 30, 46],
    [32, 20, 54],
    [-40, 40, 38],
    [-32, 40, 38],
    [-24, 40, 38],
    [-20, 40, 38],
    [-12, 40, 38],
    [-16, 20, 54],
    [0, 40, 38],
    [4, 40, 38],
    [4, 30, 46],
    [16, 40, 38],
    [20, 30, 46],
    [20, 20, 54],
    [28, 30, 46],
    [28, 20, 54],
    [-40, 40, 38],
    [-32, 40, 38],
    [-24, 40, 38],
    [-20, 40, 38],
    [-12, 40, 38],
    [-16, 20, 54],
    [0, 40, 38],
    [4, 40, 38],
    [4, 30, 46],
    [16, 40, 38],
    [20, 30, 46],
    [20, 20, 54],
    [24, 20, 54],
    [-40, 40, 38],
    [-32, 40, 38],
    [-24, 40, 38],
    [-20, 40, 38],
    [-12, 40, 38],
    [-16, 20, 54],
    [0, 40, 38],
    [4, 40, 38],
    [4, 30, 46],
    [16, 40, 38],
    [20, 30, 46],
    [20, 20, 54],
    [20, 20, 54],
    [-40, 40, 38],
    [-32, 40, 38],
    [-24, 40, 38],
    [-20, 40, 38],
    [-12, 40, 38],
    [-16, 20, 54],
    [0, 40, 38],
    [4, 40, 38],
    [4, 30, 46],
    [16, 40, 38],
    [16, 30, 46],
    [16, 20, 54],
    [-40, 40, 38],
    [-32, 40, 38],
    [-24, 40, 38],
    [-20, 40, 38],
    [-12, 40, 38],
    [-16, 20, 54],
    [0, 40, 38],
    [4, 40, 38],
    [4, 30, 46],
    [8, 20, 54],
    [12, 20, 54],
    [-40, 40, 38],
    [-32, 40, 38],
    [-24, 40, 38],
    [-20, 40, 38],
    [-12, 40, 38],
    [-16, 20, 54],
    [0, 40, 38],
    [4, 40, 38],
    [4, 30, 46],
    [8, 20, 54],
    [8, 20, 54],
    [-40, 40, 38],
    [-32, 40, 38],
    [-24, 40, 38],
    [-20, 40, 38],
    [-12, 40, 38],
    [-16, 20, 54],
    [0, 40, 38],
    [4, 40, 38],
    [4, 30, 46],
    [4, 20, 54],
    [-40, 40, 38],
    [-32, 40, 38],
    [-24, 40, 38],
    [-20, 40, 38],
    [-12, 40, 38],
    [-16, 20, 54],
    [0, 40, 38],
    [0, 30, 54],
    [0, 20, 54],
    [0, 20, 54],
    [-40, 40, 38],
    [-32, 40, 38],
    [-24, 40, 38],
    [-20, 40, 38],
    [-12, 40, 38],
    [-16, 20, 54],
    [-12, 20, 54],
    [-4, 30, 46],
    [-4, 20, 54],
    [-40, 40, 38],
    [-32, 40, 38],
    [-24, 40, 38],
    [-20, 40, 38],
    [-12, 40, 38],
    [-16, 20, 54],
    [-12, 20, 54],
    [-8, 20, 54],
    [-40, 40, 38],
    [-32, 40, 38],
    [-24, 40, 38],
    [-20, 40, 38],
    [-12, 40, 38],
    [-16, 20, 54],
    [-12, 20, 54],
    [-12, 20, 54],
    [-40, 40, 38],
    [-32, 40, 38],
    [-24, 40, 38],
    [-20, 40, 38],
    [-20, 30, 54],
    [-16, 20, 54],
    [-16, 20, 54],
    [-40, 40, 38],
    [-32, 40, 38],
    [-24, 40, 38],
    [-20, 40, 38],
    [-20, 30, 54],
    [-20, 20, 54],
    [-40, 40, 38],
    [-32, 40, 38],
    [-24, 40, 38],
    [-28, 30, 46],
    [-24, 20, 54],
    [-24, 20, 54],
    [-40, 40, 38],
    [-32, 40, 38],
    [-28, 40, 38],
    [-28, 30, 46],
    [-28, 20, 54],
    [-40, 40, 38],
    [-32, 40, 38],
    [-32, 30, 54],
    [-32, 20, 54],
    [-40, 40, 38],
    [-40, 30, 46],
    [-36, 20, 54],
    [-36, 20, 54],
    [-40, 40, 38],
    [-40, 30, 46],
    [-40, 20, 54],
    [-48, 20, 54],
    [-44, 20, 54],
    [-48, 20, 54],
    [-48, 20, 54],
    [-52, 20, 54],
    [-56, 20, 46],
];

#[derive(Clone, Copy)]
pub struct OlimarBeat {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub jumps: u8,
    pub pikmin: Option<u8>,
    pub can_jump: bool,
    pub can_rise: bool,
    pub facing: f32,
}

#[derive(Clone, Copy)]
struct Plan {
    descend_to: f32,
    jump_at: f32,
    jump_rows: u32,
    climb_at: f32,
}

#[derive(Clone, Copy)]
struct Pose {
    position: Point,
    velocity: Point,
}

#[derive(Clone, Copy)]
enum Phase {
    Descend { frames: u8 },
    Drift,
    Jump { press: u32 },
    Wing { press: u32 },
    Climb { start: u32 },
}

pub struct OlimarFlight {
    outline: Outline,
    near: Point,
    direction: f32,
    span: f32,
    floor: f32,
    plan: Plan,
    phase: Phase,
    frames: u16,
    exit: Point,
}

impl OlimarFlight {
    pub fn start(facts: &RouteFacts<'_>, beat: OlimarBeat) -> Option<Self> {
        if facts.fighter != KIND
            || beat.status != FALL
            || beat.pikmin != Some(0)
            || beat.jumps != 1
            || !beat.can_rise
            || !(0.0 < facts.body_clearance && facts.body_clearance <= BODY)
            || !beat.position.into_iter().chain(beat.velocity).all(f32::is_finite)
            || beat.velocity[0].abs() > AIR_ACCEL
            || !(-TERMINAL - 0.001..=0.0).contains(&beat.velocity[1])
            || facts.crossing_band().is_none()
        {
            return None;
        }
        let [left, right] = facts.ledges;
        let (near, far, direction) = if beat.position[0] < left[0] {
            (left, right, 1.0)
        } else {
            (right, left, -1.0)
        };
        let span = (far[0] - near[0]) * direction;
        let bottom = facts.blast[3];
        let hull = facts.outline.iter().flatten().map(|p| p[1]).reduce(f32::min)?;
        if beat.facing != direction
            || (span - 159.975).abs() > 1.0
            || (far[1] - near[1]).abs() > 0.5
            || (hull - near[1] + 40.1).abs() > 1.0
            || (bottom - near[1] + 140.0).abs() > 1.0
        {
            return None;
        }
        let outside = (near[0] - beat.position[0]) * direction;
        let height = beat.position[1] - near[1];
        if !super::region::contains(&RING, outside, height) {
            return None;
        }
        let column = (outside / 4.0) as u32;
        let row = ((40.0 - height) / 4.0) as usize;
        let index = RING[..row].iter().map(|m| m.count_ones()).sum::<u32>()
            + (RING[row] & ((1 << column) - 1)).count_ones();
        let [descend_to, jump_at, jump_rows] = SEEDS[index as usize];
        let mut outline = Outline::new();
        for &line in facts.outline {
            outline.add(line);
        }
        let mut flight = Self {
            outline,
            near,
            direction,
            span,
            floor: bottom - near[1] + 12.0,
            plan: Plan {
                descend_to: f32::from(descend_to),
                jump_at: f32::from(jump_at),
                jump_rows: jump_rows as u32,
                climb_at: 120.0,
            },
            phase: Phase::Descend { frames: 0 },
            frames: 0,
            exit: far,
        };
        let pose = flight.pose(beat);
        for descend_offset in [0.0, -1.0, 1.0, -2.0, 2.0] {
            flight.plan.descend_to = f32::from(descend_to) + descend_offset;
            for climb in [120.0, 116.0, 124.0] {
                flight.plan.climb_at = climb;
                if let Some((exit, frames)) = flight.predict(pose) {
                    flight.exit = flight.world(exit);
                    flight.frames = frames as u16;
                    return Some(flight);
                }
            }
        }
        None
    }

    pub fn exit_point(&self) -> Point {
        self.exit
    }

    pub fn crossing_frames(&self) -> u16 {
        self.frames
    }

    fn pose(&self, beat: OlimarBeat) -> Pose {
        Pose {
            position: [(beat.position[0] - self.near[0]) * self.direction, beat.position[1] - self.near[1]],
            velocity: [beat.velocity[0] * self.direction, beat.velocity[1]],
        }
    }

    fn world(&self, point: Point) -> Point {
        [self.near[0] + point[0] * self.direction, self.near[1] + point[1]]
    }

    fn advance(&self, pose: &mut Pose) -> Option<()> {
        let [x, y] = pose.position;
        let next = [x + pose.velocity[0], y + pose.velocity[1]];
        let clear = super::aerial_clear(
            self.outline.segments(),
            [self.world(pose.position), self.world(next)],
            self.near[1] + self.floor, BODY, HALF,
        );
        pose.position = next;
        clear.then_some(())
    }

    fn predict(&self, mut pose: Pose) -> Option<(Point, u32)> {
        let mut frames = 0;
        while pose.position[1] > self.plan.descend_to {
            if frames == 100 || frames < 8 && Self::near_cliff(pose.position) {
                return None;
            }
            Self::ordinary_fall(&mut pose, false);
            self.advance(&mut pose)?;
            frames += 1;
        }
        while pose.position[0] < self.plan.jump_at {
            if frames == 200 {
                return None;
            }
            Self::ordinary_fall(&mut pose, true);
            self.advance(&mut pose)?;
            if Self::near_cliff(pose.position) {
                return None;
            }
            frames += 1;
        }
        let (exit, tail) = self.after_jump(pose)?;
        Some((exit, frames + tail))
    }

    fn near_cliff([x, y]: Point) -> bool {
        (-16.0..=0.0).contains(&x) && y > -26.0 && y < 0.0
    }

    fn ordinary_fall(pose: &mut Pose, inward: bool) {
        let [vx, vy] = pose.velocity;
        pose.velocity = [Self::air_vx(vx, inward), (vy - GRAVITY).max(-TERMINAL)];
    }

    fn air_vx(vx: f32, inward: bool) -> f32 {
        if inward {
            (vx + AIR_ACCEL).min(AIR_SPEED)
        } else {
            vx.signum() * (vx.abs() - AIR_BRAKE).max(0.0)
        }
    }

    fn after_jump(&self, mut pose: Pose) -> Option<(Point, u32)> {
        Self::ordinary_fall(&mut pose, true);
        self.advance(&mut pose)?;
        let mut vy = JUMP_VY;
        for _ in 1..self.plan.jump_rows {
            pose.velocity = [Self::air_vx(pose.velocity[0], true), vy];
            self.advance(&mut pose)?;
            vy = (vy - GRAVITY).max(-TERMINAL);
        }
        let (exit, tail) = self.after_wing(pose)?;
        Some((exit, self.plan.jump_rows + tail))
    }

    fn after_wing(&self, mut pose: Pose) -> Option<(Point, u32)> {
        Self::ordinary_fall(&mut pose, false);
        self.advance(&mut pose)?;
        for row in 0..WING_START_ROWS - 1 {
            pose.velocity = [Self::air_vx(pose.velocity[0], row != 0), WING_VY - GRAVITY * row as f32];
            self.advance(&mut pose)?;
        }
        let mut rows = 0;
        while pose.position[0] < self.plan.climb_at {
            if rows >= MAX_FLY_ROWS - TURN_ROWS {
                return None;
            }
            pose.velocity = [
                (pose.velocity[0] + FLY_ACCEL).min(FLY_SPEED),
                (pose.velocity[1] - FLY_ACCEL).max(-FLY_SINK),
            ];
            self.advance(&mut pose)?;
            rows += 1;
        }
        let (exit, catch) = self.after_climb(pose, rows)?;
        Some((exit, WING_START_ROWS + catch))
    }

    fn after_climb(&self, mut pose: Pose, rows: u32) -> Option<(Point, u32)> {
        for frame in rows + 1..=MAX_FLY_ROWS {
            pose.velocity = [
                (pose.velocity[0] - FLY_ACCEL * CLIMB_STICK).max(-FLY_SPEED),
                (pose.velocity[1] + FLY_LIFT_ACCEL * CLIMB_STICK).min(FLY_LIFT),
            ];
            self.advance(&mut pose)?;
            let [x, y] = pose.position;
            let out = x - self.span;
            if out > 0.0 && y >= -20.3 {
                return (frame - rows >= TURN_ROWS
                    && frame < MAX_FLY_ROWS
                    && (4.5..=11.0).contains(&out)
                    && y <= -13.4)
                    .then_some((pose.position, frame));
            }
        }
        None
    }

    pub fn step(&mut self, beat: OlimarBeat) -> Option<Correction> {
        let pose = self.pose(beat);
        if beat.pikmin != Some(0)
            || !pose.position.into_iter().chain(pose.velocity).all(f32::is_finite)
            || pose.position[1] <= self.floor
        {
            return None;
        }
        let now = beat.frame;
        let forward = [self.direction, 0.0];
        let up = [0.0, 1.0];
        let climb = [-self.direction * CLIMB_STICK, CLIMB_STICK];
        if let Phase::Descend { frames } = self.phase {
            if beat.status != FALL || beat.jumps != 1 || !beat.can_rise || beat.facing != self.direction {
                return None;
            }
            if pose.position[1] > self.plan.descend_to {
                if frames >= 100 || frames < 8 && Self::near_cliff(pose.position) {
                    return None;
                }
                self.phase = Phase::Descend { frames: frames + 1 };
                return Some(pad([0.0, -(f32::from(frames + 1) / 8.0).min(1.0)], 0));
            }
            self.phase = Phase::Drift;
        }
        match self.phase {
            Phase::Descend { .. } => None,
            Phase::Drift if beat.status == FALL && beat.jumps == 1 && beat.can_rise => {
                if pose.position[0] < self.plan.jump_at {
                    return (!Self::near_cliff(pose.position)).then(|| pad(forward, 0));
                }
                if !beat.can_jump {
                    return None;
                }
                self.after_jump(pose)?;
                self.phase = Phase::Jump { press: now };
                Some(pad(forward, JUMP))
            }
            Phase::Jump { press } if matches!(beat.status, JUMP_AERIAL | FALL_AERIAL) => {
                let elapsed = now.wrapping_sub(press);
                if elapsed < self.plan.jump_rows {
                    return Some(pad(forward, 0));
                }
                if elapsed != self.plan.jump_rows || !beat.can_rise {
                    return None;
                }
                self.after_wing(pose)?;
                self.phase = Phase::Wing { press: now };
                Some(pad(up, SPECIAL))
            }
            Phase::Wing { press } => {
                let elapsed = now.wrapping_sub(press);
                if beat.status == WING_START && elapsed < WING_START_ROWS {
                    return Some(if elapsed == 1 { pad(up, SPECIAL) } else { pad(forward, 0) });
                }
                if beat.status != WING_FLY || elapsed < WING_START_ROWS {
                    return None;
                }
                let rows = elapsed - WING_START_ROWS;
                if rows >= MAX_FLY_ROWS - TURN_ROWS {
                    return None;
                }
                if pose.position[0] < self.plan.climb_at {
                    return Some(pad(forward, 0));
                }
                self.after_climb(pose, rows)?;
                self.phase = Phase::Climb { start: now };
                Some(pad(climb, 0))
            }
            Phase::Climb { start }
                if beat.status == WING_FLY && now.wrapping_sub(start) < MAX_FLY_ROWS =>
            {
                Some(pad(climb, 0))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
lib_unit_tests! {
    mod tests {
        use super::*;
        use super::super::{Segment, JUMP, SPECIAL};

        const RING: [Segment; 8] = [
            [[-79.986, 0.0], [-79.986, -8.319]],
            [[-79.986, -8.319], [-64.395, -25.111]],
            [[-64.395, -25.111], [-41.396, -40.099]],
            [[-41.396, -40.099], [41.416, -40.118]],
            [[41.416, -40.118], [64.387, -25.103]],
            [[64.387, -25.103], [79.989, -8.319]],
            [[79.989, -8.319], [79.989, 0.0]],
            [[79.989, 0.0], [-79.986, 0.0]],
        ];
        const RELEASE: Point = [-105.986, -26.0];
        const BEAT: OlimarBeat = OlimarBeat {
            frame: 75, status: 0xe, position: RELEASE, velocity: [0.0, -0.068],
            jumps: 1, pikmin: Some(0), can_jump: true, can_rise: true, facing: 1.0,
        };

        fn facts(stage: (i32, u32), position: Point, outline: &[Segment]) -> RouteFacts<'_> {
            RouteFacts {
                fighter: 0x2b, stage, position, player: [-77.0, 0.0], player_reach: 20.0,
                ledges: [[-79.986, 0.0], [79.989, 0.0]], blast: [-180.0, 180.0, 180.0, -140.0],
                outline, body_clearance: 11.0, ledge_distance: 11.0, fall_budget: 1.35,
                reach: [175.0, 88.0],
            }
        }

        #[test]
        fn olimar_selects_only_the_measured_empty_line_on_the_three_normal_stages() {
            for stage in [0, 3, 311] {
                let facts = facts((stage, 0), RELEASE, &RING);
                let mut flight = OlimarFlight::start(&facts, BEAT).expect("inside the proven zero-Pikmin region");
                assert!(flight.step(BEAT).is_some());
                for count in [None, Some(1), Some(2), Some(3), Some(4)] {
                    assert!(OlimarFlight::start(&facts, OlimarBeat { pikmin: count, ..BEAT }).is_none());
                }
                assert!(OlimarFlight::start(&facts, OlimarBeat { jumps: 0, ..BEAT }).is_none());
                assert!(OlimarFlight::start(&facts, OlimarBeat { can_rise: false, ..BEAT }).is_none());
            }
            for stage in [(3, 1), (89, 0)] {
                assert!(OlimarFlight::start(&facts(stage, RELEASE, &RING), BEAT).is_none());
            }
            let outside = [-159.986, 30.0];
            assert!(OlimarFlight::start(
                &facts((3, 0), outside, &RING),
                OlimarBeat { position: outside, ..BEAT },
            ).is_none());
        }

        #[test]
        fn olimar_reaches_the_far_catch_from_each_traced_inside_cell() {
            let mirrored = RING.map(|line| line.map(|[x, y]| [-x, y]));
            for (out, y) in [(10.0, 30.0), (62.0, 34.0), (46.0, 2.0), (26.0, -26.0),
                (14.0, -46.0), (10.0, -54.0), (30.0, -22.0)]
            {
                for facing in [1.0, -1.0] {
                    let position = [(-79.986 - out) * facing, y];
                    let mut facts = facts((3, 0), position, if facing == 1.0 { &RING } else { &mirrored });
                    if facing < 0.0 {
                        facts.ledges = [[-79.989, 0.0], [79.986, 0.0]];
                    }
                    for fall in [-0.068, -TERMINAL] {
                        let beat = OlimarBeat { position, velocity: [0.0, fall], facing, ..BEAT };
                        let flight = OlimarFlight::start(&facts, beat)
                            .unwrap_or_else(|| panic!("no finish for {out} out, y={y}, vy={fall}, facing={facing}"));
                        assert!((84.4..=91.0).contains(&(flight.exit_point()[0] * facing)));
                    }
                }
            }
        }

        #[test]
        fn olimar_traces_the_measured_drift_jump_and_flight_to_the_far_ledge() {
            let facts = facts((3, 0), RELEASE, &RING);
            let mut flight = OlimarFlight::start(&facts, BEAT).expect("bounded plan");
            assert!((180..=270).contains(&flight.crossing_frames()));
            assert!((84.4..=91.0).contains(&flight.exit_point()[0]));

            // a7018c95, g3-p4fd-yoga, pikmin-p4-fd-in-26-m26-1.
            flight.plan = Plan {
                descend_to: -26.0, jump_at: 20.0, jump_rows: 54, climb_at: 120.0,
            };
            let back = [-CLIMB_STICK, CLIMB_STICK];
            let forward = [1.0, 0.0];
            let up = [0.0, 1.0];
            let rows = [
                (40, FALL, [-105.986, -27.474], [0.0, -1.35], 1, forward, 0),
                (99, FALL, [-60.258, -107.124], [0.861, -1.35], 1, forward, 0),
                (100, FALL, [-59.397, -108.474], [0.861, -1.35], 1, forward, JUMP),
                (101, JUMP_AERIAL, [-58.536, -109.824], [0.861, -1.35], 0, forward, 0),
                (150, FALL_AERIAL, [-16.347, -86.868], [0.861, -1.164], 0, forward, 0),
                (154, FALL_AERIAL, [-12.903, -92.099], [0.861, -1.35], 0, up, SPECIAL),
                (155, WING_START, [-12.057, -93.449], [0.846, -1.35], 0, up, SPECIAL),
                (156, WING_START, [-11.226, -91.717], [0.831, 1.732], 0, forward, 0),
                (174, WING_FLY, [4.272, -72.169], [0.861, 0.508], 0, forward, 0),
                (200, WING_FLY, [39.781, -73.001], [1.65, -0.532], 0, forward, 0),
                (201, WING_FLY, [41.431, -73.573], [1.65, -0.572], 0, back, 0),
                (266, WING_FLY, [87.906, -20.980], [-0.192, 1.1], 0, back, 0),
            ];
            for (frame, status, position, velocity, jumps, stick, buttons) in rows {
                let beat = OlimarBeat { frame, status, position, velocity, jumps, ..BEAT };
                let edit = flight.step(beat).unwrap_or_else(|| panic!("no pad at stock row {frame}"));
                assert_eq!(edit.stick, Some(stick), "stock row {frame}");
                assert_eq!(edit.press & (JUMP | SPECIAL), buttons, "stock row {frame}");
            }
            let beat = OlimarBeat { frame: 267, status: 0x75, position: [91.236, -20.078], ..BEAT };
            assert!(flight.step(beat).is_none(), "the ledge ends the flight");
            assert!(flight.step(OlimarBeat { status: 0x4b, ..beat }).is_none());
        }
    }
}
