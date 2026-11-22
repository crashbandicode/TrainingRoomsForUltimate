//! Operator decision C (2026-11-08, #52): a Mii editor Save that matches a saved Mii keeps the reuse rule
//! (docs/MII.md rows 4 and 12: that saved Mii is used, nothing is stored), and it is no longer silent: it posts
//! "Same as ‹saved name›, nothing new saved" with the matched saved Mii's own name. A new save posts nothing.

// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
use crate::model;

use model::notify::save_notice;

fn a_save_that_matches_a_saved_mii_names_it() {
    let body = save_notice(Some("格斗")).expect("a reuse posts a notice");
    assert_eq!(body.as_str(), "Same as 格斗, nothing new saved");
    let body = save_notice(Some("covrest")).unwrap();
    assert_eq!(body.as_str(), "Same as covrest, nothing new saved");
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("a_save_that_matches_a_saved_mii_names_it", a_save_that_matches_a_saved_mii_names_it as fn()),
    ]);
}
