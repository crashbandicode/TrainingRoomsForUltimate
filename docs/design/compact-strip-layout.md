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

## Text length (2026-11-24)

A line of the strip or the card holds 40 UTF-16 units, in the plans' `Text` and
in every native text box (`panes::iv::TEXT_UNITS`); both were 24, and the card
cut its reason there. The longest line is a reason of 37 units.

A reason wider than the card's content width wraps to a second line
(`Text::fitted`, the notice box's wrap). The card grows by one line pitch
(`CardSpec::grown`): the reason's first line stays on its row, a sequence's
statistics move down, and the fade uses the grown plate. Not yet seen in a game.

Lines whose place cannot grow keep 24 units (`SHORT_UNITS`): a name on the card
(the title, a sequence's name, chips, `landed` and `dropped`), Parry's pills and
the LAST row's moves. The mark under a step shares its row with a hit combo's
verdict and can run under it.

## Requirement trace

| Operator clause | Design | Test |
| --- | --- | --- |
| “I would go with C with link history on/off switch” | Compact readouts, a corner card with three stacked sequence chips, and a separate link-cell band | `a_drilled_sequence_draws_the_corner_card_and_helper_cells`, `the_current_combo_is_always_in_a_window_of_three` |
| Link history defaults On and Off folds the latest follow-up into LINK | Persist the Display & Controls choice and reclaim the LAST row when disabled | `legacy_settings_default_link_history_to_on`, `link_history_choices_persist`, `link_history_hides_the_last_row_and_folds_the_newest_moves` |
| “Its layout should be abstracted away from its functionality” | One spec for the archived panes and live placement. The integration rows check native clearance and inherited alpha. | `runtime_and_archive_share_the_strip_layout`, `the_same_content_can_be_placed_in_a_second_layout`, `compact_bounds_and_fade_follow_the_scene_and_visible_rows` |
