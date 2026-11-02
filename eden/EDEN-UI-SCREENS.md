# Eden evidence, ui-screens stream (menu screens on packages 4 and 6)

Dated 2026-11-02. Stream `ui-screens` (lead 0caa72b8). Screenshots and logs: `eden/runs/<label>/` (gitignored); the shots of the
rows are listed per run.

## ui-01 .. ui-05: Ally, rig `C:\Eden\trainingrooms-pkg5` (before the restart rules; that rig had no firewall rule)
Build: diag of `a67c349` + the install-line fix (`81fcfdce…`), CPU pilot, tile cursor, nro_hook, param_hook. Route: Final Destination,
Steve vs the CPU. Rows that ran (ui-05, shots in `eden/runs/ui-05/shots/`):

| Row | Result | Evidence |
| --- | --- | --- |
| Named library opens from its Recording row; empty library | pass | `named-library-open.png`: crumb `Recording > Named Library`, the layer's footer on the two help lines |
| Save slot to library with an empty slot | pass (refusal) | `save-slot-refused.png`: help line "Record into the slot first" |
| Save slot to library after a recording (7 frames) | pass | `save-slot-panel.png`: `Recording > Name macro`, "Name and save", footer `Recording · 7f · 0.12s` |
| The confirmed name is written by the worker (diag `name evtest`) | pass | `macros/evtest.json` in the plugin folder; the panel closed back to its row (`after-name.png`) |
| Named library lists the saved macro | pass | `library-with-macro.png`: `evtest · Dark Samus` |
| Edit placement without a restore | pass (refusal) | `edit-placement.png`: "Load a macro and restore a Training state first" |
| Replay window | not run | the game froze before the row (below) |

`console.log`: 10 `Unmapped` lines at 5 s at `0x109f5f3b7x` (outside the plugin's window, above 0x1000): attributed stock emulator noise.
The freezes of ui-02/ui-04/ui-05 were first read as the firewall prompt of that rig; ui-06/ui-07 show they are the menu-close hang below.

## ui-06, ui-07: Ally, LENT rig `C:\Eden\c1162-w5-ally` (restart rules; `eden/lent/stage_ui.ps1` set-aside, verified restore)
- ui-06, diag of `97e8eb4` (`8f42d9ce…`): install line whole (8,979 characters, ends `result=installed heap_allocations=0`); Named
  library opened and closed (`rec-named-library-empty.png`, `rec-page-after-library.png`); then **B on the page (menu close) froze the
  game**: `game_fps 0`, last frame age growing, no `[Ui] menu closed` line, the diag counters stop.
- ui-07, integrator-2's own diag NRO of the integration head (`cargo-targets/integ2-nro/libtraining_rooms-diag.nro`, `8993aaed…`): open
  (B + Up) and close (B) at once: the same freeze. The hang is in the integrated tree, not in this stream's changes.
- Restore after ui-07: `mismatches: []`, kept trees unchanged, CLAIM back (`restore-verify-*.json` on the rig).
- Same signature as pkg4's s4-07 (a settings publish from a non-game thread, then the game and the plugin's threads stop).

Next: the probe build (`[Ui] probe close N`, `[Ui] probe publish N`, diag only) names the step the close stops at.
