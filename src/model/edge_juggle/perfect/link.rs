//! Frame-relative pad policy for the measured Remote Bomb down throw.

use super::{niche::pad, region, Correction, Point, RouteFacts, ATTACK, JUMP, SPECIAL};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Motion {
    Free,
    Pulling,
    // The live, owned bomb is held on the last sampled pull frame.
    ItemHeld,
    ItemThrow,
    // The live, owned bomb is loose on the last sampled throw frame.
    ItemLoose,
    Detonate,
}

fn input(elapsed: u32, motion: Motion) -> Option<Correction> {
    use Motion::*;
    let (stick_y, buttons) = match (elapsed, motion) {
        (0, Free) | (1..=2, Pulling) | (60, ItemLoose) | (61..=62, Detonate) => (-0.6, SPECIAL),
        (40, ItemHeld) | (41, ItemThrow) => (-1.0, ATTACK),
        (3..=39, Pulling) | (42..=59, ItemThrow) | (63..=71, Detonate) => (0.0, 0),
        _ => return None,
    };
    Some(pad([0.0, stick_y], buttons))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pull {
    Wait,
    Press,
}

fn pull_window(y: f32, vy: f32) -> Option<Pull> {
    const GRAVITY: f32 = 0.096;
    const FALL_SPEED: f32 = 1.6;
    const HIT_FRAMES: f32 = 72.0;
    if !y.is_finite() || !(-FALL_SPEED..=0.0).contains(&vy) {
        return None;
    }
    let accelerating = ((FALL_SPEED + vy) / GRAVITY).floor().min(HIT_FRAMES);
    let fall = accelerating * vy
        - GRAVITY * accelerating * (accelerating + 1.0) / 2.0
        - FALL_SPEED * (HIT_FRAMES - accelerating);
    let hit = y + fall;
    if hit < -120.0 {
        None
    } else if hit > -104.0 {
        Some(Pull::Wait)
    } else {
        Some(Pull::Press)
    }
}

fn launch_pad(elapsed: u32, status: i32, inward: f32, hit: bool) -> Option<Correction> {
    if !matches!(inward, -1.0 | 1.0) {
        return None;
    }
    const DI: f32 = 85.0 / 120.0;
    match (elapsed, status, hit) {
        (70..=71, 0x1e5, false) | (72..=81, 0x49 | 0x4a, true) => Some(pad([DI * inward, -DI], 0)),
        (82..=132, 0x49 | 0x4a | 0x4f | FALL, true) => Some(pad([inward, 0.0], 0)),
        _ => None,
    }
}

fn spin_pad(age: u32, inward: f32) -> Option<Correction> {
    if !matches!(inward, -1.0 | 1.0) {
        return None;
    }
    match age {
        0..=1 | 4..=5 => Some(pad([0.0, 1.0], SPECIAL)),
        2..=3 | 6..=37 => Some(pad([-inward, 0.0], 0)),
        _ => None,
    }
}

pub const KIND: i32 = 0x2;
pub const ROW: u8 = 44;
const FALL: i32 = 0xe;
const PULL: i32 = 0x1df;
const THROW: i32 = 0x8f;
const DETONATE: i32 = 0x1e5;
const SPIN: i32 = 0x1de;
const SPIN_END: i32 = 0x1e8;
const THROW_FRAME: u32 = 40;
const DETONATE_FRAME: u32 = 60;
const HIT_FRAME: u32 = 72;
const JUMP_FRAME: u32 = HIT_FRAME + 51 + 9;
const JUMP_TO_SPIN: u32 = 28;
// g4/tables/link-lkx.txt, with trailing empty rows omitted.
const CROSS_RING: [u32; 11] = [0x3e0; 11];

fn ends([left, right]: [Point; 2], body: Point) -> [Point; 2] {
    if body[0] > left[0].midpoint(right[0]) { [right, left] } else { [left, right] }
}

pub fn in_region(stage: (i32, u32), ledges: [Point; 2], body: Point, damage: f32) -> bool {
    let [near, far] = ends(ledges, body);
    let inward = (far[0] - near[0]).signum();
    let outside = (near[0] - body[0]) * inward;
    let height = body[1] - near[1];
    super::is_crossing_stage(stage)
        && (inward == 1.0 || stage == (3, 0))
        && (95.0..=105.0).contains(&damage)
        && (20.0..40.0).contains(&outside)
        && (-4.0..=40.0).contains(&height)
        && region::contains(&CROSS_RING, outside, height)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OwnedBomb {
    Absent,
    Held,
    Loose,
}

#[derive(Clone, Copy)]
pub struct LinkBeat {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub damage: f32,
    pub facing: f32,
    pub jumps: u8,
    pub bomb: Option<OwnedBomb>,
    pub can_pull: bool,
    pub can_jump: bool,
    pub can_spin: bool,
}

pub struct LinkFlight {
    exit: Point,
    from: Point,
    damage: f32,
    pulled: Option<u32>,
    self_hit: Option<Point>,
    jumped: bool,
}

impl LinkFlight {
    pub fn start(facts: &RouteFacts<'_>, beat: LinkBeat) -> Option<Self> {
        let [near, far] = ends(facts.ledges, beat.position);
        let inward = (far[0] - near[0]).signum();
        facts.crossing_band()?;
        let has_flat_underside = facts.outline.iter().any(|line| {
            line.iter().all(|&[_, y]| (y - near[1] + 40.1).abs() < 0.12)
                && line[0][0].min(line[1][0]) < near[0].midpoint(far[0])
                && line[0][0].max(line[1][0]) > near[0].midpoint(far[0])
        });
        if facts.fighter != KIND
            || facts.position != beat.position
            || !in_region(facts.stage, facts.ledges, beat.position, beat.damage)
            || ((far[0] - near[0]).abs() - 159.98).abs() > 0.12
            || (far[1] - near[1]).abs() > 0.12
            || (facts.blast[3] - near[1] + 140.0).abs() > 0.12
            || !has_flat_underside
            || beat.status != FALL
            || beat.velocity[0] != 0.0
            || beat.facing != -inward
            || beat.jumps != 1
            || beat.bomb != Some(OwnedBomb::Absent)
            || !beat.can_pull
            || !beat.can_spin
        {
            return None;
        }
        pull_window(beat.position[1] - near[1], beat.velocity[1])?;
        Some(Self {
            exit: far,
            from: beat.position,
            damage: beat.damage,
            pulled: None,
            self_hit: None,
            jumped: false,
        })
    }

    pub fn exit_point(&self) -> Point {
        self.exit
    }

    pub fn waiting_to_pull(&self) -> bool {
        self.pulled.is_none()
    }

    fn jump_age(&self) -> u32 {
        JUMP_FRAME + u32::from(self.damage > 100.0)
    }

    fn samples_edge(&self, frame: u32, age: u32) -> bool {
        self.pulled.is_some_and(|pull| frame.wrapping_sub(pull).wrapping_sub(age) < 2)
    }

    pub fn is_jump_frame(&self, frame: u32) -> bool {
        self.samples_edge(frame, self.jump_age())
    }

    pub fn is_spin_frame(&self, frame: u32) -> bool {
        self.samples_edge(frame, self.jump_age() + JUMP_TO_SPIN)
    }

    pub fn needs_bomb_sample(&self, frame: u32) -> bool {
        self.pulled
            .is_none_or(|pull| matches!(frame.wrapping_sub(pull), THROW_FRAME | DETONATE_FRAME))
    }

    pub fn step(&mut self, beat: LinkBeat) -> Option<Correction> {
        let [x, y] = beat.position;
        let inward = (self.exit[0] - self.from[0]).signum();
        if !x.is_finite()
            || !y.is_finite()
            || !beat.damage.is_finite()
            || !matches!(beat.facing, -1.0 | 1.0)
        {
            return None;
        }
        let age = if let Some(pulled) = self.pulled {
            beat.frame.wrapping_sub(pulled)
        } else {
            if beat.status != FALL
                || beat.facing != -inward
                || (x - self.from[0]).abs() > 0.1
                || beat.velocity[0] != 0.0
                || beat.damage != self.damage
                || beat.jumps != 1
                || beat.bomb != Some(OwnedBomb::Absent)
                || !beat.can_pull
                || !beat.can_spin
            {
                return None;
            }
            return match pull_window(y - self.exit[1], beat.velocity[1])? {
                Pull::Wait => Some(pad([0.0, 0.0], 0)),
                Pull::Press => {
                    self.pulled = Some(beat.frame);
                    input(0, Motion::Free)
                }
            };
        };
        if age < HIT_FRAME {
            if beat.jumps != 1
                || beat.damage != self.damage
                || (x - self.from[0]).abs() > 0.1
                || beat.facing != -inward
            {
                return None;
            }
            if age >= 70 {
                return launch_pad(age, beat.status, inward, false);
            }
            let motion = match (age, beat.status, beat.bomb) {
                (0, FALL | PULL, _) => Motion::Free,
                (1..=2, FALL | PULL, _) | (3..=39, PULL, _) => Motion::Pulling,
                (THROW_FRAME, PULL, Some(OwnedBomb::Held)) => Motion::ItemHeld,
                (41, PULL | THROW, _) | (42..=59, THROW, _) => Motion::ItemThrow,
                (DETONATE_FRAME, THROW, Some(OwnedBomb::Loose)) => Motion::ItemLoose,
                (61..=62, THROW | DETONATE, _) | (63..=69, DETONATE, _) => Motion::Detonate,
                _ => return None,
            };
            return input(age, motion);
        }
        if (beat.damage - (self.damage + 8.4)).abs() > 0.01 {
            return None;
        }
        if age == HIT_FRAME {
            if !matches!(beat.status, 0x49 | 0x4a)
                || (x - self.from[0]).abs() > 0.1
                || !(-124.0..=-100.0).contains(&(y - self.exit[1]))
            {
                return None;
            }
            self.self_hit = Some(beat.position);
        }
        let hit = self.self_hit?;
        let jump_age = self.jump_age();
        if age < jump_age {
            return launch_pad(age, beat.status, inward, true);
        }
        // The 105% launch plus its later jump advances 7.26 units beyond the 100% trace.
        let launch_gain = (self.damage - 100.0).max(0.0) * (7.26 / 5.0);
        let stick = [inward, 0.0];
        match age - jump_age {
            0 if beat.can_jump
                && beat.jumps == 1
                && matches!(beat.status, 0x49 | 0x4a | 0x4f | FALL)
                && (135.0..=143.0).contains(&((x - hit[0]) * inward - launch_gain)) =>
            {
                self.jumped = true;
                Some(pad(stick, JUMP))
            }
            1 if self.jumped && (beat.can_jump || beat.jumps == 0) => Some(pad(stick, JUMP)),
            2..JUMP_TO_SPIN
                if self.jumped && beat.jumps == 0 && matches!(beat.status, 0xc | 0x6d | FALL) =>
            {
                Some(pad(stick, 0))
            }
            JUMP_TO_SPIN
                if self.jumped
                    && beat.jumps == 0
                    && beat.can_spin
                    && matches!(beat.status, 0xc | FALL)
                    && (40.0..=76.0).contains(&((x - self.exit[0]) * inward + 79.99 - launch_gain))
                    && (-66.0..=-39.0).contains(&(y - self.exit[1])) =>
            {
                spin_pad(0, inward)
            }
            phase if phase > JUMP_TO_SPIN
                && self.jumped
                && (matches!(beat.status, SPIN | SPIN_END)
                    || phase == JUMP_TO_SPIN + 1 && beat.can_spin && matches!(beat.status, 0xc | FALL)) =>
            {
                spin_pad(phase - JUMP_TO_SPIN, inward)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
lib_unit_tests! {
    mod tests {
        use super::*;

        const LEDGES: [Point; 2] = [[-79.99, 0.0], [79.99, 0.0]];
        const HULL: [super::super::Segment; 4] = [
            [[-79.99, 0.0], [-22.0, -40.1]],
            [[-22.0, -40.1], [22.0, -40.1]],
            [[22.0, -40.1], [79.99, 0.0]],
            LEDGES,
        ];

        fn facts(stage: (i32, u32), position: Point) -> RouteFacts<'static> {
            RouteFacts {
                fighter: KIND, stage, position, player: [-79.0, 0.0], player_reach: 12.0,
                ledges: LEDGES, blast: [-240.0, 240.0, 180.0, -140.0],
                outline: &HULL, body_clearance: 16.2, ledge_distance: 16.2,
                fall_budget: 1.6, reach: [210.0, 80.0],
            }
        }

        fn release() -> LinkBeat {
            LinkBeat {
                frame: 16, status: 0xe, position: [-101.99, -2.288], velocity: [0.0, -0.192],
                damage: 100.0, facing: -1.0, jumps: 1, bomb: Some(OwnedBomb::Absent),
                can_pull: true, can_jump: true, can_spin: true,
            }
        }

        #[test]
        fn link_admits_only_the_owned_empty_bomb_slot_and_the_swept_envelope() {
            let live = release();
            for stage in [(3, 0), (0, 0), (311, 0)] {
                assert!(LinkFlight::start(&facts(stage, live.position), live).is_some());
            }
            for stage in [(1, 0), (3, 1), (0, 2)] {
                assert!(LinkFlight::start(&facts(stage, live.position), live).is_none());
            }
            for bad in [
                LinkBeat { facing: 1.0, ..live },
                LinkBeat { jumps: 0, ..live },
                LinkBeat { damage: 106.0, ..live },
                LinkBeat { damage: 115.0, ..live },
                LinkBeat { bomb: None, ..live },
                LinkBeat { bomb: Some(OwnedBomb::Held), ..live },
                LinkBeat { bomb: Some(OwnedBomb::Loose), ..live },
                LinkBeat { can_pull: false, ..live },
                LinkBeat { can_spin: false, ..live },
                LinkBeat { velocity: [0.25, -0.2], ..live },
                LinkBeat { position: [-101.99, -24.0], ..live },
                LinkBeat { position: [-122.0, 0.0], ..live },
            ] {
                assert!(LinkFlight::start(&facts((3, 0), bad.position), bad).is_none());
            }
        }

            #[test]
        fn link_duplicate_pull_tick_keeps_the_already_committed_pad() {
            let beat = release();
            let mut flight = LinkFlight::start(&facts((3, 0), beat.position), beat).unwrap();
            let pull = flight.step(beat).unwrap();
            assert_eq!(pull.press, SPECIAL);
            assert_eq!(flight.step(beat), Some(pull));
        }

    #[test]
        fn link_bomb_requires_ownership_on_the_actual_drop_and_detonation_edges() {
            let beat = release();
            for (frame, status, wanted, wrong) in [
                (40, 0x1df, OwnedBomb::Held, OwnedBomb::Loose),
                (60, 0x8f, OwnedBomb::Loose, OwnedBomb::Held),
            ] {
                let mut flight = LinkFlight::start(&facts((3, 0), beat.position), beat).unwrap();
                flight.step(beat).unwrap();
                let clock = LinkBeat { frame: beat.frame + frame, status, ..beat };
                assert!(flight.step(LinkBeat { bomb: Some(wrong), ..clock }).is_none());
                let correct = flight.step(LinkBeat { bomb: Some(wanted), ..clock }).unwrap();
                assert_eq!(correct.press, if frame == 40 { ATTACK } else { SPECIAL });
            }
        }


        #[test]
        fn link_uses_the_far_left_ledge_after_a_measured_right_side_release() {
            let beat = LinkBeat { position: [117.99, 9.104], velocity: [0.0, -1.6], facing: 1.0,
                damage: 105.0, ..release() };
            let live = facts((3, 0), beat.position);
            let mut flight = LinkFlight::start(&live, beat).unwrap();
            assert_eq!(flight.exit_point(), LEDGES[0]);
            assert!(flight.step(beat).is_some());
            let left_blast = LinkBeat { frame: beat.frame + 70, status: DETONATE, ..beat };
            assert_eq!(flight.step(left_blast), Some(pad([-85.0 / 120.0, -85.0 / 120.0], 0)));
            for stage in [(0, 0), (311, 0), (3, 1)] {
                assert!(LinkFlight::start(&facts(stage, beat.position), beat).is_none());
            }
        }

        #[test]
        fn link_keeps_the_105_percent_jump_and_spin_one_frame_later() {
            // Exact fg4-m16l-xps, lkb-ca-d105-1. The two edges are at 133 and 161.
            let entry = LinkBeat { position: [-101.99, -2.0], velocity: [0.0, 0.0],
                damage: 105.0, ..release() };
            let mut flight = LinkFlight::start(&facts((3, 0), entry.position), entry).unwrap();
            let beat = LinkBeat { position: [-101.99, -4.304], velocity: [0.0, -0.192], ..entry };
            assert_eq!(flight.step(beat).unwrap().press, SPECIAL);
            let clock = |age, status, position, jumps| LinkBeat {
                frame: beat.frame + age, status, position, jumps, damage: 113.4, ..beat
            };
            assert_eq!(flight.step(clock(72, 0x4a, [-101.99, -109.872], 1)).unwrap().press, 0);
            assert_eq!(flight.step(clock(132, 0x4f, [45.737, -78.494], 1)).unwrap().press, 0);
            assert_eq!(flight.step(clock(133, 0x4f, [46.841, -79.529], 1)).unwrap().press, JUMP);
            assert_eq!(flight.step(clock(134, 0xc, [47.957, -80.600], 0)).unwrap().press, JUMP);
            assert_eq!(flight.step(clock(160, 0xc, [76.003, -47.934], 0)).unwrap().press, 0);
            assert_eq!(flight.step(clock(161, 0xc, [76.927, -48.118], 0)).unwrap().press, SPECIAL);
        }

        #[test]
        fn link_changes_to_the_jump_and_spin_only_on_measured_native_transitions() {
            let beat = release();
            let mut flight = LinkFlight::start(&facts((3, 0), beat.position), beat).unwrap();
            assert_eq!(flight.step(beat).unwrap().press, SPECIAL);
            let live = |age, status, position, rest: LinkBeat| LinkBeat {
                frame: beat.frame + age, status, position, ..rest
            };
            let active = LinkBeat { damage: 108.4, ..beat };
            assert_eq!(flight.step(live(72, 0x4a, [-101.99, -107.856], active)).unwrap().press, 0);
            assert_eq!(flight.step(live(123, 0x4f, [30.446, -72.125], active)).unwrap().press, 0);
            let jump = live(132, 0x4f, [39.576, -80.085], active);
            assert_eq!(flight.step(jump).unwrap().press, JUMP);
            let consumed = LinkBeat { jumps: 0, ..active };
            assert_eq!(flight.step(live(133, 0xc, [40.654, -81.144], consumed)).unwrap().press, JUMP);
            assert_eq!(flight.step(live(159, 0xc, [68.884, -48.304], consumed)).unwrap().press, 0);
            assert_eq!(flight.step(live(160, 0xc, [69.808, -48.488], consumed)).unwrap().press, SPECIAL);
            let spin = live(162, 0x1de, [71.645, -46.421], consumed);
            assert_eq!(flight.step(spin), Some(pad([-1.0, 0.0], 0)));
        }

        #[test]
        fn link_aims_its_bomb_launch_before_the_native_hit_and_holds_it_through_hitlag() {
            for inward in [-1.0, 1.0] {
                let di = pad([inward * 85.0 / 120.0, -85.0 / 120.0], 0);
                assert_eq!(launch_pad(69, 0x1e5, inward, false), None);
                for age in [70, 71] {
                    assert_eq!(launch_pad(age, 0x1e5, inward, false), Some(di));
                }
                for age in [72, 79, 81] {
                    assert_eq!(launch_pad(age, 0x49, inward, true), Some(di));
                }
                assert_eq!(launch_pad(82, 0x49, inward, true), Some(pad([inward, 0.0], 0)));
                assert_eq!(launch_pad(123, 0x4f, inward, true), Some(pad([inward, 0.0], 0)));
            }
        }

        #[test]
        fn link_does_not_continue_the_crossing_after_a_missing_or_unrelated_launch() {
            assert_eq!(launch_pad(72, 0x1e5, 1.0, false), None);
            assert_eq!(launch_pad(72, 0x4a, 1.0, false), None);
            assert_eq!(launch_pad(82, 0x44, 1.0, true), None);
            assert_eq!(launch_pad(82, 0x4a, f32::NAN, true), None);
        }

        #[test]
        fn link_waits_until_a_down_throw_blast_can_reach_the_far_slope() {
            // The 72-frame lookahead uses the actual fall speed at the decision.
            // fg4-m13d-yoga: lkb-ca-1 pulls at -2.288 with vy -0.192.
            assert_eq!(pull_window(-2.288, -0.192), Some(Pull::Press));
            // High starts wait until they reach the conservative hit band.
            assert_eq!(pull_window(38.0, 0.0), Some(Pull::Wait));
            assert_eq!(pull_window(10.5, -1.6), Some(Pull::Press));
            assert_eq!(pull_window(-23.0, -1.6), None);
            assert_eq!(pull_window(f32::NAN, 0.0), None);
            assert_eq!(pull_window(0.0, 1.0), None);
        }

        #[test]
        fn link_faces_the_far_lip_during_the_two_spin_attack_press_edges() {
            // The confirmed route B briefly returns to Up+B, then finishes
            // facing the far lip instead of waiting for the rear catch window.
            for inward in [-1.0, 1.0] {
                for (age, sx, sy, press) in [
                    (0, 0.0, 1.0, SPECIAL), (1, 0.0, 1.0, SPECIAL),
                    (2, -inward, 0.0, 0), (3, -inward, 0.0, 0),
                    (4, 0.0, 1.0, SPECIAL), (5, 0.0, 1.0, SPECIAL),
                    (6, -inward, 0.0, 0), (31, -inward, 0.0, 0),
                ] {
                    assert_eq!(spin_pad(age, inward), Some(pad([sx, sy], press)));
                }
            }
            assert!(spin_pad(1, f32::NAN).is_none());
        }

        #[test]
        fn link_down_throw_keeps_bomb_in_range_for_the_twelve_frame_blast() {
            use Motion::*;
            // g4-lk-tdl-100, FD, 75b71de9. The first applied pull pad is f0.
            // Pad floats are the values applied to the game, not raw macro units.
            for (frame, motion, stick, buttons) in [
                (0, Free, [0.0, -0.6], SPECIAL),
                (2, Pulling, [0.0, -0.6], SPECIAL),
                (3, Pulling, [0.0, 0.0], 0),
                (39, Pulling, [0.0, 0.0], 0),
                (40, ItemHeld, [0.0, -1.0], ATTACK),
                (41, ItemThrow, [0.0, -1.0], ATTACK),
                (42, ItemThrow, [0.0, 0.0], 0),
                (59, ItemThrow, [0.0, 0.0], 0),
                (60, ItemLoose, [0.0, -0.6], SPECIAL),
                (62, Detonate, [0.0, -0.6], SPECIAL),
                (63, Detonate, [0.0, 0.0], 0),
                (71, Detonate, [0.0, 0.0], 0),
            ] {
                let pad = input(frame, motion).unwrap_or_else(|| panic!("missing frame {frame}"));
                assert_eq!((pad.stick, pad.press), (Some(stick), buttons), "frame {frame}");
            }
        }

        #[test]
        fn link_does_not_retry_a_throw_or_detonation_outside_its_measured_window() {
            use Motion::*;
            assert!(input(40, Pulling).is_none());
            assert!(input(41, ItemHeld).is_none());
            assert!(input(59, ItemLoose).is_none());
            assert!(input(60, ItemThrow).is_none());
            assert!(input(61, ItemLoose).is_none());
            assert!(input(72, Detonate).is_none());
        }
    }
}
