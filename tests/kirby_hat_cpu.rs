//! B4-3 (Surface lane, `artifacts/phase23/LANE-SURFACE-REPORT.md` "B4-3 root cause"; Ally lane root cause in
//! `LANE-ALLY-REPORT.md` "Round 4" §3): the CPU Kirby hat request stuck forever because `kirby_hats::on_frame` ran
//! only with Player 1's body (`lobby::frame`, entry 0), so the CPU role's Kirby never matched its own request.
//! The fork calls it from Kirby's per-frame copy routine (`SITE_KIRBY_OPFF = 0xb971d0`) with each Kirby's own
//! body; TrainingRooms now does the same from `state_kirby_opff_hook` on the same site.

fn source(path: &str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).expect(path)
}

fn body_of<'a>(text: &'a str, signature: &str) -> &'a str {
    let start = text.find(signature).unwrap_or_else(|| panic!("{signature} not found"));
    let end = text[start..].find("\n}\n").expect("function end") + start;
    &text[start..end]
}

/// The call site: Kirby's copy routine hook (the fork's site) runs the hat step with the hooked fighter's own
/// accessor, after the original; `lobby::frame` (Player 1's body only) no longer runs it.
#[test]
fn the_hat_step_runs_from_kirbys_copy_routine_with_its_own_body() {
    let states = source("src/runtime/features/states.rs");
    let hook = body_of(&states, "pub unsafe fn state_kirby_opff_hook");
    let original = hook.find("call_original!").expect("original call");
    let step = hook.find("kirby_hats::on_frame(").expect("the hat step in the Kirby hook");
    assert!(original < step, "the hat step runs after the original, as the fork's does");
    assert!(hook[..step].contains("FIGHTER_ACCESSOR_FIELD"), "the hooked fighter's own accessor");
    let lobby = source("src/runtime/lobby/mod.rs");
    assert!(!lobby.contains("kirby_hats::on_frame("), "lobby::frame runs on Player 1's body only");
}

/// The guard question: the hook installs on its own row when the roster cycle started, not only with every
/// save-state row (`state_boot::install`), and boot decides that before the install.
#[test]
fn the_kirby_hook_does_not_depend_on_the_other_save_state_rows() {
    let boot = source("src/runtime/state_boot.rs");
    let install = body_of(&boot, "pub fn install(");
    let hook = install.find("install_hook!(states::state_kirby_opff_hook)").expect("the Kirby hook install");
    let all_rows = install.find("if !all").expect("the all-rows return");
    assert!(hook < all_rows, "installed before the all-rows return");
    assert!(install[..hook].contains("states::KIRBY_HATS"));
    let main = source("src/runtime/boot.rs");
    let set = main.find("states::KIRBY_HATS.store(").expect("boot sets the flag");
    assert!(main[..set].contains("roster_on && state_row_usable(\"state_kirby_opff\")"));
    assert!(set < main.find("state_boot::install(").expect("state install"));
}
