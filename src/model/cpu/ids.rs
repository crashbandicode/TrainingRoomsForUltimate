//! The game constants the CPU behaviours use, by name, and the table that holds their values.
//!
//! No numeric id is written down here: the runtime asks the game's own constant table for each name
//! at run time (`hash::lua_bind_hash`, `main+0x3728e10`) and stores the answers in a `Table`. A name
//! the game does not know stays unresolved (`UNRESOLVED`, which matches no status) and is listed so
//! the install line can say which feature lost it. Each package that needs more names adds them here;
//! the order of the list is the order of `C`, nothing else depends on it.

use super::hash::lua_bind_hash;

macro_rules! constants {
    ($($variant:ident = $name:literal,)*) => {
        /// One game constant.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(u16)]
        pub enum C { $($variant),* }

        /// The identifiers, in `C` order.
        pub const NAMES: &[&str] = &[$($name),*];
    };
}

constants! {
    // Situation kinds.
    SituationGround = "SITUATION_KIND_GROUND",
    SituationAir = "SITUATION_KIND_AIR",
    SituationCliff = "SITUATION_KIND_CLIFF",
    // Fighter kinds (Ice Climbers: the second body is Nana).
    KindPopo = "FIGHTER_KIND_POPO",
    KindNana = "FIGHTER_KIND_NANA",
    // Status kinds of the situation predicates (SPEC-cpu-01 §3).
    StatusNone = "FIGHTER_STATUS_KIND_NONE",
    StatusWait = "FIGHTER_STATUS_KIND_WAIT",
    StatusDamage = "FIGHTER_STATUS_KIND_DAMAGE",
    StatusDamageFly = "FIGHTER_STATUS_KIND_DAMAGE_FLY",
    StatusDamageFall = "FIGHTER_STATUS_KIND_DAMAGE_FALL",
    StatusTreadDamage = "FIGHTER_STATUS_KIND_TREAD_DAMAGE",
    StatusTreadFall = "FIGHTER_STATUS_KIND_TREAD_FALL",
    StatusGuardOn = "FIGHTER_STATUS_KIND_GUARD_ON",
    StatusGuard = "FIGHTER_STATUS_KIND_GUARD",
    StatusGuardOff = "FIGHTER_STATUS_KIND_GUARD_OFF",
    StatusGuardDamage = "FIGHTER_STATUS_KIND_GUARD_DAMAGE",
    StatusDownStand = "FIGHTER_STATUS_KIND_DOWN_STAND",
    StatusDownStandAttack = "FIGHTER_STATUS_KIND_DOWN_STAND_ATTACK",
    StatusPassive = "FIGHTER_STATUS_KIND_PASSIVE",
    StatusPassiveCeil = "FIGHTER_STATUS_KIND_PASSIVE_CEIL",
    StatusCliffRobbed = "FIGHTER_STATUS_KIND_CLIFF_ROBBED",
    StatusLanding = "FIGHTER_STATUS_KIND_LANDING",
    StatusLandingDamageLight = "FIGHTER_STATUS_KIND_LANDING_DAMAGE_LIGHT",
    // Ledge (SPEC-cpu-03).
    StatusCliffCatch = "FIGHTER_STATUS_KIND_CLIFF_CATCH",
    StatusCliffWait = "FIGHTER_STATUS_KIND_CLIFF_WAIT",
    StatusCliffClimb = "FIGHTER_STATUS_KIND_CLIFF_CLIMB",
    StatusCliffEscape = "FIGHTER_STATUS_KIND_CLIFF_ESCAPE",
    StatusCliffJump1 = "FIGHTER_STATUS_KIND_CLIFF_JUMP1",
    StatusCliffAttack = "FIGHTER_STATUS_KIND_CLIFF_ATTACK",
    // Tech, mistech and hide tech (SPEC-cpu-04).
    StatusDown = "FIGHTER_STATUS_KIND_DOWN",
    StatusDamageFlyReflectD = "FIGHTER_STATUS_KIND_DAMAGE_FLY_REFLECT_D",
    StatusDamageFlyReflectLr = "FIGHTER_STATUS_KIND_DAMAGE_FLY_REFLECT_LR",
    StatusDamageFlyReflectU = "FIGHTER_STATUS_KIND_DAMAGE_FLY_REFLECT_U",
    StatusPassiveFb = "FIGHTER_STATUS_KIND_PASSIVE_FB",
    StatusPassiveWall = "FIGHTER_STATUS_KIND_PASSIVE_WALL",
    StatusPassiveWallJump = "FIGHTER_STATUS_KIND_PASSIVE_WALL_JUMP",
    StatusStopWall = "FIGHTER_STATUS_KIND_STOP_WALL",
    StatusStopCeil = "FIGHTER_STATUS_KIND_STOP_CEIL",
    StatusDownWait = "FIGHTER_STATUS_KIND_DOWN_WAIT",
    StatusDownWaitContinue = "FIGHTER_STATUS_KIND_DOWN_WAIT_CONTINUE",
    StatusDownStandFb = "FIGHTER_STATUS_KIND_DOWN_STAND_FB",
    StatusLayDown = "FIGHTER_STATUS_KIND_LAY_DOWN",
    StatusSlipWait = "FIGHTER_STATUS_KIND_SLIP_WAIT",
    StatusSlipStand = "FIGHTER_STATUS_KIND_SLIP_STAND",
    StatusSlipStandAttack = "FIGHTER_STATUS_KIND_SLIP_STAND_ATTACK",
    StatusSlipStandF = "FIGHTER_STATUS_KIND_SLIP_STAND_F",
    StatusSlipStandB = "FIGHTER_STATUS_KIND_SLIP_STAND_B",
    TermPassive = "FIGHTER_STATUS_TRANSITION_TERM_ID_PASSIVE",
    TermPassiveWall = "FIGHTER_STATUS_TRANSITION_TERM_ID_PASSIVE_WALL",
    TermPassiveCeil = "FIGHTER_STATUS_TRANSITION_TERM_ID_PASSIVE_CEIL",
    WorkFloatCursorOffsetY = "FIGHTER_INSTANCE_WORK_ID_FLOAT_CURSOR_OFFSET_Y",
    QuakeKindNone = "CAMERA_QUAKE_KIND_NONE",
    // Shield and out-of-shield dodges (SPEC-cpu-05).
    StatusEscape = "FIGHTER_STATUS_KIND_ESCAPE",
    StatusEscapeF = "FIGHTER_STATUS_KIND_ESCAPE_F",
    StatusEscapeB = "FIGHTER_STATUS_KIND_ESCAPE_B",
    TermContEscape = "FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_ESCAPE",
    ButtonGuard = "CONTROL_PAD_BUTTON_GUARD",
    ButtonGuardHold = "CONTROL_PAD_BUTTON_GUARD_HOLD",
    // The clatter list (SPEC-cpu-01 §3 "in clatter").
    ClatterCapturePulled = "FIGHTER_STATUS_KIND_CAPTURE_PULLED",
    ClatterCaptureWait = "FIGHTER_STATUS_KIND_CAPTURE_WAIT",
    ClatterCaptureDamage = "FIGHTER_STATUS_KIND_CAPTURE_DAMAGE",
    ClatterShoulderedDonkeyStart = "FIGHTER_STATUS_KIND_SHOULDERED_DONKEY_START",
    ClatterShoulderedDonkey = "FIGHTER_STATUS_KIND_SHOULDERED_DONKEY",
    ClatterBury = "FIGHTER_STATUS_KIND_BURY",
    ClatterBuryWait = "FIGHTER_STATUS_KIND_BURY_WAIT",
    ClatterCaptureYoshi = "FIGHTER_STATUS_KIND_CAPTURE_YOSHI",
    ClatterYoshiEgg = "FIGHTER_STATUS_KIND_YOSHI_EGG",
    ClatterCapturePulledYoshi = "FIGHTER_STATUS_KIND_CAPTURE_PULLED_YOSHI",
    ClatterCaptureWaitYoshi = "FIGHTER_STATUS_KIND_CAPTURE_WAIT_YOSHI",
    ClatterCaptureDamageYoshi = "FIGHTER_STATUS_KIND_CAPTURE_DAMAGE_YOSHI",
    ClatterSwallowed = "FIGHTER_STATUS_KIND_SWALLOWED",
    ClatterSwallowedCapture = "FIGHTER_STATUS_KIND_SWALLOWED_CAPTURE",
    ClatterCatchedAirGanon = "FIGHTER_STATUS_KIND_CATCHED_AIR_GANON",
    ClatterCatchedAirFallGanon = "FIGHTER_STATUS_KIND_CATCHED_AIR_FALL_GANON",
    ClatterBind = "FIGHTER_STATUS_KIND_BIND",
    ClatterDamageSong = "FIGHTER_STATUS_KIND_DAMAGE_SONG",
    ClatterBittenWario = "FIGHTER_STATUS_KIND_BITTEN_WARIO",
    ClatterClungDiddy = "FIGHTER_STATUS_KIND_CLUNG_DIDDY",
    ClatterClungDamageDiddy = "FIGHTER_STATUS_KIND_CLUNG_DAMAGE_DIDDY",
    ClatterIce = "FIGHTER_STATUS_KIND_ICE",
    ClatterCatchedReflet = "FIGHTER_STATUS_KIND_CATCHED_REFLET",
    ClatterKamuiPierce = "FIGHTER_STATUS_KIND_KAMUI_PIERCE",
    ClatterSwingGaogaenCatched = "FIGHTER_STATUS_KIND_SWING_GAOGAEN_CATCHED",
    ClatterDraggedRidley = "FIGHTER_STATUS_KIND_DRAGGED_RIDLEY",
    ClatterCatchedPickelTrolley = "FIGHTER_STATUS_KIND_CATCHED_PICKEL_TROLLEY",
    ClatterSleep = "FIGHTER_STATUS_KIND_SLEEP",
    ClatterFurafura = "FIGHTER_STATUS_KIND_FURAFURA",
    ClatterGimmickFishCapture = "FIGHTER_STATUS_KIND_GIMMICK_FISH_CAPTURE",
    ClatterCaptureMasterhand = "FIGHTER_STATUS_KIND_CAPTURE_MASTERHAND",
    ClatterCaptureItem = "FIGHTER_STATUS_KIND_CAPTURE_ITEM",
    ClatterCaptureBeetle = "FIGHTER_STATUS_KIND_CAPTURE_BEETLE",
    ClatterCaptureBlackhole = "FIGHTER_STATUS_KIND_CAPTURE_BLACKHOLE",
    ClatterCaptureBeitcrane = "FIGHTER_STATUS_KIND_CAPTURE_BEITCRANE",
    ClatterCaptureKawasaki = "FIGHTER_STATUS_KIND_CAPTURE_KAWASAKI",
    ClatterCaptureDriver = "FIGHTER_STATUS_KIND_CAPTURE_DRIVER",
    ClatterCaptureMimikkyu = "FIGHTER_STATUS_KIND_CAPTURE_MIMIKKYU",
    ClatterCaptureClaptrap = "FIGHTER_STATUS_KIND_CAPTURE_CLAPTRAP",
    ClatterCaptureBossgalaga = "FIGHTER_STATUS_KIND_CAPTURE_BOSSGALAGA",
    ClatterCaptureNabbit = "FIGHTER_STATUS_KIND_CAPTURE_NABBIT",
    ClatterCaptureMastercore = "FIGHTER_STATUS_KIND_CAPTURE_MASTERCORE",
    ClatterCaptureWaitOctopus = "FIGHTER_STATUS_KIND_CAPTURE_WAIT_OCTOPUS",
    // Transition terms.
    TermContCliffAttack = "FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_CLIFF_ATTACK",
    TermContCliffClimb = "FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_CLIFF_CLIMB",
    // Work ids.
    WorkFlagCatchCliff = "FIGHTER_INSTANCE_WORK_ID_FLAG_CATCH_CLIFF",
    WorkFlagDisableEscapePenalty = "FIGHTER_INSTANCE_WORK_ID_FLAG_DISABLE_ESCAPE_PENALTY",
    WorkFloatVecorCorrectStickX = "FIGHTER_STATUS_DAMAGE_WORK_FLOAT_VECOR_CORRECT_STICK_X",
    WorkFloatVecorCorrectStickY = "FIGHTER_STATUS_DAMAGE_WORK_FLOAT_VECOR_CORRECT_STICK_Y",
    // Effect attributes of the clatter flash (SPEC-cpu-02 §4).
    EffectNoJointScale = "EFFECT_SUB_ATTRIBUTE_NO_JOINT_SCALE",
    EffectFollow = "EFFECT_SUB_ATTRIBUTE_FOLLOW",
    EffectConcludeStatus = "EFFECT_SUB_ATTRIBUTE_CONCLUDE_STATUS",
}

/// Number of constants.
pub const COUNT: usize = NAMES.len();

/// The value of a constant the game did not answer for. No status, term or work id is this value.
pub const UNRESOLVED: i32 = i32::MIN;

/// The statuses of the clatter list, in the spec's order.
pub const CLATTER_STATUSES: [C; 43] = [
    C::ClatterCapturePulled,
    C::ClatterCaptureWait,
    C::ClatterCaptureDamage,
    C::ClatterShoulderedDonkeyStart,
    C::ClatterShoulderedDonkey,
    C::ClatterBury,
    C::ClatterBuryWait,
    C::ClatterCaptureYoshi,
    C::ClatterYoshiEgg,
    C::ClatterCapturePulledYoshi,
    C::ClatterCaptureWaitYoshi,
    C::ClatterCaptureDamageYoshi,
    C::ClatterSwallowed,
    C::ClatterSwallowedCapture,
    C::ClatterCatchedAirGanon,
    C::ClatterCatchedAirFallGanon,
    C::ClatterBind,
    C::ClatterDamageSong,
    C::ClatterBittenWario,
    C::ClatterClungDiddy,
    C::ClatterClungDamageDiddy,
    C::ClatterIce,
    C::ClatterCatchedReflet,
    C::ClatterKamuiPierce,
    C::ClatterSwingGaogaenCatched,
    C::ClatterDraggedRidley,
    C::ClatterCatchedPickelTrolley,
    C::ClatterSleep,
    C::ClatterFurafura,
    C::ClatterGimmickFishCapture,
    C::ClatterCaptureMasterhand,
    C::ClatterCaptureItem,
    C::ClatterCaptureBeetle,
    C::ClatterCaptureBlackhole,
    C::ClatterCaptureBeitcrane,
    C::ClatterCaptureKawasaki,
    C::ClatterCaptureDriver,
    C::ClatterCaptureMimikkyu,
    C::ClatterCaptureClaptrap,
    C::ClatterCaptureBossgalaga,
    C::ClatterCaptureNabbit,
    C::ClatterCaptureMastercore,
    C::ClatterCaptureWaitOctopus,
];

/// The resolved values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    values: [i32; COUNT],
}

impl Default for Table {
    fn default() -> Self {
        Self::unresolved()
    }
}

impl Table {
    /// Every constant unresolved.
    pub const fn unresolved() -> Self {
        Self { values: [UNRESOLVED; COUNT] }
    }

    /// Ask `lookup` for every name (by its key). Returns how many names it did not answer; those stay
    /// `UNRESOLVED`.
    pub fn resolve(&mut self, mut lookup: impl FnMut(u64) -> Option<i32>) -> usize {
        let mut missing = 0;
        for (slot, name) in self.values.iter_mut().zip(NAMES.iter()) {
            match lookup(lua_bind_hash(name)) {
                Some(value) => *slot = value,
                None => {
                    *slot = UNRESOLVED;
                    missing += 1;
                }
            }
        }
        missing
    }

    /// A table with explicit values (tests, and a runtime that resolved elsewhere).
    pub fn with(values: &[(C, i32)]) -> Self {
        let mut table = Self::unresolved();
        for (c, value) in values {
            table.values[*c as usize] = *value;
        }
        table
    }

    pub fn get(&self, c: C) -> i32 {
        self.values[c as usize]
    }

    pub fn resolved(&self, c: C) -> bool {
        self.get(c) != UNRESOLVED
    }

    /// `value == c` and `c` resolved.
    pub fn is(&self, c: C, value: i32) -> bool {
        self.resolved(c) && self.get(c) == value
    }

    /// `first <= value <= last` (both ends resolved).
    pub fn within(&self, first: C, last: C, value: i32) -> bool {
        self.resolved(first) && self.resolved(last) && self.get(first) <= value && value <= self.get(last)
    }

    /// `first <= value < end` (both ends resolved).
    pub fn below(&self, first: C, end: C, value: i32) -> bool {
        self.resolved(first) && self.resolved(end) && self.get(first) <= value && value < self.get(end)
    }

    /// The names that stayed unresolved, in list order.
    pub fn missing(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.values.iter().zip(NAMES.iter()).filter(|(v, _)| **v == UNRESOLVED).map(|(_, n)| *n)
    }
}
