//! rc.2's Direct stage pick never changed the stage: `runtime::lobby::init` hooked `lobby_stage.resource_request`
//! (`0x25f7874`) before the practice observer checked its seam group, which reads that site, so the row read our own hook
//! (`occupied`), the authoritative group skipped all its sites and no request was ever served (Eden ph2:
//! `group=practice_stage_observer result=skipped`). The fork checks first (`src/lib.rs`: `practice_stage_observer::init`
//! before `lobby_stage_probe::init`). This keeps that order.

#[test]
fn the_practice_observer_checks_its_sites_before_the_lobby_stage_hook_is_installed() {
    let src = include_str!("../src/runtime/lobby/mod.rs");
    let start = src.find("pub(crate) fn init()").expect("lobby init");
    let body = &src[start..start + src[start..].find("\n}").expect("end of init")];
    let observer = body.find("practice_stage::init();").expect("the observer is started");
    let hook = body.find("lobby_stage::init();").expect("the lobby stage hooks are installed");
    assert!(observer < hook, "practice_stage::init must run before lobby_stage::init:\n{body}");
    // The observer's group names the site the lobby stage hook takes.
    let observer_src = include_str!("../src/runtime/lobby/practice_stage.rs");
    assert!(observer_src.contains("\"lobby_stage.resource_request\""));
    let lobby_src = include_str!("../src/runtime/lobby/lobby_stage.rs");
    assert!(lobby_src.contains("install_hooks!(stage_entry, resource_request)"));
}
