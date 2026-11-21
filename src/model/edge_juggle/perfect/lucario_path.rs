//! Lucario's measured normal-ring flight, after G2's g2_lucario.py model.
//! Search is offline. A live release checks the baked schedules on its own pose.

pub(super) const NEAR: f64 = -79.986;
pub(super) const FAR: f64 = 79.989;
const BEND: f64 = 64.387;
const INNER: f64 = 41.416;
const LAND: [[f64; 2]; 6] = [
    [NEAR, -8.319],
    [-64.395, -25.111],
    [-41.396, -40.099],
    [INNER, -40.118],
    [BEND, -25.103],
    [FAR, -8.319],
];
const GRAVITY: f64 = 0.084;
const TERMINAL: f64 = 1.68;
const AIR_SPEED: f64 = 1.281;
const ACCEL: f64 = 0.09;
const AIR_BRAKE: f64 = 0.004;
const JUMP_SPEED: f64 = 2.472;
const ENTRY_CLEARANCE: f64 = 11.5;
const RUSH_START: u16 = 29;
const RUSH_STEPS: u16 = 16;
// Direction of the realized (60, 104)/120 stick, rotated by measured 10° steps.
const RUSH_DIRECTIONS: [[f64; 2]; 3] = [
    [0.49972245348957717, 0.8661855860486004],
    [0.6425420950895198, 0.7662503873003723],
    [0.7658384202721557, 0.6430330582754272],
];

type Point = [f64; 2];

#[derive(Clone, Copy, Debug)]
pub(super) struct Path {
    pub holds: u16,
    pub jump: u16,
    pub press: u16,
    pub arrival: u16,
    pub jump_at: Point,
    pub press_at: Point,
}

fn underside(x: f64) -> Option<f64> {
    LAND.windows(2).find_map(|segment| {
        let ([ax, ay], [bx, by]) = (segment[0], segment[1]);
        (ax..=bx)
            .contains(&x)
            .then(|| ay + (by - ay) * (x - ax) / (bx - ax))
    })
}

fn clear([x, y]: Point) -> bool {
    y >= 0.5
        || [-1.5, 0.0, 1.5]
            .into_iter()
            .all(|dx| underside(x + dx).is_none_or(|bottom| y + ENTRY_CLEARANCE <= bottom))
}

fn slide_x(y: f64) -> f64 {
    let ([ax, ay], [bx, by]) = if y <= LAND[4][1] {
        (LAND[3], LAND[4])
    } else {
        (LAND[4], LAND[5])
    };
    ax + (y - ay) * (bx - ax) / (by - ay)
}

fn rush_start_speed(damage: f64) -> f64 {
    6.05 + 3.0 * 0.67 * (damage - 65.0) / 125.0
}

fn rush(mut position: Point, damage: f64) -> Option<u16> {
    let mut turn = 0;
    let mut touching = false;
    let mut taken = None;
    for step in 1..=RUSH_STEPS {
        let speed = rush_start_speed(damage) - 0.2404 * f64::from(step.saturating_sub(6));
        let [dx, dy] = RUSH_DIRECTIONS[turn];
        let [x, y] = position;
        let next = [x + dx * speed, y + dy * speed];
        let bottom = underside(next[0]);
        if bottom.is_some_and(|floor| next[1] > floor - 9.5) {
            // The retained route must first contact the steep side. The flat
            // floor rejects this angle, and the shallow-slope branch is unproved.
            if !touching {
                let projected = slide_x(next[1] + 9.5);
                let pushed = if projected < FAR { projected } else { next[0] };
                if next[0] <= INNER || pushed < BEND + 0.6 || turn != 0 {
                    return None;
                }
                position = [pushed, next[1]];
                turn += 1;
                touching = true;
            } else {
                let shifted = x + speed * 0.5;
                position = [
                    shifted,
                    underside(shifted).map_or(y + dy * speed, |v| v - 9.55),
                ];
                turn += 1;
                touching = false;
            }
        } else {
            position = next;
            touching = false;
        }
        let [x, y] = position;
        if let Some(advance) = taken {
            return ((FAR + 0.5..=FAR + 8.0).contains(&x) && y <= -13.0).then_some(advance);
        }
        if (FAR - 1.6..FAR).contains(&x) && (-19.9..=-13.0).contains(&y) {
            return None;
        }
        if (FAR + 0.5..=FAR + 8.0).contains(&x) && (-18.6..=-13.0).contains(&y) {
            if step > 14 || turn == 0 || y < -17.6 {
                return None;
            }
            taken = Some(step);
        }
    }
    None
}

pub(super) fn flight(
    start: Point,
    velocity: Point,
    damage: f64,
    schedule: [u8; 3],
) -> Option<Path> {
    let [holds, drift, ascent] = schedule.map(u16::from);
    let mut pose = start;
    let [mut vx, mut vy] = velocity;
    let jump = holds + drift;
    let press = jump + ascent;
    let jump_speeds = [0.94, 1.03, 1.12, 1.21];
    for frame in 0..=jump {
        vx = if frame >= holds {
            (vx + ACCEL).min(AIR_SPEED)
        } else {
            vx.signum() * (vx.abs() - AIR_BRAKE).max(0.0)
        };
        vy = (vy - GRAVITY).max(-TERMINAL);
        pose[0] += vx;
        pose[1] += vy;
        if (NEAR - 15.5..=NEAR + 0.5).contains(&pose[0]) && (-25.5..=-11.5).contains(&pose[1])
            || !clear(pose)
        {
            return None;
        }
    }
    let jump_at = [pose[0] - vx, pose[1] - vy];
    if pose[1] < -134.0 {
        return None;
    }
    for frame in 0..ascent {
        vx = jump_speeds
            .get(usize::from(frame))
            .copied()
            .unwrap_or(AIR_SPEED);
        if frame == ascent - 1 {
            vx -= AIR_BRAKE;
        }
        pose[0] += vx;
        pose[1] += (JUMP_SPEED - GRAVITY * f64::from(frame)).max(-TERMINAL);
        if !clear(pose) {
            return None;
        }
    }
    let press_at = [
        pose[0] - vx,
        pose[1] - (JUMP_SPEED - GRAVITY * f64::from(ascent - 1)).max(-TERMINAL),
    ];
    vx *= 0.8;
    for frame in 0..RUSH_START {
        vx = (vx - 0.02).max(0.0);
        pose[0] += vx;
        pose[1] -= 0.015 * f64::from(frame.saturating_sub(11));
        if !clear(pose) {
            return None;
        }
    }
    let arrival = press + 1 + RUSH_START + rush(pose, damage)?;
    Some(Path {
        holds,
        jump,
        press,
        arrival,
        jump_at,
        press_at,
    })
}
