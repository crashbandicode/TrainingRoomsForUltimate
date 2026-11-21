//! Incineroar's measured near-side route spends Whip, jump and Cross Chop in that order.

use super::{niche::pad, region, Correction, Point, RouteFacts, JUMP, SPECIAL};

pub const KIND: i32 = 0x47;
pub const ROW: u8 = 36;
const FALL: i32 = 0xe;
const JUMP_AERIAL: i32 = 0xc;
const WHIP: i32 = 0x1dd;
const CHOP_START: i32 = 0x1de;
const CHOP_TRANSITION: i32 = 0x1e6;
const CHOP_DIVE: i32 = 0x1e7;
const GRAVITY: f32 = 0.126;
const FALL_SPEED: f32 = 1.76;
const JUMP_AT: u32 = 57;
const CHOP_AT: u32 = 83;
const FORWARD: Point = [1.0, 0.0];
const UP: Point = [0.0, 1.0];

// fusion-g3 incineroar-ring.txt, 356 tested-model cells.
#[rustfmt::skip]
const REGION: [u32; 19] = [
    0x00000000, 0x1ffffe00, 0x1fffff00, 0x1fffff80, 0x1fffff80, 0x1fffffc0,
    0x1fffffc0, 0x1fffffc0, 0x0fffffc0, 0x07ffffc0, 0x07ffffc0, 0x03ffffc0,
    0x01ffffc0, 0x00ffffc0, 0x00ffffc0, 0x007fffc0, 0x003fffc0, 0x001fffc0,
    0x001fffc0,
];

fn reaches_floor([x, y]: Point) -> bool {
    y >= 12.0 && (8.0..=120.0).contains(&(x + 0.754 * y))
}

fn initial_top(outside: f32, height: f32, vy: f32) -> Point {
    // Measured Whip +54.49 and -23.79, then one carried fall row, 25
    // rising jump rows and the first carried row of Cross Chop.
    let descent = (vy - GRAVITY).max(-FALL_SPEED);
    [-outside + 82.798, height + descent + 54.286]
}

#[derive(Clone, Copy)]
pub struct WhipBeat {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub facing: f32,
    pub jumps: u8,
    pub can_whip: bool,
    pub can_jump: bool,
    pub can_chop: bool,
}

pub struct WhipReturn {
    near: Point,
    origin: u32,
    pressed: Point,
    latest: Option<(u32, Correction)>,
}

impl WhipReturn {
    pub fn start(facts: &RouteFacts<'_>, draw: u32, beat: WhipBeat) -> Option<Self> {
        let [near, _] = facts.ledges;
        let [x, y] = beat.position;
        let outside = near[0] - x;
        let height = y - near[1];
        let [vx, vy] = beat.velocity;
        // If the guarder can intersect Whip, the grab changes its movement.
        let [px, py] = facts.player;
        let reach = facts.player_reach;
        let can_grab = (x - reach..=x + 55.0 + reach).contains(&px)
            && (y - 25.5 - reach..=y + reach).contains(&py);
        let draw_limit = if outside >= 100.0 {
            u32::MAX - (1 << 30) + 1
        } else {
            1 << 30
        };
        if !super::is_crossing_stage(facts.stage)
            || facts.fighter != KIND
            || facts.position != beat.position
            || beat.frame > 2
            || beat.status != FALL
            || beat.facing != 1.0
            || beat.jumps != 1
            || !beat.can_whip
            || !beat.can_chop
            || !(vx.abs() <= 0.3 && (-FALL_SPEED..=0.0).contains(&vy))
            || !(px.is_finite() && py.is_finite() && reach.is_finite() && reach > 0.0)
            || draw >= draw_limit
            || can_grab
            || !region::has_common_underside(facts.ledges, facts.blast[3], facts.outline)
            || !region::contains(&REGION, outside, height)
            || !reaches_floor(initial_top(outside, height, vy))
        {
            return None;
        }
        Some(Self {
            near,
            origin: beat.frame,
            pressed: beat.position,
            latest: None,
        })
    }

    pub fn step(&mut self, beat: WhipBeat) -> Option<Correction> {
        if let Some((frame, correction)) = self.latest {
            if frame == beat.frame {
                return Some(correction);
            }
        }
        let correction = self.next_pad(beat)?;
        self.latest = Some((beat.frame, correction));
        Some(correction)
    }

    pub fn whip_frame(&self, frame: u32) -> bool {
        frame == self.origin
    }

    pub fn jump_frame(&self, frame: u32) -> bool {
        frame.checked_sub(self.origin) == Some(JUMP_AT)
    }

    pub fn chop_frame(&self, frame: u32) -> bool {
        frame.checked_sub(self.origin) == Some(CHOP_AT)
    }

    fn next_pad(&self, beat: WhipBeat) -> Option<Correction> {
        let age = beat.frame.checked_sub(self.origin)?;
        if !beat.position.into_iter().all(f32::is_finite) {
            return None;
        }
        if age == 0 {
            return (beat.status == FALL && beat.can_whip).then(|| pad(FORWARD, SPECIAL));
        }
        if age < JUMP_AT {
            return (beat.status == WHIP).then(|| pad([0.0; 2], 0));
        }
        if age == JUMP_AT {
            let [dx, dy] = core::array::from_fn(|i| beat.position[i] - self.pressed[i]);
            // The native buffer accepts jump on the last Whip row. If a
            // different status took over, there is no measured route.
            return (beat.jumps == 1
                && beat.facing == 1.0
                && (beat.status == WHIP || beat.status == FALL && beat.can_jump)
                && (52.0..=57.0).contains(&dx)
                && (-27.0..=-22.0).contains(&dy))
            .then(|| pad(FORWARD, JUMP));
        }
        if age < CHOP_AT {
            return (beat.status == JUMP_AERIAL).then(|| pad(FORWARD, 0));
        }
        if age == CHOP_AT {
            let [x, y] = core::array::from_fn(|i| beat.position[i] - self.near[i]);
            let [vx, vy] = beat.velocity;
            let top = [
                x + vx - 0.015 + 5.0,
                y + (vy - GRAVITY).max(-FALL_SPEED) + 47.67,
            ];
            return (beat.status == JUMP_AERIAL
                && beat.jumps == 0
                && beat.facing == 1.0
                && beat.can_chop
                // Native speed is the difference of two f32 world positions.
                && (vx - 0.88).abs() <= 0.00002
                && (-0.4..=0.0).contains(&vy)
                && reaches_floor(top))
            .then(|| pad(UP, SPECIAL));
        }
        if age == CHOP_AT + 1 {
            return (beat.status == CHOP_START).then(|| pad(UP, SPECIAL));
        }
        (age <= 185 && matches!(beat.status, CHOP_START | CHOP_TRANSITION | CHOP_DIVE))
            .then(|| pad([0.0; 2], 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::edge_juggle::perfect::Segment;

    const NEAR: Point = [-79.986, 0.0];
    const FAR: Point = [79.989, 0.0];
    const OUTLINE: [Segment; 5] = [
        [[-79.986, -8.319], [-64.395, -25.111]],
        [[-64.395, -25.111], [-41.396, -40.099]],
        [[-41.396, -40.099], [41.416, -40.118]],
        [[41.416, -40.118], [64.387, -25.103]],
        [[64.387, -25.103], [79.989, -8.319]],
    ];
    const RELEASE: WhipBeat = WhipBeat {
        position: [-189.986, 10.0],
        velocity: [0.0; 2],
        frame: 0,
        status: FALL,
        facing: 1.0,
        jumps: 1,
        can_whip: true,
        can_jump: true,
        can_chop: true,
    };

    fn facts(stage: (i32, u32), position: Point) -> RouteFacts<'static> {
        RouteFacts {
            fighter: KIND,
            stage,
            position,
            player: FAR,
            player_reach: 14.0,
            ledges: [NEAR, FAR],
            blast: [-230.0, 230.0, 185.0, -140.0],
            outline: &OUTLINE,
            body_clearance: 14.0,
            ledge_distance: 14.0,
            fall_budget: 1.76,
            reach: [150.0, 80.0],
        }
    }

    #[test]
    fn incineroar_chop_top_matches_the_reference_generator_at_both_entry_speeds() {
        for (vy, expected) in [(0.0, [-27.202, 64.16]), (-1.76, [-27.202, 62.526])] {
            let got = initial_top(110.0, 10.0, vy);
            for (actual, expected) in got.into_iter().zip(expected) {
                assert!((actual - expected).abs() < 0.001, "{got:?}");
            }
        }
    }

    #[test]
    fn incineroar_whip_accepts_measured_long_starts_and_avoids_grabbing_p1() {
        assert_eq!(
            REGION.iter().map(|word| word.count_ones()).sum::<u32>(),
            356
        );
        for stage in [(0, 0), (3, 0), (311, 0)] {
            let stage_facts = facts(stage, RELEASE.position);
            assert!(WhipReturn::start(&stage_facts, u32::MAX / 2, RELEASE).is_some());
            assert!(WhipReturn::start(&stage_facts, u32::MAX, RELEASE).is_none());
            assert!(WhipReturn::start(
                &stage_facts,
                0,
                WhipBeat {
                    jumps: 0,
                    ..RELEASE
                }
            )
            .is_none());
        }
        let left = WhipBeat {
            position: [-119.986, 26.0],
            ..RELEASE
        };
        let unsafe_player = RouteFacts {
            player: [-79.0, 15.0],
            ..facts((3, 0), left.position)
        };
        assert!(WhipReturn::start(&unsafe_player, 0, left).is_none());
        let beyond = WhipBeat {
            position: [-229.986, 10.0],
            ..RELEASE
        };
        assert!(WhipReturn::start(&facts((3, 0), beyond.position), 0, beyond).is_none());
    }

    #[test]
    fn incineroar_chop_tracks_the_traced_native_position_deltas() {
        let release = WhipBeat {
            position: [-119.986, 26.0],
            ..RELEASE
        };
        let mut flight = WhipReturn::start(&facts((3, 0), release.position), 0, release).unwrap();
        let actual = WhipBeat {
            frame: 83,
            status: JUMP_AERIAL,
            position: [-43.049782, 32.827892],
            velocity: [0.880001, -0.212006],
            jumps: 0,
            ..release
        };
        assert_eq!(flight.step(actual), Some(pad(UP, SPECIAL)));
    }

    #[test]
    fn incineroar_whip_buffers_the_jump_and_checks_chop_dive_before_spending_it() {
        let stage_facts = facts((3, 0), RELEASE.position);
        let mut flight = WhipReturn::start(&stage_facts, 0, RELEASE).unwrap();
        assert_eq!(flight.step(RELEASE), Some(pad(FORWARD, SPECIAL)));
        assert_eq!(
            flight.step(WhipBeat {
                frame: 1,
                status: WHIP,
                ..RELEASE
            }),
            Some(pad([0.0; 2], 0))
        );
        let jump = WhipBeat {
            frame: 57,
            status: WHIP,
            position: [-135.50, -13.91],
            can_jump: false,
            ..RELEASE
        };
        assert!(flight.jump_frame(jump.frame));
        assert_eq!(flight.step(jump), Some(pad(FORWARD, JUMP)));
        assert_eq!(flight.step(jump), Some(pad(FORWARD, JUMP)));
        let lift = WhipBeat {
            frame: 58,
            status: JUMP_AERIAL,
            jumps: 0,
            ..jump
        };
        assert_eq!(flight.step(lift), Some(pad(FORWARD, 0)));
        let chop = WhipBeat {
            frame: 83,
            position: [-113.494, 17.18],
            velocity: [0.88, -0.212],
            ..lift
        };
        assert!(flight.chop_frame(chop.frame));
        assert_eq!(flight.step(chop), Some(pad(UP, SPECIAL)));
        assert_eq!(
            flight.step(WhipBeat {
                frame: 84,
                status: CHOP_START,
                ..chop
            }),
            Some(pad(UP, SPECIAL))
        );
        assert_eq!(
            flight.step(WhipBeat {
                frame: 85,
                status: CHOP_START,
                ..chop
            }),
            Some(pad([0.0; 2], 0))
        );

        let mut unsafe_flight = WhipReturn::start(&stage_facts, 0, RELEASE).unwrap();
        assert!(unsafe_flight
            .step(WhipBeat {
                position: [-113.494, -50.0],
                ..chop
            })
            .is_none());
        assert!(unsafe_flight
            .step(WhipBeat {
                can_chop: false,
                ..chop
            })
            .is_none());
    }
}
