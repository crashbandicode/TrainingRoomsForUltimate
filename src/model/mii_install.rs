//! #54 / #68: the whole CharInfo a CPU Mii install writes.
//!
//! A Mii that came from a replay, and a macro recorded on one, has its look but no create ID and no name: the capture
//! keeps CharInfo bytes `0x00..0x26` zero (cov-rec's fixture `rc4-gunner-mii` and the replay window's capture; MII.md
//! row 12 matches Miis without them). The game's own Mii setup for the Reload (`0x67e7d0` at loader state 6,
//! `mii::mii_setup`) checks the record's CharInfo first (`0x39c2930/0x39c2940`) and returns without binding the record's
//! resource pair when it does not pass. The body is then built with no Mii resource: Eden covrest-yoga-03 (2026-11-08,
//! `cov-named-mii`): `terminal ready comparison refusal ... reason=native resource pair absent`, 17 `Unmapped` console
//! lines (cov-rec's 16:08Z report: `main+0x357a914`, reads at 0x0 and 0x288). The same canonical Mii loaded from the
//! catalog (the native Gunner of covrest-14, with its create ID and name) bound its pair.
//!
//! So the install writes a whole CharInfo: the saved native Mii with the same look (MII.md row 12: "that one is used"),
//! otherwise the Mii's own bytes with a create ID and the "New Mii" name filled in. The look, the tokens and the
//! canonical identity stay the Mii's, so selections, receipts and catalog matches are unchanged.

use crate::engine::mii::inventory::{MiiConfig, TaggedId, CHARINFO_LEN, TAG_CHARA, TAG_HAT, TAG_OUTFIT};
use crate::engine::mii::lifetime::Configuration;
use crate::engine::mii::replay::Target;

/// CharInfo layout (`inventory::CHARINFO_MASK_END`): create ID, ten UTF-16 name units, the name's terminator.
const CREATE_ID: core::ops::Range<usize> = 0x00..0x10;
const NAME: core::ops::Range<usize> = 0x10..0x24;
const TERMINATOR: core::ops::Range<usize> = 0x24..0x26;
/// The empty-name fallback the native-default Mii already uses (`mii_editor_bridge::native_default`).
pub const FALLBACK_NAME: &str = "New Mii";

/// Whether `charinfo` carries a create ID and a name (what a capture leaves out).
pub fn has_identity(charinfo: &[u8; CHARINFO_LEN]) -> bool {
    has_create_id(charinfo) && has_name(charinfo)
}

fn has_create_id(charinfo: &[u8; CHARINFO_LEN]) -> bool {
    charinfo[CREATE_ID].iter().any(|byte| *byte != 0)
}

fn has_name(charinfo: &[u8; CHARINFO_LEN]) -> bool {
    charinfo[NAME.start] != 0 || charinfo[NAME.start + 1] != 0
}

/// The configuration a CPU install writes for `c`: `c` itself when it has a create ID and a name; otherwise the saved
/// native Mii among `natives` with the same canonical look; otherwise `c` with a create ID and [`FALLBACK_NAME`].
/// Allocation-free (a game callback calls it).
pub fn installable(c: Configuration, natives: &[MiiConfig]) -> Configuration {
    if has_identity(c.charinfo()) {
        return c;
    }
    let canonical = c.canonical();
    natives
        .iter()
        .filter(|native| native.canonical == canonical)
        .filter_map(|native| Configuration::from_saved(native).ok())
        .find(|native| has_identity(native.charinfo()))
        .unwrap_or_else(|| filled(c))
}

/// #54, reading B (primary 2026-11-08 23:03Z): a recorded exact-Mii target as Switch installs it ([`installable`]), the
/// configuration the CPU then holds. Comparisons stay exact, against this. A fighter target, and a Mii with its own
/// create ID and name, are unchanged. Allocation-free (the game callback calls it).
pub fn installed_target(target: Target, natives: &[MiiConfig]) -> Target {
    match target.configuration() {
        Some(c) => Target::mii(installable(c, natives)),
        None => target,
    }
}

/// `c` with the missing create ID and name written in; the look bytes and the tokens are untouched.
fn filled(c: Configuration) -> Configuration {
    let canonical = c.canonical();
    let mut charinfo = *c.charinfo();
    if !has_create_id(&charinfo) {
        charinfo[CREATE_ID].copy_from_slice(&create_id(canonical.hash()));
    }
    if !has_name(&charinfo) {
        charinfo[NAME].fill(0);
        for (i, unit) in FALLBACK_NAME.encode_utf16().enumerate() {
            charinfo[NAME.start + i * 2..NAME.start + i * 2 + 2].copy_from_slice(&unit.to_le_bytes());
        }
        charinfo[TERMINATOR].fill(0);
    }
    let (look, tokens) = (canonical.appearance, c.tokens());
    let tag = |kind: u8, row: u16, hash: u64| TaggedId((u64::from(kind) << 56) | (u64::from(row) << 40) | hash);
    Configuration::from_saved(&MiiConfig {
        slot_id: 0,
        slot_index: 0,
        name: String::new(),
        charinfo,
        raw_tags: [tag(TAG_OUTFIT, tokens.outfit, look.outfit), tag(TAG_HAT, tokens.hat, look.hat), tag(TAG_CHARA, 0, look.chara)],
        canonical,
        copies: Vec::new(),
    })
    .unwrap_or(c)
}

/// A create ID for a Mii that has none, from the hash of its look (the same Mii always gets the same ID), with the
/// version-4 and variant bits a saved Mii's create ID carries (covrest-14's native Gunner: byte 6 0x48, byte 8 0x96).
fn create_id(hash: u64) -> [u8; 16] {
    let mut id = [0u8; 16];
    id[..8].copy_from_slice(&hash.to_le_bytes());
    id[8..].copy_from_slice(&(hash.rotate_left(32) ^ 0x9e37_79b9_7f4a_7c15).to_le_bytes());
    id[6] = (id[6] & 0x0f) | 0x40;
    id[8] = (id[8] & 0x3f) | 0x80;
    id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::mii::inventory::{classify, RawSlot, CHARINFO_MASK_END};

    /// cov-rec's fixture `rc4-gunner-mii` (REPLAY185's Gunner), as the macro file stores it: create ID and name zero.
    const LOOK: [u8; CHARINFO_LEN] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 11, 0, 64, 64, 0, 0, 5, 9, 0, 0, 75, 8, 0, 10, 8, 7, 6, 1, 0, 18, 12, 8, 7, 6, 1, 1, 12, 2, 4, 9, 13, 8, 8, 6,
        1, 8, 5, 1, 8, 5, 17, 20, 5, 4, 1, 4, 2, 20, 0,
    ];
    const TAGS: [u64; 3] = [0x1300_4b14_67a2_d93a, 0x5a00_7d15_3956_e6b8, 0xc100_0012_57ab_1d87];
    /// The native Gunner of covrest-14 (Ally, 2026-11-08): its create ID; the name is "Spook".
    const SPOOK: [u8; 16] = [58, 163, 216, 244, 61, 106, 72, 81, 150, 161, 201, 137, 99, 0, 91, 180];

    fn saved(slot_id: u32, look: [u8; CHARINFO_LEN], create: [u8; 16], name: &str) -> MiiConfig {
        let mut charinfo = look;
        charinfo[CREATE_ID].copy_from_slice(&create);
        let mut units = [0u16; 10];
        for (unit, ch) in units.iter_mut().zip(name.encode_utf16()) {
            *unit = ch;
        }
        for (i, unit) in units.iter().enumerate() {
            charinfo[0x10 + i * 2..0x12 + i * 2].copy_from_slice(&unit.to_le_bytes());
        }
        classify(&RawSlot {
            index: 0, slot_id, name_len: name.encode_utf16().count() as u32, name_units: units, colour: 0, voice: 37,
            specials: [2, 2, 2, 1], tag_outfit: TaggedId(TAGS[0]), tag_hat: TaggedId(TAGS[1]),
            tag_chara: TaggedId(TAGS[2]), charinfo,
        })
        .unwrap()
    }

    fn replay(look: [u8; CHARINFO_LEN]) -> Configuration {
        let mut config = saved(0, look, [1; 16], "x");
        config.charinfo[..CHARINFO_MASK_END].fill(0);
        Configuration::from_saved(&config).unwrap()
    }

    #[test]
    fn a_replay_mii_installs_the_saved_mii_with_the_same_look() {
        let replay = replay(LOOK);
        assert!(!has_identity(replay.charinfo()), "the fixture has no create ID and no name");
        let other = {
            let mut look = LOOK;
            look[0x40] ^= 1;
            saved(2622, look, [9; 16], "Other")
        };
        let spook = saved(13348, LOOK, SPOOK, "Spook");
        let installed = installable(replay, &[other, spook.clone()]);
        assert_eq!(installed, Configuration::from_saved(&spook).unwrap(), "MII.md row 12: that one is used");
        assert_eq!(installed.canonical(), replay.canonical());
        assert_eq!(installed.tokens(), replay.tokens());
    }

    #[test]
    fn a_replay_mii_with_no_saved_twin_gets_a_create_id_and_the_new_mii_name() {
        let replay = replay(LOOK);
        let installed = installable(replay, &[]);
        let bytes = installed.charinfo();
        assert!(has_identity(bytes));
        assert_eq!(installed.canonical(), replay.canonical(), "the look is the Mii's");
        assert_eq!(installed.tokens(), replay.tokens());
        assert_eq!(bytes[TERMINATOR.end..], LOOK[TERMINATOR.end..]);
        assert_eq!((bytes[6] >> 4, bytes[8] >> 6), (4, 2), "a saved Mii's create ID bits (SPOOK: 0x48, 0x96)");
        let name: Vec<u16> =
            bytes[NAME].chunks(2).map(|unit| u16::from_le_bytes([unit[0], unit[1]])).take_while(|unit| *unit != 0).collect();
        assert_eq!(String::from_utf16(&name).unwrap(), FALLBACK_NAME);
        assert_eq!(bytes[TERMINATOR], [0, 0]);
        assert_eq!(installable(replay, &[]), installed, "the same Mii always gets the same create ID");
        let mut look = LOOK;
        look[0x40] ^= 1;
        assert_ne!(installable(self::replay(look), &[]).charinfo()[CREATE_ID], bytes[CREATE_ID]);
    }

    #[test]
    fn a_mii_with_its_own_create_id_and_name_is_installed_as_it_is() {
        let own = Configuration::from_saved(&saved(4155, LOOK, [7; 16], "Mine")).unwrap();
        assert!(has_identity(own.charinfo()));
        assert_eq!(installable(own, &[saved(13348, LOOK, SPOOK, "Spook")]), own);
        let mut nameless = saved(0, LOOK, [7; 16], "x");
        nameless.charinfo[NAME].fill(0);
        assert!(!has_identity(&nameless.charinfo), "a create ID without a name is not whole");
    }

    /// TASK-OPUS-rc6-open-fixes item 2 (#54, cov-rest 20:22Z; Eden covrest-yoga-04): after a Switch the CPU holds what
    /// the install wrote, `installable(c)` (a replay Mii's filled create ID and name), so comparing it with the recorded
    /// `c` asked Fighter Mismatch again on the next Play. Reading B (primary 2026-11-08 23:03Z): exact equality, against
    /// what Switch installs.
    #[test]
    fn a_switched_mii_matches_its_recorded_target_as_installed() {
        let recorded = Target::mii(replay(LOOK));
        let spook = saved(13348, LOOK, SPOOK, "Spook");
        for natives in [Vec::new(), vec![spook]] {
            let held = installable(replay(LOOK), &natives);
            let kind = recorded.kind();
            assert!(!recorded.ready(kind, Some(held)), "the recorded configuration has no create ID or name");
            let target = installed_target(recorded, &natives);
            assert!(target.ready(kind, Some(held)), "what Switch installed matches, exactly");
            let mut look = LOOK;
            look[0x40] ^= 1;
            assert!(!target.ready(kind, Some(installable(replay(look), &natives))), "another look never matches");
            assert!(!target.ready(kind, None), "no configuration is no match");
        }
        let fighter = Target::fighter(0x01);
        assert_eq!(installed_target(fighter, &[]), fighter, "a fighter target is unchanged");
        let own = Target::mii(Configuration::from_saved(&saved(4155, LOOK, [7; 16], "Mine")).unwrap());
        assert_eq!(installed_target(own, &[saved(13348, LOOK, SPOOK, "Spook")]), own, "a whole Mii is installed as it is");
    }

    /// The CPU install (`mii_ordinary_cpu::begin`'s exact-Mii target) writes this module's configuration.
    #[test]
    fn the_cpu_install_writes_the_installable_configuration() {
        let cpu = include_str!("../runtime/mii/mii_ordinary_cpu.rs");
        assert!(cpu.contains("let c=crate::engine::mii_install::installable(c,&catalog.configs);"));
    }
}
