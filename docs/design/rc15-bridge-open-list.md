# Bridge recording list depth

2026-11-24.

The Recording library's DRILLS sublist previously published the same
`tr.menu` list id and depth as its parent Combos list.

`Menu::depth` in `src/model/menu/mod.rs` counted only the menu's stack,
while the recording screens are stored separately in
`src/model/recording_menu.rs`. The bridge worker publishes that depth.
The recording list id used the section number, which DRILLS retains.

The depth now includes the recording face's file screen, numbered picker,
keyboard and DRILLS sublist above the face's own list. Each B backs out of
one of these levels. The page adds these levels to the menu stack; the
recording pop-up reports its own levels while it takes presses.

DRILLS uses `recording/<face>/<group>/1/drills`; other list ids and all
bridge key names remain unchanged. The Mash out and practice CPU editor
popups do not change depth.

Tests: `the_drills_sublist_is_its_own_list_one_level_down_on_both_faces`,
`every_b_that_goes_back_takes_one_off_depth_on_the_recording_lists`, and
`the_bridge_publishes_the_focused_list_and_still_omits_rows`.
