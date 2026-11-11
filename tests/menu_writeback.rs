//! rc.8 §6: the typed menu's writeback is wait-free on both sides. `compat/menu.rs::PENDING` was a mutex game threads took
//! (the native service's poll: `sync_selection` -> `menu::store`) while the menu worker could hold it, the rc.6 freeze class.
//! `tests/lock_order.rs` (`no_lock_a_game_hook_waits_on_is_held_by_a_plugin_thread`) checks the class; this checks the handoff.
#[path = "../src/model/menu_writeback.rs"]
mod menu_writeback;
use menu_writeback::newest;

fn pick(game: (Option<(u64, char)>, bool), publish: (Option<(u64, char)>, bool), adopted: u64) -> Option<(u64, char)> {
    newest(&[(&game.0, game.1), (&publish.0, publish.1)], adopted).copied()
}

#[test]
fn the_worker_adopts_the_newest_fresh_value_it_has_not_adopted() {
    // Game threads and the publishing path, one buffer each: the later stamp wins, whichever side wrote it.
    assert_eq!(pick((Some((5, 'g')), true), (Some((6, 'p')), true), 0), Some((6, 'p')));
    assert_eq!(pick((Some((7, 'g')), true), (Some((6, 'p')), true), 0), Some((7, 'g')));
    // A value already taken (not fresh) is not adopted again.
    assert_eq!(pick((Some((5, 'g')), false), (None, false), 0), None);
    assert_eq!(pick((Some((5, 'g')), false), (Some((6, 'p')), true), 0), Some((6, 'p')));
    // Stamped before the value the worker adopted last, but written after it: the newer adoption stands.
    assert_eq!(pick((Some((7, 'g')), true), (None, false), 8), None);
    assert_eq!(pick((None, true), (None, true), 0), None);
}

fn body<'a>(src: &'a str, start: &str) -> &'a str {
    let at = src.find(start).unwrap_or_else(|| panic!("{start}"));
    &src[at..at + src[at..].find("\n}\n").expect("end of the function")]
}

const MENU: &str = include_str!("../src/runtime/compat/menu.rs");

#[test]
fn no_writeback_side_takes_a_lock() {
    assert!(!MENU.contains("static PENDING"), "the writeback mutex is back");
    for (function, step) in [
        ("fn store(", "GAME_WRITEBACK.write(Some((writeback_stamp(), menu)));"),
        ("pub(crate) fn after_publish(", "PUBLISH_WRITEBACK.write(Some((writeback_stamp(), next)));"),
        ("pub fn flush_writeback(", "GAME_WRITEBACK.with_latest(|game, game_fresh| {"),
        ("pub fn flush_writeback(", "PUBLISH_WRITEBACK.with_latest(|publish, publish_fresh| {"),
    ] {
        let body = body(MENU, function);
        assert!(body.contains(step), "{function}: {body}");
        assert!(!body.contains("PENDING"), "{function}");
    }
    assert!(!body(MENU, "pub fn flush_writeback(").contains(".lock()"), "the menu worker's read never waits");
    // Game threads write their buffer one at a time: inside the `EDITED` scope `store` already holds.
    let store = body(MENU, "fn store(");
    let scope = &store[store.find("let mut edited = EDITED.lock();").expect("EDITED")..];
    let scope = &scope[..scope.find("\n    }\n").expect("the scope's end")];
    assert!(scope.contains("GAME_WRITEBACK.write("), "{store}");
}

/// Every `.rs` file under `src/runtime`, with its path relative to it.
fn runtime_sources() -> Vec<(String, String)> {
    fn walk(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, root, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                out.push((rel, std::fs::read_to_string(&path).unwrap()));
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let mut out = Vec::new();
    walk(&root, &root, &mut out);
    out
}

/// The calls of `name(` outside its own definition: `(file, the enclosing fn's header line)`.
fn callers(name: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (file, text) in runtime_sources() {
        for (at, _) in text.match_indices(&format!("{name}(")) {
            let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
            // Not its definition, not a longer name, not a method (`ADOPTED.store(`): these are plain or path calls.
            if text[line_start..at].contains("fn ") || !text[..at].ends_with(|c: char| !c.is_alphanumeric() && c != '_' && c != '.') {
                continue;
            }
            let header = text[..at].rmatch_indices("\nfn ").chain(text[..at].rmatch_indices("\npub fn ")).map(|(i, _)| i).max()
                .map_or(String::new(), |i| text[i + 1..].lines().next().unwrap_or("").to_owned());
            out.push((file.clone(), header));
        }
    }
    out
}

/// rc.8 R8 (B's finding 3): each triple buffer has one reader and its writers take turns (the contract
/// `hos_lock::TripleBuffer` states). The one reader is the menu worker's loop; `after_publish` runs only inside
/// `publish`, under `PUBLISHING`. A "flush before publish" call elsewhere, or a second publisher outside that lock, would
/// share a slot with no other test failing.
#[test]
fn the_writeback_buffers_keep_one_reader_and_one_writer_at_a_time() {
    assert_eq!(callers("flush_writeback"), vec![("ui/worker.rs".to_owned(), "fn run() {".to_owned())],
        "the menu worker's loop is the one reader");
    assert_eq!(callers("after_publish"), vec![("published.rs".to_owned(), "pub fn publish(snapshot: &Snapshot, rerolls: u8) {".to_owned())]);
    let published = runtime_sources().into_iter().find(|(f, _)| f == "published.rs").unwrap().1;
    let publish = body(&published, "pub fn publish(");
    let turn = publish.find("let _turn = PUBLISHING.lock();").expect("the publisher's turn");
    assert!(turn < publish.find("after_publish(").unwrap(), "after_publish runs under PUBLISHING: {publish}");
    // `store`, the game side's writer, is reached only from the typed menu's guard and assign (serialised by EDITED).
    let menu_callers: Vec<String> = callers("store").into_iter().filter(|(f, _)| f == "compat/menu.rs").map(|(_, h)| h).collect();
    assert_eq!(menu_callers.len(), 2, "{menu_callers:?}");
}
