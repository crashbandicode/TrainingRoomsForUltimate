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
    /// The CPU behaviours' shared plumbing (package 2): the per-frame dispatch, the game's constant table
    /// and the natives the facts are read through.
    CpuCore,
    /// SPEC-cpu-02: SDI, air-dodge direction, clatter (DI needs the `common` module, see `cpu::common`).
    CpuDefence,
    /// SPEC-cpu-03: the ledge options.
    CpuLedge,
    /// SPEC-cpu-04: tech, missed tech, hidden tech animations (the camera and the script status-change
    /// hook need sites this table does not have yet, see `IMPLEMENTATION-PROGRESS.md`).
    CpuTech,
    /// SPEC-cpu-05: shield hold, decay override and tilt (the out-of-shield hook is in the `common`
    /// script module and the damage multiplier needs the param-hook plugin, see `IMPLEMENTATION-PROGRESS.md`).
    CpuShield,
    /// Package 3 (save states): the natives and hooks of SPEC-state-01 (`sites_pkg3.rs`).
    StateCore,
    /// Package 3: the charge writes and visuals (SPEC-state-02).
    StateCharge,
    /// Package 3: the buffs and their five hooks (SPEC-state-03).
    StateBuff,
    /// Package 3: the held-item restore and its sixteen turnip hooks (SPEC-state-04).
    StateItems,
    /// Package 3: Steve, Olimar, Sora and the fighter-specific restores (SPEC-state-05).
    StateRestore,
    /// Package 4: the input recorder (SPEC-input-01): the natives it reads and writes through and the
    /// hook at the AI control function's epilogue.
    Recording,
    /// Package 4: HUD notices (SPEC-input-05): the Combo Display observers and the stale-moves toggle.
    Notices,
    /// Package 4, S4-4: the named-macro layer's game calls (control reads, kinetic reads, the panel's hold and the restore pose writes).
    NamedMacros,
    /// Package 5: the layout archive hook, the UI anchors and the manager global (`sites_ui.rs`).
    Ui,
    /// The input display's reads in the input-delay hook (the P1 body's status, found through the fighter manager): no
    /// rows of its own, only the natives it calls (R2-1). Input Delay installs on its own rows; the display degrades apart.
    InputDisplay,
    /// The run-time texture helper's rows (`sites_u4.rs`, `runtime/ui/texture.rs`): a unit of its own (they are the Ui feature's rows in
    /// their table), so the menu does not depend on a helper that is optional (R2-1 merge with u4).
    UiTexture,
    /// The hooks in the `common` script module (DI, tech status change, out of shield), installed when
    /// that module loads through the `libnro_hook` provider.
    /// SPEC-cpu-01 and SPEC-cpu-06: the mash's command flags, aerial kind and attack angle.
    CpuMash,
    CpuCommon,
    /// Package 6: the CPU-as-P1 fallback's game calls (sites_pkg6.rs); its hook is the carried seam row cpu_pilot.ai_process_begin.
    P1Cpu,
    /// Package 6: the roster cycle's game calls that no other package guards (sites_pkg6.rs `roster_*` rows); the cycle's hooks are carried seam rows.
    Roster,
    /// Package 6: the diag-only source-owner probe's game calls (sites_pkg6.rs probe_* rows).
    Probes,
    /// SPEC-cpu-05's shield damage multiplier: the game's CommonParams object, found at the moment the game fills
    /// it from its parameter file (no provider plugin).
    CpuShieldDamage,
    /// SPEC-cpu-04's fixed camera: the stage id, the stage table and the two camera hooks.
    CpuCamera,
}

impl Feature {
    pub const fn name(self) -> &'static str {
        match self {
            Feature::Training => "training",
            Feature::InputDelay => "input_delay",
            Feature::LraReset => "lra_reset",
            Feature::ResetHub => "reset_hub",
            Feature::StageHazards => "stage_hazards",
            Feature::CpuCore => "cpu_core",
            Feature::CpuDefence => "cpu_defence",
            Feature::CpuLedge => "cpu_ledge",
            Feature::CpuTech => "cpu_tech",
            Feature::CpuShield => "cpu_shield",
            Feature::StateCore => "state_core",
            Feature::StateCharge => "state_charge",
            Feature::StateBuff => "state_buff",
            Feature::StateItems => "state_items",
            Feature::StateRestore => "state_restore",
            Feature::Recording => "recording",
            Feature::Notices => "notices",
            Feature::NamedMacros => "named_macros",
            Feature::Ui => "ui",
            Feature::InputDisplay => "input_display",
            Feature::UiTexture => "ui_texture",
            Feature::CpuMash => "cpu_mash",
            Feature::CpuCommon => "cpu_common",
            Feature::P1Cpu => "p1_cpu",
            Feature::Roster => "roster",
            Feature::Probes => "probes",
            Feature::CpuShieldDamage => "cpu_shield_damage",
            Feature::CpuCamera => "cpu_camera",
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
/// Input Delay hooks sit in the two callers of the game's final input mapping (`main+0x1750dd0`), not
/// in the function: its entry, its `fim_valid` word (`0x1750df0`) and its exit (`0x1751ce8`) are the
/// CPU pilot plugin's. `note` sites run just before a call and note the out record and slot; `apply`
/// sites run right after it returns, so the delay comes after everything the function and the pilot do.
pub const INPUT_CALLER_A_NOTE_OFFSET: usize = 0x175_46e0;
pub const INPUT_CALLER_A_APPLY_OFFSET: usize = 0x175_4720;
pub const INPUT_CALLER_B_NOTE_OFFSET: usize = 0x175_523c;
pub const INPUT_CALLER_B_APPLY_OFFSET: usize = 0x175_5254;
pub const RESET_CHECK_OFFSET: usize = 0x137_8e50;
pub const ON_INIT_SLOT_OFFSET: usize = 0x4f8_0240;
pub const ON_INIT_OFFSET: usize = 0x60_dfa0;
pub const IS_TRAINING_MODE_OFFSET: usize = 0x167_1650;
pub const FIGHTER_INFORMATION_OFFSET: usize = 0x214_0fc0;
pub const IS_OPERATION_CPU_OFFSET: usize = 0x20c_9e30;
/// Package 1's sites, in install-line order.
pub static BASE_SITES: [Site; 13] = [
    Site {
        name: "input_caller_a_note",
        feature: Feature::InputDelay,
        offset: 0x17546e0,
        kind: SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid },
        guard: [
            0xe9, 0x17, 0x9f, 0x1a, 0xe2, 0x02, 0x16, 0x8b, 0xe0, 0x03, 0x15, 0xaa, 0xe1, 0x03, 0x14, 0xaa,
            0x24, 0x01, 0x08, 0x0a, 0xb7, 0xf1, 0xff, 0x97, 0x0a, 0x00, 0x00, 0x14, 0x09, 0xe1, 0x54, 0x39,
            0x3f, 0x09, 0x00, 0x71, 0xc8, 0x00, 0x00, 0x54, 0x29, 0x05, 0x00, 0x11, 0x2a, 0x1d, 0x00, 0x12,
            0x5f, 0x09, 0x00, 0x71, 0x09, 0xe1, 0x14, 0x39, 0xc9, 0xfd, 0xff, 0x54, 0xff, 0x7a, 0x34, 0xf8,
        ],
        note: "The loop in the game's per-frame input update that calls the final input mapping for each of the 8 slots, at `cset w9, eq` (the instruction before the call's argument setup finishes). x20 is the slot index and x23 + x22 the slot's 8-byte out record (the same value the call gets in x2); none of x20, x22, x23 is written before the call. The hook only notes both for this thread.",
    },
    Site {
        name: "input_caller_a_apply",
        feature: Feature::InputDelay,
        offset: 0x1754720,
        kind: SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid },
        guard: [
            0x94, 0x06, 0x00, 0x91, 0xd6, 0x22, 0x00, 0x91, 0x9f, 0x22, 0x00, 0xf1, 0x21, 0xfb, 0xff, 0x54,
            0x68, 0x02, 0x56, 0x39, 0x68, 0x00, 0x00, 0x34, 0x68, 0xa2, 0x42, 0xf9, 0x68, 0x5e, 0x01, 0xf9,
            0x68, 0x06, 0x56, 0x39, 0x68, 0x00, 0x00, 0x34, 0x68, 0xa6, 0x42, 0xf9, 0x68, 0x62, 0x01, 0xf9,
            0x68, 0x0a, 0x56, 0x39, 0x68, 0x00, 0x00, 0x34, 0x68, 0xaa, 0x42, 0xf9, 0x68, 0x66, 0x01, 0xf9,
        ],
        note: "The loop's increment (`add x20, x20, #1`), where the call returns to after `b`: the final input mapping, including the CPU pilot's hook at its exit, has finished, so the record is complete. The delay is applied here to the record noted at `input_caller_a_note`; a pass through the loop that made no call has no note.",
    },
    Site {
        name: "input_caller_b_note",
        feature: Feature::InputDelay,
        offset: 0x175523c,
        kind: SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid },
        guard: [
            0xe0, 0x03, 0x09, 0xaa, 0xeb, 0x17, 0x9f, 0x1a, 0x41, 0x7c, 0x40, 0x92, 0xe2, 0x03, 0x08, 0xaa,
            0x64, 0x01, 0x0a, 0x0a, 0xe0, 0xee, 0xff, 0x97, 0xe0, 0x03, 0x00, 0x32, 0xfd, 0x7b, 0xc1, 0xa8,
            0xc0, 0x03, 0x5f, 0xd6, 0x2a, 0x01, 0x40, 0x39, 0x5f, 0x09, 0x00, 0x71, 0xa8, 0xfb, 0xff, 0x54,
            0x4a, 0x05, 0x00, 0x11, 0x4b, 0x1d, 0x00, 0x12, 0x7f, 0x09, 0x00, 0x71, 0x2a, 0x01, 0x00, 0x39,
        ],
        note: "The single-slot wrapper (`main+0x1755160`) around the final input mapping, at `mov x0, x9` before the call: x8 holds the out record (set once from x1 at its entry) and x2 the player index. The hook only notes both for this thread.",
    },
    Site {
        name: "input_caller_b_apply",
        feature: Feature::InputDelay,
        offset: 0x1755254,
        kind: SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid },
        guard: [
            0xe0, 0x03, 0x00, 0x32, 0xfd, 0x7b, 0xc1, 0xa8, 0xc0, 0x03, 0x5f, 0xd6, 0x2a, 0x01, 0x40, 0x39,
            0x5f, 0x09, 0x00, 0x71, 0xa8, 0xfb, 0xff, 0x54, 0x4a, 0x05, 0x00, 0x11, 0x4b, 0x1d, 0x00, 0x12,
            0x7f, 0x09, 0x00, 0x71, 0x2a, 0x01, 0x00, 0x39, 0x08, 0xfb, 0xff, 0x54, 0xe8, 0xff, 0xff, 0x17,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0xb4,
        ],
        note: "The wrapper's instruction after the call (`orr w0, wzr, #1`): the record is complete. The delay is applied here to the record noted at `input_caller_b_note`.",
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
    Site {
        name: "fighter_information",
        feature: Feature::ResetHub,
        offset: 0x2140fc0,
        kind: SeamKind::Called,
        guard: [
            0x3f, 0x20, 0x00, 0x71, 0xe2, 0x00, 0x00, 0x54, 0x08, 0x00, 0x40, 0xf9, 0x29, 0x7c, 0x60, 0xd3,
            0x08, 0x75, 0x89, 0x8b, 0x08, 0x11, 0x40, 0xf9, 0x00, 0xc1, 0x03, 0x91, 0xc0, 0x03, 0x5f, 0xd6,
            0xfd, 0x7b, 0xbf, 0xa9, 0xfd, 0x03, 0x00, 0x91, 0x9a, 0x05, 0x62, 0x94, 0x00, 0x00, 0x00, 0x00,
            0x08, 0x00, 0x40, 0xf9, 0x08, 0x01, 0x43, 0x39, 0x1f, 0x25, 0x00, 0x71, 0xe0, 0x17, 0x9f, 0x1a,
        ],
        note: "The game's FighterManager::get_fighter_information (lua_bind thunk: manager pointer, entry id) used to find a fighter's FighterInformation; a called function, other plugins' hooks allowed. 64 bytes read from the owned 13.0.5 main at the symbol _ZN3app8lua_bind44FighterManager__get_fighter_information_implEPNS_14FighterManagerENS_14FighterEntryIDE.",
    },
    Site {
        name: "is_operation_cpu",
        feature: Feature::ResetHub,
        offset: 0x20c9e30,
        kind: SeamKind::Called,
        guard: [
            0x08, 0x04, 0x40, 0xf9, 0x00, 0x49, 0x42, 0x39, 0xc0, 0x03, 0x5f, 0xd6, 0x00, 0x00, 0x00, 0x00,
            0x08, 0x04, 0x40, 0xf9, 0x00, 0x4d, 0x42, 0x39, 0xc0, 0x03, 0x5f, 0xd6, 0x00, 0x00, 0x00, 0x00,
            0x08, 0x04, 0x40, 0xf9, 0x3f, 0x08, 0x00, 0x31, 0xc1, 0x00, 0x00, 0x54, 0x09, 0xa9, 0x59, 0x29,
            0x08, 0xd5, 0x40, 0xb9, 0x49, 0x01, 0x09, 0x0b, 0x00, 0x01, 0x09, 0x0b, 0xc0, 0x03, 0x5f, 0xd6,
        ],
        note: "The game's FighterInformation::is_operation_cpu (lua_bind thunk), called on the information found above. 64 bytes read from the owned 13.0.5 main at the symbol _ZN3app8lua_bind41FighterInformation__is_operation_cpu_implEPNS_18FighterInformationE.",
    },
];

/// Every site of every stream: the base table plus each stream's own file, concatenated at compile time
/// (one line per stream; the install line order is base first, then each stream in this order).
pub const SITE_TOTAL: usize = BASE_SITES.len() + super::sites_pkg2::SITES_PKG2.len();

const fn concat<const A: usize, const B: usize, const N: usize>(a: &[Site; A], b: &[Site; B]) -> [Site; N] {
    assert!(A + B == N);
    let mut out = [a[0]; N];
    let mut i = 0;
    while i < A {
        out[i] = a[i];
        i += 1;
    }
    let mut j = 0;
    while j < B {
        out[A + j] = b[j];
        j += 1;
    }
    out
}

/// The table, in install-line order.
pub static SITES: [Site; SITE_TOTAL] = concat::<13, { super::sites_pkg2::SITES_PKG2.len() }, SITE_TOTAL>(&BASE_SITES, &super::sites_pkg2::SITES_PKG2);

/// The site called `name`.
pub fn find(name: &str) -> Option<&'static Site> {
    SITES.iter().find(|site| site.name == name)
}

/// The sites a feature needs (its own, then the shared Training predicate when it uses it).
pub fn sites_of(feature: Feature) -> impl Iterator<Item = &'static Site> {
    SITES.iter().filter(move |site| site.feature == feature)
}

/// Whether a feature can be installed: every row of its closure must come out usable (R2-1,
/// `capabilities`): its own rows, the shared Training predicate, the CPU core's rows for the CPU features and
/// the rows of other features its entry points reach.
pub fn feature_usable(feature: Feature, usable: impl Fn(&Site) -> bool) -> bool {
    super::capabilities::unit_usable(super::capabilities::unit_of(feature), &usable)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_site_name_is_unique_and_found() {
        for site in &SITES {
            assert_eq!(find(site.name).map(|s| s.offset), Some(site.offset));
        }
    }

    #[test]
    fn an_unsupported_game_installs_nothing() {
        assert!(install_plan(Verdict::UnsupportedBuild).is_empty());
        assert_eq!(install_plan(Verdict::Supported).len(), SITES.len());
    }
}
