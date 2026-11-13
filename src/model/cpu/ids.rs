//! The game constants the CPU behaviours use, by name, and the table that holds their values.
//!
//! No numeric id is written down here: the runtime asks the game's own constant table for each name
//! at run time (`hash::lua_bind_hash`, `main+0x3728e10`) and stores the answers in a `Table`. A name
//! the game does not know stays unresolved (`UNRESOLVED`, which matches no status) and is listed so
//! the install line can say which feature lost it. Each package that needs more names adds them here;
//! the order of the list is the order of `C`, nothing else depends on it.

/// A list of game constants by name and the table of their resolved values: the enum `C`, `NAMES`, `COUNT`,
/// `UNRESOLVED` and `Table` with its lookups. One implementation for every such list (`model::state::ids` too); a
/// module adds its own predicates in a second `impl Table`.
macro_rules! game_constants {
    ($($variant:ident = $name:literal,)*) => {
        /// One game constant.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(u16)]
        pub enum C { $($variant),* }

        /// The identifiers, in `C` order.
        pub const NAMES: &[&str] = &[$($name),*];

        /// Number of constants.
        pub const COUNT: usize = NAMES.len();

        /// The value of a constant the game did not answer for. No status, term or work id is this value.
        pub const UNRESOLVED: i32 = i32::MIN;

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

            /// Ask `lookup` for every name (by its key, `cpu::hash::lua_bind_hash`). Returns how many names it did
            /// not answer; those stay `UNRESOLVED`.
            pub fn resolve(&mut self, mut lookup: impl FnMut(u64) -> Option<i32>) -> usize {
                let mut missing = 0;
                for (slot, name) in self.values.iter_mut().zip(NAMES.iter()) {
                    match lookup(crate::model::cpu::hash::lua_bind_hash(name)) {
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

            /// The names that stayed unresolved, in list order.
            pub fn missing(&self) -> impl Iterator<Item = &'static str> + '_ {
                self.values.iter().zip(NAMES.iter()).filter(|(v, _)| **v == UNRESOLVED).map(|(_, n)| *n)
            }
        }
    };
}
pub(crate) use game_constants;

game_constants! {
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
    TermContEscapeAir = "FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_ESCAPE_AIR",
    TermContGuardOn = "FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_GUARD_ON",
    TermDownStand = "FIGHTER_STATUS_TRANSITION_TERM_ID_DOWN_STAND",
    ButtonGuard = "CONTROL_PAD_BUTTON_GUARD",
    ButtonGuardHold = "CONTROL_PAD_BUTTON_GUARD_HOLD",
    // The mash actions and throws (SPEC-cpu-01, SPEC-cpu-06).
    CatCommand1 = "FIGHTER_PAD_COMMAND_CATEGORY1",
    CatCommand2 = "FIGHTER_PAD_COMMAND_CATEGORY2",
    FlagAirEscape = "FIGHTER_PAD_CMD_CAT1_FLAG_AIR_ESCAPE",
    FlagJumpButton = "FIGHTER_PAD_CMD_CAT1_FLAG_JUMP_BUTTON",
    FlagEscape = "FIGHTER_PAD_CMD_CAT1_FLAG_ESCAPE",
    FlagEscapeF = "FIGHTER_PAD_CMD_CAT1_FLAG_ESCAPE_F",
    FlagEscapeB = "FIGHTER_PAD_CMD_CAT1_FLAG_ESCAPE_B",
    FlagSpecialN = "FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_N",
    FlagSpecialS = "FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S",
    FlagSpecialHi = "FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI",
    FlagSpecialLw = "FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW",
    FlagAttackHi4 = "FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI4",
    FlagAttackS4 = "FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S4",
    FlagAttackLw4 = "FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW4",
    FlagCatch = "FIGHTER_PAD_CMD_CAT1_FLAG_CATCH",
    FlagAttackN = "FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_N",
    FlagAttackS3 = "FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3",
    FlagAttackHi3 = "FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3",
    FlagAttackLw3 = "FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3",
    FlagThrowF = "FIGHTER_PAD_CMD_CAT2_FLAG_THROW_F",
    FlagThrowB = "FIGHTER_PAD_CMD_CAT2_FLAG_THROW_B",
    FlagThrowHi = "FIGHTER_PAD_CMD_CAT2_FLAG_THROW_HI",
    FlagThrowLw = "FIGHTER_PAD_CMD_CAT2_FLAG_THROW_LW",
    AirKindN = "FIGHTER_COMMAND_ATTACK_AIR_KIND_N",
    AirKindF = "FIGHTER_COMMAND_ATTACK_AIR_KIND_F",
    AirKindB = "FIGHTER_COMMAND_ATTACK_AIR_KIND_B",
    AirKindHi = "FIGHTER_COMMAND_ATTACK_AIR_KIND_HI",
    AirKindLw = "FIGHTER_COMMAND_ATTACK_AIR_KIND_LW",
    StatusEscapeAir = "FIGHTER_STATUS_KIND_ESCAPE_AIR",
    StatusJumpSquat = "FIGHTER_STATUS_KIND_JUMP_SQUAT",
    StatusJumpAerial = "FIGHTER_STATUS_KIND_JUMP_AERIAL",
    StatusJump = "FIGHTER_STATUS_KIND_JUMP",
    StatusDash = "FIGHTER_STATUS_KIND_DASH",
    StatusAttackDash = "FIGHTER_STATUS_KIND_ATTACK_DASH",
    StatusAttackAir = "FIGHTER_STATUS_KIND_ATTACK_AIR",
    StatusFall = "FIGHTER_STATUS_KIND_FALL",
    StatusFallAerial = "FIGHTER_STATUS_KIND_FALL_AERIAL",
    StatusSpecialN = "FIGHTER_STATUS_KIND_SPECIAL_N",
    StatusSpecialS = "FIGHTER_STATUS_KIND_SPECIAL_S",
    StatusSpecialHi = "FIGHTER_STATUS_KIND_SPECIAL_HI",
    StatusSpecialLw = "FIGHTER_STATUS_KIND_SPECIAL_LW",
    StatusAttackHi4Start = "FIGHTER_STATUS_KIND_ATTACK_HI4_START",
    StatusAttackS4Start = "FIGHTER_STATUS_KIND_ATTACK_S4_START",
    StatusAttackLw4Start = "FIGHTER_STATUS_KIND_ATTACK_LW4_START",
    StatusCatch = "FIGHTER_STATUS_KIND_CATCH",
    StatusAttack = "FIGHTER_STATUS_KIND_ATTACK",
    StatusAttackS3 = "FIGHTER_STATUS_KIND_ATTACK_S3",
    StatusAttackHi3 = "FIGHTER_STATUS_KIND_ATTACK_HI3",
    StatusAttackLw3 = "FIGHTER_STATUS_KIND_ATTACK_LW3",
    StatusCatchWait = "FIGHTER_STATUS_KIND_CATCH_WAIT",
    StatusCatchPull = "FIGHTER_STATUS_KIND_CATCH_PULL",
    StatusCatchAttack = "FIGHTER_STATUS_KIND_CATCH_ATTACK",
    StatusSquat = "FIGHTER_STATUS_KIND_SQUAT",
    StatusSquatB = "FIGHTER_STATUS_KIND_SQUAT_B",
    StatusSquatF = "FIGHTER_STATUS_KIND_SQUAT_F",
    StatusSquatRv = "FIGHTER_STATUS_KIND_SQUAT_RV",
    StatusSquatWait = "FIGHTER_STATUS_KIND_SQUAT_WAIT",
    TermContDash = "FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_DASH",
    TermContCatchDash = "FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_CATCH_DASH",
    TermContAttackAir = "FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_ATTACK_AIR",
    TermContThrowHi = "FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_THROW_HI",
    WorkIntInvalidCatchFrame = "FIGHTER_INSTANCE_WORK_ID_INT_INVALID_CATCH_FRAME",
    WorkFlagReserveDive = "FIGHTER_STATUS_WORK_ID_FLAG_RESERVE_DIVE",
    KineticGravity = "FIGHTER_KINETIC_ENERGY_ID_GRAVITY",
    ButtonJump = "CONTROL_PAD_BUTTON_JUMP",
    ButtonFlickJump = "CONTROL_PAD_BUTTON_FLICK_JUMP",
    KindKoopa = "FIGHTER_KIND_KOOPA",
    KindTrail = "FIGHTER_KIND_TRAIL",
    KindKrool = "FIGHTER_KIND_KROOL",
    KindKirby = "FIGHTER_KIND_KIRBY",
    KoopaSpecialHiG = "FIGHTER_KOOPA_STATUS_KIND_SPECIAL_HI_G",
    KoopaSpecialHiA = "FIGHTER_KOOPA_STATUS_KIND_SPECIAL_HI_A",
    TrailSpecialN1 = "FIGHTER_TRAIL_STATUS_KIND_SPECIAL_N1",
    TrailSpecialN1Shoot = "FIGHTER_TRAIL_STATUS_KIND_SPECIAL_N1_SHOOT",
    TrailSpecialN1End = "FIGHTER_TRAIL_STATUS_KIND_SPECIAL_N1_END",
    TrailSpecialN2 = "FIGHTER_TRAIL_STATUS_KIND_SPECIAL_N2",
    TrailSpecialN3 = "FIGHTER_TRAIL_STATUS_KIND_SPECIAL_N3",
    TrailAttackAirN = "FIGHTER_TRAIL_STATUS_KIND_ATTACK_AIR_N",
    TrailAttackAirF = "FIGHTER_TRAIL_STATUS_KIND_ATTACK_AIR_F",
    KroolSpecialSThrow = "FIGHTER_KROOL_STATUS_KIND_SPECIAL_S_THROW",
    KroolSpecialSCatch = "FIGHTER_KROOL_STATUS_KIND_SPECIAL_S_CATCH",
    KroolSpecialSFailure = "FIGHTER_KROOL_STATUS_KIND_SPECIAL_S_FAILURE",
    KroolSpecialSGet = "FIGHTER_KROOL_STATUS_KIND_SPECIAL_S_GET",
    KirbyMiiSpecialN = "FIGHTER_KIRBY_STATUS_KIND_MIIFIGHTER_SPECIAL_N",
    KirbyNum = "FIGHTER_KIRBY_STATUS_KIND_NUM",
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
    // Package 4 (SPEC-input-01 §3, §5.4): the recorder's starting-status to transition-term table, the
    // Shulk dial exception, the ledge rest timer and the colour-blend model colour type.
    StatusEscapeAirSlide = "FIGHTER_STATUS_KIND_ESCAPE_AIR_SLIDE",
    TermContSpecialHi = "FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_SPECIAL_HI",
    TermContJumpSquat = "FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_JUMP_SQUAT",
    TermContJumpAerial = "FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_JUMP_AERIAL",
    TermContEscapeF = "FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_ESCAPE_F",
    TermContCatch = "FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_CATCH",
    TermContSpecialS = "FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_SPECIAL_S",
    KindShulk = "FIGHTER_KIND_SHULK",
    WorkIntEntryId = "FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID",
    WorkIntColor = "FIGHTER_INSTANCE_WORK_ID_INT_COLOR",
    WorkFlagShulkDial = "FIGHTER_SHULK_INSTANCE_WORK_ID_FLAG_SPECIAL_N_CIRCLE_MENU",
    WorkIntCliffRestTime = "FIGHTER_STATUS_CLIFF_WORK_INT_CATCH_REST_TIME",
    ModelColorBlend = "MODEL_COLOR_TYPE_COLOR_BLEND",
}

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

impl Table {
    /// `first <= value <= last` (both ends resolved).
    pub fn within(&self, first: C, last: C, value: i32) -> bool {
        self.resolved(first) && self.resolved(last) && self.get(first) <= value && value <= self.get(last)
    }

    /// `first <= value < end` (both ends resolved).
    pub fn below(&self, first: C, end: C, value: i32) -> bool {
        self.resolved(first) && self.resolved(end) && self.get(first) <= value && value < self.get(end)
    }

    /// Every resolved constant as `(name, value)`, in list order (the diag build prints them so a log can be
    /// read by name).
    pub fn resolved_entries(&self) -> impl Iterator<Item = (&'static str, i32)> + '_ {
        self.values.iter().zip(NAMES.iter()).filter(|(v, _)| **v != UNRESOLVED).map(|(v, n)| (*n, *v))
    }
}
