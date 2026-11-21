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
