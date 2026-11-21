//! Hero's measured HR-1 recovery with a level air dodge and full Kaswoosh.
use super::{niche::pad, region, Correction, Point, RouteFacts, GUARD, JUMP, SPECIAL};

#[path = "hero_path.rs"]
mod path;

pub const KIND: i32 = 0x53;
pub const ROW: u8 = 32;
pub const WIZARD_OFFSET: Point = [40.0, 20.0];

pub const fn reference_stage(stage: (i32, u32)) -> bool {
    matches!(stage, (0 | 3 | 311, 0))
}
const FALL: i32 = 0xe;
const JUMP_AERIAL: i32 = 0xc;
const DODGE: i32 = 0x22;
const SPECIAL_HI: i32 = 0x1de;
const CHARGE: i32 = 0x1eb;
const FLIGHT: i32 = 0x1ec;
const FORWARD: Point = [1.0, 0.0];
const UP: Point = [0.0, 1.0];
const REST_RING: [u32; 14] = [
    0x3fe0, 0x1fe0, 0x1fe0, 0x0fe0, 0x07e0, 0x07e0, 0x03e0, 0x01e0, 0x01e0, 0x00e0, 0x00e0, 0x0060,
    0x0020, 0x0020,
];

#[derive(Clone, Copy)]
pub struct HeroBeat {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub facing: f32,
    pub jumps: u8,
    pub can_dodge: bool,
    pub can_jump: bool,
    pub can_cast: bool,
    pub mp: Option<f32>,
}

pub struct HeroFlight {
    near: Point,
    far: Point,
    route: path::Path,
    facing: f64,
    origin: Option<u32>,
    dodge_started: bool,
    last_pad: Option<(u32, Correction)>,
}

impl HeroFlight {
    pub fn start(facts: &RouteFacts<'_>, entry: HeroBeat) -> Option<Self> {
        facts.crossing_band()?;
        let [near, far] = facts.ledges;
        let position = path::local(near, entry.position);
        if facts.fighter != KIND
            || !reference_stage(facts.stage)
            || facts.position != entry.position
            || !region::has_common_underside(facts.ledges, facts.blast[3], facts.outline)
            || entry.status != FALL
            || !matches!(entry.facing, -1.0 | 1.0)
            || entry.velocity != [0.0; 2]
            || entry.jumps != 1
            || !entry.can_dodge
            || !entry.can_jump
            || !entry.can_cast
            || !has_mana(entry)
            || !region::contains(
                &REST_RING,
                near[0] - entry.position[0],
                entry.position[1] - near[1],
            )
        {
            return None;
        }
        let facing = f64::from(entry.facing);
        let route = path::plan(
            path::Sample {
                position,
                velocity: [0.0; 2],
            },
            facing,
        )?;
        Some(Self {
            near,
            far,
            route,
            facing,
            origin: None,
            dodge_started: false,
            last_pad: None,
        })
    }

    pub fn exit_point(&self) -> Point {
        self.far
    }

    pub fn crossing_frames(&self) -> u16 {
        self.route.arrival
    }

    pub fn before_dodge(&self) -> bool {
        !self.dodge_started
    }

    fn age(&self, frame: u32) -> u32 {
        frame.wrapping_sub(self.origin.unwrap_or(frame))
    }

    pub fn observes_jump_reserve(&self, frame: u32) -> bool {
        self.age(frame) <= self.route.cast
    }

    pub fn shield_frame(&self, frame: u32) -> bool {
        self.age(frame) == self.route.shield
    }

    pub fn jump_frame(&self, frame: u32) -> bool {
        self.age(frame) == self.route.shield + path::DODGE_FRAMES
    }

    pub fn cast_frame(&self, frame: u32) -> bool {
        self.age(frame) == self.route.cast
    }

    pub fn step(&mut self, beat: HeroBeat) -> Option<Correction> {
        let position = path::local(self.near, beat.position);
        let velocity = beat.velocity.map(f64::from);
        if !position.into_iter().chain(velocity).all(f64::is_finite) || position[1] <= -140.0 {
            return None;
        }
        if let Some((_, correction)) = self.last_pad.filter(|&(frame, _)| frame == beat.frame) {
            return Some(correction);
        }
        let age = self.age(beat.frame);
        let sample = path::Sample { position, velocity };
        let correction = self.correction(beat, age, sample)?;
        self.origin.get_or_insert(beat.frame);
        if age == self.route.shield + 1 && beat.status == DODGE {
            self.dodge_started = true;
        }
        self.last_pad = Some((beat.frame, correction));
        Some(correction)
    }

    fn correction(&self, beat: HeroBeat, age: u32, sample: path::Sample) -> Option<Correction> {
        let route = self.route;
        let jump = route.shield + path::DODGE_FRAMES;
        let jump_frames = route.cast - jump;
        if age < route.shield {
            return (beat.status == FALL && beat.jumps == 1)
                .then(|| pad(if age < route.wait { [0.0; 2] } else { FORWARD }, 0));
        }
        if age == route.shield {
            return (beat.status == FALL
                && beat.jumps == 1
                && has_mana(beat)
                && beat.can_dodge
                && beat.can_jump
                && beat.can_cast
                && path::shield_reaches(sample, self.facing, jump_frames))
            .then(|| pad(FORWARD, GUARD));
        }
        if age < jump {
            return (beat.status == DODGE && beat.jumps == 1)
                .then(|| pad(FORWARD, if age == route.shield + 1 { GUARD } else { 0 }));
        }
        if age == jump {
            // The final EscapeAir think is still in that status when the
            // next pad is read. The live JumpAerial term must be enabled.
            return (matches!(beat.status, DODGE | FALL)
                && beat.jumps == 1
                && has_mana(beat)
                && beat.can_jump
                && path::jump_reaches(sample, self.facing, jump_frames))
            .then(|| pad(FORWARD, JUMP));
        }
        if age < route.cast {
            return (matches!(beat.status, JUMP_AERIAL | FALL) && beat.jumps == 0)
                .then(|| pad(FORWARD, 0));
        }
        if age == route.cast {
            return (matches!(beat.status, JUMP_AERIAL | FALL)
                && beat.jumps == 0
                && has_mana(beat)
                && beat.can_cast
                && path::cast_reaches(sample, self.facing))
            .then(|| pad(UP, SPECIAL));
        }
        match beat.status {
            SPECIAL_HI | CHARGE | FLIGHT if age < route.cast + 34 => Some(pad(
                if age < route.cast + 3 { UP } else { FORWARD },
                SPECIAL,
            )),
            FLIGHT => Some(pad(FORWARD, 0)),
            _ => None,
        }
    }
}

fn has_mana(beat: HeroBeat) -> bool {
    beat.mp.is_some_and(|mp| (18.0..=100.0).contains(&mp))
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

        const START: HeroBeat = HeroBeat {
            frame: 0,
            status: 0xe,
            position: [-119.986, 20.0],
            velocity: [0.0; 2],
            facing: 1.0,
            jumps: 1,
            can_dodge: true,
            can_jump: true,
            can_cast: true,
            mp: Some(100.0),
        };

        fn facts(beat: HeroBeat) -> RouteFacts<'static> {
            RouteFacts {
                fighter: KIND,
                stage: (3, 0),
                position: beat.position,
                player: [-74.0, 0.0],
                player_reach: 8.0,
                ledges: [LEFT, RIGHT],
                blast: [-240.0, 240.0, 185.0, -140.0],
                outline: &OUTLINE,
                body_clearance: 8.0,
                ledge_distance: 9.0,
                fall_budget: 1.57,
                reach: [160.0, 100.0],
            }
        }

        #[test]
        fn hero_offers_the_proven_high_start_band_without_guessing_at_mana() {
            for (outside, height) in [(40.0, 20.0), (30.0, 6.0), (36.0, 30.0), (24.0, 10.0), (46.0, 26.0)] {
                let beat = HeroBeat { position: [LEFT[0] - outside, height], ..START };
                assert!(HeroFlight::start(&facts(beat), beat).is_some(), "reference ({outside}, {height})");
            }
            for beat in [
                HeroBeat { mp: None, ..START },
                HeroBeat { mp: Some(f32::NAN), ..START },
                HeroBeat { mp: Some(17.99), ..START },
                HeroBeat { mp: Some(101.0), ..START },
                HeroBeat { can_dodge: false, ..START },
                HeroBeat { can_jump: false, ..START },
                HeroBeat { can_cast: false, ..START },
                HeroBeat { jumps: 0, ..START },
                HeroBeat { velocity: [0.0, -1.57], ..START },
                HeroBeat { position: [LEFT[0] - 56.0, 30.0], ..START },
            ] {
                assert!(HeroFlight::start(&facts(beat), beat).is_none());
            }
            assert!(HeroFlight::start(&RouteFacts { stage: (89, 0), ..facts(START) }, START).is_none());
            assert!(HeroFlight::start(&RouteFacts { stage: (3, 1), ..facts(START) }, START).is_none());
            assert!(HeroFlight::start(&RouteFacts { outline: &[], ..facts(START) }, START).is_none());
        }

        #[test]
        fn hero_upper_left_wizard_uses_the_measured_release_on_each_reviewed_stage() {
            use crate::model::edge_juggle::{Area, Drill, Stage, Wizard};

            let stage = Stage::new(
                [-240.0, 240.0, 192.0, -140.0],
                [-210.0, 210.0, 160.0, -130.0],
                [LEFT, RIGHT],
            ).unwrap();
            for (key, expected) in [
                ((3, 0), [LEFT[0] - 40.0, LEFT[1] + 20.0]),
                ((0, 0), [LEFT[0] - 40.0, LEFT[1] + 20.0]),
                ((311, 0), [LEFT[0] - 40.0, LEFT[1] + 20.0]),
                ((3, 1), stage.area_default(Area::UpperLeft).unwrap()),
                ((89, 0), stage.area_default(Area::UpperLeft).unwrap()),
            ] {
                let mut wizard =
                    Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, key, Some(KIND)).unwrap();
                assert_eq!(wizard.setup.cpu, stage.area_default(Area::LowerLeft).unwrap());
                wizard.change_area(stage, 2);
                assert_eq!(wizard.setup.area, Area::UpperLeft);
                assert_eq!(wizard.setup.cpu, expected);
                wizard.change_area(stage, 1);
                assert_eq!(wizard.setup.cpu, stage.area_default(Area::UpperRight).unwrap());
            }
        }

        #[test]
        fn hero_left_rest_table_requires_a_cast_and_jump_still_available_before_dodge() {
            for stage in [(3, 0), (0, 0), (311, 0)] {
                let measured = RouteFacts { stage, ..facts(START) };
                let shield = HeroBeat {
                    frame: 35, position: [-97.45, -22.61], velocity: [1.01, -1.57], ..START
                };
                let try_shield = |shield| {
                    let mut flight = HeroFlight::start(&measured, START).expect("reviewed stage");
                    flight.step(START).unwrap();
                    flight.step(shield)
                };
                assert!(try_shield(HeroBeat { can_jump: false, ..shield }).is_none(), "{stage:?}");
                assert!(try_shield(HeroBeat { can_cast: false, ..shield }).is_none(), "{stage:?}");
                assert!(try_shield(HeroBeat { mp: Some(17.99), ..shield }).is_none(), "{stage:?}");
                assert_eq!(try_shield(shield), Some(pad(FORWARD, GUARD)), "{stage:?}");
            }
        }

        #[test]
        fn hero_replays_the_reference_shield_and_declines_a_missed_entry_window() {
            let mut flight = HeroFlight::start(&facts(START), START).unwrap();
            assert_eq!(flight.step(START), Some(pad([0.0; 2], 0)));
            assert!(flight.before_dodge());
            // Zero-gap frame sequence from fusion-g4 hero_hr1.py, Upper Left.
            let shield = HeroBeat {
                frame: 35, position: [-97.45, -22.61], velocity: [1.01, -1.57], ..START
            };
            assert_eq!(flight.step(shield), Some(pad([1.0, 0.0], GUARD)));
            assert_eq!(flight.step(shield), Some(pad([1.0, 0.0], GUARD)));

            let mut outside = HeroFlight::start(&facts(START), START).unwrap();
            outside.step(START).unwrap();
            assert!(outside.step(HeroBeat { position: [-92.0, -22.61], ..shield }).is_none());
            let mut unavailable = HeroFlight::start(&facts(START), START).unwrap();
            unavailable.step(START).unwrap();
            assert!(unavailable.step(HeroBeat { can_dodge: false, ..shield }).is_none());
        }

        #[test]
        fn hero_cast_uses_live_reach_when_the_position_differs_from_the_planned_waypoint() {
            let mut flight = HeroFlight::start(&facts(START), START).unwrap();
            flight.step(START).unwrap();
            let cast = HeroBeat {
                frame: 151, status: JUMP_AERIAL, jumps: 0, position: [26.51, -95.76],
                velocity: [1.01, -0.735], ..START
            };
            let sample = path::Sample { position: path::local(LEFT, cast.position), velocity: cast.velocity.map(f64::from) };
            assert!(path::cast_reaches(sample, 1.0), "the live trajectory reaches the far ledge");
            assert_eq!(flight.step(cast), Some(pad(UP, SPECIAL)));
        }

        #[test]
        fn hero_reserves_the_jump_and_uses_a_full_charge_at_the_measured_waypoints() {
            let mut flight = HeroFlight::start(&facts(START), START).unwrap();
            assert_eq!(flight.exit_point(), RIGHT);
            assert_eq!(flight.crossing_frames(), 230);
            flight.step(START).unwrap();
            let shield = HeroBeat {
                frame: 35, position: [-97.45, -22.61], velocity: [1.01, -1.57], ..START
            };
            assert_eq!(flight.step(shield), Some(pad(FORWARD, GUARD)));
            let dodge = HeroBeat { frame: 36, status: DODGE, ..shield };
            assert_eq!(flight.step(dodge), Some(pad(FORWARD, GUARD)));
            assert!(!flight.before_dodge());

            let jump = HeroBeat {
                frame: 118, position: [-8.82, -123.32], ..dodge
            };
            assert!(flight.step(HeroBeat { can_jump: false, ..jump }).is_none());
            assert_eq!(flight.step(jump), Some(pad(FORWARD, JUMP)));
            // Native WorkModule mana is sampled on actions, not on ordinary drift.
            let aerial = HeroBeat { frame: 119, status: JUMP_AERIAL, jumps: 0, mp: None, ..jump };
            assert_eq!(flight.step(aerial), Some(pad(FORWARD, 0)));

            let cast = HeroBeat {
                frame: 151, position: [24.51, -95.76], velocity: [1.01, -0.735], mp: Some(100.0), ..aerial
            };
            assert!(flight.step(HeroBeat { mp: Some(17.0), ..cast }).is_none());
            assert!(flight.step(HeroBeat { can_cast: false, ..cast }).is_none());
            assert_eq!(flight.step(cast), Some(pad(UP, SPECIAL)));
            let charge = HeroBeat { frame: 153, status: SPECIAL_HI, ..cast };
            assert_eq!(flight.step(charge), Some(pad(UP, SPECIAL)));
            assert_eq!(flight.step(HeroBeat { frame: 154, ..charge }), Some(pad(FORWARD, SPECIAL)));
            let soar = HeroBeat { frame: 190, status: FLIGHT, mp: None, ..cast };
            let last_charge = HeroBeat { frame: 184, ..soar };
            assert_eq!(flight.step(last_charge), Some(pad(FORWARD, SPECIAL)));
            assert_eq!(flight.step(soar), Some(pad(FORWARD, 0)));
            assert!(flight.step(HeroBeat { status: 0x49, ..HeroBeat { frame: 191, ..soar } }).is_none());
        }
    }
}
