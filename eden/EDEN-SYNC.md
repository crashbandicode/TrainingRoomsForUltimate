# Eden evidence, stream/sync (the menu-close hang's root fix)

Dated 2026-11-02. Ally, LENT rig `C:\Eden\c1162-w5-ally` (set aside and restored with `eden/lent/stage_ui.ps1` on the ui-screens branch; driver
`eden/lent/rows_sync01.py` there). Outputs under `eden/runs/<label>/` (gitignored).

## Before (ui-09, ui-screens diag `9af41937`, 27d84f4 + probes)
Probes written straight to the kernel debug output: `input push B`, `close 10/11/12/1`, `publish 1..5`, then nothing; the guest froze
(`game_fps 0`). The worker never returned from `chords::publish`'s `STATE.lock()` (parking_lot, generic spin parker on os=switch).

## sync-01: slice 1 (`15a558c`, diag `aecf6047`)
Run 12:01Z–12:15Z, label `live-sync-01` (`eden/runs/sync-01/`: console.log, run.json, both diagnostics logs, rows*.json, fps*.json). Plugin set:
`libtraining_rooms.nro` aecf6047 (diag), nro_hook, param_hook, cpu pilot e3033426, tile cursor b8313ca2. Route: Final Destination,
Steve vs the CPU (tile cursor, screenshot gates).

| Row | Result | Evidence |
| --- | --- | --- |
| Arbiter self-test at boot | pass | `[Sync] arbiter self_test mismatch=0xfa01 timeout=0xea01 signal=0x0 ok=true` (the kernel's InvalidState / TimedOut / success, as the Eden kernel source says) |
| Menu open/close cycles, Macro Source numbered | pass | 20 + 10 cycles, each gated on `[Ui] menu opened` and `[Ui] menu closed` (every close publishes) |
| Numbered recording, Save slot to library | pass | `recording_complete slot=0 frames=8`; `macros/synctest.json` written |
| Load a named macro from the library | pass | `pad confirm`, then `notice_draw ... body="Loaded synctest"` (the row script's view parser failed on a line cut at 200 characters; the notice and `selected.txt=synctest` are the evidence) |
| Publish from the diag reload thread, Macro Source = Named Library (pkg4's s4-07 path) | pass | `diag_settings_reload rerolls=0x0 issues=0`, then `status named_source=true`; done twice |
| Named playback of the loaded macro | pass | `named_playback frames=8`, `named_playback_complete`, twice |
| Menu open/close cycles, Macro Source named (buffers reserved) | pass | 8 + 8 cycles (cycle 8 of the first set closed at #1022 just before the diagnostics log rotated; the script's counter reset with the rotation) |
| Total | pass | **46 menu closes, no hang** (38 in `practice-diagnostics.previous.log`, 8 in the current one) |
| Frame rate around the closes | pass | 40 samples of Eden `status` right after each close: `game_fps` 59.95–60.03 (mean 60.00). No per-hook timing exists in this build: frame rate is the evidence that the game thread kept its frames |
| Hook allocations | pass for the menu | `hook_stats ... delta=0` on every line during the cycles. Per-load: 26 + 5 at the match load. +1 around the numbered recording/save (#420–#453), +2 around each named playback (#848, #947): package 4's layer, not the menu path (attribution for pkg4) |
| console.log | pass | 10 `Unmapped` lines and one `Cannot execute ... 0x0` at 16.38 s, at boot before the first plugin menu activity, above 0x1000 (0x10f532ab..): the `tr-trace-writer` stack fault the stack stream found and fixed (not in this 27d84f4 base). Nothing near any close. No panic/abort/crash |
| Rig | pass | restore 12:15:27Z `mismatches: []`, kept trees unchanged, CLAIM back; ledger CLAIMED 12:01Z / RELEASED 12:15Z; run.json memory peak 5.37 GB |