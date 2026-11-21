//! The fixed-step HR-1 model from fusion-g4's hero_hr1.py and Kaswoosh trace.
pub(super) const LEFT: f64 = -79.986;
const RIGHT: f64 = 79.989;
const LIP_Y: f64 = -8.319;
const GRAVITY: f64 = 0.094;
const FALL_SPEED: f64 = 1.57;
const AIR_SPEED: f64 = 1.01;
const ACCEL: f64 = 0.055;
const BRAKE: f64 = 0.008;
const JUMP_SPEED: f64 = 2.367125100321025;
const FLOOR: f64 = -140.0;
const DODGE_X: std::ops::RangeInclusive<f64> = -100.0..=-94.3;
const DODGE_Y: std::ops::RangeInclusive<f64> = -26.5..=-17.5;
const DODGE_MOTION: [f64; 11] = [
    -3.500, -1.685, -1.063, -0.752, 3.786, 3.371, 2.957, 2.543, 2.129, 1.714, 1.300,
];
pub(super) const DODGE_FRAMES: u32 = 83;
const FALL_CLEARANCE: f64 = 12.3;
const KASWOOSH_CLEARANCE: [f64; 34] = [
    17.76, 16.47, 16.19, 15.22, 13.97, 14.05, 14.31, 14.41, 14.36, 14.29, 14.29, 14.23, 13.78,
    12.93, 11.81, 12.13, 12.88, 13.90, 14.82, 15.66, 16.43, 16.73, 16.75, 16.57, 16.24, 15.87,
    15.74, 15.55, 15.43, 15.32, 15.21, 15.10, 15.00, 14.89,
];
const SLOPE_POINTS: [[f64; 2]; 6] = [
    [LEFT, LIP_Y],
    [-64.395, -25.111],
    [-41.396, -40.099],
    [41.416, -40.118],
    [64.387, -25.103],
    [RIGHT, LIP_Y],
];

#[derive(Clone, Copy)]
pub(super) struct Sample {
    pub position: [f64; 2],
    pub velocity: [f64; 2],
}

#[derive(Clone, Copy)]
pub(super) struct Path {
    pub wait: u32,
    pub shield: u32,
    pub cast: u32,
    pub jump_at: Sample,
    pub cast_at: Sample,
    pub shield_at: Sample,
    pub arrival: u16,
}

#[derive(Clone, Copy)]
struct Timing {
    wait: usize,
    shield: usize,
    pad: Sample,
    error: f64,
}

fn drift(vx: f64) -> f64 {
    if vx <= AIR_SPEED {
        (vx + ACCEL).min(AIR_SPEED)
    } else {
        (vx - BRAKE).max(AIR_SPEED)
    }
}

fn fall(vy: f64) -> f64 {
    (vy - GRAVITY).max(-FALL_SPEED)
}

fn slope(x: f64) -> Option<(f64, f64)> {
    if !(LEFT < x && x < RIGHT) {
        return None;
    }
    SLOPE_POINTS.windows(2).find_map(|pair| {
        let [start, end] = [pair[0], pair[1]];
        let rate = (end[1] - start[1]) / (end[0] - start[0]);
        (x <= end[0]).then(|| (start[1] + (x - start[0]) * rate, rate))
    })
}

#[derive(Clone, Copy)]
struct Flight {
    sample: Sample,
    facing: f64,
    held: bool,
    ceiling: bool,
}

impl Flight {
    fn new(sample: Sample, facing: f64) -> Self {
        Self {
            sample,
            facing,
            held: false,
            ceiling: false,
        }
    }

    fn live(self) -> bool {
        let [x, y] = self.sample.position;
        y > FLOOR && x.abs() < 240.0
    }

    fn step(&mut self, dx: f64, dy: f64, head: f64, rising: bool) -> Option<()> {
        let [x, y] = self.sample.position;
        let mut next_x = x + dx;
        let mut next_y = y + dy;
        if next_y < 0.0 {
            if x <= LEFT && LEFT < next_x && next_y + head > LIP_Y {
                next_x = LEFT - 0.01;
            }
            if let Some((height, incline)) = slope(next_x) {
                let on_far_slope = rising && incline > 1e-6;
                let head = if on_far_slope { 9.6 } else { head };
                if next_y + head > height {
                    self.ceiling = true;
                    let height = if on_far_slope {
                        next_x = x + dx.max(1.13);
                        slope(next_x).map_or(LIP_Y, |(height, _)| height)
                    } else {
                        height
                    };
                    next_y = next_y.min(height - head);
                }
            }
        } else if (LEFT..=RIGHT).contains(&next_x) {
            return None;
        }
        self.sample.position = [next_x, next_y];
        self.live().then_some(())
    }

    fn near_catch(self, lag: bool) -> bool {
        let [x, y] = self.sample.position;
        let reach = if self.facing > 0.0 { 16.1 } else { 9.0 };
        let (right, depth) = if lag {
            (LEFT + 9.0, 26.4)
        } else {
            (LEFT, 24.0)
        };
        self.sample.velocity[1] < 0.0
            && (LEFT - reach..=right).contains(&x)
            && (10.5..=depth).contains(&-y)
    }

    fn slide_lag(&mut self, frame: u32) -> Option<()> {
        let [x, y] = self.sample.position;
        let [vx, vy] = self.sample.velocity;
        let gap = if frame <= 23 {
            12.44 - 0.052 * (f64::from(frame) - 18.0)
        } else {
            12.11 - 0.07 * (f64::from(frame) - 24.0)
        } - if self.facing > 0.0 { 0.0 } else { 0.45 };
        if let Some((height, incline)) = slope(x + vx)
            .filter(|&(height, incline)| incline < 0.0 && y + vy < 0.0 && height - (y + vy) < gap)
        {
            let distance = 0.729 * vy.abs();
            self.sample.position = if self.held {
                [x + distance, y - 1.0015 * distance]
            } else {
                [x.max(x + vx + (y + vy + gap - height) / incline), y + vy]
            };
            self.held = true;
            self.ceiling = true;
            return self.live().then_some(());
        }
        self.held = false;
        self.step(vx, vy, 0.0, false)
    }

    fn dodge(mut self) -> Option<Self> {
        let [vx, vy] = self.sample.velocity;
        self.step(drift(vx), fall(vy), FALL_CLEARANCE, false)?;
        for dx in DODGE_MOTION {
            self.step(dx, 0.0, FALL_CLEARANCE, false)?;
        }
        self.sample.velocity = [1.3, 0.0];
        for age in 12..DODGE_FRAMES {
            let [vx, vy] = self.sample.velocity;
            self.sample.velocity = [drift(vx), fall(vy)];
            self.slide_lag(age)?;
            if age >= 24 && self.near_catch(true) {
                return None;
            }
        }
        Some(self)
    }

    fn jump(mut self, frames: u32) -> Option<Self> {
        let [vx, vy] = self.sample.velocity;
        self.sample.velocity = [drift(vx), fall(vy)];
        self.step(
            self.sample.velocity[0],
            self.sample.velocity[1],
            FALL_CLEARANCE,
            false,
        )?;
        for age in 1..frames {
            let [vx, vy] = self.sample.velocity;
            let vy = if age == 1 { JUMP_SPEED } else { fall(vy) };
            self.sample.velocity = [drift(vx), vy];
            self.step(
                self.sample.velocity[0],
                vy,
                if age <= 26 { FALL_CLEARANCE } else { 0.0 },
                false,
            )?;
        }
        Some(self)
    }

    fn cast(mut self) -> Option<(u32, bool)> {
        let [vx, vy] = self.sample.velocity;
        self.sample.velocity = [(vx - BRAKE).max(0.0), fall(vy)];
        self.step(
            self.sample.velocity[0],
            self.sample.velocity[1],
            FALL_CLEARANCE,
            false,
        )?;
        let mut vx = self.sample.velocity[0] * 0.785;
        let mut vy = (0.9 * self.sample.velocity[1] - GRAVITY).max(-0.471);
        self.step(vx, vy, FALL_CLEARANCE, false)?;
        for frame in 0..40 {
            vx = (vx - 0.015).max(0.0);
            vy = if frame < 30 {
                (vy - GRAVITY).max(-0.471)
            } else {
                vy - GRAVITY
            };
            self.step(vx, vy, FALL_CLEARANCE, false)?;
        }
        if self.sample.position[0] < 44.5 {
            return None;
        }
        for (frame, vy) in KASWOOSH_RISE.into_iter().enumerate() {
            vx = if frame == 0 {
                (vx - 0.044).max(0.0)
            } else {
                (vx + 0.05).min(1.0)
            };
            let head = KASWOOSH_CLEARANCE[(frame + 11).clamp(20, 53) - 20];
            self.step(vx, vy, head, true)?;
            let [x, y] = self.sample.position;
            if (1.0..=8.0).contains(&(RIGHT - x)) && (11.5..=25.4).contains(&-y) {
                return Some((frame as u32 + 43, self.ceiling));
            }
            if x >= RIGHT {
                return None;
            }
        }
        None
    }
}

pub(super) fn local(ledge: super::Point, position: super::Point) -> [f64; 2] {
    [
        LEFT + (f64::from(position[0]) - f64::from(ledge[0])),
        f64::from(position[1]) - f64::from(ledge[1]),
    ]
}

pub(super) fn matches_position(live: [f64; 2], expected: Sample) -> bool {
    live.into_iter()
        .zip(expected.position)
        .all(|(a, b)| (a - b).abs() <= 1.0)
}

/// Advance gravity and drift separately when ranking the first 16 shield
/// poses. Time spent neutral does not change the x or y prefixes of drift.
pub(super) fn plan(start: Sample, facing: f64) -> Option<Path> {
    let mut travel = [(0.0, 0.0); 110];
    for i in 1..travel.len() {
        let (dx, vx) = travel[i - 1];
        let vx = drift(vx);
        travel[i] = (dx + vx, vx);
    }
    let [start_x, mut y] = start.position;
    let mut speed_y = start.velocity[1];
    let mut best = [None::<Timing>; 16];
    for shield in 1..110 {
        speed_y = fall(speed_y);
        y += speed_y;
        if y < *DODGE_Y.start() {
            break;
        }
        if !DODGE_Y.contains(&y) {
            continue;
        }
        for wait in 0..shield.min(69) + 1 {
            let (dx, vx) = travel[shield - wait];
            let x = start_x + dx;
            if !DODGE_X.contains(&x) {
                continue;
            }
            let center = -24.1 - 1.55 * (x + 96.44);
            let timing = Timing {
                wait,
                shield,
                pad: Sample {
                    position: [x, y],
                    velocity: [vx, speed_y],
                },
                error: (y - center).abs(),
            };
            let rank = |t: Timing| (t.error, t.wait, t.shield);
            if let Some(i) = best
                .iter()
                .position(|slot| slot.is_none_or(|prior| rank(timing) < rank(prior)))
            {
                best[i..].rotate_right(1);
                best[i] = Some(timing);
            }
        }
    }
    best.into_iter().flatten().find_map(|t| {
        let jump = Flight::new(t.pad, facing).dodge()?;
        let jump_frames = ((45.5 - jump.sample.position[0] - 20.95) / AIR_SPEED).round() as u32;
        if jump.ceiling || !(30..=37).contains(&jump_frames) {
            return None;
        }
        let cast = jump.jump(jump_frames)?;
        let (flight_frames, ceiling) = cast.cast()?;
        if ceiling {
            return None;
        }
        for dy in [-1.0, 1.0] {
            let mut adjacent = t.pad;
            adjacent.position[1] += dy;
            Flight::new(adjacent, facing)
                .dodge()?
                .jump(jump_frames)?
                .cast()?;
        }
        let shield = t.shield as u32;
        let cast_frame = shield + DODGE_FRAMES + jump_frames;
        Some(Path {
            wait: t.wait as u32,
            shield,
            cast: cast_frame,
            jump_at: jump.sample,
            cast_at: cast.sample,
            shield_at: t.pad,
            arrival: (cast_frame + flight_frames) as u16,
        })
    })
}

/// Re-evaluate the committed flight against the latest point on its route.
pub(super) fn shield_reaches(sample: Sample, facing: f64, jump_frames: u32) -> bool {
    DODGE_X.contains(&sample.position[0])
        && DODGE_Y.contains(&sample.position[1])
        && Flight::new(sample, facing)
            .dodge()
            .and_then(|f| f.jump(jump_frames))
            .and_then(Flight::cast)
            .is_some()
}

pub(super) fn jump_reaches(sample: Sample, facing: f64, jump_frames: u32) -> bool {
    Flight::new(sample, facing)
        .jump(jump_frames)
        .and_then(Flight::cast)
        .is_some()
}

pub(super) fn cast_reaches(sample: Sample, facing: f64) -> bool {
    Flight::new(sample, facing).cast().is_some()
}

// Vertical frame displacements recorded by Group 4 in g4_data.json.
const KASWOOSH_RISE: [f64; 98] = [
    4.3997, 4.2997, 4.1997, 4.0997, 3.9997, 3.8997, 3.7997, 3.6997, 3.5997, 3.4997, 3.3997, 3.2997,
    3.1997, 3.0997, 2.9997, 2.8997, 2.7997, 2.6997, 2.5997, 2.4997, 2.3997, 2.2997, 2.1997, 2.0997,
    1.9997, 1.8997, 1.7997, 1.6997, 1.5997, 1.4997, 1.3997, 1.2997, 1.1997, 1.0997, 0.9997, 0.8997,
    0.7997, 0.6997, 0.5997, 0.4997, 0.3997, 0.2997, 0.1997, 0.0997, -0.0003, -0.0905, -0.1808,
    -0.271, -0.3612, -0.4515, -0.5417, -0.632, -0.7222, -0.8124, -0.9027, -0.9929, -1.0832,
    -1.1734, -1.2636, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502,
    -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502,
    -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502,
    -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502, -1.3502,
    -1.3502,
];
