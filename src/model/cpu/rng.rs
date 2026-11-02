//! The plugin's own random number generator for every CPU-behaviour pick (SPEC-cpu-01 §3 and open
//! question 2): never the game's generator, so recordings, replays and the online desync checks see
//! the same game random stream with or without the plugin. A small SplitMix64 stream: no allocation,
//! one `u64` of state, seeded once at boot from the system tick and an address.

/// The generator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Mix two boot-time values into a seed (the tick and an address differ per boot and per load).
    pub const fn seed_from(tick: u64, entropy: u64) -> u64 {
        tick.rotate_left(17) ^ entropy.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `0..n` (`n > 0`; `0` for `n == 0`). The 64-bit product keeps the bias below 2^-32.
    pub fn below(&mut self, n: u32) -> u32 {
        ((u64::from(self.next_u32()) * u64::from(n)) >> 32) as u32
    }
}
