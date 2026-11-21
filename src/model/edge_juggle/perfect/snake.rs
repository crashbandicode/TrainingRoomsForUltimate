//! Measured left-to-right Cypher flight on the three common stage outlines.
use super::{
    niche::pad, region, Correction, Point, RouteFacts, JUMP, SPECIAL,
};

#[path = "snake_envelope.rs"]
mod envelope;

pub const KIND: i32 = 0x22;
pub const ROW: u8 = 26;
pub const FALL: i32 = 0xe;
pub const JUMP_AERIAL: i32 = 0xc;
const CYPHER_START: i32 = 0x1de;
const CYPHER_RIDE: i32 = 0x1f4;
const CYPHER_RELEASE: i32 = 0x1f5;
const FORWARD: Point = [1.0, 0.0];
const BACK: Point = [-1.0, 0.0];
const UP: Point = [0.0, 1.0];
pub fn in_region(stage: (i32, u32), near: Point, position: Point) -> bool {
    super::is_crossing_stage(stage) && flight_time(near, position, 0.0).is_some()
}

fn flight_time(near: Point, position: Point, vy: f32) -> Option<u16> {
    envelope::guarded_time(
        f64::from(near[0]) - f64::from(position[0]),
        f64::from(position[1]) - f64::from(near[1]),
        f64::from(vy),
    )
}

#[derive(Clone, Copy)]
pub struct CypherSample {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub facing: f32,
    pub jumps: u8,
    pub can_jump: bool,
    pub can_rise: bool,
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

pub struct CypherFlight {
    near: Point,
    far: Point,
    bottom: f32,
    frames: u16,
    phase: Phase,
    last_pad: Option<(u32, Correction)>,
}

impl CypherFlight {
    pub fn start(facts: &RouteFacts<'_>, sample: CypherSample) -> Option<Self> {
        let [near, far] = facts.ledges;
        facts.crossing_band()?;
        if facts.fighter != KIND
            || facts.position != sample.position
            || !region::has_common_underside(facts.ledges, facts.blast[3], facts.outline)
            || sample.status != FALL
            || sample.velocity[0] != 0.0
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
            frames: flight_time(near, sample.position, sample.velocity[1])?,
            phase: Phase::Drift,
            last_pad: None,
        })
    }

    pub fn exit_point(&self) -> Point {
        self.far
    }

    pub fn crossing_frames(&self) -> u16 {
        self.frames
    }

    pub fn step(&mut self, sample: CypherSample) -> Option<Correction> {
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
        let inside = f64::from(x) - f64::from(self.near[0]);
        let height = f64::from(y) - f64::from(self.near[1]);
        let (next, stick, buttons) = match self.phase {
            Phase::Drift if sample.status == FALL && sample.jumps == 1 && sample.facing == -1.0 => {
                if inside >= envelope::JUMP_INSIDE || height <= envelope::JUMP_HEIGHT {
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
                if inside >= envelope::CYPHER_INSIDE {
                    (sample.can_rise && envelope::CYPHER_HEIGHT.contains(&height)).then_some((
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
