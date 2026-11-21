//! Diddy Kong's measured full-charge Rocketbarrel crossing.
use super::{aerial_clear, niche::pad, region, Correction, Outline, Point, RouteFacts, SPECIAL};

pub const KIND: i32 = 0x27;
pub const ROW: u8 = 5;
pub const FALL: i32 = 0xe;
const START: i32 = 0x1de;
const CHARGE: i32 = 0x1f0;
const FLIGHT: i32 = 0x1f2;
const END: i32 = 0x1f5;
const SPECIAL_FALL: i32 = 0x10;
const GRAVITY: f32 = 0.125;
const TERMINAL: f32 = 1.75;
const AIR_SPEED: f32 = 0.924;
const AIR_ACCEL: f32 = 0.05;
const BRAKE: f32 = 0.008;
const HEAD: f32 = 11.5;
const CLEARANCE: f32 = 4.4;
const ARM: u32 = 3;
const HOLD: u32 = 68;
const FLIGHT_ROWS: u8 = 42;
const END_ROWS: u8 = 11;

// 37 cells in the DIDDY_DK_BARREL_RING bake, reference-crossings/g3/diddy.md.
const RING: [u32; 20] = [
    0, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 4,
];

// Proven charge depths by nearest one-unit outside column, ranked by slack.
// The chosen depth and pull-up are still projected on the current outline.
const DEPTHS: [[u8; 6]; 9] = [
    [36, 40, 48, 28, 44, 32],
    [32, 36, 44, 48, 28, 40],
    [32, 28, 36, 40, 44, 0],
    [28, 32, 36, 44, 40, 0],
    [32, 36, 28, 44, 40, 0],
    [32, 36, 40, 28, 0, 0],
    [36, 32, 28, 40, 0, 0],
    [32, 36, 28, 40, 0, 0],
    [36, 32, 28, 40, 0, 0],
];
const CATCH_BANDS: [[f32; 4]; 4] = [
    [5.13, 7.0, -17.5, -13.6],
    [5.13, 7.86, -17.5, -14.7],
    [5.13, 8.75, -17.5, -15.3],
    [5.13, 6.5, -18.7, -13.6],
];
const STEER_HEIGHT: f32 = -13.6;
const STEER_OUTSIDE: f32 = 6.1;

#[derive(Clone, Copy)]
pub struct BarrelSample {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub facing: f32,
    pub can_rise: bool,
}

#[derive(Clone, Copy)]
struct Pose {
    position: Point,
    velocity: Point,
}

impl Pose {
    fn moved(mut self) -> Self {
        for axis in 0..2 {
            self.position[axis] += self.velocity[axis];
        }
        self
    }

    fn air(mut self, stick: f32, clamp: bool) -> Self {
        let [mut x, y] = self.velocity;
        if clamp {
            x = x.clamp(-AIR_SPEED, AIR_SPEED);
        }
        if x.abs() > AIR_SPEED {
            x -= BRAKE.copysign(x);
        } else if stick == 0.0 {
            x = (x.abs() - BRAKE).max(0.0).copysign(x);
        } else {
            x = (x + AIR_ACCEL * stick).clamp(-AIR_SPEED, AIR_SPEED);
        }
        self.velocity = [x, (y - GRAVITY).max(-TERMINAL)];
        self.moved()
    }

    fn fly(mut self, up: bool) -> Self {
        let [x, y] = self.velocity;
        let speed = x.hypot(y);
        let angle = y.atan2(x);
        let aim = if up { std::f32::consts::FRAC_PI_2 } else { 0.0 };
        let limit = 5.0_f32.to_radians();
        let (sin, cos) = (angle + (aim - angle).clamp(-limit, limit)).sin_cos();
        self.velocity = [speed * cos, speed * sin];
        self.rocket_speed()
    }

    fn rocket_speed(mut self) -> Self {
        let [x, y] = self.velocity;
        self.velocity = [
            if x.abs() > 2.0 * AIR_SPEED {
                x - BRAKE.copysign(x)
            } else {
                x
            },
            if y > -TERMINAL {
                (y - GRAVITY).max(-TERMINAL)
            } else {
                (y + 0.016).min(-TERMINAL)
            },
        ];
        self.moved()
    }

    fn steer(self, span: f32, end_rows: u8) -> f32 {
        let mut back = self;
        for row in 1..=90 {
            back = back.air(-1.0, row <= end_rows);
            if row >= end_rows && back.velocity[1] < 0.0 && back.position[1] <= STEER_HEIGHT {
                return if back.position[0] - span > STEER_OUTSIDE {
                    -1.0
                } else {
                    1.0
                };
            }
        }
        1.0
    }

    fn falling_catch(self, span: f32) -> Option<f32> {
        if self.velocity[1] >= 0.0 {
            return None;
        }
        let [x, y] = self.position;
        let outside = x - span;
        CATCH_BANDS
            .into_iter()
            .map(|[left, right, bottom, top]| {
                (outside - left)
                    .min(right - outside)
                    .min(y - bottom)
                    .min(top - y)
            })
            .reduce(f32::max)
            .filter(|&slack| slack >= 0.5)
    }
}

struct Course {
    outline: Outline,
    near: Point,
    far: Point,
    direction: f32,
    span: f32,
    bottom: f32,
}

impl Course {
    fn from_facts(facts: &RouteFacts<'_>) -> Option<Self> {
        let [[l, ly], [r, ry]] = facts.ledges;
        let span = r - l;
        let bottom = facts.blast[3];
        if !super::is_crossing_stage(facts.stage)
            || facts.fighter != KIND
            || !(159.0..=161.0).contains(&span)
            || (ly - ry).abs() > 0.5
            || !(facts.body_clearance > 0.0 && facts.body_clearance <= HEAD)
            || !facts
                .outline
                .iter()
                .flatten()
                .flatten()
                .all(|x| x.is_finite())
        {
            return None;
        }
        facts.crossing_band()?;
        let floor = facts
            .outline
            .iter()
            .flatten()
            .map(|p| p[1])
            .reduce(f32::min)?;
        let (near, far, direction) = if facts.position[0] < l {
            ([l, ly], [r, ry], 1.0)
        } else {
            ([r, ry], [l, ly], -1.0)
        };
        if (floor - near[1] + 40.1).abs() > 1.0 || (bottom - near[1] + 140.0).abs() > 1.0 {
            return None;
        }
        let mut outline = Outline::new();
        for &line in facts.outline {
            outline.add(line.map(|[x, y]| [(x - near[0]) * direction, y - near[1]]));
        }
        Some(Self {
            outline,
            near,
            far,
            direction,
            span,
            bottom: bottom - near[1] + 4.0,
        })
    }

    fn pose(&self, sample: BarrelSample) -> Pose {
        let [x, y] = sample.position;
        let [vx, vy] = sample.velocity;
        Pose {
            position: [(x - self.near[0]) * self.direction, y - self.near[1]],
            velocity: [vx * self.direction, vy],
        }
    }

    fn clear(&self, a: Pose, b: Pose) -> bool {
        aerial_clear(
            self.outline.segments(),
            [a.position, b.position],
            self.bottom,
            HEAD + CLEARANCE,
            0.0,
        )
    }

    fn step_to(&self, pose: &mut Pose, next: Pose) -> bool {
        if !self.clear(*pose, next) {
            return false;
        }
        *pose = next;
        true
    }

    fn course_to_press(&self, mut pose: Pose, height: f32) -> Option<(Pose, u16)> {
        for frames in 0..=80 {
            if pose.position[1] <= height {
                return (-16.0..=-8.0)
                    .contains(&pose.position[0])
                    .then_some((pose, frames));
            }
            let next = pose.air(0.0, false);
            if !self.step_to(&mut pose, next) {
                return None;
            }
        }
        None
    }

    fn after_charge(&self, press: Pose) -> Option<Pose> {
        let first = press.air(0.0, false);
        // 6 startup rows and the 60-row charge plus its carry row, per gen_diddy.py.
        let charged = Pose {
            position: [first.position[0], first.position[1] - 31.275],
            velocity: [0.0; 2],
        };
        let (sin, cos) = 18.48_f32.to_radians().sin_cos();
        let moving = Pose {
            velocity: [4.575 * cos, 4.575 * sin],
            ..charged
        }
        .rocket_speed();
        (self.clear(press, first) && self.clear(first, charged) && self.clear(charged, moving))
            .then_some(moving)
    }

    fn finish(&self, mut pose: Pose, flight_row: u8) -> Option<(f32, u16)> {
        for _ in flight_row..=FLIGHT_ROWS {
            let next = pose.fly(true);
            if !self.step_to(&mut pose, next) {
                return None;
            }
        }
        let mut apex = pose.position[1];
        for row in 1u16..=60 {
            let end_rows = u16::from(END_ROWS - 1);
            let remain = end_rows.saturating_sub(row - 1) as u8;
            let next = pose.air(pose.steer(self.span, remain), remain != 0);
            if !self.step_to(&mut pose, next) {
                return None;
            }
            apex = apex.max(pose.position[1]);
            if apex > -3.01 || pose.position[1] < -18.7 && pose.velocity[1] < 0.0 && row >= end_rows
            {
                return None;
            }
            if row >= end_rows {
                if let Some(slack) = pose.falling_catch(self.span) {
                    return Some((slack, 112 + row));
                }
            }
        }
        None
    }

    fn pull_up(&self, press: Pose) -> Option<PullUp> {
        let mut pose = self.after_charge(press)?;
        let mut row = 1;
        let mut best = None;
        let mut best_slack = 0.0;
        let mut examined = 0;
        for at in (84..=126).step_by(2).map(|x| x as f32) {
            while row < FLIGHT_ROWS && pose.position[0] < at {
                let next = pose.fly(false);
                if !self.step_to(&mut pose, next) {
                    return best;
                }
                row += 1;
            }
            if row == FLIGHT_ROWS {
                break;
            }
            if examined == row {
                continue;
            }
            examined = row;
            if let Some((slack, frames)) = self.finish(pose, row) {
                if slack > best_slack {
                    best = Some(PullUp { at, frames });
                    best_slack = slack;
                }
            }
        }
        best
    }
}

#[derive(Clone, Copy)]
struct PullUp {
    at: f32,
    frames: u16,
}

enum Phase {
    Descend {
        since: Option<u32>,
        press_height: f32,
    },
    Charge {
        since: u32,
        turn_x: f32,
    },
    Fly {
        turn_x: f32,
        pulling: bool,
    },
    End {
        since: u32,
    },
}

pub struct BarrelFlight {
    course: Course,
    expected_frames: u16,
    phase: Phase,
}

impl BarrelFlight {
    pub fn start(facts: &RouteFacts<'_>, sample: BarrelSample) -> Option<Self> {
        let course = Course::from_facts(facts)?;
        let pose = course.pose(sample);
        let [x, height] = pose.position;
        let outside = -x;
        if !sample.can_rise
            || sample.status != FALL
            || sample.facing != course.direction
            || pose.velocity[0].abs() > 0.3
            || !pose.velocity[0].is_finite()
            || !(-TERMINAL..=0.0).contains(&pose.velocity[1])
            || !region::contains(&RING, outside, height)
        {
            return None;
        }
        let depths = DEPTHS.get(outside.round() as usize - 8)?;
        for depth in depths.iter().copied().take_while(|&depth| depth != 0) {
            let press_height = -f32::from(depth);
            if press_height > height {
                continue;
            }
            let Some((press, falling)) = course.course_to_press(pose, press_height) else {
                continue;
            };
            if let Some(plan) = course.pull_up(press) {
                return Some(Self {
                    course,
                    expected_frames: falling + plan.frames,
                    phase: Phase::Descend {
                        since: None,
                        press_height,
                    },
                });
            }
        }
        None
    }

    pub fn exit_point(&self) -> Point {
        self.course.far
    }

    pub fn crossing_frames(&self) -> u16 {
        self.expected_frames
    }

    pub fn step(&mut self, sample: BarrelSample) -> Option<Correction> {
        let pose = self.course.pose(sample);
        if sample.facing != self.course.direction
            || !pose
                .position
                .into_iter()
                .chain(pose.velocity)
                .all(f32::is_finite)
        {
            return None;
        }
        let forward = [self.course.direction, 0.0];
        let up = [0.0, 1.0];
        let edit = match self.phase {
            Phase::Descend {
                ref mut since,
                press_height,
            } if sample.status == FALL && sample.can_rise => {
                if !(-16.0..=-8.0).contains(&pose.position[0])
                    || pose.velocity[0].abs() > 0.3
                    || !(-TERMINAL..=0.0).contains(&pose.velocity[1])
                {
                    return None;
                }
                if pose.position[1] <= press_height {
                    let plan = self.course.pull_up(pose)?;
                    self.phase = Phase::Charge {
                        since: sample.frame,
                        turn_x: plan.at,
                    };
                    pad(up, SPECIAL)
                } else {
                    let frame = *since.get_or_insert(sample.frame);
                    let down = -(sample.frame.saturating_sub(frame).min(7) as f32 + 1.0) / 8.0;
                    pad([0.0, down], 0)
                }
            }
            Phase::Charge { since, turn_x } => {
                let elapsed = sample.frame.checked_sub(since)?;
                match (elapsed, sample.status) {
                    (0, FALL) | (1..=7, START) | (8..=67, CHARGE) => {
                        pad(if elapsed < ARM { up } else { forward }, SPECIAL)
                    }
                    (HOLD, FLIGHT) => {
                        self.phase = Phase::Fly {
                            turn_x,
                            pulling: false,
                        };
                        pad(forward, 0)
                    }
                    _ => return None,
                }
            }
            Phase::Fly {
                turn_x,
                ref mut pulling,
            } if sample.status == FLIGHT => {
                *pulling |= pose.position[0] >= turn_x;
                pad(if *pulling { up } else { forward }, 0)
            }
            Phase::Fly { pulling: true, .. } if sample.status == END => {
                // The first END sample already includes the final powered movement.
                self.phase = Phase::End {
                    since: sample.frame,
                };
                pad(
                    [
                        self.course.direction * pose.steer(self.course.span, END_ROWS - 1),
                        0.0,
                    ],
                    0,
                )
            }
            Phase::End { since } if sample.status == END => {
                let elapsed = sample.frame.checked_sub(since)?;
                let remain = u32::from(END_ROWS - 1).checked_sub(elapsed)? as u8;
                pad(
                    [
                        self.course.direction * pose.steer(self.course.span, remain),
                        0.0,
                    ],
                    0,
                )
            }
            Phase::End { .. } if sample.status == SPECIAL_FALL => pad(
                [self.course.direction * pose.steer(self.course.span, 0), 0.0],
                0,
            ),
            _ => return None,
        };
        Some(edit)
    }
}

#[cfg(test)]
lib_unit_tests! {
    #[path = "diddy_tests.rs"]
    mod tests;
}
