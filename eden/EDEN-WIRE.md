# Eden evidence, stream `wire` (cross-stream wiring of packages 2-5)

Dated 2026-11-02 (UTC, host clock of the Yoga). Tester: Claude Sonnet 5.5 (high), stream `wire` under lead `0caa72b8`. Worktree
`/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-wire`, branch `stream/wire`. Playbook `ae87443`. Host: the Yoga, display `:92`, work root
`/home/intpa/trainingrooms-eden-wire`, own profile `w1` (copy of `ssbu-desync-1029/profiles/b`, game content hard-linked; never `a`). Muted: every boot ran
`settings.set core.volume 0`, `settings.save`, `settings.get` (value 0) before `game.load`. Claims in the Yoga ledger (owner `Claude 0caa72b8/wire-eden`): w-01 07:24Z-07:28Z,
w-02 07:5xZ-07:56Z; one Eden at a time; the profile was restored after each run (`tr_eden_w.py restore`: arcropolis and the desync detector back, this stream's output
folders removed). `console.log` of both runs: 0 lines matching panic/abort/crash/Unmapped (5,635 lines each). `memory.peak` 6.21 GB (w-01), 6.18 GB (w-02); `run.json` kept.
Outputs under `eden/runs/<label>/` (gitignored): `rows.txt`, `diag.log`, `events.log`, `console.log`, screenshots, `button_config.ini`, `replay_capture.ini`, `hook-counts.txt`.

## Builds under test

| Run | NRO | sha256 | Source |
| --- | --- | --- | --- |
| w-01 | diag, Surface build | `72cf357068f64d23285ae6337fcb651bbf83606c9c1ed25eda253442a6eba9b8` | `3a682e9` |
| w-02 | diag, Surface build | `2a86a6804e8b816812bed8e5f138643fe096a9d6254e86f9a7e732cc146d4640` | `475deb4` (the merge of `work/implementation` `915fda1`, r1fix) |

Other plugins: the CPU pilot `libssbu_cpu_pilot.nro` (`e3033426...`, loads before ours) and the tile cursor `b8313ca2...`. Route (`eden/route_w.py`): title, main menu,
Pokemon Stadium 2, Steve, Training; every gate passed. Rows: `eden/rows_w.py`; every wait is gated on a diag line.

## w-01 (3a682e9): 18 of 19 rows pass

The one failure, "input display plan applied to the panes", was a real defect: the input log's row panes were not found (`input_display pane missing: row; display off for
this session`) because `pane::find` from the layout root walks the stock panes first and stops at its 4,096-node cap (`resolved total=4096`). Fixed in `2f4dee5`: the
notice and log containers are resolved once per layout registration and every other pane is searched below its container. The notice slot drew in w-01 already
(screenshot: "Frame Advantage" / "-10" in red).

## w-02 (merged tree): all 19 rows pass

| Row | Result | Evidence |
| --- | --- | --- |
| install line (recorder, chords, notices), `heap_allocations=0` | pass | `events.log` |
| HUD draw registered and running on `info_training` | pass | `hud_draw layout registrations=1` |
| input log on screen | pass | `input_display_plan ops=1 rows=1 icons=0` and screenshot `075448-input-log.png`: frame counters (59, 6, 34, 7, 31, 5, 99) and the A / X / B glyphs down the left side |
| notices on screen | pass | `notice_draw header="Input Recording" body="Record into this slot first"`; the Frame Advantage notice ("-4", red) in the screenshot |
| Save chord ZL + D-pad Down reaches package 3 | pass | `state saved_notice slot=1` |
| Load chord ZL + D-pad Up loads | pass | `state load_request entry=0`, `state restored ledge=false`; P1 x -45.0 saved, 2.24 after moving, -45.0 after the load |
| recording of a slot | pass | `recording_complete slot=0 frames=13` |
| a looping playback stops on a load request (`recorder::stop` from package 3) | pass | `recorder_stop mode=3`, no `playback_complete` 0.5 s later |
| the restore reaches the recorder (`recorder::on_state_restored`) | pass | `state_restored ledge=0 action=0` |
| Recording Trigger = Save State Load: the restore starts a recording | pass | `state_restored ledge=0 action=1`, `record_lockout cpu_kind=6 frames=60` |
| frame advantage shown | pass | `notice_draw header="Frame Advantage" body="-16" color=[200, 8, 8, 255]` |
| Playback Mash Interrupt stops a playback | pass | `cpu_event playback_stop entry=1` then `playback_interrupt mash=1` |
| shortcut edit through the writer thread | pass | `button_config.ini written by the writer thread`; file: `shortcut_1=Minus`, `shortcut_2=X + Y`, comment kept |
| replay-capture toggle through the writer thread | pass | `replay_capture.ini written by the writer thread enabled=true`; file `[capture]` `enabled=on` |
| R5: hook-scope allocations across both writes | pass | `input_hook_counts ... heap_allocations=0` in all 7 reports (`hook-counts.txt`) |
| the real menu: open (B + D-pad Up), change a row, close | pass | `[Ui] menu opened`, `menu closed changed=true`, `settings.json written by the writer thread` |

## w-03 (`5676b32`, the R2 fixes; boot with `button_config.ini` = `shortcut_1=Plus`, `shortcut_2=disabled` placed before the first open)

Diag NRO `1d5d0323b1a18ca8c8d02015eb58aa9e1af60be986348b91fc3e5ccaa5b92b9b`; `console.log` 5,644 lines, 0 matching panic/abort/crash/Unmapped; `memory.peak` 6.07 GB. All 29 rows pass
(`eden/runs/w-03/rows.txt`): the 19 rows of w-02 and these:

| Row | Evidence |
| --- | --- |
| R2-6 the imported Plus shortcut opens the real menu before any edit (and the game's pause does not open: screenshot `080228-menu-plus-shortcut.png`) | `[Ui] menu opened` |
| R2-6 the menu closes; the default Minus no longer opens it; the permanent B + D-pad Up still opens it | `menu closed`; no `menu opened` line for Minus; `menu opened` for B + Up |
| R2-6 a shortcut edit is persisted by the writer thread and applies at once | `button_config.ini written by the writer thread`, file `shortcut_1=Plus`, `shortcut_2=X + Y` (comment kept); X + Y then opens the menu |
| R2-4 the CPU runtime sees one loading fact | `cpu_event save_loading=true` ... `save_loading=false` around `state load_request` / `state restored` |
| R2-4 the record chord pressed with the load is ignored while it loads | `chord_ignored_loading record=1 playback=0`, 0 `record_lockout` lines |
| R2-5 with Playback Mash Interrupt Off the playback continues under the same mash | 17 `playback_complete`, no `playback_interrupt` |

Not run: the menu's own shortcut editor (the confirmed edit is driven through `diag-shortcut.req`, the call the menu's close makes), a second boot reading the edited file (the first boot's read of the same
file and parser is the import row).
## Not run

- The shortcut and replay-capture rows use the diag request files (`diag-shortcut.req`, `diag-capture.req`), which call the same `chords::commit_shortcut` and
  `writer::request_replay_capture` the menu's close calls; the menu path itself was driven only for a settings change.
- A refused shortcut edit and a failed write (the notice) are not driven in Eden.
- Lobby practice (no pkg6 engine in this tree), the ledge-playback branch of `on_state_restored`, B4 (P1 driven by an AI).