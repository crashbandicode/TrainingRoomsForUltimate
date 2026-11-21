//! Bounded port of the measured S-CY flight in g4_sim.py. Coordinates are
//! relative to the verified common outline. Only the measured release catch counts.

const NEAR: f64 = -79.986;
const FAR: f64 = 79.989;
const BLAST: f64 = -140.0;
const GRAVITY: f64 = 0.08;
const TERMINAL: f64 = 1.73;
const AIR: f64 = 0.987;
const ACCEL: f64 = 0.03;
const JUMP_SPEED: f64 = 2.294780503601998; // sqrt(2 * gravity * 34.07) - gravity/2
const OUTLINE: [[f64; 2]; 6] = [
    [NEAR, -8.319],
    [-64.395, -25.111],
    [-41.396, -40.099],
    [41.416, -40.118],
    [64.387, -25.103],
    [FAR, -8.319],
];

pub(super) const JUMP_INSIDE: f64 = 37.1;
pub(super) const JUMP_HEIGHT: f64 = -128.0;
pub(super) const CYPHER_INSIDE: f64 = 62.8;
pub(super) const CYPHER_HEIGHT: std::ops::RangeInclusive<f64> = -108.0..=-77.0;

fn drift(vx: f64, stick: i8, acceleration: f64, cap: f64) -> f64 {
    match stick {
        1 if vx > cap => (vx - 0.005).max(cap),
        1 => (vx + acceleration).min(cap),
        -1 if vx < -cap => (vx + 0.005).min(-cap),
        -1 => (vx - acceleration).max(-cap),
        _ if vx > 0.0 => (vx - 0.005).max(0.0),
        _ => (vx + 0.005).min(0.0),
    }
}

fn fall(vy: f64) -> f64 {
    (vy - GRAVITY).max(-TERMINAL)
}

fn pressed_frames(mut x: f64, mut y: f64, mut vy: f64) -> Option<(u16, u16)> {
    let mut vx = 0.0;
    let mut jump = None;
    for frame in 0..400 {
        if jump.is_none() && (x >= NEAR + JUMP_INSIDE || y <= JUMP_HEIGHT) {
            jump = Some(frame);
        }
        if let Some(press) = jump.filter(|&press| frame > press && x >= NEAR + CYPHER_INSIDE) {
            return CYPHER_HEIGHT.contains(&y).then_some((press, frame));
        }
        if jump.is_some_and(|press| frame == press + 1) {
            vy = JUMP_SPEED;
            vx = drift(0.9, 1, ACCEL, AIR);
        } else {
            vy = fall(vy);
            vx = drift(vx, 1, ACCEL, AIR);
        }
        x += vx;
        y += vy;
        if y < BLAST {
            return None;
        }
    }
    None
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Travel {
    Fall,
    Jump,
    Start,
    Ride,
    Release,
}

fn bottom_at(x: f64) -> Option<f64> {
    if x <= NEAR || x >= FAR {
        return None;
    }
    OUTLINE.windows(2).find_map(|line| {
        let ([x0, y0], [x1, y1]) = (line[0], line[1]);
        (x0 <= x && x <= x1).then(|| y0 + (y1 - y0) * (x - x0) / (x1 - x0))
    })
}

fn reaches_far_ledge(mut x: f64, mut y: f64, mut vy: f64) -> Option<u16> {
    let (jump, cypher) = pressed_frames(x, y, vy)?;
    let mut vx = 0.0;
    let mut travel = Travel::Fall;
    let mut age = 0;
    for frame in 0..420 {
        if frame == cypher && matches!(travel, Travel::Fall | Travel::Jump) {
            travel = Travel::Start;
            age = 0;
        } else if frame == jump && travel == Travel::Fall {
            travel = Travel::Jump;
            age = 0;
        }
        let stick = if frame < cypher + 95 { 1 } else { -1 };
        match travel {
            Travel::Fall => {
                vy = fall(vy);
                vx = drift(vx, stick, ACCEL, AIR);
            }
            Travel::Jump => {
                vy = match age {
                    0 => fall(vy),
                    1 => JUMP_SPEED,
                    _ => vy - GRAVITY,
                };
                vx = drift(if age == 1 { 0.9 } else { vx }, stick, ACCEL, AIR);
                if age >= 2 && vy <= -TERMINAL {
                    travel = Travel::Fall;
                }
            }
            Travel::Start => {
                vy = fall(vy);
                vx = drift(vx, if age < 3 { 0 } else { stick }, ACCEL, AIR);
                if age == 6 {
                    travel = Travel::Ride;
                    age = 0;
                    vy = vy.max(-1.0);
                } else {
                    age += 1;
                }
            }
            Travel::Ride => {
                vy = if vy < 1.25 {
                    (vy + 0.05).min(1.25)
                } else {
                    (vy - 0.005).max(1.25)
                };
                vx = drift(vx, stick, 0.011, 1.0);
                if age == 87 {
                    travel = Travel::Release;
                    age = 0;
                } else {
                    age += 1;
                }
            }
            Travel::Release => {
                vy -= GRAVITY;
                vx = drift(vx.min(AIR), stick, ACCEL, AIR);
                if age == 22 {
                    return None;
                }
                age += 1;
            }
        }
        if travel == Travel::Jump {
            age += 1;
        }
        x += vx;
        y += vy;
        let suspended = matches!(travel, Travel::Ride | Travel::Release);
        if let Some(bottom) = bottom_at(x) {
            if y > 0.0 {
                return None;
            }
            let clearance = match (suspended, x.abs() <= 42.45) {
                (true, true) => 29.6,
                (false, true) => 17.0,
                (_, false) => 17.1,
            };
            if y + clearance > bottom {
                y = bottom - clearance;
                if !suspended {
                    vy = vy.min(0.0);
                }
            }
        }
        if y < BLAST || ((travel == Travel::Ride || vy < 0.0)
            && (NEAR - 8.6..=NEAR).contains(&x) && (-22.7..=-11.5).contains(&y))
        {
            return None;
        }
        if travel == Travel::Release
            && (FAR..=FAR + 15.0).contains(&x)
            && (-24.0..=-11.5).contains(&y)
        {
            return Some(frame + 1);
        }
    }
    None
}

/// Validates the live start and its four 2-unit neighbours against the same
/// route. The extra eight frames cover the measured model-to-catch time lag.
pub(super) fn guarded_time(outside: f64, height: f64, vy: f64) -> Option<u16> {
    if !((0.0..120.0).contains(&outside)
        && outside > 0.0
        && (BLAST..=180.0).contains(&height)
        && (f64::from(-1.73_f32)..=0.0).contains(&vy))
    {
        return None;
    }
    let x = NEAR - outside;
    let mut frames = 0;
    for (dx, dy) in [(0.0, 0.0), (-2.0, 0.0), (2.0, 0.0), (0.0, -2.0), (0.0, 2.0)] {
        frames = frames.max(reaches_far_ledge(x + dx, height + dy, vy)?);
    }
    Some(frames + 8)
}
