//! The game's path hash (`hash40`): the path's byte length in the top byte pair and the IEEE CRC-32 of the
//! bytes below it, `(len << 32) | crc32(path)`. ARCropolis and the game's archive address files by it
//! (SPEC-core-07 §4.5). The path is the archive-relative one, lower case, without a mount prefix.

/// `crc32` (IEEE 802.3, reflected, polynomial 0xedb88320).
pub const fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    let mut i = 0;
    while i < bytes.len() {
        crc ^= bytes[i] as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
            bit += 1;
        }
        i += 1;
    }
    !crc
}

pub const fn hash40(path: &str) -> u64 {
    ((path.len() as u64) << 32) | crc32(path.as_bytes()) as u64
}

