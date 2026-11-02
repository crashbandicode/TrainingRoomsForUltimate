//! The key of the game's constant table.
//!
//! The game keeps its Lua constants (`FIGHTER_STATUS_KIND_*`, work ids, transition terms, ...) in a
//! table that `lib::lua_bind_get_value<int>(u64 key, int &out) -> bool` (`main+0x3728e10`, a guarded
//! site of this plugin) answers; the key is the 64-bit xxHash (XXH64, the standard algorithm) of the
//! constant's identifier with the seed `u64::MAX`. The spec says to resolve the constants through the
//! game's own table (SPEC-cpu-01 §3, "Situation predicates"), so no numeric id is written down in
//! this plugin: `ids::Table` resolves every name at run time.
//!
//! The seed and the algorithm are checked against the published test vectors of the third-party MIT
//! crate `lua_bind_hash` 1.0.1 (the strings `password`, `password1234` and a constant name, in
//! `tests/cpu_ids.rs`); the algorithm itself is XXH64.

const P1: u64 = 0x9E37_79B1_85EB_CA87;
const P2: u64 = 0xC2B2_AE3D_27D4_EB4F;
const P3: u64 = 0x1656_67B1_9E37_79F9;
const P4: u64 = 0x85EB_CA77_C2B2_AE63;
const P5: u64 = 0x27D4_EB2F_1656_67C5;

/// The seed the game's table uses.
pub const SEED: u64 = u64::MAX;

fn round(acc: u64, input: u64) -> u64 {
    acc.wrapping_add(input.wrapping_mul(P2)).rotate_left(31).wrapping_mul(P1)
}

fn merge(acc: u64, value: u64) -> u64 {
    (acc ^ round(0, value)).wrapping_mul(P1).wrapping_add(P4)
}

fn read_u64(bytes: &[u8], at: usize) -> u64 {
    let mut word = [0u8; 8];
    word.copy_from_slice(&bytes[at..at + 8]);
    u64::from_le_bytes(word)
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    let mut word = [0u8; 4];
    word.copy_from_slice(&bytes[at..at + 4]);
    u32::from_le_bytes(word)
}

/// XXH64 of `bytes` with `seed`.
pub fn xxh64(bytes: &[u8], seed: u64) -> u64 {
    let len = bytes.len();
    let mut at = 0usize;
    let mut hash;
    if len >= 32 {
        let mut v1 = seed.wrapping_add(P1).wrapping_add(P2);
        let mut v2 = seed.wrapping_add(P2);
        let mut v3 = seed;
        let mut v4 = seed.wrapping_sub(P1);
        while at + 32 <= len {
            v1 = round(v1, read_u64(bytes, at));
            v2 = round(v2, read_u64(bytes, at + 8));
            v3 = round(v3, read_u64(bytes, at + 16));
            v4 = round(v4, read_u64(bytes, at + 24));
            at += 32;
        }
        hash = v1.rotate_left(1).wrapping_add(v2.rotate_left(7)).wrapping_add(v3.rotate_left(12)).wrapping_add(v4.rotate_left(18));
        hash = merge(hash, v1);
        hash = merge(hash, v2);
        hash = merge(hash, v3);
        hash = merge(hash, v4);
    } else {
        hash = seed.wrapping_add(P5);
    }
    hash = hash.wrapping_add(len as u64);
    while at + 8 <= len {
        hash ^= round(0, read_u64(bytes, at));
        hash = hash.rotate_left(27).wrapping_mul(P1).wrapping_add(P4);
        at += 8;
    }
    if at + 4 <= len {
        hash ^= u64::from(read_u32(bytes, at)).wrapping_mul(P1);
        hash = hash.rotate_left(23).wrapping_mul(P2).wrapping_add(P3);
        at += 4;
    }
    while at < len {
        hash ^= u64::from(bytes[at]).wrapping_mul(P5);
        hash = hash.rotate_left(11).wrapping_mul(P1);
        at += 1;
    }
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(P2);
    hash ^= hash >> 29;
    hash = hash.wrapping_mul(P3);
    hash ^ (hash >> 32)
}

/// The key of a constant's identifier in the game's table.
pub fn lua_bind_hash(name: &str) -> u64 {
    xxh64(name.as_bytes(), SEED)
}

/// CRC-32 (IEEE 802.3, the zlib polynomial), bitwise: the low half of the game's `Hash40`.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// The game's `phx::Hash40` of a name: the CRC-32 in the low 32 bits and the length in bits 32..40.
pub fn hash40(name: &str) -> u64 {
    u64::from(crc32(name.as_bytes())) | ((name.len() as u64) << 32)
}
