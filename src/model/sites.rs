//! This plugin's site table: every place in the 13.0.5 `main` the plugin hooks, patches, calls or
//! relies on, with the site's 64 bytes (playbook "Hook safety": a 64-byte guard on every hooked
//! site, every called function and every relied-on instruction). Pure data and pure rules; the
//! Switch runtime reads the bytes now at each site and asks `Site::outcome`.
//!
//! The offsets and bytes are the earlier fork's seam rows for the same sites (stage m5o,
//! `training_mod_consts/src/native_seams/rows_a3.rs` and `rows_a5.rs`, ours-new), re-keyed to this
//! plugin's names; `is_training_mode`'s bytes were read from the owned 13.0.5 ELF at its symbol
//! (`tests/core01_sites.rs` re-checks every row against that ELF). Offsets are from the start of
//! `.text` (13.0.5 `main`; `.text` maps at 0). See CARRY-OVER.md.
//!
//! Package 1 holds the sites of boot, Input Delay, L+R+A Reset and Stage Hazards; later packages add
//! rows here.

use super::guard::{outcome_for, GuardOutcome, HookForm, Placement, SeamKind, Sharing, GUARD_BYTES};
use super::version::Verdict;

/// The feature a site belongs to: a feature is installed only when every one of its sites is usable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    /// Shared by every Training feature (the Training-mode predicate).
    Training,
    InputDelay,
    LraReset,
    /// The Training reset hub (SPEC-state-06 §5): the fighter on-init wrapper that runs it.
    ResetHub,
    StageHazards,
}

impl Feature {
    pub const fn name(self) -> &'static str {
        match self {
            Feature::Training => "training",
            Feature::InputDelay => "input_delay",
            Feature::LraReset => "lra_reset",
            Feature::ResetHub => "reset_hub",
            Feature::StageHazards => "stage_hazards",
        }
    }
}

/// One guarded site.
#[derive(Clone, Copy, Debug)]
pub struct Site {
    /// Unique; the install line's `<site>=` key.
    pub name: &'static str,
    pub feature: Feature,
    /// Offset from the start of `main` `.text`.
    pub offset: usize,
    pub kind: SeamKind,
    /// The 13.0.5 bytes at `offset`. For a data pointer slot only `guard[..8]` matters: the
    /// little-endian offset (from `.text`) the slot points at.
    pub guard: [u8; GUARD_BYTES],
    /// What the site is and why the placement is allowed.
    pub note: &'static str,
}

impl Site {
    /// Whether the runtime compares the 64 bytes (code), as opposed to a relocated pointer slot.
    pub const fn compared(&self) -> bool {
        !matches!(self.kind, SeamKind::Hook { form: HookForm::VtableSlot, .. })
    }

    /// For a vtable slot: the `.text` offset the slot holds in 13.0.5.
    pub fn expected_slot_target(&self) -> Option<u64> {
        if self.compared() {
            return None;
        }
        let mut word = [0u8; 8];
        word.copy_from_slice(&self.guard[..8]);
        Some(u64::from_le_bytes(word))
    }

    /// The outcome for the 64 bytes now at the site (code rows).
    pub fn outcome(&self, actual: &[u8; GUARD_BYTES]) -> GuardOutcome {
        outcome_for(&self.guard, actual, self.kind, Sharing::None)
    }

    /// The outcome for a vtable slot now holding `.text` offset `found`.
    pub fn slot_outcome(&self, found: u64) -> GuardOutcome {
        match self.expected_slot_target() {
            Some(expected) if expected == found => GuardOutcome::Stock,
            Some(_) => GuardOutcome::Foreign { first_diff: 0 },
            None => GuardOutcome::Foreign { first_diff: 0 },
        }
    }

    /// The first byte after the 64-byte window at the site.
    pub const fn window_end(&self) -> usize {
        self.offset + GUARD_BYTES
    }

    /// Whether the two 64-byte windows share bytes (a hook at one then changes what the other reads).
    pub const fn overlaps(&self, other: &Site) -> bool {
        self.offset < other.window_end() && other.offset < self.window_end()
    }
}

/// Offsets of the sites the hooks name (each is a row of `SITES`; `tests/core01.rs` checks them).
pub const FINAL_INPUT_MAPPING_OFFSET: usize = 0x175_0dd0;
pub const RESET_CHECK_OFFSET: usize = 0x137_8e50;
pub const ON_INIT_SLOT_OFFSET: usize = 0x4f8_0240;
pub const ON_INIT_OFFSET: usize = 0x60_dfa0;
pub const IS_TRAINING_MODE_OFFSET: usize = 0x167_1650;

/// The table, in install-line order.
pub static SITES: [Site; 8] = [    Site {
        name: "final_input_mapping",
        feature: Feature::InputDelay,
        offset: 0x1750dd0,
        kind: SeamKind::Hook { form: HookForm::Replace, placement: Placement::Entry },
        guard: [
            0xff, 0x03, 0x02, 0xd1, 0xf7, 0x23, 0x00, 0xf9, 0xf6, 0x57, 0x05, 0xa9, 0xf4, 0x4f, 0x06, 0xa9,
            0xfd, 0x7b, 0x07, 0xa9, 0xfd, 0xc3, 0x01, 0x91, 0x3f, 0x04, 0x00, 0x31, 0xe0, 0x77, 0x00, 0x54,
            0x37, 0x7c, 0x40, 0x93, 0xff, 0x22, 0x00, 0xf1, 0x62, 0x9f, 0x00, 0x54, 0xf3, 0x03, 0x02, 0xaa,
            0x5f, 0x00, 0x00, 0xf9, 0x23, 0x77, 0x00, 0xb4, 0x74, 0x08, 0x40, 0xf9, 0x82, 0x9e, 0x40, 0xb9,
        ],
        note: "Replace hook at the function entry: the function is the game's final input mapping (a function, not a part of one); the original runs first and its result record is what this plugin changes.",
    },
    Site {
        name: "reset_check",
        feature: Feature::LraReset,
        offset: 0x1378e50,
        kind: SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid },
        guard: [
            0x1f, 0x09, 0x00, 0x71, 0x41, 0x1c, 0x00, 0x54, 0x08, 0xfa, 0x01, 0xb0, 0x08, 0x7d, 0x42, 0xf9,
            0x08, 0x01, 0x40, 0xf9, 0x09, 0xa1, 0x40, 0xb9, 0x3f, 0x05, 0x00, 0x71, 0x2b, 0x0a, 0x00, 0x54,
            0x0c, 0x11, 0x40, 0xf9, 0xea, 0x03, 0x1f, 0x2a, 0x0b, 0x00, 0x80, 0x12, 0xed, 0x0b, 0x00, 0x32,
            0xae, 0x00, 0x80, 0x52, 0x0f, 0x24, 0x8b, 0x52, 0x50, 0x9a, 0x88, 0x52, 0x31, 0x24, 0x8b, 0x52,
        ],
        note: "Inline hook inside the game's Training reset check; register x8 is the compare operand of the L+R+A test (SPEC-state-06 question 4: confirm on the owned binary).",
    },
    Site {
        name: "hazard_window",
        feature: Feature::StageHazards,
        offset: 0x2311be4,
        kind: SeamKind::Relied,
        guard: [
            0xc9, 0x7f, 0x01, 0xb0, 0x39, 0x69, 0x89, 0xb9, 0x88, 0x05, 0x00, 0x34, 0x3f, 0x0f, 0x00, 0x71,
            0x22, 0x77, 0x01, 0x54, 0xba, 0x7d, 0x01, 0x90, 0x5a, 0x03, 0x06, 0x91, 0x48, 0x17, 0x19, 0x8b,
            0x09, 0xc8, 0x8a, 0x52, 0x89, 0x00, 0xa0, 0x72, 0xe1, 0xf2, 0x89, 0xd2, 0x01, 0xcf, 0xaa, 0xf2,
            0xa1, 0x01, 0xc0, 0xf2, 0x1b, 0x20, 0xed, 0xd2, 0x16, 0x69, 0x69, 0xf8, 0x09, 0xc9, 0x8a, 0x52,
        ],
        note: "The adrp/ldrsw pair the hazard scan matches before the cbz; relied on, never written.",
    },
    Site {
        name: "hazard_hook",
        feature: Feature::StageHazards,
        offset: 0x2311bec,
        kind: SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid },
        guard: [
            0x88, 0x05, 0x00, 0x34, 0x3f, 0x0f, 0x00, 0x71, 0x22, 0x77, 0x01, 0x54, 0xba, 0x7d, 0x01, 0x90,
            0x5a, 0x03, 0x06, 0x91, 0x48, 0x17, 0x19, 0x8b, 0x09, 0xc8, 0x8a, 0x52, 0x89, 0x00, 0xa0, 0x72,
            0xe1, 0xf2, 0x89, 0xd2, 0x01, 0xcf, 0xaa, 0xf2, 0xa1, 0x01, 0xc0, 0xf2, 0x1b, 0x20, 0xed, 0xd2,
            0x16, 0x69, 0x69, 0xf8, 0x09, 0xc9, 0x8a, 0x52, 0x89, 0x00, 0xa0, 0x72, 0x13, 0x69, 0x69, 0xb8,
        ],
        note: "The cbz of the adrp/ldrsw/cbz run in the stage-loading routine; the hook writes the hazard rule byte just before the game reads it.",
    },
    Site {
        name: "hazard_flag",
        feature: Feature::StageHazards,
        offset: 0x17408c4,
        kind: SeamKind::Relied,
        guard: [
            0x54, 0xde, 0x01, 0xd0, 0x94, 0xf2, 0x25, 0x91, 0xa8, 0x03, 0x51, 0xf8, 0x89, 0x02, 0x40, 0x39,
            0x08, 0x01, 0x40, 0xf9, 0x2a, 0x05, 0x00, 0x51, 0x5f, 0x11, 0x00, 0x71, 0xea, 0x03, 0x00, 0x32,
            0x49, 0x25, 0x89, 0x1a, 0x22, 0x1d, 0x40, 0x92, 0x00, 0x01, 0x13, 0x8b, 0x61, 0x59, 0x01, 0xf0,
            0x21, 0xbc, 0x1b, 0x91, 0xea, 0x15, 0x8a, 0x94, 0xa8, 0x03, 0x51, 0xf8, 0x08, 0x01, 0x40, 0xf9,
        ],
        note: "adrp/add/ldur/ldrb/ldr/sub: the instruction run the hazard byte address is derived from; relied on, never written.",
    },
    Site {
        name: "on_init_slot",
        feature: Feature::ResetHub,
        offset: 0x4f80240,
        kind: SeamKind::Hook { form: HookForm::VtableSlot, placement: Placement::Data },
        guard: [
            0xa0, 0xdf, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
        note: "Fighter vtable on_init pointer slot (data): holds main+0x60dfa0 in 13.0.5; the wrapper replaces it.",
    },
    Site {
        name: "on_init",
        feature: Feature::ResetHub,
        offset: 0x60dfa0,
        kind: SeamKind::Called,
        guard: [
            0xff, 0xc3, 0x04, 0xd1, 0xfc, 0x6f, 0x0d, 0xa9, 0xfa, 0x67, 0x0e, 0xa9, 0xf8, 0x5f, 0x0f, 0xa9,
            0xf6, 0x57, 0x10, 0xa9, 0xf4, 0x4f, 0x11, 0xa9, 0xfd, 0x7b, 0x12, 0xa9, 0xfd, 0x83, 0x04, 0x91,
            0x09, 0xa0, 0x41, 0x29, 0x78, 0x65, 0x02, 0x90, 0x1b, 0x7f, 0x42, 0xf9, 0x2a, 0x2d, 0x01, 0x51,
            0xf4, 0x03, 0x01, 0xaa, 0xf3, 0x03, 0x00, 0xaa, 0x08, 0x7d, 0x40, 0x93, 0x5f, 0x05, 0x00, 0x71,
        ],
        note: "The fighter on_init function the vtable slot points at; the wrapper calls it first.",
    },
    Site {
        name: "is_training_mode",
        feature: Feature::Training,
        offset: 0x1671650,
        kind: SeamKind::Called,
        guard: [
            0xfd, 0x7b, 0xbf, 0xa9, 0xfd, 0x03, 0x00, 0x91, 0x08, 0xe5, 0x01, 0xf0, 0x08, 0x61, 0x13, 0x91,
            0x08, 0xfd, 0xdf, 0x08, 0x48, 0x01, 0x00, 0x36, 0xa8, 0xe2, 0x01, 0x90, 0x08, 0xc1, 0x46, 0x39,
            0xe9, 0x03, 0x00, 0x32, 0x1f, 0x81, 0x00, 0x71, 0x2a, 0x01, 0x9f, 0x1a, 0x1f, 0x31, 0x00, 0x71,
            0x20, 0x01, 0x8a, 0x1a, 0xfd, 0x7b, 0xc1, 0xa8, 0xc0, 0x03, 0x5f, 0xd6, 0x00, 0xe5, 0x01, 0xf0,
        ],
        note: "The game's Training-mode predicate (symbol _ZN3app9smashball16is_training_modeEv), called by every feature; 64 bytes read from the owned 13.0.5 main.",
    },
];

/// The site called `name`.
pub fn find(name: &str) -> Option<&'static Site> {
    SITES.iter().find(|site| site.name == name)
}

/// The sites a feature needs (its own, then the shared Training predicate when it uses it).
pub fn sites_of(feature: Feature) -> impl Iterator<Item = &'static Site> {
    SITES.iter().filter(move |site| site.feature == feature)
}

/// Whether a feature can be installed: every one of its sites and the shared Training predicate
/// must come out usable.
pub fn feature_usable(feature: Feature, usable: impl Fn(&Site) -> bool) -> bool {
    sites_of(feature).chain(sites_of(Feature::Training)).all(usable)
}

/// What boot does with the table for a game verdict: nothing for an unsupported game
/// (SPEC-core-01 §2: install nothing and log one line), every site otherwise.
pub fn install_plan(verdict: Verdict) -> &'static [Site] {
    match verdict {
        Verdict::Supported => &SITES,
        _ => &[],
    }
}

/// Boot's two phases for sites (SPEC-core-01 §5.1 step 5b, §6 "Overlapping thunks"): every window is
/// compared first, then anything is written. A write at one site changes the bytes another site's
/// window reads, so checking after installing would see our own hook as a foreign change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    CheckEveryWindow,
    InstallUsableFeatures,
}

pub const BOOT_PHASES: [Phase; 2] = [Phase::CheckEveryWindow, Phase::InstallUsableFeatures];

/// Index pairs `(a, b)`, `a < b`, of sites whose 64-byte windows overlap.
pub fn overlapping_pairs() -> impl Iterator<Item = (usize, usize)> {
    (0..SITES.len())
        .flat_map(|a| (a + 1..SITES.len()).map(move |b| (a, b)))
        .filter(|&(a, b)| SITES[a].overlaps(&SITES[b]))
}