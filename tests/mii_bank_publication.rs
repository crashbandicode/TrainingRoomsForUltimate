#[allow(dead_code, unused_imports)]
#[path = "../src/model/mii/mod.rs"]
pub mod mii;
#[allow(unused_imports)]
mod engine {
    pub use super::mii;
}

// MII-PHASEB part 2: the switch-time publication of a Mii group switch is part of the
// transaction. Surface s2 (M4b diag `c3f6951f…`): #167/#372 `bank publish … scalars_written=true`,
// the switch refused after the Phase B rollback (#183/#388 `bank settled on another member`),
// and every later request refused `preflight current configuration changed` (#192, #409,
// #411) until Quit: the owner kept by the cascade guard still held the pre-publish receipt
// while native held the published scalars.
use crate::engine::mii::ordinary_player::*;

/// The fields of the owner's receipt the switch-time publication and the Phase B Mii
/// setup can touch, in the shape `same_owned` compares them (equality).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Receipt { descriptor: [u32; 4], scalars: [u16; 4], record: [u8; 0x68] }

/// `begin()`'s gate for the next request: `same_owned(now, owner.expected)`.
fn admits_next(expected: &Receipt, now: &Receipt) -> bool { expected == now }

fn before() -> Receipt {
    // Pikachu active in the [Samus, Fox, Pikachu] window; record 0 already carries the
    // group's CharInfo (prefetch publication adopted it into `expected`), pair 0/0.
    let mut record = [0u8; 0x68];
    record[..88].copy_from_slice(&[0x3a; 88]);
    Receipt { descriptor: [3, 3, 7, 8], scalars: [0, 0, 0, 0], record }
}

/// The window words of the published one-member Mii group (count 1, Gunner, two padding cells).
const PUBLISHED_GROUP: [u32; 4] = [1, 0x4a, u32::MAX, u32::MAX];

/// What `bank_publish` leaves: the same CharInfo, the Mii's E/D scalars (75/125/0/37).
fn published(r: Receipt) -> Receipt { Receipt { scalars: [75, 125, 0, 37], ..r } }

/// The compensation `bank_refused` runs (CharInfo and scalars back; native's pair untouched),
/// then its receipt, read back under G.
fn restore(before: &Receipt, now: &mut Receipt) -> RestoreReceipt {
    now.record[..88].copy_from_slice(&before.record[..88]);
    now.scalars = before.scalars;
    restore_receipt(&before.record, &now.record, now.descriptor == before.descriptor && now.scalars == before.scalars)
}

#[test]
fn a_refused_mii_switch_restores_its_publication_and_the_next_request_is_admitted() {
    let before = before();
    let effects = bank_refusal_effects(false);
    assert!(effects.keep_owner && !effects.retire_authority, "cascade guard: the owner and its receipt stay");
    let expected = before;
    // M3: the window was rolled back, native settled on the committed in-window member.
    let mut now = published(before);
    assert!(!admits_next(&expected, &now), "today's strand: the published scalars outlive the refusal");
    assert_eq!(publication_rollback(true, now.descriptor == PUBLISHED_GROUP), PublicationRollback::Restore);
    let receipt = restore(&before, &mut now);
    assert_eq!(receipt, RestoreReceipt::Exact);
    assert!(receipt.verified() && admits_next(&expected, &now), "the next request (e.g. Fox) is admitted without Quit");
}

#[test]
fn a_crossed_mii_setup_leaves_native_s_pair_which_the_receipt_adopts() {
    let before = before();
    let mut expected = before;
    let mut now = published(before);
    // The Phase B setup ran (`0x3759500` published the pair at `record+0x58`), the build failed
    // afterwards and the window rolled back: native owns that pair until its next bind.
    now.record[88..96].copy_from_slice(&0x1209a8cdf0u64.to_le_bytes());
    now.record[96..104].copy_from_slice(&0x11f98bb1a0u64.to_le_bytes());
    let receipt = restore(&before, &mut now);
    assert_eq!(receipt, RestoreReceipt::NativePairAdopted);
    assert!(!admits_next(&expected, &now));
    expected.record = now.record; // the adapter adopts the record under the verified receipt
    assert!(admits_next(&expected, &now));
    // A half pair is no native publication: it is not adopted.
    let mut half = published(before);
    half.record[88..96].copy_from_slice(&0x1209a8cdf0u64.to_le_bytes());
    assert_eq!(restore(&before, &mut half), RestoreReceipt::Differs);
}

#[test]
fn a_live_exchange_keeps_the_publication_and_nothing_published_restores_nothing() {
    // Live = the window *is* the published group; any other descriptor change is not "live".
    let before = before();
    let mut exchanged = published(before);
    exchanged.descriptor = PUBLISHED_GROUP;
    assert_eq!(publication_rollback(true, exchanged.descriptor == PUBLISHED_GROUP), PublicationRollback::KeepLive);
    assert_eq!(publication_rollback(false, false), PublicationRollback::Nothing);
    assert_eq!(publication_rollback(false, true), PublicationRollback::Nothing);
    // Another window (not the published group, not the pre-publish one): restored, then the
    // read-back does not verify, so the refusal is reported and diverged rather than "live".
    let mut moved = published(before);
    moved.descriptor = [2, 3, 7, u32::MAX];
    assert_eq!(publication_rollback(true, moved.descriptor == PUBLISHED_GROUP), PublicationRollback::Restore);
    assert_eq!(restore(&before, &mut moved), RestoreReceipt::Differs);
    let mut other_charinfo = before;
    other_charinfo.record[0] = 0;
    assert_eq!(restore_receipt(&before.record, &other_charinfo.record, true), RestoreReceipt::Differs);
    // Several records (a CPU's destination and current record) combine to the weakest.
    use RestoreReceipt::*;
    assert_eq!(Exact.and(Exact), Exact);
    assert_eq!(Exact.and(NativePairAdopted), NativePairAdopted);
    assert_eq!(NativePairAdopted.and(Differs), Differs);
    assert!(!Differs.verified() && NativePairAdopted.verified());
}
