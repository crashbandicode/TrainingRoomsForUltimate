//! The game constants the save states use, by name. No numeric id is written down here: the runtime asks
//! the game's own constant table for each identifier (`model::cpu::hash::lua_bind_hash`, resolved
//! through `game::lua_const`, the mechanism package 2 built) and keeps the answers in a `Table`. A name the
//! game does not know stays `UNRESOLVED` and is reported, so the install line and the log say which
//! feature lost it. Each milestone adds its names; the order is the order of `C`, nothing depends on it.

use crate::model::cpu::hash::lua_bind_hash;

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
    // Situations (SPEC-state-01 §5.3).
    SituationGround = "SITUATION_KIND_GROUND",
    SituationAir = "SITUATION_KIND_AIR",
    SituationCliff = "SITUATION_KIND_CLIFF",
    // Statuses of the load machine.
    StatusDead = "FIGHTER_STATUS_KIND_DEAD",
    StatusStandby = "FIGHTER_STATUS_KIND_STANDBY",
    StatusRebirth = "FIGHTER_STATUS_KIND_REBIRTH",
    StatusWait = "FIGHTER_STATUS_KIND_WAIT",
    StatusFall = "FIGHTER_STATUS_KIND_FALL",
    StatusCliffCatchMove = "FIGHTER_STATUS_KIND_CLIFF_CATCH_MOVE",
    StatusCliffCatch = "FIGHTER_STATUS_KIND_CLIFF_CATCH",
    StatusNone = "FIGHTER_STATUS_KIND_NONE",
    // Fighter kinds the machine branches on.
    KindPopo = "FIGHTER_KIND_POPO",
    KindNana = "FIGHTER_KIND_NANA",
    KindKirby = "FIGHTER_KIND_KIRBY",
    KindPokemonSquirtle = "FIGHTER_KIND_PZENIGAME",
    KindPokemonIvysaur = "FIGHTER_KIND_PFUSHIGISOU",
    KindPokemonCharizard = "FIGHTER_KIND_PLIZARDON",
    KindHero = "FIGHTER_KIND_BRAVE",
    KindCloud = "FIGHTER_KIND_CLOUD",
    KindJoker = "FIGHTER_KIND_JACK",
    KindLittleMac = "FIGHTER_KIND_LITTLEMAC",
    KindSephiroth = "FIGHTER_KIND_EDGE",
    KindWiiFit = "FIGHTER_KIND_WIIFIT",
    KindShulk = "FIGHTER_KIND_SHULK",
    KindMinMin = "FIGHTER_KIND_TANTAN",
    KindWario = "FIGHTER_KIND_WARIO",
    // The respawn cleanup (SPEC-state-01 §4).
    QuakeKindNone = "CAMERA_QUAKE_KIND_NONE",
    QuakeKindMax = "CAMERA_QUAKE_KIND_MAX",
    WorkIntJostle = "FIGHTER_INSTANCE_WORK_ID_INT_HIT_STOP_IGNORE_JOSTLE_FRAME",
    WorkIntShulkFlash = "FIGHTER_INSTANCE_WORK_ID_INT_SHULK_MONAD_ARTS_DAMAGE_FLASH_FRAME",
    EffectNoJointScale = "EFFECT_SUB_ATTRIBUTE_NO_JOINT_SCALE",
    EffectFollow = "EFFECT_SUB_ATTRIBUTE_FOLLOW",
    EffectConcludeStatus = "EFFECT_SUB_ATTRIBUTE_CONCLUDE_STATUS",
    // The article keep-list: the (kind, article) pairs of SPEC-state-05 §4.1.
    KindMiiFighter = "FIGHTER_KIND_MIIFIGHTER",
    ArticleMiiFighterHat = "FIGHTER_MIIFIGHTER_GENERATE_ARTICLE_HAT",
    KindMiiSwordsman = "FIGHTER_KIND_MIISWORDSMAN",
    ArticleMiiSwordsmanHat = "FIGHTER_MIISWORDSMAN_GENERATE_ARTICLE_HAT",
    KindMiiGunner = "FIGHTER_KIND_MIIGUNNER",
    ArticleMiiGunnerHat = "FIGHTER_MIIGUNNER_GENERATE_ARTICLE_HAT",
    KindRosalina = "FIGHTER_KIND_ROSETTA",
    ArticleRosalinaTico = "FIGHTER_ROSETTA_GENERATE_ARTICLE_TICO",
    KindSteve = "FIGHTER_KIND_PICKEL",
    ArticleSteveTable = "FIGHTER_PICKEL_GENERATE_ARTICLE_TABLE",
    KindMythra = "FIGHTER_KIND_ELIGHT",
    ArticleMythraSword = "FIGHTER_ELIGHT_GENERATE_ARTICLE_ESWORD",
    KindPyra = "FIGHTER_KIND_EFLAME",
    ArticlePyraSword = "FIGHTER_EFLAME_GENERATE_ARTICLE_ESWORD",
    KindOlimar = "FIGHTER_KIND_PIKMIN",
    ArticleOlimarPikmin = "FIGHTER_PIKMIN_GENERATE_ARTICLE_PIKMIN",
    KindDuckHunt = "FIGHTER_KIND_DUCKHUNT",
    ArticleDuckHuntReticle = "FIGHTER_DUCKHUNT_GENERATE_ARTICLE_RETICLE",
    // Charges (SPEC-state-02): the kinds and the work ids of each fighter's chargeable special.
    KindMario = "FIGHTER_KIND_MARIO",
    KindDonkey = "FIGHTER_KIND_DONKEY",
    KindSamus = "FIGHTER_KIND_SAMUS",
    KindSamusD = "FIGHTER_KIND_SAMUSD",
    KindSheik = "FIGHTER_KIND_SHEIK",
    KindMewtwo = "FIGHTER_KIND_MEWTWO",
    KindGameWatch = "FIGHTER_KIND_GAMEWATCH",
    KindLucario = "FIGHTER_KIND_LUCARIO",
    KindRob = "FIGHTER_KIND_ROBOT",
    KindPacMan = "FIGHTER_KIND_PACMAN",
    KindRobin = "FIGHTER_KIND_REFLET",
    KindIncineroar = "FIGHTER_KIND_GAOGAEN",
    KindPiranha = "FIGHTER_KIND_PACKUN",
    KindBanjo = "FIGHTER_KIND_BUDDY",
    KindSora = "FIGHTER_KIND_TRAIL",
    FacialSpecial = "FIGHTER_FACIAL_SPECIAL",
    MarioFluddCharge = "FIGHTER_MARIO_INSTANCE_WORK_ID_INT_SPECIAL_LW_CHARGE",
    DonkeyPunchCount = "FIGHTER_DONKEY_INSTANCE_WORK_ID_INT_SPECIAL_N_COUNT",
    SamusShotCount = "FIGHTER_SAMUS_INSTANCE_WORK_ID_INT_SPECIAL_N_COUNT",
    SamusEffectHandle = "FIGHTER_SAMUS_INSTANCE_WORK_ID_INT_EFH_CHARGE_MAX",
    SheikNeedleCount = "FIGHTER_SHEIK_INSTANCE_WORK_ID_INT_NEEDLE_COUNT",
    SheikNeedleArticle = "FIGHTER_SHEIK_GENERATE_ARTICLE_NEEDLEHAVE",
    MewtwoCharge = "FIGHTER_MEWTWO_INSTANCE_WORK_ID_INT_SHADOWBALL_CHARGE_FRAME",
    MewtwoPrevCharge = "FIGHTER_MEWTWO_INSTANCE_WORK_ID_INT_PREV_SHADOWBALL_CHARGE_FRAME",
    MewtwoHad = "FIGHTER_MEWTWO_INSTANCE_WORK_ID_FLAG_SHADOWBALL_HAD",
    MewtwoEffectLeft = "FIGHTER_MEWTWO_INSTANCE_WORK_ID_INT_EF_ID_SHADOWBALL_MAX_L",
    MewtwoEffectRight = "FIGHTER_MEWTWO_INSTANCE_WORK_ID_INT_EF_ID_SHADOWBALL_MAX_R",
    GameWatchGauge = "FIGHTER_GAMEWATCH_INSTANCE_WORK_ID_FLOAT_SPECIAL_LW_GAUGE",
    GameWatchAttack = "FIGHTER_GAMEWATCH_INSTANCE_WORK_ID_FLOAT_SPECIAL_LW_ATTACK",
    WarioGassCount = "FIGHTER_WARIO_INSTANCE_WORK_ID_INT_GASS_COUNT",
    WarioGassLevel = "FIGHTER_WARIO_INSTANCE_WORK_ID_INT_GASS_LEVEL",
    SquirtleCharge = "FIGHTER_PZENIGAME_INSTANCE_WORK_ID_INT_SPECIAL_N_CHARGE",
    LucarioCharge = "FIGHTER_LUCARIO_INSTANCE_WORK_ID_INT_AURABALL_CHARGE_FRAME",
    LucarioPrevCharge = "FIGHTER_LUCARIO_INSTANCE_WORK_ID_INT_PREV_AURABALL_CHARGE_FRAME",
    LucarioHad = "FIGHTER_LUCARIO_INSTANCE_WORK_ID_FLAG_AURABALL_HAD",
    LucarioEffectLeft = "FIGHTER_LUCARIO_INSTANCE_WORK_ID_INT_EF_ID_AURABALL_MAX_L",
    LucarioEffectRight = "FIGHTER_LUCARIO_INSTANCE_WORK_ID_INT_EF_ID_AURABALL_MAX_R",
    RobBeam = "FIGHTER_ROBOT_INSTANCE_WORK_ID_FLOAT_BEAM_ENERGY_VALUE",
    RobGyro = "FIGHTER_ROBOT_INSTANCE_WORK_ID_FLOAT_GYRO_CHARGE_VALUE",
    RobBurner = "FIGHTER_ROBOT_INSTANCE_WORK_ID_FLOAT_BURNER_ENERGY_VALUE",
    WiiFitBreath = "FIGHTER_WIIFIT_INSTANCE_WORK_ID_FLOAT_SPECIAL_N_CHARGE_LEVEL_RATIO",
    PacManRank = "FIGHTER_PACMAN_INSTANCE_WORK_ID_INT_SPECIAL_N_CHARGE_RANK",
    PacManPullThrow = "FIGHTER_PACMAN_INSTANCE_WORK_ID_FLAG_SPECIAL_N_PULL_THROW",
    PacManMaxHaveItem = "FIGHTER_PACMAN_INSTANCE_WORK_ID_FLAG_SPECIAL_N_MAX_HAVE_ITEM",
    RobinThunderKind = "FIGHTER_REFLET_INSTANCE_WORK_ID_INT_SPECIAL_N_THUNDER_KIND",
    RobinYellow = "FIGHTER_REFLET_INSTANCE_WORK_ID_INT_SPECIAL_N_CURRENT_POINT",
    RobinRed = "FIGHTER_REFLET_INSTANCE_WORK_ID_FLOAT_SPECIAL_S_CURRENT_POINT",
    RobinGreen = "FIGHTER_REFLET_INSTANCE_WORK_ID_INT_SPECIAL_HI_CURRENT_POINT",
    RobinPurple = "FIGHTER_REFLET_INSTANCE_WORK_ID_INT_SPECIAL_LW_CURRENT_POINT",
    RobinLevinCharge = "FIGHTER_REFLET_INSTANCE_WORK_ID_INT_THUNDER_SWORD_CURRENT_POINT",
    RobinLevinRecharge = "FIGHTER_REFLET_INSTANCE_WORK_ID_INT_THUNDER_SWORD_REVIVAL_COUNT",
    RobinLevinOn = "FIGHTER_REFLET_INSTANCE_WORK_ID_FLAG_THUNDER_SWORD_ON",
    IncineroarRevengeNo = "FIGHTER_GAOGAEN_INSTANCE_WORK_ID_INT_ATTACK_HIT_REVENGE_ATTACK_NO",
    IncineroarRevengeTotal = "FIGHTER_GAOGAEN_INSTANCE_WORK_ID_FLOAT_REVENGE_TOTAL_DAMAGE",
    IncineroarRevengeRate = "FIGHTER_GAOGAEN_INSTANCE_WORK_ID_FLOAT_REVENGE_RATE",
    IncineroarIsRevenge = "FIGHTER_GAOGAEN_INSTANCE_WORK_ID_FLAG_IS_REVENGE",
    IncineroarRevengeTimer = "FIGHTER_GAOGAEN_INSTANCE_WORK_ID_INT_REVENGE_TIMER",
    PiranhaCount = "FIGHTER_PACKUN_INSTANCE_WORK_ID_INT_SPECIAL_S_COUNT",
    PiranhaEffectHandle = "FIGHTER_PACKUN_INSTANCE_WORK_ID_INT_SPECIAL_S_CHARGE_MAX_EFFECT_HANDLE",
    HeroHoldFrame = "FIGHTER_BRAVE_INSTANCE_WORK_ID_INT_SPECIAL_N_HOLD_FRAME",
    HeroMaxEffect = "FIGHTER_BRAVE_INSTANCE_WORK_ID_FLAG_SPECIAL_N_MAX_EFFECT",
    BanjoFeathers = "FIGHTER_BUDDY_INSTANCE_WORK_ID_INT_SPECIAL_S_REMAIN",
    GunnerCharge = "FIGHTER_MIIGUNNER_INSTANCE_WORK_ID_INT_GUNNER_CHARGE_COUNT",
    GunnerEffectHandle = "FIGHTER_MIIGUNNER_INSTANCE_WORK_ID_INT_EFH_CHARGE_MAX",
    // Steve and Olimar (SPEC-state-05, SPEC-state-02 §5.1).
    SteveMatG1 = "FIGHTER_PICKEL_INSTANCE_WORK_ID_INT_MATERIAL_NUM_GRADE_1",
    SteveMatWood = "FIGHTER_PICKEL_INSTANCE_WORK_ID_INT_MATERIAL_NUM_WOOD",
    SteveMatStone = "FIGHTER_PICKEL_INSTANCE_WORK_ID_INT_MATERIAL_NUM_STONE",
    SteveMatIron = "FIGHTER_PICKEL_INSTANCE_WORK_ID_INT_MATERIAL_NUM_IRON",
    SteveMatGold = "FIGHTER_PICKEL_INSTANCE_WORK_ID_INT_MATERIAL_NUM_GOLD",
    SteveMatRedstone = "FIGHTER_PICKEL_INSTANCE_WORK_ID_INT_MATERIAL_NUM_RED_STONE",
    SteveMatDiamond = "FIGHTER_PICKEL_INSTANCE_WORK_ID_INT_MATERIAL_NUM_DIAMOND",
    SteveExtendBuffer = "FIGHTER_PICKEL_INSTANCE_WORK_ID_INT_EXTEND_BUFFER",
    OlimarHold0 = "FIGHTER_PIKMIN_INSTANCE_WORK_INT_PIKMIN_HOLD_PIKMIN_OBJECT_ID_0",
    OlimarHold1 = "FIGHTER_PIKMIN_INSTANCE_WORK_INT_PIKMIN_HOLD_PIKMIN_OBJECT_ID_1",
    OlimarHold2 = "FIGHTER_PIKMIN_INSTANCE_WORK_INT_PIKMIN_HOLD_PIKMIN_OBJECT_ID_2",
    PikminVariation = "WEAPON_PIKMIN_PIKMIN_INSTANCE_WORK_ID_INT_VARIATION",
    OlimarPreVariation = "FIGHTER_PIKMIN_INSTANCE_WORK_INT_PRE_PIKMIN_VARIATION",
    OlimarBeforePreVariation = "FIGHTER_PIKMIN_INSTANCE_WORK_INT_BEFORE_PRE_PIKMIN_VARIATION",
    PikminStatusAirFollow = "WEAPON_PIKMIN_PIKMIN_STATUS_KIND_AIR_FOLLOW",
    PikminStatusAttackAir = "WEAPON_PIKMIN_PIKMIN_STATUS_KIND_ATTACK_AIR",
    PikminStatusAttackHi4Landing = "WEAPON_PIKMIN_PIKMIN_STATUS_KIND_ATTACK_HI4_LANDING",
    ArticleAllTarget = "ARTICLE_OPE_TARGET_ALL",
    // Character items (SPEC-state-04 §2.1): the item kinds and variations a held-item snapshot restores.
    ItemBuddyBomb = "ITEM_KIND_BUDDYBOMB",
    ItemMechakoopa = "ITEM_KIND_MECHAKOOPA",
    ItemDoseisan = "ITEM_KIND_DOSEISAN",
    ItemBombhei = "ITEM_KIND_BOMBHEI",
    ItemBombheiNormal = "ITEM_VARIATION_BOMBHEI_NORMAL",
    ItemDiddyPeanuts = "ITEM_KIND_DIDDYPEANUTS",
    ItemKroolCrown = "ITEM_KIND_KROOLCROWN",
    ItemLinkBomb = "ITEM_KIND_LINKBOMB",
    ItemLinkArrow = "ITEM_KIND_LINKARROW",
    ItemMetalBlade = "ITEM_KIND_METALBLADE",
    ItemCherry = "ITEM_KIND_PACMANCHERRY",
    ItemStrawberry = "ITEM_KIND_PACMANSTRAWBERRY",
    ItemOrange = "ITEM_KIND_PACMANORANGE",
    ItemApple = "ITEM_KIND_PACMANAPPLE",
    ItemMelon = "ITEM_KIND_PACMANMELON",
    ItemGalaxian = "ITEM_KIND_PACMANBOSS",
    ItemBell = "ITEM_KIND_PACMANBELL",
    ItemKey = "ITEM_KIND_PACMANKEY",
    ItemRichterWater = "ITEM_KIND_RICHTERHOLYWATER",
    ItemGyro = "ITEM_KIND_ROBOTGYRO",
    ItemGyro1p = "ITEM_VARIATION_ROBOTGYRO_1P",
    ItemBook = "ITEM_KIND_BOOK",
    ItemLevin = "ITEM_KIND_THUNDERSWORD",
    ItemBurstGrenade = "ITEM_KIND_EXPLOSIONBOMB",
    ItemSimonWater = "ITEM_KIND_SIMONHOLYWATER",
    ItemSnakeGrenade = "ITEM_KIND_SNAKEGRENADE",
    ItemToonBomb = "ITEM_KIND_TOONLINKBOMB",
    ItemWoodChip = "ITEM_KIND_WOOD",
    ItemYoungBomb = "ITEM_KIND_YOUNGLINKBOMB",
    ItemBanana = "ITEM_KIND_BANANA",
    ItemDaisyTurnip = "ITEM_KIND_DAISYDAIKON",
    ItemPeachTurnip = "ITEM_KIND_PEACHDAIKON",
    DaisyTurnipInit = "ITEM_DAISYDAIKON_ACTION_SPECIAL_INIT",
    PeachTurnipInit = "ITEM_PEACHDAIKON_ACTION_SPECIAL_INIT",
    ItemWorkVariation = "ITEM_INSTANCE_WORK_INT_VARIATION",
    ItemWorkLifeTime = "ITEM_INSTANCE_WORK_INT_LIFE_TIME",
    ItemWorkLifeTimeMax = "ITEM_INSTANCE_WORK_INT_LIFE_TIME_MAX",
    ItemWorkBombCounter = "ITEM_INSTANCE_WORK_INT_BOMB_COUNTER",
    ItemWorkBombCounterMax = "ITEM_INSTANCE_WORK_INT_BOMB_COUNTER_MAX",
    CategoryItem = "BATTLE_OBJECT_CATEGORY_ITEM",
    KindDiddy = "FIGHTER_KIND_DIDDY",
    KindLink = "FIGHTER_KIND_LINK",
    KindBowserJr = "FIGHTER_KIND_KOOPAJR",
    KindKrool = "FIGHTER_KIND_KROOL",
    KindMegaMan = "FIGHTER_KIND_ROCKMAN",
    KindRichter = "FIGHTER_KIND_RICHTER",
    KindSimon = "FIGHTER_KIND_SIMON",
    KindSnake = "FIGHTER_KIND_SNAKE",
    KindToonLink = "FIGHTER_KIND_TOONLINK",
    KindVillager = "FIGHTER_KIND_MURABITO",
    KindYoungLink = "FIGHTER_KIND_YOUNGLINK",
    KindDaisy = "FIGHTER_KIND_DAISY",
    KindPeach = "FIGHTER_KIND_PEACH",
    // Buffs (SPEC-state-03).
    QuakeKindM = "CAMERA_QUAKE_KIND_M",
    QuakeKindS = "CAMERA_QUAKE_KIND_S",
    HeroDecideCommand = "FIGHTER_BRAVE_INSTANCE_WORK_ID_INT_SPECIAL_LW_DECIDE_COMMAND",
    HeroSpellAccelerate = "FIGHTER_BRAVE_SPECIAL_LW_COMMAND11_SPEED_UP",
    HeroSpellOomph = "FIGHTER_BRAVE_SPECIAL_LW_COMMAND12_ATTACK_UP",
    HeroSpellPsyche = "FIGHTER_BRAVE_SPECIAL_LW_COMMAND21_CHARGE",
    HeroSpellBounce = "FIGHTER_BRAVE_SPECIAL_LW_COMMAND13_REFLECT",
    HeroStatusSpecialLwStart = "FIGHTER_BRAVE_STATUS_KIND_SPECIAL_LW_START",
    LittleMacKoGauge = "FIGHTER_LITTLEMAC_INSTANCE_WORK_ID_FLOAT_KO_GAGE",
    SephirothRuleHp = "FIGHTER_EDGE_INSTANCE_WORK_ID_FLAG_IS_RULE_HP",
    SephirothSuddenDeath = "FIGHTER_EDGE_INSTANCE_WORK_ID_FLAG_SUDDEN_DEATH",
    SephirothOneWinged = "FIGHTER_EDGE_INSTANCE_WORK_ID_FLAG_ONE_WINGED_ACTIVATED",
    WarioGassLevelM = "FIGHTER_WARIO_GASS_LEVEL_M",
    WarioGassLevelL = "FIGHTER_WARIO_GASS_LEVEL_L",
    WarioGassLevelFly = "FIGHTER_WARIO_GASS_LEVEL_FLY",
    ShulkTypeSelect = "FIGHTER_SHULK_INSTANCE_WORK_ID_INT_SPECIAL_N_TYPE_SELECT",
    ShulkSelectTimer = "FIGHTER_SHULK_INSTANCE_WORK_ID_INT_SPECIAL_N_SELECT_TIMER",
    ShulkSelectFlag = "FIGHTER_SHULK_INSTANCE_WORK_ID_FLAG_SPECIAL_N_SELECT",
    ShulkStatusAction = "FIGHTER_SHULK_STATUS_KIND_SPECIAL_N_ACTION",
    ShulkArtJump = "FIGHTER_SHULK_MONAD_TYPE_JUMP",
    ShulkArtSpeed = "FIGHTER_SHULK_MONAD_TYPE_SPEED",
    ShulkArtShield = "FIGHTER_SHULK_MONAD_TYPE_SHIELD",
    ShulkArtBuster = "FIGHTER_SHULK_MONAD_TYPE_BUSTER",
    ShulkArtSmash = "FIGHTER_SHULK_MONAD_TYPE_SMASH",
    WiiFitStatusSuccess = "FIGHTER_WIIFIT_STATUS_KIND_SPECIAL_LW_SUCCESS",
    MinMinSpiralLeft = "FIGHTER_TANTAN_GENERATE_ARTICLE_SPIRALLEFT",
    MinMinArmBigFrame = "FIGHTER_TANTAN_INSTANCE_WORK_ID_INT_ARM_L_BIG_FRAME",
    MinMinDragonizeL = "FIGHTER_TANTAN_INSTANCE_WORK_ID_FLAG_DRAGONIZE_L",
    MinMinReinforceHandle = "FIGHTER_TANTAN_INSTANCE_WORK_ID_INT_REINFORCE_L_EFFECT_HANDLE_L",
    MinMinPartDragon = "FIGHTER_TANTAN_MOTION_PART_SET_KIND_DRAGON",
    JokerStatusSummon = "FIGHTER_JACK_STATUS_KIND_SUMMON",
    SoraMagicKind = "FIGHTER_TRAIL_INSTANCE_WORK_ID_INT_SPECIAL_N_MAGIC_KIND",
    SoraChangeMagicFlag = "FIGHTER_TRAIL_STATUS_SPECIAL_N1_FLAG_CHANGE_MAGIC",
}

/// Number of constants.
pub const COUNT: usize = NAMES.len();

/// The value of a constant the game did not answer for. No status, kind or work id is this value.
pub const UNRESOLVED: i32 = i32::MIN;

/// The articles a respawn cleanup keeps, as `(kind, article)` pairs (SPEC-state-05 §4.1).
pub const KEEP_LIST: [(C, C); 9] = [
    (C::KindMiiFighter, C::ArticleMiiFighterHat),
    (C::KindMiiSwordsman, C::ArticleMiiSwordsmanHat),
    (C::KindMiiGunner, C::ArticleMiiGunnerHat),
    (C::KindRosalina, C::ArticleRosalinaTico),
    (C::KindSteve, C::ArticleSteveTable),
    (C::KindMythra, C::ArticleMythraSword),
    (C::KindPyra, C::ArticlePyraSword),
    (C::KindOlimar, C::ArticleOlimarPikmin),
    (C::KindDuckHunt, C::ArticleDuckHuntReticle),
];

/// Article indices a cleanup visits (SPEC-state-01 §4: `0..=0x25`).
pub const ARTICLE_COUNT: i32 = 0x26;

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
    pub const fn unresolved() -> Self {
        Self { values: [UNRESOLVED; COUNT] }
    }

    /// Ask `lookup` for every name (by its key). Returns how many it did not answer.
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

    pub fn with(values: &[(C, i32)]) -> Self {
        let mut table = Self::unresolved();
        for (c, value) in values {
            table.values[*c as usize] = *value;
        }
        table
    }

    /// Every constant resolved to `base + its index` (host tests: distinct, known values).
    pub fn sequential(base: i32) -> Self {
        let mut table = Self::unresolved();
        for (i, slot) in table.values.iter_mut().enumerate() {
            *slot = base + i as i32;
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

    /// The names that stayed unresolved.
    pub fn missing(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.values.iter().zip(NAMES.iter()).filter(|(v, _)| **v == UNRESOLVED).map(|(_, n)| *n)
    }

    /// The situation constants the machine classifies with.
    pub fn situations(&self) -> super::machine::SituationIds {
        super::machine::SituationIds {
            ground: self.get(C::SituationGround),
            air: self.get(C::SituationAir),
            cliff: self.get(C::SituationCliff),
        }
    }

    /// Whether the fighter kind keeps the article `article` through a respawn cleanup.
    pub fn keeps_article(&self, kind: i32, article: i32) -> bool {
        KEEP_LIST.iter().any(|(k, a)| self.is(*k, kind) && self.is(*a, article))
    }

    /// The Hero, Cloud, Joker, Little Mac, Sephiroth, Wii Fit, Shulk, Min Min, Wario kinds
    /// (SPEC-state-01 §5.3 step 8d).
    pub fn is_buffable(&self, kind: i32) -> bool {
        [
            C::KindHero,
            C::KindCloud,
            C::KindJoker,
            C::KindLittleMac,
            C::KindSephiroth,
            C::KindWiiFit,
            C::KindShulk,
            C::KindMinMin,
            C::KindWario,
        ]
        .iter()
        .any(|c| self.is(*c, kind))
    }

    pub fn is_pokemon(&self, kind: i32) -> bool {
        [C::KindPokemonSquirtle, C::KindPokemonIvysaur, C::KindPokemonCharizard].iter().any(|c| self.is(*c, kind))
    }
}
