//! Quickplay waiting shares the local-wireless backend shape and pure CPU policy; its
//! mode word, match excursion, portrait admission and menu-catalog ownership stay
//! distinct. Host value checks only: no native adapters, files or threads.


use crate::engine::mii_root::mii_hud_portrait_policy as portrait_policy;




fn portrait_stores_admit_quickplay_waiting_and_refuse_its_match_and_arenas() {
    assert!(portrait_policy::may_write_scene(0x0d, 0x07010101));
    assert!(portrait_policy::may_write_scene(0x0d, 0x02010000));
    assert!(portrait_policy::may_write_scene(0x20, 0x04040000));
    assert!(!portrait_policy::may_write_scene(0x0e, 0x07010101));
    assert!(!portrait_policy::may_write_scene(0x0d, 0x07010300));
    assert!(portrait_policy::may_write_scene(0x0d, 0x07010400));
    assert!(!portrait_policy::may_write_scene(0x10, 0x07010400));
    assert!(!portrait_policy::may_write_scene(0x0d, 0x07010401));
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("portrait_stores_admit_quickplay_waiting_and_refuse_its_match_and_arenas", portrait_stores_admit_quickplay_waiting_and_refuse_its_match_and_arenas as fn()),
    ]);
}
