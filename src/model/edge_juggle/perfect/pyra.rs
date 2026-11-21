//! Measured near-side Pyra to Mythra recovery.
use super::{niche::pad, region, Correction, Point, RouteFacts, JUMP, SPECIAL};

pub const PYRA: i32 = 0x5a;
pub const MYTHRA: i32 = 0x5b;
pub const ROW: u8 = 38;
const FALL: i32 = 0x0e;
const JUMP_AERIAL: i32 = 0x0c;
const FALL_SPECIAL: i32 = 0x10;
const SWAP_START: i32 = 0x1df;
const SWAP_END: i32 = 0x1e2;
const RAY_START: i32 = 0x1de;
const RAY_RISE: i32 = 0x1eb;
const RAY_GLIDE: i32 = 0x1ee;

#[derive(Clone, Copy)]
pub struct Beat {
    pub frame: u32,
    pub fighter: i32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub facing: f32,
    pub jumps: u8,
    pub can_jump: bool,
    pub can_swap: bool,
    pub can_ray: bool,
}

// Group 4 measured cells; omit only the empty rows after the last set bit.
const SWAP_FIRST_RING: [u32; 22] = [
    0x0, 0x7ffffc0, 0x3ffffc0, 0x1ffffc0, 0x1ffffc0, 0xffffc0,
    0x7fff80, 0x7fff80, 0x3ff800, 0x1ff000, 0x1ff000, 0xff000,
    0x7f000, 0x7f000, 0x3f000, 0x3f000, 0x1f000, 0x1f000,
    0xf000, 0xf000, 0x7000, 0x3000,
];
const JUMP_FIRST_RING: [u32; 19] = [
    0x0, 0x1fffffc0, 0x1fffffc0, 0x1fffffc0, 0x1fffffc0, 0x1fffffc0,
    0x1ff3ffc0, 0x1fe1ffc0, 0x1fe0ffc0, 0x1fe0ffc0, 0xfe07fc0, 0x7e03fc0,
    0x7e01fc0, 0x3e00fc0, 0x3e003c0, 0x1e00000, 0x1e00000, 0xe00000,
    0xe00000,
];
const JUMP_FIRST_LOW_RING: [u32; 24] = [
    0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
    0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
    0x0, 0x0, 0x0, 0x0, 0x0, 0x10000000,
    0x10000000, 0x10000000, 0x18000000, 0xc000000, 0xe000000, 0x7000000,
];

#[derive(Clone, Copy)]
enum Approach {
    SwapFirst,
    JumpFirst,
}

#[derive(PartialEq, Eq)]
enum Form {
    Pyra,
    Swapping,
    Mythra,
}

pub struct SwapReturn {
    start: u32,
    approach: Approach,
    form: Form,
}

impl SwapReturn {
    pub fn start(facts: &RouteFacts<'_>, beat: Beat) -> Option<Self> {
        let near = facts.ledges[0];
        let outside = near[0] - beat.position[0];
        let height = beat.position[1] - near[1];
        if !super::is_crossing_stage(facts.stage)
            || facts.fighter != PYRA
            || beat.fighter != PYRA
            || beat.position != facts.position
            || beat.status != FALL
            || beat.facing != 1.0
            || beat.jumps != 1
            || beat.velocity[0] != 0.0
            || !(-1.62..=0.0).contains(&beat.velocity[1])
            || !beat.can_swap
            || !region::has_common_underside(facts.ledges, facts.blast[3], facts.outline)
        {
            return None;
        }
        let approach = if beat.can_jump
            && (region::contains(&JUMP_FIRST_RING, outside, height)
                || region::contains(&JUMP_FIRST_LOW_RING, outside, height))
        {
            Approach::JumpFirst
        } else if region::contains(&SWAP_FIRST_RING, outside, height) {
            Approach::SwapFirst
        } else {
            return None;
        };
        Some(Self { start: beat.frame, approach, form: Form::Pyra })
    }

    pub fn waiting_for_mythra(&self, frame: u32) -> bool {
        self.form == Form::Swapping
            && frame.checked_sub(self.start).is_some_and(|age| {
                (self.swap_age() + 1..=self.swap_age() + 26).contains(&age)
            })
    }

    pub fn follow(&mut self, frame: u32, fighter: i32, status: i32) -> bool {
        if !self.waiting_for_mythra(frame) || fighter != MYTHRA || status != SWAP_END {
            return false;
        }
        self.form = Form::Mythra;
        true
    }

    pub fn needs_swap(&self, frame: u32) -> bool {
        frame.checked_sub(self.start) == Some(self.swap_age())
    }

    pub fn needs_jump(&self, frame: u32) -> bool {
        frame.checked_sub(self.start) == Some(self.jump_age())
    }

    pub fn needs_ray(&self, frame: u32) -> bool {
        frame.checked_sub(self.start) == Some(self.ray_age())
    }

    fn swap_age(&self) -> u32 {
        match self.approach {
            Approach::SwapFirst => 0,
            Approach::JumpFirst => 41,
        }
    }

    fn jump_age(&self) -> u32 {
        match self.approach {
            Approach::SwapFirst => 27,
            Approach::JumpFirst => 0,
        }
    }

    fn ray_age(&self) -> u32 {
        match self.approach {
            Approach::SwapFirst => 49,
            Approach::JumpFirst => 68,
        }
    }

    pub fn step(&mut self, beat: Beat) -> Option<Correction> {
        let age = beat.frame.checked_sub(self.start)?;
        let status = beat.status;
        let swap = self.swap_age();
        let jump = self.jump_age();
        let ray = self.ray_age();
        let inward = [1.0, 0.0];

        if self.form == Form::Mythra {
            if beat.fighter != MYTHRA {
                return None;
            }
            let in_swap_tail = status == SWAP_END && age >= swap + 12;
            let special_fall = match self.approach {
                Approach::SwapFirst => 87..=117,
                Approach::JumpFirst => 93..=114,
            };
            return match age {
                f if in_swap_tail && f < swap + 27 => {
                    Some(pad(inward, 0))
                }
                f if f == jump && in_swap_tail && beat.can_jump && beat.jumps == 1 => {
                    Some(pad(inward, JUMP))
                }
                f if f == jump + 1 && status == JUMP_AERIAL => Some(pad(inward, JUMP)),
                f if (jump + 2..ray).contains(&f) && status == JUMP_AERIAL => {
                    Some(pad(inward, 0))
                }
                f if f == ray && beat.can_ray && matches!(status, SWAP_END | JUMP_AERIAL) => {
                    Some(pad([0.0, 1.0], SPECIAL))
                }
                f if status == RAY_START
                    && ((ray + 1..=ray + 2).contains(&f)
                        || matches!(self.approach, Approach::JumpFirst)
                            && matches!(f.saturating_sub(ray), 3 | 4 | 7 | 8)) =>
                {
                    Some(pad([0.0, 1.0], SPECIAL))
                }
                f if (ray + 3..=ray + 112).contains(&f)
                    && matches!(status, RAY_START | RAY_RISE | 0x1ec | 0x1ed | RAY_GLIDE) =>
                {
                    Some(pad(inward, 0))
                }
                f if status == FALL_SPECIAL && special_fall.contains(&f.saturating_sub(ray)) => {
                    Some(pad(inward, 0))
                }
                _ => None,
            };
        }
        if beat.fighter != PYRA {
            return None;
        }
        match age {
            f if f == jump && jump < swap && beat.can_jump && beat.jumps == 1 && status == FALL => {
                Some(pad(inward, JUMP))
            }
            f if f == jump + 1 && jump < swap && status == JUMP_AERIAL => {
                Some(pad(inward, JUMP))
            }
            f if (jump + 2..swap).contains(&f) && status == JUMP_AERIAL => Some(pad(inward, 0)),
            f if f == swap && beat.can_swap && matches!(status, FALL | JUMP_AERIAL) => {
                Some(pad([0.0, -0.6], SPECIAL))
            }
            f if (swap + 1..=swap + 11).contains(&f) && status == SWAP_START => {
                self.form = Form::Swapping;
                Some(if f <= swap + 2 { pad([0.0, -0.6], SPECIAL) } else { pad(inward, 0) })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "pyra_tests.rs"]
mod tests;
