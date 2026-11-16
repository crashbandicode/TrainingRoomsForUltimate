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
        Plan::SideSpecial | Plan::Cling | Plan::Drag | Plan::Whip | Plan::Roller => {
            "FIGHTER_STATUS_KIND_SPECIAL_S"
        }
        Plan::Egg | Plan::Inhale | Plan::NeutralSpecial | Plan::Freeze | Plan::IceShot => {
            "FIGHTER_STATUS_KIND_SPECIAL_N"
        }
        Plan::UpSpecial => "FIGHTER_STATUS_KIND_SPECIAL_HI",
        Plan::Drain | Plan::Blizzard => "FIGHTER_STATUS_KIND_SPECIAL_LW",
        Plan::DownTilt => "FIGHTER_STATUS_KIND_ATTACK_LW3",
        Plan::DownSmash => "FIGHTER_STATUS_KIND_ATTACK_LW4_START",
        Plan::Jab3 => "FIGHTER_STATUS_KIND_ATTACK",
        // These need their verified native sequence, loadout or meter setup before admission.
        _ => return None,
    })
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
