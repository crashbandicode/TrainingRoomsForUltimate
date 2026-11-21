//! Measured Cypher flight on the three common stage outlines.
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
const UP: Point = [0.0, 1.0];
// Only Final Destination has a flown right-to-left reference.
fn crossing_side(stage: (i32, u32), ledges: [Point; 2], position: Point) -> Option<(Point, Point, f32)> {
    let [left, right] = ledges;
    if super::is_crossing_stage(stage) && position[0] < left[0] {
        Some((left, right, 1.0))
    } else if stage == (3, 0) && position[0] > right[0] {
        Some((right, left, -1.0))
    } else {
        None
    }
}

pub fn in_region(stage: (i32, u32), ledges: [Point; 2], position: Point) -> bool {
    crossing_side(stage, ledges, position).is_some_and(|(near, _, inward)| {
        flight_time(near, position, inward, 0.0).is_some()
    })
}

fn flight_time(near: Point, position: Point, inward: f32, vy: f32) -> Option<u16> {
    envelope::guarded_time(
        (f64::from(near[0]) - f64::from(position[0])) * f64::from(inward),
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
    inward: f32,
    bottom: f32,
    frames: u16,
    phase: Phase,
    last_pad: Option<(u32, Correction)>,
}

impl CypherFlight {
    pub fn start(facts: &RouteFacts<'_>, sample: CypherSample) -> Option<Self> {
        facts.crossing_band()?;
        let (near, far, inward) = crossing_side(facts.stage, facts.ledges, sample.position)?;
        if facts.fighter != KIND
            || facts.position != sample.position
            || !region::has_common_underside(facts.ledges, facts.blast[3], facts.outline)
            || sample.status != FALL
            || sample.velocity[0] != 0.0
            || sample.facing != -inward
            || sample.jumps != 1
            || !sample.can_rise
        {
            return None;
        }
        Some(Self {
            near,
            far,
            inward,
            bottom: facts.blast[3],
            frames: flight_time(near, sample.position, inward, sample.velocity[1])?,
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
        let inside = (f64::from(x) - f64::from(self.near[0])) * f64::from(self.inward);
        let forward = [self.inward, 0.0];
        let back = [-self.inward, 0.0];
        let facing_away = sample.facing == -self.inward;
        let height = f64::from(y) - f64::from(self.near[1]);
        let (next, stick, buttons) = match self.phase {
            Phase::Drift if sample.status == FALL && sample.jumps == 1 && facing_away => {
                if inside >= envelope::JUMP_INSIDE || height <= envelope::JUMP_HEIGHT {
                    sample
                        .can_jump
                        .then_some((Phase::Jump(sample.frame), forward, JUMP))?
                } else {
                    (Phase::Drift, forward, 0)
                }
            }
            Phase::Jump(press)
                if sample.frame.wrapping_sub(press) == 1
                    && facing_away
                    && matches!(sample.status, FALL | JUMP_AERIAL) =>
            {
                (Phase::Ascent, forward, JUMP)
            }
            Phase::Ascent
                if sample.status == JUMP_AERIAL && sample.jumps == 0 && facing_away =>
            {
                if inside >= envelope::CYPHER_INSIDE {
                    (sample.can_rise && envelope::CYPHER_HEIGHT.contains(&height)).then_some((
                        Phase::Start(sample.frame),
                        UP,
                        SPECIAL,
                    ))?
                } else {
                    (Phase::Ascent, forward, 0)
                }
            }
            Phase::Start(press) if sample.status == CYPHER_START && facing_away => {
                let age = sample.frame.wrapping_sub(press);
                if age <= 2 {
                    (Phase::Start(press), UP, SPECIAL)
                } else {
                    (Phase::Start(press), forward, 0)
                }
            }
            Phase::Start(_) | Phase::Ride
                if sample.status == CYPHER_RIDE && facing_away =>
            {
                (Phase::Ride, forward, 0)
            }
            Phase::Ride | Phase::Release if sample.status == CYPHER_RELEASE => {
                (Phase::Release, back, 0)
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
