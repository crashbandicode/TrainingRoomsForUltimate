//! Native move entry, sharing the recorder's mapped-input format. No escape formula lives here.
use super::moves::{Plan, Scenario, State};
use crate::model::{cpu::ids::C, input_delay::MappedInputs};

/// The original move status is requested on the attacker only. Victim statuses and links are native.
pub fn start(move_: Scenario) -> Option<&'static str> {
    Some(match move_.plan {
        Plan::Grab | Plan::DownThrow => match move_.grab {
            1 => "FIGHTER_STATUS_KIND_CATCH_DASH",
            2 => "FIGHTER_STATUS_KIND_CATCH_TURN",
            _ => "FIGHTER_STATUS_KIND_CATCH",
        },
        Plan::AirSideSpecial => "FIGHTER_STATUS_KIND_JUMP_SQUAT",
        Plan::SideSpecial | Plan::Cling | Plan::Drag | Plan::Whip | Plan::Roller | Plan::Pin | Plan::Judge | Plan::Trolley => {
            "FIGHTER_STATUS_KIND_SPECIAL_S"
        }
        Plan::Egg | Plan::Inhale | Plan::NeutralSpecial | Plan::Freeze | Plan::IceShot => {
            "FIGHTER_STATUS_KIND_SPECIAL_N"
        }
        Plan::UpSpecial => "FIGHTER_STATUS_KIND_SPECIAL_HI",
        Plan::Drain | Plan::Blizzard | Plan::Flip => "FIGHTER_STATUS_KIND_SPECIAL_LW",
        Plan::DownTilt => "FIGHTER_STATUS_KIND_ATTACK_LW3",
        Plan::DownSmash => "FIGHTER_STATUS_KIND_ATTACK_LW4_START",
        Plan::Jab3 => "FIGHTER_STATUS_KIND_ATTACK",
        // These need their verified native sequence, loadout or meter setup before admission.
        _ => return None,
    })
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
            (TrolleyPhase::AwaitRide, TrolleyStatus::Special | TrolleyStatus::Other) =>
                Ok(MappedInputs::NEUTRAL),
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

/// Only down throws first acquire an ordinary grab, without grading that preparatory hold.
pub fn preparatory_grab(plan: Plan) -> bool {
    plan == Plan::DownThrow
}

/// A preparatory native capture initializes its own timer before the selected scoring phase.
pub fn preparatory(plan: Plan) -> bool {
    preparatory_grab(plan) || plan == Plan::Egg
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
        Plan::Inhale | Plan::NeutralSpecial => (2, 0),
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
