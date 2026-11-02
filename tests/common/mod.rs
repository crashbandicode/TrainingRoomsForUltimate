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
    Some(OwnedMain { bytes, text, rodata, data })
}
