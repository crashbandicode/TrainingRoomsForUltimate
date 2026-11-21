//! Measured Mario Cape timing mixup (M-CS): one Cape, then the stock return.

use super::{niche::pad, recovery::Zone, region, Correction, Point, RouteFacts, SPECIAL};

pub const KIND: i32 = 0x00;
pub const ROW: u8 = 45;
const FALL: i32 = 0xe;
const CAPE: i32 = 0x1dd;
const MACRO_FRAMES: u32 = 42;
const EARLIEST_PRESS_SPEED: f32 = -0.173;
const LATEST_PRESS_SPEED: f32 = -0.175;
const GRAVITY: f32 = 0.087;
const TERMINAL_FALL: f32 = -1.5;
const LEAD_IN: u32 = 30;

// fusion-g4 g4/tables/mario-mcs.txt. M15F flew all four corner cells; M15B
// and M15W confirmed the shared near geometry. 4-by-4 cells in the common ring.
#[rustfmt::skip]
const MARIO_CS_REST_RING: [u32; 22] = [
    0, 0, 0,
    0x07ffff00, 0x07ffff00, 0x07ffff00, 0x07ffff00, 0x07ffff00, 0x07ffff00,
    0x07ffff00, 0x07ffff00, 0x07ffff00,
    0x0001ff00, 0x0001ff00, 0x0001ff00, 0x0001ff00, 0x0001ff00, 0x0001ff00,
    0x0001ff00, 0x0001ff00, 0x0001ff00, 0x0001ff00,
];

// M17 and M18 flew the terminal-speed entry on normal Final Destination.
#[rustfmt::skip]
const MARIO_CS_RING: [u32; 22] = [
    0, 0, 0,
    0x07ffff00, 0x07ffff00, 0x07ffff00, 0x07ffff00, 0x07ffff00, 0x07ffff00,
    0x0001ff00, 0x0001ff00, 0x0001ff00, 0x0001ff00, 0x0001ff00, 0x0001ff00,
    0x0001ff00, 0x0001ff00, 0x0001ff00, 0x0001ff00, 0x0001ff00, 0x0001ff00, 0x0001ff00,
];

pub const fn reference_stage(stage: (i32, u32)) -> bool {
    matches!(stage, (0 | 3 | 311, 0))
}

fn in_region(table: &[u32], near: Point, position: Point) -> bool {
    region::contains(table, near[0] - position[0], position[1] - near[1])
}

fn terminal_entry(near: Point, entry: CapeBeat) -> Option<u32> {
    let [x, mut y] = entry.position;
    let mut vy = entry.velocity[1];
    // The release sample can follow zero, one or two native fall updates.
    let elapsed = (-vy / GRAVITY).round() as u32;
    let remaining = LEAD_IN - elapsed;
    for _ in 0..remaining {
        vy = (vy - GRAVITY).max(TERMINAL_FALL);
        y += vy;
    }
    in_region(&MARIO_CS_RING, near, [x, y]).then_some(entry.frame + remaining)
}

#[derive(Clone, Copy)]
pub struct CapeBeat {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub jumps: u8,
    pub facing: f32,
    pub can_cape: bool,
    pub can_rise: bool,
}

pub struct CapeStall {
    near: Point,
    terminal_press: Option<u32>,
    pressed: Option<u32>,
    observed: bool,
    latest: Option<(u32, Option<Correction>)>,
}

impl CapeStall {
    pub fn start(facts: &RouteFacts<'_>, zone: Zone, draw: u32, entry: CapeBeat) -> Option<Self> {
        let [near, far] = facts.ledges;
        if !reference_stage(facts.stage)
            || !matches!(zone, Zone::Ledge | Zone::Hang)
            || draw >= 1 << 30
            || entry.frame > 1
            || !Self::fresh_fall(entry)
            || !(LATEST_PRESS_SPEED..=0.0).contains(&entry.velocity[1])
            || !region::has_common_underside([near, far], facts.blast[3], facts.outline)
        {
            return None;
        }
        let terminal_press = if in_region(&MARIO_CS_REST_RING, near, entry.position) {
            None
        } else if facts.stage == (3, 0) {
            Some(terminal_entry(near, entry)?)
        } else {
            return None;
        };
        Some(Self {
            near,
            terminal_press,
            pressed: None,
            observed: false,
            latest: None,
        })
    }

    pub fn awaiting_press(&self) -> bool {
        self.pressed.is_none()
    }

    pub fn step(&mut self, beat: CapeBeat) -> Option<Correction> {
        if let Some((_, pad)) = self.latest.filter(|(frame, _)| *frame == beat.frame) {
            return pad;
        }
        let pad = self.next_pad(beat);
        self.latest = Some((beat.frame, pad));
        pad
    }

    fn next_pad(&mut self, beat: CapeBeat) -> Option<Correction> {
        if let Some(pressed) = self.pressed {
            let age = beat.frame.checked_sub(pressed)?;
            if age >= MACRO_FRAMES {
                return self.observed.then_some(Correction::default());
            }
            if beat.status != CAPE {
                return None;
            }
            self.observed = true;
            return Some(if age == 1 {
                pad([1.0, 0.0], SPECIAL)
            } else {
                pad([0.0, 0.0], 0)
            });
        }
        if !Self::fresh_fall(beat) {
            return None;
        }
        if let Some(frame) = self.terminal_press {
            if beat.frame < frame && (TERMINAL_FALL..=0.0).contains(&beat.velocity[1]) {
                return Some(pad([0.0; 2], 0));
            }
            if beat.frame != frame
                || beat.velocity[1] != TERMINAL_FALL
                || !in_region(&MARIO_CS_RING, self.near, beat.position)
            {
                return None;
            }
        } else {
            if beat.frame > 4
                || !(LATEST_PRESS_SPEED..=0.0).contains(&beat.velocity[1])
                || !in_region(&MARIO_CS_REST_RING, self.near, beat.position)
            {
                return None;
            }
            if beat.velocity[1] > EARLIEST_PRESS_SPEED {
                // The at-rest proof has two uncontested Fall updates before pressing.
                return Some(pad([0.0; 2], 0));
            }
        }
        if !beat.can_cape {
            return None;
        }
        self.pressed = Some(beat.frame);
        Some(pad([1.0, 0.0], SPECIAL))
    }

    fn fresh_fall(beat: CapeBeat) -> bool {
        beat.status == FALL
            && beat.facing == 1.0
            && beat.jumps == 1
            && beat.can_rise
            && beat.velocity[0] == 0.0
            && beat.position.into_iter().all(f32::is_finite)
    }
}

#[cfg(test)]
lib_unit_tests! {
    mod tests {
        use super::*;
        use super::super::Segment;

        const LEFT: Point = [-79.986, 0.0];
        const RIGHT: Point = [79.989, 0.0];
        const OUTLINE: [Segment; 8] = [
            [LEFT, [-79.986, -8.319]],
            [[-79.986, -8.319], [-64.395, -25.111]],
            [[-64.395, -25.111], [-41.396, -40.099]],
            [[-41.396, -40.099], [41.416, -40.118]],
            [[41.416, -40.118], [64.387, -25.103]],
            [[64.387, -25.103], [79.989, -8.319]],
            [[79.989, -8.319], RIGHT],
            [RIGHT, LEFT],
        ];

        const START: CapeBeat = CapeBeat {
            frame: 0,
            status: FALL,
            position: [-145.986, -46.0],
            velocity: [0.0; 2],
            jumps: 1,
            facing: 1.0,
            can_cape: true,
            can_rise: true,
        };

        fn facts(stage: (i32, u32), position: Point) -> RouteFacts<'static> {
            RouteFacts {
                fighter: KIND,
                stage,
                position,
                player: LEFT,
                player_reach: 8.0,
                ledges: [LEFT, RIGHT],
                blast: [-230.0, 230.0, 185.0, -140.0],
                outline: &OUTLINE,
                body_clearance: 8.0,
                ledge_distance: 8.0,
                fall_budget: 1.5,
                reach: [140.0, 85.0],
            }
        }

        #[test]
        fn mario_cape_admits_the_proven_corners_and_rejects_far_low_starts() {
            assert_eq!(MARIO_CS_REST_RING.iter().map(|word| word.count_ones()).sum::<u32>(), 261);
            assert_eq!(MARIO_CS_RING.iter().map(|word| word.count_ones()).sum::<u32>(), 231);
            for stage in [(3, 0), (0, 0), (311, 0)] {
                for (outside, height) in [(66.0, -46.0), (34.0, -46.0), (106.0, -6.0), (106.0, 26.0)] {
                    let position = [LEFT[0] - outside, height];
                    let beat = CapeBeat { position, ..START };
                    assert!(CapeStall::start(&facts(stage, position), Zone::Ledge, 0, beat).is_some());
                }
            }
            for (outside, height) in [(110.0, -30.0), (90.0, -50.0), (31.0, 0.0), (-34.0, 0.0)] {
                let position = [LEFT[0] - outside, height];
                let beat = CapeBeat { position, ..START };
                assert!(CapeStall::start(&facts((3, 0), position), Zone::Ledge, 0, beat).is_none());
            }
        }

        #[test]
        fn mario_cape_replays_the_terminal_entry_and_respects_its_far_low_boundary() {
            // M18 mcs-tf-t: press after 30 neutral rows from (106 out, +38.81).
            let outside = 106.0;
            let position = [LEFT[0] - outside, 38.81];
            let initial = CapeBeat { position, ..START };
            let mut flight = CapeStall::start(&facts((3, 0), position), Zone::Ledge, 0, initial).unwrap();
            let mut y = position[1];
            let mut vy: f32 = 0.0;
            for frame in 0..=30 {
                let live = CapeBeat { frame, position: [position[0], y], velocity: [0.0, vy], ..initial };
                let expected = if frame == 30 {
                    Some(pad([1.0, 0.0], SPECIAL))
                } else {
                    Some(pad([0.0; 2], 0))
                };
                assert_eq!(flight.step(live), expected, "frame {frame}");
                assert_eq!(flight.step(live), expected, "repeated think {frame}");
                vy = (vy - 0.087).max(-1.5);
                y += vy;
            }
            assert_eq!(flight.step(CapeBeat { frame: 31, status: CAPE, ..initial }), Some(pad([1.0, 0.0], SPECIAL)));
            assert_eq!(flight.step(CapeBeat { frame: 32, status: CAPE, ..initial }), Some(pad([0.0; 2], 0)));
            assert_eq!(flight.step(CapeBeat { frame: 72, status: CAPE, ..initial }), Some(Correction::default()));

            // The rest table admits the at-rest corner below, while M17
            // returned only once in five when pressing at terminal speed there.
            let outside_press = CapeBeat { frame: 30, position: [position[0], -6.0], velocity: [0.0, -1.5], ..initial };
            let mut drift = CapeStall::start(&facts((3, 0), position), Zone::Ledge, 0, initial).unwrap();
            assert!(drift.step(outside_press).is_none());
        }

        #[test]
        fn mario_cape_requires_a_fresh_release_with_resources_and_ledge_pressure() {
            let fd = facts((3, 0), START.position);
            for no in [
                CapeBeat { velocity: [0.01, -0.087], ..START },
                CapeBeat { velocity: [0.0, -1.5], ..START },
                CapeBeat { velocity: [0.0, f32::NAN], ..START },
                CapeBeat { facing: -1.0, ..START },
                CapeBeat { frame: 2, ..START },
                CapeBeat { jumps: 0, ..START },
                CapeBeat { can_rise: false, ..START },
                CapeBeat { status: CAPE, ..START },
            ] {
                assert!(CapeStall::start(&fd, Zone::Ledge, 0, no).is_none());
            }
            let never = |zone, draw| CapeStall::start(&fd, zone, draw, START).is_none();
            assert!(never(Zone::Far, 0));
            assert!(never(Zone::OffHigh, 0));
            assert!(never(Zone::Ledge, 1 << 30));
            assert!(CapeStall::start(&fd, Zone::Hang, (1 << 30) - 1, START).is_some());
            assert!(CapeStall::start(&facts((3, 1), START.position), Zone::Ledge, 0, START).is_none());
            assert!(CapeStall::start(&RouteFacts { outline: &[], ..fd }, Zone::Ledge, 0, START).is_none());
        }

        #[test]
        fn mario_cape_replays_the_measured_press_neutral_and_stock_tail() {
            let fd = facts((3, 0), START.position);
            let mut cape = CapeStall::start(&fd, Zone::Ledge, 0, START).unwrap();
            let neutral = Some(pad([0.0; 2], 0));
            let press = Some(pad([1.0, 0.0], SPECIAL));
            // Exact M15F mcs-ca-1 native velocity and statuses at sequences 1140..1149.
            for (frame, status, velocity, expected) in [
                (0, FALL, [0.0, 0.0], neutral),
                (1, FALL, [0.0, -0.087], neutral),
                (2, FALL, [0.0, -0.174], press),
                (3, CAPE, [0.080, -0.261], press),
                (4, CAPE, [0.029, -0.348], neutral),
                (43, CAPE, [0.0, -1.5], neutral),
                (44, CAPE, [0.0, -1.5], Some(Correction::default())),
                (60, 0x1de, [0.0, 2.4], Some(Correction::default())),
            ] {
                let beat = CapeBeat { frame, status, velocity, ..START };
                assert_eq!(cape.step(beat), expected, "frame {frame}");
                assert_eq!(cape.step(beat), expected, "repeated think {frame}");
            }
        }

        #[test]
        fn mario_cape_returns_stock_if_the_press_is_unavailable_or_never_accepted() {
            let fd = facts((3, 0), START.position);
            let mut cape = CapeStall::start(&fd, Zone::Ledge, 0, START).unwrap();
            let press = CapeBeat { frame: 2, velocity: [0.0, -0.174], ..START };
            assert_eq!(cape.step(CapeBeat { can_cape: false, ..press }), None);
            let mut cape = CapeStall::start(&fd, Zone::Ledge, 0, START).unwrap();
            assert_eq!(cape.step(press), Some(pad([1.0, 0.0], SPECIAL)));
            assert_eq!(cape.step(CapeBeat { frame: 3, ..press }), None);
            assert_eq!(cape.step(CapeBeat { frame: 44, ..press }), None);
        }
    }
}
