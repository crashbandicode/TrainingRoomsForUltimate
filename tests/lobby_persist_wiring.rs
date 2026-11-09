//! rc.6, the waiting room's Training partners and CPU action persist (task `TASK-OPUS-rc6-lobby-cpu-settings.md`
//! requirement 1; design `artifacts/phase23/rc6-lobby-persist/DESIGN.md`). The rule is host-tested in
//! `src/model/lobby_persist.rs`; this pins the settings key through the real settings code and the Switch runtime's
//! order: the entry hook applies through the game's own row handler, the row handler hook saves the player's change, the
//! writer stores it, and nothing runs outside the two waiting families.
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use std::path::Path;

use model::lobby_persist::{Saved, ATTACK, KEY, STOP};
use model::settings::{parse, Settings};

fn source(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn body<'a>(src: &'a str, start: &str) -> &'a str {
    let at = src.find(start).unwrap_or_else(|| panic!("missing: {start}"));
    &src[at..at + src[at..].find("\n}\n").expect("end of the function")]
}

fn ordered(src: &str, steps: &[&str]) {
    let mut from = 0;
    for step in steps {
        let at = src[from..].find(step).unwrap_or_else(|| panic!("missing or out of order: {step}\n{src}"));
        from += at + step.len();
    }
}

#[test]
fn the_writer_stores_the_saved_pair() {
    let mut settings = Settings::factory();
    assert_eq!(settings.lobby_practice_panel(), Saved::NONE, "a new install has nothing saved");
    let saved = Saved { partners: Some(1), behavior: Some(STOP) };
    settings.set_lobby_practice_panel(saved);
    let loaded = parse(&settings.to_json());
    assert!(!loaded.corrupt(), "{:?}", loaded.issues);
    assert_eq!(loaded.settings.lobby_practice_panel(), saved);
    assert!(loaded.settings.extra.contains_key(KEY));
    // A page reset restores menu rows only; the pair is not a menu row.
    assert_eq!(loaded.settings.menu, Settings::factory().menu);
    settings.set_lobby_practice_panel(Saved { partners: None, behavior: Some(ATTACK) });
    assert_eq!(parse(&settings.to_json()).settings.lobby_practice_panel().partners, None);
    // The writer thread asks for it with the intro flag's pattern: a due flag, then the live settings.
    let writer = source("src/runtime/features/writer.rs");
    ordered(body(&writer, "fn run("), &["let pair = crate::runtime::lobby::lobby_persist::store_due();",
        "settings_json_for_write(intro, pair)", "crate::runtime::lobby::lobby_persist::clear_store(pair);"]);
    let worker = source("src/runtime/ui/worker.rs");
    ordered(body(&worker, "pub fn settings_json_for_write("), &["if let Some(saved) = pair {",
        "settings.set_lobby_practice_panel(saved);", "Some(settings.to_json())"]);
}

#[test]
fn the_entry_hook_applies_the_saved_pair_through_the_row_handler() {
    let src = source("src/runtime/lobby/lobby_persist.rs");
    let entry = body(&src, "unsafe fn entry_applied(");
    ordered(entry, &["if !waiting() {", "let holder = ctx.registers[23].x() as usize;",
        "let Some(panel) = panel_of(holder) else", "let Some(choices) = choices(panel) else",
        "restore(published(), choices.as_slice())", "apply(panel, step)"]);
    let apply = body(&src, "unsafe fn apply(");
    ordered(apply, &["APPLYING.store(true, Ordering::Release);", "PARTNERS_ROW =>",
        "write::<u32>(panel, PARTNERS, step.value)", "write::<u32>(panel, PARTNERS_INDEX, step.index)",
        "BEHAVIOR_ROW =>", "write::<u32>(panel, BEHAVIOR, step.value)",
        "row_handler(ctx.as_ptr() as usize, &row, &index)", "APPLYING.store(false, Ordering::Release);"]);
    let init = body(&src, "pub(crate) fn init(");
    ordered(init, &["gate(\"lobby_persist\", ROWS, true)", "skyline::install_hooks!(entry_applied, setter_entry);"]);
    assert!(source("src/runtime/lobby/mod.rs").contains("lobby_persist::init();"));
}

#[test]
fn the_row_handler_hook_saves_the_players_change() {
    let src = source("src/runtime/lobby/lobby_persist.rs");
    let hook = body(&src, "unsafe fn setter_entry(");
    ordered(hook, &["if APPLYING.load(Ordering::Acquire) || !waiting() {", "let row = read::<i32>(ctx.registers[1].x() as usize, 0);",
        "let index = read::<u32>(ctx.registers[2].x() as usize, 0);", "let Some(panel) = child(ctx.registers[0].x() as usize, 8)",
        "chosen(row, index, choices.as_slice())", "save(change)"]);
    let save = body(&src, "fn save(");
    ordered(save, &["let next = published().with(change);", "if next == published() {", "SAVED.store(next.pack(), Ordering::Release);",
        "STORE_DUE.store(true, Ordering::Release);"]);
    // Boot publishes what the file holds.
    ordered(&src, &["pub(crate) fn publish(saved: Saved)", "SAVED.store(saved.pack(), Ordering::Release);"]);
    assert!(source("src/runtime/boot.rs").contains("crate::runtime::lobby::lobby_persist::publish("));
}

#[test]
fn the_hooks_act_only_in_the_two_waiting_families() {
    let src = source("src/runtime/lobby/lobby_persist.rs");
    let waiting = body(&src, "fn waiting(");
    ordered(waiting, &["crate::runtime::lobby::game_mode::current()", "family(mode.mode).is_some()"]);
    for hook in ["unsafe fn entry_applied(", "unsafe fn setter_entry("] {
        assert!(body(&src, hook).contains("waiting()"), "{hook}");
    }
    // The rows name both hook sites, the row handler the apply calls, and the panel layout the hooks rely on.
    let rows = source("src/model/seams/native_seams/rows_lobby_persist.rs");
    for label in ["lobby_persist.entry_applied", "lobby_persist.setter_entry", "lobby_persist.row_handler",
        "lobby_persist.entry_defaults", "lobby_persist.behavior_label"] {
        assert!(rows.contains(&format!("label: \"{label}\"")), "{label}");
    }
    assert!(source("src/runtime/seams/seam_install.rs").contains("\"lobby_persist\","), "the group is authoritative");
}
