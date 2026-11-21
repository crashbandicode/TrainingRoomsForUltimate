//! The measured left-origin Skull Bash, Thunder and Quick Attack finish.

use super::{Correction, Point};
use crate::model::edge_juggle::perfect::{niche::pad, JUMP, SPECIAL};

pub const ROW: u8 = 2;

const UP: Point = [0.0, 1.0];
const SIDE_END: i32 = 0x1e6;
const JUMP_AERIAL: i32 = 0xc;
const SPECIAL_LW: i32 = 0x1df;
const DISCHARGE: i32 = 0x1e7;
const UP_START: i32 = 0x1de;
const UP_TRAVEL: i32 = 0x1e1;
const UP_END: i32 = 0x1e2;
const CAST_AGE: u32 = 12;

// From the stock reference before the epilogue writes the Side+B pad. The
// game falls another 1.55 before the native SPECIAL_S entry moves horizontally.
const BASH_DEPTH: std::ops::RangeInclusive<f32> = 53.519..=54.945;
const OUTSIDE: std::ops::RangeInclusive<f32> = 8.0..=39.5;

pub fn measured_approach(position: Point, velocity: Point, ledge: Point) -> bool {
    OUTSIDE.contains(&(ledge[0] - position[0]))
        && (18.0..=23.0).contains(&(ledge[1] - position[1]))
        && velocity[0] == 0.0
        && (-0.2..=0.0).contains(&velocity[1])
}

pub fn above_bash_line(y: f32, ledge_y: f32) -> bool {
    ledge_y - y < *BASH_DEPTH.start()
}

#[derive(Clone, Copy)]
pub struct Bash {
    pub frame: u32,
    pub position: Point,
    pub velocity: Point,
}

#[derive(Clone, Copy)]
pub struct Jump {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub jumps: u8,
    pub guarder: Option<Point>,
    pub up_available: bool,
}

pub struct Thunder {
    press: Bash,
    ledge_y: f32,
    aim: Point,
    jump_at: Option<u32>,
}

impl Thunder {
    pub fn from_bash(press: Bash, ledge: Point) -> Option<Self> {
        let outside = ledge[0] - press.position[0];
        if !OUTSIDE.contains(&outside)
            || !BASH_DEPTH.contains(&(ledge[1] - press.position[1]))
            || press.velocity[0] != 0.0
            || !(-1.551..=-1.549).contains(&press.velocity[1])
        {
            return None;
        }
        // Exact mapped stick pairs from pthu5/pthu6 (41.3 or 27 degrees).
        let aim = if outside < 14.0 {
            [90.0 / 120.0, 79.0 / 120.0]
        } else {
            [107.0 / 120.0, 54.0 / 120.0]
        };
        Some(Self {
            press,
            ledge_y: ledge[1],
            aim,
            jump_at: None,
        })
    }

    pub fn jump_due(&self, frame: u32) -> bool {
        frame.wrapping_sub(self.press.frame) == 98
    }

    pub fn active(&self) -> bool {
        self.jump_at.is_some()
    }

    pub fn cast_due(&self, frame: u32) -> bool {
        self.jump_at
            .is_some_and(|jump| frame.wrapping_sub(jump) == CAST_AGE)
    }

    pub fn begin(&mut self, jump: Jump) -> Option<Correction> {
        let [gx, gy] = jump.guarder?;
        let [sx, sy] = self.press.position;
        // Jump cancel on the particular native Skull Bash end row demonstrated
        // in the reference. The next frame carries us to the bolt's column.
        if !self.jump_due(jump.frame)
            || jump.status != SIDE_END
            || jump.jumps != 1
            || !jump.up_available
            || !(133.38..=133.50).contains(&(jump.position[0] - sx))
            || !(63.44..=63.56).contains(&(sy - jump.position[1]))
            || !(1.06..=1.07).contains(&jump.velocity[0])
            || !(-1.551..=-1.549).contains(&jump.velocity[1])
            || !(-6.0..=6.0).contains(&(gx - (sx + 134.5)))
            || !(-0.1..=0.1).contains(&(gy - self.ledge_y))
        {
            return None;
        }
        self.jump_at = Some(jump.frame);
        Some(pad([0.0; 2], JUMP))
    }

    pub fn step(
        &self,
        frame: u32,
        status: i32,
        can_down: bool,
        up_available: bool,
    ) -> Option<Correction> {
        let age = frame.wrapping_sub(self.jump_at?);
        // Both late Up+B edges land in the measured native buffer window.
        // Once the neutral jump commits, Thunder's recoil sets up the far zip.
        let neutral = [0.0; 2];
        let (stick, buttons) = match (age, status) {
            (0, SIDE_END) => (neutral, JUMP),
            (1..=11, JUMP_AERIAL) | (13..=48, SPECIAL_LW) | (49..=77, DISCHARGE) => (neutral, 0),
            (CAST_AGE, JUMP_AERIAL) if can_down && up_available => ([0.0, -1.0], SPECIAL),
            (78 | 82, DISCHARGE) if up_available => (UP, SPECIAL),
            (79..=81, DISCHARGE) => (UP, 0),
            (83, DISCHARGE) | (84..=97, UP_START) | (98..=101, UP_TRAVEL) => (self.aim, 0),
            (102..=119, UP_TRAVEL | UP_END) => (UP, 0),
            _ => return None,
        };
        Some(pad(stick, buttons))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 990288d6 t5-14-b27-1. Trace 12629 is the pad, 12632 its native
    // side-special start, and 12830 is the neutral jump from SIDE_END.
    fn bash() -> Bash {
        Bash {
            frame: 28,
            position: [-93.99, -54.089_99],
            velocity: [0.0, -1.55],
        }
    }

    fn jump() -> Jump {
        Jump {
            frame: 126,
            status: 0x1e6,
            position: [39.447_45, -117.588_8],
            velocity: [1.0625, -1.55],
            jumps: 1,
            guarder: Some([41.502, 0.010_26]),
            up_available: true,
        }
    }

    fn flight() -> Thunder {
        Thunder::from_bash(bash(), [-79.99, 0.0]).expect("measured press and reserve")
    }

    #[test]
    fn pika_skull_bash_only_arms_thunder_on_the_flown_left_underside() {
        use super::super::{skull_bash_start, SkullBashFlight};
        use crate::model::edge_juggle::perfect::{
            niche::{self, Motion, Sample, Sequence},
            Outline,
        };

        let ledges = [[-79.986, 0.0], [79.989, 0.0]];
        let collision = [
            [[-79.986, -8.319], [-64.395, -25.111]],
            [[-64.395, -25.111], [-41.396, -40.099]],
            [[-41.396, -40.099], [41.416, -40.118]],
            [[41.416, -40.118], [64.387, -25.103]],
            [[64.387, -25.103], [79.989, -8.319]],
        ];
        let make = |position, speed, altered: bool| {
            let mut lines = collision;
            if altered {
                lines[2][0][1] += 2.0;
            }
            let path = skull_bash_start((3, 0), &lines, ledges, position, -140.0, 9.0).unwrap();
            let mut outline = Outline::new();
            for line in lines {
                outline.add(line);
            }
            SkullBashFlight::new(path, outline, -140.0).with_measured_thunder(
                ledges,
                position,
                speed,
                path.direction,
            )
        };
        let start = [-93.986, -22.0];
        let speed = [0.0, -0.095];
        let row = niche::measured_row(3, niche::CANDIDATE_ROWS).unwrap();
        let mut flight = make(start, speed, false);
        let mut sequence = Sequence::new(row);
        let sample = Sample {
            frame: 0,
            motion: Motion::Free,
            can_start: true,
            valid: true,
            at_exit: true,
            can_enter: true,
            low: false,
            hold_over: false,
            direction: 1.0,
            aim: [0.0; 2],
        };
        assert_eq!(
            flight.step(&mut sequence, sample, start, speed[1], 1),
            Some(pad([0.0; 2], 0))
        );

        let press = Bash {
            position: [-93.986, -54.089_99],
            ..bash()
        };
        let input = flight.step(
            &mut sequence,
            Sample {
                frame: press.frame,
                ..sample
            },
            press.position,
            press.velocity[1],
            1,
        );
        assert_eq!(input, Some(pad([1.0, 0.0], SPECIAL)));
        flight.observe_bash_pad(press);
        assert!(flight.thunder.is_some());

        for mut normal in [
            make(start, speed, true),
            make([93.989, -22.0], speed, false),
            make([-93.986, -55.0], speed, false),
        ] {
            normal.observe_bash_pad(press);
            assert!(normal.thunder.is_none());
        }
    }

    #[test]
    fn timed_pikachu_thunder_issues_down_and_b_on_the_same_frame() {
        let mut flight = flight();
        assert_eq!(flight.begin(jump()), Some(pad([0.0, 0.0], JUMP)));
        assert_eq!(flight.step(127, 0xc, true, true), Some(pad([0.0, 0.0], 0)));
        assert_eq!(flight.step(137, 0xc, true, true), Some(pad([0.0, 0.0], 0)));
        let thunder = Some(pad([0.0, -1.0], SPECIAL));
        assert_eq!(flight.step(138, 0xc, true, true), thunder);
        assert_eq!(flight.step(138, 0xc, true, true), thunder);
        assert_eq!(
            flight.step(139, 0x1df, false, true),
            Some(pad([0.0, 0.0], 0))
        );
        assert_eq!(
            flight.step(203, 0x1e7, false, true),
            Some(pad([0.0, 0.0], 0))
        );
        assert_eq!(
            flight.step(204, 0x1e7, false, true),
            Some(pad([0.0, 1.0], SPECIAL))
        );
        assert_eq!(
            flight.step(205, 0x1e7, false, true),
            Some(pad([0.0, 1.0], 0))
        );
        assert_eq!(
            flight.step(208, 0x1e7, false, true),
            Some(pad([0.0, 1.0], SPECIAL))
        );
        assert_eq!(
            flight.step(209, 0x1e7, false, true),
            Some(pad([107.0 / 120.0, 54.0 / 120.0], 0))
        );
        assert_eq!(
            flight.step(228, 0x1e1, false, true),
            Some(pad([0.0, 1.0], 0))
        );
    }

    #[test]
    fn pikachu_only_commits_the_thunder_finish_from_its_measured_native_window() {
        for press in [
            Bash {
                position: [bash().position[0], -58.0],
                ..bash()
            },
            Bash {
                position: [93.99, -54.09],
                ..bash()
            },
            Bash {
                velocity: [0.0, -2.48],
                ..bash()
            },
        ] {
            assert!(Thunder::from_bash(press, [-79.99, 0.0]).is_none());
        }
        for beat in [
            Jump {
                frame: 125,
                ..jump()
            },
            Jump {
                status: 0xe,
                ..jump()
            },
            Jump {
                position: [39.447_45, -115.0],
                ..jump()
            },
            Jump { jumps: 0, ..jump() },
            Jump {
                up_available: false,
                ..jump()
            },
            Jump {
                guarder: None,
                ..jump()
            },
            Jump {
                guarder: Some([47.0, 0.0]),
                ..jump()
            },
            Jump {
                guarder: Some([41.502, 24.0]),
                ..jump()
            },
        ] {
            assert_eq!(flight().begin(beat), None);
        }
        assert!(measured_approach(
            [-93.99, -22.0],
            [0.0, -0.095],
            [-79.99, 0.0]
        ));
        assert!(!measured_approach(
            [-93.99, -55.0],
            [0.0, -0.095],
            [-79.99, 0.0]
        ));
        assert!(above_bash_line(-52.5, 0.0));
        assert!(!above_bash_line(bash().position[1], 0.0));

        let mut flight = flight();
        assert!(!flight.active());
        assert!(flight.begin(jump()).is_some());
        assert!(flight.active());
        assert!(flight.cast_due(138));
        assert!(flight.step(138, 0xc, false, true).is_none());
        assert!(flight.step(139, 0xc, false, true).is_none());
        assert!(flight.step(204, 0x1e7, false, false).is_none());
    }

    #[test]
    fn pikachu_thunder_uses_the_steeper_zip_below_fourteen_out() {
        let mut flight = Thunder::from_bash(
            Bash {
                position: [-87.99, -53.615],
                ..bash()
            },
            [-79.99, 0.0],
        )
        .unwrap();
        assert!(flight
            .begin(Jump {
                position: [45.447_45, -117.113_8],
                guarder: Some([47.502, 0.0]),
                ..jump()
            })
            .is_some());
        assert_eq!(
            flight.step(209, 0x1e7, false, true),
            Some(pad([90.0 / 120.0, 79.0 / 120.0], 0)),
        );
    }
}
