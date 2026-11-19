//! Menu speed (Switch 2026-11-19, 30 macros + 25 saves: the first page after each menu open waited 0.4–1.2 s): the menu
//! worker keeps the recording catalog across menu opens, builds it a file at a time between its input passes, and never
//! reads the card for a menu open. Read from the sources the Switch build compiles (the runtime is not built on the host).
use std::path::Path;

fn src(path: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}

/// The text of the item that starts with `head`, to its closing brace at column 0.
fn item<'a>(text: &'a str, head: &str) -> &'a str {
    let at = text.find(head).unwrap_or_else(|| panic!("{head} not found"));
    let end = text[at..].find("\n}\n").map_or(text.len(), |e| at + e + 3);
    &text[at..end]
}

#[test]
fn a_menu_open_takes_the_kept_catalog_and_never_reads_the_card() {
    let worker = src("src/runtime/ui/worker.rs");
    assert!(!worker.contains("load_catalog("), "the worker reads the card only through the stepped CatalogCache");
    let refresh = item(&worker, "fn refresh_recording(");
    assert!(refresh.contains("s.catalog.ready(gen)"), "a due menu adopts the kept catalog:\n{refresh}");
    assert!(refresh.contains("set_loading()"), "no catalog yet: the lists say loading:\n{refresh}");
    let session = item(&worker, "struct Session {");
    assert!(session.contains("catalog: CatalogCache"), "the cache outlives the menu model:\n{session}");
    let run = item(&worker, "fn run() {");
    assert!(run.contains("session.catalog.want(crate::engine::rr_host::catalog_generation())"), "{run}");
    assert!(run.contains("session.catalog.step()"), "one file per pass, input drained between: {run}");
    let handle = item(&worker, "fn handle(event: Event, s: &mut Session) {");
    let open = &handle[..handle.find("let Event::Poll").expect("the poll branch")];
    let refreshed = open.find("refresh_recording(s)").expect("an open adopts the kept catalog before its first frame");
    assert!(refreshed < open.find("publish_frame(s)").unwrap(), "{open}");
}

/// Files copied outside TR: each menu open starts the folder check, and a change it finds moves the catalog generation
/// in the worker loop (the stepped rebuild then picks it up).
#[test]
fn a_menu_open_checks_the_folders_and_a_change_moves_the_generation() {
    let worker = src("src/runtime/ui/worker.rs");
    let handle = item(&worker, "fn handle(event: Event, s: &mut Session) {");
    let open = &handle[..handle.find("let Event::Poll").expect("the poll branch")];
    assert!(open.contains("s.catalog.check(crate::engine::rr_host::catalog_generation())"), "{open}");
    let run = item(&worker, "fn run() {");
    assert!(
        run.contains("if session.catalog.take_external_change() {\n            crate::engine::rr_host::bump_catalog();"),
        "{run}"
    );
}

#[test]
fn a_drill_is_not_cleared_while_the_catalog_loads() {
    let worker = src("src/runtime/ui/worker.rs");
    let effect = item(&worker, "fn post_recording_effect(menu: &mut Menu) {");
    assert!(effect.contains("menu.recording().loading()"), "no catalog, no drill kind to follow:\n{effect}");
}
