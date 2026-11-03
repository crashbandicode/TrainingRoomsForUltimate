#[path = "../src/model/swap/pristine_load.rs"]
mod pristine_load;

use pristine_load::{fingerprint, VerifiedLoad};

const BASE: usize = 0x1000_0000;
// Synthetic 592-byte range; no game instructions. Independent Python FNV oracle.
const EXPECTED: u64 = 0xb1244642cca7ced8;

fn fixture() -> [u8; 592] {
    std::array::from_fn(|i| (i % 251) as u8)
}

#[test]
fn pristine_proof_survives_the_later_owned_interior_patch() {
    let mut bytes = fixture();
    assert_eq!(fingerprint(&bytes), EXPECTED);
    let proof = VerifiedLoad::capture(BASE, || fingerprint(&bytes) == EXPECTED);
    // The observed reload observer starts 28 bytes into the checked range.
    bytes[28..44].fill(0xa5);
    assert_ne!(fingerprint(&bytes), EXPECTED);
    assert!(VerifiedLoad::permits(proof, BASE));
    // Original order: hashing after installation rejects this same range.
    let late = VerifiedLoad::capture(BASE, || fingerprint(&bytes) == EXPECTED);
    assert!(!VerifiedLoad::permits(late, BASE));
}

#[test]
fn existing_corruption_is_never_excused_as_an_owned_hook() {
    for offset in [0, 28, 43, 64, 591] {
        let mut bytes = fixture();
        bytes[offset] ^= 1;
        let proof = VerifiedLoad::capture(BASE, || fingerprint(&bytes) == EXPECTED);
        assert!(!VerifiedLoad::permits(proof, BASE), "offset {offset}");
    }
}

#[test]
fn successful_verification_is_bound_to_one_text_image() {
    let bytes = fixture();
    let proof = VerifiedLoad::capture(BASE, || fingerprint(&bytes) == EXPECTED);
    assert!(!VerifiedLoad::permits(proof, BASE + 0x1000));
    let proof = VerifiedLoad::capture(BASE, || fingerprint(&bytes) == EXPECTED);
    assert!(VerifiedLoad::permits(proof, BASE));
}

#[test]
fn absent_failed_or_zero_base_verification_cannot_activate() {
    assert!(!VerifiedLoad::permits(None, BASE));
    assert!(!VerifiedLoad::permits(VerifiedLoad::capture(BASE, || false), BASE));
    let proof = VerifiedLoad::capture(0, || panic!("zero base must not read native memory"));
    assert!(!VerifiedLoad::permits(proof, 0));
}
