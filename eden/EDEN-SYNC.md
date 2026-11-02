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
## sync-02: slice 2 (`909168e`, diag `c27dc0d7`)
Run 12:48Z–13:00Z, label `live-sync-02` (`eden/runs/sync-02/` on the ui-screens branch's tree: console.log, run.json, diagnostics log,
save_states.json, rows*.json, fps*.json). Same plugin set and route as sync-01; drivers `rows_sync01.py`, `rows_sync01b.py`, `rows_sync02c.py`.

| Row | Result | Evidence |
| --- | --- | --- |
| Arbiter self-test | pass | `mismatch=0xfa01 timeout=0xea01 signal=0x0 ok=true` |
| Menu open/close cycles | pass | **45 closes** (20 + 10 numbered source, 15 named source), each gated on its `[Ui] menu closed` line |
| Save slot to library, load from the library | pass | `macros/synctest.json`; `notice_draw ... body="Loaded synctest"` |
| Publish from the diag reload thread, Macro Source = Named | pass | `diag_settings_reload rerolls=0x0 issues=0`, `status named_source=true` (twice) |
| Named playback | pass | `named_playback frames=6` ... `named_playback_complete` (twice) |
| Save State through the writer's mailbox (decision (a)) | pass | 1 + 12 Saves in a row: `file_written` 13/13, `save_states.json` mtime advanced; the file holds the captured slot (`fighter_kind=88`, `marker=NoAction`) |
| Load State after the saves | pass | `state cleanup entry=0` (the load ran from the state machine's own file) |
| Hook allocations | pass | `hook_stats ... delta=0` throughout; 0 added by the 45 closes, the 13 saves and the load (36 before and after part c). Per-load 26 + 5; +1 at the numbered recording, +2 per named playback (package 4's layer, forwarded by the lead) |
| Frame rate | pass | 48 samples right after closes and publishes: `game_fps` 59.93–60.06, mean 60.00 |
| console.log | pass | the same boot-time `tr-trace-writer` fault only (10 `Unmapped` lines and one `Cannot execute ... 0x0` at 10.49 s, above 0x1000, fixed in stream/stack); nothing later; no panic/abort/crash |
| Rig | pass | restore 12:59:56Z `mismatches: []`, CLAIM back; ledger CLAIMED 12:48Z / RELEASED 13:00Z; memory peak 5.34 GB; no firewall rule added or changed |
