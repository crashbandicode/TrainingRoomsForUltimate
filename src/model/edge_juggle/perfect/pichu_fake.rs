//! Pichu's measured P-FD feint, ending at the near ledge.

use super::{
    aerial_clear, pad, snapped_direction, Correction, Outline, PichuBeat, Point, AGILITY_START,
    AGILITY_TRAVEL, FALL, FALL_SPEED, FAST_FALL_SPEED, FIRST_SPEED, GRAVITY, HALF, HEAD,
    HORIZONTAL_BRAKE, JUMP, JUMP_AERIAL, SPECIAL,
};

const RUN: f32 = 1.029;
const DRIFT_ACCEL: f32 = 0.10;
const PRESS_DEPTH: f32 = 48.0;
const JUMP_RISE: f32 = 35.99;

#[derive(Clone, Copy)]
enum Finish {
    Apex { forward: u32, brake: u32 },
    Zip { jump_rows: u32 },
}

#[derive(Clone, Copy)]
enum Phase {
    Dive,
    Jump { at: u32, finish: Finish },
    Zip { at: u32, aim: Point },
}

pub struct PichuFakeOut {
    outline: Outline,
    ledge: Point,
    inward: f32,
    bottom: f32,
    phase: Phase,
}

impl PichuFakeOut {
    pub(super) fn start(
        outline: Outline,
        ledge: Point,
        inward: f32,
        bottom: f32,
        beat: PichuBeat,
    ) -> Option<Self> {
        if !matches!(inward, -1.0 | 1.0)
            || beat.status != FALL
            || beat.jumps != 1
            || !(0.0..=DRIFT_ACCEL).contains(&beat.velocity[0].abs())
            || !(-FAST_FALL_SPEED..=0.0).contains(&beat.velocity[1])
            || !beat
                .position
                .into_iter()
                .chain(ledge)
                .chain([bottom])
                .all(f32::is_finite)
        {
            return None;
        }
        let flight = Self {
            outline,
            ledge,
            inward,
            bottom,
            phase: Phase::Dive,
        };
        let mut fall = Self::next_fall(beat.velocity[1]);
        let mut carry = beat.position;
        for _ in 0..64 {
            let next = [carry[0], carry[1] + fall];
            if !flight.clear(carry, next) {
                return None;
            }
            carry = next;
            if flight.ledge[1] - carry[1] >= PRESS_DEPTH {
                return flight.finish_from(carry).map(|_| flight);
            }
            fall = -FAST_FALL_SPEED;
        }
        None
    }

    fn clear(&self, from: Point, next: Point) -> bool {
        aerial_clear(
            self.outline.segments(),
            [from, next],
            self.bottom + 4.0,
            HEAD,
            HALF,
        )
    }

    fn next_fall(vy: f32) -> f32 {
        if vy < -FALL_SPEED {
            vy
        } else {
            (vy - GRAVITY).max(-FALL_SPEED)
        }
    }

    fn aim_from(&self, p: Point) -> Option<(f32, Point)> {
        let mut best = None;
        for degrees in (50u8..=90).step_by(2) {
            let (stick, aim) = snapped_direction(f32::from(degrees));
            let backward = [aim[0] * self.inward, aim[1]];
            for row in 1u8..=2 {
                let travel = FIRST_SPEED * f32::from(row);
                let end = [p[0] + backward[0] * travel, p[1] + backward[1] * travel];
                let outside = (self.ledge[0] - end[0]) * self.inward;
                let depth = self.ledge[1] - end[1];
                let margin = (outside - 6.3)
                    .min(11.4 - outside)
                    .min(depth - 13.6)
                    .min(15.6 - depth);
                if margin >= 0.3
                    && best.is_none_or(|(prior, _)| margin > prior + 0.05)
                    && self.clear(p, end)
                {
                    best = Some((margin, [stick[0] * self.inward, stick[1]]));
                }
            }
        }
        best
    }

    fn finish_from(&self, carry: Point) -> Option<Finish> {
        let outside = (self.ledge[0] - carry[0]) * self.inward;
        let ascent = (2.0 * GRAVITY * 36.02_f32).sqrt() - GRAVITY / 2.0;
        let apex_depth = self.ledge[1] - carry[1] - JUMP_RISE;
        if (8.0..=16.0).contains(&apex_depth) && (6.4..=38.0).contains(&outside) {
            // Same braking schedule as the flown P-FD apex form. Reserve seven
            // ledge-side units after braking and require the whole rise outside.
            let remaining = outside - 7.0;
            let mut moved = 0.1;
            let mut vx = 0.9;
            let mut forward = 0;
            for _ in 0..24 {
                let next = (vx + DRIFT_ACCEL).min(RUN);
                let stopping: f32 = (1u8..=11)
                    .map(|i| (next - DRIFT_ACCEL * f32::from(i)).max(0.0))
                    .sum();
                if moved + next + stopping > remaining {
                    break;
                }
                moved += next;
                vx = next;
                forward += 1;
            }
            if forward == 0 && outside <= 14.0 {
                return Some(Finish::Apex {
                    forward: 0,
                    brake: 0,
                });
            }
            if forward >= 2 {
                let brake = (vx / DRIFT_ACCEL).ceil() as u32;
                return Some(Finish::Apex { forward, brake });
            }
        }

        let mut p = [carry[0] + self.inward * 0.1, carry[1]];
        let mut vy = ascent;
        let mut vx = 1.0;
        let mut best = None;
        for row in 1..=45 {
            let zip_approach = if row >= 3 { vx - HORIZONTAL_BRAKE } else { vx };
            let finish = [p[0] + self.inward * zip_approach, p[1] + vy];
            if !self.clear(p, finish) {
                break;
            }
            if let Some((margin, _)) = self.aim_from(finish) {
                if best.is_none_or(|(prior, _)| margin > prior + 0.05) {
                    best = Some((margin, row));
                }
            }
            p = [p[0] + self.inward * vx, p[1] + vy];
            vx = RUN;
            vy = (vy - GRAVITY).max(-FALL_SPEED);
        }
        best.map(|(_, jump_rows)| Finish::Zip { jump_rows })
    }

    pub fn exit_point(&self) -> Point {
        [self.ledge[0] - self.inward * 8.85, self.ledge[1] - 14.6]
    }

    pub fn step(&mut self, beat: PichuBeat) -> Option<Correction> {
        if !beat
            .position
            .into_iter()
            .chain(beat.velocity)
            .all(f32::is_finite)
            || beat.position[1] <= self.bottom + 4.0
        {
            return None;
        }
        let [x, y] = beat.position;
        let forward = [self.inward, 0.0];
        let still = [0.0, 0.0];
        let jump = pad(forward, JUMP);
        let rise = pad([0.0, 1.0], SPECIAL);
        match self.phase {
            Phase::Dive if beat.status == FALL && beat.jumps == 1 => {
                let next = [x + beat.velocity[0], y + Self::next_fall(beat.velocity[1])];
                if self.ledge[1] - next[1] < PRESS_DEPTH {
                    return Some(pad([0.0, -1.0], 0));
                }
                if !beat.can_jump {
                    return None;
                }
                let finish = self.finish_from(next)?;
                self.phase = Phase::Jump {
                    at: beat.frame,
                    finish,
                };
                Some(if matches!(finish, Finish::Apex { forward: 0, .. }) {
                    pad(still, JUMP)
                } else {
                    jump
                })
            }
            Phase::Jump { at, finish } if beat.status == FALL && beat.frame == at => {
                Some(if matches!(finish, Finish::Apex { forward: 0, .. }) {
                    pad(still, JUMP)
                } else {
                    jump
                })
            }
            Phase::Jump { at, finish } if beat.status == JUMP_AERIAL => {
                let row = beat.frame.wrapping_sub(at);
                match (row, finish) {
                    (1, Finish::Apex { forward: 0, .. }) => Some(pad(still, JUMP)),
                    (1, _) => Some(jump),
                    (
                        2..=50,
                        Finish::Apex {
                            forward: frames,
                            brake,
                        },
                    ) => {
                        let stick = if row < frames {
                            forward
                        } else if row < frames + brake {
                            [-self.inward, 0.0]
                        } else {
                            still
                        };
                        Some(pad(stick, 0))
                    }
                    (_, Finish::Zip { jump_rows }) if row < jump_rows => Some(pad(forward, 0)),
                    (_, Finish::Zip { jump_rows }) if row <= jump_rows + 2 && beat.up => {
                        // The direction is aimed again at the live Agility launch.
                        let start = [
                            x + beat.velocity[0] - self.inward * HORIZONTAL_BRAKE,
                            y + (beat.velocity[1] - GRAVITY).max(-FALL_SPEED),
                        ];
                        let Some((_, aim)) = self.aim_from(start) else {
                            return (row < jump_rows + 2).then_some(pad(forward, 0));
                        };
                        self.phase = Phase::Zip {
                            at: beat.frame,
                            aim,
                        };
                        Some(rise)
                    }
                    _ => None,
                }
            }
            Phase::Zip { at, aim } => match (beat.frame.wrapping_sub(at), beat.status) {
                (0, JUMP_AERIAL) => Some(rise),
                (1, AGILITY_START) => Some(rise),
                (2..=20, AGILITY_START | AGILITY_TRAVEL) => Some(pad(aim, 0)),
                _ => None,
            },
            _ => None,
        }
    }
}

#[cfg(test)]
lib_unit_tests! {
    mod tests {
        use super::*;
        use super::super::{FALL, JUMP_AERIAL, AGILITY_START, AGILITY_TRAVEL, JUMP, SPECIAL};

        fn live(frame: u32, status: i32, position: Point, velocity: Point, jumps: u8) -> PichuBeat {
            PichuBeat { frame, status, position, velocity, jumps, can_jump: jumps == 1, side: true, up: true }
        }

        fn fixture() -> Outline {
            let mut outline = Outline::new();
            outline.add([[-79.986, 0.0], [79.989, 0.0]]);
            outline.add([[-79.986, 0.0], [-79.986, -8.319]]);
            outline.add([[-79.986, -8.319], [-64.395, -25.111]]);
            outline.add([[-64.395, -25.111], [-41.396, -40.099]]);
            outline.add([[-41.396, -40.099], [41.416, -40.118]]);
            outline
        }

        #[test]
        fn pichu_fake_out_mirrors_the_dive_then_brakes_for_the_near_apex_catch() {
            let ledge = [-79.986, 0.0];
            let reference = [
                (0, FALL, [-99.986, -28.0], [0.0, 0.0], 1, [0.0, -1.0], 0),
                (7, FALL, [-99.986, -43.14], [0.0, -2.5], 1, [0.0, -1.0], 0),
                (8, FALL, [-99.986, -45.64], [0.0, -2.5], 1, [1.0, 0.0], JUMP),
                (9, JUMP_AERIAL, [-99.886, -48.14], [0.1, -2.5], 0, [1.0, 0.0], JUMP),
                (14, JUMP_AERIAL, [-94.77, -34.0111], [1.029, 2.5458], 0, [1.0, 0.0], 0),
                (15, JUMP_AERIAL, [-93.741, -31.6053], [1.029, 2.4058], 0, [-1.0, 0.0], 0),
                (25, JUMP_AERIAL, [-88.951, -15.2475], [0.029, 1.0058], 0, [-1.0, 0.0], 0),
                (26, JUMP_AERIAL, [-89.022, -14.3817], [-0.071, 0.8658], 0, [0.0, 0.0], 0),
                (32, JUMP_AERIAL, [-89.2905, -12.127], [-0.026, 0.0258], 0, [0.0, 0.0], 0),
            ];
            for inward in [-1.0, 1.0] {
                let spawn = live(0, FALL, [-99.986 * inward, -28.0], [0.0, 0.0], 1);
                let mut stage = Outline::new();
                for line in fixture().segments() {
                    stage.add(line.map(|[x, y]| [x * inward, y]));
                }
                let mut flight = PichuFakeOut::start(stage, [ledge[0] * inward, ledge[1]], inward, -140.0, spawn)
                    .expect("valid fake-out at the matched Lower-area start");
                for (frame, status, p, v, jumps, stick, button) in reference {
                    let next = live(frame, status, [p[0] * inward, p[1]], [v[0] * inward, v[1]], jumps);
                    let pad = flight.step(next).unwrap_or_else(|| panic!("missing frame {frame}"));
                    assert_eq!(pad.stick, Some([stick[0] * inward, stick[1]]), "frame {frame}");
                    assert_eq!(pad.press & (JUMP | SPECIAL), button, "frame {frame}");
                }
                assert!(flight.step(live(33, 0x75, [-88.7324 * inward, -13.45], [0.0, 0.0], 0)).is_none());
            }
        }

        #[test]
        fn pichu_fake_out_near_zip_lands_a_measured_step_and_requires_agility() {
            let mut flight = PichuFakeOut::start(fixture(), [-79.986, 0.0], 1.0, -140.0,
                live(0, FALL, [-123.986, -0.42], [0.0, -0.28], 1)).expect("valid near zip");
            let before_jump = live(20, FALL, [-123.986, -48.06], [0.0, -2.5], 1);
            assert_eq!(flight.step(before_jump).expect("jump").press & JUMP, JUMP);
            let ascent = live(54, JUMP_AERIAL, [-89.958, -21.9892], [1.029, -1.3742], 0);
            assert!(flight.step(PichuBeat { up: false, ..ascent }).is_none());
            assert_eq!(flight.step(ascent).expect("Agility").press, SPECIAL);
            assert_eq!(flight.step(live(55, AGILITY_START, [-88.9365, -23.5034], [1.0215, -1.5142], 0))
                .expect("second B frame").press, SPECIAL);
            let aim = flight.step(live(67, AGILITY_TRAVEL, [-88.9365, -23.4954], [0.0, 0.0053], 0))
                .expect("zip aim").stick.unwrap();
            let length = aim[0].hypot(aim[1]);
            let out = 8.9505 - aim[0] / length * 8.3;
            let depth = 23.4954 - aim[1] / length * 8.3;
            assert!((6.3..=11.4).contains(&out) && (13.6..=15.6).contains(&depth));
        }
    }
}
