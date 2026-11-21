//! Lucas L-X from rest, measured by Fusion G2 on the common collision ring.
use super::{niche::pad, region, Correction, Point, RouteFacts, SpecialReach, GUARD, JUMP, SPECIAL};

pub const KIND: i32 = 0x28;
pub const ROW: u8 = 28;
pub const WIZARD_OFFSET: Point = [18.0, 2.0];

const FALL: i32 = 0x0e;
const AIR_DODGE: i32 = 0x22;
const JUMP_AERIAL: i32 = 0x0c;
const FALL_AERIAL: i32 = 0x0f;
const THUNDER_START: i32 = 0x1de;
const THUNDER_HOLD: i32 = 0x1e4;
const THUNDER_FLIGHT: i32 = 0x1e6;
const GRAVITY: f32 = 0.09;
const TERMINAL_FALL: f32 = 1.37;
// The lower endpoint is the exact f32 from the measured first dodge row.
const DODGE_HEIGHT: core::ops::RangeInclusive<f32> = -1.890_000_1..=2.65;
const AIR_BRAKE: f32 = 0.014;
const DODGE_PRESS: u32 = 4;
const DODGE_ROWS: u32 = 84;
const JUMP_ROWS: u32 = 74;
const THUNDER_ROWS: u32 = 93;
const NEUTRAL: Point = [0.0; 2];
const INWARD: Point = [1.0, 0.0];
const INWARD_DOWN: Point = [85.0 / 120.0, -85.0 / 120.0];
const UP: Point = [0.0, 1.0];

pub const fn reference_stage(stage: (i32, u32)) -> bool {
    matches!(stage, (0 | 3 | 311, 0))
}

pub fn measured_envelope(kind: i32, variant: i32, reach: SpecialReach) -> bool {
    kind == KIND && variant == 0 && reach.rise == 78.0 && reach.horizontal == -1.0
}

#[derive(Clone, Copy)]
pub struct LucasBeat {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub facing: f32,
    pub jumps: u8,
    pub can_dodge: bool,
    pub can_jump: bool,
    pub can_thunder: bool,
}

#[derive(Clone, Copy)]
enum Phase {
    Fall(u32),
    Dodge(u32),
    Jump(u32),
    Thunder(u32),
    Flight(u32),
}

pub struct UnderStage {
    near: Point,
    far: Point,
    bottom: f32,
    phase: Phase,
    dodge_at: u32,
    last_pad: Option<(u32, Correction)>,
}

impl UnderStage {
    pub fn start(facts: &RouteFacts<'_>, beat: LucasBeat) -> Option<Self> {
        facts.crossing_band()?;
        let [near, far] = facts.ledges;
        let [vx, vy] = beat.velocity;
        let out = near[0] - beat.position[0];
        let above = beat.position[1] - near[1] - vy;
        if facts.fighter != KIND
            || !reference_stage(facts.stage)
            || !clear_guarder(near, far, facts.player)
            || !region::has_common_underside(facts.ledges, facts.blast[3], facts.outline)
            || beat.position != facts.position
            || beat.status != FALL
            || !matches!(beat.facing, -1.0 | 1.0)
            || beat.jumps != 1
            || !beat.can_dodge
            || !beat.can_jump
            || !beat.can_thunder
            || vx != 0.0
            || !(-0.090001..=0.0).contains(&vy)
            // Four adjacent cells from 8 to 24 out. Above +4, wait before dodging.
            || !(8.0..=24.0).contains(&out)
            || !(0.0..=38.0).contains(&above)
        {
            return None;
        }
        let dodge_at = if above > 4.0 {
            dodge_after_wait(beat.position[1] - near[1], vy)?
        } else {
            DODGE_PRESS
        };
        Some(Self {
            near,
            far,
            bottom: facts.blast[3],
            phase: Phase::Fall(beat.frame),
            dodge_at,
            last_pad: None,
        })
    }

    pub fn exit_point(&self) -> Point {
        self.far
    }

    pub fn crossing_frames(&self) -> u16 {
        self.dodge_at as u16 + 305 // The first native cliff row, including the lead.
    }

    pub fn before_dodge(&self) -> bool {
        matches!(self.phase, Phase::Fall(_))
    }

    pub fn guarder_clear(&self, player: Point) -> bool {
        clear_guarder(self.near, self.far, player)
    }

    pub fn guard_frame(&self, frame: u32) -> bool {
        matches!(self.phase, Phase::Fall(origin) if frame.wrapping_sub(origin) == self.dodge_at)
    }

    pub fn jump_frame(&self, frame: u32) -> bool {
        matches!(self.phase, Phase::Dodge(origin) if frame.wrapping_sub(origin) == DODGE_ROWS - 1)
    }

    pub fn first_jump_row(&self, frame: u32) -> bool {
        matches!(self.phase, Phase::Dodge(origin) if frame.wrapping_sub(origin) == DODGE_ROWS)
    }

    pub fn launch_frame(&self, frame: u32) -> bool {
        matches!(self.phase, Phase::Thunder(origin) if frame.wrapping_sub(origin) == THUNDER_ROWS)
    }

    pub fn cast_frame(&self, frame: u32) -> bool {
        matches!(self.phase, Phase::Jump(origin) if frame.wrapping_sub(origin) == JUMP_ROWS - 1)
    }

    pub fn step(&mut self, beat: LucasBeat) -> Option<Correction> {
        if !beat.position.into_iter().chain(beat.velocity).all(f32::is_finite)
            || beat.position[1] <= self.bottom
        {
            return None;
        }
        if let Some((_, correction)) = self.last_pad.filter(|&(frame, _)| frame == beat.frame) {
            return Some(correction);
        }
        let correction = self.correct(beat)?;
        self.last_pad = Some((beat.frame, correction));
        Some(correction)
    }

    fn correct(&mut self, beat: LucasBeat) -> Option<Correction> {
        let phase = self.phase;
        let origin = match phase {
            Phase::Fall(origin)
            | Phase::Dodge(origin)
            | Phase::Jump(origin)
            | Phase::Thunder(origin)
            | Phase::Flight(origin) => origin,
        };
        let age = beat.frame.wrapping_sub(origin);
        match phase {
            Phase::Fall(_) if age < self.dodge_at && beat.status == FALL => Some(pad(NEUTRAL, 0)),
            Phase::Fall(_) if age == self.dodge_at && beat.status == FALL => {
                let [x, y] = beat.position;
                let [vx, vy] = beat.velocity;
                (beat.can_dodge
                    && beat.can_jump
                    && beat.can_thunder
                    && beat.jumps == 1
                    && vx == 0.0
                    && (8.0..=24.0).contains(&(self.near[0] - x))
                    && DODGE_HEIGHT.contains(&(y + next_fall(vy) - self.near[1])))
                    .then(|| pad(INWARD_DOWN, GUARD))
            }
            Phase::Fall(_) if age == self.dodge_at + 1 && beat.status == AIR_DODGE => {
                if !DODGE_HEIGHT.contains(&(beat.position[1] - self.near[1])) {
                    return None;
                }
                self.phase = Phase::Dodge(beat.frame);
                Some(pad(INWARD_DOWN, GUARD))
            }
            Phase::Dodge(_) if age < DODGE_ROWS - 1 && beat.status == AIR_DODGE => {
                Some(pad(INWARD, 0))
            }
            Phase::Dodge(_) if age == DODGE_ROWS - 1 && beat.status == AIR_DODGE => {
                let [x, y] = beat.position;
                let [vx, vy] = beat.velocity;
                (beat.can_jump
                    && beat.jumps == 1
                    && self.jump_row([x + vx, y + vy]))
                    .then(|| pad(INWARD, JUMP))
            }
            Phase::Dodge(_) if age == DODGE_ROWS && beat.status == JUMP_AERIAL => {
                if beat.jumps != 0 || !self.jump_row(beat.position) {
                    return None;
                }
                self.phase = Phase::Jump(beat.frame);
                Some(pad(INWARD, JUMP))
            }
            Phase::Jump(_) if age < JUMP_ROWS - 1 && matches!(beat.status, JUMP_AERIAL | FALL_AERIAL) => {
                Some(pad(INWARD, 0))
            }
            Phase::Jump(_) if age == JUMP_ROWS - 1 && beat.status == FALL_AERIAL => {
                let [x, y] = beat.position;
                let [vx, vy] = beat.velocity;
                (beat.can_thunder
                    && self.thunder_row([x + vx - AIR_BRAKE, y + vy]))
                    .then(|| pad(UP, SPECIAL))
            }
            Phase::Jump(_) if age == JUMP_ROWS && beat.status == THUNDER_START => {
                if !self.thunder_row(beat.position) {
                    return None;
                }
                self.phase = Phase::Thunder(beat.frame);
                Some(pad(UP, SPECIAL))
            }
            Phase::Thunder(_) if age < 19 && beat.status == THUNDER_START => Some(pad(NEUTRAL, 0)),
            Phase::Thunder(_) if (19..THUNDER_ROWS).contains(&age) && beat.status == THUNDER_HOLD => {
                Some(pad(ball_stick(age), 0))
            }
            Phase::Thunder(_) if age == THUNDER_ROWS && beat.status == THUNDER_FLIGHT => {
                if beat.facing != -1.0 || !self.launch_row(beat.position) {
                    return None;
                }
                self.phase = Phase::Flight(beat.frame);
                Some(pad(UP, 0))
            }
            Phase::Flight(_) if age <= 50 && beat.status == THUNDER_FLIGHT => Some(pad(UP, 0)),
            _ => None,
        }
    }

    fn jump_row(&self, point: Point) -> bool {
        self.in_waypoint(point, -14.072..=-6.071, -110.721..=-105.242)
    }

    fn thunder_row(&self, point: Point) -> bool {
        self.in_waypoint(point, 70.889..=78.890, -91.643..=-86.163)
    }

    fn launch_row(&self, point: Point) -> bool {
        self.in_waypoint(point, 78.102..=86.103, -116.888..=-111.408)
    }

    fn in_waypoint(
        &self,
        [x, y]: Point,
        x_from_middle: core::ops::RangeInclusive<f32>,
        y_from_lip: core::ops::RangeInclusive<f32>,
    ) -> bool {
        x_from_middle.contains(&(x - self.near[0].midpoint(self.far[0])))
            && y_from_lip.contains(&(y - self.near[1]))
    }
}

fn clear_guarder(near: Point, far: Point, [x, y]: Point) -> bool {
    // Before the dodge, require a grounded guarder inside the near lip and
    // away from the drop. The far-side race handles interception after that.
    (near[0] + 10.0..=near[0].midpoint(far[0])).contains(&x)
        && y.is_finite()
        && y >= near[1] - 0.12
}

fn next_fall(vy: f32) -> f32 {
    (vy - GRAVITY).max(-TERMINAL_FALL)
}

fn dodge_after_wait(mut height: f32, mut vy: f32) -> Option<u32> {
    (0..=33).find(|&frame| {
        vy = next_fall(vy);
        height += vy;
        frame >= DODGE_PRESS && height <= 2.0
    })
}

fn ball_stick(thunder_age: u32) -> Point {
    // The held_loop(28) reference: one pad per native row.
    const TO_DOWN: [[i8; 2]; 14] = [
        [111, 45], [116, 29], [119, 13], [120, -4], [118, -21], [114, -37], [108, -53],
        [99, -67], [89, -80], [77, -92], [64, -102], [49, -110], [33, -115], [17, -119],
    ];
    const AFTER_BACK: [[i8; 2]; 8] = [
        [0, 120], [17, 119], [33, 115], [49, 110], [64, 102], [77, 92], [89, 80], [99, 67],
    ];
    let xy = match thunder_age {
        19..=32 => TO_DOWN[(thunder_age - 19) as usize],
        33..=69 => [0, -120],
        70..=83 => TO_DOWN[(thunder_age - 70) as usize].map(|axis| -axis),
        84..=91 => AFTER_BACK[(thunder_age - 84) as usize],
        _ => return UP,
    };
    xy.map(|axis| f32::from(axis) / 120.0)
}

#[cfg(test)]
#[path = "lucas_tests.rs"]
mod tests;
