//! Bounded solver for Greninja's two Hydro Pump bursts along the measured ring.
pub(super) const NEAR_X: f64 = -79.986;
const FAR_X: f64 = 79.989;
const GRAVITY: f64 = 0.18;
const FALL_SPEED: f64 = 1.85;
const AIR_SPEED: f64 = 1.239;
const B1_STEP: f64 = 3.70 * std::f64::consts::FRAC_1_SQRT_2;
const B2_STEP: f64 = 3.33;
const FLAT: f64 = -40.118;
const CORNER: f64 = 41.416;
const KNEE: [f64; 2] = [64.387, -25.103];
const T33: f64 = (KNEE[1] - FLAT) / (KNEE[0] - CORNER);
const T47: f64 = (-8.319 - KNEE[1]) / (FAR_X - KNEE[0]);
const RISE33: f64 = 1.821_957_849_727_382;
const RISE47: f64 = 2.438_980_523_537_131;
const START_ROWS: u16 = 16;
const CATCH_DEPTH: f64 = -23.65;
const B2_GAPS: [f64; 5] = [12.57, 10.45, 10.40, 10.48, 10.58];
const B2_GAP: f64 = 10.60;
const END: [[f64; 2]; 9] = [
    [1.380, 1.644],
    [1.250, 1.518],
    [1.106, 1.402],
    [0.969, 1.281],
    [0.834, 1.154],
    [0.701, 1.023],
    [0.576, 0.887],
    [0.465, 0.743],
    [0.378, 0.590],
];
const GRAZE: [f64; 3] = [0.0, -0.3, 0.3];
const PRE_ROLL: [f64; 3] = [0.0, 0.18, 0.54];
const RING: [[f64; 2]; 6] = [
    [NEAR_X, -8.319],
    [-64.395, -25.111],
    [-41.396, -40.099],
    [CORNER, FLAT],
    KNEE,
    [FAR_X, -8.319],
];
pub(super) const FLOOR: f64 = -134.0;

#[derive(Clone, Copy, Debug)]
pub(super) struct Timing {
    pub down: u8,
    ease: u8,
    drift: u8,
    jump_rows: u8,
}

impl From<[u8; 4]> for Timing {
    fn from([down, ease, drift, jump_rows]: [u8; 4]) -> Self {
        Self {
            down,
            ease,
            drift,
            jump_rows,
        }
    }
}

impl Timing {
    pub fn eased(self) -> u32 {
        u32::from(self.down) + u32::from(self.ease)
    }

    pub fn jump_input(self) -> u32 {
        self.eased() + u32::from(self.drift)
    }

    pub fn up_input(self) -> u32 {
        self.jump_input() + u32::from(self.jump_rows)
    }
}

#[derive(Clone, Copy)]
pub(super) struct Fall {
    pub position: [f64; 2],
    velocity: [f64; 2],
}

impl Fall {
    pub fn new(position: [f64; 2], vy: f64) -> Self {
        Self {
            position,
            velocity: [0.0, vy],
        }
    }

    pub fn advance(&mut self, timing: Timing, age: u32) {
        let [vx, vy] = &mut self.velocity;
        if age >= timing.eased() {
            *vx = (*vx + 0.08).min(AIR_SPEED);
        } else if age >= u32::from(timing.down) {
            *vx = (*vx + 0.045).min(0.6194);
        }
        *vy = (*vy - GRAVITY).max(-FALL_SPEED);
        self.position[0] += *vx;
        self.position[1] += *vy;
    }
}

pub(super) struct Flight {
    pub timing: Timing,
    pub press_at: [f64; 2],
    pub arrival: u16,
}

fn underside(x: f64) -> Option<f64> {
    RING.windows(2).find_map(|line| {
        let [[x0, y0], [x1, y1]] = [line[0], line[1]];
        (x0..=x1)
            .contains(&x)
            .then(|| y0 + (y1 - y0) * (x - x0) / (x1 - x0))
    })
}

fn clear([x, y]: [f64; 2]) -> bool {
    y >= FLOOR
        && (y >= 0.5
            || [-1.5, 0.0, 1.5]
                .into_iter()
                .all(|dx| underside(x + dx).is_none_or(|roof| y + 13.0 <= roof)))
}

fn jump_row(timing: Timing, step: u8) -> [f64; 2] {
    let mut x = [0.98, 1.06, 1.14, 1.22]
        .get(usize::from(step))
        .copied()
        .unwrap_or(AIR_SPEED);
    if step >= timing.jump_rows - 2 {
        x -= 0.008;
    }
    [x, (3.979 - GRAVITY * f64::from(step)).max(-FALL_SPEED)]
}

fn advance_air(position: [f64; 2], [x, y]: [f64; 2]) -> [f64; 2] {
    [position[0] + x, position[1] + y]
}

fn caught(x: f64, y: f64) -> bool {
    y >= CATCH_DEPTH && x < FAR_X && FAR_X - x <= 10.35
}

fn x_on_33(y: f64, gap: f64) -> f64 {
    CORNER + (y + gap - FLAT) / T33
}

fn x_on_47(y: f64, bias: f64) -> f64 {
    KNEE[0] + (y + 11.49 + bias - KNEE[1]) / T47
}

fn finish([mut x, mut y]: [f64; 2], bias: f64) -> Option<u16> {
    for _ in 0..11 {
        x += B1_STEP;
        y += B1_STEP;
        if underside(x).is_some_and(|roof| y > roof - 15.4) {
            return None;
        }
    }
    // Greninja must finish the first burst with room for the inter-burst step.
    x += 0.5 * B1_STEP - 0.03;
    y += 0.7 * B1_STEP - GRAVITY;
    if !(41.54..FAR_X - 8.0).contains(&x) {
        return None;
    }
    let roof = if x - 3.0 > CORNER {
        underside(x - 3.0)?
    } else {
        FLAT
    };
    if y > roof - 15.0 {
        return None;
    }
    y = y.min(roof - 14.96);
    let mut riding = false;
    for step in 0..11 {
        let gap = B2_GAPS.get(step).copied().unwrap_or(B2_GAP) + bias;
        let prev = step
            .checked_sub(1)
            .and_then(|i| B2_GAPS.get(i))
            .copied()
            .unwrap_or(B2_GAP)
            + bias;
        if !riding {
            y += B2_STEP;
            let next = x_on_33(y, gap).min(x_on_47(y, bias));
            if x < next {
                x = next;
                riding = step > 0;
            }
        } else if x_on_33(y, prev) <= x_on_47(y, bias) {
            y += RISE33 - 0.5 * (gap - prev);
            x = x_on_33(y, gap).min(x_on_47(y, bias));
        } else {
            y += RISE47;
            x = x_on_47(y, bias);
        }
        if caught(x, y) {
            return Some(0);
        }
    }
    if !riding || x_on_33(y, B2_GAP + bias) <= x_on_47(y, bias) {
        return None;
    }
    END.into_iter()
        .position(|[dx, dy]| {
            x += dx;
            y += dy;
            caught(x, y)
        })
        .map(|index| index as u16 + 1)
}

fn press_at(mut fall: Fall, timing: Timing) -> Option<[f64; 2]> {
    for age in 0..=timing.jump_input() {
        fall.advance(timing, age);
        let [x, y] = fall.position;
        let inside_catch = (-0.5..=17.75).contains(&(NEAR_X - x)) && (7.5..=24.5).contains(&-y);
        if (inside_catch && (age >= timing.eased() || age < 8))
            || (x > NEAR_X - 4.0 && y > -12.0)
            || !clear(fall.position)
        {
            return None;
        }
    }
    let mut position = fall.position;
    for step in 0..timing.jump_rows - 1 {
        position = advance_air(position, jump_row(timing, step));
        if !clear(position) {
            return None;
        }
    }
    Some(position)
}

fn finish_from_press(position: [f64; 2], timing: Timing) -> Option<u16> {
    let next = advance_air(position, jump_row(timing, timing.jump_rows - 1));
    if !clear(next) {
        return None;
    }
    let mut worst = 0;
    for bias in GRAZE {
        worst = worst.max(finish(next, bias)?);
    }
    Some(worst)
}

pub(super) fn can_finish_from_press(position: [f64; 2], timing: Timing) -> bool {
    finish_from_press(position, timing).is_some()
}

pub(super) fn estimate(fall: Fall, timing: Timing) -> Option<Flight> {
    let mut worst = 0;
    let mut press = None;
    for amount in PRE_ROLL {
        let mut start = fall;
        start.position[1] -= amount;
        let point = press_at(start, timing)?;
        worst = worst.max(finish_from_press(point, timing)?);
        if press.is_none() {
            press = Some(point);
        }
    }
    // The status starts on the row after the first Up+B input.
    let arrival = timing.up_input() as u16 + 1 + START_ROWS + 11 + 1 + 11 + worst + 2;
    Some(Flight {
        timing,
        press_at: press?,
        arrival,
    })
}
