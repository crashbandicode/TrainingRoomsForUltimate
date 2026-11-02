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

    /// Entry `choice` is the item named `ITEM_NAMES[choice]` (the saved item choice indexes this table directly).
    pub static ALL_CHAR_ITEMS: [CharItem; 38] = [
        item!(FIGHTER_KIND_BUDDY, ITEM_KIND_BUDDYBOMB),
        item!(FIGHTER_KIND_KOOPAJR, ITEM_KIND_MECHAKOOPA),
        item!(FIGHTER_KIND_DAISY, , ITEM_VARIATION_DAISYDAIKON_1),
        item!(FIGHTER_KIND_DAISY, , ITEM_VARIATION_DAISYDAIKON_6),
        item!(FIGHTER_KIND_DAISY, , ITEM_VARIATION_DAISYDAIKON_7),
        item!(FIGHTER_KIND_DAISY, , ITEM_VARIATION_DAISYDAIKON_8),
        item!(FIGHTER_KIND_DAISY, ITEM_KIND_DOSEISAN),
        item!(FIGHTER_KIND_DAISY, ITEM_KIND_BOMBHEI, ITEM_VARIATION_BOMBHEI_NORMAL),
        item!(FIGHTER_KIND_DIDDY),
        item!(FIGHTER_KIND_DIDDY, ITEM_KIND_DIDDYPEANUTS),
        item!(FIGHTER_KIND_KROOL, ITEM_KIND_KROOLCROWN),
        item!(FIGHTER_KIND_LINK, ITEM_KIND_LINKBOMB),
        item!(FIGHTER_KIND_LINK, ITEM_KIND_LINKARROW),
        item!(FIGHTER_KIND_ROCKMAN, ITEM_KIND_METALBLADE),
        item!(FIGHTER_KIND_PACMAN, ITEM_KIND_PACMANCHERRY),
        item!(FIGHTER_KIND_PACMAN, ITEM_KIND_PACMANSTRAWBERRY),
        item!(FIGHTER_KIND_PACMAN, ITEM_KIND_PACMANORANGE),
        item!(FIGHTER_KIND_PACMAN, ITEM_KIND_PACMANAPPLE),
        item!(FIGHTER_KIND_PACMAN, ITEM_KIND_PACMANMELON),
        item!(FIGHTER_KIND_PACMAN, ITEM_KIND_PACMANBOSS),
        item!(FIGHTER_KIND_PACMAN, ITEM_KIND_PACMANBELL),
        item!(FIGHTER_KIND_PACMAN, ITEM_KIND_PACMANKEY),
        item!(FIGHTER_KIND_PEACH, , ITEM_VARIATION_PEACHDAIKON_1),
        item!(FIGHTER_KIND_PEACH, , ITEM_VARIATION_PEACHDAIKON_6),
        item!(FIGHTER_KIND_PEACH, , ITEM_VARIATION_PEACHDAIKON_7),
        item!(FIGHTER_KIND_PEACH, , ITEM_VARIATION_PEACHDAIKON_8),
        item!(FIGHTER_KIND_PEACH, ITEM_KIND_DOSEISAN),
        item!(FIGHTER_KIND_PEACH, ITEM_KIND_BOMBHEI, ITEM_VARIATION_BOMBHEI_NORMAL),
        item!(FIGHTER_KIND_RICHTER, ITEM_KIND_RICHTERHOLYWATER),
        item!(FIGHTER_KIND_ROBOT, ITEM_KIND_ROBOTGYRO, ITEM_VARIATION_ROBOTGYRO_1P),
        item!(FIGHTER_KIND_REFLET, ITEM_KIND_BOOK),
        item!(FIGHTER_KIND_REFLET, ITEM_KIND_THUNDERSWORD),
        item!(FIGHTER_KIND_SHEIK, ITEM_KIND_EXPLOSIONBOMB),
        item!(FIGHTER_KIND_SIMON, ITEM_KIND_SIMONHOLYWATER),
        item!(FIGHTER_KIND_SNAKE, ITEM_KIND_SNAKEGRENADE),
        item!(FIGHTER_KIND_TOONLINK, ITEM_KIND_TOONLINKBOMB),
        item!(FIGHTER_KIND_MURABITO, ITEM_KIND_WOOD),
        item!(FIGHTER_KIND_YOUNGLINK, ITEM_KIND_YOUNGLINKBOMB),
    ];

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
