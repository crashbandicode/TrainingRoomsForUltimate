# Replay editor: what came from the fork (2026-11-24)

- The crop and preview editor of install 4 (`feat/replay-editor`, base `b5177cdb`) copies no fork code.
- The fork-own hunks it uses were already in this repository through the package-4 carry of 2026-11-02 (`tools/carry_pkg4.py`, stage m5q, ledger `pkg4-carry-ledger.md`).
- This table lists them by the fork commit that added them.
- Fork worktree: `UltimateTrainingModpack-lobby-parity`; line ranges are the added lines at that commit.
- "Verbatim" counts the added lines of more than 14 characters found unchanged in this repository at `b5177cdb`.

| Fork commit | Fork file | Added lines | Here | Verbatim |
| --- | --- | --- | --- | --- |
| `eb4eec2` (accelerating scrub) | `training_mod_tui/src/clip_panel.rs` | +16-124, +288, +477-537 | `src/model/macro_ui/clip_panel.rs` | 84 of 111 |
| `eb4eec2` | `src/replay_capture.rs` | +15, +361-364, +386-395, +399-407, +419-420, +423-427 | `src/runtime/macros/replay_capture.rs` | 18 of 19 |
| `eb4eec2` | `src/training/ui/clip_editor.rs` | +12, +474-484, +486-496, +498-499 | not carried | 3 of 20 |
| `15ada37` (snap to move starts) | `training_mod_macros/src/move_start.rs` | +1-256 | `src/model/macros/move_start.rs` | 145 of 185 |
| `15ada37` | `src/replay_move_start.rs` | +1-45 | `src/runtime/macros/replay_move_start.rs` | 40 of 41 |
| `15ada37` | `training_mod_macros/src/replay.rs` | +3, +65, +67-85, +89, +92-93 | `src/model/macros/replay.rs` | 18 of 19 |
| `15ada37` | `training_mod_tui/src/clip_panel.rs` | 28 hunks from +2 to +704 | `src/model/macro_ui/clip_panel.rs` | 67 of 79 |
| `15ada37` | `training_mod_macros/tests/move_crop_continuation.rs` | +1-63 | the carried unit test of `clip_panel.rs` | 2 of 52 |
| `6f250bb` (preview without a manual save) | `src/training/save_states.rs` | 17 hunks from +2 to +1177 | not carried | 1 of 88 |
| `6f250bb` | `src/replay_preview.rs` | +31-32, +56, +59, +155, +165, +188, +208, +211-214 | not carried | 0 of 11 |
| `6f250bb` | `training_mod_tui/src/clip_panel.rs` | +119 | `src/model/macro_ui/clip_panel.rs` | 1 of 1 |

- **`6f250bb`:** its preview is not here. Previews play through Recording's player and keep the pair in a game-side
  checkpoint (fix F7, `6bc005f6`), written in this repository.
- **The fork's `src/training/ui/clip_editor.rs`** (the fork's `4c0d785` and `6a77411`) draws through the Training
  Modpack's menu panes and was never carried. The strip over the game, the timeline and the play target are written
  here from the fork's documents and clips (`docs/named-macros.md`, `docs/named-macros/images/`).
- **Upstream Training Modpack** `7c522c0` (#587) and `1c00d2a` (#607) are not used.
- **New in this branch, written here:** `src/model/macro_ui/clip_marks.rs` (move marks, the watch, snapping), the strip
  geometry, the play-target swap and the `actions` file key. The edits to carried files are recorded in the package-4
  manifest.

Change a carried file by adding manifest edits: `python3 -I tools/carry_edits.py <dest> <base> --reason <reason>`
writes them from the file as it was and the file as wanted. Then run `carry_pkg4.py carry`, `ledger` and `verify`.

The counts come from a scan of `git show --unified=0 <commit> -- '*.rs'` against `src/**/*.rs`; the scan and its output
are kept with the lane's artifacts (`artifacts/phase23/replay-editor/prov.py`, `provenance-scan.txt`).
