//! Greninja's measured Hydro Pump recovery on the common stage ring.
use super::{niche::pad, region, Correction, Point, RouteFacts, JUMP, SPECIAL};

#[path = "greninja_path.rs"]
mod path;
#[path = "greninja_schedule.rs"]
mod schedule;

pub const KIND: i32 = 0x35;
pub const ROW: u8 = 13;
const FALL: i32 = 0xe;
const FALL_AERIAL: i32 = 0xf;
const JUMP_AERIAL: i32 = 0xc;
const HYDRO_PUMP: i32 = 0x1de;
const HYDRO_RUSH: i32 = 0x1e7;
const HYDRO_END: i32 = 0x1e9;
const FORWARD: Point = [1.0, 0.0];
const UP: Point = [0.0, 1.0];
const DIAGONAL: Point = [85.0 / 120.0; 2];

pub fn wizard_offset(stage: (i32, u32)) -> Option<Point> {
    super::is_crossing_stage(stage).then_some([18.0, 34.0])
}

#[derive(Clone, Copy)]
pub struct HydroSample {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub facing: f32,
    pub jumps: u8,
    pub can_jump: bool,
    pub can_rise: bool,
}

pub struct HydroFlight {
    near: Point,
    far: Point,
    chosen: path::Flight,
    predicted: path::Fall,
    predicted_age: u32,
    origin: Option<u32>,
    last_pad: Option<(u32, Correction)>,
}

fn local(near: Point, position: Point) -> [f64; 2] {
    [
        path::NEAR_X + f64::from(position[0]) - f64::from(near[0]),
        f64::from(position[1]) - f64::from(near[1]),
    ]
}

fn at_waypoint(actual: [f64; 2], expected: [f64; 2]) -> bool {
    actual
        .into_iter()
        .zip(expected)
        .all(|(a, b)| (a - b).abs() <= 1.0)
}

impl HydroFlight {
    pub fn start(facts: &RouteFacts<'_>, sample: HydroSample) -> Option<Self> {
        facts.crossing_band()?;
        let [near, far] = facts.ledges;
        let [vx, vy] = sample.velocity;
        let [x, y] = local(near, sample.position);
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
            || !(-1.85..=0.0).contains(&vy)
            || !region::contains(&schedule::REGION, outside, height)
        {
            return None;
        }
        let row = ((40.0 - height) / 4.0) as u16;
        let column = (outside / 4.0) as u16;
        let cell = row * 30 + column;
        let predicted = path::Fall::new([x, y], f64::from(vy));
        let i = schedule::PLANS
            .binary_search_by_key(&cell, |entry| entry.0)
            .ok()?;
        let chosen = schedule::PLANS[i]
            .1
            .iter()
            .find_map(|&values| path::estimate(predicted, path::Timing::from(values)))?;
        Some(Self {
            near,
            far,
            chosen,
            predicted,
            predicted_age: 0,
            origin: None,
            last_pad: None,
        })
    }

    pub fn exit_point(&self) -> Point {
        self.far
    }

    pub fn crossing_frames(&self) -> u16 {
        self.chosen.arrival
    }

    fn age(&self, frame: u32) -> u32 {
        frame.wrapping_sub(self.origin.unwrap_or(frame))
    }

    pub fn jump_frame(&self, frame: u32) -> bool {
        self.age(frame) == self.chosen.timing.jump_input()
    }

    pub fn rise_frame(&self, frame: u32) -> bool {
        self.age(frame) == self.chosen.timing.up_input()
    }

    pub fn step(&mut self, sample: HydroSample) -> Option<Correction> {
        let position = local(self.near, sample.position);
        if !position.into_iter().all(f64::is_finite) || position[1] <= path::FLOOR {
            return None;
        }
        if let Some((_, correction)) = self.last_pad.filter(|&(frame, _)| frame == sample.frame) {
            return Some(correction);
        }
        let age = self.age(sample.frame);
        let timing = self.chosen.timing;
        let jump = timing.jump_input();
        let up = timing.up_input();
        let correction = if age <= jump {
            while self.predicted_age < age {
                self.predicted.advance(timing, self.predicted_age);
                self.predicted_age += 1;
            }
            if sample.status != FALL
                || sample.jumps != 1
                || !at_waypoint(position, self.predicted.position)
            {
                return None;
            }
            if age == jump {
                (sample.can_jump && sample.facing == 1.0).then(|| pad(FORWARD, JUMP))?
            } else {
                let x = if age < u32::from(timing.down) {
                    0.0
                } else {
                    0.5
                };
                let down = -13.0 * (age + 1).min(8) as f32 / 120.0;
                let stick = if age < timing.eased() {
                    [x, down]
                } else {
                    FORWARD
                };
                pad(stick, 0)
            }
        } else if age == jump + 1 {
            (sample.status == JUMP_AERIAL && sample.jumps == 0).then(|| pad(FORWARD, JUMP))?
        } else if age < up {
            (matches!(sample.status, JUMP_AERIAL | FALL_AERIAL) && sample.jumps == 0)
                .then(|| pad(FORWARD, 0))?
        } else if age == up {
            (matches!(sample.status, JUMP_AERIAL | FALL_AERIAL)
                && sample.jumps == 0
                && sample.can_rise
                && sample.facing == 1.0
                && at_waypoint(position, self.chosen.press_at)
                && path::can_finish_from_press(position, timing))
            .then(|| pad(UP, SPECIAL))?
        } else if age == up + 1 {
            (sample.status == HYDRO_PUMP).then(|| pad(UP, SPECIAL))?
        } else if age <= up + 11 {
            (sample.status == HYDRO_PUMP).then(|| pad(DIAGONAL, 0))?
        } else {
            (matches!(sample.status, HYDRO_PUMP | HYDRO_RUSH | HYDRO_END)
                && age <= u32::from(self.chosen.arrival))
            .then(|| pad(UP, 0))?
        };
        self.origin.get_or_insert(sample.frame);
        self.last_pad = Some((sample.frame, correction));
        Some(correction)
    }
}

#[cfg(test)]
mod tests {
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
            player_reach: 10.0,
            ledges: [[-79.986, 0.0], [79.989, 0.0]],
            blast: [-230.0, 230.0, 180.0, -140.0],
            outline: &RING,
            body_clearance: 12.0,
            ledge_distance: 17.25,
            fall_budget: 1.85,
            reach: [150.0, 70.0],
        }
    }

    fn fresh(position: Point) -> HydroSample {
        HydroSample {
            frame: 100,
            status: FALL,
            position,
            velocity: [0.0, 0.0],
            facing: 1.0,
            jumps: 1,
            can_jump: true,
            can_rise: true,
        }
    }

    #[test]
    fn hydro_pump_offers_the_proven_left_start_on_each_live_ring() {
        let sample = fresh([-97.986, 34.0]);
        for stage in [(0, 0), (3, 0), (311, 0)] {
            let flight = HydroFlight::start(&facts(stage, sample.position), sample)
                .expect("the 18 out, 34 above release completed on all three stages");
            assert!((140..=180).contains(&flight.crossing_frames()));
        }
        for stage in [(3, 1), (89, 0), (3, 0)] {
            let sample = if stage == (3, 0) {
                fresh([-117.986, 22.0])
            } else {
                sample
            };
            assert!(HydroFlight::start(&facts(stage, sample.position), sample).is_none());
        }
    }

    #[test]
    fn hydro_pump_follows_the_native_jump_two_up_presses_and_burst_aim() {
        // 990288d6, fg2-g2-yoga-12, attempt 1, 18 out/34 above.
        let mut sample = fresh([-97.986, 34.0]);
        let mut flight = HydroFlight::start(&facts((0, 0), sample.position), sample).unwrap();
        assert_eq!(
            flight.step(sample).unwrap().stick,
            Some([0.0, -13.0 / 120.0])
        );
        assert_eq!(
            flight.step(sample).unwrap().stick,
            Some([0.0, -13.0 / 120.0])
        );
        sample.frame = 176;
        sample.position = [-37.755, -98.0];
        assert_eq!(flight.step(sample).unwrap().press, JUMP);
        sample.frame = 177;
        sample.position = [-36.516, -99.85];
        sample.status = JUMP_AERIAL;
        sample.jumps = 0;
        assert_eq!(flight.step(sample).unwrap().press, JUMP);
        sample.frame = 221;
        sample.position = [17.444, -83.92];
        sample.status = FALL_AERIAL;
        let first_up = flight.step(sample).unwrap();
        assert_eq!(
            (first_up.stick, first_up.press),
            (Some([0.0, 1.0]), SPECIAL)
        );
        sample.frame = 222;
        sample.status = HYDRO_PUMP;
        sample.position = [18.668, -85.77];
        assert_eq!(flight.step(sample).unwrap(), first_up);
        sample.frame = 223;
        assert_eq!(flight.step(sample).unwrap().stick, Some([85.0 / 120.0; 2]));
        sample.frame = 233;
        assert_eq!(flight.step(sample).unwrap().stick, Some([0.0, 1.0]));
        sample.frame = 238;
        sample.status = HYDRO_RUSH;
        assert_eq!(flight.step(sample).unwrap().stick, Some([0.0, 1.0]));
    }

    #[test]
    fn hydro_pump_keeps_the_proven_window_and_requires_both_resources() {
        let offset = wizard_offset((3, 0)).unwrap();
        let sample = fresh([-79.986 - offset[0], offset[1]]);
        let mut facts = facts((3, 0), sample.position);
        for sample in [
            HydroSample { jumps: 0, ..sample },
            HydroSample {
                can_rise: false,
                ..sample
            },
            HydroSample {
                velocity: [0.2, 0.0],
                ..sample
            },
            HydroSample {
                facing: -1.0,
                ..sample
            },
        ] {
            assert!(HydroFlight::start(&facts, sample).is_none());
        }
        facts.outline = &RING[1..];
        assert!(HydroFlight::start(&facts, sample).is_none());
        assert!(wizard_offset((89, 0)).is_none());
    }
}
