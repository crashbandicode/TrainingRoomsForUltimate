//! The 64-byte site guard (playbook "Guard, then hook" and HOOK-SHARING "Guards"): compare the 64
//! bytes now at a site with the 13.0.5 bytes and say what is there (stock, another plugin's Skyline
//! hook in a place we can tolerate or not, or a foreign change), then apply the sharing rules for
//! chaining onto a far jump at an entry. Pure: nothing here reads the game.
//!
//! Carried over from the earlier fork's `training_mod_consts/src/native_seams/mod.rs` (stage m5o,
//! ours-new): lines 38-130 (constants and the kind/placement/sharing types) and 546-676, 713-719,
//! 736-837 (the rules). The fork's rows, `Seam`/`Site` tables, `Scopes` and image rows are not
//! carried; this plugin's own site table is `sites.rs`. See CARRY-OVER.md.
/// Bytes compared per site.
pub const GUARD_BYTES: usize = 64;
/// Words a Skyline hook may overwrite at a site: `nop` + far jump (5 words, 20 bytes).
pub const HOOK_SPAN_WORDS: usize = 5;

/// Layout of the owned 13.0.5 `main`, from its section table (offsets from `.text`, which maps at 0):
/// `.text` is `[0, MAIN_TEXT_END)`, `.rodata` is `[MAIN_RODATA_START, MAIN_RODATA_END)`. A row's
/// region follows from its offset; host test `compared_rows_match_the_owned_elf` checks these values.
pub const MAIN_TEXT_END: usize = 0x39c_8440;
pub const MAIN_RODATA_START: usize = 0x39c_9000;
pub const MAIN_RODATA_END: usize = 0x4f1_6f60;

/// The read-only part of `main` a row's 64 bytes lie in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainRegion {
    /// Code: compared with the kind rules (another plugin's hook may be allowed there).
    Text,
    /// Read-only data the plugin relies on (switch jump tables, e.g. the Mii ordinary gate's): never
    /// hooked, so it is compared exactly.
    Rodata,
}

/// The region holding all 64 bytes at `offset`, or `None` when they are not inside one read-only section.
pub const fn region_of(offset: usize) -> Option<MainRegion> {
    match offset.checked_add(GUARD_BYTES) {
        Some(end) if end <= MAIN_TEXT_END => Some(MainRegion::Text),
        Some(end) if offset >= MAIN_RODATA_START && end <= MAIN_RODATA_END => Some(MainRegion::Rodata),
        _ => None,
    }
}

/// How this plugin writes a site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookForm {
    /// `#[skyline::hook(offset|replace = …)]` on a function: Skyline far-jumps the entry.
    Replace,
    /// `#[skyline::hook(…, inline)]`: a one-word branch (or far jump) at the site.
    Inline,
    /// A hand-made `A64HookFunction` / `A64InlineHook`.
    Raw,
    /// A text patch (`nop`, data word).
    Patch,
    /// A data pointer slot (vtable); not code, not compared byte for byte at runtime.
    VtableSlot,
}

/// Where in its function the site sits (the playbook's "hook after the prologue" audit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// The first instruction of a function. Allowed only with a justification in `Seam::note`.
    Entry,
    /// Right after the prologue.
    AfterPrologue,
    /// Inside the body.
    Mid,
    /// Not code (`.data`/`.rodata`).
    Data,
}

/// What this plugin does with the site, which decides what another plugin may have changed there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeamKind {
    /// We write here: the words our write replaces (`HOOK_SPAN_WORDS`) must be stock.
    Hook { form: HookForm, placement: Placement },
    /// We call this function: other plugins' hooks anywhere in its 64 bytes are fine.
    Called,
    /// We rely on this instruction: its own word must stay; hooks after it are fine.
    Relied,
}

/// How the site is shared with other plugins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sharing {
    None,
    /// Another plugin may own the entry; then we use the first body word that still runs when it
    /// calls the original (`live_replay` 0x143e2a0 / 0x143e2b0 with smash_replay_to_mp4). `body` is the
    /// body site's offset (a row of its own). The entry row is usable `Stock` (we take the entry) or
    /// `Shared`: a far jump at the entry (any plugin's, `owner` in particular) that ends before `body`,
    /// with every other word of the 64 bytes stock (we take the body). A one-word branch, a far jump
    /// reaching `body`, or any other change stays unusable (`entry_or_body_outcome`).
    EntryOrBody { owner: &'static str, body: usize },
    /// We chain onto another plugin's far jump at the entry (`effect_hook_chain`).
    Chain,
    /// Listed but not converted, with the reason. No row uses it since the CPU-as-P1 merge guarded
    /// `cpu_pilot` (WORKPLAN §2.9's boundary, WORKAROUND-DECISIONS row 40).
    External { reason: &'static str },
    /// The site is also the entry of this plugin's own replace hook `hook` (a
    /// `Hook { form: Replace, placement: Entry }` row at the same offset), installed before this row is
    /// checked. Skyline's far jump there whose `.quad` target is the replacement address the installer
    /// registered for `hook` (`outcome_for_own`), with the rest of the 64 bytes stock, is our own write:
    /// `Stock` for this row. Any other far jump or branch keeps the kind rule's outcome.
    OwnHook { hook: &'static str },
}

/// Whether a far jump (`ldr x17, #8; br x17`, optionally after a `nop`) starts at word `word`.
pub fn far_jump_at(actual: &[u8; GUARD_BYTES], word: usize) -> bool {
    let words = words(actual);
    let at = |k: usize| words.get(word + k).copied();
    (at(0) == Some(LDR_X17_LITERAL_8) && at(1) == Some(BR_X17))
        || (at(0) == Some(NOP) && at(1) == Some(LDR_X17_LITERAL_8) && at(2) == Some(BR_X17))
}

/// `guard_outcome` plus the row's sharing rule (WORKAROUND-DECISIONS row 28): a `Sharing::Chain`
/// hook chains onto another plugin's far jump at its entry (`effect_hook_chain`), so that case is
/// usable; a one-word branch or anything else at the entry is not.
pub fn outcome_for(
    expected: &[u8; GUARD_BYTES],
    actual: &[u8; GUARD_BYTES],
    kind: SeamKind,
    sharing: Sharing,
) -> GuardOutcome {
    let outcome = guard_outcome(expected, actual, kind);
    match (sharing, outcome) {
        (Sharing::Chain, GuardOutcome::HookedByOther { at: 0, usable: false })
            if far_jump_at(actual, 0) && guard_outcome(expected, &rest_from(actual, expected), kind).usable() =>
        {
            GuardOutcome::HookedByOther { at: 0, usable: true }
        }
        _ => outcome,
    }
}

/// The `Sharing::EntryOrBody` rule for an entry row whose body site is `body_word` words after it:
/// `Stock` (we take the entry), `Shared` when a far jump at the entry (any plugin's) ends at or before
/// the body word and every other word of the 64 bytes is stock (we take the body, which still runs when
/// that plugin calls the original), otherwise the kind rule's outcome (a one-word branch at the entry,
/// a far jump reaching the body, or any other change stays unusable for an entry hook).
pub fn entry_or_body_outcome(
    expected: &[u8; GUARD_BYTES],
    actual: &[u8; GUARD_BYTES],
    kind: SeamKind,
    body_word: usize,
) -> GuardOutcome {
    let outcome = guard_outcome(expected, actual, kind);
    let jump_words = if words(actual)[0] == NOP { 5 } else { 4 };
    match outcome {
        GuardOutcome::HookedByOther { at: 0, .. }
            if far_jump_at(actual, 0)
                && (jump_words..WORDS).contains(&body_word)
                && rest_from(actual, expected) == *expected =>
        {
            GuardOutcome::Shared
        }
        _ => outcome,
    }
}

/// What a site's first words hold, for the diagnostic stand-in drill's `<site>=<entry>/<outcome>`:
/// the 13.0.5 words, a one-word branch, a far jump, or something else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryForm {
    Stock,
    Branch,
    FarJump,
    Changed,
}

impl EntryForm {
    pub const fn name(self) -> &'static str {
        match self {
            EntryForm::Stock => "stock",
            EntryForm::Branch => "branch",
            EntryForm::FarJump => "far",
            EntryForm::Changed => "changed",
        }
    }
}

/// The form at word 0 of `actual` against the 13.0.5 bytes `expected`.
pub fn entry_form(expected: &[u8; GUARD_BYTES], actual: &[u8; GUARD_BYTES]) -> EntryForm {
    let first = words(actual)[0];
    if far_jump_at(actual, 0) {
        EntryForm::FarJump
    } else if first != words(expected)[0] && first & B_MASK == B_OPCODE {
        EntryForm::Branch
    } else if actual == expected {
        EntryForm::Stock
    } else {
        EntryForm::Changed
    }
}

/// `actual` with the entry far jump (and its `nop`) replaced by the expected words, to check the rest.
fn rest_from(actual: &[u8; GUARD_BYTES], expected: &[u8; GUARD_BYTES]) -> [u8; GUARD_BYTES] {
    let len = (if words(actual)[0] == NOP { 5 } else { 4 }) * 4;
    let mut out = *actual;
    out[..len].copy_from_slice(&expected[..len]);
    out
}

/// The `.quad` target of a far jump at the entry: bytes 8..16 after `ldr x17, #8; br x17`, or 12..20
/// when a `nop` aligns it (Skyline / And64InlineHook writes the replacement address there).
pub fn entry_far_jump_target(actual: &[u8; GUARD_BYTES]) -> Option<u64> {
    if !far_jump_at(actual, 0) {
        return None;
    }
    let at = if words(actual)[0] == NOP { 12 } else { 8 };
    let mut quad = [0u8; 8];
    quad.copy_from_slice(&actual[at..at + 8]);
    Some(u64::from_le_bytes(quad))
}

/// `outcome_for` plus the `Sharing::OwnHook` rule. `own_target` is the replacement address the
/// installer registered when it installed the named hook (`None`: not installed). A far jump at the
/// entry to exactly that address, with the rest of the 64 bytes stock, is our own write: `Stock`.
/// Anything else (another target, a one-word branch, a further hook or change in the rest, or no
/// registration) keeps `outcome_for`'s answer.
pub fn outcome_for_own(
    expected: &[u8; GUARD_BYTES],
    actual: &[u8; GUARD_BYTES],
    kind: SeamKind,
    sharing: Sharing,
    own_target: Option<u64>,
) -> GuardOutcome {
    match (sharing, own_target) {
        (Sharing::OwnHook { .. }, Some(target))
            if entry_far_jump_target(actual) == Some(target)
                && guard_outcome(expected, &rest_from(actual, expected), kind) == GuardOutcome::Stock =>
        {
            GuardOutcome::Stock
        }
        _ => outcome_for(expected, actual, kind, sharing),
    }
}


/// Byte-exact comparison: `Stock`, or `Foreign` at the first differing byte.
pub fn exact_outcome(expected: &[u8; GUARD_BYTES], actual: &[u8; GUARD_BYTES]) -> GuardOutcome {
    match expected.iter().zip(actual.iter()).position(|(e, a)| e != a) {
        None => GuardOutcome::Stock,
        Some(first_diff) => GuardOutcome::Foreign { first_diff },
    }
}

/// The 64-byte comparison of one site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardOutcome {
    /// Identical to 13.0.5.
    Stock,
    /// Only Skyline hook shapes differ. `at` is the byte offset of the first; `usable` applies the
    /// kind's rule (false: another plugin owns the words we would replace or rely on).
    HookedByOther { at: usize, usable: bool },
    /// A `Sharing::EntryOrBody` entry row whose entry another plugin's far jump owns, with the body
    /// after it stock: usable, through the body site (`entry_or_body_outcome`).
    Shared,
    /// A change that is not a hook: the site is left alone.
    Foreign { first_diff: usize },
}

impl GuardOutcome {
    pub const fn usable(self) -> bool {
        match self {
            GuardOutcome::Stock | GuardOutcome::Shared => true,
            GuardOutcome::HookedByOther { usable, .. } => usable,
            GuardOutcome::Foreign { .. } => false,
        }
    }

    /// The install line's word for the outcome.
    pub const fn code(self) -> &'static str {
        match self {
            GuardOutcome::Stock => "stock",
            GuardOutcome::HookedByOther { usable: true, .. } => "other",
            GuardOutcome::HookedByOther { usable: false, .. } => "occupied",
            GuardOutcome::Shared => "shared",
            GuardOutcome::Foreign { .. } => "foreign",
        }
    }
}

pub const WORDS: usize = GUARD_BYTES / 4;
pub const NOP: u32 = 0xd503_201f;
pub const LDR_X17_LITERAL_8: u32 = 0x5800_0051;
pub const BR_X17: u32 = 0xd61f_0220;
pub const B_MASK: u32 = 0xfc00_0000;
pub const B_OPCODE: u32 = 0x1400_0000;

fn words(bytes: &[u8; GUARD_BYTES]) -> [u32; WORDS] {
    let mut out = [0u32; WORDS];
    for (i, word) in out.iter_mut().enumerate() {
        *word = u32::from_le_bytes([bytes[4 * i], bytes[4 * i + 1], bytes[4 * i + 2], bytes[4 * i + 3]]);
    }
    out
}

/// Length in words of a Skyline hook shape starting at word `i` (cut at the end of the window).
fn hook_shape(actual: &[u32; WORDS], i: usize) -> Option<usize> {
    let at = |k: usize| actual.get(i + k).copied();
    let len = if at(0) == Some(NOP) && at(1) == Some(LDR_X17_LITERAL_8) && at(2) == Some(BR_X17) {
        5
    } else if at(0) == Some(LDR_X17_LITERAL_8) && at(1) == Some(BR_X17) {
        4
    } else if actual[i] & B_MASK == B_OPCODE {
        1
    } else {
        return None;
    };
    Some(len.min(WORDS - i))
}

/// Whether a hook shape at word `word` takes words this kind needs as they are.
const fn blocks(kind: SeamKind, word: usize) -> bool {
    match kind {
        SeamKind::Hook { .. } => word < HOOK_SPAN_WORDS,
        SeamKind::Called => false,
        SeamKind::Relied => word == 0,
    }
}

/// Compare a site's 64 bytes now (`actual`) with its 13.0.5 bytes (`expected`).
pub fn guard_outcome(expected: &[u8; GUARD_BYTES], actual: &[u8; GUARD_BYTES], kind: SeamKind) -> GuardOutcome {
    let expected = words(expected);
    let actual = words(actual);
    let mut first_hook = None;
    let mut usable = true;
    let mut i = 0;
    while i < WORDS {
        if actual[i] == expected[i] {
            i += 1;
            continue;
        }
        let Some(len) = hook_shape(&actual, i) else {
            return GuardOutcome::Foreign { first_diff: 4 * i };
        };
        first_hook.get_or_insert(4 * i);
        // A shape that starts inside the span also covers every word it spans.
        if (i..i + len).any(|w| blocks(kind, w)) {
            usable = false;
        }
        i += len;
    }
    match first_hook {
        None => GuardOutcome::Stock,
        Some(at) => GuardOutcome::HookedByOther { at, usable },
    }
}
