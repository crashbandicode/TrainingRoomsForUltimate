//! Operator decision C (2026-11-08, #52): a Mii editor Save that matches a saved Mii keeps the reuse rule
//! (docs/MII.md rows 4 and 12: that saved Mii is used, nothing is stored), and it is no longer silent: it posts
//! "Same as ‹saved name›, nothing new saved" with the matched saved Mii's own name. A new save posts nothing.

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::notify::{save_notice, Queue, DEFAULT_COLOR, SAME_AS_SAVED_FRAMES, SAME_AS_SAVED_HEADER};

fn a_save_that_matches_a_saved_mii_names_it() {
    let body = save_notice(Some("格斗")).expect("a reuse posts a notice");
    assert_eq!(body.as_str(), "Same as 格斗, nothing new saved");
    let body = save_notice(Some("covrest")).unwrap();
    assert_eq!(body.as_str(), "Same as covrest, nothing new saved");
}

fn a_new_save_posts_no_notice() {
    assert!(save_notice(None).is_none());
}

fn the_notice_is_drawn_with_that_text() {
    let mut queue = Queue::new();
    queue.post(SAME_AS_SAVED_HEADER, save_notice(Some("Mii")).unwrap(), SAME_AS_SAVED_FRAMES, DEFAULT_COLOR);
    let out = queue.tick(true, false);
    let draw = out.draw.expect("drawn on the first tick");
    assert_eq!(draw.header, SAME_AS_SAVED_HEADER);
    assert_eq!(draw.body.as_str(), "Same as Mii, nothing new saved");
}

fn the_editor_posts_it_only_where_the_saved_mii_is_reused() {
    let bridge = include_str!("../src/runtime/mii/mii_editor_bridge/mod.rs");
    let decide = &bridge[bridge.find("unsafe fn edit_decide").expect("edit_decide")..];
    let decide = &decide[..decide.find("#[skyline::hook").expect("next hook")];
    let reused = decide.find("if let Some(existing) = s.catalog.configs").expect("the reuse branch");
    let notice = decide.find("save_notice(Some(&existing.name))").expect("the notice names the matched saved Mii");
    let requested = decide.find("\"native_save_requested\"").expect("the create branch");
    assert!(reused < notice && notice < requested, "the notice is built in the reuse branch only");
    assert_eq!(decide.matches("save_notice(").count(), 1, "one notice, in the reuse branch");
    // Posted after the session lock is released: the post follows the lock's block.
    let post = decide.find("post_body(").expect("posted through the notice mailbox");
    let unlocked = decide.find("if reused {").expect("after the lock");
    assert!(post > unlocked, "posted outside the lock");
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("a_save_that_matches_a_saved_mii_names_it", a_save_that_matches_a_saved_mii_names_it as fn()),
        ("a_new_save_posts_no_notice", a_new_save_posts_no_notice as fn()),
        ("the_notice_is_drawn_with_that_text", the_notice_is_drawn_with_that_text as fn()),
        ("the_editor_posts_it_only_where_the_saved_mii_is_reused", the_editor_posts_it_only_where_the_saved_mii_is_reused as fn()),
    ]);
}
