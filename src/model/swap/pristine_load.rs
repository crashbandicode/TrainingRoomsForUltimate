//! A boot-local load-contract proof, captured before owned hooks change native code.

pub fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

/// Not Copy/Clone: activation consumes the proof for the verified text image.
pub struct VerifiedLoad {
    base: usize,
}

impl VerifiedLoad {
    pub fn capture(base: usize, verify: impl FnOnce() -> bool) -> Option<Self> {
        (base != 0 && verify()).then_some(Self { base })
    }

    pub fn permits(proof: Option<Self>, base: usize) -> bool {
        proof.is_some_and(|proof| proof.base == base)
    }
}
