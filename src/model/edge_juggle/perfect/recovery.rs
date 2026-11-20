//! Player-grade recovery decisions for Guard the Edge. Pure: the runtime applies pad edits only.

use super::{
    choose_route, hit_recovery, AirPhysics, Correction, Plan, Point, Route, SpecialReach, JUMP,
};

/// Where the edge-guarder is against the fighter's near ledge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Far,
    Mid,
    Ledge,
    Hang,
    OffHigh,
    OffLow,
}

#[derive(Clone, Copy)]
pub struct Guarder {
    pub position: Point,
    pub hanging: bool,
    pub airborne: bool,
    pub reach: f32,
}

pub fn zone(guard: Guarder, ledge: Point, _cpu: Point) -> Zone {
    let [x, y] = [guard.position[0] - ledge[0], guard.position[1] - ledge[1]];
    if x.hypot(y) <= guard.reach {
        Zone::Ledge
    } else {
        Zone::Far
    }
}

pub fn mix(options: [Option<Plan>; 4], _zone: Zone, last: Option<Route>, draw: u32) -> Option<Plan> {
    choose_route(options, last, draw)
}

#[derive(Clone, Copy)]
pub struct Reach {
    pub air: AirPhysics,
    pub jump: bool,
    pub special: Option<SpecialReach>,
}

impl Reach {
    pub fn returns(self, from: Point, speed: Point, ledge: Point, delay: u8, bottom: f32) -> bool {
        let delay = if self.jump { delay } else { u8::MAX };
        self.air.reaches_with_jump(from, speed, ledge, delay, bottom)
    }

    pub fn deadline(self, from: Point, speed: Point, ledge: Point, bottom: f32) -> Option<u8> {
        (0..=8)
            .take_while(|&delay| self.returns(from, speed, ledge, delay, bottom))
            .last()
    }
}

pub fn planned_delay(route: Route, _zone: Zone, _deadline: Option<u8>, draw: u32) -> u8 {
    let delay = (draw % 9) as u8;
    if route == Route::LateLow {
        delay.max(1)
    } else {
        delay
    }
}

#[derive(Clone, Copy)]
pub struct HoldFacts {
    pub buttons: u32,
    pub button: u32,
    pub planned: u8,
    pub can_wait: bool,
    pub hot: bool,
}

#[derive(Default)]
pub struct Hold {
    pending: Option<u8>,
}

impl Hold {
    pub fn waiting(&self) -> bool {
        self.pending.is_some()
    }

    pub fn step(&mut self, facts: HoldFacts) -> Correction {
        let mut out = Correction::default();
        if facts.button != JUMP || facts.buttons & !(JUMP | super::FLICK_JUMP) != 0 {
            self.pending = None;
            return out;
        }
        if let Some(remaining) = self.pending.take() {
            if remaining == 0 || !facts.can_wait {
                out.press = JUMP;
                return out;
            }
            self.pending = Some(remaining - 1);
        } else if facts.buttons & JUMP != 0 && facts.planned != 0 && facts.can_wait {
            self.pending = Some(facts.planned.min(8) - 1);
        }
        if self.pending.is_some() {
            out.release = facts.buttons & JUMP;
        }
        out
    }
}

pub fn hot(_attack_age: Option<u32>, _reaction: u8, _covers: bool) -> bool {
    false
}

pub fn short(_reach: Reach, _from: Point, _speed: Point, _ledge: Point, _bottom: f32) -> bool {
    true
}

#[derive(Clone, Copy, Default)]
pub struct Measured {
    pub run: f32,
    pub rise: f32,
}

#[derive(Clone, Copy)]
pub struct Crossing {
    pub span: f32,
    pub band: Point,
    pub foot: f32,
    pub ledge: f32,
}

pub fn crossing_margin(
    _air: AirPhysics,
    _jumps: u8,
    _special: Option<SpecialReach>,
    _aimed: bool,
    _measured: Measured,
    _crossing: Crossing,
) -> Option<f32> {
    Some(1.0)
}

pub fn edge_di(
    cpu: Point,
    ledge: Point,
    speed: Point,
    hitstun: f32,
    blast: [f32; 4],
) -> Option<Point> {
    hit_recovery(cpu, ledge, speed, hitstun, blast)
}

#[cfg(test)]
lib_unit_tests! {
    #[path = "../../../../tests/common/edge_juggle_recovery.rs"]
    mod tests;
}
