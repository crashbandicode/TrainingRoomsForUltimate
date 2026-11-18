//! Native move entry, sharing the recorder's mapped-input format. No escape formula lives here.
use super::moves::{Plan, Scenario, State};
use crate::model::{cpu::ids::C, input_delay::MappedInputs};

/// B04 keeps G&W's 14.2 tip spacing. Move108's Flip value is the midpoint
/// from native frames30/31 in artifacts/rc15-mashout/MEASURED-FIXTURES-108-95.md;
/// it is translation evidence, not collision or floor-contact proof.
pub fn placement_spacing(scenario: Scenario) -> f32 {
    if scenario.evidence == "B04" {
        14.2
    } else if scenario.plan == Plan::Flip {
        41.123_188
    } else if scenario.plan == Plan::Trolley {
        // A2-GEOMETRY.md: occupied reach + widest idle hurtbox + 3, rounded up to f32.
        45.150_684
    } else {
        6.0
    }
}

pub fn trolley_solid_floor(flags: u32) -> bool {
    const FLOOR: u32 = 0x0800;
    const PASS_THROUGH: u32 = 0x2000;
    flags & (FLOOR | PASS_THROUGH) == FLOOR
}

/// Require floor from Place to P1 and one unit beyond the widest observed P1
/// hurtbox on the capture side. The post-capture tail cannot affect admission.
pub fn trolley_floor_runway(
    [p1_x, p1_y]: [f32; 2],
    place_x: f32,
    facing: f32,
    [[left_x, left_y], [right_x, right_y]]: [[f32; 2]; 2],
) -> bool {
    const CAPTURE_MARGIN: f32 = 7.950_678; // A2 maximum idle hurt extent + 1, rounded up.
    let end_x = p1_x - facing * CAPTURE_MARGIN;
    let low = left_x.min(right_x);
    let high = left_x.max(right_x);
    [left_x, left_y, right_x, right_y, p1_y].into_iter().all(f32::is_finite)
        && left_y == right_y
        && (p1_y - left_y).abs() <= 0.25
        && [p1_x, place_x, end_x].into_iter().all(|x| low < x && x < high)
}

/// The original move status is requested on the attacker only. Victim statuses and links are native.
pub fn start(move_: Scenario) -> Option<&'static str> {
    Some(match move_.plan {
        // Cargo is common CatchWaitMain0xcee30's forward continuation of a real
        // ordinary catch (Donkey slot0x45 = SHOULDER_START); its catalog grab is0.
        Plan::Grab | Plan::DownThrow | Plan::Cargo => match move_.grab {
            1 => "FIGHTER_STATUS_KIND_CATCH_DASH",
            2 => "FIGHTER_STATUS_KIND_CATCH_TURN",
            _ => "FIGHTER_STATUS_KIND_CATCH",
        },
        Plan::AirSideSpecial => "FIGHTER_STATUS_KIND_JUMP_SQUAT",
        Plan::SideSpecial
        | Plan::Cling
        | Plan::Drag
        | Plan::Whip
        | Plan::Roller
        | Plan::Pin
        | Plan::Judge
        | Plan::Trolley => "FIGHTER_STATUS_KIND_SPECIAL_S",
        Plan::Egg
        | Plan::Inhale
        | Plan::NeutralSpecial
        | Plan::Freeze
        | Plan::IceShot
        | Plan::IceMagic => "FIGHTER_STATUS_KIND_SPECIAL_N",
        Plan::UpSpecial => "FIGHTER_STATUS_KIND_SPECIAL_HI",
        Plan::Drain | Plan::Blizzard | Plan::Flip | Plan::Hero => "FIGHTER_STATUS_KIND_SPECIAL_LW",
        Plan::DownTilt => "FIGHTER_STATUS_KIND_ATTACK_LW3",
        Plan::DownSmash => "FIGHTER_STATUS_KIND_ATTACK_LW4_START",
        Plan::Jab3 => "FIGHTER_STATUS_KIND_ATTACK",
        // These need their verified native sequence, loadout or meter setup before admission.
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrailMagicKind {
    Fire,
    Blizzard,
    Thunder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrailMagicStatus {
    Wait,
    SpecialN,
    N1,
    N1Shoot,
    N1End,
    N3,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrailMagicProgress {
    Pending,
    StartThunder,
    Prepared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrailFirePhase {
    Start,
    N1,
    Shoot,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrailMagicPhase {
    Fire(TrailFirePhase),
    AwaitThunder,
    Thunder { saw_n3: bool },
    Prepared,
}

/// Observes Trail's real N1/N3 status phases before allowing the next magic kind.
/// The actor never requests status or changes Trail's native work values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrailMagicEntry {
    phase: TrailMagicPhase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrailMagicError { Unavailable }

impl TrailMagicEntry {
    pub fn new(kind: TrailMagicKind) -> Result<Self, TrailMagicError> {
        let phase = match kind {
            TrailMagicKind::Fire => TrailMagicPhase::Fire(TrailFirePhase::Start),
            TrailMagicKind::Thunder => TrailMagicPhase::Thunder { saw_n3: false },
            TrailMagicKind::Blizzard => return Err(TrailMagicError::Unavailable),
        };
        Ok(Self { phase })
    }

    pub fn preparing(self) -> bool {
        true
    }

    pub fn prepared(self) -> bool {
        self.phase == TrailMagicPhase::Prepared
    }

    pub fn begin_thunder(&mut self, kind: TrailMagicKind) -> Result<(), TrailMagicError> {
        if self.phase != TrailMagicPhase::AwaitThunder || kind != TrailMagicKind::Thunder {
            return Err(TrailMagicError::Unavailable);
        }
        self.phase = TrailMagicPhase::Thunder { saw_n3: false };
        Ok(())
    }

    pub fn observe(
        &mut self,
        status: TrailMagicStatus,
        kind: TrailMagicKind,
        released: bool,
    ) -> Result<TrailMagicProgress, TrailMagicError> {
        let progress = match self.phase {
            TrailMagicPhase::Fire(phase) => {
                if !matches!(kind, TrailMagicKind::Fire | TrailMagicKind::Thunder) {
                    return Err(TrailMagicError::Unavailable);
                }
                match (phase, status) {
                    (TrailFirePhase::Start, TrailMagicStatus::SpecialN) => {
                        TrailMagicProgress::Pending
                    }
                    (TrailFirePhase::Start, TrailMagicStatus::N1) => {
                        self.phase = TrailMagicPhase::Fire(TrailFirePhase::N1);
                        TrailMagicProgress::Pending
                    }
                    (TrailFirePhase::N1, TrailMagicStatus::N1) => TrailMagicProgress::Pending,
                    (TrailFirePhase::N1, TrailMagicStatus::N1Shoot) => {
                        self.phase = TrailMagicPhase::Fire(TrailFirePhase::Shoot);
                        TrailMagicProgress::Pending
                    }
                    (TrailFirePhase::Shoot, TrailMagicStatus::N1Shoot) => {
                        TrailMagicProgress::Pending
                    }
                    (TrailFirePhase::Shoot, TrailMagicStatus::N1End) => {
                        self.phase = TrailMagicPhase::Fire(TrailFirePhase::End);
                        TrailMagicProgress::Pending
                    }
                    (TrailFirePhase::End, TrailMagicStatus::N1End) => TrailMagicProgress::Pending,
                    (TrailFirePhase::End, TrailMagicStatus::Wait) => match kind {
                        TrailMagicKind::Fire => TrailMagicProgress::Pending,
                        TrailMagicKind::Thunder if released => {
                            self.phase = TrailMagicPhase::AwaitThunder;
                            TrailMagicProgress::StartThunder
                        }
                        TrailMagicKind::Thunder => TrailMagicProgress::Pending,
                        // Fire must cycle to Thunder; a Blizzard kind here is unexpected native state: fail closed.
                        TrailMagicKind::Blizzard => return Err(TrailMagicError::Unavailable),
                    },
                    _ => return Err(TrailMagicError::Unavailable),
                }
            }
            TrailMagicPhase::Thunder { saw_n3 } => {
                if !matches!(kind, TrailMagicKind::Thunder | TrailMagicKind::Blizzard) {
                    return Err(TrailMagicError::Unavailable);
                }
                match (saw_n3, status) {
                    (false, TrailMagicStatus::SpecialN) => TrailMagicProgress::Pending,
                    (false, TrailMagicStatus::N3) => {
                        self.phase = TrailMagicPhase::Thunder { saw_n3: true };
                        TrailMagicProgress::Pending
                    }
                    (true, TrailMagicStatus::N3) => TrailMagicProgress::Pending,
                    (true, TrailMagicStatus::Wait) if kind == TrailMagicKind::Thunder => {
                        TrailMagicProgress::Pending
                    }
                    (true, TrailMagicStatus::Wait) if released => {
                        self.phase = TrailMagicPhase::Prepared;
                        TrailMagicProgress::Prepared
                    }
                    (true, TrailMagicStatus::Wait) => TrailMagicProgress::Pending,
                    _ => return Err(TrailMagicError::Unavailable),
                }
            }
            TrailMagicPhase::AwaitThunder | TrailMagicPhase::Prepared => return Err(TrailMagicError::Unavailable),
        };
        Ok(progress)
    }
}

/// OPUS-DESIGN-REVIEW-sora116-refreeze G3 S1 (116): at the escape or Result, the Blizzard provider is closed only
/// once the CPU has left the Blizzard status (SPECIAL_N2) and no Ice article is live, so no unobserved shard can
/// extend the hold.
pub fn trail_blizzard_closed(cpu_status: i32, blizzard_status: i32, ice_live: bool) -> bool {
    cpu_status != blizzard_status && !ice_live
}

// The stock Brave menu mirrors four native command enums into work IDs. The
// value named COMMAND18_ICESWORD is 17; COMMAND_NUM is 21, so valid entries are
// 0..20 and the native COMMAND_NONE sentinel is not a menu choice.
pub const HERO_COMMAND_COUNT: i32 = 0x15;
const HERO_WINDOW_OPEN: i32 = 3;
const HERO_INPUT_SPECIAL: u32 = 0x2;
// src/runtime/compat/input.rs documents the mapped guard bit as 0x8.
const HERO_INPUT_GUARD: u32 = 0x8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeroStatus { SpecialLw, Select, Cancel, Start, Failure, Other }

/// Only populated from granted native reads on the current CPU body. Menu
/// fields are read only when the observed window is OPEN3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeroRead {
    pub readers_ready: bool,
    pub cpu: usize,
    pub status: HeroStatus,
    pub ice_sword_command: i32,
    pub window_state: Option<i32>,
    pub index: Option<i32>,
    pub commands: Option<[i32; 4]>,
    pub success_sp: Option<bool>,
    pub active_command: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeroOutcome { Pending, Confirmed, Cancelled, Rejected }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeroError { Unavailable }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HeroPhase {
    AwaitMenu,
    Navigating { target: usize, commands: [i32; 4], from: usize },
    AfterMove { target: usize, commands: [i32; 4], index: usize },
    ConfirmPending { target: usize, commands: [i32; 4] },
    CancelPending,
    Confirmed,
    Rejected,
}

/// Follows the stock Brave menu using observed SELECT state and mapped input.
/// It never writes status, work values, resources or a victim state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeroEntry {
    cpu: usize,
    phase: HeroPhase,
}

impl HeroEntry {
    pub fn new(cpu: usize) -> Option<Self> {
        (cpu != 0).then_some(Self { cpu, phase: HeroPhase::AwaitMenu })
    }

    pub fn confirmed(&self) -> bool { self.phase == HeroPhase::Confirmed }

    fn menu(read: HeroRead) -> Result<Option<(usize, [i32; 4])>, HeroError> {
        let window = read.window_state.ok_or(HeroError::Unavailable)?;
        if !(0..=4).contains(&window) { return Err(HeroError::Unavailable); }
        if window != HERO_WINDOW_OPEN { return Ok(None); }
        let index = read.index.ok_or(HeroError::Unavailable)?;
        let commands = read.commands.ok_or(HeroError::Unavailable)?;
        if !(0..4).contains(&index)
            || commands.iter().any(|command| !(0..HERO_COMMAND_COUNT).contains(command))
        { return Err(HeroError::Unavailable); }
        Ok(Some((index as usize, commands)))
    }

    fn input(buttons: u32, lstick_y: i8) -> MappedInputs {
        MappedInputs { buttons, lstick_y, ..MappedInputs::NEUTRAL }
    }

    fn answer(input: MappedInputs, outcome: HeroOutcome) -> Result<(MappedInputs, HeroOutcome), HeroError> {
        Ok((input, outcome))
    }

    fn reject(&mut self) -> Result<(MappedInputs, HeroOutcome), HeroError> {
        self.phase = HeroPhase::Rejected;
        Self::answer(MappedInputs::NEUTRAL, HeroOutcome::Rejected)
    }

    fn checked_menu(read: HeroRead, commands: [i32; 4]) -> Result<usize, HeroError> {
        if read.status != HeroStatus::Select { return Err(HeroError::Unavailable); }
        let Some((index, observed)) = Self::menu(read)? else { return Err(HeroError::Unavailable); };
        if observed != commands { return Err(HeroError::Unavailable); }
        Ok(index)
    }

    /// Return exactly one mapped record and an outcome backed by native status,
    /// menu, success flag and current START active-command observations.
    pub fn observe(&mut self, read: HeroRead) -> Result<(MappedInputs, HeroOutcome), HeroError> {
        if !read.readers_ready || read.cpu != self.cpu
            || !(0..HERO_COMMAND_COUNT).contains(&read.ice_sword_command)
        { return Err(HeroError::Unavailable); }
        use HeroPhase::*;
        use HeroStatus::*;
        match self.phase {
            Confirmed => Self::answer(MappedInputs::NEUTRAL, HeroOutcome::Confirmed),
            Rejected => Self::answer(MappedInputs::NEUTRAL, HeroOutcome::Rejected),
            AwaitMenu => match read.status {
                SpecialLw | Other => Self::answer(MappedInputs::NEUTRAL, HeroOutcome::Pending),
                Failure => self.reject(),
                Cancel => Err(HeroError::Unavailable),
                Start => self.reject(),
                Select => {
                    let Some((index, commands)) = Self::menu(read)? else {
                        return Self::answer(MappedInputs::NEUTRAL, HeroOutcome::Pending);
                    };
                    let Some(target) = commands.iter().position(|command| *command == read.ice_sword_command) else {
                        self.phase = CancelPending;
                        return Self::answer(Self::input(HERO_INPUT_GUARD, 0), HeroOutcome::Pending);
                    };
                    if index == target {
                        self.phase = ConfirmPending { target, commands };
                        Self::answer(Self::input(HERO_INPUT_SPECIAL, 0), HeroOutcome::Pending)
                    } else {
                        self.phase = Navigating { target, commands, from: index };
                        Self::answer(Self::input(0, -120), HeroOutcome::Pending)
                    }
                }
            },
            Navigating { target, commands, from } => match read.status {
                Failure => self.reject(),
                Select => {
                    let index = Self::checked_menu(read, commands)?;
                    if index == from {
                        Self::answer(MappedInputs::NEUTRAL, HeroOutcome::Pending)
                    } else if index == (from + 1) % 4 {
                        self.phase = AfterMove { target, commands, index };
                        Self::answer(MappedInputs::NEUTRAL, HeroOutcome::Pending)
                    } else {
                        Err(HeroError::Unavailable)
                    }
                }
                _ => Err(HeroError::Unavailable),
            },
            AfterMove { target, commands, index: expected } => match read.status {
                Failure => self.reject(),
                Select => {
                    let index = Self::checked_menu(read, commands)?;
                    if index != expected { return Err(HeroError::Unavailable); }
                    if index == target {
                        self.phase = ConfirmPending { target, commands };
                        Self::answer(Self::input(HERO_INPUT_SPECIAL, 0), HeroOutcome::Pending)
                    } else {
                        self.phase = Navigating { target, commands, from: index };
                        Self::answer(Self::input(0, -120), HeroOutcome::Pending)
                    }
                }
                _ => Err(HeroError::Unavailable),
            },
            ConfirmPending { target, commands } => match read.status {
                Failure => self.reject(),
                Select => {
                    if read.window_state == Some(4) {
                        return Self::answer(MappedInputs::NEUTRAL, HeroOutcome::Pending);
                    }
                    let index = Self::checked_menu(read, commands)?;
                    if index != target { return Err(HeroError::Unavailable); }
                    Self::answer(MappedInputs::NEUTRAL, HeroOutcome::Pending)
                }
                Start => {
                    if read.success_sp == Some(true)
                        && read.active_command == Some(read.ice_sword_command)
                    {
                        self.phase = Confirmed;
                        Self::answer(MappedInputs::NEUTRAL, HeroOutcome::Confirmed)
                    } else {
                        self.reject()
                    }
                }
                Other => Self::answer(MappedInputs::NEUTRAL, HeroOutcome::Pending),
                _ => Err(HeroError::Unavailable),
            },
            CancelPending => match read.status {
                Cancel => Self::answer(MappedInputs::NEUTRAL, HeroOutcome::Cancelled),
                Failure => self.reject(),
                Select => match read.window_state {
                    Some(1) => Self::answer(MappedInputs::NEUTRAL, HeroOutcome::Pending),
                    Some(HERO_WINDOW_OPEN) => {
                        let _ = Self::menu(read)?;
                        Self::answer(MappedInputs::NEUTRAL, HeroOutcome::Pending)
                    }
                    _ => Err(HeroError::Unavailable),
                },
                _ => Err(HeroError::Unavailable),
            },
        }
    }
}

/// Judge INIT clears FIX_PANEL; ACMD fixes the natural choice at frame16.
/// A stale fixed flag or an unguarded/default read never classifies an outcome.
pub fn judge_outcome(saw_unfixed: bool, fixed: bool, kind: i32) -> Result<Option<bool>, &'static str> {
    if !saw_unfixed || !fixed { return Ok(None); }
    if !(0..=8).contains(&kind) { return Err("invalid native Judge choice"); }
    Ok(Some(kind == 7))
}

/// Native Pickel entry phases, not elapsed-frame or capture predictions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrolleyStatus { Special, Ride, Drive, Jump, Failed, Other }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrolleyPhase { AwaitRide, Ride, Drive, JumpRequested, Ejected }

/// The current CPU-owned article -> object id -> active BOMA chain.
/// Runtime must obtain all fields through granted native reads; Article* is not BOMA.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnedTrolley {
    pub cpu: usize,
    pub article: usize,
    pub object_id: u32,
    pub accessor: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrolleyRead {
    /// All native grants/constants and the CPU-owned active article chain were checked.
    pub readers_ready: bool,
    pub cpu: usize,
    pub cart: Option<OwnedTrolley>,
    /// None means no guarded native read, never an inferred/default zero.
    pub jump_disable: Option<i32>,
}

/// Per-attempt state. It supplies only mapped input: no article/resource/status writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrolleyEntry {
    cpu: usize,
    phase: TrolleyPhase,
    cart: Option<OwnedTrolley>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrolleyError { Unavailable }
impl TrolleyEntry {
    pub fn new(cpu: usize) -> Option<Self> {
        (cpu != 0).then_some(Self { cpu, phase: TrolleyPhase::AwaitRide, cart: None })
    }
    fn owned(&self, read: TrolleyRead) -> Result<OwnedTrolley, TrolleyError> {
        let cart = read.cart.ok_or(TrolleyError::Unavailable)?;
        if cart.cpu != self.cpu || cart.article == 0 || cart.accessor == 0
            || self.cart.is_some_and(|old| old != cart)
        { return Err(TrolleyError::Unavailable); }
        Ok(cart)
    }
    fn drive(&mut self, read: TrolleyRead) -> Result<MappedInputs, TrolleyError> {
        let cart = self.owned(read)?;
        let counter = read.jump_disable.ok_or(TrolleyError::Unavailable)?;
        if self.phase == TrolleyPhase::JumpRequested && counter != 0 {
            return Err(TrolleyError::Unavailable);
        }
        self.cart = Some(cart);
        if counter == 0 {
            self.phase = TrolleyPhase::JumpRequested;
            // pickel4a340..4a3b8 consumes JumpButton only at native exact zero.
            // Keep one held edge until actual JUMP; never pulse by callback count.
            Ok(MappedInputs { buttons: 0x4, ..MappedInputs::NEUTRAL })
        } else {
            self.phase = TrolleyPhase::Drive;
            Ok(MappedInputs::NEUTRAL)
        }
    }
    /// Call only for the selected current CPU. Err requires ordinary revoke/failure.
    /// Ejected confirms native JUMP only; it is not empty-cart contact or a seed.
    /// This does not extend the runtime's existing WAIT-without-capture policy.
    pub fn observe(&mut self, status: TrolleyStatus, read: TrolleyRead)
        -> Result<MappedInputs, TrolleyError>
    {
        if !read.readers_ready { return Err(TrolleyError::Unavailable); }
        if read.cpu != self.cpu { return Err(TrolleyError::Unavailable); }
        if status == TrolleyStatus::Failed { return Err(TrolleyError::Unavailable); }
        match (self.phase, status) {
            (TrolleyPhase::AwaitRide, TrolleyStatus::Special | TrolleyStatus::Other) => {
                Ok(MappedInputs::NEUTRAL)
            }
            (TrolleyPhase::AwaitRide | TrolleyPhase::Ride, TrolleyStatus::Ride) => {
                self.phase = TrolleyPhase::Ride;
                Ok(MappedInputs::NEUTRAL)
            }
            (TrolleyPhase::Ride | TrolleyPhase::Drive | TrolleyPhase::JumpRequested,
                TrolleyStatus::Drive) => self.drive(read),
            (TrolleyPhase::JumpRequested, TrolleyStatus::Jump) => {
                self.owned(read)?;
                self.phase = TrolleyPhase::Ejected;
                Ok(MappedInputs::NEUTRAL)
            }
            (TrolleyPhase::Ejected, _) => Ok(MappedInputs::NEUTRAL),
            _ => Err(TrolleyError::Unavailable),
        }
    }
}

/// Ganondorf's aerial side special never calls end_clatter. On landing the attacker's fall
/// (ganon 0x150fc→0x15124) enters SPECIAL_AIR_S_END and the victim leaves the clatter hold
/// for CATCHED_AIR_END_GANON, whose Main (common 0xd7aa0) reads no clatter and only falls.
/// Exactly that pair is the native end before escape; the caller keeps escape and
/// exhaustion first, and every other unlinked status stays an interruption.
pub fn native_landing_end(plan: Plan, is_victim: impl Fn(C) -> bool, is_attacker: impl Fn(C) -> bool) -> bool {
    plan == Plan::AirSideSpecial && is_victim(C::CatchedAirEndGanon) && is_attacker(C::GanonSpecialAirSEnd)
}

/// Down throws and Cargo first acquire an ordinary grab, without grading that preparatory hold.
pub fn preparatory_grab(plan: Plan) -> bool {
    matches!(plan, Plan::DownThrow | Plan::Cargo)
}

/// Cargo's one forward command while the attacker is in actual CATCH_WAIT: a full
/// left-stick tilt toward the attacker's native facing, nothing else. An unusable
/// facing is not guessed.
pub fn cargo_throw(attacker_lr: f32) -> Option<MappedInputs> {
    if !attacker_lr.is_finite() || attacker_lr == 0.0 {
        return None;
    }
    Some(MappedInputs { lstick_x: if attacker_lr > 0.0 { 120 } else { -120 }, ..MappedInputs::NEUTRAL })
}

/// Cargo's scoring seed must come from the victim's native shouldered statuses, never
/// from the preparatory grab's capture statuses, which State::Grab also holds.
pub fn cargo_holding(is: impl Fn(C) -> bool) -> bool {
    is(C::ClatterShoulderedDonkeyStart) || is(C::ClatterShoulderedDonkey)
}

/// A preparatory native capture initializes its own timer before the selected scoring phase.
pub fn preparatory(plan: Plan) -> bool {
    preparatory_grab(plan) || plan == Plan::Egg
}

/// Row 40 (Olimar's ordinary grab) is graded with exactly one Pikmin: the native add-clatter
/// callback needs HOLD_NUM > 1 (OPUS-DESIGN-REVIEW-M3-row40-delegates §2), so this line adds none.
/// The colour is a fixed one (red).
pub const ONE_PIKMIN: [Option<i32>; 3] = [Some(0), None, None];

pub fn one_pikmin_grab(scenario: Scenario) -> bool {
    scenario.attacker_kind == super::moves::OLIMAR && scenario.plan == Plan::Grab
}

/// A settled read of Olimar's line (held first, then following) shows exactly one Pikmin.
pub fn one_pikmin(line: [Option<i32>; 3]) -> bool {
    matches!(line, [Some(_), None, None])
}

/// Keep ownership of Egg's tongue capture while waiting for its separate native egg initializer.
pub fn preparatory_holding(plan: Plan, is: impl Fn(C) -> bool) -> bool {
    plan == Plan::Egg && is(C::ClatterCaptureYoshi)
}

/// Full mapped records reuse recording::inject_values and the guarded AI controller writer.
/// These are mapped ATTACK/SPECIAL bits, not physical Joy-Con button masks.
pub fn controls(plan: Plan, caught: bool) -> MappedInputs {
    let (buttons, lstick_y) = match plan {
        Plan::DownThrow if caught => (1, -120),
        Plan::UpSpecial if caught => (2, 0),
        Plan::Pin if caught => (1, 0),
        Plan::Inhale | Plan::NeutralSpecial | Plan::IceMagic => (2, 0),
        Plan::Freeze if !caught => (2, 0),
        Plan::Jab3 => (1, 0),
        _ => (0, 0),
    };
    MappedInputs {
        buttons,
        lstick_y,
        ..MappedInputs::NEUTRAL
    }
}

/// States are resolved from the project's existing native constant names. A setup grab is
/// deliberately absent from Bury so its clatter seed can never become the throw's grade.
pub fn holding(state: State, is: impl Fn(C) -> bool) -> bool {
    use C::*;
    let statuses: &[C] = match state {
        State::Grab => &[
            ClatterCapturePulled,
            ClatterCaptureWait,
            ClatterCaptureDamage,
            ClatterCapturePulledYoshi,
            ClatterCaptureWaitYoshi,
            ClatterCaptureDamageYoshi,
            ClatterShoulderedDonkeyStart,
            ClatterShoulderedDonkey,
            ClatterCatchedAirGanon,
            ClatterCatchedAirFallGanon,
            ClatterBittenWario,
            ClatterClungDiddy,
            ClatterClungDamageDiddy,
            ClatterCatchedReflet,
            ClatterKamuiPierce,
            ClatterSwingGaogaenCatched,
            ClatterDraggedRidley,
            ClatterCatchedPickelTrolley,
        ],
        State::Bury => &[ClatterBury, ClatterBuryWait],
        State::Frozen => &[ClatterIce],
        State::Egg => &[ClatterYoshiEgg],
        State::Inhale => &[ClatterSwallowed, ClatterSwallowedCapture],
    };
    statuses.iter().copied().any(is)
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
        use super::*;
        use crate::model::mashout::moves::Config;

        #[test]
        fn trolley_place_spacing_clears_occupied_and_reaches_vacant() {
            let trolley = super::super::moves::CATALOG.iter().find(|row| row.id == 97).unwrap();
            assert_eq!(trolley.plan, Plan::Trolley);
            let occupied = 35.200_004_577_6_f64 + 6.950_677_514_1;
            let vacant = 91.863_304_138_2_f64 + 4.326_402_187_3;
            assert!(6.0 <= occupied, "the old placement must reproduce occupied overlap");
            let spacing = f64::from(placement_spacing(*trolley));
            assert!(spacing >= occupied + 3.0 && spacing <= vacant - 1.0,
                "row 97 spacing {spacing} is outside the tolerated interval");
        }

        #[test]
        fn trolley_place_refuses_incomplete_native_floor() {
            let place = |p1, face, floor| {
                trolley_floor_runway(p1, p1[0] + 45.150_684 * face, face, floor)
            };
            let floor = [[-85., 0.], [85., 0.]];
            assert!(place([-40., 0.], 1., floor));
            assert!(place([40., 0.], -1., [floor[1], floor[0]]));
            assert!(!place([60., 0.], 1., floor)); // CPU Place falls beyond the right edge.
            assert!(place([-50., 0.], 1., floor)); // The unused far cart tail can leave the floor.
            assert!(!place([-80., 0.], 1., floor)); // Capture-side room ends before the margin.
            assert!(trolley_solid_floor(0x0800));
            for flags in [0x0000, 0x2000, 0x2800] {
                assert!(!trolley_solid_floor(flags));
            }
            assert!(!place([-40., 3.], 1., floor)); // A lower surface cannot support this pose.
            assert!(!place([-40., 0.], 1., [[-85., 0.], [85., 1.]]));
            assert!(!place([-40., 0.], 1., [[f32::NAN, 0.], [85., 0.]]));
            for scenario in super::super::moves::CATALOG.iter().filter(|row| row.id != 97) {
                let unchanged = if scenario.evidence == "B04" {
                    14.2
                } else if scenario.plan == Plan::Flip {
                    41.123_188
                } else {
                    6.0
                };
                assert_eq!(placement_spacing(*scenario), unchanged, "row {}", scenario.id);
            }
        }

        #[test]
        fn one_pikmin_line_is_exactly_one_held_or_following() {
            assert!(one_pikmin(ONE_PIKMIN));
            for colour in 0..5 {
                assert!(one_pikmin([Some(colour), None, None]));
                for other in [[Some(colour), Some(colour), None], [Some(colour), Some(0), Some(1)],
                    [None, Some(colour), None], [None, None, Some(colour)], [Some(colour), None, Some(colour)]] {
                    assert!(!one_pikmin(other), "{other:?}");
                }
            }
            assert!(!one_pikmin([None; 3]));
            let olimar = super::super::moves::CATALOG[40];
            assert!(one_pikmin_grab(olimar));
            assert!(super::super::moves::CATALOG.iter().filter(|s| one_pikmin_grab(**s)).map(|s| s.id).eq([40]));
        }

        #[test]
        fn trail_blizzard_provider_is_closed_only_after_n2_with_no_live_ice() {
            assert!(trail_blizzard_closed(0, 486, false));
            assert!(!trail_blizzard_closed(486, 486, false), "still casting Blizzard");
            assert!(!trail_blizzard_closed(0, 486, true), "a live Ice shard can still re-freeze");
            assert!(!trail_blizzard_closed(486, 486, true));
        }

        #[test]
        fn trail_fire_cycle_fails_closed_on_unexpected_blizzard_without_panicking() {
            let mut actor = TrailMagicEntry::new(TrailMagicKind::Fire).unwrap();
            for status in [TrailMagicStatus::SpecialN, TrailMagicStatus::N1, TrailMagicStatus::N1Shoot, TrailMagicStatus::N1End] {
                assert_eq!(actor.observe(status, TrailMagicKind::Fire, false), Ok(TrailMagicProgress::Pending));
            }
            // A Blizzard kind at the end of the Fire cast is unexpected native state: an error, never a panic.
            assert_eq!(actor.observe(TrailMagicStatus::Wait, TrailMagicKind::Blizzard, true), Err(TrailMagicError::Unavailable));
            assert_eq!(TrailMagicEntry::new(TrailMagicKind::Blizzard), Err(TrailMagicError::Unavailable));
        }

        #[test]
        fn throw_setup_is_not_the_scored_state() {
            assert!(!holding(State::Bury, |c| c == C::ClatterCaptureWait));
            assert!(holding(State::Bury, |c| c == C::ClatterBuryWait));
            assert!(preparatory_grab(Plan::DownThrow));
            assert!(!preparatory_grab(Plan::Grab));
            let input = controls(Plan::DownThrow, true);
            assert_eq!((input.buttons, input.lstick_x, input.lstick_y), (1, 0, -120));
            assert_eq!(controls(Plan::DownThrow, false), MappedInputs::NEUTRAL);
        }

        #[test]
        fn cargo_is_a_preparatory_real_catch_with_one_facing_forward_command() {
            let cargo = super::super::moves::CATALOG[89];
            assert_eq!((cargo.id, cargo.attacker_kind, cargo.plan, cargo.grab), (89, 0x01, Plan::Cargo, 0));
            assert_eq!(start(cargo), Some("FIGHTER_STATUS_KIND_CATCH"));
            assert!(preparatory_grab(Plan::Cargo) && preparatory(Plan::Cargo));
            assert!(!preparatory_holding(Plan::Cargo, |_| true));
            assert_eq!(controls(Plan::Cargo, false), MappedInputs::NEUTRAL);
            assert_eq!(controls(Plan::Cargo, true), MappedInputs::NEUTRAL);
            for (lr, x) in [(1.0, 120), (-1.0, -120), (0.25, 120), (-0.25, -120)] {
                assert_eq!(cargo_throw(lr), Some(MappedInputs { lstick_x: x, ..MappedInputs::NEUTRAL }));
            }
            for lr in [0.0, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                assert_eq!(cargo_throw(lr), None);
            }
            assert!(cargo_holding(|c| c == C::ClatterShoulderedDonkeyStart));
            assert!(cargo_holding(|c| c == C::ClatterShoulderedDonkey));
            for capture in [C::ClatterCapturePulled, C::ClatterCaptureWait, C::ClatterCaptureDamage] {
                assert!(holding(State::Grab, |c| c == capture));
                assert!(!cargo_holding(|c| c == capture));
            }
            // Other plans keep their entries.
            for plan in [Plan::Grab, Plan::Whip, Plan::AirSideSpecial, Plan::Inhale] {
                assert!(!preparatory_grab(plan));
            }
        }

        #[test]
        fn ganon_native_landing_end_needs_the_exact_pair_and_plan() {
            let victim = |want: C| move |c: C| c == want;
            assert!(native_landing_end(Plan::AirSideSpecial, victim(C::CatchedAirEndGanon), victim(C::GanonSpecialAirSEnd)));
            for (v, a) in [
                (C::ClatterCatchedAirFallGanon, C::GanonSpecialAirSEnd),
                (C::CatchedAirEndGanon, C::StatusSpecialS),
                (C::ClatterCaptureWait, C::GanonSpecialAirSEnd),
            ] {
                assert!(!native_landing_end(Plan::AirSideSpecial, victim(v), victim(a)), "{v:?} {a:?}");
            }
            for plan in [Plan::Grab, Plan::Drag, Plan::Cargo, Plan::Whip, Plan::Inhale] {
                assert!(!native_landing_end(plan, victim(C::CatchedAirEndGanon), victim(C::GanonSpecialAirSEnd)));
            }
            // The after-landing status is not a clatter hold.
            assert!(!holding(State::Grab, |c| c == C::CatchedAirEndGanon));
            assert!(holding(State::Grab, |c| c == C::ClatterCatchedAirFallGanon));
        }

        #[test]
        fn ordinary_variants_request_distinct_real_move_statuses() {
            let mut move_ = Config::default().resolve(0, 0).unwrap();
            for (grab, status) in ["CATCH", "CATCH_DASH", "CATCH_TURN"].iter().enumerate() {
                move_.grab = grab;
                assert_eq!(start(move_).unwrap(), format!("FIGHTER_STATUS_KIND_{status}"));
            }
        }

        #[test]
        fn pin_uses_real_side_special_then_an_attack_edge_without_direction() {
            let pin = super::super::moves::CATALOG[94];
            assert_eq!(start(pin), Some("FIGHTER_STATUS_KIND_SPECIAL_S"));
            assert_eq!(controls(pin.plan, false), MappedInputs::NEUTRAL);
            let strike = controls(pin.plan, true);
            assert_eq!(strike.buttons, 1);
            assert_eq!((strike.lstick_x, strike.lstick_y, strike.rstick_x, strike.rstick_y), (0, 0, 0, 0));
            assert!(holding(State::Grab, |c| c == C::ClatterKamuiPierce));
            assert!(!holding(State::Grab, |c| c == C::StatusSpecialS));
        }

        #[test]
        fn judge_requires_a_fresh_fixed_natural_choice() {
            let judge = super::super::moves::CATALOG[111];
            assert_eq!(start(judge), Some("FIGHTER_STATUS_KIND_SPECIAL_S"));
            assert_eq!(controls(Plan::Judge, false), MappedInputs::NEUTRAL);
            assert_eq!(controls(Plan::Judge, true), MappedInputs::NEUTRAL);
            assert_eq!(judge_outcome(false, true, 7), Ok(None));
            assert_eq!(judge_outcome(true, false, 7), Ok(None));
            for kind in 0..=8 {
                assert_eq!(judge_outcome(true, true, kind), Ok(Some(kind == 7)));
            }
            assert!(judge_outcome(true, true, -1).is_err());
            assert!(judge_outcome(true, true, 9).is_err());
        }

        fn trolley_read(counter: Option<i32>) -> TrolleyRead {
            TrolleyRead { readers_ready: true, cpu: 10,
                cart: Some(OwnedTrolley { cpu: 10, article: 20, object_id: 30, accessor: 40 }),
                jump_disable: counter }
        }
        fn trolley_riding() -> TrolleyEntry {
            let mut entry = TrolleyEntry::new(10).unwrap();
            assert_eq!(entry.observe(TrolleyStatus::Ride, trolley_read(None)), Ok(MappedInputs::NEUTRAL));
            entry
        }
        #[test]
        fn trolley_start_is_real_side_special_and_only_victim_status_can_hold() {
            let cart = super::super::moves::CATALOG[97];
            assert_eq!(cart.attacker_kind, 0x58);
            assert_eq!(cart.plan, Plan::Trolley);
            assert_eq!(start(cart), Some("FIGHTER_STATUS_KIND_SPECIAL_S"));
            assert_eq!(controls(Plan::Trolley, false), MappedInputs::NEUTRAL);
            assert_eq!(controls(Plan::Trolley, true), MappedInputs::NEUTRAL);
            assert!(holding(State::Grab, |c| c == C::ClatterCatchedPickelTrolley));
            assert!(!holding(State::Grab, |c| c == C::PickelTrolleyRide));
            assert!(!holding(State::Grab, |c| c == C::PickelTrolleyJump));
        }
        #[test]
        fn trolley_real_phases_supply_one_jump_edge_then_release_on_actual_jump() {
            let mut entry = TrolleyEntry::new(10).unwrap();
            let mut inputs = Vec::new();
            let mut pending = trolley_read(None);
            pending.cart = None; // native creation need not have produced it yet
            inputs.push(entry.observe(TrolleyStatus::Special, pending).unwrap());
            inputs.push(entry.observe(TrolleyStatus::Ride, pending).unwrap());
            inputs.push(entry.observe(TrolleyStatus::Drive, trolley_read(Some(2))).unwrap());
            assert_eq!(entry.phase, TrolleyPhase::Drive);
            for _ in 0..3 {
                inputs.push(entry.observe(TrolleyStatus::Drive, trolley_read(Some(0))).unwrap());
            }
            assert_eq!(entry.phase, TrolleyPhase::JumpRequested);
            inputs.push(entry.observe(TrolleyStatus::Jump, trolley_read(None)).unwrap());
            assert_eq!(entry.phase, TrolleyPhase::Ejected);
            inputs.push(entry.observe(TrolleyStatus::Other, trolley_read(None)).unwrap());
            assert_eq!(inputs.iter().map(|i| i.buttons).collect::<Vec<_>>(), vec![0,0,0,4,4,4,0,0]);
            assert_eq!(inputs.windows(2).filter(|w| w[0].buttons == 0 && w[1].buttons == 4).count(), 1);
            assert!(inputs.iter().all(|i| {
                let neutral = MappedInputs { buttons: i.buttons, ..MappedInputs::NEUTRAL };
                *i == neutral
            }));
        }
        #[test]
        fn trolley_never_jumps_in_ride_or_without_observed_ride() {
            let mut entry = TrolleyEntry::new(10).unwrap();
            for _ in 0..3 {
                assert_eq!(entry.observe(TrolleyStatus::Ride, trolley_read(Some(0))), Ok(MappedInputs::NEUTRAL));
            }
            let mut entry = TrolleyEntry::new(10).unwrap();
            assert!(entry.observe(TrolleyStatus::Drive, trolley_read(Some(0))).is_err());
            assert!(entry.observe(TrolleyStatus::Jump, trolley_read(Some(0))).is_err());
            assert!(TrolleyEntry::new(0).is_none());
        }
        #[test]
        fn trolley_zero_is_exact_and_missing_read_is_not_a_zero() {
            let mut entry = trolley_riding();
            for counter in [1, -1, i32::MIN, i32::MAX] {
                assert_eq!(entry.observe(TrolleyStatus::Drive, trolley_read(Some(counter))), Ok(MappedInputs::NEUTRAL));
            }
            let before = entry;
            assert!(entry.observe(TrolleyStatus::Drive, trolley_read(None)).is_err());
            assert_eq!(entry, before);
            assert_eq!(entry.observe(TrolleyStatus::Drive, trolley_read(Some(0))).unwrap().buttons, 4);
        }
        #[test]
        fn trolley_missing_grant_cpu_or_owned_cart_refuses_before_jump() {
            for bad in 0..6 {
                let mut entry = trolley_riding();
                let mut read = trolley_read(Some(0));
                match bad {
                    0 => read.readers_ready = false,
                    1 => read.cpu = 11,
                    2 => read.cart = None,
                    3 => read.cart.as_mut().unwrap().cpu = 11,
                    4 => read.cart.as_mut().unwrap().article = 0,
                    _ => read.cart.as_mut().unwrap().accessor = 0,
                }
                let before = entry;
                assert!(entry.observe(TrolleyStatus::Drive, read).is_err());
                assert_eq!(entry, before);
            }
        }
        #[test]
        fn trolley_replaced_identity_or_changed_gate_cannot_issue_another_edge() {
            for requested in [false, true] {
                for field in 0..4 {
                    let mut entry = trolley_riding();
                    entry.observe(TrolleyStatus::Drive, trolley_read(Some(if requested {0} else {2}))).unwrap();
                    let mut replaced = trolley_read(Some(0));
                    let cart = replaced.cart.as_mut().unwrap();
                    match field {
                        0 => cart.cpu += 1,
                        1 => cart.article += 1,
                        2 => cart.object_id += 1,
                        _ => cart.accessor += 1,
                    }
                    let before = entry;
                    assert!(entry.observe(TrolleyStatus::Drive, replaced).is_err());
                    assert_eq!(entry, before);
                }
            }
            let mut entry = trolley_riding();
            entry.observe(TrolleyStatus::Drive, trolley_read(Some(0))).unwrap();
            let before = entry;
            assert!(entry.observe(TrolleyStatus::Drive, trolley_read(Some(1))).is_err());
            assert_eq!(entry, before);
            let mut missing = trolley_read(Some(0)); missing.readers_ready = false;
            assert!(entry.observe(TrolleyStatus::Drive, missing).is_err());
        }
        #[test]
        fn trolley_creation_failure_and_unconfirmed_exit_keep_failure_boundary() {
            let mut entry = trolley_riding();
            assert!(entry.observe(TrolleyStatus::Failed, trolley_read(None)).is_err());
            entry.observe(TrolleyStatus::Drive, trolley_read(Some(2))).unwrap();
            assert!(entry.observe(TrolleyStatus::Other, trolley_read(Some(0))).is_err());
            assert!(entry.observe(TrolleyStatus::Jump, trolley_read(Some(0))).is_err());
            assert_eq!(entry.phase, TrolleyPhase::Drive);
        }

        #[test]
        fn each_state_requires_its_own_victim_status() {
            for (state, status) in [
                (State::Grab, C::ClatterBittenWario),
                (State::Bury, C::ClatterBury),
                (State::Frozen, C::ClatterIce),
                (State::Egg, C::ClatterYoshiEgg),
                (State::Inhale, C::ClatterSwallowedCapture),
            ] {
                for other in [State::Grab, State::Bury, State::Frozen, State::Egg, State::Inhale] {
                    assert_eq!(holding(other, |c| c == status), other == state);
                }
            }
            assert_eq!(controls(Plan::Inhale, false).buttons, 2);
            assert_eq!(controls(Plan::Egg, false), MappedInputs::NEUTRAL);
        }
    }
}
