//! Character-specific recovery pads for Guard the Edge.

use super::recovery::{crossing_margin, Crossing, Measured};
use super::{
    AirPhysics, Correction, Point, Route, SpecialReach, ATTACK, FLICK_JUMP, GUARD, JUMP, SPECIAL,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Drift,
    /// Glide along the corridor; one jump each time the foot sinks to the hold line.
    Hold,
    Jump,
    Side,
    ChargedSide,
    Down,
    Counter,
    Rise,
    Travel,
    QuickAttack,
    Barrel,
    Hover,
    SoraChain,
    Bolt,
    Glide,
    Float,
    Bomb,
    Missile,
    Swap,
}

#[derive(Clone, Copy)]
pub struct NearReach {
    pub gap: f32,
    pub climb: f32,
    pub body: f32,
    pub special: SpecialReach,
    pub can_jump: bool,
}

/// Live inputs of the under-stage crossing budget.
#[derive(Clone, Copy)]
pub struct Budget {
    pub air: AirPhysics,
    pub jumps: u8,
    pub special: Option<SpecialReach>,
    pub crossing: Crossing,
}

#[derive(Clone, Copy)]
pub struct Facts {
    pub fighter: i32,
    pub corridor: Option<Point>,
    pub covered_ledge: bool,
    pub player_over_crossing: bool,
    pub long_approach: bool,
    pub counter_contact: bool,
    pub near: Option<NearReach>,
    /// The plain return is short of distance, not of height.
    pub short: bool,
    /// Ledge height minus the fighter's.
    pub climb: f32,
    /// An unread budget refuses every crossing.
    pub budget: Option<Budget>,
}

#[derive(Clone, Copy)]
enum Intent {
    Across,
    ThunderOverhead,
    LongApproach,
    /// A level extension for a horizontal shortfall.
    Near,
    /// The same, flown only from ledge height or above (a wall below ends it).
    NearAbove,
    CounterContact,
    WhipGap,
    ChopColumn,
    SwapGap,
    PhotonGap,
    RayColumn,
}

/// Trace-measured side and down specials by row. Empty until NICHE-RESULTS.md has them.
const MEASURED: &[(u8, Measured)] = &[];

pub struct Row {
    pub id: u8,
    fighter: i32,
    intent: Intent,
    pub route: Route,
    pub moves: &'static [Move],
}

impl Row {
    const fn new(id: u8, fighter: i32, intent: Intent, moves: &'static [Move]) -> Self {
        let route = match intent {
            Intent::Near
            | Intent::NearAbove
            | Intent::CounterContact
            | Intent::WhipGap
            | Intent::ChopColumn
            | Intent::SwapGap
            | Intent::PhotonGap
            | Intent::RayColumn => Route::Near,
            _ => Route::Opposite,
        };
        Self {
            id,
            fighter,
            intent,
            route,
            moves,
        }
    }

    /// Spare climb of this row's crossing from the live budget. A near row has
    /// nothing to cross. Thunder's fall is unmodelled, so its row needs a measurement.
    pub fn margin(&self, facts: Facts) -> Option<f32> {
        if self.route == Route::Near {
            return Some(1.0);
        }
        let measured = MEASURED.iter().find(|(row, _)| *row == self.id);
        if matches!(self.intent, Intent::ThunderOverhead) && measured.is_none() {
            return None;
        }
        let budget = facts.budget?;
        crossing_margin(
            budget.air,
            budget.jumps,
            budget.special,
            self.aimed(),
            measured.map_or(Measured::default(), |&(_, travel)| travel),
            budget.crossing,
        )
    }

    /// The row's up special can be flown level under the stage.
    pub fn aimed(&self) -> bool {
        self.moves
            .iter()
            .any(|step| matches!(step, QuickAttack | Travel | Barrel | Hover | Glide | Bolt))
    }

    fn matches(&self, facts: Facts) -> bool {
        let clear = facts
            .corridor
            .is_some_and(|[bottom, top]| bottom.is_finite() && top.is_finite() && bottom < top);
        self.fighter == facts.fighter
            && match self.intent {
                Intent::Across => clear && facts.covered_ledge,
                Intent::ThunderOverhead => clear && facts.player_over_crossing,
                Intent::LongApproach => clear && facts.covered_ledge && facts.long_approach,
                Intent::Near => facts.short,
                Intent::NearAbove => facts.short && facts.climb <= 0.0,
                Intent::CounterContact => facts.counter_contact,
                intent => facts.near.is_some_and(|near| intent.matches_near(near)),
            }
            && self.margin(facts).is_some()
    }
}

impl Intent {
    fn matches_near(self, near: NearReach) -> bool {
        if !(near.special.valid()
            && near.gap.is_finite()
            && near.gap > 0.0
            && near.climb.is_finite()
            && near.body.is_finite()
            && near.body > 0.0)
        {
            return false;
        }
        let short = near.gap > near.special.horizontal;
        match self {
            Self::WhipGap => short && near.climb <= near.body,
            Self::ChopColumn => !short && near.climb <= near.special.rise,
            Self::SwapGap => short,
            Self::PhotonGap => short && near.climb <= 0.0,
            Self::RayColumn => !short && near.climb > 0.0 && near.can_jump,
            _ => false,
        }
    }
}

// Native body kinds follow TR's roster_grid; Charizard is the separate 0x26 body.
use Move::*;
// A crossing glides and holds its height first. A special that ends in special
// fall (SmashWiki "Helpless") is the row's last move; a jump is never burst at the entry.
const ROWS: [Row; 40] = [
    Row::new(1, 0x8, Intent::Across, &[Hold, QuickAttack]),
    Row::new(2, 0x8, Intent::ThunderOverhead, &[Down, Hold, QuickAttack]),
    Row::new(3, 0x8, Intent::LongApproach, &[Side, Hold, QuickAttack]),
    Row::new(4, 0x1c, Intent::Across, &[Hold, Rise]),
    Row::new(5, 0x27, Intent::Across, &[Hold, Barrel]),
    Row::new(6, 0x27, Intent::LongApproach, &[Side, Hold, Barrel]),
    Row::new(7, 0x2d, Intent::Across, &[Hold, Hover]),
    // Sonic Blade is special fall: it follows Aerial Sweep at its apex, at the exit.
    Row::new(8, 0x5d, Intent::Across, &[Hold, SoraChain]),
    Row::new(9, 0x34, Intent::Near, &[Side, Rise]),
    Row::new(10, 0x34, Intent::CounterContact, &[Counter, Rise]),
    Row::new(11, 0x10, Intent::Near, &[Down, Rise]),
    Row::new(12, 0x10, Intent::Across, &[Hold, Travel]),
    Row::new(13, 0x35, Intent::Across, &[ChargedSide, Hold, Travel]),
    Row::new(14, 0xa, Intent::Across, &[Hold, Bolt]),
    Row::new(15, 0x58, Intent::Across, &[Hold, Glide]),
    Row::new(16, 0x13, Intent::Across, &[Hold, QuickAttack]),
    Row::new(17, 0xc, Intent::Across, &[Hold]),
    Row::new(18, 0x1d, Intent::Across, &[Hold, Side]),
    Row::new(19, 0x1e, Intent::Across, &[Hold, Travel]),
    Row::new(20, 0x1f, Intent::Across, &[Hold, Travel]),
    Row::new(21, 0x37, Intent::Across, &[Hold, Travel]),
    // One Witch Twist per jump: the second needs the double jump between.
    Row::new(22, 0x40, Intent::Across, &[Hold, Travel, Jump, Travel, Side]),
    Row::new(23, 0x29, Intent::Across, &[Side, Hold, Rise]),
    Row::new(24, 0xd, Intent::Across, &[Float, Hold, Rise]),
    Row::new(25, 0xe, Intent::Across, &[Float, Hold, Rise]),
    Row::new(26, 0x22, Intent::Across, &[Hold, Hover]),
    Row::new(27, 0x2, Intent::Across, &[Bomb, Hold, Rise]),
    Row::new(28, 0x28, Intent::Across, &[Hold, Bolt]),
    Row::new(29, 0x2c, Intent::Across, &[Hold, Travel]),
    Row::new(30, 0x2b, Intent::Across, &[Hold, Hover]),
    Row::new(31, 0x3a, Intent::Across, &[Side, Hold, Rise]),
    Row::new(32, 0x53, Intent::Across, &[Hold, Rise]),
    // Yoshi's climb is the double jump: it waits for the exit.
    Row::new(33, 0x5, Intent::Across, &[Hold, Jump, Rise]),
    Row::new(34, 0x9, Intent::NearAbove, &[Missile]),
    Row::new(35, 0x26, Intent::Across, &[Hold, Rise]),
    Row::new(36, 0x47, Intent::WhipGap, &[Side, Rise]),
    Row::new(37, 0x47, Intent::ChopColumn, &[Rise]),
    Row::new(38, 0x5a, Intent::SwapGap, &[Swap]),
    Row::new(39, 0x5b, Intent::PhotonGap, &[Side]),
    Row::new(40, 0x5b, Intent::RayColumn, &[Jump, Rise]),
];

pub const CANDIDATE_ROWS: u64 = (1 << ROWS.len()) - 1;

pub fn select(facts: Facts, enabled: u64) -> Option<&'static Row> {
    ROWS.iter()
        .rev()
        .find(|row| enabled & (1 << (row.id - 1)) != 0 && row.matches(facts))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    Free,
    Starting,
    Charging,
    Travel,
    AimAgain,
    Finish,
    Interrupted,
}

#[derive(Clone, Copy)]
pub struct Sample {
    pub frame: u32,
    pub motion: Motion,
    pub can_start: bool,
    pub valid: bool,
    pub at_exit: bool,
    pub can_enter: bool,
    pub approaching: bool,
    /// The foot is under the hold line and a jump fits inside the band.
    pub low: bool,
    /// The glide ends: at the exit, or where the next special must start to arrive.
    pub hold_over: bool,
    pub direction: f32,
    pub aim: Point,
}

#[derive(Clone, Copy)]
enum Stage {
    Ready,
    Pressed(u32),
    Accepted(u32),
}

const ACTION_BUTTONS: u32 = ATTACK | SPECIAL | JUMP | GUARD | FLICK_JUMP;

pub struct Sequence {
    row: &'static Row,
    index: usize,
    stage: Stage,
    charge_start: Option<u32>,
    sora_aim_open: bool,
    last: Option<(u32, Option<Correction>)>,
}

impl Sequence {
    pub fn new(row: &'static Row) -> Self {
        Self {
            row,
            index: 0,
            stage: Stage::Ready,
            charge_start: None,
            sora_aim_open: false,
            last: None,
        }
    }

    pub fn action(&self) -> Option<Move> {
        self.row.moves.get(self.index).copied()
    }

    /// Only a down-special issued by this sequence and accepted by the game can
    /// carry an Edge attempt across the native form handoff.
    pub fn accepts_form(&self, fighter: Option<i32>) -> bool {
        self.action() == Some(Swap)
            && matches!(self.stage, Stage::Accepted(_))
            && matches!(fighter, None | Some(0x5b))
    }

    pub fn row(&self) -> &'static Row {
        self.row
    }

    fn advance(&mut self) {
        self.index += 1;
        self.stage = Stage::Ready;
        self.charge_start = None;
        self.sora_aim_open = false;
    }

    fn hold_frames(&mut self, movement: Move, sample: Sample, accepted: u32) -> u32 {
        let start = if movement == Barrel && sample.motion == Motion::Charging {
            *self.charge_start.get_or_insert(sample.frame)
        } else {
            accepted
        };
        sample.frame.saturating_sub(start)
    }

    pub fn step(&mut self, sample: Sample) -> Option<Correction> {
        if !sample.valid
            || sample.motion == Motion::Interrupted
            || ![-1.0, 1.0].contains(&sample.direction)
            || !sample
                .aim
                .into_iter()
                .all(|value| value.is_finite() && value.abs() <= 1.0)
        {
            self.index = self.row.moves.len();
            self.last = None;
            return None;
        }
        if let Some((frame, output)) = self.last.filter(|(frame, _)| *frame == sample.frame) {
            debug_assert_eq!(frame, sample.frame);
            return output;
        }
        let output = self.on_frame(sample);
        if output.is_none() {
            self.index = self.row.moves.len();
        }
        self.last = Some((sample.frame, output));
        output
    }

    /// The jump button of one altitude-hold frame.
    fn hold(&mut self, sample: Sample) -> u32 {
        match (self.stage, sample.motion) {
            (Stage::Ready | Stage::Accepted(_), Motion::Free) if sample.hold_over => self.advance(),
            (Stage::Ready, Motion::Free) if sample.low && sample.can_start => {
                self.stage = Stage::Pressed(sample.frame);
                return JUMP;
            }
            (Stage::Pressed(_), Motion::Travel) => self.stage = Stage::Accepted(sample.frame),
            // A refused press gets one frame, like any other move.
            (Stage::Pressed(frame), Motion::Free) if sample.frame.saturating_sub(frame) <= 1 => {}
            (Stage::Pressed(_) | Stage::Accepted(_), Motion::Free) => self.stage = Stage::Ready,
            _ => {}
        }
        0
    }

    fn on_frame(&mut self, sample: Sample) -> Option<Correction> {
        let movement = self.action()?;
        let [forward, level] = [sample.direction, 0.0];
        let aim_open = movement == SoraChain && sample.motion == Motion::AimAgain;
        let press_again = aim_open && !self.sora_aim_open;
        self.sora_aim_open = aim_open;
        if !sample.can_enter && matches!(self.stage, Stage::Ready) {
            return Some(pad([0.0; 2], 0));
        }
        if sample.approaching && matches!(self.stage, Stage::Ready) {
            return Some(pad([forward, level], 0));
        }
        if movement == Hold {
            return Some(pad([forward, level], self.hold(sample)));
        }
        if movement == Jump
            && !sample.can_start
            && sample.motion == Motion::Free
            && matches!(self.stage, Stage::Ready)
        {
            // The glide spent the last jump: go on with what follows.
            self.advance();
            return Some(pad([forward, level], 0));
        }
        if movement == Drift
            || matches!(movement, Rise | SoraChain)
                && self.row.route == Route::Opposite
                && !sample.at_exit
                && matches!(self.stage, Stage::Ready)
        {
            if movement == Drift && sample.at_exit {
                self.advance();
            }
            return Some(pad([forward, level], 0));
        }
        match (self.stage, sample.motion) {
            (Stage::Ready, Motion::Free) if sample.can_start => {
                self.stage = Stage::Pressed(sample.frame);
                Some(initial_input(movement, forward))
            }
            (Stage::Ready, Motion::Finish) => Some(pad([forward, level], 0)),
            (Stage::Pressed(frame), Motion::Free) if sample.frame.saturating_sub(frame) <= 1 => {
                // The accepted input must produce a native status. Do not retrigger it blindly.
                Some(pad([forward, level], 0))
            }
            (
                Stage::Pressed(_),
                Motion::Starting | Motion::Charging | Motion::Travel | Motion::AimAgain,
            ) => {
                self.stage = Stage::Accepted(sample.frame);
                Some(continuation(
                    movement,
                    sample,
                    self.hold_frames(movement, sample, sample.frame),
                    press_again,
                ))
            }
            (Stage::Accepted(_), Motion::Free) => {
                self.advance();
                Some(pad([forward, level], 0))
            }
            (Stage::Accepted(_), Motion::Finish) => {
                // Quick Attack samples the second warp's stick during the first END status.
                let aim = if movement == QuickAttack {
                    sample.aim
                } else {
                    [forward, level]
                };
                Some(pad(aim, 0))
            }
            (
                Stage::Accepted(frame),
                Motion::Starting | Motion::Charging | Motion::Travel | Motion::AimAgain,
            ) => Some(continuation(
                movement,
                sample,
                self.hold_frames(movement, sample, frame),
                press_again,
            )),
            _ => None,
        }
    }
}

fn pad(stick: Point, press: u32) -> Correction {
    Correction {
        stick: Some(stick),
        press,
        release: ACTION_BUTTONS & !press,
    }
}

fn initial_input(movement: Move, forward: f32) -> Correction {
    let (stick, button) = match movement {
        Drift => ([forward, 0.0], 0),
        Jump => ([forward, 0.0], JUMP),
        Float => ([0.0, -1.0], JUMP),
        Side | ChargedSide | Missile => ([forward, 0.0], SPECIAL),
        Down | Counter | Bomb | Swap => ([0.0, -1.0], SPECIAL),
        _ => ([0.0, 1.0], SPECIAL),
    };
    pad(stick, button)
}

fn continuation(
    movement: Move,
    sample: Sample,
    accepted_frames: u32,
    press_again: bool,
) -> Correction {
    let forward = [sample.direction, 0.0];
    let aim = match movement {
        Float if sample.motion == Motion::Starting => [0.0, -1.0],
        QuickAttack if sample.motion == Motion::Starting => forward,
        Down | Counter | Bomb | Swap if sample.motion == Motion::Starting => [0.0, -1.0],
        Rise | Travel | QuickAttack | Barrel | Hover | Bolt | Glide => sample.aim,
        _ => forward,
    };
    let press = match movement {
        ChargedSide
            if matches!(sample.motion, Motion::Starting | Motion::Charging)
                && accepted_frames < 20 =>
        {
            SPECIAL
        }
        Barrel | Missile if sample.motion == Motion::Starting => SPECIAL,
        Barrel if sample.motion == Motion::Charging && accepted_frames < 20 => SPECIAL,
        Float if sample.motion == Motion::Starting || !sample.at_exit => JUMP,
        SoraChain if press_again => SPECIAL,
        _ => 0,
    };
    pad(aim, press)
}

#[cfg(test)]
lib_unit_tests! {
    #[path = "../../../../tests/common/edge_juggle_niche.rs"]
    mod tests;
}
