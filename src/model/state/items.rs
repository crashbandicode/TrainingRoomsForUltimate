//! Restoring a held-item snapshot (SPEC-state-04): the item of a snapshot's `choice`, the fighter whose
//! data it needs, and the turnip face override. The mechanism is the fork's item engine (stage m5o
//! `src/character_items.rs`, ours-new; CARRY-OVER.md), which supersedes the spec's grant paths A to C:
//! the player's hand is cleared and `ItemModule::have_item(kind, variation)` is called for every item,
//! a turnip then gets its special-init action with its power, and an item that needs a fighter's own data
//! is granted only when a fighter of that kind is in the match.

use super::held_item::HeldItem;
use super::ids::C;

/// What a choice needs to be granted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spec {
    /// The `ITEM_KIND_*` constant.
    pub kind: C,
    /// The variation `have_item` is called with when the snapshot has none of its own (gyro colour and
    /// turnip face come from the snapshot).
    pub variation: Variation,
    /// The fighter whose data the item needs; `None` for items any fighter can make (Bob-omb, Mr.
    /// Saturn, Banana Peel).
    pub source: Option<C>,
    pub turnip: Option<Turnip>,
}

/// An item variation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variation {
    Number(i32),
    /// A game constant (Bob-omb's `NORMAL`).
    Constant(C),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turnip {
    Daisy,
    Peach,
}

const fn spec(kind: C, source: Option<C>) -> Option<Spec> {
    Some(Spec { kind, variation: Variation::Number(0), source, turnip: None })
}

const fn turnip(who: Turnip) -> Option<Spec> {
    let (kind, source) = match who {
        Turnip::Daisy => (C::ItemDaisyTurnip, C::KindDaisy),
        Turnip::Peach => (C::ItemPeachTurnip, C::KindPeach),
    };
    Some(Spec { kind, variation: Variation::Number(0), source: Some(source), turnip: Some(who) })
}

/// The spec of the fork's item `choice` (an index into `held_item::ITEM_NAMES`, SPEC-state-04 §2.1).
pub fn spec_for(choice: u8) -> Option<Spec> {
    match choice {
        0 => spec(C::ItemBuddyBomb, Some(C::KindBanjo)),
        1 => spec(C::ItemMechakoopa, Some(C::KindBowserJr)),
        2..=5 => turnip(Turnip::Daisy),
        6 => spec(C::ItemDoseisan, None),
        7 => Some(Spec { kind: C::ItemBombhei, variation: Variation::Constant(C::ItemBombheiNormal), source: None, turnip: None }),
        8 => spec(C::ItemBanana, None),
        9 => spec(C::ItemDiddyPeanuts, Some(C::KindDiddy)),
        10 => spec(C::ItemKroolCrown, Some(C::KindKrool)),
        11 => spec(C::ItemLinkBomb, Some(C::KindLink)),
        12 => spec(C::ItemLinkArrow, Some(C::KindLink)),
        13 => spec(C::ItemMetalBlade, Some(C::KindMegaMan)),
        14 => spec(C::ItemCherry, Some(C::KindPacMan)),
        15 => spec(C::ItemStrawberry, Some(C::KindPacMan)),
        16 => spec(C::ItemOrange, Some(C::KindPacMan)),
        17 => spec(C::ItemApple, Some(C::KindPacMan)),
        18 => spec(C::ItemMelon, Some(C::KindPacMan)),
        19 => spec(C::ItemGalaxian, Some(C::KindPacMan)),
        20 => spec(C::ItemBell, Some(C::KindPacMan)),
        21 => spec(C::ItemKey, Some(C::KindPacMan)),
        22..=25 => turnip(Turnip::Peach),
        26 => spec(C::ItemDoseisan, None),
        27 => Some(Spec { kind: C::ItemBombhei, variation: Variation::Constant(C::ItemBombheiNormal), source: None, turnip: None }),
        28 => spec(C::ItemRichterWater, Some(C::KindRichter)),
        29 => spec(C::ItemGyro, Some(C::KindRob)),
        30 => spec(C::ItemBook, Some(C::KindRobin)),
        31 => spec(C::ItemLevin, Some(C::KindRobin)),
        32 => spec(C::ItemBurstGrenade, Some(C::KindSheik)),
        33 => spec(C::ItemSimonWater, Some(C::KindSimon)),
        34 => spec(C::ItemSnakeGrenade, Some(C::KindSnake)),
        35 => spec(C::ItemToonBomb, Some(C::KindToonLink)),
        36 => spec(C::ItemWoodChip, Some(C::KindVillager)),
        37 => spec(C::ItemYoungBomb, Some(C::KindYoungLink)),
        _ => None,
    }
}

/// What a restore does for a snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Restore {
    /// Clear the player's hand (an `Empty` snapshot).
    Clear,
    /// An unsupported item, an invalid snapshot or a choice outside the table: leave the hand alone.
    Nothing,
    Grant { spec: Spec, variation: i32, timers: Timers },
}

/// The timers a restored item gets back, only when the live item's maxima equal the saved maxima (a
/// different resource version must not receive incompatible timers).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timers {
    pub lifetime: i32,
    pub lifetime_max: i32,
    pub fuse: i32,
    pub fuse_max: i32,
}

/// The restore for a snapshot.
pub fn restore_for(snapshot: &HeldItem) -> Restore {
    if !snapshot.valid() {
        return Restore::Nothing;
    }
    match *snapshot {
        HeldItem::Empty => Restore::Clear,
        HeldItem::Unsupported => Restore::Nothing,
        HeldItem::Item { choice, variation, lifetime, lifetime_max, fuse, fuse_max } => match spec_for(choice) {
            Some(spec) => Restore::Grant { spec, variation, timers: Timers { lifetime, lifetime_max, fuse, fuse_max } },
            None => Restore::Nothing,
        },
    }
}

/// Whether the timers are written: both live maxima equal the saved ones.
pub fn timers_apply(timers: &Timers, live_lifetime_max: i32, live_fuse_max: i32) -> bool {
    timers.lifetime_max == live_lifetime_max && timers.fuse_max == live_fuse_max
}

/// The face override a turnip grant sets while it is made: the chosen face is `variation + 1`.
#[cfg(test)]
pub fn turnip_face(variation: i32) -> u32 {
    (variation + 1) as u32
}

/// The face the probability getter of face `n` (1..=8) answers while `override_face` is set: certain
/// (58.0) for the chosen face, zero for the rest; `None` lets the game answer.
pub fn face_probability(n: u8, override_face: Option<u32>) -> Option<f32> {
    override_face.map(|face| if face == u32::from(n) { FACE_CHOSEN } else { 0.0 })
}

pub const FACE_CHOSEN: f32 = 58.0;

/// The choice a held item of `kind` and `variation` is saved as: the first choice whose spec has that
/// kind and accepts that variation. `kind_of` resolves a spec's kind constant to the live value.
#[cfg(test)]
pub fn choice_of(kind: i32, variation: i32, kind_of: impl Fn(C) -> i32) -> Option<u8> {
    (0..super::held_item::ITEM_NAMES.len() as u8).find(|choice| {
        spec_for(*choice).is_some_and(|spec| kind_of(spec.kind) == kind)
            && super::held_item::valid_variation(usize::from(*choice), variation)
    })
}
