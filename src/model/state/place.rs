//! Placement rules of a load (SPEC-state-01 §5.3): the mirrored x, the stage centre offsets, the ledge
//! height adjustment per fighter, the damage a load writes. Pure functions over numbers.

use crate::model::cpu::rng::Rng;

use super::settings::{DamageMode, Mirroring};

/// Stage ids the mirror needs (the game's `StageID` enumeration, 13.0.5).
pub const STAGE_YOSHIS_ISLAND: i32 = 0x56;
pub const STAGE_SMASHVILLE: i32 = 0x5f;
pub const STAGE_TOWN_AND_CITY: i32 = 0x101;

/// The stage-centre offsets of the three asymmetric stages (SPEC-state-01 §5.3 step 3; open question 2:
/// to be measured in Eden).
pub fn stage_offset(stage_id: i32) -> f32 {
    match stage_id {
        STAGE_SMASHVILLE => 1.195,
        STAGE_TOWN_AND_CITY => 1.448,
        STAGE_YOSHIS_ISLAND => -1.053,
        _ => 0.0,
    }
}

/// The x a fighter is placed at: the saved x, or, with mirror sign -1, the saved x reflected across the
/// stage centre.
pub fn target_x(saved_x: f32, mirror_sign: f32, stage_id: i32) -> f32 {
    if mirror_sign < 0.0 {
        -(saved_x - stage_offset(stage_id))
    } else {
        saved_x
    }
}

/// The facing a fighter is placed with.
pub fn target_lr(saved_lr: f32, mirror_sign: f32) -> f32 {
    mirror_sign * saved_lr
}

/// The next mirror sign for a load request: None +1; Alternate the negation of the previous sign;
/// Random +1 or -1 with equal probability (SPEC-state-01 §5.1 step 2).
pub fn next_mirror_sign(mode: Mirroring, previous: f32, rng: &mut Rng) -> f32 {
    match mode {
        Mirroring::None => 1.0,
        Mirroring::Alternate => -previous,
        Mirroring::Random => {
            if rng.below(2) == 0 {
                1.0
            } else {
                -1.0
            }
        }
    }
}

/// Fighters whose ledge grab needs the saved ledge state raised (SPEC-state-01 §5.3 step 4). The
/// kinds are the game's `FIGHTER_KIND_*` values; Terry is `DOLLY`, Mythra `ELIGHT`.
pub fn ledge_raise(kind: i32) -> f32 {
    const DK: i32 = 0x1;
    const DEDEDE: i32 = 0x2a;
    const KROOL: i32 = 0x45;
    const SAMUS: i32 = 0x3;
    const SAMUSD: i32 = 0x4;
    const MEWTWO: i32 = 0x19;
    const TERRY: i32 = 0x55;
    const BOWSER: i32 = 0xf;
    const MYTHRA: i32 = 0x5b;
    const SNAKE: i32 = 0x22;
    match kind {
        DK | DEDEDE | KROOL => 6.0,
        SAMUS | SAMUSD | MEWTWO | TERRY => 4.0,
        BOWSER | MYTHRA | SNAKE => 2.0,
        _ => 0.0,
    }
}

/// The damage a load writes, or `None` to leave the respawn damage (0%). `Random` is uniform between
/// the lower and upper end of the role's range, in whatever order they are stored (SPEC-state-01 open
/// question 6: the CPU's ends were passed swapped upstream).
pub fn load_damage(mode: DamageMode, saved: f32, limits: (u8, u8), rng: &mut Rng) -> Option<f32> {
    match mode {
        DamageMode::Default => None,
        DamageMode::SaveState => Some(saved),
        DamageMode::Random => {
            let (low, high) = (f32::from(limits.0.min(limits.1)), f32::from(limits.0.max(limits.1)));
            let unit = rng.next_u32() as f32 / (u32::MAX as f32 + 1.0);
            Some(low + (high - low) * unit)
        }
    }
}
