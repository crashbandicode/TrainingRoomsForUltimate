//! The hooks in the `common` script module (SPEC-cpu-02 §4 DI, SPEC-cpu-04 §4 tech status change,
//! SPEC-cpu-05 §4 out of shield): their sites are checked against the owned 13.0.5 module, which is read
//! from `research/game-13.0.5/nro/lua2cpp_common.nro` (extracted from `data.arc`; skipped with a visible
//! line only when the file is absent and `SKIP_OWNED_ELF_FACT` is set).

mod common;

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::guard::{GuardOutcome, HookForm, Placement, SeamKind};
use model::sites_pkg2::COMMON_SITES;

#[test]
fn cpu_common_guards_match_the_owned_module() {
    let Some(nro) = common::owned_common_nro("cpu_common_guards_match_the_owned_module") else { return };
    for site in &COMMON_SITES {
        let window = nro.window(site.symbol).unwrap_or_else(|| panic!("{} is not in the owned module", site.symbol));
        assert_eq!(window, site.guard, "{}", site.name);
        assert_eq!(site.outcome(&window), GuardOutcome::Stock);
    }
}

#[test]
fn cpu02_common_guard_gate() {
    // The hook installs only when the function's 64 bytes are the 13.0.5 ones (or another plugin's hook
    // the shared rule tolerates); any other change leaves the function alone.
    let site = COMMON_SITES.iter().find(|s| s.name == "common_di_hook").unwrap();
    assert!(matches!(site.kind(), SeamKind::Hook { form: HookForm::Replace, placement: Placement::Entry }));
    assert!(site.outcome(&site.guard).usable());
    let mut changed = site.guard;
    changed[20] ^= 0xff;
    assert!(!site.outcome(&changed).usable(), "a changed word inside the replaced span is refused");
    // The called function accepts hooks anywhere in its window; its own bytes elsewhere stay compared.
    let called = COMMON_SITES.iter().find(|s| !s.hooked).unwrap();
    assert!(matches!(called.kind(), SeamKind::Called));
    assert!(called.outcome(&called.guard).usable());
}

#[test]
fn cpu_common_the_three_hooks_and_the_called_function_are_the_specs_symbols() {
    let names: Vec<_> = COMMON_SITES.iter().map(|s| (s.name, s.hooked)).collect();
    assert_eq!(
        names,
        [
            ("common_di_hook", true),
            ("common_change_status_hook", true),
            ("common_sub_guard_cont_hook", true),
            ("common_is_enable_passive", false)
        ]
    );
    assert!(COMMON_SITES[0].symbol.contains("FighterStatusDamage__correctDamageVectorCommon"));
    assert!(COMMON_SITES[1].symbol.contains("L2CFighterBase13change_status"));
    assert!(COMMON_SITES[2].symbol.contains("sub_guard_cont"));
    for site in &COMMON_SITES {
        assert!(!site.note.is_empty());
    }
}

#[test]
fn cpu_common_the_module_functions_have_prologues_the_hook_may_replace() {
    // The three hooked functions start with the usual `sub sp, sp, #n` prologue: a replace hook at the
    // entry forwards the arguments untouched (the arguments are by reference, the result is in x8).
    for site in COMMON_SITES.iter().filter(|s| s.hooked) {
        let first = u32::from_le_bytes([site.guard[0], site.guard[1], site.guard[2], site.guard[3]]);
        assert_eq!(first & 0xffc0_03ff, 0xd100_03ff, "{}: not `sub sp, sp, #imm`", site.name);
    }
}
