//! The owned 13.0.5 main ELF linked next to the stage (`<stage>/../research`), read the way `pause_helper_sites.rs` and
//! `native_seams`' owned-ELF facts read it: its executable segments, a word at an address, and direct branch targets.

pub fn owned_elf() -> Option<Vec<u8>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../research/game-13.0.5/main-13.0.5.elf");
    match std::fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(_) if std::env::var_os("SKIP_OWNED_ELF_FACT").is_some() => None,
        Err(error) => panic!("owned 13.0.5 ELF missing at {path} ({error}): link <stage>/../research, or set SKIP_OWNED_ELF_FACT=1"),
    }
}

fn u16_at(d: &[u8], at: usize) -> usize {
    u16::from_le_bytes([d[at], d[at + 1]]) as usize
}

fn u64_at(d: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(d[at..at + 8].try_into().unwrap())
}

/// The target of a direct branch word at `pc`: B, BL, B.cond, CBZ/CBNZ, TBZ/TBNZ.
pub fn target(pc: u64, w: u32) -> Option<u64> {
    let rel = |imm: u32, bits: u32| ((imm << (32 - bits)) as i32 >> (32 - bits)) as i64 * 4;
    let off = match w {
        w if w >> 26 == 0b000101 || w >> 26 == 0b100101 => rel(w & 0x03ff_ffff, 26),
        w if w >> 24 == 0b0101_0100 || (w >> 25) & 0x3f == 0b011010 => rel((w >> 5) & 0x7ffff, 19),
        w if (w >> 25) & 0x3f == 0b011011 => rel((w >> 5) & 0x3fff, 14),
        _ => return None,
    };
    Some(pc.wrapping_add_signed(off))
}

/// The executable load segments: (file offset, address, size).
pub fn exec(d: &[u8]) -> Vec<(usize, u64, usize)> {
    let (phoff, phentsize, phnum) = (u64_at(d, 0x20) as usize, u16_at(d, 0x36), u16_at(d, 0x38));
    (0..phnum)
        .map(|i| phoff + i * phentsize)
        .filter(|h| u32::from_le_bytes(d[*h..*h + 4].try_into().unwrap()) == 1 && d[*h + 4] & 1 != 0)
        .map(|h| (u64_at(d, h + 8) as usize, u64_at(d, h + 16), u64_at(d, h + 32) as usize))
        .collect()
}

pub fn word(d: &[u8], at: u64) -> u32 {
    let (off, va, _) = exec(d).into_iter().find(|(_, va, size)| (*va..*va + *size as u64).contains(&at)).unwrap();
    let at = off + (at - va) as usize;
    u32::from_le_bytes(d[at..at + 4].try_into().unwrap())
}

/// Every direct branch in the executable segments whose target lies in `range`: (pc, target). Also returns how many
/// branches were decoded, so a caller can check the segments were found at all.
pub fn branches_into(d: &[u8], range: std::ops::Range<u64>) -> (Vec<(u64, u64)>, usize) {
    let (mut hits, mut scanned) = (Vec::new(), 0usize);
    for (off, va, size) in exec(d) {
        for (n, w) in d[off..off + size].as_chunks::<4>().0.iter().enumerate() {
            let pc = va + 4 * n as u64;
            if let Some(to) = target(pc, u32::from_le_bytes(*w)) {
                scanned += 1;
                if range.contains(&to) {
                    hits.push((pc, to));
                }
            }
        }
    }
    (hits, scanned)
}
