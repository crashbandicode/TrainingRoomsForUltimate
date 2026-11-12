//! The seam checks against this plugin's own writes (`model::own_patches`, `runtime::seams::seam_install::prime` /
//! `seal`) on the real rows: a group checked after another of our modules hooked inside its windows still verifies, and
//! another plugin's hook over ours still does not.

lib_unit_tests! {
use std::collections::BTreeMap;

use super::native_seams::{self, find, Seam, PATCH_SITES, SITES};
use crate::model::guard::{GuardOutcome, BR_X17, GUARD_BYTES, LDR_X17_LITERAL_8, NOP};
use crate::model::own_patches::{unpatch, OwnPatch, SPAN};

/// `practice_stage::init`'s enforcing group (`src/runtime/lobby/practice_stage.rs`).
const PRACTICE_STAGE: [&str; 9] = [
    "practice_stage.stage_update",
    "practice_stage.update_fields",
    "lobby_stage.resource_request",
    "practice_stage.item_manager",
    "practice_stage.item_predicate",
    "practice_stage.item_predicate+0x40",
    "practice_stage.item_list_init",
    "practice_stage.item_list_init+0x40",
    "practice_stage.scene_fields",
];

/// `main`'s code as the rows know it: every compared row's 64 bytes (rows that overlap agree: they come from one ELF).
fn text() -> BTreeMap<usize, u8> {
    let mut text = BTreeMap::new();
    for seam in native_seams::all().filter(|seam| seam.compared()) {
        for (i, byte) in seam.guard.iter().enumerate() {
            let previous = text.insert(seam.offset + i, *byte);
            assert!(previous.is_none_or(|b| b == *byte), "{} disagrees with an overlapping row", seam.label);
        }
    }
    text
}

fn read<const N: usize>(text: &BTreeMap<usize, u8>, at: usize) -> Option<[u8; N]> {
    let bytes: Option<Vec<u8>> = (at..at + N).map(|a| text.get(&a).copied()).collect();
    bytes.map(|bytes| bytes.try_into().unwrap())
}

/// Skyline's far jump to `target` at `site`, with the `nop` when the site is 4 past an 8-byte boundary.
fn hook(text: &mut BTreeMap<usize, u8>, site: usize, target: u64) {
    let mut words = Vec::new();
    if site % 8 == 4 {
        words.push(NOP);
    }
    words.extend([LDR_X17_LITERAL_8, BR_X17, target as u32, (target >> 32) as u32]);
    for (i, word) in words.iter().enumerate() {
        for (j, byte) in word.to_le_bytes().iter().enumerate() {
            text.insert(site + 4 * i + j, *byte);
        }
    }
}

/// `seam_install::prime`: the bytes before our boot wrote anything, at every compared hook row the table knows.
fn prime(text: &BTreeMap<usize, u8>) -> Vec<OwnPatch> {
    PATCH_SITES
        .iter()
        .filter_map(|&index| {
            let offset = SITES[index].offset;
            read::<SPAN>(text, offset).map(|before| OwnPatch { offset, before, after: None })
        })
        .collect()
}

/// `seam_install::seal`: what our writes left.
fn seal(text: &BTreeMap<usize, u8>, patches: &mut [OwnPatch]) {
    for patch in patches {
        patch.after = read::<SPAN>(text, patch.offset);
    }
}

fn row(label: &str) -> &'static Seam {
    find(label).unwrap_or_else(|| panic!("no row {label}")).1
}

/// `seam_install::check` on this text: the window, our own writes put back, the row's rule.
fn check(text: &BTreeMap<usize, u8>, patches: &[OwnPatch], label: &str) -> GuardOutcome {
    let seam = row(label);
    let actual: [u8; GUARD_BYTES] = read(text, seam.offset).expect("the row's own bytes");
    let (seen, _) = unpatch(seam.offset, &actual, patches.iter().copied());
    seam.outcome(&seen)
}

fn every_hook_row_in_code_is_a_patch_site_and_nothing_else_is() {
    let expected: Vec<usize> = native_seams::all()
        .enumerate()
        .filter(|(_, seam)| matches!(seam.kind, crate::model::guard::SeamKind::Hook { .. }) && seam.compared()
            && seam.region() == Some(crate::model::guard::MainRegion::Text))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(PATCH_SITES.to_vec(), expected);
    assert!(PATCH_SITES.iter().any(|&index| SITES[index].label == "lobby_stage.resource_request"));
}

fn the_practice_stage_group_verifies_after_the_lobby_stage_hook_in_the_wrong_order() {
    let mut text = text();
    let mut patches = prime(&text);
    // rc.2's order: `lobby_stage::init` hooks the stage-resource request before `practice_stage::init` checks.
    let site = row("lobby_stage.resource_request").offset;
    hook(&mut text, site, 0x8_0010_2000);
    // Today's bug: read as it is, the row is our own hook, `occupied`, and the enforcing group is skipped.
    let raw: [u8; GUARD_BYTES] = read(&text, site).unwrap();
    assert_eq!(row("lobby_stage.resource_request").outcome(&raw).code(), "occupied");
    // While the boot installs, the change since `prime` is ours: every row of the group verifies.
    for label in PRACTICE_STAGE {
        assert_eq!(check(&text, &patches, label), GuardOutcome::Stock, "{label} while booting");
    }
    // After the install line the records hold exactly our write: the same answer, now and in a late check.
    seal(&text, &mut patches);
    for label in PRACTICE_STAGE {
        assert_eq!(check(&text, &patches, label), GuardOutcome::Stock, "{label} after the seal");
    }
    // Another plugin's hook over ours afterwards is not ours: occupied again, as it should be.
    hook(&mut text, site, 0x9_0020_3000);
    assert_eq!(check(&text, &patches, "lobby_stage.resource_request").code(), "occupied");
}

fn every_group_reads_stock_with_all_our_hooks_installed_in_any_order() {
    // Every compared hook row's site hooked (the most our boot can write), checked after all of them: every compared row
    // reads stock, in either order, because each write is put back to the bytes it replaced.
    let mut text = text();
    let mut patches = prime(&text);
    let sites: Vec<usize> = patches.iter().map(|patch| patch.offset).collect();
    for (n, site) in sites.iter().rev().enumerate() {
        hook(&mut text, *site, 0x8_0000_0000 + 0x100 * n as u64);
    }
    for stage in ["booting", "sealed"] {
        if stage == "sealed" {
            seal(&text, &mut patches);
        }
        for seam in native_seams::all().filter(|seam| seam.compared()) {
            let actual: [u8; GUARD_BYTES] = read(&text, seam.offset).unwrap();
            let (seen, _) = unpatch(seam.offset, &actual, patches.iter().copied());
            assert_eq!(seen, seam.guard, "{} ({stage})", seam.label);
        }
    }
}

fn a_refused_row_is_named_once_in_the_shipped_log() {
    use super::native_seams::report::{names_row, RefusedLine, FOREIGN, OCCUPIED, OTHER, OUT_OF_REGION, SKIPPED, STOCK, UNCHECKED};
    // A first check, or a change, to a refused code names the row; the same code again, or a usable one, does not.
    for code in [OCCUPIED, FOREIGN, OUT_OF_REGION] {
        assert!(names_row(UNCHECKED, code) && names_row(STOCK, code) && names_row(STOCK | SKIPPED, code));
        assert!(!names_row(code | SKIPPED, code), "the same code is named once");
    }
    assert!(names_row(OCCUPIED, FOREIGN));
    for code in [STOCK, OTHER] {
        assert!(!names_row(UNCHECKED, code) && !names_row(OCCUPIED, code));
    }
    // The line names the row, its group, its offset and the outcome, under a shipped tag (`diag_ring::SHIPPED_TAGS`).
    let line = RefusedLine { label: "lobby_stage.resource_request", group: "lobby_stage_probe", offset: 0x25f7874, code: OCCUPIED };
    let text = line.to_string();
    assert_eq!(text, "[TrainingSeams] seam=lobby_stage.resource_request group=lobby_stage_probe offset=0x25f7874 outcome=occupied");
    assert!(crate::model::diag_ring::shipped(text.as_bytes()));
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("every_hook_row_in_code_is_a_patch_site_and_nothing_else_is", every_hook_row_in_code_is_a_patch_site_and_nothing_else_is as fn()),
        ("the_practice_stage_group_verifies_after_the_lobby_stage_hook_in_the_wrong_order", the_practice_stage_group_verifies_after_the_lobby_stage_hook_in_the_wrong_order as fn()),
        ("every_group_reads_stock_with_all_our_hooks_installed_in_any_order", every_group_reads_stock_with_all_our_hooks_installed_in_any_order as fn()),
        ("a_refused_row_is_named_once_in_the_shipped_log", a_refused_row_is_named_once_in_the_shipped_log as fn()),
    ]);
}
}
