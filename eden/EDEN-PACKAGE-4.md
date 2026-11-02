# Eden evidence, package 4 (recording, chords, notifications, input display data path)

Dated 2026-11-02 (UTC, host clock of the Yoga). Tester: Claude Sonnet 5.5 (high), stream `pkg4` under lead `0caa72b8`. Worktree
`/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-pkg4`, branch `stream/pkg4`. Playbook `009c9ac`. Host: the Yoga, display `:94`,
work root `/home/intpa/trainingrooms-eden-p4`, own profile `profiles/t4` (copy of `ssbu-desync-1029/profiles/b`, game content hard-linked; never `a`).
Eden `current` = the same AppImage package 1 used (sha256 `d483c082…`). Muted: every boot ran `settings.set core.volume 0`, `settings.save`, `settings.get`
before `game.load`. Claims: s4-01 CLAIMED 05:26Z / RELEASED 05:33Z, s4-02 CLAIMED ~05:34Z / RELEASED 05:37Z in the Yoga ledger, one Eden at a time,
MemAvailable above 10 GiB, `df` 282 GB free. Both `Down`s printed `alive: false`, `left: []`; the profile was restored (`plugins/` holds arcropolis and
the desync detector again, the TrainingRooms / tile-cursor / pilot output folders removed). `console.log` of each run: `scan` = 0 lines matching
panic/abort/crash/Unmapped (5,617 lines each; `runs/<label>/console.log` on the Yoga). `memory.peak` kept (6.05 GB and 6.12 GB), `run.json` kept.

## Builds under test

| Run | NRO | sha256 | Settings | Other plugins |
| --- | --- | --- | --- | --- |
| s4-01 (boot A) | diag, Surface build of commit 5a5d42e | `fc36ea010c09984a4c48636e6800e997126a6b13a4e75a9d6f8cdd5d9ed4029c` | defaults, Recording Duration 60 | CPU pilot v0.2.1 `libssbu_cpu_pilot.nro` (loads before ours), tile cursor 0.1.0 |
| s4-02 (boot B) | diag, Yoga build of the commit with the writer thread | `233ea667a1a48b6f84fede07174389c0e28d90a49a28413b8dd15188cd52ac38` | Playback Loop On, Save State Playback Slot 1, Duration 60 | the pilot installed as `libzz_cpu_pilot.nro` (loads **after** ours), tile cursor |

Install line of both boots (`runs/<label>/events.log`): 12 package-4 rows `stock`, written `near` for `ai_epilogue`, `combo_display_a`, `combo_display_b`,
`stale_moves_toggle`, `checked` for the five natives and the two Combo Display pins; `chords=installed recording=installed notices=installed`;
`result=installed heap_allocations=0`. The CPU pilot (hooking the AI function's entry `0x2da180`) and our epilogue hook coexist in both load orders.

Route (gated, `eden/route4.py`): title, A until the main menu matches, the existing stage-select macro, tile cursor to Pokemon Stadium 2 (tile 41), CSS, Training;
every gate passed (`training scene` distance 24.3 / 15.6 against threshold 30). Rows: `eden/rows4.py` (marks by the diag lines' millisecond stamps, both diag
files read because the ring rotates at 128 KiB). The diag build writes one line per event; the shipped build writes none (`strings` of the release NRO: 0
`PracticeRecording` / `standin` matches).

## Rows (every one gated on a diag line; outputs in `eden/runs/s4-0N/`)

| Row | Result | Evidence |
| --- | --- | --- |
| Install line, hooks, `heap_allocations=0` | **pass** | both events.log |
| Playback chord raw-controller detection (P1 poll, before the delay) | **pass** | `chords_passed bits=0x8` |
| Empty slot refused with a notice (Q11, B8) | **pass** | `notice_draw … body="Record into this slot first"` |
| Record chord: lockout (30 frames), standby notice once, P1 acts, recording ends, crop | **pass** | `record_lockout cpu_kind=59 frames=60`, `notice_draw … "Standby"`, `recording_complete slot=0 frames=12` (60-frame duration cropped to 12) |
| Playback of the recorded slot plays every stored frame | **pass** | `playback_shortcut slot=0 frames=12`, `playback_complete slot=0 restarted=0` |
| Phase tint and notices (lockout blue, standby magenta, recording red) | **partial** | the notices are logged; the tint call (`ModelModule::set_color_rgb`) ran without a fault (screenshots `runs/s4-01/shots/*lockout*`, `*standby*`, `*recording*` were taken; the colour was **not** checked pixel-wise) |
| Notice hidden while the menu stand-in owns the input, shown again after, countdown only while visible (B8) | **pass** | `notice_visible=false menu_open=true` then `notice_visible=true menu_open=false` (the first pass of the row missed: the refusal notice had expired; the lockout notice was used) |
| N7: X+Y opens nothing before the edit; the edit (stand-in) writes `shortcut_2=X + Y` into `button_config.ini` and the new chord opens the menu stand-in at once | **pass** | s4-01: `standin shortcut_edit … Ok(())`, then `standin menu_owner open=true` on X+Y; file read back: `shortcut_1=Minus`, `shortcut_2=X + Y`, the comment line kept |
| N7 through the writer thread, no file I/O in hook scope (lead ruling) | **pass** | s4-02: `button_config.ini written by the writer thread`; `input_hook_counts … heap_allocations=0` in all 4 reports across the edit (`runs/s4-02/hook-counts.txt`). In s4-01, where the stand-in wrote the file inside the hook, the counter rose 0 → 18 exactly at the edit and stayed flat after: the detector works and the cause was the file I/O in hook scope, now removed |
| Input display: logger runs, plans produced for new rows (panes wait for package 5) | **pass (data path)** | `input_display_plan ops=16 rows=5 icons=1` after button presses; no pane writes exist yet |
| Hook invocation counts and allocation counter after the hooks ran (R5) | **pass** | `input_hook_counts p1_poll=600 ai_frame=600 inject=288 notice_tick=600 heap_allocations=0` per 600 frames; one P1 poll and one AI frame per game frame |
| Playback Loop On restarts the playback | **pass** | s4-02: 18 `playback_complete … restarted=1` lines in about 3 s |
| L+R+A reset stops the recorder (decision B8, SPEC-input-01 question 13) | **pass** | `hub_run cause=TrainingReset` at 495853357; 0 `playback_complete` / `playback_shortcut` lines in the next 2 s (also in the first pass: no completion between `hub_run` 495783507 and the next start 495786487) |
| Save-state playback slot plays on a restore (**stand-in**: the save-state chord calls `on_state_restored`) | **pass (stand-in)** | `playback_shortcut slot=0 frames=12` |
| Coexistence with the CPU pilot in both load orders | **pass** | s4-01 (pilot first), s4-02 (pilot second): install line, all rows, zero console faults |

## Not run, and why (not implied by the table)

- **Mash-triggered playback, hitstun timing modes, Playback Mash Interrupt**: need package 2's per-frame mash trigger evaluation, which is not built (its
  progress note). The recorder side is host-tested (`hitstun_timing_modes`, `mash_interrupt_stops`).
- **Real save-state playback / record-on-load**: package 3's restore completion is not on this branch; only the stand-in ran.
- **B4 (record refused while an AI drives P1)**: needs the pilot or the fallback to drive P1 through its menu; not driven. The check is `natives::operated_by_cpu(0)`
  (the game's own flag, the same call the reset hub uses); host-tested only as the `RecordRefusal` type.
- **Mirroring against the CPU's facing, Shulk's dial, Ice Climbers CPU, Pokémon Trainer switch, hot swap, lobby practice**: not driven (host tests for the model rules).
- **Pane writes** of the input display, the notice slot, the Combo Display consumer and the damage helper: package 5's layout.
- **Crop/loop/hitstun with the real mash**, the **desync / online rows** (hooks only act in Training; the online review is in `docs/test-provenance/package-4.md`).
- Screenshots of the recording tints were taken but not measured.
