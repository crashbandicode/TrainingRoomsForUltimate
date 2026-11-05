//! PAUSE-HELPER hook sites (Ally Round 8 F2): an inline hook may replace up to 20 bytes (`native_seams` exact=20), so
//! no direct branch anywhere in main may land inside `[site+4, site+20)` of either hook site. `0x1558660` failed this
//! (`0x15581f0 b 0x1558664`); the install hook sits at `0x1558668`. Reads the owned 13.0.5 ELF linked next to the
//! stage (`<stage>/../research`), like `native_seams`' owned-ELF facts.

const SITES: [u64; 2] = [0x1558668, 0x1558f4c];

fn owned_elf() -> Option<Vec<u8>> {
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
fn target(pc: u64, w: u32) -> Option<u64> {
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
fn exec(d: &[u8]) -> Vec<(usize, u64, usize)> {
    let (phoff, phentsize, phnum) = (u64_at(d, 0x20) as usize, u16_at(d, 0x36), u16_at(d, 0x38));
    (0..phnum)
        .map(|i| phoff + i * phentsize)
        .filter(|h| u32::from_le_bytes(d[*h..*h + 4].try_into().unwrap()) == 1 && d[*h + 4] & 1 != 0)
        .map(|h| (u64_at(d, h + 8) as usize, u64_at(d, h + 16), u64_at(d, h + 32) as usize))
        .collect()
}

fn word(d: &[u8], at: u64) -> u32 {
    let (off, va, _) = exec(d).into_iter().find(|(_, va, size)| (*va..*va + *size as u64).contains(&at)).unwrap();
    let at = off + (at - va) as usize;
    u32::from_le_bytes(d[at..at + 4].try_into().unwrap())
}

/// `PAUSE_CONTRACT` as written in the runtime source: (site, its exact words).
fn contract() -> Vec<(u64, Vec<u32>)> {
    let src = include_str!("../src/runtime/roster/native_source_capacity.rs");
    let value = |name: &str| {
        let key = format!("const {name}: usize = 0x");
        let at = src.find(&key).unwrap_or_else(|| panic!("{name}")) + key.len();
        u64::from_str_radix(&src[at..at + src[at..].find(';').unwrap()], 16).unwrap()
    };
    let start = src.find("const PAUSE_CONTRACT").unwrap();
    let body = &src[start..start + src[start..].find("\n];").unwrap()];
    body.split("\n    (")
        .skip(1)
        .map(|row| {
            let list = &row[row.find('[').unwrap() + 1..row.find(']').unwrap()];
            let words = list.split(',').map(str::trim).filter(|w| !w.is_empty());
            let words = words.map(|w| w.strip_prefix("0x").map_or_else(|| w.parse().unwrap(), |h| u32::from_str_radix(h, 16).unwrap()));
            (value(&row[..row.find(',').unwrap()]), words.collect())
        })
        .collect()
}

/// Every exact-word contract row is 13.0.5's words, and the owner step the re-creation relies on (Round 8 F1) reads
/// `[G+0xe8+8*slot]` with G = `[0x5324680]` and calls `0x17eb000` only when it is zero.
#[test]
fn the_pause_contract_is_main_and_the_owner_step_creates_only_a_missing_owner() {
    let Some(d) = owned_elf() else { return };
    let rows = contract();
    assert_eq!(rows.iter().map(|row| row.0).collect::<Vec<_>>(), [0x1558668, 0x1558f4c, 0x4eb00c, 0x4eb750]);
    for (site, words) in &rows {
        assert_eq!(words.len(), 16, "0x{site:x}");
        for (n, expected) in words.iter().enumerate() {
            assert_eq!(word(&d, site + 4 * n as u64), *expected, "0x{site:x} word {n}");
        }
    }
    let adrp = word(&d, 0x4eb750);
    let page = (0x4eb750u64 & !0xfff) + ((((adrp >> 5) & 0x7ffff) << 2 | (adrp >> 29) & 3) as u64) * 0x1000;
    assert_eq!((adrp >> 24) & 0x9f, 0x90, "adrp");
    assert_eq!(page + 8 * ((word(&d, 0x4eb754) >> 10) & 0xfff) as u64, 0x5324680, "G = [0x5324680]");
    assert_eq!(8 * ((word(&d, 0x4eb75c) >> 10) & 0xfff), 0xe8, "the owner alias [G+8*slot+0xe8]");
    assert_eq!(word(&d, 0x4eb760) >> 24, 0xb5, "cbnz x8: a live owner skips the owner creation");
    assert_eq!(target(0x4eb760, word(&d, 0x4eb760)), Some(0x4eb770));
    assert_eq!(target(0x4eb76c, word(&d, 0x4eb76c)), Some(0x17eb000), "bl 0x17eb000, the owner creation");
}

/// F3: the install gate's hold (`InstallGate::hold`), with the gate's own tests, by path.
#[allow(dead_code)]
#[path = "../src/runtime/roster/native_install.rs"]
mod native_install;

#[test]
fn no_branch_lands_inside_either_pause_hook_site() {
    let Some(d) = owned_elf() else { return };
    let mut scanned = 0usize;
    for (off, va, size) in exec(&d) {
        for (n, word) in d[off..off + size].as_chunks::<4>().0.iter().enumerate() {
            let pc = va + 4 * n as u64;
            if let Some(to) = target(pc, u32::from_le_bytes(*word)) {
                for site in SITES {
                    assert!(!(site + 4..site + 20).contains(&to), "0x{pc:x} branches to 0x{to:x}, inside hook site 0x{site:x}");
                }
                scanned += 1;
            }
        }
    }
    assert!(scanned > 100_000, "the executable segment was found and scanned ({scanned} branches)");
    assert_eq!(target(0x15581f0, 0x1400_011d), Some(0x1558664), "the decoder reads the known branch next to 0x1558660");
}
