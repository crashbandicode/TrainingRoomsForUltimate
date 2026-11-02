//! Shared test support: the owned 13.0.5 `main` ELF (`<smash-training-lobby>/research/game-13.0.5/
//! main-13.0.5.elf`, one level above this worktree). A missing ELF fails the test unless
//! `SKIP_OWNED_ELF_FACT=1` is set, which prints a visible SKIPPED line (the fork's rule for these
//! FACT checks, carried over in `tests/carried_model_facade.rs`).

#![allow(dead_code)]

const ELF_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../research/game-13.0.5/main-13.0.5.elf");

/// One section of the ELF: its address (`main`-relative, `.text` maps at 0), file offset and size.
#[derive(Clone, Copy, Debug)]
pub struct Section {
    pub addr: usize,
    pub offset: usize,
    pub size: usize,
}

pub struct OwnedMain {
    /// `(symbol name, main-relative address)` of every function and object in the ELF's symbol table.
    pub symbols: Vec<(String, usize)>,
    pub bytes: Vec<u8>,
    pub text: Section,
    pub rodata: Section,
    pub data: Section,
}

impl OwnedMain {
    /// The 64 bytes at `main+offset` in `.text`, `.rodata` or `.data`.
    pub fn window(&self, offset: usize) -> [u8; 64] {
        let section = [self.text, self.rodata, self.data]
            .into_iter()
            .find(|s| offset >= s.addr && offset + 64 <= s.addr + s.size)
            .unwrap_or_else(|| panic!("{offset:#x} is not inside .text, .rodata or .data"));
        let at = section.offset + (offset - section.addr);
        let mut out = [0u8; 64];
        out.copy_from_slice(&self.bytes[at..at + 64]);
        out
    }

    /// `.text` as little-endian instruction words.
    pub fn text_words(&self) -> Vec<u32> {
        let (words, _) = self.bytes[self.text.offset..self.text.offset + self.text.size].as_chunks::<4>();
        words.iter().map(|w| u32::from_le_bytes(*w)).collect()
    }
}

fn sections(elf: &[u8]) -> Vec<(String, Section)> {
    let u16_at = |o: usize| u16::from_le_bytes([elf[o], elf[o + 1]]) as usize;
    let u32_at = |o: usize| u32::from_le_bytes([elf[o], elf[o + 1], elf[o + 2], elf[o + 3]]) as usize;
    let u64_at = |o: usize| {
        let mut word = [0u8; 8];
        word.copy_from_slice(&elf[o..o + 8]);
        u64::from_le_bytes(word) as usize
    };
    assert_eq!(&elf[..4], b"\x7fELF");
    let (shoff, shentsize, shnum, shstrndx) = (u64_at(0x28), u16_at(0x3a), u16_at(0x3c), u16_at(0x3e));
    let header = |i: usize| shoff + i * shentsize;
    let strtab = u64_at(header(shstrndx) + 0x18);
    (0..shnum)
        .map(|i| {
            let h = header(i);
            let name_at = strtab + u32_at(h);
            let len = elf[name_at..].iter().position(|b| *b == 0).unwrap();
            let name = String::from_utf8_lossy(&elf[name_at..name_at + len]).into_owned();
            (name, Section { addr: u64_at(h + 0x10), offset: u64_at(h + 0x18), size: u64_at(h + 0x20) })
        })
        .collect()
}

/// The owned `main`, or `None` (after a visible SKIPPED line) when `SKIP_OWNED_ELF_FACT` is set and
/// the file is absent.
pub fn owned_main(test: &str) -> Option<OwnedMain> {
    let bytes = match std::fs::read(ELF_PATH) {
        Ok(bytes) => bytes,
        Err(_) => {
            assert!(
                std::env::var_os("SKIP_OWNED_ELF_FACT").is_some(),
                "owned 13.0.5 ELF missing at {ELF_PATH}: link ../research, or set SKIP_OWNED_ELF_FACT=1 to skip"
            );
            println!("SKIPPED {test}: {ELF_PATH} absent");
            return None;
        }
    };
    let all = sections(&bytes);
    let find = |name: &str| {
        all.iter().find(|(n, _)| n == name).map(|(_, s)| *s).unwrap_or_else(|| panic!("no {name} section"))
    };
    let (text, rodata, data) = (find(".text"), find(".rodata"), find(".data"));
    let symbols = symbols(&bytes, &all);
    Some(OwnedMain { symbols, bytes, text, rodata, data })
}

/// The symbol table (`.symtab` with `.strtab`): `(name, value)` of every named symbol.
fn symbols(elf: &[u8], sections: &[(String, Section)]) -> Vec<(String, usize)> {
    let find = |name: &str| sections.iter().find(|(n, _)| n == name).map(|(_, s)| *s);
    let (Some(symtab), Some(strtab)) = (find(".symtab"), find(".strtab")) else { return Vec::new() };
    let u32_at = |o: usize| u32::from_le_bytes([elf[o], elf[o + 1], elf[o + 2], elf[o + 3]]) as usize;
    let u64_at = |o: usize| {
        let mut word = [0u8; 8];
        word.copy_from_slice(&elf[o..o + 8]);
        u64::from_le_bytes(word) as usize
    };
    (0..symtab.size / 24)
        .filter_map(|i| {
            let at = symtab.offset + i * 24;
            let name_at = strtab.offset + u32_at(at);
            let len = elf[name_at..].iter().position(|b| *b == 0)?;
            (len > 0).then(|| (String::from_utf8_lossy(&elf[name_at..name_at + len]).into_owned(), u64_at(at + 8)))
        })
        .collect()
}

impl OwnedMain {
    /// The address of the symbol called exactly `name`.
    pub fn symbol(&self, name: &str) -> Option<usize> {
        self.symbols.iter().find(|(n, _)| n == name).map(|(_, a)| *a)
    }
}

const COMMON_NRO_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../research/game-13.0.5/nro/lua2cpp_common.nro");

/// The owned 13.0.5 `common` script module (`lua2cpp_common.nro`, extracted from `data.arc`; local research,
/// never committed): its bytes and the address of every exported function by symbol name.
pub struct CommonNro {
    pub bytes: Vec<u8>,
    pub symbols: Vec<(String, usize)>,
}

impl CommonNro {
    /// The 64 bytes at the function called `symbol`.
    pub fn window(&self, symbol: &str) -> Option<[u8; 64]> {
        let at = self.symbols.iter().find(|(n, _)| n == symbol)?.1;
        let mut out = [0u8; 64];
        out.copy_from_slice(self.bytes.get(at..at + 64)?);
        Some(out)
    }
}

/// The module, or `None` (after a visible SKIPPED line) when `SKIP_OWNED_ELF_FACT` is set and the file is
/// absent, as `owned_main` does.
pub fn owned_common_nro(test: &str) -> Option<CommonNro> {
    let bytes = match std::fs::read(COMMON_NRO_PATH) {
        Ok(bytes) => bytes,
        Err(_) => {
            assert!(
                std::env::var_os("SKIP_OWNED_ELF_FACT").is_some(),
                "owned 13.0.5 common NRO missing at {COMMON_NRO_PATH}: extract it (IMPLEMENTATION-PROGRESS-pkg2.md), or set SKIP_OWNED_ELF_FACT=1 to skip"
            );
            println!("SKIPPED {test}: {COMMON_NRO_PATH} absent");
            return None;
        }
    };
    let u32_at = |o: usize| u32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]) as usize;
    let u64_at = |o: usize| {
        let mut word = [0u8; 8];
        word.copy_from_slice(&bytes[o..o + 8]);
        u64::from_le_bytes(word) as usize
    };
    assert_eq!(&bytes[0x10..0x14], b"NRO0");
    // The text segment starts at 0: the word at +4 is the MOD0 offset; MOD0 + 4 is the dynamic section's
    // offset from MOD0; DT_STRTAB (5) and DT_SYMTAB (6) give the symbol table.
    let mod0 = u32_at(4);
    assert_eq!(&bytes[mod0..mod0 + 4], b"MOD0");
    let dynamic = (mod0 as i64 + i64::from(u32_at(mod0 + 4) as u32 as i32)) as usize;
    let (mut symtab, mut strtab) = (0, 0);
    let mut at = dynamic;
    loop {
        let (tag, value) = (u64_at(at), u64_at(at + 8));
        at += 16;
        match tag {
            0 => break,
            5 => strtab = value,
            6 => symtab = value,
            _ => {}
        }
    }
    let symbols = (0..(strtab - symtab) / 24)
        .filter_map(|i| {
            let at = symtab + i * 24;
            let name_at = strtab + u32_at(at);
            let len = bytes[name_at..].iter().position(|b| *b == 0)?;
            let value = u64_at(at + 8);
            (value != 0 && len > 0).then(|| (String::from_utf8_lossy(&bytes[name_at..name_at + len]).into_owned(), value))
        })
        .collect();
    Some(CommonNro { bytes, symbols })
}
