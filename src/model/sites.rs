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
#[cfg(test)]
use super::version::Verdict;

macro_rules! features {
    ($( $(#[$meta:meta])* $variant:ident => $name:literal, )+) => {
        /// The feature a site belongs to: installed only when every one of its sites is usable.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Feature {
            $( $(#[$meta])* $variant, )+
        }

        impl Feature {
            pub const fn name(self) -> &'static str {
                match self {
                    $( Self::$variant => $name, )+
                }
            }
        }
    };
}

features! {
    /// Shared by every Training feature (the Training-mode predicate).
    Training => "training",
    InputDelay => "input_delay",
    LraReset => "lra_reset",
    /// The Training reset hub (SPEC-state-06 §5): the fighter on-init wrapper that runs it.
    ResetHub => "reset_hub",
    StageHazards => "stage_hazards",
    /// The CPU behaviours' shared plumbing (package 2): the per-frame dispatch, the game's constant table
    /// and the natives the facts are read through.
    CpuCore => "cpu_core",
    /// SPEC-cpu-02: SDI, air-dodge direction, clatter (DI needs the `common` module, see `cpu::common`).
    CpuDefence => "cpu_defence",
    /// SPEC-cpu-03: the ledge options.
    CpuLedge => "cpu_ledge",
    /// SPEC-cpu-04: tech, missed tech, hidden tech animations (the camera and the script status-change
    /// hook need sites this table does not have yet, see `IMPLEMENTATION-PROGRESS.md (removed after v0.2.0-rc.1; see that tag)`).
    CpuTech => "cpu_tech",
    /// SPEC-cpu-05: shield hold, decay override and tilt (the out-of-shield hook is in the `common`
    /// script module and the damage multiplier needs the param-hook plugin, see `IMPLEMENTATION-PROGRESS.md (removed after v0.2.0-rc.1; see that tag)`).
    CpuShield => "cpu_shield",
    /// Package 3 (save states): the natives and hooks of SPEC-state-01 (`sites_pkg3.rs`).
    StateCore => "state_core",
    /// Package 3: the charge writes and visuals (SPEC-state-02).
    StateCharge => "state_charge",
    /// Package 3: the buffs and their five hooks (SPEC-state-03).
    StateBuff => "state_buff",
    /// Package 3: the held-item restore and its sixteen turnip hooks (SPEC-state-04).
    StateItems => "state_items",
    /// Package 3: Steve, Olimar, Sora and the fighter-specific restores (SPEC-state-05).
    StateRestore => "state_restore",
    /// Package 4: the input recorder (SPEC-input-01): the natives it reads and writes through and the
    /// hook at the AI control function's epilogue.
    Recording => "recording",
    /// Package 4: HUD notices (SPEC-input-05): the Combo Display observers and the stale-moves toggle.
    Notices => "notices",
    /// Package 4, S4-4: the named-macro layer's game calls (control reads, kinetic reads, the panel's hold and the restore pose writes).
    NamedMacros => "named_macros",
    /// Package 5: the layout archive hook, the UI anchors and the manager global (`sites_ui.rs`).
    Ui => "ui",
    /// The input display's reads in the input-delay hook (the P1 body's status, found through the fighter manager): no
    /// rows of its own, only the natives it calls (R2-1). Input Delay installs on its own rows; the display degrades apart.
    InputDisplay => "input_display",
    /// The run-time texture helper's rows (`sites_u4.rs`, `runtime/ui/texture.rs`): a unit of its own (they are the Ui feature's rows in
    /// their table), so the menu does not depend on a helper that is optional (R2-1 merge with u4).
    UiTexture => "ui_texture",
    /// The hooks in the `common` script module (DI, tech status change, out of shield), installed when
    /// that module loads through the `libnro_hook` provider.
    /// SPEC-cpu-01 and SPEC-cpu-06: the mash's command flags, aerial kind and attack angle.
    CpuMash => "cpu_mash",
    CpuCommon => "cpu_common",
    /// Package 6: the CPU-as-P1 fallback's game calls (sites_pkg6.rs); its hook is the carried seam row cpu_pilot.ai_process_begin.
    P1Cpu => "p1_cpu",
    /// Package 6: the roster cycle's game calls that no other package guards (sites_pkg6.rs `roster_*` rows); the cycle's hooks are carried seam rows.
    Roster => "roster",
    /// Reserved legacy source-owner feature id; its diagnostic calls were retired for the RC.
    Probes => "probes",
    /// SPEC-core-06 guarded visualizer.
    Hitbox => "hitbox",
    /// SPEC-cpu-05's shield damage multiplier: the game's CommonParams object, found at the moment the game fills
    /// it from its parameter file (no provider plugin).
    CpuShieldDamage => "cpu_shield_damage",
    /// SPEC-cpu-04's fixed camera: the stage id, the stage table and the two camera hooks.
    CpuCamera => "cpu_camera",
    DeveloperDebug => "developer_debug",
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
    #[cfg(test)]
    pub const fn window_end(&self) -> usize {
        self.offset + GUARD_BYTES
    }

    /// Whether the two 64-byte windows share bytes (a hook at one then changes what the other reads).
    #[cfg(test)]
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
pub static BASE_SITES: [Site; 13] = include!(concat!(env!("OUT_DIR"), "/tables/sites.BASE_SITES.rs"));

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
#[cfg(test)]
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
#[cfg(test)]
pub fn install_plan(verdict: Verdict) -> &'static [Site] {
    match verdict {
        Verdict::Supported => &SITES,
        _ => &[],
    }
}

/// Boot's two phases for sites (SPEC-core-01 §5.1 step 5b, §6 "Overlapping thunks"): every window is
/// compared first, then anything is written. A write at one site changes the bytes another site's
/// window reads, so checking after installing would see our own hook as a foreign change.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    CheckEveryWindow,
    InstallUsableFeatures,
}

#[cfg(test)]
pub const BOOT_PHASES: [Phase; 2] = [Phase::CheckEveryWindow, Phase::InstallUsableFeatures];

/// Index pairs `(a, b)`, `a < b`, of sites whose 64-byte windows overlap.
#[cfg(test)]
pub fn overlapping_pairs() -> impl Iterator<Item = (usize, usize)> {
    (0..SITES.len())
        .flat_map(|a| (a + 1..SITES.len()).map(move |b| (a, b)))
        .filter(|&(a, b)| SITES[a].overlaps(&SITES[b]))
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn every_site_name_is_unique_and_found() {
        for site in &SITES {
            assert_eq!(find(site.name).map(|s| s.offset), Some(site.offset));
        }
    }

    fn an_unsupported_game_installs_nothing() {
        assert!(install_plan(Verdict::UnsupportedBuild).is_empty());
        assert_eq!(install_plan(Verdict::Supported).len(), SITES.len());
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("every_site_name_is_unique_and_found", every_site_name_is_unique_and_found as fn()),
            ("an_unsupported_game_installs_nothing", an_unsupported_game_installs_nothing as fn()),
        ]);
    }
    }
}
