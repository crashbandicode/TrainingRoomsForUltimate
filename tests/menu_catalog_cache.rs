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
    assert!(refresh.contains("s.catalog.ready(version.0)"), "a due menu adopts the kept catalog:\n{refresh}");
    assert!(refresh.contains("set_loading()"), "no catalog yet: the lists say loading:\n{refresh}");
    let session = item(&worker, "struct Session {");
    assert!(session.contains("catalog: CatalogCache"), "the cache outlives the menu model:\n{session}");
    let run = item(&worker, "fn run() {");
    assert!(run.contains("session.catalog.want(version.0)"), "{run}");
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
    assert!(open.contains("s.catalog.check()"), "{open}");
    let run = item(&worker, "fn run() {");
    assert!(
        run.contains("if session.catalog.take_external_change() {\n            crate::engine::rr_host::bump_catalog();"),
        "{run}"
    );
}

#[test]
fn a_drill_is_not_cleared_while_the_catalog_loads() {
    let worker = src("src/runtime/ui/worker.rs");
    let effect = item(&worker, "fn post_recording_effect(");
    assert!(effect.contains("screens.loading()"), "no catalog, no drill kind to follow:\n{effect}");
}

#[test]
fn recording_actions_and_writeback_require_the_current_generation() {
    let worker = src("src/runtime/ui/worker.rs");
    let refresh = item(&worker, "fn refresh_recording(");
    assert!(!refresh.contains("!s.catalog_watch.adopted()"), "an adopted but superseded catalog must enter loading");
    let effect = item(&worker, "fn post_recording_effect(");
    let check = effect.find("observed != Some(version)").expect("check the adopted revision before dispatch");
    assert!(check < effect.find("publish_selection_at(").unwrap(), "check before selection writeback");
    assert!(check < effect.find("post_effect_at(").unwrap(), "check before dispatching a file effect");
    let host = src("src/model/rr_host.rs");
    let consume = item(&host, "fn apply_menu(");
    assert!(consume.find("catalog_generation()").unwrap() < consume.find("match effect").unwrap(),
        "recheck after taking a queued file action");
    let index = src("src/model/library/hot.rs");
    let write = item(&index, "    pub fn save_at(");
    assert!(write.find("recording_write_guard(").unwrap() < write.find("catalog_generation()").unwrap());
    assert!(write.find("catalog_generation()").unwrap() < write.find("write_replace(").unwrap());
}

#[test]
fn recording_editor_writes_share_the_store_invalidation_boundary() {
    let editor = src("src/runtime/macros/named_macros.rs");
    assert!(item(&editor, "fn save_new(recording:").contains("library().save_new(&recording)"));
    let combos = src("src/runtime/ui/combos.rs");
    assert!(item(&combos, "fn save(&mut self)").contains("library.save_new(&record)"));
    assert!(item(&combos, "fn named(&mut self").contains("library.overwrite(&id, &record)"));
    assert!(item(&combos, "fn delete(&mut self").contains("library.delete(id)"));
    let store = src("src/model/store.rs");
    for (write, notify) in [("fn write_atomic(", "note_catalog_write("), ("pub fn delete(", "note_catalog_write(")] {
        assert!(item(&store, write).contains(notify), "{write} must invalidate the exact record after success");
    }
}
