#[allow(dead_code, unused_imports)]
#[path = "../src/model/mii/mod.rs"]
pub mod mii;
#[allow(unused_imports)]
mod engine {
    pub use super::mii;
}

// Value rules of the Mii one-member group switch (MII-GROUPS-DESIGN §5): the
// bank variant of the headgear publication proof, the exact settle proof that
// replaces the canonical `completed()` for a bank switch, the presentation
// intent, and the owner-aware staging the planner derives identities with.
// Copied values only; no native read or write authority is claimed here.
use crate::engine::mii::backend::Layout;
use crate::engine::mii::inventory::{MiiClass, CHARINFO_LEN};
use crate::engine::mii::lifetime::{Configuration, Tokens};
use crate::engine::mii::native_values::RowIds;
use crate::engine::mii::ordinary_cpu::{staged_descriptor_for_owner, staged_descriptor_in, HatPublication};
use crate::engine::mii::ordinary_player::{
    bank_notice, bank_refusal_effects, staged_descriptor, ExactBankSettle, MiiBankSettle, RefusalEffects, Settle,
    MII_BANK_WINDOW_FLAGS,
};

fn configuration(class: MiiClass, hat: u16) -> Configuration {
    let mut charinfo = [0x5au8; CHARINFO_LEN];
    charinfo[0] = hat as u8;
    // Row ids carry their token in bits 40..56 (`RowIds::validate_tokens`).
    Configuration::from_observed(
        class,
        Tokens { outfit: 75, hat, colour: 0, voice: 37, specials: [2, 2, 2, 1] },
        charinfo,
        RowIds { outfit: (0x13u64 << 56) | (75u64 << 40) | 0x1bfef7a5f7, hat: (0x5au64 << 56) | (u64::from(hat) << 40) | 0x0fa9efa5ea },
    )
    .unwrap()
}

fn window() -> [u8; 0x70] {
    let mut window = [0u8; 0x70];
    window[0x14..0x18].copy_from_slice(&3u32.to_le_bytes());
    window[0x18..0x24].copy_from_slice(&[8, 0, 0, 0, 3, 0, 0, 0, 7, 0, 0, 0]);
    window[0x28] = 1;
    window[0x41] = 2;
    window[0x53] = 1;
    window
}

/// The settled values a correct Mii group build leaves for `c` at `owner`.
fn settled<'a>(c: Configuration, owner: u32, descriptor: &'a [u8; 0x70], record: &'a [u8; 104]) -> ExactBankSettle<'a> {
    let t = c.tokens();
    let index = u16::from_le_bytes([descriptor[0x46], descriptor[0x47]]);
    ExactBankSettle {
        owner,
        descriptor,
        e_tokens: t,
        d_outfit: t.outfit,
        d_colour: t.colour,
        d_specials: t.specials.map(u32::from),
        record,
        d_mirror: [index, t.outfit, t.hat, t.colour],
        o_header: [owner, c.canonical().class as u32, u32::MAX, u32::MAX, u32::from(descriptor[0x41])],
        o_index: index,
        o_tokens: [t.outfit, t.hat, t.colour, t.voice].map(u32::from),
        e_index: index,
        entry_flags: MII_BANK_WINDOW_FLAGS,
    }
}

fn published_record(c: Configuration) -> [u8; 104] {
    let mut record = [0u8; 104];
    record[..CHARINFO_LEN].copy_from_slice(c.charinfo());
    record[88..96].copy_from_slice(&0x7100_0000u64.to_le_bytes());
    record[96..104].copy_from_slice(&0x7200_0000u64.to_le_bytes());
    record
}

#[test]
fn owner_aware_staging_keeps_player_index_and_takes_the_cpu_slot() {
    let gunner = configuration(MiiClass::Gunner, 125);
    let mut window = window();
    window[0x46..0x48].copy_from_slice(&0u16.to_le_bytes());
    // Player0 is no selector on any layout: `staged_descriptor_in` refuses it, the
    // owner-aware rule stages the Player bytes with the window's own index.
    for layout in [Layout::Training, Layout::Ldn, Layout::Quickplay] {
        assert_eq!(staged_descriptor_in(layout, 0, window, Some(gunner)), None);
        let player = staged_descriptor_for_owner(layout, 0, window, Some(gunner)).unwrap();
        assert_eq!(player, staged_descriptor(window, Some(gunner)));
        assert_eq!(&player[0x46..0x48], &[0, 0]);
        assert_eq!(u32::from_le_bytes(player[0x18..0x1c].try_into().unwrap()), 0x4a);
        assert_eq!(&player[0x48..0x50], &[75, 0, 125, 0, 0, 0, 37, 0]);
        assert_eq!(player[0x43], 0);
    }
    // The Training CPU is selector 1; a waiting CPU is selector 4; a non-selector is refused.
    let cpu = staged_descriptor_for_owner(Layout::Training, 1, window, Some(gunner)).unwrap();
    assert_eq!(cpu, staged_descriptor_in(Layout::Training, 1, window, Some(gunner)).unwrap());
    assert_eq!(&cpu[0x46..0x48], &[1, 0]);
    let waiting = staged_descriptor_for_owner(Layout::Ldn, 4, window, Some(gunner)).unwrap();
    assert_eq!(&waiting[0x46..0x48], &[4, 0]);
    assert_eq!(staged_descriptor_for_owner(Layout::Training, 4, window, Some(gunner)), None);
    assert_eq!(staged_descriptor_for_owner(Layout::Training, 7, window, Some(gunner)), None);
    // Without a configuration the bytes are the window's (no identity to stage).
    assert_eq!(staged_descriptor_for_owner(Layout::Training, 0, window, None), Some(window));
    assert_eq!(staged_descriptor_for_owner(Layout::Training, 1, window, None), Some(window));
}

#[test]
fn bank_headgear_publication_replaces_fresh_o_with_the_group_hat_proof() {
    let gunner = configuration(MiiClass::Gunner, 125);
    let descriptor = staged_descriptor_in(Layout::Training, 1, window(), Some(gunner)).unwrap();
    let publication = HatPublication {
        layout: Layout::Training, owner: 1, source: false, opt_in: true,
        // The live window's resource owner is initialized: the canonical proof refuses it.
        initialized: true, fingerprints: true, before: 0, target: 125,
        destination_index: 1, original_index: 0, descriptor: &descriptor, payload: Some(gunner),
    };
    assert_eq!(publication.validate(), Err("unqualified headgear publication"));
    assert_eq!(publication.validate_bank(125), Ok(()));
    // The group must have prefetched exactly this hat; the payload must name it too.
    assert_eq!(publication.validate_bank(0), Err("Mii group hat differs from the published hat"));
    let other = configuration(MiiClass::Gunner, 0);
    assert_eq!(HatPublication { payload: Some(other), ..publication }.validate_bank(125),
        Err("Mii group hat differs from the published hat"));
    assert_eq!(HatPublication { payload: None, ..publication }.validate_bank(125),
        Err("Mii group publication without a payload"));
    // Every other canonical requirement still applies.
    assert_eq!(HatPublication { fingerprints: false, ..publication }.validate_bank(125), Err("unqualified headgear publication"));
    assert_eq!(HatPublication { opt_in: false, ..publication }.validate_bank(125), Err("unqualified headgear publication"));
    assert_eq!(HatPublication { owner: 7, ..publication }.validate_bank(125), Err("headgear row is not on this layout"));
    assert_eq!(HatPublication { destination_index: 0, ..publication }.validate_bank(125), Err("headgear payload/row mismatch"));
    let mut wrong_row = descriptor;
    wrong_row[0x4a..0x4c].copy_from_slice(&7u16.to_le_bytes());
    assert_eq!(HatPublication { descriptor: &wrong_row, ..publication }.validate_bank(125), Err("headgear descriptor/scalar mismatch"));
    let mut wrong_class = descriptor;
    wrong_class[0x18..0x1c].copy_from_slice(&0x48u32.to_le_bytes());
    assert_eq!(HatPublication { descriptor: &wrong_class, ..publication }.validate_bank(125), Err("headgear payload/row mismatch"));
    // An equal hat without opt-in keeps the equal-hat contract, group hat still proven.
    let same = HatPublication { before: 125, opt_in: false, ..publication };
    assert_eq!(same.validate_bank(125), Ok(()));
    assert_eq!(same.validate_bank(126), Err("Mii group hat differs from the published hat"));
    // Player0 on Training: its own row, destination index 0.
    let player = staged_descriptor(window(), Some(gunner));
    let p = HatPublication { owner: 0, destination_index: 0, descriptor: &player, ..publication };
    assert_eq!(p.validate_bank(125), Ok(()));
}

#[test]
fn exact_bank_settle_accepts_the_canonical_values_and_fails_closed_per_field() {
    let gunner = configuration(MiiClass::Gunner, 125);
    for owner in [0u32, 1] {
        let descriptor = staged_descriptor_for_owner(Layout::Training, owner as usize, window(), Some(gunner)).unwrap();
        let record = published_record(gunner);
        let good = settled(gunner, owner, &descriptor, &record);
        assert_eq!(good.verify(&gunner), Ok(()));
        // Another configuration of the class (hat) or another class is refused on its tokens.
        let other = configuration(MiiClass::Gunner, 0);
        assert!(good.verify(&other).is_err());
        assert!(good.verify(&configuration(MiiClass::Brawler, 125)).is_err());
        // Field by field.
        let mut canonical_flags = good;
        canonical_flags.entry_flags = 0;
        assert_eq!(canonical_flags.verify(&gunner), Err("Mii group settle: entry flags are not the bank flag"));
        let mut e = good;
        e.e_tokens.hat = 0;
        assert_eq!(e.verify(&gunner), Err("Mii group settle: E tokens differ from the configuration"));
        let mut d = good;
        d.d_colour = 3;
        assert_eq!(d.verify(&gunner), Err("Mii group settle: D scalars differ from the configuration"));
        let mut specials = good;
        specials.d_specials = [0; 4];
        assert_eq!(specials.verify(&gunner), Err("Mii group settle: D scalars differ from the configuration"));
        let mut stale = record;
        stale[0] ^= 0xff;
        assert_eq!(ExactBankSettle { record: &stale, ..good }.verify(&gunner), Err("Mii group settle: selector CharInfo differs"));
        // The resource clause is its own half (WP-M2 split): structural passes on a
        // pair-less record, `resources` names the pair, `verify` keeps the canonical answer.
        let mut unbuilt = record;
        unbuilt[88..96].fill(0);
        let pairless = ExactBankSettle { record: &unbuilt, ..good };
        assert_eq!(pairless.structural(&gunner), Ok(()));
        assert_eq!(pairless.resources(), Err("Mii group settle: native Mii resource pair absent"));
        assert_eq!(pairless.verify(&gunner), Err("Mii group settle: native Mii resource pair absent"));
        // The resource clause is hard (row 9): a pair-less settle diverges, it never degrades.
        assert_eq!(pairless.settle(&gunner).outcome(), Settle::Diverged("Mii group settle: native Mii resource pair absent"));
        let mut half = record;
        half[96..104].fill(0);
        assert_eq!(ExactBankSettle { record: &half, ..good }.resources(), Err("Mii group settle: native Mii resource pair absent"));
        assert_eq!(good.resources(), Ok(()));
        assert_eq!(good.settle(&gunner).outcome(), Settle::Adopted);
        // A structural failure names the body, not the pair.
        assert_eq!(ExactBankSettle { record: &stale, ..good }.settle(&gunner).outcome(),
            Settle::Diverged("Mii group settle: selector CharInfo differs"));
        let mut e_index = good;
        e_index.e_index ^= 1;
        assert_eq!(e_index.verify(&gunner), Err("Mii group settle: E index differs from the entry"));
        let mut mirror = good;
        mirror.d_mirror[2] = 0;
        assert_eq!(mirror.verify(&gunner), Err("Mii group settle: native D descriptor mirror mismatch"));
        let mut header = good;
        header.o_header[1] = 0x48;
        assert_eq!(header.verify(&gunner), Err("Mii group settle: native O header differs"));
        let mut costume = good;
        costume.o_header[4] = 3;
        assert_eq!(costume.verify(&gunner), Err("Mii group settle: native O header differs"));
        let mut o_index = good;
        o_index.o_index ^= 1;
        assert_eq!(o_index.verify(&gunner), Err("Mii group settle: native O index differs"));
        let mut tokens = good;
        tokens.o_tokens[1] = 0;
        assert_eq!(tokens.verify(&gunner), Err("Mii group settle: native O descriptor tokens mismatch"));
        // The entry itself must be the one-member Mii window with the configuration's tokens.
        let mut two = descriptor;
        two[0x1c..0x20].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(ExactBankSettle { descriptor: &two, ..good }.verify(&gunner), Err("Mii group settle: entry is not the one-member Mii window"));
        let mut wrong_tokens = descriptor;
        wrong_tokens[0x4a..0x4c].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(ExactBankSettle { descriptor: &wrong_tokens, ..good }.verify(&gunner), Err("Mii group settle: entry tokens differ from the configuration"));
    }
}

#[test]
fn a_settle_outcome_adopts_or_diverges_with_a_hard_resource_clause_and_never_retires_the_owner() {
    let gunner = configuration(MiiClass::Gunner, 125);
    let adopted = MiiBankSettle { structural: Ok(()), resources: Ok(()) }.outcome();
    let pairless = MiiBankSettle { structural: Ok(()), resources: Err("pair") }.outcome();
    let diverged = MiiBankSettle { structural: Err("body"), resources: Err("pair") }.outcome();
    let diverged_with_pair = MiiBankSettle { structural: Err("body"), resources: Ok(()) }.outcome();
    assert_eq!(adopted, Settle::Adopted);
    // Row 9: the resource clause is hard — an absent pair diverges with its own reason.
    assert_eq!(pairless, Settle::Diverged("pair"));
    assert_eq!(diverged, Settle::Diverged("body"));
    assert_eq!(diverged_with_pair, Settle::Diverged("body"));
    // Only an adopted settle keeps the exact configuration (= the HUD intent, row 23).
    assert_eq!(adopted.active(Some(gunner)), Some(gunner));
    assert_eq!(pairless.active(Some(gunner)), None);
    assert_eq!(diverged.active(Some(gunner)), None);
    assert_eq!(pairless.diverged_reason(), Some("pair"));
    assert_eq!(diverged.diverged_reason(), Some("body"));
    assert_eq!(adopted.diverged_reason(), None);
    // Cascade guard (B124PRE D-MII2): no outcome and no refusal retires authority or
    // drops the owner; the pending track and request go, the origin stays.
    for s in [adopted, pairless, diverged] { assert!(!s.retires_authority()); }
    for diverged_flag in [false, true] {
        assert_eq!(bank_refusal_effects(diverged_flag),
            RefusalEffects { drop_bank: true, drop_request: true, keep_owner: true, retire_authority: false });
    }
}

#[test]
fn bank_notices_name_the_role_and_the_exact_or_original_outcome() {
    assert_eq!(bank_notice(false, true, false), "CPU Mii configuration applied");
    assert_eq!(bank_notice(true, false, false), "Player fighter applied");
    assert_eq!(bank_notice(false, false, false), "CPU fighter applied");
    assert_eq!(bank_notice(true, false, true), "Original fighter restored");
    assert_eq!(bank_notice(false, true, true), "Original fighter restored");
}
