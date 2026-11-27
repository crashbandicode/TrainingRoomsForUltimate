//! Ledge Trap: the stage clock, the round time and the CPU's pad on its way to the ledge.
use super::perfect::{Correction, FLICK_JUMP};
use super::{Observation, Outcome};
use crate::model::cpu::settings::Weights;
use crate::model::stage_geometry::{Place, Unread};

/// The CPU's start beside a ledge: this far outside it, at its height.
pub const HANG_OUT: f32 = 8.0;
/// Frames after release in which the CPU has to hang, stand or be touched; else the attempt is not counted.
pub const HANG_FRAMES: u32 = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rules {
    /// Frames the CPU may stand on the stage.
    pub fail: u16,
    /// Frames a round lasts; none: until a KO.
    pub time: Option<u16>,
}

impl Rules {
    pub const DEFAULT: Self = Self { fail: 120, time: Some(1800) };

    /// From the selected options of the Fail after and Round time rows.
    pub fn from_settings(fail: usize, time: usize) -> Self {
        Self {
            fail: [120, 150, 180][fail.min(2)],
            time: [Some(1200), Some(1800), Some(3600), None][time.min(3)],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trap {
    pub rules: Rules,
    clock: u16,
    engaged: bool,
}

impl Trap {
    pub const fn new(rules: Rules) -> Self {
        Self { rules, clock: 0, engaged: false }
    }

    /// Frames the CPU has stood on the stage in the running count.
    pub const fn clock(self) -> u16 {
        self.clock
    }

    /// One Live frame: `frames` since release; `touched`: Player 1 has hit or grabbed the CPU this attempt.
    pub fn step(&mut self, frames: u32, touched: bool, observed: Observation) -> Option<Outcome> {
        if observed.dead_or_rebirthing {
            return Some(if touched { Outcome::KeptOffKo } else { Outcome::Unresolved });
        }
        if observed.player_ko {
            return Some(Outcome::PlayerKo);
        }
        self.engaged |= observed.hanging || observed.grounded || touched;
        if !self.engaged {
            return (frames >= HANG_FRAMES).then_some(Outcome::Unresolved);
        }
        if observed.grabbed || observed.hanging || !observed.on_run {
            self.clock = 0;
        } else if observed.grounded {
            self.clock = self.clock.saturating_add(1);
            if self.clock >= self.rules.fail {
                return Some(Outcome::OnStage);
            }
        }
        self.rules.time.filter(|limit| frames >= u32::from(*limit)).map(|_| Outcome::KeptOffTime)
    }

    /// The CPU's pad on a Live frame: toward the stage until the attempt is engaged, neutral while it hangs.
    pub fn pad(self, hanging: bool, toward: f32, buttons: u32) -> Correction {
        if hanging || !self.engaged {
            Correction {
                stick: Some([if hanging { 0.0 } else { toward }, 0.0]),
                release: buttons & !FLICK_JUMP,
                ..Correction::default()
            }
        } else {
            Correction::default()
        }
    }
}

/// Player 1's start for a crosshair at `at`: the floor under it, else the point itself in open air.
pub fn player_start(floor: Option<[f32; 2]>, place: Result<Place, Unread>, at: [f32; 2]) -> Option<[f32; 2]> {
    floor.or_else(|| (place == Ok(Place::Open)).then_some(at))
}

/// The Ledge options row as the CPU ledge step's weights: none ticked reads as all four.
pub fn options(row: &[u8]) -> Weights<10> {
    let mut weights = [0; 10];
    for (slot, value) in weights[..4].iter_mut().zip(row) {
        *slot = *value;
    }
    if weights[..4].iter().all(|weight| *weight == 0) {
        weights[..4].fill(1);
    }
    Weights(weights)
}
