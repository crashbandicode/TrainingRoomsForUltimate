//! Per-body identity view: the pure rule (WP-R; refactor/wp-r/CATALOG.md, D-KIRBY1-ROB1-RCA §4).
//!
//! Native keeps one identity per fighter entry in its resource owner `R`: the member kinds
//! `R+0x58/0x5c/0x60` (cell 0 is the "primary"), the costume `R+0x64` and the Mii record `R+0x68`.
//! Its readers assume that identity describes every body of the entry, which holds for every entry
//! native builds (one fighter, or Pokémon Trainer, Pyra/Mythra, Ice Climbers whose members share one
//! costume rule). The body bank breaks that: a window or a detached group holds different kinds on
//! one `R`, and a Phase B group build leaves the home window's `R+0x58..0x60` in place
//! (`native_body_bank.rs` `write_group_words`; the exchange moves the header only with a Mii).
//!
//! FACTs this module encodes (listings in `refactor/wp-r/re/`):
//! - The costume selector (`native_costume`) is inlined 20 times in 13.0.5 (`selector_scan.tsv`) and
//!   keys on `R+0x58`: R.O.B. (`0x2d`) swaps costumes 0 and 1 for every region code
//!   `g = *(main+0x523d00c)` above 13 or in 1..=10 (`orr #0x7fe; lsr; tbz`, e.g. `0x17e8494-0x17e84e0`),
//!   the three Mii fighters (`0x48..=0x4a`) take costume 0, every other kind takes `R+0x64`.
//! - The per-body value is the selector evaluated on the kind the computation serves. Body-scoped
//!   sites (init, setup, unload) know the body's kind; request-scoped sites (acquire, the
//!   load-completion mirror, release, the path builder, construct, HUD/article objects) know a
//!   requested kind, which is a body of `R` when it is in `R+0x58..0x60` or the bank published it
//!   for this `R`; any other kind is a foreign request (a Kirby copy, the `0x72`/`0x75` objects) and
//!   keeps native's key, the primary.
//! - A weapon's costume byte (`+0x9a`, weapon init `0x33a0490`, selector at `0x33a05a8`) is computed
//!   on its owner entry's `R` and keys the weapon's file node in `G68+0x48`
//!   (`weapon kind | costume << 25 | table`, `0x17eeee0` then the lookup `0x17def20` at `0x33a0974`).
//!   The fighter the weapon belongs to is native's own weapon-kind table `main+0x455e7e4`
//!   (`WEAPON_OWNER`, read by the per-fighter weapon dispatchers `0x33ac5e4`, `0x33ad51c`,
//!   `0x33ae47c`, `0x33af3dc`, `0x33b6b9c`). A body creates its article pools during its own init
//!   (`0x6081f8 → 0x3a7320 → … → 0x3a6408`), so the active body is not the weapon's fighter: the
//!   view keys the weapon as a request for its owner fighter (C116.1 R.O.B. Gyro `0xb7` crash).
//! - Init `0x607eb4` passes `R+0x58[sub]` to `prepare_entry 0x17f1a60` as the body's kind and the
//!   copy tuples it registers carry that kind with the selector's costume; the view keys that
//!   costume on the kind `prepare_entry` receives and leaves the kind alone. A Phase B group keeps
//!   the home window's header and roster, so that kind can differ from the body's; changing it
//!   alone would move the body into the Kirby (or non-Kirby) branch against a roster that still
//!   describes the home window (refactor/wp-r/CATALOG.md §3.1, an open finding for WP-K).
//!
//! Invariants (host-tested below): every entry native builds keys exactly as native does (I3); a
//! body's kind gives the same costume through a body scope and a request scope once it is a body of
//! `R` (I2); a published set only grows.

/// R.O.B.
pub const ROB: u32 = 0x2d;
/// Mii Brawler, Swordfighter and Gunner (`0x48..=0x4a`): the selector's costume-0 kinds.
pub const MII_FIRST: u32 = 0x48;
pub const MII_LAST: u32 = 0x4a;
/// An empty member word (`u32::MAX`, native `-1`).
pub const NO_KIND: u32 = u32::MAX;
/// The region code the selector reads, `main+0x523d00c` (u32).
pub const REGION_OFFSET: usize = 0x523d00c;
/// Kinds the published set can hold (fighter kinds are below `0x60`).
pub const PUBLISHABLE_KINDS: u32 = 128;

/// Whether the selector swaps R.O.B.'s costumes 0 and 1 for this region code: `g > 13` (unsigned
/// `b.hi`) or bit `g` of `0x7fe`, i.e. `g` in `1..=10`.
pub const fn region_swaps(region: u32) -> bool {
    region > 13 || (0x7feu32 >> region) & 1 != 0
}

/// The native costume selector for a body of `kind` with the entry costume `costume`.
pub const fn native_costume(kind: u32, costume: u32, region: u32) -> u32 {
    if kind == ROB {
        if region_swaps(region) {
            match costume {
                0 => 1,
                1 => 0,
                other => other,
            }
        } else {
            costume
        }
    } else if kind.wrapping_sub(MII_FIRST) < MII_LAST - MII_FIRST + 1 {
        0
    } else {
        costume
    }
}

/// The identity words of `R` the selector reads: members `R+0x58/0x5c/0x60`, costume `R+0x64`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub kinds: [u32; 3],
    pub costume: u32,
}

impl Header {
    /// Cell 0, the kind native keys on.
    pub const fn primary(&self) -> u32 {
        self.kinds[0]
    }

    /// `kind` is one of the header's member words.
    pub fn holds(&self, kind: u32) -> bool {
        kind != NO_KIND && self.kinds.contains(&kind)
    }
}

/// Kinds the bank made bodies of one `R` beyond its header (detached groups), for one `R` epoch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Published(pub u128);

impl Published {
    pub const EMPTY: Published = Published(0);

    /// The set with `kind` added; a kind outside `0..PUBLISHABLE_KINDS` is never stored.
    pub const fn with(self, kind: u32) -> Published {
        if kind < PUBLISHABLE_KINDS { Published(self.0 | (1u128 << kind)) } else { self }
    }

    pub const fn contains(self, kind: u32) -> bool {
        kind < PUBLISHABLE_KINDS && self.0 & (1u128 << kind) != 0
    }

    /// The two halves an atomic table stores (`low`, `high`).
    pub const fn halves(self) -> (u64, u64) {
        (self.0 as u64, (self.0 >> 64) as u64)
    }

    pub const fn from_halves(low: u64, high: u64) -> Published {
        Published(((high as u128) << 64) | low as u128)
    }
}

/// What a selector copy computes the costume for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// A body's own computation (init, setup, unload): the body's kind.
    Body(u32),
    /// A request for a kind on `R` (acquire, mirror, release, paths, construct, HUD/article objects).
    Request(u32),
}

/// The kind a selector copy keys on.
pub fn key(scope: Scope, header: &Header, published: Published) -> u32 {
    match scope {
        Scope::Body(kind) => kind,
        Scope::Request(kind) if header.holds(kind) || published.contains(kind) => kind,
        Scope::Request(_) => header.primary(),
    }
}

/// The per-body costume a selector copy must produce.
#[cfg(test)]
pub fn costume(scope: Scope, header: &Header, published: Published, region: u32) -> u32 {
    native_costume(key(scope, header, published), header.costume, region)
}

/// What every selector copy computes today (keyed on the primary): the vanilla oracle.
#[cfg(test)]
pub const fn native_selector(header: &Header, region: u32) -> u32 {
    native_costume(header.primary(), header.costume, region)
}

/// The Mii identity words loader configure (`0x651334–0x65137c`) writes into `R+0x68..0x7b` for a
/// staged descriptor: `R+0x68` (u16) = `FE+0x46`, `R+0x6c/0x70/0x74/0x78` = `FE+0x48/0x4a/0x4c/0x4e`
/// (u16 each, zero-extended). The same transcription as `body_bank::MiiIdentity` (MII-GROUPS-DESIGN
/// §1.1; `0x651358` `stp x8,x9,[x21,#0x68]` is guarded as "cached tokens -> O+68..77" in
/// `mii_ordinary_gate::headgear_valid`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfiguredMiiWords {
    /// `R+0x68`: the incoming Mii's record index (`FE+0x46`).
    pub index: u16,
    /// `R+0x6c..0x7c`: outfit, hat, colour, voice.
    pub tokens: [u32; 4],
}

/// W-M2-4 (refactor/wp-r/CATALOG.md §R68, `wp5c/R68-GATE-WRITER.md`): when `R+0x4c` bit0 is set at
/// `0x6510f4`, configure is skipped and the bind `0x651444 → 0x67e7d0` retires the record named by
/// whatever `R+0x68` holds. The gate writer installs these words first. `None` when the descriptor is
/// not an exact-Mii staging (`MiiIdentity::from_descriptor`: one Mii kind `0x48..=0x4a`, count 1, one
/// actor, empty padding cells) or names no record (`FE+0x46 == 0xffff`).
pub fn configured_mii_words(descriptor: &[u8; 0x70]) -> Option<ConfiguredMiiWords> {
    let identity = crate::engine::swap::body_bank::MiiIdentity::from_descriptor(descriptor)?;
    if identity.index == u16::MAX {
        return None;
    }
    Some(ConfiguredMiiWords { index: identity.index, tokens: identity.o_tokens() })
}

/// Native's weapon-kind → owner-fighter-kind table, `main+0x455e7e4` (u32 per weapon kind).
pub const WEAPON_OWNER_OFFSET: usize = 0x455e7e4;
/// Weapon kinds in that table (the word after it, `0x4520c4c`, is not a fighter kind).
pub const WEAPON_KINDS: usize = 0x267;

/// `main+0x455e7e4[0..0x267]` of 13.0.5, transcribed by `refactor/wp-r/tools/gen_weapon_owner.py`
/// (sha256 of the 0x267 source words `56f8e60afa35dcbeba23dadab4b7862f15e022512700f8c250b5746eae3cc8e0`;
/// every entry is at most `0x75`, the bound of native's switch at `0x33ac5f0`). The Switch side
/// compares it with the running game's table before installing any site.
pub static WEAPON_OWNER: [u8; WEAPON_KINDS] = [
    0x48, 0x48, 0x49, 0x49, 0x49, 0x49, 0x49, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a,
    0x4a, 0x4a, 0x4a, 0x4a, 0x4a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x02, 0x02,
    0x02, 0x02, 0x02, 0x02, 0x02, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x04, 0x04,
    0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x05, 0x05, 0x05, 0x05, 0x06, 0x06, 0x06, 0x06,
    0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x08, 0x08, 0x08, 0x08,
    0x08, 0x08, 0x08, 0x09, 0x09, 0x09, 0x09, 0x0b, 0x0b, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x0a,
    0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0d, 0x0d, 0x0d, 0x0e, 0x0e, 0x0e, 0x0f, 0x0f, 0x11,
    0x11, 0x11, 0x11, 0x10, 0x10, 0x10, 0x10, 0x1c, 0x1c, 0x1c, 0x1c, 0x1c, 0x1c, 0x1c, 0x1c, 0x1c,
    0x1c, 0x18, 0x18, 0x18, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x21, 0x21, 0x21, 0x1d, 0x1d, 0x1d,
    0x1e, 0x1e, 0x1e, 0x1e, 0x1e, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x2b, 0x2b, 0x2b,
    0x2b, 0x2b, 0x27, 0x27, 0x27, 0x27, 0x27, 0x27, 0x27, 0x27, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a,
    0x2a, 0x2a, 0x2a, 0x23, 0x2c, 0x2c, 0x2c, 0x2d, 0x2d, 0x2d, 0x2d, 0x2d, 0x2d, 0x2d, 0x2d, 0x2d,
    0x2d, 0x2e, 0x2e, 0x2e, 0x2e, 0x2e, 0x2e, 0x2e, 0x2e, 0x2f, 0x2f, 0x2f, 0x2f, 0x2f, 0x26, 0x26,
    0x26, 0x29, 0x29, 0x29, 0x29, 0x0c, 0x0c, 0x12, 0x12, 0x12, 0x12, 0x12, 0x13, 0x13, 0x13, 0x13,
    0x13, 0x13, 0x13, 0x16, 0x1f, 0x1f, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x32, 0x32, 0x32, 0x32,
    0x32, 0x32, 0x32, 0x34, 0x34, 0x34, 0x34, 0x34, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30,
    0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x36,
    0x36, 0x36, 0x36, 0x36, 0x36, 0x36, 0x36, 0x36, 0x38, 0x38, 0x38, 0x38, 0x38, 0x38, 0x3b, 0x3b,
    0x3b, 0x3b, 0x3b, 0x3b, 0x3b, 0x3b, 0x3b, 0x3b, 0x3b, 0x3b, 0x3b, 0x3b, 0x3a, 0x3a, 0x3a, 0x3a,
    0x3a, 0x3a, 0x3a, 0x3a, 0x39, 0x39, 0x39, 0x35, 0x35, 0x35, 0x35, 0x35, 0x35, 0x35, 0x37, 0x37,
    0x37, 0x37, 0x37, 0x37, 0x37, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31,
    0x31, 0x31, 0x31, 0x31, 0x31, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x3c, 0x3c, 0x3c, 0x3d, 0x3d,
    0x3d, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x28, 0x1a, 0x3e, 0x3e, 0x40, 0x40,
    0x40, 0x40, 0x40, 0x40, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x4b, 0x4b, 0x4b, 0x4b, 0x4b, 0x4b, 0x4b,
    0x4b, 0x24, 0x25, 0x25, 0x25, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
    0x41, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
    0x45, 0x45, 0x45, 0x45, 0x45, 0x45, 0x17, 0x17, 0x17, 0x17, 0x17, 0x17, 0x17, 0x43, 0x43, 0x43,
    0x43, 0x43, 0x43, 0x43, 0x43, 0x43, 0x1b, 0x42, 0x42, 0x4d, 0x47, 0x47, 0x47, 0x47, 0x44, 0x44,
    0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x00, 0x50, 0x50, 0x72, 0x72, 0x72, 0x72, 0x72, 0x75,
    0x75, 0x46, 0x46, 0x46, 0x46, 0x46, 0x46, 0x46, 0x46, 0x46, 0x46, 0x46, 0x46, 0x46, 0x46, 0x46,
    0x46, 0x46, 0x46, 0x46, 0x46, 0x46, 0x46, 0x46, 0x51, 0x51, 0x51, 0x51, 0x52, 0x52, 0x52, 0x52,
    0x52, 0x52, 0x52, 0x52, 0x52, 0x53, 0x53, 0x53, 0x53, 0x53, 0x53, 0x53, 0x53, 0x53, 0x53, 0x54,
    0x54, 0x54, 0x54, 0x54, 0x54, 0x54, 0x54, 0x54, 0x55, 0x55, 0x55, 0x55, 0x56, 0x56, 0x56, 0x56,
    0x56, 0x56, 0x56, 0x56, 0x56, 0x56, 0x57, 0x57, 0x57, 0x57, 0x57, 0x57, 0x57, 0x57, 0x57, 0x57,
    0x57, 0x57, 0x57, 0x57, 0x57, 0x57, 0x57, 0x57, 0x57, 0x57, 0x57, 0x57, 0x57, 0x57, 0x57, 0x58,
    0x58, 0x58, 0x58, 0x58, 0x58, 0x58, 0x58, 0x58, 0x58, 0x58, 0x58, 0x58, 0x58, 0x58, 0x58, 0x58,
    0x58, 0x58, 0x58, 0x58, 0x58, 0x58, 0x59, 0x59, 0x59, 0x59, 0x59, 0x59, 0x5a, 0x5a, 0x5a, 0x5a,
    0x5a, 0x5b, 0x5b, 0x5b, 0x5b, 0x5b, 0x5b, 0x5b, 0x5b, 0x5c, 0x5c, 0x5c, 0x5c, 0x5c, 0x5d, 0x5d,
    0x5d, 0x5d, 0x5d, 0x5d, 0x5d, 0x5d, 0x5d,
];

/// The fighter kind a weapon belongs to (`WEAPON_OWNER`), or `NO_KIND` for a word outside the
/// table (native reads the weapon kind with `ldrsw`, so a negative kind arrives here as a large
/// `u32`). Used as `Scope::Request(weapon_owner(kind))` on the weapon's owner `R`.
pub fn weapon_owner(weapon_kind: u32) -> u32 {
    WEAPON_OWNER.get(weapon_kind as usize).map_or(NO_KIND, |&k| u32::from(k))
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    

    const FOX: u32 = 0x07;
    const KIRBY: u32 = 0x06;
    const MARIO: u32 = 0x00;
    const LUCARIO: u32 = 0x2c;
    const GUNNER: u32 = 0x4a;
    const BRAWLER: u32 = 0x48;
    const JOKER: u32 = 0x52;

    fn header(kinds: [u32; 3], costume: u32) -> Header {
        Header { kinds, costume }
    }

    fn the_published_set_only_grows_and_ignores_unstorable_kinds() {
        let mut set = Published::EMPTY;
        for kind in [0u32, 1, ROB, 0x5d, 0x7f, 0x80, 0xffff, NO_KIND] {
            let before = set;
            set = set.with(kind);
            assert_eq!(set.0 & before.0, before.0, "monotone");
            assert_eq!(set.contains(kind), kind < PUBLISHABLE_KINDS);
        }
        let (low, high) = set.halves();
        assert_eq!(Published::from_halves(low, high), set);
        assert!(set.contains(0x7f) && !set.contains(0x7e));
    }

    /// An exact-Mii staged descriptor: count 1, one actor, empty padding, record `index`, tokens.
    fn mii_descriptor(kind: u32, index: u16, tokens: [u16; 4]) -> [u8; 0x70] {
        let mut d = [0u8; 0x70];
        d[0x14..0x18].copy_from_slice(&1u32.to_le_bytes());
        d[0x18..0x1c].copy_from_slice(&kind.to_le_bytes());
        d[0x1c..0x20].copy_from_slice(&NO_KIND.to_le_bytes());
        d[0x20..0x24].copy_from_slice(&NO_KIND.to_le_bytes());
        d[0x28..0x2c].copy_from_slice(&1u32.to_le_bytes());
        d[0x41] = 3;
        d[0x46..0x48].copy_from_slice(&index.to_le_bytes());
        for (i, t) in tokens.iter().enumerate() {
            d[0x48 + 2 * i..0x4a + 2 * i].copy_from_slice(&t.to_le_bytes());
        }
        d
    }

    fn configured_mii_words_are_the_state_six_index_and_tokens() {
        for kind in MII_FIRST..=MII_LAST {
            for (index, tokens) in [(0u16, [75u16, 125, 0, 37]), (7, [1, 2, 3, 4]), (0xfffe, [0xffff, 0, 0x8000, 9])] {
                let d = mii_descriptor(kind, index, tokens);
                let words = configured_mii_words(&d).expect("an exact-Mii staging");
                assert_eq!(words.index, index);
                assert_eq!(words.tokens, tokens.map(u32::from));
                // The same words the bank's state-6 transcription puts into R+0x68..0x7b.
                let identity = crate::engine::swap::body_bank::MiiIdentity::from_descriptor(&d).unwrap();
                let r = identity.resource_words([0xdead_0000; crate::engine::swap::body_bank::RESOURCE_WORDS]);
                assert_eq!(r[4] & 0xffff, u32::from(words.index));
                assert_eq!([r[5], r[6], r[7], r[8]], words.tokens);
            }
        }
    }

    // Weapon kinds named by skyline-smash `lua_const` (WEAPON_KIND_*), cross-checked against the table.
    const OLIMAR: u32 = 0x2b;

    /// I2 for every weapon kind: a weapon whose fighter is a body of `R` keys as that body's own
    /// registration; any other weapon (a Kirby copy's, a common one) keeps native's primary key.
    fn every_weapon_agrees_with_its_fighters_registration_or_stays_native() {
        let homes = [[OLIMAR, LUCARIO, ROB], [FOX, MARIO, LUCARIO], [KIRBY, FOX, NO_KIND], [ROB, JOKER, MARIO], [GUNNER, NO_KIND, NO_KIND]];
        let groups = [[ROB, LUCARIO], [BRAWLER, NO_KIND], [KIRBY, JOKER], [OLIMAR, GUNNER]];
        for home in homes {
            for group in groups {
                let published = group.iter().fold(Published::EMPTY, |p, &k| p.with(k));
                for &c in &[0u32, 1, 2, 7] {
                    let h = header(home, c);
                    for &region in &[0u32, 1, 11, 14] {
                        for w in 0..WEAPON_KINDS as u32 {
                            let owner = weapon_owner(w);
                            let keyed = costume(Scope::Request(owner), &h, published, region);
                            if h.holds(owner) || published.contains(owner) {
                                assert_eq!(keyed, costume(Scope::Body(owner), &h, published, region), "weapon {:#x}", w);
                            } else {
                                assert_eq!(keyed, native_selector(&h, region), "weapon {:#x}", w);
                            }
                        }
                    }
                }
            }
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_published_set_only_grows_and_ignores_unstorable_kinds", the_published_set_only_grows_and_ignores_unstorable_kinds as fn()),
            ("configured_mii_words_are_the_state_six_index_and_tokens", configured_mii_words_are_the_state_six_index_and_tokens as fn()),
            ("every_weapon_agrees_with_its_fighters_registration_or_stays_native", every_weapon_agrees_with_its_fighters_registration_or_stays_native as fn()),
        ]);
    }
    }
}
