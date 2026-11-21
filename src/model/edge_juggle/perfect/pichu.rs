//! Pichu's measured two-zip finish from Group 2's Agility captures.

use super::{
    aerial_clear, niche::pad, Correction, Outline, Point, RouteFacts, Segment, JUMP, SPECIAL,
};

const FALL: i32 = 0xe;
const JUMP_AERIAL: i32 = 0xc;
const BASH_START: i32 = 0x1dd;
const BASH_TRAVEL: i32 = 0x1e5;
pub const BASH_END: i32 = 0x1e6;
const AGILITY_START: i32 = 0x1de;
const AGILITY_TRAVEL: i32 = 0x1e1;
const AGILITY_END: i32 = 0x1e2;

#[derive(Clone, Copy)]
pub struct PichuBeat {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub jumps: u8,
    pub can_jump: bool,
    pub side: bool,
    pub up: bool,
}

pub struct PichuFlight {
    outline: Outline,
    far: Point,
    bottom: f32,
    direction: f32,
    phase: PichuPhase,
}

enum PichuPhase {
    Descend,
    Bash(u32),
    Jump(u32),
    Agility(u32, ZipPlan),
}

const GRAVITY: f32 = 0.14;
const FALL_SPEED: f32 = 1.9;
const FAST_FALL_SPEED: f32 = 2.5;
const HORIZONTAL_BRAKE: f32 = 0.0075;
const JUMP_TO_AGILITY: u32 = 22;
const BASH_END_JUMPS: [u32; 4] = [88, 92, 96, 100];

// Group 2 P-SB bake, from reference-crossings/g2/pichu-p-sb-ring.txt.
const PICHU_P_SB_RING: [u32; 44] = [
    0x00000000, 0x00001ffc, 0x00000ffc, 0x00001ffc, 0x00001ffc, 0x00001ffc, 0x00001ffc, 0x00000ffc,
    0x00001ffc, 0x00001ffc, 0x00001ffc, 0x00001ffc, 0x00000ffc, 0x00001ffc, 0x00001ffc, 0x00001ffc,
    0x00001ffc, 0x00000ffc, 0x00001ffc, 0x00001ffc, 0x00001ffc, 0x00001ffc, 0x00000ffc, 0x000007fc,
    0x000007fc, 0x000003fc, 0x000001fc, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000,
];

impl PichuFlight {
    pub fn start(facts: &RouteFacts<'_>, beat: PichuBeat) -> Option<Self> {
        if facts.fighter != 0x13
            || beat.status != FALL
            || beat.jumps != 1
            || !beat.side
            || !beat.up
            || beat.velocity[0].abs() > 0.1
            || !(-FAST_FALL_SPEED..=0.0).contains(&beat.velocity[1])
            || !(0.0 < facts.body_clearance && facts.body_clearance <= HEAD)
            || facts.crossing_band().is_none()
        {
            return None;
        }

        let [left, right] = facts.ledges;
        let span = right[0] - left[0];
        let [x, y] = beat.position;
        let (near, far, direction) = if x < left[0] {
            (left, right, 1.0)
        } else {
            (right, left, -1.0)
        };
        let outside = (near[0] - x) * direction;
        let flat = facts
            .outline
            .iter()
            .flatten()
            .map(|p| p[1])
            .reduce(f32::min)?;
        if (span - 159.975).abs() > 1.0
            || (right[1] - left[1]).abs() > 0.5
            || (flat - near[1] + 40.1).abs() > 1.0
            || (facts.blast[3] - near[1] + 140.0).abs() > 1.0
            || !super::region::contains(&PICHU_P_SB_RING, outside, y - near[1])
        {
            return None;
        }
        let mut outline = Outline::new();
        for &line in facts.outline {
            outline.add(line);
        }
        Some(Self {
            outline,
            far,
            bottom: facts.blast[3],
            direction,
            phase: PichuPhase::Descend,
        })
    }

    pub fn exit_point(&self) -> Point {
        [self.far[0] + self.direction * 8.85, self.far[1] - 14.6]
    }

    fn aim_from(&self, point: Point) -> Option<ZipPlan> {
        agility(
            self.outline.segments(),
            point,
            self.far,
            self.direction,
            self.bottom,
        )
    }

    fn jump_will_reach(&self, beat: PichuBeat) -> bool {
        let [x, y] = beat.position;
        let [vx, vy] = beat.velocity;
        // The transition into JumpAerial carries BashEnd's last fall row.
        let mut tip = [
            x + vx - HORIZONTAL_BRAKE * self.direction,
            y + (vy - 0.11).max(-FALL_SPEED),
        ];
        let clear =
            |path| aerial_clear(self.outline.segments(), path, self.bottom + 4.0, HEAD, HALF);
        if !clear([beat.position, tip]) {
            return false;
        }
        let ascent = (2.0 * GRAVITY * 36.02_f32).sqrt() - GRAVITY / 2.0;
        for tick in 0..JUMP_TO_AGILITY {
            let drift = match tick {
                0 => 1.0,
                21 => 1.029 - HORIZONTAL_BRAKE,
                _ => 1.029,
            };
            let next = [
                tip[0] + self.direction * drift,
                tip[1] + ascent - GRAVITY * tick as f32,
            ];
            if !clear([tip, next]) {
                return false;
            }
            tip = next;
        }
        self.aim_from(tip).is_some()
    }

    pub fn step(&mut self, beat: PichuBeat) -> Option<Correction> {
        let PichuBeat {
            frame,
            status,
            position,
            velocity,
            jumps,
            can_jump,
            side,
            up,
        } = beat;
        if !position.into_iter().chain(velocity).all(f32::is_finite)
            || position[1] <= self.bottom + 4.0
        {
            return None;
        }
        let forward = [self.direction, 0.0];
        let bash = pad(forward, SPECIAL);
        let jump = pad(forward, JUMP);
        let drift = pad(forward, 0);
        let rise = pad([0.0, 1.0], SPECIAL);
        match &self.phase {
            PichuPhase::Descend if status == FALL && jumps > 0 => {
                let fall = if velocity[1] < -FALL_SPEED {
                    velocity[1] // Native fast-fall speed is already active.
                } else {
                    (velocity[1] - GRAVITY).max(-FALL_SPEED)
                };
                let next_y = position[1] + fall;
                if next_y - self.far[1] <= -48.0 {
                    if !side {
                        return None;
                    }
                    self.phase = PichuPhase::Bash(frame);
                    Some(bash)
                } else {
                    Some(pad([0.0, -1.0], 0))
                }
            }
            PichuPhase::Bash(since) => {
                let age = frame.wrapping_sub(*since);
                match (age, status) {
                    (0, FALL) | (1, BASH_START) => Some(bash),
                    (2..=51, BASH_START | BASH_TRAVEL) => Some(drift),
                    (52..=100, BASH_END) if jumps > 0 => {
                        if can_jump && BASH_END_JUMPS.contains(&age) && self.jump_will_reach(beat) {
                            self.phase = PichuPhase::Jump(frame);
                            Some(jump)
                        } else {
                            (age < 100).then_some(drift)
                        }
                    }
                    _ => None,
                }
            }
            PichuPhase::Jump(since) => match (frame.wrapping_sub(*since), status) {
                (0, BASH_END) | (1, JUMP_AERIAL) => Some(jump),
                (2..=21, JUMP_AERIAL) => Some(drift),
                (22, JUMP_AERIAL) if up => {
                    let anchor = [
                        position[0] + velocity[0] - HORIZONTAL_BRAKE * self.direction,
                        position[1] + velocity[1] - GRAVITY,
                    ];
                    self.phase = PichuPhase::Agility(frame, self.aim_from(anchor)?);
                    Some(rise)
                }
                _ => None,
            },
            PichuPhase::Agility(since, plan) => match (frame.wrapping_sub(*since), status) {
                (0, JUMP_AERIAL) | (1, AGILITY_START) => Some(rise),
                (2..=20, AGILITY_START | AGILITY_TRAVEL) => Some(pad(plan.first, 0)),
                (21..=42, AGILITY_END | AGILITY_TRAVEL) => Some(pad(plan.second, 0)),
                _ => None,
            },
            _ => None,
        }
    }
}

#[derive(Debug)]
struct ZipPlan {
    first: Point,
    second: Point,
    #[cfg(test)]
    catch: Point,
    #[cfg(test)]
    row: u8,
}

const FIRST_SPEED: f32 = 8.3;
const SECOND_SPEED: f32 = 7.47;
const FIRST_LENGTH: f32 = 8.0 * FIRST_SPEED;
const FIRST_BRAKE: f32 = 0.29;
const HEAD: f32 = 9.5;
const HALF: f32 = 1.5;
const STICK_SCALE: f32 = 120.0;

#[derive(Clone, Copy)]
struct Candidate {
    slack: f32,
    first: Point,
    second: Point,
    first_end: Point,
    second_start: Point,
    catch: Point,
    #[cfg(test)]
    row: u8,
}

fn snapped_direction(degrees: f32) -> (Point, Point) {
    let [x, y] = direction(degrees);
    let raw = [(x * STICK_SCALE).round(), (y * STICK_SCALE).round()];
    let length = raw[0].hypot(raw[1]);
    (
        raw.map(|axis| axis / STICK_SCALE),
        raw.map(|axis| axis / length),
    )
}

fn direction(degrees: f32) -> Point {
    let (sine, cosine) = degrees.to_radians().sin_cos();
    [cosine, sine]
}

fn between_zips(x: f32) -> f32 {
    x.signum() * (0.01 + 0.09 * x.abs()) * 45.0
}

fn agility(
    outline: &[Segment],
    from: Point,
    far: Point,
    inward: f32,
    bottom: f32,
) -> Option<ZipPlan> {
    if !matches!(inward, -1.0 | 1.0)
        || !from
            .into_iter()
            .chain(far)
            .chain([bottom])
            .all(f32::is_finite)
        || outline.is_empty()
    {
        return None;
    }
    let [x, y] = [(from[0] - far[0]) * inward, from[1] - far[1]];
    let mut best: [Option<Candidate>; 12] = [None; 12];
    for angle in 100..=170 {
        let [cosine, sine] = direction(angle as f32);
        let (stick2, zip2) = snapped_direction(angle as f32);
        let lead = between_zips(cosine);
        let drift = between_zips(zip2[0]);
        for row in 1..=7 {
            let length = f32::from(row) * SECOND_SPEED;
            let dx = 8.85 - length * cosine - lead - x + FIRST_BRAKE;
            let dy = -14.6 - length * sine - y;
            let needed = dx * dx + dy * dy;
            if !((FIRST_LENGTH - 0.6).powi(2)..=(FIRST_LENGTH + 0.6).powi(2)).contains(&needed) {
                continue;
            }
            let first_angle = (dy.atan2(dx).to_degrees() * 2.0).round() * 0.5;
            if !(15.0..=90.0).contains(&first_angle) || angle as f32 - first_angle < 45.0 {
                continue;
            }
            let (stick1, zip1) = snapped_direction(first_angle);
            let brake = if (0.0..1.0).contains(&zip1[0]) && zip1[0] != 0.0 {
                FIRST_BRAKE
            } else {
                0.0
            };
            let first_end = [
                x + FIRST_LENGTH * zip1[0] - brake,
                y + FIRST_LENGTH * zip1[1],
            ];
            let second_start = [first_end[0] + drift, first_end[1]];
            let catch = [
                second_start[0] + length * zip2[0],
                second_start[1] + length * zip2[1],
            ];
            let slack = (catch[0] - 6.3)
                .min(11.4 - catch[0])
                .min(catch[1] + 15.6)
                .min(-13.6 - catch[1]);
            if slack < 0.3 {
                continue;
            }
            if let Some(index) = best
                .iter()
                .position(|slot| slot.is_none_or(|prior| slack > prior.slack))
            {
                best.copy_within(index..11, index + 1);
                best[index] = Some(Candidate {
                    slack,
                    first: stick1,
                    second: stick2,
                    first_end,
                    second_start,
                    catch,
                    #[cfg(test)]
                    row,
                });
            }
        }
    }
    let world = |[x, y]: Point| [far[0] + inward * x, far[1] + y];
    for candidate in best.into_iter().flatten() {
        let end = world(candidate.first_end);
        let after_drift = world(candidate.second_start);
        let catch = world(candidate.catch);
        if [[from, end], [end, after_drift], [after_drift, catch]]
            .into_iter()
            .all(|segment| aerial_clear(outline, segment, bottom + 4.0, HEAD, HALF))
        {
            return Some(ZipPlan {
                first: [inward * candidate.first[0], candidate.first[1]],
                second: [inward * candidate.second[0], candidate.second[1]],
                #[cfg(test)]
                catch,
                #[cfg(test)]
                row: candidate.row,
            });
        }
    }
    None
}

#[cfg(test)]
lib_unit_tests! {
    mod tests {
        use super::*;

        const FAR: Point = [79.989, 0.0];
        const RING: [Segment; 8] = [
            [[-79.986, 0.0], [-79.986, -8.319]],
            [[-79.986, -8.319], [-64.395, -25.111]],
            [[-64.395, -25.111], [-41.396, -40.099]],
            [[-41.396, -40.099], [41.416, -40.118]],
            [[41.416, -40.118], [64.387, -25.103]],
            [[64.387, -25.103], [79.989, -8.319]],
            [[79.989, -8.319], FAR],
            [FAR, [-79.986, 0.0]],
        ];

        #[test]
        fn pichu_region_requires_the_proven_ring_and_reserve_jump() {
            let release = PichuBeat {
                frame: 0, status: FALL, position: [-99.986, -28.0], velocity: [0.0; 2],
                jumps: 1, can_jump: true, side: true, up: true,
            };
            let facts = super::super::RouteFacts {
                fighter: 0x13, stage: (3, 0), position: release.position, player: [-75.0, 0.0],
                player_reach: 10.0, ledges: [[-79.986, 0.0], FAR],
                blast: [-230.0, 230.0, 185.0, -140.0], outline: &RING, body_clearance: 8.5,
                ledge_distance: 8.5, fall_budget: FALL_SPEED, reach: [150.0, 70.0],
            };
            for stage in [(0, 0), (3, 0), (311, 0)] {
                let facts = super::super::RouteFacts { stage, ..facts };
                assert!(PichuFlight::start(&facts, release).is_some(), "{stage:?}");
            }
            for stage in [(89, 0), (3, 1), (311, 2)] {
                let facts = super::super::RouteFacts { stage, ..facts };
                assert!(PichuFlight::start(&facts, release).is_none(), "{stage:?}");
            }
            for release in [
                PichuBeat { jumps: 0, ..release },
                PichuBeat { side: false, ..release },
                PichuBeat { up: false, ..release },
                PichuBeat { velocity: [0.25, 0.0], ..release },
                PichuBeat { position: [-155.986, -28.0], ..release },
            ] {
                assert!(PichuFlight::start(&facts, release).is_none());
            }
        }

        #[test]
        fn pichu_agility_matches_the_verified_zip_angles_and_catch_rows() {
            // g2_sim.py pichu plan 20 -28 and 36 -40, from the measured
            // Agility apex models for the paired reference releases.
            for (apex, sticks, row, catch) in [
                ([60.7678, -72.12777], [[71.0, 97.0], [-102.0, 64.0]], 1, [89.4883, -14.57714]),
                ([44.7678, -74.12777], [[88.0, 81.0], [-27.0, 117.0]], 2, [88.6125, -14.60178]),
            ] {
                let plan = agility(&RING, apex, FAR, 1.0, -140.0).expect("measured far-side exit");
                assert_eq!(plan.first, sticks[0].map(|axis| axis / 120.0));
                assert_eq!(plan.second, sticks[1].map(|axis| axis / 120.0));
                assert_eq!(plan.row, row);
                for (actual, expected) in plan.catch.into_iter().zip(catch) {
                    assert!((actual - expected).abs() < 0.01);
                }
            }
        }

        #[test]
        fn pichu_crossing_switches_pads_at_the_native_bash_jump_and_zip_boundaries() {
            let mut outline = Outline::new();
            for line in RING {
                outline.add(line);
            }
            let mut flight = PichuFlight {
                outline,
                far: FAR,
                bottom: -140.0,
                direction: 1.0,
                phase: PichuPhase::Descend,
            };
            let beat = |frame, status, position, velocity, jumps| PichuBeat {
                frame, status, position, velocity, jumps, can_jump: true, side: true, up: true,
            };
            for (sample, stick, buttons) in [
                (beat(3, FALL, [-115.986, -45.28], [0.0, -2.5], 1), [0.0, -1.0], 0),
                (beat(4, FALL, [-115.986, -47.78], [0.0, -2.5], 1), [1.0, 0.0], SPECIAL),
                (beat(5, BASH_START, [-115.886, -50.28], [0.1, -2.5], 1), [1.0, 0.0], SPECIAL),
                (beat(6, BASH_START, [-115.826, -50.33], [0.06, -0.05], 1), [1.0, 0.0], 0),
                (beat(17, BASH_TRAVEL, [-115.786, -54.83], [0.0, -0.65], 1), [1.0, 0.0], 0),
                (beat(92, BASH_END, [20.882347, -108.337_02], [1.296165, -1.900002], 1), [1.0, 0.0], JUMP),
                (beat(93, JUMP_AERIAL, [22.171013, -110.237_02], [1.288666, -1.90], 0), [1.0, 0.0], JUMP),
                (beat(94, JUMP_AERIAL, [23.171013, -107.13124], [1.0, 3.105782], 0), [1.0, 0.0], 0),
                (beat(114, JUMP_AERIAL, [43.751, -74.415_58], [1.028999, 0.305786], 0), [0.0, 1.0], SPECIAL),
                (beat(115, AGILITY_START, [44.772_5, -74.249_8], [1.0215, 0.165779], 0), [0.0, 1.0], SPECIAL),
            ] {
                let output = flight.step(sample).expect("verified applied pad");
                assert_eq!(output.stick, Some(stick), "frame {}", sample.frame);
                assert_ne!(output.press & (JUMP | SPECIAL), (JUMP | SPECIAL));
                assert_eq!(output.press & (JUMP | SPECIAL), buttons, "frame {}", sample.frame);
            }
            let launch = agility(&RING, [44.772_5, -74.249_8], FAR, 1.0, -140.0)
                .expect("live anchor can complete");
            let first = flight.step(beat(116, AGILITY_START, [44.772_5, -74.249_8], [0.0; 2], 0))
                .expect("first zip aim");
            assert_eq!(first.stick, Some(launch.first));
            let rev = flight.step(beat(135, AGILITY_END, [105.84375, -48.821102], [7.60765, 3.177585], 0))
                .expect("second zip aim");
            assert_eq!(rev.stick, Some(launch.second));
            assert!(flight.step(beat(140, 0x49, [90.0, -20.0], [0.0; 2], 0)).is_none());
        }

        #[test]
        fn pichu_baked_cells_have_a_complete_live_skull_bash_and_zip_sequence() {
            let mut checked = 0;
            for (row, &mask) in PICHU_P_SB_RING.iter().enumerate() {
                for column in 0..30 {
                    if mask & (1 << column) == 0 {
                        continue;
                    }
                    let center = [column as f32 * 4.0 + 2.0, 38.0 - row as f32 * 4.0];
                    for [dx, dy] in [[0.0, 0.0], [-1.99, 0.0], [1.99, 0.0], [0.0, -1.99], [0.0, 1.99]] {
                        for vy in [0.0, -FALL_SPEED] {
                            let label = [center[0] + dx, center[1] + dy, vy];
                            assert!(rollout(label), "baked-cell rollout at {label:?}");
                            checked += 1;
                        }
                    }
                }
            }
            assert_eq!(checked, 2_700);
        }

        // Advance the actual pad state machine from native kinetic measurements
        // in Group 2's c01/c03/c07 samples, independently of the live projection.
        fn rollout([outside, height, vy]: [f32; 3]) -> bool {
            let near = [-79.986, 0.0];
            let release = [near[0] - outside, height];
            let facts = super::super::RouteFacts {
                fighter: 0x13, stage: (3, 0), position: release, player: [-75.0, 0.0],
                player_reach: 10.0, ledges: [near, FAR],
                blast: [-230.0, 230.0, 185.0, -140.0], outline: &RING, body_clearance: 8.5,
                ledge_distance: 8.5, fall_budget: FALL_SPEED, reach: [150.0, 70.0],
            };
            let mut beat = PichuBeat {
                frame: 0, status: FALL, position: release, velocity: [0.0, vy],
                jumps: 1, can_jump: true, side: true, up: true,
            };
            let Some(mut flight) = PichuFlight::start(&facts, beat) else { return false };
            let mut bash_at = None;
            let mut jump_at = None;
            for frame in 0..200 {
                beat.frame = frame;
                let Some(edit) = flight.step(beat) else { return false };
                let Some(stick) = edit.stick else { return false };
                let speed = if let Some(jump) = jump_at {
                    if edit.press & SPECIAL != 0 {
                        let PichuPhase::Agility(_, plan) = flight.phase else { return false };
                        return plan.catch[0] - FAR[0] > 6.3
                            && plan.catch[0] - FAR[0] < 11.4
                            && (-15.6..-13.6).contains(&plan.catch[1]);
                    }
                    let age = (frame - jump) as f32 - 1.0;
                    [if age == 0.0 { 1.0 } else { 1.029 }, 3.105_782 - 0.14 * age]
                } else if let Some(bash) = bash_at {
                    let age = frame - bash;
                    match age {
                        1..=12 => {
                            let vx = [0.06, 0.04, 0.02, 0.0, -0.02]
                                .get(age as usize - 1).copied().unwrap_or(0.0);
                            [vx, -0.05 * age as f32]
                        }
                        13 => {
                            beat.status = BASH_TRAVEL;
                            [0.0, -0.65]
                        }
                        14..=51 => [2.2267, 0.224 - 0.011 * (age - 14) as f32],
                        52 => {
                            beat.status = BASH_END;
                            [2.2267, -0.194]
                        }
                        _ => {
                            if edit.press & JUMP != 0 {
                                jump_at = Some(frame);
                                beat.status = JUMP_AERIAL;
                                beat.jumps = 0;
                            }
                            let ago = (age - 53) as f32;
                            [1.551 - 0.0075 * ago, (-0.304 - 0.11 * ago).max(-1.9)]
                        }
                    }
                } else {
                    if edit.press & SPECIAL != 0 {
                        bash_at = Some(frame);
                        beat.status = BASH_START;
                    }
                    let vy = if beat.velocity[1] == -2.5
                        || (stick[1] == -1.0 && beat.velocity[1] < 0.0)
                    {
                        -2.5
                    } else {
                        (beat.velocity[1] - 0.14).max(-1.9)
                    };
                    [0.1 * stick[0], vy]
                };
                // The activation row reports the new native status before the next pad.
                if bash_at.is_some_and(|bash| frame + 1 == bash + 13) {
                    beat.status = BASH_TRAVEL;
                }
                if bash_at.is_some_and(|bash| frame + 1 == bash + 52) {
                    beat.status = BASH_END;
                }
                let next = [beat.position[0] + speed[0], beat.position[1] + speed[1]];
                if !aerial_clear(&RING, [beat.position, next], -136.0, HEAD, HALF) {
                    return false;
                }
                beat.position = next;
                beat.velocity = speed;
            }
            false
        }

        #[test]
        fn pichu_agility_declines_if_a_solid_line_blocks_both_zips() {
            let mut outline = RING.to_vec();
            outline.push([[67.0, -139.0], [67.0, 0.0]]);
            assert!(agility(&outline, [60.7678, -72.12777], FAR, 1.0, -140.0).is_none());
            assert!(agility(&RING, [-55.0, -80.0], FAR, 1.0, -140.0).is_none());
            assert!(agility(&RING, [60.0, f32::NAN], FAR, 1.0, -140.0).is_none());
        }
    }
}
