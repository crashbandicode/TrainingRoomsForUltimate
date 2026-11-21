//! The measured float, double jump and parasol crossing for Peach and Daisy.

use super::{
    aerial_clear, is_crossing_stage, niche::pad, Correction, Outline, Point, RouteFacts, JUMP,
    SPECIAL,
};

const FALL: i32 = 0xe;
const FALL_AERIAL: i32 = 0xf;
const FLOAT_START: i32 = 0x1e8;
const FLOAT: i32 = 0x1e9;
const AERIAL_JUMP: i32 = 0xc;
const PARASOL_RISE: i32 = 0x1de;
const PARASOL_OPEN: i32 = 0x1e4;
const FALL_SPEED: f32 = 1.19;
const GRAVITY: f32 = 0.07;
const AIR_SPEED: f32 = 1.029;
const ACCEL: f32 = 0.12;
const HEAD: f32 = 17.0;
const HALF: f32 = 3.0;
const FLOAT_FRAMES: u32 = 150;
const FLOAT_CUT_OUTSIDE: f32 = 3.0;
const SINK_TARGET: f32 = -67.0;
const JUMP_RISE: f32 = 30.0266;
const PARASOL_BAND: core::ops::RangeInclusive<f32> = -48.0..=-36.0;

// Intersection of the stock route's ring and cells whose two entry speeds and five
// modeled sample points press Parasol at -46 or higher, leaving two units of slack.
const PEACH_CROSS_RING: [u32; 44] = [
    0x00000000, 0x003fffc0, 0x001fffc0, 0x000fffc0, 0x0007ffc0, 0x0003ffc0, 0x0001ffc0, 0x0000ffc0,
    0x00007fc0, 0x00007fc0, 0x00003fc0, 0x00001fc0, 0x00000fc0, 0x000007c0, 0x000003c0, 0x000001c0,
    0x000001e0, 0x000000f0, 0x0000007c, 0x0000003c, 0x0000001c, 0x0000000c, 0x00000004, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000,
];

#[derive(Clone, Copy)]
pub struct FloatBeat {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub facing: f32,
    pub jumps: u8,
    pub can_float: bool,
    pub can_jump: bool,
    pub can_rise: bool,
}

#[derive(Clone, Copy, Debug)]
enum Phase {
    Descend,
    Float(u32),
    Sink,
    ReleaseForJump(u32),
    Jump(u32),
    Parasol(u32),
}

pub struct FloatFlight {
    outline: Outline,
    near: Point,
    far: Point,
    float_y: f32,
    bottom: f32,
    inward: f32,
    phase: Phase,
}

impl FloatFlight {
    pub fn start(facts: &RouteFacts<'_>, beat: FloatBeat) -> Option<Self> {
        let [left, right] = facts.ledges;
        let [near, far] = if beat.position[0] < left[0] {
            [left, right]
        } else {
            [right, left]
        };
        let inward = (far[0] - near[0]).signum();
        let outside = (near[0] - beat.position[0]) * inward;
        let height = beat.position[1] - near[1];
        let flat = facts
            .outline
            .iter()
            .flatten()
            .map(|p| p[1])
            .reduce(f32::min)?;
        let bottom = facts.blast[3] + 6.0;
        if !matches!(facts.fighter, 0xd | 0xe)
            || !is_crossing_stage(facts.stage)
            || !(159.0..=161.0).contains(&(right[0] - left[0]))
            || (near[1] - far[1]).abs() > 0.25
            || !(-40.4..=-39.8).contains(&(flat - near[1]))
            || !(139.0..=141.0).contains(&(near[1] - facts.blast[3]))
            || !(facts.body_clearance > 0.0 && facts.body_clearance <= HEAD)
            || beat.status != FALL
            || beat.facing != inward
            || beat.jumps != 1
            || !beat.can_float
            || !beat.can_jump
            || !beat.can_rise
            || !(-0.01..=0.01).contains(&beat.velocity[0])
            || !(-FALL_SPEED..=0.0).contains(&beat.velocity[1])
            || !super::region::contains(&PEACH_CROSS_RING, outside, height)
        {
            return None;
        }
        let float_y = flat - HEAD - 2.0;
        let path = [[near[0], float_y], [far[0], float_y]];
        if !aerial_clear(facts.outline, path, bottom, HEAD + 1.0, HALF) {
            return None;
        }
        let mut outline = Outline::new();
        for &line in facts.outline {
            outline.add(line);
        }
        Some(Self {
            outline,
            near,
            far,
            float_y,
            bottom,
            inward,
            phase: Phase::Descend,
        })
    }

    pub fn exit_point(&self) -> Point {
        self.far
    }

    pub fn needs_float(&self) -> bool {
        matches!(self.phase, Phase::Descend)
    }

    fn clear(&self, from: Point, to: Point, clearance: f32) -> bool {
        aerial_clear(
            self.outline.segments(),
            [from, to],
            self.bottom,
            clearance,
            HALF,
        )
    }

    fn far_out(&self, point: Point) -> f32 {
        (point[0] - self.far[0]) * self.inward
    }

    fn in_near_box(&self, [x, y]: Point) -> bool {
        (-17.0..=12.0).contains(&((x - self.near[0]) * self.inward))
            && (-25.0..=-7.0).contains(&(y - self.near[1]))
    }

    fn forward_descent_is_clear(&self, mut p: Point, [vx, mut vy]: Point) -> bool {
        let mut vx = vx * self.inward;
        for _ in 0..160 {
            vy = (vy - GRAVITY).max(-FALL_SPEED);
            vx = (vx + ACCEL).min(AIR_SPEED);
            let next = [p[0] + vx * self.inward, p[1] + vy];
            if self.in_near_box(next) || !self.clear(p, next, HEAD + 2.0) {
                return false;
            }
            if next[1] <= self.float_y {
                return true;
            }
            p = next;
        }
        false
    }

    pub fn step(&mut self, beat: FloatBeat) -> Option<Correction> {
        let FloatBeat {
            frame,
            status,
            position,
            velocity,
            jumps,
            can_float,
            can_jump,
            can_rise,
            ..
        } = beat;
        if !position.into_iter().chain(velocity).all(f32::is_finite) || position[1] <= self.bottom {
            return None;
        }
        let outside = self.far_out(position);
        let forward = [self.inward, 0.0];
        let backward = [-self.inward, 0.0];
        let neutral = [0.0; 2];
        let down_jump = pad([0.0, -1.0], JUMP);
        match self.phase {
            Phase::Descend
                if status == FALL && jumps == 1 && can_float && !self.in_near_box(position) =>
            {
                let next_y = position[1] + (velocity[1] - GRAVITY).max(-FALL_SPEED);
                if next_y <= self.float_y {
                    self.phase = Phase::Float(frame);
                    return Some(down_jump);
                }
                let stick = if self.forward_descent_is_clear(position, velocity) {
                    forward
                } else {
                    neutral
                };
                Some(pad(stick, 0))
            }
            Phase::Float(since) if status == FALL && frame == since => Some(down_jump),
            Phase::Float(since) if matches!(status, FLOAT_START | FLOAT) && jumps == 1 => {
                let age = frame.checked_sub(since)?;
                if age < 4 {
                    Some(down_jump)
                } else if outside >= FLOAT_CUT_OUTSIDE || age > FLOAT_FRAMES {
                    self.phase = Phase::Sink;
                    Some(pad(
                        if outside < FLOAT_CUT_OUTSIDE {
                            forward
                        } else {
                            neutral
                        },
                        0,
                    ))
                } else {
                    Some(pad(forward, JUMP))
                }
            }
            Phase::Float(since)
                if matches!(status, FALL | FALL_AERIAL)
                    && frame.saturating_sub(since) >= FLOAT_FRAMES =>
            {
                self.phase = Phase::Sink;
                self.sink(beat, outside)
            }
            Phase::Sink if matches!(status, FALL | FALL_AERIAL) => self.sink(beat, outside),
            Phase::ReleaseForJump(since)
                if matches!(status, FALL | FALL_AERIAL) && jumps == 1 && can_jump && can_rise =>
            {
                if frame == since {
                    return Some(pad(neutral, 0));
                }
                self.phase = Phase::Jump(frame);
                Some(pad(neutral, JUMP))
            }
            Phase::Jump(since) => {
                let age = frame.checked_sub(since)?;
                if age < 2 && matches!(status, FALL | FALL_AERIAL | AERIAL_JUMP) {
                    return Some(pad(neutral, JUMP));
                }
                if !matches!(status, FALL | FALL_AERIAL | AERIAL_JUMP) || jumps != 0 {
                    return None;
                }
                if age <= 30 && velocity[1] > GRAVITY {
                    return Some(pad(neutral, 0));
                }
                if age >= 27
                    && can_rise
                    && (3.0..=9.5).contains(&outside)
                    && PARASOL_BAND.contains(&(position[1] - self.far[1]))
                {
                    self.phase = Phase::Parasol(frame);
                    return Some(pad([0.0, 1.0], SPECIAL));
                }
                None
            }
            Phase::Parasol(since) if frame == since => Some(pad([0.0, 1.0], SPECIAL)),
            Phase::Parasol(since)
                if matches!(status, PARASOL_RISE | PARASOL_OPEN)
                    && frame.saturating_sub(since) <= 120 =>
            {
                Some(pad(backward, 0))
            }
            _ => None,
        }
    }

    fn sink(&mut self, beat: FloatBeat, outside: f32) -> Option<Correction> {
        let height = beat.position[1] - self.far[1];
        // One neutral think and one buffered jump think still fall before the rise.
        let released_vy = (beat.velocity[1] - GRAVITY).max(-FALL_SPEED);
        let buffered_vy = (released_vy - GRAVITY).max(-FALL_SPEED);
        let launch_height = height + released_vy + buffered_vy + JUMP_RISE;
        if beat.jumps != 1 || outside > 9.5 || launch_height < *PARASOL_BAND.start() {
            return None;
        }
        let stick = if outside < FLOAT_CUT_OUTSIDE {
            [self.inward, 0.0]
        } else if height <= SINK_TARGET {
            self.phase = Phase::ReleaseForJump(beat.frame);
            [0.0; 2]
        } else if beat.velocity[0] * self.inward > ACCEL {
            [-self.inward, 0.0]
        } else {
            [0.0; 2]
        };
        Some(pad(stick, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::edge_juggle::perfect::{Segment, JUMP, SPECIAL};

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

    fn facts(fighter: i32, stage: (i32, u32), position: Point) -> RouteFacts<'static> {
        RouteFacts {
            fighter,
            stage,
            position,
            player: [-75.0, 0.0],
            player_reach: 15.0,
            ledges: [LEFT, RIGHT],
            blast: [-200.0, 200.0, 140.0, -140.0],
            outline: &RING,
            body_clearance: 17.0,
            ledge_distance: 10.0,
            fall_budget: 100.0,
            reach: [0.0; 2],
        }
    }

    fn live(position: Point) -> FloatBeat {
        FloatBeat {
            frame: 0,
            status: 0xe,
            position,
            velocity: [0.0; 2],
            facing: 1.0,
            jumps: 1,
            can_float: true,
            can_jump: true,
            can_rise: true,
        }
    }

    #[test]
    fn parasol_crossing_requires_a_proven_cell_and_all_three_resources() {
        let outside = live([-119.986, 20.0]);
        for fighter in [0xd, 0xe] {
            for stage in [(3, 0), (0, 0), (311, 0)] {
                let f = facts(fighter, stage, outside.position);
                assert!(FloatFlight::start(&f, outside).is_some());
                for invalid in [
                    FloatBeat {
                        can_float: false,
                        ..outside
                    },
                    FloatBeat {
                        can_rise: false,
                        ..outside
                    },
                    FloatBeat {
                        jumps: 0,
                        ..outside
                    },
                    FloatBeat {
                        facing: -1.0,
                        ..outside
                    },
                    live([-219.986, 30.0]),
                ] {
                    assert!(FloatFlight::start(&f, invalid).is_none());
                }
            }
            let non_normal = facts(fighter, (311, 1), outside.position);
            assert!(FloatFlight::start(&non_normal, outside).is_none());
        }
    }

    #[test]
    fn parasol_follows_the_native_float_cut_jump_and_special_edges() {
        // Waiting Room trace: peach-wr-in1-1, first float 82510, jump 82839,
        // Parasol 82900, far CliffCatch 82933. Daisy has the same waypoints.
        let seed = live([-119.986, 20.0]);
        let f = facts(0xd, (311, 0), seed.position);
        let mut route = FloatFlight::start(&f, seed).expect("accept the native set cell");
        let rows = [
            (
                74,
                0xe,
                [-67.303, -59.590],
                [1.029, -1.19],
                1,
                [0.0, -1.0],
                JUMP,
            ),
            (
                75,
                0x1e8,
                [-66.284, -60.780],
                [1.019, -1.19],
                1,
                [0.0, -1.0],
                JUMP,
            ),
            (
                77,
                0x1e8,
                [-64.276, -60.780],
                [0.999, 0.0],
                1,
                [0.0, -1.0],
                JUMP,
            ),
            (
                78,
                0x1e8,
                [-63.287, -60.780],
                [0.989, 0.0],
                1,
                [1.0, 0.0],
                JUMP,
            ),
            (
                87,
                0x1e9,
                [-54.026, -60.780],
                [1.029, 0.0],
                1,
                [1.0, 0.0],
                JUMP,
            ),
            (
                221,
                0x1e9,
                [83.860, -60.780],
                [1.029, 0.0],
                1,
                [0.0, 0.0],
                0,
            ),
            (222, 0xe, [84.879, -60.780], [1.019, 0.0], 1, [-1.0, 0.0], 0),
            (
                230,
                0xe,
                [88.711, -63.300],
                [0.059, -0.56],
                1,
                [0.0, 0.0],
                0,
            ),
            (236, 0xe, [88.856, -68.130], [0.0, -0.98], 1, [0.0, 0.0], 0),
            (
                237,
                0xe,
                [88.856, -69.180],
                [0.0, -1.05],
                1,
                [0.0, 0.0],
                JUMP,
            ),
            (
                238,
                0xc,
                [88.856, -70.300],
                [0.0, -1.12],
                0,
                [0.0, 0.0],
                JUMP,
            ),
            (239, 0xc, [88.856, -68.285], [0.0, 2.0154], 0, [0.0, 0.0], 0),
            (
                267,
                0xc,
                [88.856, -40.273],
                [0.0, 0.0554],
                0,
                [0.0, 1.0],
                SPECIAL,
            ),
            (
                268,
                0x1de,
                [88.856, -40.288],
                [0.0, -0.0146],
                0,
                [-1.0, 0.0],
                0,
            ),
        ];
        for (frame, status, position, velocity, jumps, stick, press) in rows {
            let edit = route
                .step(FloatBeat {
                    frame,
                    status,
                    position,
                    velocity,
                    jumps,
                    ..seed
                })
                .unwrap_or_else(|| panic!("lost frame {frame} at status {status:#x}"));
            assert_eq!(
                (edit.stick, edit.press),
                (Some(stick), press),
                "frame {frame}"
            );
        }
    }

    lib_unit_tests! {
    #[test]
    fn accepted_parasol_cells_reach_the_measured_far_special_window() {
        for (row, bits) in PEACH_CROSS_RING.into_iter().enumerate() {
            for col in (0..30).filter(|col| bits & (1 << col) != 0) {
                let points = [[2.0, 2.0], [0.01, 0.01], [3.99, 0.01], [0.01, 3.99], [3.99, 3.99]];
                for ([dx, dy], vy) in points.into_iter().flat_map(|p| [0.0, -FALL_SPEED].map(|v| (p, v))) {
                    let position = [LEFT[0] - dx - col as f32 * 4.0, 40.0 - dy - row as f32 * 4.0];
                    let mut beat = FloatBeat { velocity: [0.0, vy], ..live(position) };
                    let f = facts(0xd, (3, 0), position);
                    let mut flight = FloatFlight::start(&f, beat).expect("set cell");
                    let mut float_since = None;
                    let mut jump_since = None;
                    for frame in 0..400 {
                        beat.frame = frame;
                        let edit = flight.step(beat).unwrap_or_else(|| {
                            panic!(
                                "route ended before up-B: {position:?} vy {vy} frame {frame}: {:?}, {:?}, {:#x}, {:?}",
                                beat.position, beat.velocity, beat.status, flight.phase,
                            )
                        });
                        let [sx, sy] = edit.stick.expect("the route owns the stick");
                        if edit.press & SPECIAL != 0 {
                            let outside = beat.position[0] - RIGHT[0];
                            assert!((3.0..=9.5).contains(&outside));
                            assert!(PARASOL_BAND.contains(&beat.position[1]));
                            break;
                        }
                        if edit.press & JUMP != 0 && sy < -0.5 && float_since.is_none() {
                            float_since = Some(frame);
                        } else if edit.press & JUMP != 0 && sy == 0.0 && beat.status == FALL {
                            jump_since = Some(frame);
                        }
                        let is_floating = (beat.status == FLOAT_START || beat.status == FLOAT)
                            && edit.press & JUMP != 0
                            && float_since.is_some_and(|start| frame - start <= FLOAT_FRAMES);
                        let brake = 0.01_f32.min(beat.velocity[0].abs());
                        beat.velocity[0] = match sx {
                            sx if sx > 0.0 => (beat.velocity[0] + ACCEL).min(AIR_SPEED),
                            sx if sx < 0.0 => (beat.velocity[0] - ACCEL).max(-AIR_SPEED),
                            _ => beat.velocity[0] - brake.copysign(beat.velocity[0]),
                        };
                        if jump_since.is_some_and(|start| frame == start + 1) {
                            beat.velocity = [0.0, 2.0154];
                            beat.jumps = 0;
                        } else if matches!(beat.status, FLOAT_START | FLOAT) {
                            beat.velocity[1] = 0.0;
                        } else {
                            beat.velocity[1] = (beat.velocity[1] - GRAVITY).max(-FALL_SPEED);
                        }
                        beat.position = core::array::from_fn(|axis| beat.position[axis] + beat.velocity[axis]);
                        beat.status = if jump_since.is_some() {
                            AERIAL_JUMP
                        } else if let Some(since) = float_since {
                            match frame - since {
                                0 => FLOAT_START,
                                1..=11 if is_floating => FLOAT_START,
                                _ if is_floating => FLOAT,
                                _ => FALL,
                            }
                        } else {
                            FALL
                        };
                        assert!(frame < 399, "cell {position:?} never pressed up-B");
                    }
                }
            }
        }
    }
    }
}
