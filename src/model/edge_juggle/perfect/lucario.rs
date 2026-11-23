//! Lucario's measured double jump and Extreme Speed crossing on the common ring.
use super::{niche::pad, region, Correction, Point, RouteFacts, JUMP, SPECIAL};

#[path = "lucario_path.rs"]
mod path;
#[path = "lucario_schedule.rs"]
mod schedule;

pub const KIND: i32 = 0x2c;
pub const ROW: u8 = 29;
pub const FALL: i32 = 0xe;
pub const AERIAL_JUMP: i32 = 0xc;
const SPECIAL_HI: i32 = 0x1de;
const RUSH: i32 = 0x1e6;
const READY: Point = [0.0, 0.0];
const FORWARD: Point = [1.0, 0.0];
const UP: Point = [0.0, 1.0];
const AIM: Point = [0.5, 104.0 / 120.0];

type Candidates = &'static [(u16, [[u8; 3]; 6])];

fn region_at(damage: f32) -> Option<(&'static [u32], Candidates)> {
    match damage {
        100.0 => Some((&schedule::REGION_100, schedule::PLANS_100)),
        130.0 => Some((&schedule::REGION_130, schedule::PLANS_130)),
        _ => None,
    }
}

pub fn wizard_offset(stage: (i32, u32), damage: u16) -> Option<Point> {
    if !super::is_crossing_stage(stage) {
        return None;
    }
    match damage {
        100 => Some([22.0, -14.0]),
        130 => Some([22.0, -22.0]),
        _ => None,
    }
}

#[derive(Clone, Copy)]
pub struct AuraSample {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub damage: f32,
    pub facing: f32,
    pub jumps: u8,
    pub can_jump: bool,
    pub can_rise: bool,
}

pub struct AuraFlight {
    near: Point,
    far: Point,
    chosen: path::Path,
    damage: f32,
    origin: Option<u32>,
    last_pad: Option<(u32, Correction)>,
}

fn local(near: Point, position: Point) -> [f64; 2] {
    [
        path::NEAR + f64::from(position[0]) - f64::from(near[0]),
        f64::from(position[1]) - f64::from(near[1]),
    ]
}

fn near_waypoint(actual: [f64; 2], expected: [f64; 2]) -> bool {
    actual
        .into_iter()
        .zip(expected)
        .all(|(a, b)| (a - b).abs() <= 2.0)
}

impl AuraFlight {
    pub fn start(facts: &RouteFacts<'_>, sample: AuraSample) -> Option<Self> {
        facts.crossing_band()?;
        let [near, far] = facts.ledges;
        let [vx, vy] = sample.velocity;
        let [x, y] = local(near, sample.position);
        let (region, schedules) = region_at(sample.damage)?;
        let outside = near[0] - sample.position[0];
        let height = sample.position[1] - near[1];
        if facts.fighter != KIND
            || facts.position != sample.position
            || !region::has_common_underside(facts.ledges, facts.blast[3], facts.outline)
            || sample.status != FALL
            || sample.facing != 1.0
            || sample.jumps != 1
            || !sample.can_rise
            || vx != 0.0
            || !(-1.68..=0.0).contains(&vy)
            || !region::contains(region, outside, height)
        {
            return None;
        }
        let row = ((40.0 - height) / 4.0) as u16;
        let column = (outside / 4.0) as u16;
        let id = row * 30 + column;
        let mut chosen = None;
        // The entered cell supplies its own schedules. Adjacent cells offer
        // different timings if drift or placement moved off its centre.
        for index in [
            id,
            id.saturating_sub(1),
            id + 1,
            id.saturating_sub(30),
            id + 30,
        ] {
            let Ok(i) = schedules.binary_search_by_key(&index, |entry| entry.0) else {
                continue;
            };
            for timing in schedules[i].1 {
                if timing[2] == 0 {
                    continue;
                }
                let velocity = [f64::from(vx), f64::from(vy)];
                let damage = f64::from(sample.damage);
                let Some(estimate) = path::flight([x, y], velocity, damage, timing) else {
                    continue;
                };
                if path::flight([x, y - 0.1], velocity, damage, timing).is_some() {
                    chosen = Some(estimate);
                    break;
                }
            }
            if chosen.is_some() {
                break;
            }
        }
        Some(Self {
            near,
            far,
            chosen: chosen?,
            damage: sample.damage,
            origin: None,
            last_pad: None,
        })
    }

    pub fn exit_point(&self) -> Point {
        self.far
    }

    pub fn crossing_frames(&self) -> u16 {
        self.chosen.arrival + 8
    }

    fn age(&self, frame: u32) -> u32 {
        frame.wrapping_sub(self.origin.unwrap_or(frame))
    }

    pub fn jump_frame(&self, frame: u32) -> bool {
        self.age(frame) == u32::from(self.chosen.jump)
    }

    pub fn rise_frame(&self, frame: u32) -> bool {
        self.age(frame) == u32::from(self.chosen.press)
    }

    pub fn step(&mut self, sample: AuraSample) -> Option<Correction> {
        let pos = local(self.near, sample.position);
        if !pos.into_iter().all(f64::is_finite) || pos[1] <= -140.0 || sample.damage != self.damage
        {
            return None;
        }
        if let Some((_, correction)) = self.last_pad.filter(|&(frame, _)| frame == sample.frame) {
            return Some(correction);
        }
        let age = self.age(sample.frame);
        let start_jump = u32::from(self.chosen.jump);
        let start_up = u32::from(self.chosen.press);
        let edit = if age < start_jump {
            (sample.status == FALL && sample.jumps == 1).then(|| {
                let stick = if age < u32::from(self.chosen.holds) {
                    READY
                } else {
                    FORWARD
                };
                pad(stick, 0)
            })?
        } else if age == start_jump {
            (sample.status == FALL
                && sample.jumps == 1
                && sample.can_jump
                && sample.facing == 1.0
                && near_waypoint(pos, self.chosen.jump_at))
            .then(|| pad(FORWARD, JUMP))?
        } else if age == start_jump + 1 {
            (matches!(sample.status, FALL | AERIAL_JUMP)).then(|| pad(FORWARD, JUMP))?
        } else if age < start_up {
            (sample.status == AERIAL_JUMP && sample.jumps == 0).then(|| pad(FORWARD, 0))?
        } else if age == start_up {
            (sample.status == AERIAL_JUMP
                && sample.jumps == 0
                && sample.can_rise
                && sample.facing == 1.0
                && near_waypoint(pos, self.chosen.press_at))
            .then(|| pad(UP, SPECIAL))?
        } else if age == start_up + 1 {
            (matches!(sample.status, AERIAL_JUMP | SPECIAL_HI)).then(|| pad(UP, SPECIAL))?
        } else {
            (matches!(sample.status, SPECIAL_HI | RUSH)).then(|| pad(AIM, 0))?
        };
        self.origin.get_or_insert(sample.frame);
        self.last_pad = Some((sample.frame, edit));
        Some(edit)
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::model::edge_juggle::perfect::Segment;

    const RING: [Segment; 5] = [
        [[-79.986, -8.319], [-64.395, -25.111]],
        [[-64.395, -25.111], [-41.396, -40.099]],
        [[-41.396, -40.099], [41.416, -40.118]],
        [[41.416, -40.118], [64.387, -25.103]],
        [[64.387, -25.103], [79.989, -8.319]],
    ];

    fn facts(stage: (i32, u32), position: Point) -> RouteFacts<'static> {
        RouteFacts {
            fighter: KIND,
            stage,
            position,
            player: [-75.0, 0.0],
            player_reach: 14.0,
            ledges: [[-79.986, 0.0], [79.989, 0.0]],
            blast: [-230.0, 230.0, 180.0, -140.0],
            outline: &RING,
            body_clearance: 10.5,
            ledge_distance: 22.0,
            fall_budget: 1.68,
            reach: [100.0, 130.0],
        }
    }

    fn fresh(damage: f32, position: Point, vy: f32) -> AuraSample {
        AuraSample {
            frame: 210,
            status: FALL,
            position,
            velocity: [0.0, vy],
            facing: 1.0,
            jumps: 1,
            damage,
            can_jump: true,
            can_rise: true,
        }
    }

    #[test]
    fn aura_flight_replays_exact_100_and_130_native_directional_pads() {
        // Native frame rows from fg2-f2-yoga-10, first attempt at each default.
        // Playback resets kinetic speed at frame 0 while retaining the live position.
        // The in-product guide instead plans from the measured speed below.
        for (damage, position, vy, hold, jump, up, jump_at, up_at) in [
            (
                100.0,
                [-101.986, -14.0],
                0.0,
                8,
                66,
                107,
                [-36.1721, -108.92],
                [15.5249, -77.2403],
            ),
            (
                130.0,
                [-101.986, -22.252],
                0.0,
                0,
                54,
                101,
                [-41.2961, -97.012],
                [18.0869, -71.9204],
            ),
        ] {
            for stage in [(0, 0), (3, 0), (311, 0)] {
                let beat = fresh(damage, position, vy);
                let mut flight = AuraFlight::start(&facts(stage, position), beat)
                    .unwrap_or_else(|| panic!("measured {damage}% release on {stage:?}"));
                assert_eq!(flight.exit_point(), [79.989, 0.0]);
                assert_eq!(
                    (flight.chosen.holds, flight.chosen.jump, flight.chosen.press),
                    (hold, jump, up),
                );
                assert!((130..=191).contains(&flight.crossing_frames()));
                for age in 0..=up + 30 {
                    let (status, jumps) = if age <= jump + 1 {
                        (FALL, 1)
                    } else if age <= up + 1 {
                        (AERIAL_JUMP, 0)
                    } else if age < up + 30 {
                        (SPECIAL_HI, 0)
                    } else {
                        (RUSH, 0)
                    };
                    let actual = if age == jump {
                        jump_at
                    } else if age == up {
                        up_at
                    } else {
                        position
                    };
                    let input = AuraSample {
                        frame: beat.frame + u32::from(age),
                        status,
                        jumps,
                        position: actual,
                        ..beat
                    };
                    let expected = if age < hold {
                        pad(READY, 0)
                    } else if (jump..=jump + 1).contains(&age) {
                        pad(FORWARD, JUMP)
                    } else if age < up {
                        pad(FORWARD, 0)
                    } else if age <= up + 1 {
                        pad(UP, SPECIAL)
                    } else {
                        pad(AIM, 0)
                    };
                    assert_eq!(flight.step(input), Some(expected), "row {age}, {damage}%");
                    assert_eq!(flight.step(input), Some(expected), "same-frame reread");
                }
            }
        }
    }

    #[test]
    fn release_plans_use_native_velocity_without_a_playback_reset() {
        for (damage, height) in [(100.0, -14.0), (130.0, -22.0)] {
            for (drop, velocity) in [(0.0, 0.0), (-0.084, -0.084), (-0.252, -0.168)] {
                let position = [-101.986, height + drop];
                let beat = fresh(damage, position, velocity);
                let flight = AuraFlight::start(&facts((3, 0), position), beat)
                    .unwrap_or_else(|| panic!("{damage}% release at {position:?}, vy={velocity}"));
                assert!((130..=191).contains(&flight.crossing_frames()));
            }
        }
    }

    #[test]
    fn aura_flight_needs_live_geometry_aura_and_reserved_moves() {
        let position = [-101.986, -14.0];
        let beat = fresh(100.0, position, 0.0);
        let ring = facts((3, 0), position);
        for invalid in [
            AuraSample {
                can_rise: false,
                ..beat
            },
            AuraSample { jumps: 0, ..beat },
            AuraSample {
                facing: -1.0,
                ..beat
            },
            AuraSample {
                damage: 65.0,
                ..beat
            },
            AuraSample {
                damage: f32::NAN,
                ..beat
            },
            AuraSample {
                velocity: [0.1, 0.0],
                ..beat
            },
        ] {
            assert!(AuraFlight::start(&ring, invalid).is_none());
        }
        assert!(AuraFlight::start(&facts((89, 0), position), beat).is_none());
        assert!(AuraFlight::start(&facts((3, 1), position), beat).is_none());
        assert!(AuraFlight::start(
            &RouteFacts {
                outline: &[],
                ..ring
            },
            beat
        )
        .is_none());
        let outside = [-131.986, -14.0];
        assert!(AuraFlight::start(&facts((3, 0), outside), fresh(100.0, outside, 0.0)).is_none());
    }

    #[test]
    fn fail_closed_before_spending_the_jump_when_the_live_row_diverges() {
        let position = [-101.986, -14.0];
        let first = fresh(100.0, position, 0.0);
        let mut flight = AuraFlight::start(&facts((3, 0), position), first).unwrap();
        assert_eq!(flight.step(first), Some(pad(READY, 0)));
        let jump = first.frame + u32::from(flight.chosen.jump);
        assert!(flight.jump_frame(jump));
        assert_eq!(
            flight.step(AuraSample {
                frame: jump,
                ..first
            }),
            None
        );
    }

    #[test]
    fn the_shipped_tables_keep_all_reference_cells_and_bound_the_search() {
        for (damage, expected) in [(100.0, 64), (130.0, 85)] {
            let (region, candidates) = region_at(damage).unwrap();
            assert_eq!(
                region.iter().map(|word| word.count_ones()).sum::<u32>(),
                expected
            );
            assert_eq!(candidates.len(), expected as usize);
            let mut prev = None;
            for (index, plans) in candidates {
                assert!(prev.is_none_or(|last| last < *index));
                prev = Some(*index);
                assert!(plans.iter().any(|schedule| schedule[2] != 0));
                for [hold, drift, jump] in plans.iter().copied().filter(|plan| plan[2] != 0) {
                    assert!(hold <= 40 && (24..=78).contains(&drift) && (38..=59).contains(&jump));
                }
            }
        }
    }
    }
}
