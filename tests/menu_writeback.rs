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
