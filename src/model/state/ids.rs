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
