//! Measured Bowser Jr. kart crossing on the normal Battlefield, FD and Waiting Room hull.

use super::{aerial_clear, niche::pad, Correction, Outline, Point, RouteFacts, JUMP, SPECIAL};

pub const KIND: i32 = 0x3a;
const GRAVITY: f32 = 0.092;
const TERMINAL: f32 = 1.65;
const AIR_SPEED: f32 = 1.134;
const AIR_JUMP_Y: f32 = 2.469_87;
const SHIP_NEUTRAL_BRAKE: f32 = 0.015;
const HEAD: f32 = 17.0;
const HALF: f32 = 5.0;
const FALL: i32 = 0xe;
const JUMP_AERIAL: i32 = 0xc;
const KART_START: i32 = 0x1dd;
const KART_DASH: i32 = 0x1e3;
const KART_JUMP: i32 = 0x1e4;
const SHIP_START: i32 = 0x1de;
const SHIP_RISE: i32 = 0x1e7;
const MIN_KART_JUMP_ROWS: u32 = 33;

// One schedule per admitted cell in reference-crossings/g3/koopajr-ring.txt.
// Columns start at 12 units out; rows start at +36. Zero is outside the region.
// Bits 0–5 are 1 plus half the descent depth. The next fields select dash,
// kart-jump and double-jump times, respectively (see gen_g3.py koopajr).
#[rustfmt::skip]
const PLANS: [[u16; 13]; 23] = [
    [8851, 8913, 8976, 12687, 12749, 12812, 19147, 19209, 19271, 19333, 17603, 11394, 19521],
    [8851, 8913, 8976, 12687, 12749, 12812, 19147, 19209, 19271, 19333, 17603, 11394, 19521],
    [8851, 8913, 8976, 12687, 12749, 12812, 19147, 19209, 19271, 19333, 17603, 11394, 19521],
    [8851, 8913, 8976, 12687, 12749, 12812, 19147, 19209, 19271, 19333, 17603, 11394, 19521],
    [8851, 8913, 8976, 12687, 12749, 12812, 19147, 19209, 19271, 19333, 17603, 11394, 19521],
    [8851, 8913, 8976, 12687, 12749, 12812, 19147, 19209, 19271, 19333, 17603, 11394, 19521],
    [8851, 8913, 8976, 12687, 12749, 12812, 19147, 19209, 19271, 19333, 17603, 11394, 19521],
    [8851, 8913, 8976, 12687, 12749, 12812, 19147, 19209, 19271, 19333, 17603, 11394, 19521],
    [8851, 8913, 8976, 12687, 12749, 12812, 19147, 19209, 19271, 19333, 17603, 11394, 19521],
    [8851, 8913, 8976, 12687, 12749, 12812, 19147, 19209, 19271, 19333, 17603, 11394, 19522],
    [8851, 8913, 8976, 12687, 12749, 12812, 19147, 19209, 19271, 19333, 17604, 19460, 0],
    [8851, 8913, 8976, 12687, 12749, 12812, 19147, 19209, 19271, 19334, 17606, 0, 0],
    [8851, 8913, 8976, 12687, 12749, 12812, 19147, 19209, 19272, 19336, 0, 0, 0],
    [8851, 8913, 8976, 12687, 12749, 12812, 19147, 19210, 19274, 0, 0, 0, 0],
    [8851, 8913, 8976, 12687, 12749, 12812, 19148, 19212, 0, 0, 0, 0, 0],
    [8851, 8913, 8976, 12687, 12750, 19086, 19150, 0, 0, 0, 0, 0, 0],
    [8851, 8913, 8976, 12688, 9104, 19088, 19152, 0, 0, 0, 0, 0, 0],
    [8851, 8914, 8978, 9042, 19026, 19090, 0, 0, 0, 0, 0, 0, 0],
    [8852, 8916, 8980, 18964, 19028, 0, 0, 0, 0, 0, 0, 0, 0],
    [8854, 8918, 18902, 18966, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [8856, 16984, 18904, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [16922, 16986, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [16924, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
];

#[derive(Clone, Copy)]
struct Schedule {
    depth: f32,
    dash: u32,
    kart: u32,
    aerial: u32,
}

impl Schedule {
    fn at(outside: f32, height: f32) -> Option<Self> {
        if !(12.0..64.0).contains(&outside) || !(-56.0..=36.0).contains(&height) {
            return None;
        }
        let column = (outside / 4.0) as usize - 3;
        let row = ((40.0 - height) / 4.0) as usize - 1;
        let packed = *PLANS.get(row)?.get(column)?;
        (packed != 0).then(|| Self {
            depth: f32::from((packed & 63) - 1) * 2.0,
            dash: 36 + 2 * u32::from(packed >> 6 & 31),
            kart: [32, 44, 56][(packed >> 11 & 3) as usize],
            aerial: [16, 24, 30][(packed >> 13 & 3) as usize],
        })
    }

    fn dash_step(row: u32) -> Point {
        let row = row as f32;
        [2.49 - 0.01 * row, (0.42 - 0.08 * row).max(-2.0)]
    }

    fn kart_step(self, row: u32) -> Point {
        let dash_speed = Self::dash_step(self.dash - 1)[0];
        [
            (0.5 * dash_speed + 0.08 * (row + 1) as f32).min(AIR_SPEED),
            (2.108 - GRAVITY * row as f32).max(-TERMINAL),
        ]
    }

    fn jump_step(row: u32) -> Point {
        let vx = match row {
            0 => 0.98,
            1 => 1.06,
            _ => AIR_SPEED,
        };
        [vx, AIR_JUMP_Y - GRAVITY * row as f32]
    }

    fn jump_rows(self) -> u32 {
        self.aerial - 1 - MIN_KART_JUMP_ROWS.saturating_sub(self.kart)
    }
}

#[derive(Clone, Copy)]
pub struct KartState {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub jumps: u8,
    pub side: bool,
    pub up: bool,
}

enum Phase {
    Descend(Option<u32>),
    Dash(u32),
    KartJump(u32),
    AirJump(u32),
    Rise(u32),
}

pub struct KartFlight {
    hull: Outline,
    schedule: Schedule,
    far: Point,
    bottom: f32,
    direction: f32,
    press_y: f32,
    expected_frames: u16,
    phase: Phase,
}

impl KartFlight {
    pub fn start(facts: &RouteFacts<'_>, velocity: Point) -> Option<Self> {
        if !super::is_crossing_stage(facts.stage)
            || facts.fighter != KIND
            || !(facts.body_clearance > 0.0 && facts.body_clearance <= HEAD)
            || !velocity.into_iter().all(f32::is_finite)
            || velocity[0] != 0.0
            || !(-TERMINAL..=0.0).contains(&velocity[1])
        {
            return None;
        }
        facts.crossing_band()?;
        let [left, right] = facts.ledges;
        let [x, y] = facts.position;
        let [near, far] = if x < left[0] {
            [left, right]
        } else {
            [right, left]
        };
        let direction = (far[0] - near[0]).signum();
        let schedule = Schedule::at((near[0] - x) * direction, y - near[1])?;
        let bottom = facts.blast[3];
        let floor = facts
            .outline
            .iter()
            .flatten()
            .map(|p| p[1])
            .reduce(f32::min)?;
        if ((right[0] - left[0]) * 0.5 - 79.99).abs() > 0.5
            || (right[1] - left[1]).abs() > 0.5
            || (floor - near[1] + 40.1).abs() > 1.0
            || (bottom - near[1] + 140.0).abs() > 1.0
        {
            return None;
        }
        let mut hull = Outline::new();
        for &line in facts.outline {
            hull.add(line);
        }
        let mut flight = Self {
            hull,
            schedule,
            far,
            bottom,
            direction,
            press_y: near[1] - schedule.depth,
            expected_frames: 0,
            phase: Phase::Descend(None),
        };
        let mut current = [x, y];
        let mut vy = velocity[1];
        for fall_frames in 0..=100 {
            if current[1] <= flight.press_y {
                flight.expected_frames = fall_frames
                    + (17 + schedule.dash + 1 + schedule.kart + schedule.aerial + 51) as u16;
                return flight.can_launch(current, vy).then_some(flight);
            }
            vy = (vy - GRAVITY).max(-TERMINAL);
            current[1] += vy;
        }
        None
    }

    pub fn exit_point(&self) -> Point {
        [self.far[0] + self.direction * 7.55, self.far[1] - 13.86]
    }

    pub fn crossing_frames(&self) -> u16 {
        self.expected_frames
    }

    fn clear(&self, position: &mut Point, delta: Point) -> bool {
        let next = [
            position[0] + delta[0] * self.direction,
            position[1] + delta[1],
        ];
        let clear = aerial_clear(
            self.hull.segments(),
            [*position, next],
            self.bottom + 4.0,
            HEAD + 2.0,
            HALF,
        );
        *position = next;
        clear
    }

    fn reaches_rise(&self, [x, y]: Point) -> bool {
        let base = y + 2.47 - self.far[1];
        let outside = (x - self.far[0]) * self.direction;
        // Group 3 measured -11.87 to 5.98 outside and a -78.24 to -55.07 base.
        // Reserve the same sideways and release-height margins as its baker.
        (-8.87..=2.98).contains(&outside) && (-77.74..=-55.57).contains(&base)
    }

    fn ship_carry([vx, vy]: Point) -> Point {
        [
            vx.signum() * (vx.abs() - SHIP_NEUTRAL_BRAKE).max(0.0),
            (vy - GRAVITY).max(-TERMINAL),
        ]
    }

    // Recheck the authored movement against the current hull and the live press.
    // The first rows of each new move still carry the preceding move's velocity.
    fn can_launch(&self, position: Point, vy: f32) -> bool {
        self.project_rise(position, vy)
            .is_some_and(|rise| self.reaches_rise(rise))
    }

    fn project_rise(&self, mut position: Point, vy: f32) -> Option<Point> {
        if !self.clear(&mut position, [0.08, (vy - GRAVITY).max(-TERMINAL)])
            || !(0..16).all(|_| self.clear(&mut position, [-1.0, -0.5]))
        {
            return None;
        }
        if !(0..self.schedule.dash).all(|row| self.clear(&mut position, Schedule::dash_step(row))) {
            return None;
        }
        self.project_after_dash(position, 0)
    }

    fn project_after_dash(&self, mut position: Point, kart_rows_done: u32) -> Option<Point> {
        if !(kart_rows_done..self.schedule.kart.max(MIN_KART_JUMP_ROWS))
            .all(|row| self.clear(&mut position, self.schedule.kart_step(row)))
        {
            return None;
        }
        let jump_rows = self.schedule.jump_rows();
        if !(0..jump_rows).all(|row| self.clear(&mut position, Schedule::jump_step(row))) {
            return None;
        }
        let carry = Self::ship_carry(Schedule::jump_step(jump_rows - 1));
        self.clear(&mut position, carry).then_some(position)
    }

    pub fn step(&mut self, state: KartState) -> Option<Correction> {
        let KartState {
            frame,
            status,
            position,
            velocity,
            jumps,
            side,
            up,
        } = state;
        if !position.into_iter().chain(velocity).all(f32::is_finite)
            || position[1] <= self.bottom + 4.0
        {
            return None;
        }
        let forward = [self.direction, 0.0];
        let up_stick = [0.0, 1.0];
        match self.phase {
            Phase::Descend(start) => {
                if status != FALL
                    || jumps != 1
                    || velocity[0] != 0.0
                    || !(-TERMINAL..=0.0).contains(&velocity[1])
                {
                    return None;
                }
                let start = start.unwrap_or(frame);
                if position[1] > self.press_y {
                    self.phase = Phase::Descend(Some(start));
                    // Reference input ramps down over eight thinks so it will
                    // bypass the near ledge without triggering a fast fall.
                    let ramp = (frame.saturating_sub(start).min(7) + 1) as f32;
                    return Some(pad([0.0, -ramp * 0.125], 0));
                }
                if !side || !up || !self.can_launch(position, velocity[1]) {
                    return None;
                }
                self.phase = Phase::Dash(frame);
                Some(pad(forward, SPECIAL))
            }
            Phase::Dash(start) => {
                let age = frame.saturating_sub(start);
                let follow = 16 + self.schedule.dash;
                if age == 0 && matches!(status, FALL | KART_START) {
                    Some(pad(forward, SPECIAL))
                } else if age == follow && status == KART_DASH && jumps == 1 {
                    // The last dash velocity is carried into the first kart-jump status row.
                    let mut after_dash = position;
                    let last_dash = Schedule::dash_step(self.schedule.dash - 1);
                    if !self.clear(&mut after_dash, last_dash)
                        || !self
                            .project_after_dash(after_dash, 0)
                            .is_some_and(|rise| self.reaches_rise(rise))
                    {
                        return None;
                    }
                    self.phase = Phase::KartJump(frame);
                    Some(pad(forward, JUMP))
                } else if age < follow
                    && (matches!(status, KART_START | KART_DASH) || age <= 1 && status == FALL)
                {
                    Some(pad(forward, 0))
                } else {
                    None
                }
            }
            Phase::KartJump(start) => {
                let age = frame.saturating_sub(start);
                if age == 0 && matches!(status, KART_DASH | KART_JUMP) {
                    Some(pad(forward, JUMP))
                } else if age == self.schedule.kart && status == KART_JUMP && jumps == 1 {
                    if !self
                        .project_after_dash(position, self.schedule.kart - 1)
                        .is_some_and(|rise| self.reaches_rise(rise))
                    {
                        return None;
                    }
                    self.phase = Phase::AirJump(frame);
                    Some(pad(forward, JUMP))
                } else if age < self.schedule.kart
                    && (status == KART_JUMP || age <= 1 && status == KART_DASH)
                {
                    Some(pad(forward, 0))
                } else {
                    None
                }
            }
            Phase::AirJump(start) => {
                let age = frame.saturating_sub(start);
                let airborne = matches!(status, JUMP_AERIAL | FALL);
                if age == 0 && matches!(status, KART_JUMP | JUMP_AERIAL) {
                    Some(pad(forward, JUMP))
                } else if age == self.schedule.aerial && airborne && jumps == 0 && up {
                    let carry = Self::ship_carry([velocity[0] * self.direction, velocity[1]]);
                    let mut carried = position;
                    if !self.clear(&mut carried, carry) || !self.reaches_rise(carried) {
                        return None;
                    }
                    self.phase = Phase::Rise(frame);
                    Some(pad(up_stick, SPECIAL))
                } else if age < self.schedule.aerial
                    && (airborne && jumps == 0 || age <= 1 && status == KART_JUMP)
                {
                    Some(pad(forward, 0))
                } else {
                    None
                }
            }
            Phase::Rise(start) => {
                let age = frame.saturating_sub(start);
                if age <= 1 && matches!(status, SHIP_START | JUMP_AERIAL | FALL) {
                    Some(pad(up_stick, SPECIAL))
                } else if age <= 51 && matches!(status, SHIP_START | SHIP_RISE) {
                    Some(pad([0.0; 2], 0))
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
lib_unit_tests! {
    mod tests {
        use super::*;
        use crate::model::edge_juggle::perfect::Segment;

        const LEFT: Point = [-79.986, 0.0];
        const RIGHT: Point = [79.989, 0.0];
        const FLOOR: [Segment; 8] = [
            [LEFT, [-79.986, -8.319]],
            [[-79.986, -8.319], [-64.395, -25.111]],
            [[-64.395, -25.111], [-41.396, -40.099]],
            [[-41.396, -40.099], [41.416, -40.118]],
            [[41.416, -40.118], [64.387, -25.103]],
            [[64.387, -25.103], [79.989, -8.319]],
            [[79.989, -8.319], RIGHT],
            [LEFT, RIGHT],
        ];

        fn facts(position: Point, stage: (i32, u32)) -> RouteFacts<'static> {
            RouteFacts {
                fighter: KIND,
                stage,
                position,
                player: [-75.0, 0.0],
                player_reach: 5.0,
                ledges: [LEFT, RIGHT],
                blast: [-230.0, 230.0, 185.0, -140.0],
                outline: &FLOOR,
                body_clearance: HEAD,
                ledge_distance: HEAD,
                fall_budget: TERMINAL,
                reach: [160.0, 100.0],
            }
        }

        #[test]
        fn koopajr_rise_projection_matches_the_native_press_and_carry_rows() {
            let mut flight = KartFlight::start(&facts([LEFT[0] - 14.0, 30.0], (3, 0)), [0.0, 0.0])
                .expect("the reference start is in the region");
            flight.schedule = Schedule { depth: 36.0, dash: 52, kart: 32, aerial: 30 };
            // g3-p2fd-xps, attempt koopajr-p2-fd-in-14-p30-1:
            // seq 2049 is the side-B press and seq 2311 is the first SpecialHi sample.
            let actual = [76.31503, -57.53577];
            let projected = flight.project_rise([-93.986, -36.96799], -TERMINAL).unwrap();
            for axis in 0..2 {
                assert!((actual[axis] - projected[axis]).abs() < 0.01, "{actual:?} vs {projected:?}");
            }
        }

        #[test]
        fn koopajr_baked_region_admits_only_proven_stage_forms_and_resources() {
            let position = [LEFT[0] - 22.0, -42.0];
            for stage in [(0, 0), (3, 0), (311, 0)] {
                assert!(KartFlight::start(&facts(position, stage), [0.0, 0.0]).is_some());
            }
            for stage in [(0, 1), (3, 1), (311, 2), (89, 0), (4, 0)] {
                assert!(KartFlight::start(&facts(position, stage), [0.0, 0.0]).is_none());
            }
            let legal = facts(position, (3, 0));
            for velocity in [[0.5, 0.0], [0.0, -2.0], [0.0, 0.1], [0.0, f32::NAN]] {
                assert!(KartFlight::start(&legal, velocity).is_none());
            }
            let outside = facts([LEFT[0] - 66.0, 0.0], (3, 0));
            assert!(KartFlight::start(&outside, [0.0, 0.0]).is_none());
        }

        #[test]
        fn koopajr_rechecks_the_six_reference_starts_both_ways() {
            for [out, y] in [[14.0, 30.0], [30.0, 10.0], [62.0, 6.0], [22.0, -42.0], [14.0, -50.0], [14.0, -54.0]] {
                for x in [LEFT[0] - out, RIGHT[0] + out] {
                    for vy in [0.0, -TERMINAL] {
                        let flight = KartFlight::start(&facts([x, y], (3, 0)), [0.0, vy]);
                        assert!(flight.is_some(), "start out={out}, y={y}, x={x}, vy={vy}");
                    }
                }
            }
        }

        #[test]
        fn koopajr_authors_distinct_kart_and_air_jump_edges_then_two_frames_of_up_b() {
            let mut state = KartState {
                frame: 0,
                status: FALL,
                position: [LEFT[0] - 22.0, 12.0],
                velocity: [0.0, 0.0],
                jumps: 1,
                side: true,
                up: true,
            };
            let mut flight = KartFlight::start(&facts(state.position, (3, 0)), state.velocity).unwrap();
            while state.position[1] > flight.press_y {
                let expected_y = -((state.frame.min(7) + 1) as f32) * 0.125;
                assert_eq!(flight.step(state), Some(pad([0.0, expected_y], 0)));
                state.velocity[1] = (state.velocity[1] - GRAVITY).max(-TERMINAL);
                state.position[1] += state.velocity[1];
                state.frame += 1;
            }
            let forward = [1.0, 0.0];
            assert_eq!(flight.step(state), Some(pad(forward, SPECIAL)));
            assert_eq!(flight.step(state), Some(pad(forward, SPECIAL)), "same think can repeat");
            let mut pose = state.position;
            assert!(flight.clear(&mut pose, [0.08, (state.velocity[1] - GRAVITY).max(-TERMINAL)]));
            for _ in 0..16 {
                assert!(flight.clear(&mut pose, [-1.0, -0.5]));
            }
            for row in 0..flight.schedule.dash - 1 {
                assert!(flight.clear(&mut pose, Schedule::dash_step(row)));
            }
            state.status = KART_DASH;
            state.frame += 16 + flight.schedule.dash - 1;
            assert_eq!(flight.step(state), Some(pad(forward, 0)));
            state.frame += 1;
            state.position = pose;
            assert_eq!(flight.step(state), Some(pad(forward, JUMP)));
            assert!(flight.clear(&mut pose, Schedule::dash_step(flight.schedule.dash - 1)));
            for row in 0..flight.schedule.kart - 1 {
                assert!(flight.clear(&mut pose, flight.schedule.kart_step(row)));
            }
            state.status = KART_JUMP;
            state.frame += flight.schedule.kart - 1;
            assert_eq!(flight.step(state), Some(pad(forward, 0)));
            state.frame += 1;
            state.position = pose;
            assert_eq!(flight.step(state), Some(pad(forward, JUMP)));
            state.status = JUMP_AERIAL;
            state.jumps = 0;
            state.frame += flight.schedule.aerial - 1;
            assert_eq!(flight.step(state), Some(pad(forward, 0)));
            state.frame += 1;
            state.position = [RIGHT[0] - 1.0, -63.0];
            state.velocity = [AIR_SPEED, 0.0];
            let rise = pad([0.0, 1.0], SPECIAL);
            assert_eq!(flight.step(state), Some(rise));
            state.status = SHIP_START;
            state.frame += 1;
            assert_eq!(flight.step(state), Some(rise));
            state.frame += 1;
            assert_eq!(flight.step(state), Some(pad([0.0; 2], 0)));
        }
    }
}
