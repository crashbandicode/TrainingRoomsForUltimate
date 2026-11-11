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

