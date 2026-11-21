# Frame strip layout

`src/model/ui/strip_layout` holds the shipped layout. The `C` spec composes the
strip and card geometry. It also owns the waiting-room drop, the space reserved
for Link history and the condition and opacity for fading the whole panel.

Adjust the strip and card positions, widths, row spacing and heights in this
module. `frame_strip_plan` and `combo_plan` supply the text, visibility, colours
and state for each pane. Keep move names, verdicts, grades and sequence state
in those plans.

The same layout module supplies the pane recipe builder and the live HUD. After
changing the geometry, rebuild both the Training and waiting-room recipes with
the commands in [Building](../BUILD.md). Test the geometry with both history
settings and both card modes. In Eden, check the card against the native damage
and Waiting panels and move a fighter behind the panel to check that the
backdrops, text and marks fade together.

## Requirement trace

| Operator clause | Design | Test |
| --- | --- | --- |
| “I would go with C with link history on/off switch” | Compact readouts, a corner card with three stacked sequence chips, and a separate link-cell band | `a_drilled_sequence_draws_the_corner_card_and_helper_cells`, `the_current_combo_is_always_in_a_window_of_three` |
| Link history defaults On and Off folds the latest follow-up into LINK | Persist the Display & Controls choice and reclaim the LAST row when disabled | `legacy_settings_default_link_history_to_on`, `link_history_choices_persist`, `link_history_hides_the_last_row_and_folds_the_newest_moves` |
| “Its layout should be abstracted away from its functionality” | One spec for the archived panes and live placement. The integration rows check native clearance and inherited alpha. | `runtime_and_archive_share_the_strip_layout`, `the_same_content_can_be_placed_in_a_second_layout`, `compact_bounds_and_fade_follow_the_scene_and_visible_rows` |
