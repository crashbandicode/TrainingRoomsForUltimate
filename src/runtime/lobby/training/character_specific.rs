//! The fork's `training::character_specific`, as far as the roster cycle's character items use it: the table of the 38 grantable
//! items and the turnip face override. Our code: the table is SPEC-state-04 section 2.1 (item kind, owner fighter, variation per
//! entry, in the order of `engine::items::character_items::ITEM_NAMES`); the face override is package 3's (the sixteen turnip
//! probability hooks read it).

pub(crate) mod items {
    use std::sync::LazyLock;

    use smash::lib::lua_const::*;

    use crate::runtime::compat::sync::{Assignable, Readable};

    /// One grantable item: the fighter that owns it, its item kind (`None` for the article kinds, whose generator the grant uses:
    /// the turnips and the banana) and its variation (`None` is variation 0). Each value is a game constant resolved on first use.
    pub struct CharItem {
        pub fighter_kind: LazyLock<i32>,
        pub item_kind: Option<LazyLock<i32>>,
        pub variation: Option<LazyLock<i32>>,
    }

    macro_rules! constant {
        ($name:ident) => {
            LazyLock::new(|| *$name)
        };
    }

    macro_rules! item {
        ($fighter:ident) => {
            CharItem { fighter_kind: constant!($fighter), item_kind: None, variation: None }
        };
        ($fighter:ident, $kind:ident) => {
            CharItem { fighter_kind: constant!($fighter), item_kind: Some(constant!($kind)), variation: None }
        };
        ($fighter:ident, $kind:ident, $variation:ident) => {
            CharItem { fighter_kind: constant!($fighter), item_kind: Some(constant!($kind)), variation: Some(constant!($variation)) }
        };
        ($fighter:ident, , $variation:ident) => {
            CharItem { fighter_kind: constant!($fighter), item_kind: None, variation: Some(constant!($variation)) }
        };
    }

    #[cfg(feature = "diag")]
    macro_rules! names {
        ($fighter:ident) => {
            (stringify!($fighter), None, None)
        };
        ($fighter:ident, $kind:ident) => {
            (stringify!($fighter), Some(stringify!($kind)), None)
        };
        ($fighter:ident, $kind:ident, $variation:ident) => {
            (stringify!($fighter), Some(stringify!($kind)), Some(stringify!($variation)))
        };
        ($fighter:ident, , $variation:ident) => {
            (stringify!($fighter), None, Some(stringify!($variation)))
        };
    }

    /// The table rows, handed to `item!` (the table) or `names!` (the diag dump of the constants' values).
    macro_rules! rows {
        ($m:ident) => {
            [
                $m!(FIGHTER_KIND_BUDDY, ITEM_KIND_BUDDYBOMB),
                $m!(FIGHTER_KIND_KOOPAJR, ITEM_KIND_MECHAKOOPA),
                $m!(FIGHTER_KIND_DAISY, , ITEM_VARIATION_DAISYDAIKON_1),
                $m!(FIGHTER_KIND_DAISY, , ITEM_VARIATION_DAISYDAIKON_6),
                $m!(FIGHTER_KIND_DAISY, , ITEM_VARIATION_DAISYDAIKON_7),
                $m!(FIGHTER_KIND_DAISY, , ITEM_VARIATION_DAISYDAIKON_8),
                $m!(FIGHTER_KIND_DAISY, ITEM_KIND_DOSEISAN),
                $m!(FIGHTER_KIND_DAISY, ITEM_KIND_BOMBHEI, ITEM_VARIATION_BOMBHEI_NORMAL),
                $m!(FIGHTER_KIND_DIDDY),
                $m!(FIGHTER_KIND_DIDDY, ITEM_KIND_DIDDYPEANUTS),
                $m!(FIGHTER_KIND_KROOL, ITEM_KIND_KROOLCROWN),
                $m!(FIGHTER_KIND_LINK, ITEM_KIND_LINKBOMB),
                $m!(FIGHTER_KIND_LINK, ITEM_KIND_LINKARROW),
                $m!(FIGHTER_KIND_ROCKMAN, ITEM_KIND_METALBLADE),
                $m!(FIGHTER_KIND_PACMAN, ITEM_KIND_PACMANCHERRY),
                $m!(FIGHTER_KIND_PACMAN, ITEM_KIND_PACMANSTRAWBERRY),
                $m!(FIGHTER_KIND_PACMAN, ITEM_KIND_PACMANORANGE),
                $m!(FIGHTER_KIND_PACMAN, ITEM_KIND_PACMANAPPLE),
                $m!(FIGHTER_KIND_PACMAN, ITEM_KIND_PACMANMELON),
                $m!(FIGHTER_KIND_PACMAN, ITEM_KIND_PACMANBOSS),
                $m!(FIGHTER_KIND_PACMAN, ITEM_KIND_PACMANBELL),
                $m!(FIGHTER_KIND_PACMAN, ITEM_KIND_PACMANKEY),
                $m!(FIGHTER_KIND_PEACH, , ITEM_VARIATION_PEACHDAIKON_1),
                $m!(FIGHTER_KIND_PEACH, , ITEM_VARIATION_PEACHDAIKON_6),
                $m!(FIGHTER_KIND_PEACH, , ITEM_VARIATION_PEACHDAIKON_7),
                $m!(FIGHTER_KIND_PEACH, , ITEM_VARIATION_PEACHDAIKON_8),
                $m!(FIGHTER_KIND_PEACH, ITEM_KIND_DOSEISAN),
                $m!(FIGHTER_KIND_PEACH, ITEM_KIND_BOMBHEI, ITEM_VARIATION_BOMBHEI_NORMAL),
                $m!(FIGHTER_KIND_RICHTER, ITEM_KIND_RICHTERHOLYWATER),
                $m!(FIGHTER_KIND_ROBOT, ITEM_KIND_ROBOTGYRO, ITEM_VARIATION_ROBOTGYRO_1P),
                $m!(FIGHTER_KIND_REFLET, ITEM_KIND_BOOK),
                $m!(FIGHTER_KIND_REFLET, ITEM_KIND_THUNDERSWORD),
                $m!(FIGHTER_KIND_SHEIK, ITEM_KIND_EXPLOSIONBOMB),
                $m!(FIGHTER_KIND_SIMON, ITEM_KIND_SIMONHOLYWATER),
                $m!(FIGHTER_KIND_SNAKE, ITEM_KIND_SNAKEGRENADE),
                $m!(FIGHTER_KIND_TOONLINK, ITEM_KIND_TOONLINKBOMB),
                $m!(FIGHTER_KIND_MURABITO, ITEM_KIND_WOOD),
                $m!(FIGHTER_KIND_YOUNGLINK, ITEM_KIND_YOUNGLINKBOMB),
            ]
        };
    }

    /// Entry `choice` is the item named `ITEM_NAMES[choice]` (the saved item choice indexes this table directly).
    pub static ALL_CHAR_ITEMS: [CharItem; 38] = rows!(item);

    /// The same rows by constant name, for the diag dump (`[ItemTable]`): `(fighter, item kind, variation)`.
    #[cfg(feature = "diag")]
    const NAMES: [(&str, Option<&str>, Option<&str>); 38] = rows!(names);

    /// Diag build, once, from the first practice frame (the game's constant table is built by then): every constant of the table,
    /// resolved through the game's own lookup (`game::lua_const`, package 2's mechanism), one `[ItemTable]` line per entry. A name
    /// the game does not know reads `MISSING`.
    #[cfg(feature = "diag")]
    pub(crate) fn dump_once() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static DONE: AtomicBool = AtomicBool::new(false);
        if DONE.swap(true, Ordering::AcqRel) {
            return;
        }
        let value = |name: &str| {
            // SAFETY: called from the per-fighter frame callback, which runs only after `lua_const_lookup` passed its guard.
            match unsafe { crate::runtime::game::lua_const(crate::model::cpu::hash::lua_bind_hash(name)) } {
                Some(value) => format!("{name}={value}"),
                None => format!("{name}=MISSING"),
            }
        };
        for (index, (fighter, kind, variation)) in NAMES.iter().enumerate() {
            crate::runtime::diag_log::diagnostic(format_args!(
                "[ItemTable] choice={index} {} {} {}",
                value(fighter),
                kind.map_or_else(|| "kind=article".to_string(), value),
                variation.map_or_else(|| "variation=none".to_string(), value),
            ));
        }
    }

    /// The turnip face the next generated turnip is forced to (1..=8), `None` for the game's own roll: package 3's override.
    pub struct TurnipChosen;
    pub static TURNIP_CHOSEN: TurnipChosen = TurnipChosen;

    impl Readable for TurnipChosen {
        type Value = Option<u32>;
        fn read_value(&self) -> Option<u32> {
            crate::runtime::features::state_restore::chosen_face()
        }
    }

    impl Assignable for TurnipChosen {
        type Value = Option<u32>;
        fn assign_value(&self, face: Option<u32>) {
            crate::runtime::features::state_restore::set_chosen_face(face);
        }
    }
}
