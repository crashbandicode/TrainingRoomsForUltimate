//! Measured left-to-right Cypher flight on the three common stage outlines.
use super::{
    line_height, niche::pad, ordered_x, region, Correction, Point, RouteFacts, JUMP, SPECIAL,
};

pub(crate) const KIND: i32 = 0x22;
pub(crate) const ROW: u8 = 26;
pub(crate) const FALL: i32 = 0xe;
pub(crate) const JUMP_AERIAL: i32 = 0xc;
const CYPHER_START: i32 = 0x1de;
const CYPHER_RIDE: i32 = 0x1f4;
const CYPHER_RELEASE: i32 = 0x1f5;
const FORWARD: Point = [1.0, 0.0];
const BACK: Point = [-1.0, 0.0];
const UP: Point = [0.0, 1.0];
// g4/bake-snake-product.py. Only the flown near-rest band with neighbour margin.
const REST_RING: [u32; 15] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 96, 32, 48, 16, 16];

pub(crate) fn in_region(stage: (i32, u32), near: Point, position: Point) -> bool {
    super::is_crossing_stage(stage)
        && region::contains(&REST_RING, near[0] - position[0], position[1] - near[1])
}

fn has_common_underside(facts: &RouteFacts<'_>) -> bool {
    let [near, far] = facts.ledges;
    ((far[0] - near[0]) - 159.975).abs() < 0.12
        && (far[1] - near[1]).abs() < 0.12
        && (facts.blast[3] - near[1] + 140.0).abs() < 0.12
        && [
            (1.0, -9.396),
            (15.591, -25.111),
            (38.590, -40.099),
            (79.9875, -40.109),
            (121.402, -40.118),
            (144.373, -25.103),
            (158.975, -9.395),
        ]
        .into_iter()
        .all(|(offset, expected)| {
            let x = near[0] + offset;
            let lower = facts
                .outline
                .iter()
                .filter_map(|&line| {
                    let [start, end] = ordered_x(line);
                    (start..=end).contains(&x).then(|| line_height(line, x)[0])
                })
                .reduce(f32::min);
            lower.is_some_and(|y| (y - near[1] - expected).abs() < 0.12)
        })
}

#[derive(Clone, Copy)]
pub(crate) struct CypherSample {
    pub(crate) frame: u32,
    pub(crate) status: i32,
    pub(crate) position: Point,
    pub(crate) velocity: Point,
    pub(crate) facing: f32,
    pub(crate) jumps: u8,
    pub(crate) can_jump: bool,
    pub(crate) can_rise: bool,
}

#[derive(Clone, Copy)]
enum Phase {
    Drift,
    Jump(u32),
    Ascent,
    Start(u32),
    Ride,
    Release,
}

pub(crate) struct CypherFlight {
    near: Point,
    far: Point,
    bottom: f32,
    phase: Phase,
    last_pad: Option<(u32, Correction)>,
}

impl CypherFlight {
    pub(crate) fn start(facts: &RouteFacts<'_>, sample: CypherSample) -> Option<Self> {
        let [near, far] = facts.ledges;
        facts.crossing_band()?;
        if facts.fighter != KIND
            || facts.position != sample.position
            || !in_region(facts.stage, near, sample.position)
            || !has_common_underside(facts)
            || sample.status != FALL
            || sample.velocity[0] != 0.0
            || !(-0.32..=0.0).contains(&sample.velocity[1])
            || sample.facing != -1.0
            || sample.jumps != 1
            || !sample.can_rise
        {
            return None;
        }
        Some(Self {
            near,
            far,
            bottom: facts.blast[3],
            phase: Phase::Drift,
            last_pad: None,
        })
    }

    pub(crate) fn exit_point(&self) -> Point {
        self.far
    }

    pub(crate) fn step(&mut self, sample: CypherSample) -> Option<Correction> {
        let [x, y] = sample.position;
        if !x.is_finite()
            || !y.is_finite()
            || y <= self.bottom
            || !sample.velocity.into_iter().all(f32::is_finite)
        {
            return None;
        }
        if let Some((_, correction)) = self.last_pad.filter(|&(frame, _)| frame == sample.frame) {
            return Some(correction);
        }
        let inside = x - self.near[0];
        let height = y - self.near[1];
        let (next, stick, buttons) = match self.phase {
            Phase::Drift if sample.status == FALL && sample.jumps == 1 && sample.facing == -1.0 => {
                if inside >= 37.1 || height <= -128.0 {
                    sample
                        .can_jump
                        .then_some((Phase::Jump(sample.frame), FORWARD, JUMP))?
                } else {
                    (Phase::Drift, FORWARD, 0)
                }
            }
            Phase::Jump(press)
                if sample.frame.wrapping_sub(press) == 1
                    && sample.facing == -1.0
                    && matches!(sample.status, FALL | JUMP_AERIAL) =>
            {
                (Phase::Ascent, FORWARD, JUMP)
            }
            Phase::Ascent
                if sample.status == JUMP_AERIAL && sample.jumps == 0 && sample.facing == -1.0 =>
            {
                if inside >= 62.8 {
                    (sample.can_rise && (-108.0..=-77.0).contains(&height)).then_some((
                        Phase::Start(sample.frame),
                        UP,
                        SPECIAL,
                    ))?
                } else {
                    (Phase::Ascent, FORWARD, 0)
                }
            }
            Phase::Start(press) if sample.status == CYPHER_START && sample.facing == -1.0 => {
                let age = sample.frame.wrapping_sub(press);
                if age <= 2 {
                    (Phase::Start(press), UP, SPECIAL)
                } else {
                    (Phase::Start(press), FORWARD, 0)
                }
            }
            Phase::Start(_) | Phase::Ride
                if sample.status == CYPHER_RIDE && sample.facing == -1.0 =>
            {
                (Phase::Ride, FORWARD, 0)
            }
            Phase::Ride | Phase::Release if sample.status == CYPHER_RELEASE => {
                (Phase::Release, BACK, 0)
            }
            _ => return None,
        };
        self.phase = next;
        let correction = pad(stick, buttons);
        self.last_pad = Some((sample.frame, correction));
        Some(correction)
    }
}

#[cfg(test)]
#[path = "snake_tests.rs"]
mod tests;
