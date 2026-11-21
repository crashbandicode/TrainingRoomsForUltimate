//! Ness NS-1. The measured double-jump and PK Thunder 2 paths, with the fitted
//! ball path, come from Fusion G2's exact-release model and proof sweeps.
use super::{is_crossing_stage, region, Correction, Point, RouteFacts, SpecialReach};
use super::{ATTACK, FLICK_JUMP, GUARD, JUMP, SPECIAL};

#[path = "ness_data.rs"]
mod data;

pub const KIND: i32 = 0x0a;
pub const ROW: u8 = 46;
const FALL: i32 = 0x0e;
const JUMP_AERIAL: i32 = 0x0c;
const THUNDER_START: i32 = 0x1de;
const THUNDER_HOLD: i32 = 0x1e4;
const THUNDER_FLIGHT: i32 = 0x1e6;
const GRAVITY: f32 = 0.077;
const LEAD: u32 = 4;
const HIT: u32 = 57; // 61 native rows from rest, with B pressed at row 4.
const HOLD_DROP: f32 = 6.426;
const TRACK_DISTANCE: f32 = 1.0;

#[derive(Clone, Copy)]
pub struct ThunderBeat {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub facing: f32,
    pub jumps: u8,
    pub can_jump: bool,
    pub can_thunder: bool,
}

#[derive(Clone, Copy)]
struct Program {
    jump_rows: u32,
    loop_kind: usize,
}

pub struct ThunderReturn {
    program: Program,
    start: Point,
    ledge: Point,
    player: Point,
    origin: u32,
    lead_elapsed: u32,
    deep_rescue: bool,
    last_pad: Option<(u32, Correction)>,
}

/// The measured near-side gate accepts only the exact unsupported stock tuple.
pub fn measured_envelope(kind: i32, variant: i32, reach: SpecialReach) -> bool {
    kind == KIND && variant == 0 && reach.rise == 46.0 && reach.horizontal == -1.0
}

pub fn wizard_offset(stage: (i32, u32)) -> Option<Point> {
    match stage {
        (3, 0) => Some([120.0, -108.0]),
        (0 | 311, 0) => Some([84.0, -88.0]),
        _ => None,
    }
}

fn at_flown_start(out: f32, below: f32, [ref_out, ref_below]: Point) -> bool {
    (out - ref_out).abs() <= 0.03 && (below - ref_below).abs() <= 0.03
}

fn deep_stock_loss(stage: (i32, u32), out: f32, below: f32) -> bool {
    // Each reference is a measured Final Destination save with zero stock returns.
    stage == (3, 0)
        && [[108.0, 108.0], [110.0, 110.0], [114.0, 110.0], [120.0, 108.0]]
            .into_iter()
            .any(|start| at_flown_start(out, below, start))
}

fn program(stage: (i32, u32), out: f32, below: f32) -> Option<Program> {
    if !(16.0..=156.0).contains(&out) || !(0.0..120.0).contains(&below) {
        return None;
    }
    let key = (out / 4.0) as u16 * 30 + (below / 4.0) as u16;
    let code = if stage == (3, 0) && at_flown_start(out, below, [120.0, 108.0]) {
        // Independent deep sweep: the centre completed 5/5 against five
        // stock KOs. The nearby points were not certified by the cell bake.
        51 * 4 + 2
    } else {
        let slot = data::CELLS
            .binary_search_by_key(&key, |&(cell, _)| cell)
            .ok()?;
        data::CELLS[slot].1
    };
    Some(Program {
        jump_rows: u32::from(code / 4),
        loop_kind: usize::from(code % 4),
    })
}

fn add([x, y]: Point, [dx, dy]: Point) -> Point {
    [x + dx, y + dy]
}

fn distance_squared([x, y]: Point, [a, b]: Point) -> f32 {
    (x - a).powi(2) + (y - b).powi(2)
}

fn drop(rows: u32) -> f32 {
    let rows = rows as f32;
    GRAVITY * rows * (rows + 1.0) * 0.5
}

fn jump_position(start: Point, rows: u32) -> Point {
    data::JUMP_STEPS[..rows as usize]
        .iter()
        .fold(add(start, [0.1, -1.155]), |pos, &[dx, dy]| {
            add(pos, [f32::from(dx) * 0.0001, f32::from(dy) * 0.0001])
        })
}

fn jump_cast_launch(start: Point, jump_rows: u32) -> Point {
    // PK Thunder brakes on the cast row, halves that speed, and keeps
    // braking through the startup and hold until horizontal speed is zero.
    const AIR_BRAKE: f32 = 0.0225;
    let jump_speed = f32::from(data::JUMP_STEPS[jump_rows as usize - 1][0]) * 0.0001;
    let mut displacement = -AIR_BRAKE;
    let mut speed = (jump_speed - AIR_BRAKE) * 0.5;
    for _ in 1..HIT {
        speed -= AIR_BRAKE;
        if speed <= 0.0 {
            break;
        }
        displacement += speed;
    }
    add(jump_position(start, jump_rows), [displacement, -HOLD_DROP])
}

fn clean_start(foot: Point, ledge: Point) -> bool {
    foot[0] <= ledge[0] - 16.0 && foot[1] >= ledge[1] - 125.0
}

impl Program {
    fn flight(self) -> &'static [[i16; 2]] {
        [
            &data::FLIGHT_0,
            &data::FLIGHT_1,
            &data::FLIGHT_2,
            &data::FLIGHT_3,
        ][self.loop_kind]
    }

    fn can_finish(self, pos: Point, ledge: Point, player: Point) -> bool {
        self.guard_clearance(pos, ledge, player)
            .is_some_and(|clearance| clearance >= 0.0)
    }

    /// Minimum squared separation above the ball/body safety bounds, in units of each bound.
    /// Only a trajectory that retains the measured ledge catch supplies a clearance.
    fn guard_clearance(self, mut pos: Point, ledge: Point, player: Point) -> Option<f32> {
        if !clean_start(pos, ledge) {
            return None;
        }
        let ball = [&data::BALL_0, &data::BALL_1, &data::BALL_2, &data::BALL_3][self.loop_kind];
        let mut clearance = f32::MAX;
        for &[dx, dy] in ball {
            let centre = add(pos, [dx as f32 * 0.0001, dy as f32 * 0.0001 + 5.0]);
            clearance = clearance.min(distance_squared(centre, add(player, [0.0, 6.0])) / (20.0 * 20.0) - 1.0);
        }
        let mut approaching = false;
        let mut trusted = false;
        for &[dx, dy] in self.flight() {
            pos = add(pos, [f32::from(dx) * 0.0001, f32::from(dy) * 0.0001]);
            let out = ledge[0] - pos[0];
            let below = ledge[1] - pos[1];
            approaching |= (-0.5..=15.5).contains(&out) && (7.5..=22.5).contains(&below);
            if approaching {
                clearance = clearance.min(distance_squared(pos, player) / (14.0 * 14.0) - 1.0);
            }
            if (1.5..=13.0).contains(&out) && (9.5..=18.5).contains(&below) {
                if trusted {
                    return Some(clearance);
                }
                trusted = true;
            } else if trusted || out < 1.5 {
                return None;
            }
        }
        None
    }

    fn farthest_loop(self, pos: Point, ledge: Point, player: Point) -> Option<Self> {
        (0..4)
            .filter_map(|loop_kind| {
                let program = Self { loop_kind, ..self };
                Some((program, program.guard_clearance(pos, ledge, player)?))
            })
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(program, _)| program)
    }

    fn steering(self, age: u32) -> Point {
        let delay = if self.loop_kind == 1 { 20 } else { 16 };
        let Some(index) = age.checked_sub(delay) else {
            return [0.0; 2];
        };
        let pad = [
            &data::STICK_0,
            &data::STICK_1,
            &data::STICK_2,
            &data::STICK_3,
        ][self.loop_kind];
        let [x, y] = pad[index.min(31) as usize];
        [f32::from(x) / 120.0, f32::from(y) / 120.0]
    }
}

fn pad(stick: Point, press: u32) -> Correction {
    Correction {
        stick: Some(stick),
        press,
        release: (JUMP | FLICK_JUMP | SPECIAL | ATTACK | GUARD) & !press,
    }
}

impl ThunderReturn {
    pub fn start(facts: &RouteFacts<'_>, beat: ThunderBeat, guarder_wait: bool) -> Option<Self> {
        let [ledge, far] = facts.ledges;
        let below = ledge[1] - beat.position[1];
        let elapsed = (-beat.velocity[1] / GRAVITY).round();
        if facts.fighter != KIND
            || !facts.player.into_iter().all(f32::is_finite)
            || !is_crossing_stage(facts.stage)
            || facts.position != beat.position
            || beat.status != FALL
            || beat.facing != 1.0
            || beat.jumps != 1
            || !beat.can_jump
            || !beat.can_thunder
            || beat.velocity[0] != 0.0
            || !(0.0..=LEAD as f32).contains(&elapsed)
            || (beat.velocity[1] + elapsed * GRAVITY).abs() > 0.003
            || !region::has_common_underside([ledge, far], facts.blast[3], facts.outline)
        {
            return None;
        }
        let lead_elapsed = elapsed as u32;
        let start = add(beat.position, [0.0, drop(lead_elapsed)]);
        let out = ledge[0] - start[0];
        let below = below - drop(lead_elapsed);
        let deep_rescue = deep_stock_loss(facts.stage, out, below);
        if !deep_rescue
            && (!guarder_wait || facts.player[0] - ledge[0] < 4.99
                || (facts.player[1] - ledge[1]).abs() > 0.12)
        {
            return None;
        }
        let program = program(facts.stage, out, below)?;
        let launch = if program.jump_rows == 0 {
            add(start, [0.0, -7.581])
        } else {
            let launch = jump_cast_launch(start, program.jump_rows);
            if !clean_start(jump_position(start, 0), ledge)
                || (1..=program.jump_rows).any(|n| !clean_start(jump_position(start, n), ledge))
            {
                return None;
            }
            launch
        };
        let clearance = program.guard_clearance(launch, ledge, facts.player)?;
        (deep_rescue || clearance >= 0.0).then_some(Self {
                program,
                start,
                ledge,
                player: facts.player,
                origin: beat.frame,
                lead_elapsed,
                deep_rescue,
                last_pad: None,
            })
    }

    fn age(&self, frame: u32) -> Option<u32> {
        frame
            .wrapping_sub(self.origin)
            .checked_add(self.lead_elapsed)
    }

    fn cast_frame(&self) -> u32 {
        LEAD + self.program.jump_rows
    }

    pub fn jump_press(&self, frame: u32) -> bool {
        self.program.jump_rows != 0 && self.age(frame) == Some(LEAD)
    }

    pub fn cast_press(&self, frame: u32) -> bool {
        self.age(frame) == Some(self.cast_frame())
    }

    pub fn awaits_cast(&self, frame: u32) -> bool {
        self.age(frame).is_some_and(|age| age <= self.cast_frame())
    }

    pub fn step(&mut self, beat: ThunderBeat, guarder: Option<(Point, bool)>) -> Option<Correction> {
        if !beat.position.into_iter().all(f32::is_finite)
            || beat.position[1] <= self.ledge[1] - 125.0
        {
            return None;
        }
        if let Some((_, correction)) = self.last_pad.filter(|&(frame, _)| frame == beat.frame) {
            return Some(correction);
        }
        let age = self.age(beat.frame)?;
        let cast = self.cast_frame();
        let correction = if age <= cast {
            let started_jump = age > LEAD;
            let (player, waiting) = guarder?;
            if !player.into_iter().all(f32::is_finite)
                || (!self.deep_rescue && !started_jump
                    && (!waiting || distance_squared(self.player, player) > 1.0))
            {
                return None;
            }
            let (expected, status, jumps) = if started_jump {
                (jump_position(self.start, age - LEAD - 1), JUMP_AERIAL, 0)
            } else {
                (add(self.start, [0.0, -drop(age)]), FALL, 1)
            };
            if beat.status != status
                || beat.facing != 1.0
                || beat.jumps != jumps
                || distance_squared(expected, beat.position) > TRACK_DISTANCE * TRACK_DISTANCE
            {
                return None;
            }
            if age == cast {
                if !beat.can_thunder {
                    return None;
                }
                let nominal_launch = if started_jump {
                    jump_cast_launch(self.start, self.program.jump_rows)
                } else {
                    add(self.start, [0.0, -7.581])
                };
                let live_launch = add(
                    nominal_launch,
                    [
                        beat.position[0] - expected[0],
                        beat.position[1] - expected[1],
                    ],
                );
                if !self.program.can_finish(live_launch, self.ledge, player) {
                    if !started_jump && !self.deep_rescue {
                        return None;
                    }
                    self.program = self.program.farthest_loop(live_launch, self.ledge, player)?;
                }
                pad([0.0, 1.0], SPECIAL)
            } else if age == LEAD {
                beat.can_jump.then(|| pad([1.0, 0.0], JUMP))?
            } else if age > LEAD {
                pad([1.0, 0.0], if age == LEAD + 1 { JUMP } else { 0 })
            } else {
                pad([0.0; 2], 0)
            }
        } else {
            let since_cast = age - cast;
            match beat.status {
                THUNDER_START if since_cast == 1 => pad([0.0, 1.0], SPECIAL),
                THUNDER_START | THUNDER_HOLD if since_cast <= HIT + 4 => {
                    pad(self.program.steering(since_cast), 0)
                }
                THUNDER_FLIGHT if (HIT..=HIT + 66).contains(&since_cast) => {
                    pad(self.program.steering(since_cast), 0)
                }
                _ => return None,
            }
        };
        self.last_pad = Some((beat.frame, correction));
        Some(correction)
    }
}

#[cfg(test)]
#[path = "ness_tests.rs"]
mod tests;
