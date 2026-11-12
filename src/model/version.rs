//! Version guard (playbook layout: pure logic in `model`, the game only in the Switch runtime).
//! It holds the game identity this build supports and the guard's comparison. Nothing here reads
//! or writes the game, allocates, or depends on the Switch target: it compiles for every target,
//! and `tests/carried_model_facade.rs` tests it.
//!
//! Carried over from the earlier fork's `src/model.rs` (stage m5o, ours-new); see CARRY-OVER.md for
//! the two edits made (the `training_mod_consts` re-export is gone, this doc comment).
//!
//! FACT (owned 13.0.5 main ELF `research/game-13.0.5/main-13.0.5.elf`, sha256 0789f0a4…536c2):
//! the GNU build-id note (`namesz` 4, `descsz` 0x14, `type` 3, `"GNU\0"`) is the last 0x24 bytes
//! of `.rodata`, at main+0x4f16f3c (`.rodata` 0x39c9000 + 0x154df3c; `.data` starts at 0x4f17000),
//! and its descriptor is the NSO build ID `21450c64…5d597d`. Upstream never had a version guard:
//! an unsupported game panicked inside a needle `.expect()` (error 2-0069, ROWS-ANALYSIS row 5).

/// The only game display version this build installs on.
pub const SUPPORTED_VERSION: &str = "13.0.5";
/// NSO build ID of the 13.0.5 `main` every offset and 64-byte guard in this build was read from.
#[cfg(test)]
pub const MAIN_BUILD_ID: &str = "21450c647b8c5940ed5b68218ea128fd3c5d597d";
/// `MAIN_BUILD_ID` as bytes (the note descriptor).
pub const MAIN_BUILD_ID_BYTES: [u8; 20] = [
    0x21, 0x45, 0x0c, 0x64, 0x7b, 0x8c, 0x59, 0x40, 0xed, 0x5b, 0x68, 0x21, 0x8e, 0xa1, 0x28, 0xfd,
    0x3c, 0x5d, 0x59, 0x7d,
];
/// main+offset of the 13.0.5 build-id note (documentation and a host check; the guard scans).
#[cfg(test)]
pub const MAIN_BUILD_ID_NOTE_OFFSET: usize = 0x4f16f3c;
/// How much of the end of `.rodata` the guard scans for the note (the note is the last 0x24 bytes
/// of `.rodata` in 13.0.5; one page also covers a differently sized tail on another build).
pub const BUILD_ID_SCAN_BYTES: usize = 0x1000;

const NOTE_HEADER: usize = 16;
const GNU_NAME: [u8; 4] = *b"GNU\0";
const NT_GNU_BUILD_ID: u32 = 3;

fn le32(bytes: &[u8], at: usize) -> Option<u32> {
    let word = bytes.get(at..at + 4)?;
    Some(u32::from_le_bytes([word[0], word[1], word[2], word[3]]))
}

/// The descriptor of a GNU build-id note that starts at `note[0]`, if it is one (1–32 bytes).
pub fn gnu_build_id_at(note: &[u8]) -> Option<&[u8]> {
    let name_size = le32(note, 0)?;
    let desc_size = le32(note, 4)? as usize;
    if name_size != 4 || le32(note, 8)? != NT_GNU_BUILD_ID || !(1..=32).contains(&desc_size) {
        return None;
    }
    if note.get(12..NOTE_HEADER)? != GNU_NAME {
        return None;
    }
    note.get(NOTE_HEADER..NOTE_HEADER + desc_size)
}

/// The last 4-aligned GNU build-id note in `tail` (the end of `.rodata`): its offset in `tail` and
/// its descriptor.
pub fn find_gnu_build_id(tail: &[u8]) -> Option<(usize, &[u8])> {
    let mut at = tail.len().checked_sub(NOTE_HEADER + 1)? & !3;
    loop {
        if let Some(id) = gnu_build_id_at(&tail[at..]) {
            return Some((at, id));
        }
        at = at.checked_sub(4)?;
    }
}

/// `DisplayVersion.name` up to its NUL.
pub fn display_version(name: &[u8]) -> &[u8] {
    let end = name.iter().position(|b| *b == 0).unwrap_or(name.len());
    &name[..end]
}

/// The version guard's decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Build ID and display version are 13.0.5's.
    Supported,
    /// The display version is not 13.0.5.
    UnsupportedVersion,
    /// The display version reads 13.0.5 but the build ID differs.
    UnsupportedBuild,
    /// No build-id note at the end of `.rodata`.
    BuildIdMissing,
}

impl Verdict {
    /// The install line's `result=` value.
    pub const fn result(self) -> &'static str {
        match self {
            Verdict::Supported => "installed",
            _ => "unsupported_version",
        }
    }

    /// The install line's `reason=` value.
    pub const fn reason(self) -> &'static str {
        match self {
            Verdict::Supported => "ok",
            Verdict::UnsupportedVersion => "display_version",
            Verdict::UnsupportedBuild => "build_id",
            Verdict::BuildIdMissing => "build_id_missing",
        }
    }
}

/// Compare the running game with the supported identity. The build ID decides; the display
/// version is checked too so a mislabelled or patched main never passes on one field alone.
pub fn verdict(display: &[u8], build_id: Option<&[u8]>) -> Verdict {
    if display != SUPPORTED_VERSION.as_bytes() {
        return Verdict::UnsupportedVersion;
    }
    match build_id {
        None => Verdict::BuildIdMissing,
        Some(id) if id == MAIN_BUILD_ID_BYTES => Verdict::Supported,
        Some(_) => Verdict::UnsupportedBuild,
    }
}

/// Lower-case hex of a byte string, `none` when empty, for log lines (no allocation).
pub struct Hex<'a>(pub &'a [u8]);

impl core::fmt::Display for Hex<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.0.is_empty() {
            return f.write_str("none");
        }
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

/// Printable display version for a log line (`?` for any non-ASCII-graphic byte).
pub struct Printable<'a>(pub &'a [u8]);

impl core::fmt::Display for Printable<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use core::fmt::Write;
        if self.0.is_empty() {
            return f.write_str("none");
        }
        for b in self.0 {
            f.write_char(if b.is_ascii_graphic() { *b as char } else { '?' })?;
        }
        Ok(())
    }
}
