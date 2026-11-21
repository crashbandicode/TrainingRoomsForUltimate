//! Sheik's measured Vanish finish after a Bouncing Fish and double jump.

use super::{Correction, Outline, Point, RouteFacts, Segment, JUMP, SPECIAL};

#[derive(Debug)]
struct Vanish {
    stick: Point,
    catch: Point,
    steps: u8,
}

// Frame fits from g2/g2_sim.py, checked by the 990288d6 stock macro traces.
const GRAVITY: f32 = 0.15;
const BRAKE: f32 = 0.015;
const HEAD: f32 = 7.29;
const HALF: f32 = 1.5;
const VANISH_STEP: f32 = 2.5;
const VANISH_RANGE: u8 = 20;
const FALL: i32 = 0xe;
const JUMP_AERIAL: i32 = 0xc;
const FISH_START: i32 = 0x1df;
pub const FISH_END: i32 = 0x1e9;
const VANISH_START: i32 = 0x1de;
const VANISH_TRAVEL: i32 = 0x1e7;
const DURING_FISH: u32 = 50;
const AIR_SPEED: f32 = 1.155;
const AIR_ACCEL: f32 = 0.09;
const FALL_SPEED: f32 = 1.75;
const FISH_HEAD: f32 = 10.5;

fn vanish(
    outline: &[Segment],
    start: Point,
    speed: Point,
    ledge: Point,
    forward: f32,
    floor_limit: f32,
) -> Option<Vanish> {
    if !(matches!(forward, -1.0 | 1.0)
        && start
            .into_iter()
            .chain(speed)
            .chain(ledge)
            .all(f32::is_finite))
    {
        return None;
    }
    let mut from = start;
    let mut vx = speed[0] * forward;
    for step in 0..36 {
        vx = (vx - BRAKE).max(0.0);
        let dy = match step {
            0 => speed[1] - GRAVITY,
            1..=18 => 3.01 - (step - 1) as f32 * 0.19,
            _ => -0.25,
        };
        let next = [from[0] + vx * forward, from[1] + dy];
        if !super::aerial_clear(outline, [from, next], floor_limit, HEAD, HALF) {
            return None;
        }
        from = next;
    }

    let outside = (from[0] - ledge[0]) * forward;
    let depth = ledge[1] - from[1];
    let mut best = None;
    for degrees in (30..=90).step_by(3) {
        let (y, x) = (degrees as f32).to_radians().sin_cos();
        let raw = [(120.0 * x).round(), (120.0 * y).round()];
        let norm = raw[0].hypot(raw[1]);
        let [x, y] = raw.map(|axis| axis / norm);
        let to_catch = (depth - 19.0) / y;
        if !(0.0..=VANISH_STEP * f32::from(VANISH_RANGE - 2)).contains(&to_catch) {
            continue;
        }
        let to_high = ((depth - 14.0) / y).min(VANISH_STEP * f32::from(VANISH_RANGE));
        let slack = (outside + to_catch * x - 1.0).min(8.6 - outside - to_high * x);
        if slack < 1.5 || best.as_ref().is_some_and(|(score, _)| slack <= *score) {
            continue;
        }

        let mut previous = from;
        for step in 1..=VANISH_RANGE {
            let distance = VANISH_STEP * f32::from(step);
            let next = [from[0] + x * forward * distance, from[1] + y * distance];
            if !super::aerial_clear(outline, [previous, next], floor_limit, HEAD, HALF) {
                break;
            }
            if (1.0..=8.6).contains(&((next[0] - ledge[0]) * forward))
                && (11.7..=19.0).contains(&(ledge[1] - next[1]))
            {
                best = Some((
                    slack,
                    Vanish {
                        stick: [raw[0] * forward / 120.0, raw[1] / 120.0],
                        catch: next,
                        steps: step,
                    },
                ));
                break;
            }
            previous = next;
        }
    }
    best.map(|(_, route)| route)
}

#[derive(Clone, Copy)]
struct FishSchedule {
    quiet: u32,
    drift: u32,
    rise: u32,
}

#[derive(Clone, Copy)]
pub struct SheikBeat {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub jumps: u8,
    pub can_jump: bool,
    pub down: bool,
    pub up: bool,
}

pub struct SheikFlight {
    outline: Outline,
    ledge: Point,
    near: Point,
    inward: f32,
    origin: Option<u32>,
    schedule: FishSchedule,
    floor_limit: f32,
    estimate: u16,
    target: Point,
    aim: Option<Point>,
}

// Group 2's S-X ring mask and center schedules from g2_sim.py.
// Each set cell has a schedule at release and at Sheik's terminal fall speed.
const S_X_RING: [u32; 44] = [
    0x00000000, 0x0000ffc0, 0x00007fc0, 0x00003fc0, 0x00003fc0, 0x00001fc0, 0x00001fc0, 0x00000fc0,
    0x000007c0, 0x000003c0, 0x000001c0, 0x000001c0, 0x000001c0, 0x000000c0, 0x000000c0, 0x00000000,
    0x00000000, 0x00000010, 0x0000000c, 0x00000004, 0x00000004, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000,
];
const S_X_TIMING: [[u8; 3]; 164] = [
    [24, 40, 24],
    [20, 42, 24],
    [24, 40, 24],
    [16, 46, 22],
    [20, 46, 24],
    [12, 48, 24],
    [16, 52, 24],
    [12, 48, 24],
    [12, 56, 20],
    [8, 54, 24],
    [8, 58, 24],
    [4, 58, 22],
    [4, 62, 22],
    [0, 60, 24],
    [0, 64, 24],
    [0, 64, 22],
    [0, 68, 22],
    [0, 62, 20],
    [0, 72, 24],
    [0, 62, 24],
    [24, 42, 20],
    [20, 40, 24],
    [20, 44, 24],
    [16, 44, 22],
    [16, 48, 22],
    [12, 46, 24],
    [12, 50, 24],
    [8, 54, 20],
    [12, 54, 22],
    [4, 56, 22],
    [8, 56, 24],
    [0, 58, 24],
    [4, 60, 22],
    [0, 62, 24],
    [0, 66, 24],
    [0, 62, 24],
    [0, 66, 24],
    [0, 60, 24],
    [20, 40, 22],
    [16, 42, 22],
    [20, 42, 24],
    [12, 44, 24],
    [16, 46, 22],
    [8, 48, 22],
    [12, 52, 24],
    [4, 54, 24],
    [8, 56, 22],
    [4, 54, 24],
    [4, 58, 24],
    [0, 54, 24],
    [0, 64, 24],
    [0, 60, 20],
    [0, 64, 24],
    [0, 56, 24],
    [20, 42, 22],
    [12, 40, 24],
    [16, 44, 24],
    [12, 40, 24],
    [12, 50, 24],
    [8, 46, 24],
    [8, 50, 24],
    [4, 52, 24],
    [4, 56, 24],
    [0, 52, 24],
    [0, 60, 20],
    [0, 60, 24],
    [0, 64, 20],
    [0, 58, 22],
    [0, 62, 24],
    [0, 56, 24],
    [16, 42, 24],
    [12, 38, 24],
    [12, 46, 20],
    [8, 44, 24],
    [8, 48, 24],
    [8, 44, 24],
    [8, 48, 24],
    [0, 50, 24],
    [4, 54, 24],
    [0, 54, 22],
    [0, 58, 22],
    [0, 56, 20],
    [0, 62, 22],
    [0, 56, 24],
    [16, 42, 24],
    [8, 42, 24],
    [12, 44, 22],
    [8, 42, 24],
    [8, 50, 24],
    [4, 50, 20],
    [4, 50, 22],
    [0, 52, 24],
    [0, 56, 22],
    [0, 52, 24],
    [0, 56, 24],
    [0, 54, 22],
    [0, 60, 24],
    [4, 48, 24],
    [12, 42, 22],
    [8, 42, 22],
    [12, 42, 24],
    [4, 44, 24],
    [4, 48, 24],
    [0, 50, 24],
    [4, 52, 22],
    [0, 50, 24],
    [0, 54, 24],
    [0, 50, 24],
    [0, 58, 22],
    [0, 52, 24],
    [8, 40, 24],
    [4, 42, 24],
    [8, 40, 24],
    [0, 46, 22],
    [4, 46, 24],
    [0, 48, 18],
    [0, 52, 24],
    [0, 48, 24],
    [0, 52, 24],
    [4, 42, 24],
    [8, 42, 20],
    [4, 40, 24],
    [4, 44, 24],
    [0, 44, 22],
    [0, 48, 22],
    [0, 48, 22],
    [0, 52, 22],
    [0, 46, 24],
    [4, 40, 22],
    [0, 42, 22],
    [4, 42, 24],
    [0, 42, 24],
    [0, 46, 22],
    [4, 40, 24],
    [4, 42, 22],
    [0, 40, 24],
    [0, 44, 24],
    [0, 40, 24],
    [0, 48, 20],
    [0, 42, 24],
    [0, 42, 24],
    [0, 38, 24],
    [0, 42, 24],
    [4, 36, 24],
    [0, 46, 22],
    [0, 40, 24],
    [0, 42, 24],
    [0, 36, 24],
    [0, 40, 24],
    [0, 38, 24],
    [0, 40, 24],
    [0, 34, 24],
    [0, 38, 24],
    [0, 32, 24],
    [0, 30, 24],
    [0, 28, 22],
    [0, 26, 22],
    [0, 24, 20],
    [0, 28, 24],
    [0, 24, 24],
    [0, 28, 22],
    [0, 22, 24],
    [0, 26, 24],
    [0, 20, 24],
];

impl SheikFlight {
    pub fn start(facts: &RouteFacts<'_>, beat: SheikBeat) -> Option<Self> {
        if facts.fighter != 0x10
            || beat.status != FALL
            || beat.jumps != 1
            || !beat.down
            || !beat.up
            // The head joint validates the native read. The collision checks
            // in the flight projection use the measured shape of each move.
            || !facts.body_clearance.is_finite()
            || facts.body_clearance <= 0.0
            || !beat
                .position
                .into_iter()
                .chain(beat.velocity)
                .all(f32::is_finite)
            || beat.velocity[0].abs() > AIR_ACCEL
            || !(-FALL_SPEED..=0.0).contains(&beat.velocity[1])
            || facts.crossing_band().is_none()
        {
            return None;
        }

        let [left, right] = facts.ledges;
        let (near, ledge, inward) = if beat.position[0] < left[0] {
            (left, right, 1.0)
        } else {
            (right, left, -1.0)
        };
        let flat = facts
            .outline
            .iter()
            .flatten()
            .map(|p| p[1])
            .reduce(f32::min)?;
        let bottom = facts.blast[3];
        if (right[0] - left[0] - 159.975).abs() > 1.0
            || (right[1] - left[1]).abs() > 0.5
            || (flat - near[1] + 40.1).abs() > 1.0
            || (bottom - near[1] + 140.0).abs() > 1.0
        {
            return None;
        }
        let outside = (near[0] - beat.position[0]) * inward;
        let height = beat.position[1] - near[1];
        if !super::region::contains(&S_X_RING, outside, height) {
            return None;
        }
        let column = (outside / 4.0) as u32;
        let row = ((40.0 - height) / 4.0) as usize;
        let slot = S_X_RING[..row]
            .iter()
            .map(|mask| mask.count_ones())
            .sum::<u32>()
            + (S_X_RING[row] & ((1 << column) - 1)).count_ones();
        let [at_rest, at_terminal] = S_X_TIMING.as_chunks::<2>().0[slot as usize];
        let mut outline = Outline::new();
        for &line in facts.outline {
            outline.add(line);
        }

        let mut flight = Self {
            outline,
            ledge,
            near,
            inward,
            origin: None,
            schedule: FishSchedule {
                quiet: 0,
                drift: 0,
                rise: 0,
            },
            floor_limit: bottom + 4.0,
            estimate: 0,
            target: ledge,
            aim: None,
        };
        let seeds = if beat.velocity[1] > -FALL_SPEED / 2.0 {
            [at_rest, at_terminal]
        } else {
            [at_terminal, at_rest]
        };
        for offset in [0, -2, 2, -4, 4] {
            for [quiet, drift, rise] in seeds {
                flight.schedule = FishSchedule {
                    quiet: u32::from(quiet),
                    drift: u32::from(drift.saturating_add_signed(offset)),
                    rise: u32::from(rise),
                };
                if let Some(finish) = flight.predict(near, beat) {
                    let FishSchedule { quiet, drift, rise } = flight.schedule;
                    flight.estimate =
                        (quiet + drift + DURING_FISH + rise + 36) as u16 + u16::from(finish.steps);
                    flight.target = finish.catch;
                    return Some(flight);
                }
            }
        }
        None
    }

    pub fn crossing_frames(&self) -> u16 {
        self.estimate
    }

    pub fn exit_point(&self) -> Point {
        self.target
    }

    fn predict(&self, near: Point, beat: SheikBeat) -> Option<Vanish> {
        let FishSchedule { quiet, drift, .. } = self.schedule;
        let mut from = beat.position;
        let mut vx = beat.velocity[0] * self.inward;
        let mut vy = beat.velocity[1];
        for frame in 0..quiet + drift {
            vx = if frame < quiet {
                vx.signum() * (vx.abs() - BRAKE).max(0.0)
            } else {
                (vx + AIR_ACCEL).min(AIR_SPEED)
            };
            vy = (vy - GRAVITY).max(-FALL_SPEED);
            if !self.project_row(&mut from, vx, vy, HEAD) || self.near_cliff(near, from) {
                return None;
            }
        }
        self.finish_fish(near, from, vx, vy)
    }

    fn project_row(&self, from: &mut Point, vx: f32, vy: f32, height: f32) -> bool {
        let next = [from[0] + vx * self.inward, from[1] + vy];
        let clear = super::aerial_clear(self.outline.segments(), [*from, next], self.floor_limit, height, HALF);
        *from = next;
        clear
    }

    fn near_cliff(&self, near: Point, position: Point) -> bool {
        (-9.6..=16.5).contains(&((near[0] - position[0]) * self.inward))
            && (7.5..=24.5).contains(&(near[1] - position[1]))
    }

    fn finish_fish(&self, near: Point, mut from: Point, vx: f32, vy: f32) -> Option<Vanish> {
        let vx = vx.signum() * (vx.abs() - BRAKE).max(0.0);
        let mut vy = (vy - GRAVITY).max(-FALL_SPEED);
        if !self.project_row(&mut from, vx, vy, HEAD) || self.near_cliff(near, from) {
            return None;
        }

        // From the first Fish status row through the transition into the double jump.
        for frame in 0..21 {
            let dx = (1.5 + 0.2 * (frame as f32 - 1.0).max(0.0)).min(2.4);
            if !self.project_row(&mut from, dx, 2.775 - 0.225 * frame as f32, FISH_HEAD) {
                return None;
            }
        }
        const FISH_BRAKE: [f32; 9] = [2.16, 1.944, 1.75, 1.575, 1.417, 1.276, 1.148, 1.033, 1.123];
        for frame in 0..29 {
            let dx = FISH_BRAKE.get(frame).copied().unwrap_or(AIR_SPEED);
            if !self.project_row(&mut from, dx, -FALL_SPEED, HEAD) {
                return None;
            }
        }

        // The actual launch pad is chosen again from the observed Vanish press row.
        vy = (2.0 * GRAVITY * 40.0).sqrt() - GRAVITY / 2.0;
        for frame in 0..self.schedule.rise - 1 {
            let dx = if frame == 0 { 1.09 } else { AIR_SPEED };
            if !self.project_row(&mut from, dx, vy, HEAD) {
                return None;
            }
            vy -= GRAVITY;
        }
        vanish(
            self.outline.segments(),
            from,
            [self.inward * AIR_SPEED, vy + GRAVITY],
            self.ledge,
            self.inward,
            self.floor_limit,
        )
    }

    pub fn step(&mut self, beat: SheikBeat) -> Option<Correction> {
        use super::niche::pad;

        if !beat
            .position
            .into_iter()
            .chain(beat.velocity)
            .all(f32::is_finite)
            || beat.position[1] <= self.floor_limit
        {
            return None;
        }
        let origin = *self.origin.get_or_insert(beat.frame);
        let at = beat.frame.wrapping_sub(origin);
        let FishSchedule { quiet, drift, rise } = self.schedule;
        let fish = quiet + drift;
        let jump = fish + DURING_FISH;
        let vanish_at = jump + rise;
        let forward = [self.inward, 0.0];
        let drift = pad(forward, 0);
        let down = pad([0.0, -1.0], SPECIAL);
        let up = pad([0.0, 1.0], SPECIAL);
        if at < quiet && beat.status == FALL && beat.jumps == 1 {
            return Some(pad([0.0, 0.0], 0));
        }
        if at < fish && beat.status == FALL && beat.jumps == 1 {
            return Some(drift);
        }
        match (at, beat.status) {
            (n, FALL) if n == fish && beat.down && beat.up && beat.jumps == 1 => {
                self.finish_fish(self.near, beat.position, beat.velocity[0] * self.inward, beat.velocity[1])?;
                Some(down)
            }
            (n, FISH_START) if n == fish + 1 => Some(down),
            (n, FISH_START | FISH_END) if n > fish + 1 && n < jump => Some(drift),
            (n, FISH_END) if n == jump && beat.can_jump && beat.jumps == 1 => {
                Some(pad(forward, JUMP))
            }
            (n, JUMP_AERIAL) if n == jump + 1 => Some(pad(forward, JUMP)),
            (n, JUMP_AERIAL) if n > jump + 1 && n < vanish_at => Some(drift),
            (n, JUMP_AERIAL) if n == vanish_at && beat.up => {
                let path = vanish(
                    self.outline.segments(),
                    beat.position,
                    beat.velocity,
                    self.ledge,
                    self.inward,
                    self.floor_limit,
                )?;
                self.aim = Some(path.stick);
                Some(up)
            }
            (n, VANISH_START) if n == vanish_at + 1 && self.aim.is_some() => Some(up),
            (n, VANISH_START | VANISH_TRAVEL) if n > vanish_at + 1 && n <= vanish_at + 56 => {
                self.aim.map(|stick| pad(stick, 0))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
lib_unit_tests! {
    mod tests {
        use super::*;

        const LEDGE: Point = [79.989, 0.0];
        const RING: [Segment; 8] = [
            [[-79.986, 0.0], [-79.986, -8.319]],
            [[-79.986, -8.319], [-64.395, -25.111]],
            [[-64.395, -25.111], [-41.396, -40.099]],
            [[-41.396, -40.099], [41.416, -40.118]],
            [[41.416, -40.118], [64.387, -25.103]],
            [[64.387, -25.103], [79.989, -8.319]],
            [[79.989, -8.319], LEDGE],
            [LEDGE, [-79.986, 0.0]],
        ];

        #[test]
        fn sheik_vanish_reaches_the_far_ledge_from_the_measured_jump_apex() {
            // fg2-wr-yoga-2 seq 25529: the pad that begins Vanish.
            // Seq 25603 at travel entry is (75.19899, -62.5048).
            let start = [43.60899, -83.60389];
            let speed = [1.155, 0.3891];
            let route = vanish(&RING, start, speed, LEDGE, 1.0, -136.0).expect("far Vanish");
            assert_eq!(route.stick, [25.0 / 120.0, 117.0 / 120.0]);
            assert!((3.0..=6.0).contains(&(route.catch[0] - LEDGE[0])));
            assert!((-19.0..=-11.7).contains(&route.catch[1]));

            let reflected = RING.map(|line| line.map(|[x, y]| [-x, y]));
            let reverse = vanish(&reflected, [-start[0], start[1]], [-speed[0], speed[1]], [-LEDGE[0], LEDGE[1]], -1.0, -136.0)
                .expect("mirrored far Vanish");
            assert_eq!(reverse.stick, [-route.stick[0], route.stick[1]]);
            assert_eq!(reverse.catch, [-route.catch[0], route.catch[1]]);
        }

        #[test]
        fn sheik_bouncing_fish_jump_and_vanish_follow_the_measured_applied_pads() {
            let mut outline = Outline::new();
            for line in RING {
                outline.add(line);
            }
            let mut flight = SheikFlight {
                outline,
                ledge: LEDGE,
                near: [-79.986, 0.0],
                inward: 1.0,
                origin: Some(0),
                schedule: FishSchedule { quiet: 0, drift: 36, rise: 22 },
                floor_limit: -136.0,
                estimate: 161,
                target: LEDGE,
                aim: None,
            };
            let drift = [1.0, 0.0];
            let down = [0.0, -1.0];
            let up = [0.0, 1.0];
            let angle = [25.0 / 120.0, 117.0 / 120.0];
            // Exact 990288d6, fg2-wr-yoga-2, wr-s-x-20-out-y-28 attempt 1.
            let rows = [
                (0, 0xe, [-99.986, -28.15], [0.0, -0.15], 1, drift, 0),
                (35, 0xe, [-66.401, -80.05], [1.155, -1.75], 1, drift, 0),
                (36, 0xe, [-65.246, -81.8], [1.155, -1.75], 1, down, SPECIAL),
                (37, 0x1df, [-64.106, -83.55], [1.14, -1.75], 1, down, SPECIAL),
                (38, 0x1df, [-62.606, -80.775], [1.5, 2.775], 1, drift, 0),
                (58, 0x1e9, [-17.106, -72.525], [2.4, -1.725], 1, drift, 0),
                (86, 0x1e9, [18.264, -121.525], [1.155, -1.75], 1, drift, JUMP),
                (87, 0xc, [19.419, -123.275], [1.155, -1.75], 0, drift, JUMP),
                (88, 0xc, [20.509, -119.8859], [1.09, 3.3891], 0, drift, 0),
                (108, 0xc, [43.609, -83.6039], [1.155, 0.3891], 0, up, SPECIAL),
                (109, 0x1de, [44.749, -83.3648], [1.14, 0.2391], 0, up, SPECIAL),
                (110, 0x1de, [45.874, -80.3548], [1.125, 3.01], 0, angle, 0),
                (144, 0x1e7, [75.199, -62.5048], [0.615, -0.25], 0, angle, 0),
                (160, 0x1e7, [83.5474, -23.4346], [0.5218, 2.4419], 0, angle, 0),
            ];
            for (frame, status, position, velocity, jumps, stick, button) in rows {
                let beat = SheikBeat {
                    frame, status, position, velocity, jumps, can_jump: true, down: true, up: true,
                };
                if frame == 86 {
                    assert!(flight.step(SheikBeat { can_jump: false, ..beat }).is_none());
                }
                let edit = flight.step(beat).unwrap_or_else(|| panic!("no correction at frame {frame}"));
                assert_eq!(edit.stick, Some(stick), "frame {frame}");
                assert_eq!(edit.press & (SPECIAL | JUMP), button, "frame {frame}");
            }
            let ended = SheikBeat {
                frame: 161, status: 0x75, position: [84.6565, -16.9736], velocity: [1.1091, 6.461],
                jumps: 0, can_jump: false, down: true, up: true,
            };
            assert!(flight.step(ended).is_none());
        }

        #[test]
        fn sheik_checks_live_fish_commit_after_a_trajectory_change() {
            let mut outline = Outline::new();
            for line in RING {
                outline.add(line);
            }
            let mut flight = SheikFlight {
                outline,
                ledge: LEDGE,
                near: [-79.986, 0.0],
                inward: 1.0,
                origin: Some(0),
                schedule: FishSchedule { quiet: 0, drift: 36, rise: 22 },
                floor_limit: -136.0,
                estimate: 161,
                target: LEDGE,
                aim: None,
            };
            let press = SheikBeat {
                frame: 36,
                status: FALL,
                position: [-65.246, -81.8],
                velocity: [1.155, -1.75],
                jumps: 1,
                can_jump: false,
                down: true,
                up: true,
            };
            assert_eq!(flight.step(press).expect("clear Fish").press & SPECIAL, SPECIAL);
            let blocked = SheikBeat { position: [-65.246, -55.0], ..press };
            assert!(flight.step(blocked).is_none(), "Fish would graze the underside");
            let short = SheikBeat { position: [-65.246, -111.0], ..press };
            assert!(flight.step(short).is_none(), "Fish would exhaust the fall budget");
        }

        #[test]
        fn sheik_selects_the_measured_crossing_with_the_live_head_joint_height() {
            let start = [-97.986, -30.0];
            let beat = SheikBeat {
                frame: 0, status: 0xe, position: start, velocity: [0.0, -0.15],
                jumps: 1, can_jump: true, down: true, up: true,
            };
            let mut facts = super::super::RouteFacts {
                fighter: 0x10,
                stage: (3, 0),
                position: start,
                player: [-75.0, 0.0],
                player_reach: 26.0,
                ledges: [[-79.986, 0.0], LEDGE],
                blast: [-180.0, 180.0, 180.0, -140.0],
                outline: &RING,
                // Exact 2ecd1229 product trace, Lower Left at 100%. The
                // animated head joint is above the 7.29 collision envelope.
                body_clearance: 10.017,
                ledge_distance: 3.0,
                fall_budget: 15.0,
                reach: [150.0, 100.0],
            };
            for stage in [0, 3, 311] {
                facts.stage = (stage, 0);
                let mut flight = SheikFlight::start(&facts, beat)
                    .unwrap_or_else(|| panic!("stage {stage}"));
                let expected = SheikFlight::start(&facts, beat).unwrap().step(beat);
                assert!(expected.is_some());
                assert_eq!(flight.step(SheikBeat { frame: 500, ..beat }), expected,
                    "the model clock starts at the first owned pad");
                assert!((130..=180).contains(&flight.crossing_frames()));
                assert!((81.0..=88.6).contains(&flight.exit_point()[0]));
            }
            facts.stage = (3, 1);
            assert!(SheikFlight::start(&facts, beat).is_none());
            facts.stage = (3, 0);
            for invalid_head in [0.0, -1.0, f32::NAN, f32::INFINITY] {
                facts.body_clearance = invalid_head;
                assert!(SheikFlight::start(&facts, beat).is_none());
            }
            facts.body_clearance = 10.017;
            assert!(SheikFlight::start(&facts, SheikBeat { jumps: 0, ..beat }).is_none());
            assert!(SheikFlight::start(&facts, SheikBeat { down: false, ..beat }).is_none());
            assert!(SheikFlight::start(&facts, SheikBeat { up: false, ..beat }).is_none());
            let outside = [-155.986, -30.0];
            assert!(SheikFlight::start(&facts, SheikBeat { position: outside, ..beat }).is_none());
        }

        #[test]
        fn sheik_center_and_edge_rollouts_only_offer_a_live_far_finish() {
            let near = [-79.986, 0.0];
            let reverse = RING.map(|line| line.map(|[x, y]| [-x, y]));
            let mut offered = 0;
            let mut reachable = 0;
            for (row, mask) in S_X_RING.into_iter().enumerate() {
                for column in 0..30 {
                    if mask & (1 << column) == 0 {
                        continue;
                    }
                    let height = 38.0 - row as f32 * 4.0;
                    let outside = 2.0 + column as f32 * 4.0;
                    for (dx, dy) in [(0.0, 0.0), (-1.99, 0.0), (1.99, 0.0), (0.0, -1.99), (0.0, 1.99)] {
                        for speed in [0.0, -0.15, -0.3, -0.875, -FALL_SPEED] {
                            let left_start = [near[0] - outside - dx, height + dy];
                            for direction in [-1.0, 1.0] {
                                let position = [left_start[0] * direction, left_start[1]];
                                let beat = SheikBeat {
                                    frame: 0, status: FALL, position, velocity: [0.0, speed],
                                    jumps: 1, can_jump: true, down: true, up: true,
                                };
                                let (ledge, far, outline) = if direction == 1.0 {
                                    (near, LEDGE, &RING)
                                } else { ([-near[0], 0.0], [-LEDGE[0], 0.0], &reverse) };
                                let ledges = if direction == 1.0 { [ledge, far] } else { [far, ledge] };
                                let facts = super::super::RouteFacts {
                                    fighter: 0x10, stage: (3, 0), position,
                                    player: [ledge[0] + 5.0 * direction, 0.0],
                                    player_reach: 10.0, ledges, blast: [-180.0, 180.0, 180.0, -140.0],
                                    outline, body_clearance: 7.29, ledge_distance: 7.29,
                                    fall_budget: FALL_SPEED, reach: [160.0, 90.0],
                                };
                                offered += 1;
                                if let Some(flight) = SheikFlight::start(&facts, beat) {
                                    let predicted = flight.predict(ledge, beat).expect("live far finish");
                                    assert_eq!(flight.exit_point(), predicted.catch);
                                    let outside = (predicted.catch[0] - far[0]) * direction;
                                    let below = far[1] - predicted.catch[1];
                                    assert!((1.0..=8.6).contains(&outside));
                                    assert!((11.7..=19.0).contains(&below));
                                    reachable += 1;
                                }
                            }
                        }
                    }
                }
            }
            assert_eq!(offered, 4100);
            println!("{reachable}/{offered} live starts admitted");
            assert!(reachable >= 3800, "too many baked footprints declined");
        }

        #[test]
        fn sheik_vanish_refuses_a_short_start_or_blocked_travel() {
            assert!(vanish(&RING, [40.0, -115.0], [1.155, 0.2], LEDGE, 1.0, -136.0).is_none());
            let mut blocked = RING.to_vec();
            blocked.push([[70.0, -42.0], [105.0, -42.0]]);
            assert!(vanish(&blocked, [43.60899, -83.60389], [1.155, 0.3891], LEDGE, 1.0, -136.0).is_none());
        }
    }
}
