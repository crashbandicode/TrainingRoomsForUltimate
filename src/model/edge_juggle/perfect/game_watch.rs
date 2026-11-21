//! The measured one-per-airtime Oil Panic stall.

use super::{
    niche::pad,
    player_on_path,
    recovery::{Reach, Zone},
    Correction, Point, SpecialReach, SPECIAL,
};

pub const KIND: i32 = 0x1c;
pub const ROW: u8 = 42;

const FALL: i32 = 0xe;
const JUMP_AERIAL: i32 = 0xc;
const SPECIAL_LW: i32 = 0x1e3;
const OIL_WAIT: i32 = 0x1e4;
const OIL_END: i32 = 0x1e8;
const FRAMES: u32 = 44;
const LAST_START: u32 = 23;
const TERMINAL: f32 = 1.24;
const STALL_TERMINAL: f32 = 1.20;
const STALL_GRAVITY: f32 = 0.04;
const STALL_BRAKE: f32 = 0.0025;

#[derive(Clone, Copy)]
pub struct OilInputs {
    pub frame: u32,
    pub status: i32,
    pub position: Point,
    pub velocity: Point,
    pub ledge: Point,
    pub player: Point,
    pub player_reach: f32,
    pub zone: Zone,
    pub reach: Reach,
    pub bottom: f32,
    pub can_start: bool,
}

#[derive(Default)]
enum Phase {
    #[default]
    Unarmed,
    Waiting(u32),
    Pressed(u32),
    Native(u32),
    Spent,
}

#[derive(Default)]
pub struct OilStall {
    phase: Phase,
    prior: bool,
    observed: bool,
}

impl OilStall {
    pub fn arm(&mut self, zone: Zone, pick: u32, timing: u32) {
        if !matches!(self.phase, Phase::Unarmed) {
            return;
        }
        let mut share = match zone {
            Zone::Ledge | Zone::Hang | Zone::OffHigh | Zone::OffLow => 60u64,
            Zone::Mid => 30,
            Zone::Far => 0,
        };
        if self.prior {
            share /= 2;
        }
        self.phase = if u64::from(pick) * 200 < share * (1u64 << 32) {
            Phase::Waiting(timing % (LAST_START + 1))
        } else {
            Phase::Spent
        };
    }

    pub fn due(&self, frame: u32) -> bool {
        matches!(self.phase, Phase::Waiting(due) if (due..=LAST_START).contains(&frame))
    }

    pub fn active(&self) -> bool {
        matches!(self.phase, Phase::Pressed(_) | Phase::Native(_))
    }

    pub fn cancel(&mut self) {
        self.phase = Phase::Spent;
    }

    pub fn launch(&mut self, inputs: OilInputs) -> Option<Correction> {
        let Phase::Waiting(due) = self.phase else {
            return None;
        };
        if inputs.frame < due {
            return None;
        }
        self.phase = Phase::Spent;
        if inputs.frame > LAST_START
            || !matches!(inputs.status, FALL | JUMP_AERIAL)
            || !inputs.can_start
            || inputs.zone == Zone::Far
        {
            return None;
        }
        if !Self::room_for_stall(inputs) {
            return None;
        }
        self.phase = Phase::Pressed(inputs.frame);
        Some(pad([0.0, -1.0], SPECIAL))
    }

    pub fn sustain(&mut self, status: i32, frame: u32) -> Option<Correction> {
        match self.phase {
            Phase::Pressed(since) if since == frame && matches!(status, FALL | JUMP_AERIAL) => {
                // The same game's think can be observed twice.
                Some(pad([0.0, -1.0], SPECIAL))
            }
            Phase::Pressed(since) if frame.wrapping_sub(since) == 1 && status == SPECIAL_LW => {
                self.phase = Phase::Native(since);
                self.observed = true;
                Some(pad([0.0, 0.0], 0))
            }
            Phase::Native(since)
                if frame.wrapping_sub(since) <= FRAMES
                    && matches!(status, SPECIAL_LW | OIL_WAIT | OIL_END) =>
            {
                Some(pad([0.0, 0.0], 0))
            }
            Phase::Pressed(_) | Phase::Native(_) => {
                self.phase = Phase::Spent;
                None
            }
            _ => None,
        }
    }

    pub fn next_attempt(&mut self) {
        self.prior = self.observed;
        self.phase = Phase::Unarmed;
        self.observed = false;
    }

    fn room_for_stall(inputs: OilInputs) -> bool {
        let OilInputs {
            position,
            velocity: [vx, vy],
            ledge,
            player,
            player_reach,
            zone,
            reach,
            bottom,
            ..
        } = inputs;
        let Some(special) = reach.special.filter(|reach| reach.valid()) else {
            return false;
        };
        if !position
            .into_iter()
            .chain(ledge)
            .chain(player)
            .chain([vx, vy, bottom, player_reach])
            .all(f32::is_finite)
            || player_reach <= 0.0
            || vy > 0.0
            || (position[0] - ledge[0]).abs() < 2.0
        {
            return false;
        }
        let inward = (ledge[0] - position[0]).signum();
        let speed = |brake: f32| vx.signum() * (vx.abs() - brake).max(0.0);
        let mut horizontal = speed(0.012);
        let mut from = position;
        let passes_guarded_lip = matches!(zone, Zone::Ledge | Zone::Hang);
        for row in 0..=FRAMES {
            let fall = if row == 0 {
                TERMINAL
            } else {
                (STALL_GRAVITY * row as f32).min(STALL_TERMINAL)
            };
            let to = [from[0] + horizontal, from[1] - fall];
            let outside = (ledge[0] - to[0]) * inward;
            let depth = ledge[1] - to[1];
            if outside < 2.0
                || to[1] <= bottom + 4.0
                || (!passes_guarded_lip && outside <= 16.0 && (5.0..=21.0).contains(&depth))
                || player_on_path(player, [from, to], player_reach)
            {
                return false;
            }
            from = to;
            horizontal = speed(0.012 + STALL_BRAKE * row as f32);
        }

        Reach {
            air: reach.air,
            jump: false,
            special: Some(SpecialReach {
                horizontal: special.horizontal.min(35.34),
                rise: special.rise.min(69.41),
            }),
        }
        .returns(
            [from[0], from[1] - 4.0],
            [horizontal, -STALL_TERMINAL],
            ledge,
            4,
            bottom,
        )
    }
}

#[cfg(test)]
lib_unit_tests! {
    mod tests {
        use super::*;
        use super::super::{AirPhysics, SpecialReach, SPECIAL};

        fn opening() -> OilInputs {
            OilInputs {
                frame: 11,
                status: FALL,
                position: [-108.0, -14.0],
                velocity: [0.0, -1.24],
                ledge: [-79.989, 0.0],
                player: [-75.0, 0.0],
                player_reach: 7.5,
                zone: Zone::Ledge,
                reach: Reach::trusted(
                    AirPhysics {
                        horizontal_speed: 1.176,
                        horizontal_acceleration: 0.10,
                        gravity: 0.08,
                        terminal_speed: 1.24,
                        jump_speed: 2.4,
                    },
                    true,
                    Some(SpecialReach { horizontal: 50.0, rise: 90.0 }),
                ),
                bottom: -140.0,
                can_start: true,
            }
        }

        #[test]
        fn oil_panic_taps_once_and_releases_until_the_measured_fall_return() {
            let mut oil = OilStall::default();
            oil.arm(Zone::Ledge, 0, 11);
            let input = opening();
            assert!(oil.launch(OilInputs { frame: 10, ..input }).is_none());
            let press = oil.launch(input).expect("one safe tap");
            assert_eq!(press.stick, Some([0.0, -1.0]));
            assert_eq!(press.press, SPECIAL);
            assert!(oil.launch(input).is_none());
            for (frame, status) in [(12, 0x1e3), (15, 0x1e3), (16, 0x1e4), (40, 0x1e4), (41, 0x1e8), (55, 0x1e8)] {
                let pad = oil.sustain(status, frame).expect("release the native stance");
                assert_eq!(pad.press & SPECIAL, 0);
                assert_eq!(pad.release & SPECIAL, SPECIAL);
            }
            assert!(oil.sustain(0xe, 56).is_none());
            oil.arm(Zone::Ledge, 0, 0);
            assert!(oil.launch(input).is_none(), "the same airtime cannot stall again");
            oil.next_attempt();
            oil.arm(Zone::Ledge, u32::MAX / 5, 0);
            assert!(oil.launch(input).is_none(), "the stall share was halved");
        }

        #[test]
        fn oil_panic_rejects_spent_fire_the_guarder_and_a_shortfall_after_the_stall() {
            let safe = opening();
            for invalid in [
                OilInputs { can_start: false, ..safe },
                OilInputs { status: SPECIAL_LW, ..safe },
                OilInputs { reach: Reach { special: None, ..safe.reach }, ..safe },
                OilInputs { position: [-108.0, -65.0], ..safe },
                OilInputs { velocity: [1.164, -1.24], ..safe },
                OilInputs { player: [-110.0, -38.0], zone: Zone::OffLow, ..safe },
            ] {
                let mut oil = OilStall::default();
                oil.arm(invalid.zone, 0, 0);
                assert!(oil.launch(invalid).is_none());
            }
            for (zone, expected) in [(Zone::Ledge, 30), (Zone::Hang, 30), (Zone::OffLow, 30), (Zone::Mid, 15), (Zone::Far, 0)] {
                let count = (0..100u64).filter(|&i| {
                    let mut oil = OilStall::default();
                    oil.arm(zone, (((2 * i + 1) * (1u64 << 32)) / 200) as u32, 0);
                    oil.launch(OilInputs { zone, ..safe }).is_some()
                }).count();
                assert_eq!(count, expected, "zone {}", zone as usize);
            }
        }
    }
}
