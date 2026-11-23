//! Steve crosses the common ring with Elytra and a measured quarter-stick pull-up.

use super::{niche::pad, region, Correction, Point, RouteFacts, SPECIAL};

pub const KIND: i32 = 0x58;
pub const ROW: u8 = 15;
const FALL: i32 = 0xe;
const ELYTRA_START: i32 = 0x20b;
const ELYTRA_GLIDE: i32 = 0x20c;
const FALL_SPEED: f32 = 1.42;
// Battlefield frame 21 measures the next f32 value past the nominal cap.
const NATIVE_FALL_CAP: f32 = 1.420_000_1;
// The 30-above reference pressed no earlier than -47.18. Its preceding
// row reached -46.18 when the macro started late. Use that measured gap.
const PRESS_Y: f32 = -46.25;
const PULL_X: f32 = 85.3;
const PULL_WINDOW_END: f32 = 91.3;

// fusion-g3 g3/steve-ring.txt, 217 cells and one inset cell of margin.
#[rustfmt::skip]
const REGION: [u32; 26] = [
    0x00000000, 0x00001ff0, 0x00001ff0, 0x00001ff0, 0x00001ff0, 0x00001ff0,
    0x00001ff0, 0x00001ff0, 0x00001ff0, 0x00001ff0, 0x00001ff0, 0x00001ff0,
    0x00001ff0, 0x00001ff0, 0x00001ff0, 0x00001ff0, 0x00001ff0, 0x00001ff0,
    0x00001ff0, 0x00001ff0, 0x00001ff0, 0x00001ff0, 0x00001ff0, 0x00001ff0,
    0x000007f0, 0x00000070,
];

fn safe_press(outside: f32, height: f32) -> bool {
    if !(12.0..=58.0).contains(&outside) || !(-70.0..=-40.0).contains(&height) {
        return false;
    }
    let pull = 55.0 + ((outside - 15.0) / 1.667).round();
    let sink = 12.4 + 0.39 * (pull.min(67.0) - 55.0) + 0.5 * (pull - 67.0).max(0.0);
    height - 1.45 - sink - 1.25 >= -86.0
}

#[derive(Clone, Copy)]
pub struct ElytraBeat {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub facing: f32,
    pub can_rise: bool,
}

pub struct ElytraFlight {
    near: Point,
    far: Point,
    origin: u32,
    press: Option<u32>,
    glide_seen: bool,
    pull_up: bool,
    crossing_frames: u16,
    latest: Option<(u32, Correction)>,
}

impl ElytraFlight {
    pub fn start(facts: &RouteFacts<'_>, beat: ElytraBeat) -> Option<Self> {
        facts.crossing_band()?;
        let [near, far] = facts.ledges;
        let outside = near[0] - beat.position[0];
        let height = beat.position[1] - near[1];
        if facts.fighter != KIND
            || facts.position != beat.position
            || beat.frame > 2
            || !Self::unspent(beat)
            || !region::has_common_underside(facts.ledges, facts.blast[3], facts.outline)
            || !region::contains(&REGION, outside, height)
            || !safe_press(outside, height.min(PRESS_Y))
        {
            return None;
        }
        // Upper bound on waiting, including reaching terminal speed from rest.
        let drop = (height - PRESS_Y).max(0.0);
        let fall_frames = if drop > 0.0 {
            (drop / FALL_SPEED).ceil() as u16 + 21
        } else {
            0
        };
        Some(Self {
            near,
            far,
            origin: beat.frame,
            press: None,
            glide_seen: false,
            pull_up: false,
            crossing_frames: 150 + fall_frames,
            latest: None,
        })
    }

    pub fn exit_point(&self) -> Point {
        self.far
    }

    pub fn crossing_frames(&self) -> u16 {
        self.crossing_frames
    }

    pub fn needs_enabled_special(&self) -> bool {
        self.press.is_none()
    }

    pub fn step(&mut self, beat: ElytraBeat) -> Option<Correction> {
        if let Some((frame, correction)) = self.latest {
            if frame == beat.frame {
                return Some(correction);
            }
        }
        let correction = self.next_pad(beat)?;
        self.latest = Some((beat.frame, correction));
        Some(correction)
    }

    fn next_pad(&mut self, beat: ElytraBeat) -> Option<Correction> {
        let height = beat.position[1] - self.near[1];
        let x = beat.position[0] - self.near[0];
        if !height.is_finite() || !x.is_finite() {
            return None;
        }
        if let Some(press) = self.press {
            let age = beat.frame.checked_sub(press)?;
            return match beat.status {
                FALL | ELYTRA_START if age == 1 => Some(pad([0.0, 1.0], SPECIAL)),
                ELYTRA_START if !self.glide_seen && age <= 7 => Some(pad([0.0; 2], 0)),
                ELYTRA_GLIDE if age <= 217 => {
                    self.glide_seen = true;
                    if !self.pull_up && x >= PULL_X {
                        if x > PULL_WINDOW_END {
                            return None;
                        }
                        self.pull_up = true;
                    }
                    Some(pad([0.0, if self.pull_up { 0.25 } else { 0.0 }], 0))
                }
                _ => None,
            };
        }
        if !Self::unspent(beat) || !region::contains(&REGION, -x, height) {
            return None;
        }
        if height > PRESS_Y {
            let frames = beat.frame.saturating_sub(self.origin) + 1;
            return Some(pad([0.0, -(frames.min(8) as f32) / 8.0], 0));
        }
        if !safe_press(-x, height) {
            return None;
        }
        self.press = Some(beat.frame);
        Some(pad([0.0, 1.0], SPECIAL))
    }

    fn unspent(beat: ElytraBeat) -> bool {
        beat.status == FALL
            && beat.can_rise
            && beat.facing == 1.0
            && beat.velocity[0].abs() <= 0.3
            && (-NATIVE_FALL_CAP..=0.0).contains(&beat.velocity[1])
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::model::edge_juggle::perfect::Segment;

    const NEAR: Point = [-79.986, 0.0];
    const FAR: Point = [79.989, 0.0];
    const RING: [Segment; 5] = [
        [[-79.986, -8.319], [-64.395, -25.111]],
        [[-64.395, -25.111], [-41.396, -40.099]],
        [[-41.396, -40.099], [41.416, -40.118]],
        [[41.416, -40.118], [64.387, -25.103]],
        [[64.387, -25.103], [79.989, -8.319]],
    ];
    const RELEASE: ElytraBeat = ElytraBeat {
        position: [-97.986, 30.0],
        velocity: [0.0; 2],
        frame: 0,
        status: FALL,
        facing: 1.0,
        can_rise: true,
    };

    fn facts(stage: (i32, u32), position: Point) -> RouteFacts<'static> {
        RouteFacts {
            fighter: KIND,
            stage,
            position,
            player: NEAR,
            player_reach: 12.0,
            ledges: [NEAR, FAR],
            blast: [-230.0, 230.0, 185.0, -140.0],
            outline: &RING,
            body_clearance: 12.0,
            ledge_distance: 12.0,
            fall_budget: 1.42,
            reach: [240.0, 75.0],
        }
    }

    #[test]
    fn steve_elytra_draws_only_inside_the_flown_ring() {
        assert_eq!(
            REGION.iter().map(|word| word.count_ones()).sum::<u32>(),
            217
        );
        for stage in [(0, 0), (3, 0), (311, 0)] {
            let plan = ElytraFlight::start(&facts(stage, RELEASE.position), RELEASE).unwrap();
            assert_eq!(plan.exit_point(), FAR);
            assert!(plan.crossing_frames() >= 150);
        }
        for beat in [
            ElytraBeat {
                position: [-149.986, 30.0],
                ..RELEASE
            },
            ElytraBeat {
                position: [-95.986, -96.0],
                ..RELEASE
            },
            ElytraBeat {
                can_rise: false,
                ..RELEASE
            },
            ElytraBeat {
                facing: -1.0,
                ..RELEASE
            },
        ] {
            assert!(ElytraFlight::start(&facts((3, 0), beat.position), beat).is_none());
        }
        for stage in [(89, 0), (3, 1)] {
            assert!(ElytraFlight::start(&facts(stage, RELEASE.position), RELEASE).is_none());
        }
    }

    #[test]
    fn steve_elytra_uses_live_pull_up_position_and_the_measured_pad() {
        let mut flight = ElytraFlight::start(&facts((3, 0), RELEASE.position), RELEASE).unwrap();
        assert_eq!(flight.step(RELEASE), Some(pad([0.0, -0.125], 0)));
        assert_eq!(
            flight.step(ElytraBeat {
                frame: 7,
                ..RELEASE
            }),
            Some(pad([0.0, -1.0], 0))
        );
        let terminal = ElytraBeat {
            frame: 21,
            position: [-97.986, 13.88],
            velocity: [0.0, -1.420_000_1],
            ..RELEASE
        };
        assert_eq!(flight.step(terminal), Some(pad([0.0, -1.0], 0)));
        let before_press = ElytraBeat {
            frame: 63,
            position: [-97.986, -46.179_98],
            velocity: [0.0, -1.419_998],
            ..RELEASE
        };
        assert_eq!(flight.step(before_press), Some(pad([0.0, -1.0], 0)));
        let press = ElytraBeat {
            frame: 64,
            position: [-97.986, -46.4],
            velocity: [0.0, -1.42],
            ..RELEASE
        };
        assert_eq!(flight.step(press), Some(pad([0.0, 1.0], SPECIAL)));
        assert_eq!(flight.step(press), Some(pad([0.0, 1.0], SPECIAL)));
        assert_eq!(
            flight.step(ElytraBeat {
                frame: 65,
                status: ELYTRA_START,
                ..press
            }),
            Some(pad([0.0, 1.0], SPECIAL))
        );
        let glide = ElytraBeat {
            frame: 126,
            status: ELYTRA_GLIDE,
            position: [4.7, -64.2],
            ..press
        };
        assert_eq!(flight.step(glide), Some(pad([0.0; 2], 0)));
        let turn = ElytraBeat {
            frame: 127,
            position: [6.3, -64.7],
            ..glide
        };
        assert_eq!(flight.step(turn), Some(pad([0.0, 0.25], 0)));
        assert_eq!(
            flight.step(ElytraBeat {
                frame: 160,
                position: [60.0, -40.0],
                ..turn
            }),
            Some(pad([0.0, 0.25], 0))
        );
        assert!(flight
            .step(ElytraBeat {
                frame: 161,
                status: 0x4b,
                ..turn
            })
            .is_none());

        // By 95.3 in, every forced late pull-up missed. Do not issue it.
        let mut late = ElytraFlight::start(&facts((3, 0), RELEASE.position), RELEASE).unwrap();
        assert!(late.step(press).is_some());
        assert!(late
            .step(ElytraBeat {
                position: [15.4, -65.0],
                ..glide
            })
            .is_none());
    }
    }
}
