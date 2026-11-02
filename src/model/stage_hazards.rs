//! Stage Hazards (SPEC-state-07): Training has no rules screen, so stages always load with their
//! hazards on. The plugin writes the game's own "hazards" rule byte at the moment the stage-loading
//! code is about to read it.
//!
//! The 13.0.5 addresses are pinned in `sites.rs` and guarded byte for byte (SPEC-state-07 question
//! 3). The instruction-run scans below are what found them; the host tests re-run them on the owned
//! 13.0.5 `.text` and the Switch runtime may use them as a cross-check in a `diag` build. They are
//! not run in the shipped plugin.

/// The inline hook site: the `cbz` of the `adrp; ldrsw; cbz` run in the stage-loading routine.
pub const HOOK_SITE_OFFSET: usize = 0x231_1bec;
/// The first instruction of the `adrp; add; ldur; ldrb; ldr; sub` run the flag address derives from.
pub const FLAG_SCAN_OFFSET: usize = 0x174_08c4;
/// The hazard rule byte in 13.0.5 (`.bss`), offset from `.text`.
pub const FLAG_OFFSET: usize = 0x530_a985;
/// The constant the scan adds to `page + adrp + add` (the byte's offset inside the object).
pub const FLAG_BYTE_ADJUST: usize = 9;

/// The byte to write at the hook, or `None` when nothing is written: only while Training is live
/// (SPEC-state-07 §4: outside Training the game's own rule value is never touched).
pub const fn rule_byte(hazards_on: bool, training_live: bool) -> Option<u8> {
    if training_live {
        Some(hazards_on as u8)
    } else {
        None
    }
}

/// What is missing when the feature cannot be installed (SPEC-state-07 question 2: each condition
/// gets its own text; upstream attached them the wrong way round).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Missing {
    /// The rule byte's address could not be derived.
    Flag,
    /// The hook site was not usable.
    Hook,
    Both,
}

impl Missing {
    pub const fn text(self) -> &'static str {
        match self {
            Missing::Flag => "hazard rule byte not located",
            Missing::Hook => "stage-loading hook site not located",
            Missing::Both => "hazard rule byte and stage-loading hook site not located",
        }
    }
}

/// Whether the feature installs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    Enabled,
    /// The feature stays off for this launch; the install line says why (no dialog).
    Disabled(Missing),
}

/// Both a derived flag address and a usable hook site are needed.
pub const fn plan(flag: Option<usize>, hook_usable: bool) -> Plan {
    match (flag.is_some(), hook_usable) {
        (true, true) => Plan::Enabled,
        (true, false) => Plan::Disabled(Missing::Hook),
        (false, true) => Plan::Disabled(Missing::Flag),
        (false, false) => Plan::Disabled(Missing::Both),
    }
}

/// `adrp`: bits `1 immlo(2) 10000 immhi(19) Rd`.
pub const fn is_adrp(word: u32) -> bool {
    word & 0x9f00_0000 == 0x9000_0000
}

/// `add Rd, Rn, #imm12` (32- or 64-bit, no flags).
pub const fn is_add_imm(word: u32) -> bool {
    word & 0x7f80_0000 == 0x1100_0000
}

/// `ldur Rt, [Rn, #simm9]` (integer load).
pub const fn is_ldur(word: u32) -> bool {
    word & 0x3fe0_0c00 == 0x3840_0000
}

/// `ldrb Wt, [Xn, #imm12]`.
pub const fn is_ldrb_imm(word: u32) -> bool {
    word & 0xffc0_0000 == 0x3940_0000
}

/// `ldr Rt, [Xn, #imm12]` (32- or 64-bit).
pub const fn is_ldr_imm(word: u32) -> bool {
    word & 0xbfc0_0000 == 0xb940_0000
}

/// `sub Rd, Rn, #imm12` (32- or 64-bit, no flags).
pub const fn is_sub_imm(word: u32) -> bool {
    word & 0x7f80_0000 == 0x5100_0000
}

/// `ldrsw Xt, [Xn, #imm12]`.
pub const fn is_ldrsw_imm(word: u32) -> bool {
    word & 0xffc0_0000 == 0xb980_0000
}

/// `cbz` / `cbnz`'s zero form (`cbz Rt, label`, 32- or 64-bit).
pub const fn is_cbz(word: u32) -> bool {
    word & 0x7f00_0000 == 0x3400_0000
}

/// The byte offset `adrp` adds to the page of its own address (signed, a multiple of 4096).
pub const fn adrp_offset(word: u32) -> i64 {
    let immlo = ((word >> 29) & 3) as i64;
    let immhi = ((word >> 5) & 0x7_ffff) as i64;
    let imm21 = (immhi << 2) | immlo;
    let signed = (imm21 << 43) >> 43;
    signed << 12
}

/// The constant `add Rd, Rn, #imm12 {, lsl #12}` adds.
pub const fn add_imm_value(word: u32) -> usize {
    let imm = ((word >> 10) & 0xfff) as usize;
    if (word >> 22) & 1 == 1 {
        imm << 12
    } else {
        imm
    }
}

/// The hazard rule byte's offset from `.text`, from the `adrp` at `adrp_offset_in_text` and the
/// `add` after it: `(pc & !0xfff) + adrp + add + 9` (SPEC-state-07 §5.1). `None` when either word is
/// not the instruction it should be or the result is negative.
pub fn flag_offset(adrp_site: usize, adrp: u32, add: u32) -> Option<usize> {
    if !is_adrp(adrp) || !is_add_imm(add) {
        return None;
    }
    let page = i64::try_from(adrp_site & !0xfff).ok()?;
    let total = page.checked_add(adrp_offset(adrp))?.checked_add(i64::try_from(add_imm_value(add)).ok()?)?;
    usize::try_from(total).ok()?.checked_add(FLAG_BYTE_ADJUST)
}

/// Index of the first run of consecutive instructions of the given classes.
pub fn find_run(words: &[u32], classes: &[fn(u32) -> bool]) -> Option<usize> {
    if classes.is_empty() || words.len() < classes.len() {
        return None;
    }
    words.windows(classes.len()).position(|run| run.iter().zip(classes).all(|(word, class)| class(*word)))
}

/// The flag scan: the first `adrp, add, ldur, ldrb, ldr, sub` run. Returns the index of its `adrp`.
pub fn find_flag_run(words: &[u32]) -> Option<usize> {
    find_run(words, &[is_adrp, is_add_imm, is_ldur, is_ldrb_imm, is_ldr_imm, is_sub_imm])
}

/// The hook scan: the first `adrp, ldrsw, cbz` run. Returns the index of its `adrp`; the site is the
/// `cbz`, two words after it.
pub fn find_hook_run(words: &[u32]) -> Option<usize> {
    find_run(words, &[is_adrp, is_ldrsw_imm, is_cbz])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_byte_only_in_training() {
        assert_eq!(rule_byte(true, true), Some(1));
        assert_eq!(rule_byte(true, false), None);
    }

    #[test]
    fn adrp_offsets_are_signed_pages() {
        assert_eq!(adrp_offset(0x9000_0000), 0);
        assert_eq!(adrp_offset(0xf000_0000 | (0x7_ffff << 5)), -4096);
    }

    #[test]
    fn a_run_is_found_by_class() {
        let words = [0x1400_0000, 0x9000_0000, 0xb980_0000, 0x3400_0000];
        assert_eq!(find_hook_run(&words), Some(1));
        assert_eq!(find_hook_run(&words[..3]), None);
    }
}