//! The situation predicates shared by the CPU specs (SPEC-cpu-01 §3, "Situation predicates"). Pure
//! functions over the resolved constant table: a status, previous status or situation kind in, a bool
//! out. A predicate whose constants did not resolve is false.

use super::ids::{Table, C, CLATTER_STATUSES};

/// The situation of a fighter (`StatusModule::situation_kind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    Ground,
    Air,
    Cliff,
    Other,
}

impl Situation {
    /// Classify a raw situation kind with the game's constants.
    pub fn of(table: &Table, raw: i32) -> Self {
        if table.is(C::SituationGround, raw) {
            Situation::Ground
        } else if table.is(C::SituationAir, raw) {
            Situation::Air
        } else if table.is(C::SituationCliff, raw) {
            Situation::Cliff
        } else {
            Situation::Other
        }
    }
}

/// In hitstun: `DAMAGE .. DAMAGE_FALL`, end exclusive (the range contains tumble; callers that need
/// the difference test `in_tumble` first).
pub fn in_hitstun(t: &Table, status: i32) -> bool {
    t.below(C::StatusDamage, C::StatusDamageFall, status)
}

/// In tumble: `DAMAGE_FLY ..= DAMAGE_FALL`.
pub fn in_tumble(t: &Table, status: i32) -> bool {
    t.within(C::StatusDamageFly, C::StatusDamageFall, status)
}

/// In footstool: `TREAD_DAMAGE ..= TREAD_FALL`.
pub fn in_footstool(t: &Table, status: i32) -> bool {
    t.within(C::StatusTreadDamage, C::StatusTreadFall, status)
}

/// Shielding: `GUARD_ON ..= GUARD_DAMAGE`.
pub fn shielding(t: &Table, status: i32) -> bool {
    t.within(C::StatusGuardOn, C::StatusGuardDamage, status)
}

/// In shieldstun: `GUARD_DAMAGE`, or `GUARD_OFF` right after `GUARD_DAMAGE`.
pub fn in_shieldstun(t: &Table, status: i32, prev: i32) -> bool {
    t.is(C::StatusGuardDamage, status) || (t.is(C::StatusGuardOff, status) && t.is(C::StatusGuardDamage, prev))
}

/// In tech: `DOWN_STAND ..= DOWN_STAND_ATTACK` or `PASSIVE ..= PASSIVE_CEIL`.
pub fn in_tech(t: &Table, status: i32) -> bool {
    t.within(C::StatusDownStand, C::StatusDownStandAttack, status) || t.within(C::StatusPassive, C::StatusPassiveCeil, status)
}

/// In ledge trump: `CLIFF_ROBBED`.
pub fn in_ledge_trump(t: &Table, status: i32) -> bool {
    t.is(C::StatusCliffRobbed, status)
}

/// In landing: `LANDING ..= LANDING_DAMAGE_LIGHT`.
pub fn in_landing(t: &Table, status: i32) -> bool {
    t.within(C::StatusLanding, C::StatusLandingDamageLight, status)
}

/// In clatter: one of the 43 hold statuses.
pub fn in_clatter(t: &Table, status: i32) -> bool {
    CLATTER_STATUSES.iter().any(|c| t.is(*c, status))
}

/// Idle: `WAIT`.
pub fn idle(t: &Table, status: i32) -> bool {
    t.is(C::StatusWait, status)
}
