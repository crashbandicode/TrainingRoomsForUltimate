//! Cycle-preload candidate settings.
//!
//! One menu choice and one INI describe the same six facts: how many upcoming
//! cycle groups to retain, whether those groups are fighters, hats, or both,
//! whether the run records sparse samples, how many fighter bodies each role
//! keeps ready in its own native window, whether extra built groups may be
//! swapped into that window (group swap), and the total number of ready fighter
//! bodies across both visible roles. Slot count 0 is off. The inclusive maximum
//! is [`crate::engine::preload::cycle_preload_capacity::MAX_TARGETS`]. Saved counts are stored
//! values; native helper capacity is separate. A bank member count is 0 (off) or
//! [`BANK_MEMBERS_MIN`] through [`crate::engine::swap::body_bank::NATIVE_MEMBERS`]. The ready
//! fighter total is [`READY_FIGHTERS_MIN`] through [`READY_FIGHTERS_MAX`].

use std::convert::TryFrom;
use std::fs::OpenOptions;
use std::io::{Read, Write};

use {crate::engine::compat::choice::byteflags, crate::engine::compat::choice::count};
use serde::Deserialize;

use crate::engine::swap::body_bank::NATIVE_MEMBERS;
use crate::engine::preload::cycle_preload_capacity::MAX_TARGETS;

pub const PATH: &str = "sd:/ultimate/TrainingRooms/cycle_preload.ini";
pub const MAX_BYTES: usize = 2048;
const MAX_LINES: usize = 32;
const MAX_LINE: usize = 160;

/// Cached choice read by scheduling code.
///
/// `slots == 0` preloads nothing while measurements can still record the Off control, even when `measurements`
/// is true. Nonzero counts are the saved values through [`MAX_TARGETS`]. `fighters` and `hats`
/// select which upcoming cycle groups that saved count names. `bank_members` is
/// the number of bodies a role keeps in its own native entry, including the
/// active fighter: 0 is off, otherwise [`BANK_MEMBERS_MIN`] through
/// [`NATIVE_MEMBERS`]. It is a requested size, not permission or capacity; the
/// runtime still budgets and qualifies every member. `group_swap` allows extra
/// built groups beyond the window for a role; `ready_fighters` is the total
/// number of built bodies across Player and CPU, including the two active
/// fighters, that those groups may reach. Both are bounded at runtime by the
/// fighter pool and process headroom. `ic_pair_bank` lets a normal Ice Climbers
/// (Popo and Nana) target own a three-cell bank with one ordinary kind, so
/// cycling between the pair and that kind is an in-window switch.
/// `original_bank` lets a role's slot-0 original (the CSS pick) be an ordinary
/// bank/group member, so returning to it is a warm switch instead of the
/// canonical Reload that retires every bank and group (EXCLUSIONS-AUDIT §9).
/// Since rc.6 a normal Ice Climbers original counts too while `ic_pair_bank` is
/// on: its pair-lead window or pair group serves the return.
/// `solo_popo_groups` admits Solo Popo as the first member of a detached swap
/// group (§4b) and `ic_pair_groups` lets a pair-lead window own swap groups
/// (§4c); both default on (B123-pre Eden rows 3 and 4 passed). `group_swap` is
/// the master switch for every detached group: with it off, no group of any
/// kind is planned or kept, whatever the per-kind switches say (every
/// `*_groups_enabled` predicate includes [`Settings::groups_enabled`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub slots: u8,
    pub fighters: bool,
    pub hats: bool,
    pub measurements: bool,
    pub bank_members: u8,
    pub group_swap: bool,
    pub ready_fighters: u8,
    pub ic_pair_bank: bool,
    pub original_bank: bool,
    pub solo_popo_groups: bool,
    pub ic_pair_groups: bool,
    /// Pokémon Trainer and Pyra/Mythra form their own detached swap groups
    /// (SPECIAL-GROUPS-DESIGN §2/§3); default on.
    pub native_groups: bool,
    /// The normal Ice Climbers pair as its own detached group (§5); default on
    /// (C116.1: the final Eden run decides, the adapter's shipped constant also
    /// gates it).
    pub pair_groups: bool,
    /// A lane Mii (preset or exact) as its own one-member swap group
    /// (MII-GROUPS-DESIGN); default on (C116.1: the final Eden run decides, the
    /// adapter's shipped constant also gates it). One Mii per role at a time.
    pub mii_groups: bool,
}

const fn default_slot_count() -> u8 {
    assert!(MAX_TARGETS <= u8::MAX as usize);
    MAX_TARGETS as u8
}

/// Smallest native bank: the active fighter plus one parked body, the same
/// lower bound as `body_bank::Members::from_forms` (an extra swap group may hold
/// a single body; a window bank never starts with one).
pub const BANK_MEMBERS_MIN: u8 = 2;

/// Largest native bank: the entry's physical body/context cells.
pub const BANK_MEMBERS_MAX: u8 = {
    assert!(NATIVE_MEMBERS <= u8::MAX as usize);
    assert!(NATIVE_MEMBERS >= BANK_MEMBERS_MIN as usize);
    NATIVE_MEMBERS as u8
};

/// 0 (off) or a native bank size.
pub const fn valid_bank_members(members: u8) -> bool {
    members == 0 || (members >= BANK_MEMBERS_MIN && members <= BANK_MEMBERS_MAX)
}

/// Fewest ready fighters: the two active fighters alone (no extra group).
pub const READY_FIGHTERS_MIN: u8 = 2;
/// Most ready fighters a configuration may ask for. The fighter pool holds 24
/// bodies; two visible roles, up to five helpers and a six-body reserve leave
/// room for this many built bodies. The runtime still bounds every group.
pub const READY_FIGHTERS_MAX: u8 = 12;
/// Operator target (DECISIONS 2026-11-02, P-HOT): ten hot fighters across Player and CPU, the two
/// active fighters included.
pub const READY_FIGHTERS_DEFAULT: u8 = 10;

pub const fn valid_ready_fighters(total: u8) -> bool {
    total >= READY_FIGHTERS_MIN && total <= READY_FIGHTERS_MAX
}

impl Settings {
    /// Missing file and the menu's built-in selection for Training and lobby waiting.
    ///
    /// The count is the shared maximum. Sources are both. Measurements start on.
    /// Slot 0 remains an explicit Off. Bank members are the native maximum.
    pub const DEFAULT: Self = Self {
        slots: default_slot_count(),
        fighters: true,
        hats: true,
        measurements: false,
        bank_members: BANK_MEMBERS_MAX,
        group_swap: true,
        ready_fighters: READY_FIGHTERS_DEFAULT,
        ic_pair_bank: true,
        original_bank: true,
        solo_popo_groups: true,
        ic_pair_groups: true,
        native_groups: true,
        pair_groups: true,
        mii_groups: true,
    };

    /// Malformed input. The count, the bank, group swap, the pair bank and the
    /// later switches stay off and the bad file is not rewritten.
    pub const FAIL_CLOSED: Self = Self {
        slots: 0,
        fighters: true,
        hats: true,
        measurements: true,
        bank_members: 0,
        group_swap: false,
        ready_fighters: READY_FIGHTERS_DEFAULT,
        ic_pair_bank: false,
        original_bank: false,
        solo_popo_groups: false,
        ic_pair_groups: false,
        native_groups: false,
        pair_groups: false,
        mii_groups: false,
    };

    /// Snapshot before startup load, and any cache word that is not a menu choice.
    pub const CLOSED: Self = Self {
        slots: 0,
        fighters: false,
        hats: false,
        measurements: false,
        bank_members: 0,
        group_swap: false,
        ready_fighters: 0,
        ic_pair_bank: false,
        original_bank: false,
        solo_popo_groups: false,
        ic_pair_groups: false,
        native_groups: false,
        pair_groups: false,
        mii_groups: false,
    };

    pub const fn selectable(self) -> bool {
        self.slots as usize <= MAX_TARGETS
            && matches!(
                (self.fighters, self.hats),
                (true, false) | (false, true) | (true, true)
            )
            && valid_bank_members(self.bank_members)
            && valid_ready_fighters(self.ready_fighters)
    }

    /// Extra groups exist only with a window bank and the switch on.
    pub const fn groups_enabled(self) -> bool {
        self.group_swap && self.slots != 0 && self.fighters && self.bank_members >= BANK_MEMBERS_MIN
    }

    /// The pair bank needs all three native cells (Popo, Nana, one extra kind),
    /// fighter preloading and the switch on.
    pub const fn pair_bank_enabled(self) -> bool {
        self.ic_pair_bank && self.slots != 0 && self.fighters && self.bank_members >= BANK_MEMBERS_MAX
    }

    /// The slot-0 original joins its role's bank/groups only with a window bank
    /// and the switch on (the adapter still requires an ordinary bankable kind).
    pub const fn original_bank_enabled(self) -> bool {
        self.original_bank && self.slots != 0 && self.fighters && self.bank_members >= BANK_MEMBERS_MIN
    }

    /// Solo Popo as a detached group's first member: needs group swap and the switch.
    pub const fn solo_popo_groups_enabled(self) -> bool {
        self.solo_popo_groups && self.groups_enabled()
    }

    /// A pair-lead window owning swap groups: needs the pair bank, group swap and the switch.
    pub const fn ic_pair_groups_enabled(self) -> bool {
        self.ic_pair_groups && self.groups_enabled() && self.pair_bank_enabled()
    }

    /// Pokémon Trainer / Pyra-Mythra as their own swap groups: needs group swap and the switch.
    pub const fn native_groups_enabled(self) -> bool {
        self.native_groups && self.groups_enabled()
    }

    /// The pair as its own swap group: needs the pair bank, group swap and the switch.
    pub const fn pair_groups_enabled(self) -> bool {
        self.pair_groups && self.groups_enabled() && self.pair_bank_enabled()
    }

    /// A lane Mii as its own one-member swap group: needs group swap and the switch.
    pub const fn mii_groups_enabled(self) -> bool {
        self.mii_groups && self.groups_enabled()
    }
}

// Menu rows. The JSON keys match these submenu ids.
byteflags! {
    pub struct CyclePreloadSlots {
        pub OFF = "Off",
        pub ONE = "1",
        pub TWO = "2",
        pub THREE = "3",
        pub FOUR = "4",
        pub FIVE = "5",
    }
}

const _: () = assert!(count!(OFF ONE TWO THREE FOUR FIVE) == MAX_TARGETS + 1);
const _: () = assert!(CyclePreloadSlots::ALL_CONSTS.len() == MAX_TARGETS + 1);

byteflags! {
    pub struct CyclePreloadSources {
        pub FIGHTERS = "Fighters",
        pub HATS = "Hats",
        pub BOTH = "Both",
    }
}

// Off, then one choice per native bank size. Changing NATIVE_MEMBERS must
// change these rows; the assertions below refuse a silent mismatch.
byteflags! {
    pub struct CyclePreloadBankMembers {
        pub OFF = "Off",
        pub TWO = "2",
        pub THREE = "3",
    }
}

const BANK_CHOICES: usize = (BANK_MEMBERS_MAX - BANK_MEMBERS_MIN) as usize + 2;
const _: () = assert!(count!(OFF TWO THREE) == BANK_CHOICES);
const _: () = assert!(CyclePreloadBankMembers::ALL_CONSTS.len() == BANK_CHOICES);

pub const fn default_slots() -> CyclePreloadSlots {
    CyclePreloadSlots::ALL_CONSTS[MAX_TARGETS]
}

/// Menu row index for a bank member count: Off is 0, then each native size.
const fn bank_index(members: u8) -> Option<usize> {
    if members == 0 {
        Some(0)
    } else if valid_bank_members(members) {
        Some((members - BANK_MEMBERS_MIN) as usize + 1)
    } else {
        None
    }
}

fn bank_members_at(index: usize) -> Option<u8> {
    if index == 0 {
        return Some(0);
    }
    u8::try_from(index - 1 + usize::from(BANK_MEMBERS_MIN))
        .ok()
        .filter(|members| valid_bank_members(*members))
}

/// Built-in menu selection: the native maximum.
pub const fn default_bank_members() -> CyclePreloadBankMembers {
    CyclePreloadBankMembers::ALL_CONSTS[BANK_CHOICES - 1]
}

// One row per allowed ready-fighter total, READY_FIGHTERS_MIN first. The
// assertions refuse a silent mismatch with the bounds above.
byteflags! {
    pub struct CyclePreloadReadyFighters {
        pub TWO = "2",
        pub THREE = "3",
        pub FOUR = "4",
        pub FIVE = "5",
        pub SIX = "6",
        pub SEVEN = "7",
        pub EIGHT = "8",
        pub NINE = "9",
        pub TEN = "10",
        pub ELEVEN = "11",
        pub TWELVE = "12",
    }
}

const READY_CHOICES: usize = (READY_FIGHTERS_MAX - READY_FIGHTERS_MIN) as usize + 1;
const _: () = assert!(count!(TWO THREE FOUR FIVE SIX SEVEN EIGHT NINE TEN ELEVEN TWELVE) == READY_CHOICES);
const _: () = assert!(CyclePreloadReadyFighters::ALL_CONSTS.len() == READY_CHOICES);

/// Menu row index for a ready-fighter total.
const fn ready_index(total: u8) -> Option<usize> {
    if valid_ready_fighters(total) {
        Some((total - READY_FIGHTERS_MIN) as usize)
    } else {
        None
    }
}

fn ready_fighters_at(index: usize) -> Option<u8> {
    u8::try_from(index + usize::from(READY_FIGHTERS_MIN))
        .ok()
        .filter(|total| valid_ready_fighters(*total))
}

/// Built-in menu selection: the operator's eight.
pub const fn default_ready_fighters() -> CyclePreloadReadyFighters {
    CyclePreloadReadyFighters::ALL_CONSTS[(READY_FIGHTERS_DEFAULT - READY_FIGHTERS_MIN) as usize]
}

/// Built-in menu selection: group swap on.
pub fn default_group_swap() -> crate::engine::compat::menu::OnOff {
    crate::engine::compat::menu::OnOff::ON
}

/// Built-in menu selection: Ice Climbers pair bank on.
pub fn default_ic_pair_bank() -> crate::engine::compat::menu::OnOff {
    crate::engine::compat::menu::OnOff::ON
}

/// Built-in menu selection: the slot-0 original joins its bank (on).
pub fn default_original_bank() -> crate::engine::compat::menu::OnOff {
    crate::engine::compat::menu::OnOff::ON
}

/// Built-in menu selection: Solo Popo in detached groups (on; B123-pre row 3 passed).
pub fn default_solo_popo_groups() -> crate::engine::compat::menu::OnOff {
    crate::engine::compat::menu::OnOff::ON
}

/// Built-in menu selection: pair-lead window owning groups (on; B123-pre row 4 passed).
pub fn default_ic_pair_groups() -> crate::engine::compat::menu::OnOff {
    crate::engine::compat::menu::OnOff::ON
}

/// Built-in menu selection: PT and Pyra/Mythra as their own swap groups (on).
pub fn default_native_groups() -> crate::engine::compat::menu::OnOff {
    crate::engine::compat::menu::OnOff::ON
}

/// Built-in menu selection: the Ice Climbers pair as its own swap group (on; C116.1).
pub fn default_pair_groups() -> crate::engine::compat::menu::OnOff {
    crate::engine::compat::menu::OnOff::ON
}

/// Built-in menu selection: a lane Mii as its own one-member swap group (on; C116.1).
pub fn default_mii_groups() -> crate::engine::compat::menu::OnOff {
    crate::engine::compat::menu::OnOff::ON
}

/// A menu file without this key keeps the built-in total. A list that is not
/// exactly one choice becomes the built-in total as well: an unreadable total
/// must not switch group swap off, and the startup INI overlay restores the
/// cached choice anyway.
pub fn deserialize_ready_fighters<'de, D>(deserializer: D) -> Result<CyclePreloadReadyFighters, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<u8>::deserialize(deserializer).map_err(serde::de::Error::custom)?;
    Ok(ready_from_values(&values).unwrap_or(default_ready_fighters()))
}

fn ready_from_values(values: &[u8]) -> Option<CyclePreloadReadyFighters> {
    if values.len() != CyclePreloadReadyFighters::ALL_CONSTS.len() {
        return None;
    }
    if values.iter().copied().any(|value| value > 1) {
        return None;
    }
    CyclePreloadReadyFighters::ALL_CONSTS.get(one_hot(values)?).copied()
}

/// A menu file without this key keeps the built-in selection. A list that is not
/// exactly one choice becomes Off; the startup INI overlay then restores the
/// cached choice.
pub fn deserialize_bank_members<'de, D>(deserializer: D) -> Result<CyclePreloadBankMembers, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<u8>::deserialize(deserializer).map_err(serde::de::Error::custom)?;
    Ok(bank_from_values(&values).unwrap_or(CyclePreloadBankMembers::OFF))
}

fn bank_from_values(values: &[u8]) -> Option<CyclePreloadBankMembers> {
    if values.len() != CyclePreloadBankMembers::ALL_CONSTS.len() {
        return None;
    }
    if values.iter().copied().any(|value| value > 1) {
        return None;
    }
    CyclePreloadBankMembers::ALL_CONSTS.get(one_hot(values)?).copied()
}

/// Startup cache value. A missing file uses the shared maximum, whether or not
/// its default could be saved. A rejected file stays off.
pub fn startup_settings(load: IniLoad) -> Settings {
    match load {
        IniLoad::Parsed(settings) | IniLoad::Created(settings) => settings,
        IniLoad::Unsaved(_) => Settings::DEFAULT,
        IniLoad::Rejected(_) => Settings::FAIL_CLOSED,
    }
}

pub fn default_sources() -> CyclePreloadSources {
    CyclePreloadSources::BOTH
}

pub fn default_measurements() -> crate::engine::compat::menu::OnOff {
    crate::engine::compat::menu::OnOff::OFF
}

/// Menu JSON may still store the older three-choice array. Missing tail entries
/// stay unselected. A list that is not one choice from Off through [`MAX_TARGETS`]
/// becomes Off.
pub fn deserialize_slots<'de, D>(deserializer: D) -> Result<CyclePreloadSlots, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<u8>::deserialize(deserializer).map_err(serde::de::Error::custom)?;
    Ok(slots_from_values(&values).unwrap_or(CyclePreloadSlots::OFF))
}

fn slots_from_values(values: &[u8]) -> Option<CyclePreloadSlots> {
    if values.is_empty() || values.len() > CyclePreloadSlots::ALL_CONSTS.len() {
        return None;
    }
    if values.iter().copied().any(|value| value > 1) {
        return None;
    }
    let mut padded = [0u8; CyclePreloadSlots::ALL_CONSTS.len()];
    padded[..values.len()].copy_from_slice(values);
    let index = one_hot(&padded)?;
    CyclePreloadSlots::ALL_CONSTS.get(index).copied()
}

const BANK_SHIFT: u32 = 16;
const GROUP_SWAP_BIT: u32 = 11;
const IC_PAIR_BIT: u32 = 12;
const ORIGINAL_BANK_BIT: u32 = 13;
const SOLO_POPO_GROUPS_BIT: u32 = 14;
const IC_PAIR_GROUPS_BIT: u32 = 15;
const READY_SHIFT: u32 = 24;
/// `ready_fighters` is at most [`READY_FIGHTERS_MAX`] (12): four bits.
const READY_MASK: u32 = 0xf;
const NATIVE_GROUPS_BIT: u32 = 28;
const PAIR_GROUPS_BIT: u32 = 29;
const MII_GROUPS_BIT: u32 = 30;

/// `slots` in bits 0..8, then fighters, hats, measurements, group swap (bit 11),
/// the Ice Climbers pair bank (bit 12), the original bank (bit 13), Solo Popo
/// groups (bit 14) and pair-lead groups (bit 15), `bank_members` in bits 16..24,
/// `ready_fighters` in bits 24..28, native (PT/Aegis) groups (bit 28), the
/// pair group (bit 29) and Mii groups (bit 30). A word that is not a selectable
/// choice unpacks as [`Settings::CLOSED`].
pub fn pack(settings: Settings) -> u32 {
    if !settings.selectable() {
        return 0;
    }
    let mut word = u32::from(settings.slots);
    if settings.fighters {
        word |= 1 << 8;
    }
    if settings.hats {
        word |= 1 << 9;
    }
    if settings.measurements {
        word |= 1 << 10;
    }
    if settings.group_swap {
        word |= 1 << GROUP_SWAP_BIT;
    }
    if settings.ic_pair_bank {
        word |= 1 << IC_PAIR_BIT;
    }
    if settings.original_bank {
        word |= 1 << ORIGINAL_BANK_BIT;
    }
    if settings.solo_popo_groups {
        word |= 1 << SOLO_POPO_GROUPS_BIT;
    }
    if settings.ic_pair_groups {
        word |= 1 << IC_PAIR_GROUPS_BIT;
    }
    if settings.native_groups {
        word |= 1 << NATIVE_GROUPS_BIT;
    }
    if settings.pair_groups {
        word |= 1 << PAIR_GROUPS_BIT;
    }
    if settings.mii_groups {
        word |= 1 << MII_GROUPS_BIT;
    }
    word | (u32::from(settings.bank_members) << BANK_SHIFT)
        | ((u32::from(settings.ready_fighters) & READY_MASK) << READY_SHIFT)
}

pub fn unpack(word: u32) -> Settings {
    let settings = Settings {
        slots: (word & 0xff) as u8,
        fighters: word & (1 << 8) != 0,
        hats: word & (1 << 9) != 0,
        measurements: word & (1 << 10) != 0,
        bank_members: ((word >> BANK_SHIFT) & 0xff) as u8,
        group_swap: word & (1 << GROUP_SWAP_BIT) != 0,
        ready_fighters: ((word >> READY_SHIFT) & READY_MASK) as u8,
        ic_pair_bank: word & (1 << IC_PAIR_BIT) != 0,
        original_bank: word & (1 << ORIGINAL_BANK_BIT) != 0,
        solo_popo_groups: word & (1 << SOLO_POPO_GROUPS_BIT) != 0,
        ic_pair_groups: word & (1 << IC_PAIR_GROUPS_BIT) != 0,
        native_groups: word & (1 << NATIVE_GROUPS_BIT) != 0,
        pair_groups: word & (1 << PAIR_GROUPS_BIT) != 0,
        mii_groups: word & (1 << MII_GROUPS_BIT) != 0,
    };
    if settings.selectable() {
        settings
    } else {
        Settings::CLOSED
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IniLoad {
    /// A present file, parsed.
    Parsed(Settings),
    /// No file existed; the default was written and read back.
    Created(Settings),
    /// No file existed and the default could not be saved (the reason). The
    /// session runs [`Settings::DEFAULT`], the missing-file choice, so the menu
    /// shows the defaults and a later menu apply saves what the menu shows
    /// instead of fail-closed values. The next boot tries to create it again.
    Unsaved(&'static str),
    /// A present file that could not be read or parsed, or a path whose state
    /// could not be read. It is left untouched and the session fails closed.
    Rejected(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Publish {
    /// Menu matches the cached choice, or the menu value cannot be stored.
    Keep(Settings),
    /// Write this choice and then cache it.
    Store(Settings),
}

/// Optional keys a parsed INI omitted (a file written before they existed).
/// `parse` fills them with the built-in choice; the first startup reconcile
/// replaces that choice with the saved menu selection through [`adopt_unset`]
/// and writes the INI, so a menu Off is honoured by an older INI as well.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Unset {
    pub bank_members: bool,
    pub group_swap: bool,
    pub ready_fighters: bool,
    pub ic_pair_bank: bool,
    pub original_bank: bool,
    pub solo_popo_groups: bool,
    pub ic_pair_groups: bool,
    pub native_groups: bool,
    pub pair_groups: bool,
    pub mii_groups: bool,
}

impl Unset {
    pub const NONE: Self = Self {
        bank_members: false,
        group_swap: false,
        ready_fighters: false,
        ic_pair_bank: false,
        original_bank: false,
        solo_popo_groups: false,
        ic_pair_groups: false,
        native_groups: false,
        pair_groups: false,
        mii_groups: false,
    };

    pub const fn any(self) -> bool {
        self.bank_members || self.group_swap || self.ready_fighters || self.ic_pair_bank
            || self.original_bank || self.solo_popo_groups || self.ic_pair_groups
            || self.native_groups || self.pair_groups || self.mii_groups
    }

    /// Ten-bit word for an atomic cache: bank_members, group_swap, ready_fighters,
    /// ic_pair_bank, original_bank, solo_popo_groups, ic_pair_groups, native_groups,
    /// pair_groups, mii_groups.
    pub const fn to_bits(self) -> u16 {
        (self.bank_members as u16)
            | ((self.group_swap as u16) << 1)
            | ((self.ready_fighters as u16) << 2)
            | ((self.ic_pair_bank as u16) << 3)
            | ((self.original_bank as u16) << 4)
            | ((self.solo_popo_groups as u16) << 5)
            | ((self.ic_pair_groups as u16) << 6)
            | ((self.native_groups as u16) << 7)
            | ((self.pair_groups as u16) << 8)
            | ((self.mii_groups as u16) << 9)
    }

    pub const fn from_bits(bits: u16) -> Self {
        Self {
            bank_members: bits & 1 != 0,
            group_swap: bits & 2 != 0,
            ready_fighters: bits & 4 != 0,
            ic_pair_bank: bits & 8 != 0,
            original_bank: bits & 16 != 0,
            solo_popo_groups: bits & 32 != 0,
            ic_pair_groups: bits & 64 != 0,
            native_groups: bits & 128 != 0,
            pair_groups: bits & 256 != 0,
            mii_groups: bits & 512 != 0,
        }
    }
}

/// Saved menu choices for the keys an older INI omitted. A key the INI carries
/// keeps the INI value; a menu list that is not exactly one choice keeps the
/// built-in choice. The result is never a non-selectable setting.
pub fn adopt_unset(settings: Settings, unset: Unset, menu: &crate::engine::compat::menu::TrainingModpackMenu) -> Settings {
    let mut adopted = settings;
    if unset.bank_members {
        if let Some(members) =
            one_hot(&menu.cycle_preload_bank_members.to_byte_vec()).and_then(bank_members_at)
        {
            adopted.bank_members = members;
        }
    }
    if unset.group_swap {
        if let Some(on) = on_off_choice(&menu.cycle_preload_group_swap) {
            adopted.group_swap = on;
        }
    }
    if unset.ready_fighters {
        if let Some(total) =
            one_hot(&menu.cycle_preload_ready_fighters.to_byte_vec()).and_then(ready_fighters_at)
        {
            adopted.ready_fighters = total;
        }
    }
    if unset.ic_pair_bank {
        if let Some(on) = on_off_choice(&menu.cycle_preload_ic_pair_bank) {
            adopted.ic_pair_bank = on;
        }
    }
    if unset.original_bank {
        if let Some(on) = on_off_choice(&menu.cycle_preload_original_bank) {
            adopted.original_bank = on;
        }
    }
    if unset.solo_popo_groups {
        if let Some(on) = on_off_choice(&menu.cycle_preload_solo_popo_groups) {
            adopted.solo_popo_groups = on;
        }
    }
    if unset.ic_pair_groups {
        if let Some(on) = on_off_choice(&menu.cycle_preload_ic_pair_groups) {
            adopted.ic_pair_groups = on;
        }
    }
    if unset.native_groups {
        if let Some(on) = on_off_choice(&menu.cycle_preload_native_groups) {
            adopted.native_groups = on;
        }
    }
    if unset.pair_groups {
        if let Some(on) = on_off_choice(&menu.cycle_preload_pair_groups) {
            adopted.pair_groups = on;
        }
    }
    if unset.mii_groups {
        if let Some(on) = on_off_choice(&menu.cycle_preload_mii_groups) {
            adopted.mii_groups = on;
        }
    }
    if adopted.selectable() {
        adopted
    } else {
        settings
    }
}

fn on_off_choice(value: &crate::engine::compat::menu::OnOff) -> Option<bool> {
    match one_hot(&value.to_byte_vec())? {
        0 => Some(true),
        1 => Some(false),
        _ => None,
    }
}

pub fn settings_from_menu(menu: &crate::engine::compat::menu::TrainingModpackMenu) -> Option<Settings> {
    let slots = one_hot(&menu.cycle_preload_slots.to_byte_vec())
        .and_then(|index| u8::try_from(index).ok())
        .filter(|slots| usize::from(*slots) <= MAX_TARGETS)?;
    let (fighters, hats) = match one_hot(&menu.cycle_preload_sources.to_byte_vec())? {
        0 => (true, false),
        1 => (false, true),
        2 => (true, true),
        _ => return None,
    };
    let measurements = match one_hot(&menu.cycle_preload_measurements.to_byte_vec())? {
        0 => true,
        1 => false,
        _ => return None,
    };
    let bank_members =
        bank_members_at(one_hot(&menu.cycle_preload_bank_members.to_byte_vec())?)?;
    let group_swap = match one_hot(&menu.cycle_preload_group_swap.to_byte_vec())? {
        0 => true,
        1 => false,
        _ => return None,
    };
    let ready_fighters =
        ready_fighters_at(one_hot(&menu.cycle_preload_ready_fighters.to_byte_vec())?)?;
    let ic_pair_bank = match one_hot(&menu.cycle_preload_ic_pair_bank.to_byte_vec())? {
        0 => true,
        1 => false,
        _ => return None,
    };
    let original_bank = on_off_choice(&menu.cycle_preload_original_bank)?;
    let solo_popo_groups = on_off_choice(&menu.cycle_preload_solo_popo_groups)?;
    let ic_pair_groups = on_off_choice(&menu.cycle_preload_ic_pair_groups)?;
    let native_groups = on_off_choice(&menu.cycle_preload_native_groups)?;
    let pair_groups = on_off_choice(&menu.cycle_preload_pair_groups)?;
    let mii_groups = on_off_choice(&menu.cycle_preload_mii_groups)?;
    let settings = Settings {
        slots,
        fighters,
        hats,
        measurements,
        bank_members,
        group_swap,
        ready_fighters,
        ic_pair_bank,
        original_bank,
        solo_popo_groups,
        ic_pair_groups,
        native_groups,
        pair_groups,
        mii_groups,
    };
    if settings.selectable() {
        Some(settings)
    } else {
        None
    }
}

fn on_off(on: bool) -> crate::engine::compat::menu::OnOff {
    if on {
        crate::engine::compat::menu::OnOff::ON
    } else {
        crate::engine::compat::menu::OnOff::OFF
    }
}

pub fn apply_settings(menu: &mut crate::engine::compat::menu::TrainingModpackMenu, settings: Settings) {
    let settings = if settings.selectable() {
        settings
    } else {
        Settings::FAIL_CLOSED
    };
    menu.cycle_preload_slots = CyclePreloadSlots::ALL_CONSTS
        .get(usize::from(settings.slots))
        .copied()
        .unwrap_or(CyclePreloadSlots::OFF);
    menu.cycle_preload_sources = match (settings.fighters, settings.hats) {
        (true, false) => CyclePreloadSources::FIGHTERS,
        (false, true) => CyclePreloadSources::HATS,
        _ => CyclePreloadSources::BOTH,
    };
    menu.cycle_preload_measurements = if settings.measurements {
        crate::engine::compat::menu::OnOff::ON
    } else {
        crate::engine::compat::menu::OnOff::OFF
    };
    menu.cycle_preload_bank_members = bank_index(settings.bank_members)
        .and_then(|index| CyclePreloadBankMembers::ALL_CONSTS.get(index))
        .copied()
        .unwrap_or(CyclePreloadBankMembers::OFF);
    menu.cycle_preload_group_swap = if settings.group_swap {
        crate::engine::compat::menu::OnOff::ON
    } else {
        crate::engine::compat::menu::OnOff::OFF
    };
    menu.cycle_preload_ready_fighters = ready_index(settings.ready_fighters)
        .and_then(|index| CyclePreloadReadyFighters::ALL_CONSTS.get(index))
        .copied()
        .unwrap_or(default_ready_fighters());
    menu.cycle_preload_ic_pair_bank = if settings.ic_pair_bank {
        crate::engine::compat::menu::OnOff::ON
    } else {
        crate::engine::compat::menu::OnOff::OFF
    };
    menu.cycle_preload_original_bank = on_off(settings.original_bank);
    menu.cycle_preload_solo_popo_groups = on_off(settings.solo_popo_groups);
    menu.cycle_preload_ic_pair_groups = on_off(settings.ic_pair_groups);
    menu.cycle_preload_native_groups = on_off(settings.native_groups);
    menu.cycle_preload_pair_groups = on_off(settings.pair_groups);
    menu.cycle_preload_mii_groups = on_off(settings.mii_groups);
}

pub fn publish_decision(previous: Settings, menu: &crate::engine::compat::menu::TrainingModpackMenu) -> Publish {
    let fallback = if previous.selectable() {
        previous
    } else {
        Settings::FAIL_CLOSED
    };
    match settings_from_menu(menu) {
        Some(requested) if requested == previous => Publish::Keep(requested),
        Some(requested) => Publish::Store(requested),
        None => Publish::Keep(fallback),
    }
}

pub fn render(settings: Settings) -> String {
    let settings = if settings.selectable() {
        settings
    } else {
        Settings::FAIL_CLOSED
    };
    let sources = match (settings.fighters, settings.hats) {
        (true, false) => "fighters",
        (false, true) => "hats",
        _ => "both",
    };
    let measurements = if settings.measurements { "on" } else { "off" };
    let group_swap = if settings.group_swap { "on" } else { "off" };
    let ic_pair_bank = if settings.ic_pair_bank { "on" } else { "off" };
    let original_bank = if settings.original_bank { "on" } else { "off" };
    let solo_popo_groups = if settings.solo_popo_groups { "on" } else { "off" };
    let ic_pair_groups = if settings.ic_pair_groups { "on" } else { "off" };
    let native_groups = if settings.native_groups { "on" } else { "off" };
    let pair_groups = if settings.pair_groups { "on" } else { "off" };
    let mii_groups = if settings.mii_groups { "on" } else { "off" };
    format!(
        "# Cycle preload candidate. slots is 0 (off) through {max}.\n\
         # sources is fighters, hats, or both. measurements is on or off.\n\
         # slots 0 preloads nothing while measurements can still record the Off control. Invalid files stay off.\n\
         # bank_members is 0 (off) or {bank_min} through {bank_max} ready bodies per role, including the active fighter.\n\
         # group_swap is on or off: extra built groups beyond the window, swapped in place. Off disables every detached group below.\n\
         # ready_fighters is {ready_min} through {ready_max} built fighters across Player and CPU, counting both active fighters.\n\
         # ic_pair_bank is on or off: a normal Ice Climbers target keeps Popo, Nana and one more fighter ready in its window.\n\
         # original_bank is on or off: the slot-0 original (CSS pick) joins its role's ready bodies, so returning to it is a swap.\n\
         # solo_popo_groups is on or off: Solo Popo may lead a detached swap group.\n\
         # ic_pair_groups is on or off: an Ice Climbers pair window may own swap groups.\n\
         # native_groups is on or off: Pokemon Trainer and Pyra/Mythra form their own swap groups.\n\
         # pair_groups is on or off: the Ice Climbers pair forms its own swap group.\n\
         # mii_groups is on or off: a Mii in the cycle (preset or exact) forms its own one-member swap group.\n\
         [cycle_preload]\n\
         slots={slots}\n\
         sources={sources}\n\
         measurements={measurements}\n\
         bank_members={bank_members}\n\
         group_swap={group_swap}\n\
         ready_fighters={ready_fighters}\n\
         ic_pair_bank={ic_pair_bank}\n\
         original_bank={original_bank}\n\
         solo_popo_groups={solo_popo_groups}\n\
         ic_pair_groups={ic_pair_groups}\n\
         native_groups={native_groups}\n\
         pair_groups={pair_groups}\n\
         mii_groups={mii_groups}\n",
        max = MAX_TARGETS,
        slots = settings.slots,
        bank_min = BANK_MEMBERS_MIN,
        bank_max = BANK_MEMBERS_MAX,
        bank_members = settings.bank_members,
        ready_min = READY_FIGHTERS_MIN,
        ready_max = READY_FIGHTERS_MAX,
        ready_fighters = settings.ready_fighters,
    )
}

pub fn parse(text: &str) -> Result<Settings, &'static str> {
    parse_unset(text).map(|(settings, _)| settings)
}

/// [`parse`] plus the optional keys the file omitted.
pub fn parse_unset(text: &str) -> Result<(Settings, Unset), &'static str> {
    if text.len() > MAX_BYTES {
        return Err("cycle preload ini exceeds 2048 bytes");
    }
    if text.bytes().any(|byte| byte == 0) || text.starts_with('\u{feff}') {
        return Err("cycle preload ini is not utf-8");
    }
    let mut slots = None;
    let mut sources = None;
    let mut measurements = None;
    let mut bank_members = None;
    let mut group_swap = None;
    let mut ready_fighters = None;
    let mut ic_pair_bank = None;
    let mut original_bank = None;
    let mut solo_popo_groups = None;
    let mut ic_pair_groups = None;
    let mut native_groups = None;
    let mut pair_groups = None;
    let mut mii_groups = None;
    let mut in_section = false;
    let mut lines = 0usize;
    for raw in text.lines() {
        lines += 1;
        if lines > MAX_LINES {
            return Err("cycle preload ini has too many lines");
        }
        if raw.len() > MAX_LINE {
            return Err("cycle preload line is too long");
        }
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let without_comment = trimmed.split('#').next().unwrap_or("").trim();
        if without_comment.is_empty() {
            continue;
        }
        if without_comment.starts_with('[') {
            if in_section || without_comment != "[cycle_preload]" {
                return Err("unexpected cycle preload section");
            }
            in_section = true;
            continue;
        }
        if !in_section {
            return Err("cycle preload ini is missing [cycle_preload]");
        }
        let Some((key, value)) = without_comment.split_once('=') else {
            return Err("malformed cycle preload line");
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty()
            || value.is_empty()
            || value.len() > 16
            || value.chars().any(char::is_whitespace)
        {
            return Err("malformed cycle preload line");
        }
        match key.to_ascii_lowercase().as_str() {
            "slots" if slots.is_none() => slots = Some(parse_slots(value)?),
            "sources" if sources.is_none() => sources = Some(parse_sources(value)?),
            "measurements" if measurements.is_none() => {
                measurements = Some(parse_measurements(value)?)
            }
            "bank_members" if bank_members.is_none() => {
                bank_members = Some(parse_bank_members(value)?)
            }
            "group_swap" if group_swap.is_none() => group_swap = Some(parse_group_swap(value)?),
            "ready_fighters" if ready_fighters.is_none() => {
                ready_fighters = Some(parse_ready_fighters(value)?)
            }
            "ic_pair_bank" if ic_pair_bank.is_none() => {
                ic_pair_bank = Some(parse_on_off(value, "ic_pair_bank must be on or off")?)
            }
            "original_bank" if original_bank.is_none() => {
                original_bank = Some(parse_on_off(value, "original_bank must be on or off")?)
            }
            "solo_popo_groups" if solo_popo_groups.is_none() => {
                solo_popo_groups = Some(parse_on_off(value, "solo_popo_groups must be on or off")?)
            }
            "ic_pair_groups" if ic_pair_groups.is_none() => {
                ic_pair_groups = Some(parse_on_off(value, "ic_pair_groups must be on or off")?)
            }
            "native_groups" if native_groups.is_none() => {
                native_groups = Some(parse_on_off(value, "native_groups must be on or off")?)
            }
            "pair_groups" if pair_groups.is_none() => {
                pair_groups = Some(parse_on_off(value, "pair_groups must be on or off")?)
            }
            "mii_groups" if mii_groups.is_none() => {
                mii_groups = Some(parse_on_off(value, "mii_groups must be on or off")?)
            }
            "slots" | "sources" | "measurements" | "bank_members" | "group_swap"
            | "ready_fighters" | "ic_pair_bank" | "original_bank" | "solo_popo_groups"
            | "ic_pair_groups" | "native_groups" | "pair_groups" | "mii_groups" => return Err("duplicate cycle preload key"),
            _ => return Err("unknown cycle preload key"),
        }
    }
    if !in_section {
        return Err("cycle preload ini is missing [cycle_preload]");
    }
    let (fighters, hats) = sources.ok_or("cycle preload ini is missing sources")?;
    let unset = Unset {
        bank_members: bank_members.is_none(),
        group_swap: group_swap.is_none(),
        ready_fighters: ready_fighters.is_none(),
        ic_pair_bank: ic_pair_bank.is_none(),
        original_bank: original_bank.is_none(),
        solo_popo_groups: solo_popo_groups.is_none(),
        ic_pair_groups: ic_pair_groups.is_none(),
        native_groups: native_groups.is_none(),
        pair_groups: pair_groups.is_none(),
        mii_groups: mii_groups.is_none(),
    };
    let settings = Settings {
        slots: slots.ok_or("cycle preload ini is missing slots")?,
        fighters,
        hats,
        measurements: measurements.ok_or("cycle preload ini is missing measurements")?,
        // Optional: a file written before these keys adopts the built-in choices
        // here; the startup reconcile then prefers the saved menu choice (`Unset`).
        bank_members: bank_members.unwrap_or(Settings::DEFAULT.bank_members),
        group_swap: group_swap.unwrap_or(Settings::DEFAULT.group_swap),
        ready_fighters: ready_fighters.unwrap_or(Settings::DEFAULT.ready_fighters),
        ic_pair_bank: ic_pair_bank.unwrap_or(Settings::DEFAULT.ic_pair_bank),
        original_bank: original_bank.unwrap_or(Settings::DEFAULT.original_bank),
        solo_popo_groups: solo_popo_groups.unwrap_or(Settings::DEFAULT.solo_popo_groups),
        ic_pair_groups: ic_pair_groups.unwrap_or(Settings::DEFAULT.ic_pair_groups),
        native_groups: native_groups.unwrap_or(Settings::DEFAULT.native_groups),
        pair_groups: pair_groups.unwrap_or(Settings::DEFAULT.pair_groups),
        mii_groups: mii_groups.unwrap_or(Settings::DEFAULT.mii_groups),
    };
    Ok((settings, unset))
}

pub fn load_ini(path: &str) -> IniLoad {
    load_ini_unset(path).0
}

/// [`load_ini`] plus the optional keys a parsed file omitted (`Unset::NONE` for
/// a created, recovered-and-complete or rejected file).
pub fn load_ini_unset(path: &str) -> (IniLoad, Unset) {
    match std::fs::metadata(path) {
        Ok(metadata) => load_present(path, &metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            recover_previous(path).unwrap_or_else(|| create_default(path))
        }
        Err(_) => (IniLoad::Rejected("could not read cycle preload ini"), Unset::NONE),
    }
}

/// A present path: read and parse it, never write it.
fn load_present(path: &str, metadata: &std::fs::Metadata) -> (IniLoad, Unset) {
    if !metadata.is_file() {
        return (IniLoad::Rejected("cycle preload ini is not a file"), Unset::NONE);
    }
    if metadata.len() > MAX_BYTES as u64 {
        return (IniLoad::Rejected("cycle preload ini exceeds 2048 bytes"), Unset::NONE);
    }
    read_existing(path)
}

pub fn store_ini(path: &str, settings: Settings) -> Result<(), &'static str> {
    if !settings.selectable() {
        return Err("refusing to store a closed cycle preload setting");
    }
    let text = render(settings);
    if text.len() > MAX_BYTES {
        return Err("rendered cycle preload ini exceeds 2048 bytes");
    }
    let Some(parent) = parent_dir(path) else {
        return Err("cycle preload ini path has no directory");
    };
    if std::fs::create_dir_all(parent).is_err() {
        return Err("could not create cycle preload directory");
    }
    let tmp = format!("{path}.tmp");
    if let Err(reason) = write_new(&tmp, &text) {
        let _ = std::fs::remove_file(&tmp);
        return Err(reason);
    }
    replace_live(path, &tmp)
}

fn parse_slots(value: &str) -> Result<u8, &'static str> {
    let bytes = value.as_bytes();
    if bytes.len() != 1 || !bytes[0].is_ascii_digit() {
        return Err("slots is outside the configured count");
    }
    let slots = bytes[0] - b'0';
    if usize::from(slots) > MAX_TARGETS {
        return Err("slots is outside the configured count");
    }
    Ok(slots)
}

fn parse_sources(value: &str) -> Result<(bool, bool), &'static str> {
    match value.to_ascii_lowercase().as_str() {
        "fighters" => Ok((true, false)),
        "hats" => Ok((false, true)),
        "both" => Ok((true, true)),
        _ => Err("sources must be fighters, hats, or both"),
    }
}

fn parse_bank_members(value: &str) -> Result<u8, &'static str> {
    let bytes = value.as_bytes();
    if bytes.len() != 1 || !bytes[0].is_ascii_digit() || !valid_bank_members(bytes[0] - b'0') {
        return Err("bank_members must be 0 (off) or a native bank size");
    }
    Ok(bytes[0] - b'0')
}

fn parse_measurements(value: &str) -> Result<bool, &'static str> {
    match value.to_ascii_lowercase().as_str() {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err("measurements must be on or off"),
    }
}

fn parse_group_swap(value: &str) -> Result<bool, &'static str> {
    parse_on_off(value, "group_swap must be on or off")
}

fn parse_on_off(value: &str, error: &'static str) -> Result<bool, &'static str> {
    match value.to_ascii_lowercase().as_str() {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(error),
    }
}

/// One or two decimal digits without a leading zero, inside the allowed range.
fn parse_ready_fighters(value: &str) -> Result<u8, &'static str> {
    let bytes = value.as_bytes();
    let total = match bytes {
        [digit] if digit.is_ascii_digit() => digit - b'0',
        [tens, ones] if (b'1'..=b'9').contains(tens) && ones.is_ascii_digit() => {
            (tens - b'0') * 10 + (ones - b'0')
        }
        _ => return Err("ready_fighters must be a total inside the allowed range"),
    };
    if !valid_ready_fighters(total) {
        return Err("ready_fighters must be a total inside the allowed range");
    }
    Ok(total)
}

fn one_hot(values: &[u8]) -> Option<usize> {
    let mut found = None;
    for (index, value) in values.iter().copied().enumerate() {
        if value == 0 {
            continue;
        }
        if value != 1 || found.is_some() {
            return None;
        }
        found = Some(index);
    }
    found
}

fn parent_dir(path: &str) -> Option<&str> {
    let (dir, file) = path.rsplit_once('/')?;
    if dir.is_empty() || file.is_empty() {
        None
    } else {
        Some(dir)
    }
}

fn read_existing(path: &str) -> (IniLoad, Unset) {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return (IniLoad::Rejected("could not read cycle preload ini"), Unset::NONE),
    };
    let mut bytes = Vec::new();
    if file
        .take((MAX_BYTES as u64) + 1)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return (IniLoad::Rejected("could not read cycle preload ini"), Unset::NONE);
    }
    if bytes.len() > MAX_BYTES {
        return (IniLoad::Rejected("cycle preload ini exceeds 2048 bytes"), Unset::NONE);
    }
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return (IniLoad::Rejected("cycle preload ini is not utf-8"), Unset::NONE);
    };
    match parse_unset(text) {
        Ok((settings, unset)) => (IniLoad::Parsed(settings), unset),
        Err(reason) => (IniLoad::Rejected(reason), Unset::NONE),
    }
}

fn recover_previous(path: &str) -> Option<(IniLoad, Unset)> {
    let previous = format!("{path}.previous");
    let metadata = std::fs::metadata(&previous).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES as u64 {
        return None;
    }
    std::fs::rename(&previous, path).ok()?;
    Some(read_existing(path))
}

/// The missing-file branch: write the default and prove it by reading it back.
///
/// No exclusive create. The pinned Skyline std (`sys/fs/switch.rs`,
/// `File::open`) asks `nn::fs::GetEntryType` first: for a present path
/// `create_new(true)` returns `AlreadyExists`, and a missing path (result 514,
/// `PathNotFound`) is created only when `create(true)` is set. `create_new(true)`
/// alone therefore fails a missing path with `ErrorKind::Other` ("Result code:
/// 514"), which is how the first boot used to fail closed (Eden E8, M4b). This
/// re-checks that the path is absent, saves the default through [`store_ini`]
/// (`create(true)` on a temporary file, then a rename: the path every menu apply
/// uses) and reads it back. A present file is only read. Only the plugin writes
/// this file, once at boot, so the check-then-write has no race to lose.
fn create_default(path: &str) -> (IniLoad, Unset) {
    match std::fs::metadata(path) {
        Ok(metadata) => return load_present(path, &metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return (IniLoad::Rejected("could not read cycle preload ini"), Unset::NONE),
    }
    if let Err(reason) = store_ini(path, Settings::DEFAULT) {
        return (IniLoad::Unsaved(reason), Unset::NONE);
    }
    let created = verify_default(read_existing(path));
    if matches!(created, IniLoad::Unsaved(_)) {
        // The file is the one written above, not a user's: leave the path
        // missing so the next boot creates it again.
        let _ = std::fs::remove_file(path);
    }
    (created, Unset::NONE)
}

/// The read-back of a freshly written default: `Created` only when it parses to
/// [`Settings::DEFAULT`] with every key present.
fn verify_default(readback: (IniLoad, Unset)) -> IniLoad {
    match readback {
        (IniLoad::Parsed(settings), unset) if settings == Settings::DEFAULT && !unset.any() => {
            IniLoad::Created(Settings::DEFAULT)
        }
        _ => IniLoad::Unsaved("default cycle preload ini did not read back"),
    }
}

fn write_new(path: &str, text: &str) -> Result<(), &'static str> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|_| "could not write cycle preload ini")?;
    if file.write_all(text.as_bytes()).is_err() || file.sync_all().is_err() {
        return Err("could not write cycle preload ini");
    }
    Ok(())
}

fn replace_live(path: &str, tmp: &str) -> Result<(), &'static str> {
    let previous = format!("{path}.previous");
    if std::fs::metadata(path).is_ok() {
        let _ = std::fs::remove_file(&previous);
        if std::fs::rename(path, &previous).is_err() {
            let _ = std::fs::remove_file(tmp);
            return Err("could not replace cycle preload ini");
        }
        if std::fs::rename(tmp, path).is_err() {
            if std::fs::rename(&previous, path).is_err() {
                return Err("could not restore cycle preload ini");
            }
            return Err("could not replace cycle preload ini");
        }
        let _ = std::fs::remove_file(&previous);
        return Ok(());
    }
    match std::fs::rename(tmp, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            let _ = std::fs::remove_file(tmp);
            Err("could not replace cycle preload ini")
        }
    }
}

