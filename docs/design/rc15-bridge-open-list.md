# Bridge recording list depth

The 2026-11-24 fix gave the Recording library’s DRILLS sublist its own `tr.menu` identity and depth.

## Original defect

| Component | Before the fix |
| --- | --- |
| `Menu::depth` | Counted the menu stack alone. Recording screens lived outside that stack. |
| Bridge worker | Published that depth for both Combos and DRILLS. |
| Recording list ID | Used the section number, which DRILLS shared with its parent. |

## Shipped bridge snapshot

| Focus | Depth and list |
| --- | --- |
| Recording page | Add recording depth to the menu stack. |
| Recording pop-up taking input | Report the pop-up’s own recording depth. |
| DRILLS or Recordings sublist | Add one level for the sublist. |
| File, numbered picker, keyboard | Each open screen adds one level on its face. |
| B returning one screen | Subtract one level. |
| File or picker with selectable rows | Publish `recording/file` or `recording/picker`. |
| Empty list, loading overlay or keyboard | Publish no focused list on either face. |
| DRILLS | Append `/drills` to `recording/<face>/<group>/1`. |
| Recordings sublist | Append `/recordings` to its section ID. |
| Mash out and practice CPU editor pop-ups | The recording depth change did not affect them. |

The fix retained the bridge key names.

## Regression coverage

| What the test checks | Test |
| --- | --- |
| DRILLS on the page and pop-up | `the_drills_sublist_is_its_own_list_one_level_down_on_both_faces` |
| B and depth | `every_b_that_goes_back_takes_one_off_depth_on_the_recording_lists` |
| Focused list in the published block | `the_bridge_publishes_the_focused_list_and_still_omits_rows` |
| A recording screen without a row | `a_recording_screen_with_no_row_publishes_no_list_not_the_page_under_it` |
