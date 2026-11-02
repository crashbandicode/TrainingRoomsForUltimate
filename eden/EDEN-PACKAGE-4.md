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

## S4-4: named macros, replay capture adapter, replay window (2026-11-02, Yoga, display :94, profile t4)

Builds: diag of stream/pkg4 at each run (s4-03 99d626f6, s4-04 68a54540, s4-05 ff63f59a (Yoga build), s4-07 e6595ff6 = work/implementation 8dfc4fc + pkg4).
Other plugins: CPU pilot v0.2.1 (loads first), tile cursor 0.1.0. Data folder seeded before boot (`tr_eden4.py --seed`): a real fork capture
(`replay_captures/7265…6e.smd`, 12,692 frames, from `runtime156/collected156`), the fork's `macros/` (4 macros + `selected.txt`), `replay_capture.ini` `enabled=on`.
Rows are driven by `eden/route4.py` then `eden/rows_s44.py` through the diag request file (`runtime/macros/diag.rs`); every wait gated on a diag line or a file.
Claims/releases in the Yoga ledger for every run; profile restored after each (`tr_eden4.py restore`). Muted (core.volume 0 set and saved before game.load).

| Row | s4-03 | s4-04 | s4-05 | s4-07 | Evidence |
| --- | --- | --- | --- | --- | --- |
| Install line: named_macros, live_replay installed, macros_port reason, rows guarded | pass | pass | pass | pass (line cut, see below) | events.log of each run |
| Live replay adapter: 18 seam rows guarded, hooks installed with capture on | pass | pass | pass | pass | `[ReplayCapture] guarded live adapter ready capture=on host_registered=false save=entry` |
| Numbered record chord: lockout, recording, crop | pass | pass | pass | pass | `record_lockout ... frames=60`, `recording_complete slot=0 frames=12/13` |
| Save slot to library: macros/<name>.json has the slot's frames, notice | pass | pass | pass | pass | `evtest.json` schema 1, frames = recorded, `notice_draw ... "Saved evtest"` |
| Named library lists saved + seeded macros | (froze before) | pass | pass | pass | `[NamedMacroDiag] view title="Named Library" ...` |
| Load from the library (selected.txt) | - | pass | fail (row parsing) | fail (grid: Down jumps a row) | s4-04 `selected.txt=evtest` |
| Replay Window lists the seeded capture and opens it (Fighter Mismatch prompt) | - | pass | pass | - | `capture id=7265… frames=12692`, view `Fighter Mismatch` |
| Keyboard applet (release path) opened by our code, answered through Eden's control API, result consumed | froze (diag opened the applet unanswered) | - | partial | partial | `[NativeKeyboard] begin/returned/consume/applied`; macro saved with the default name (the typed text did not reach the line edit: the UI job was not waited for) |
| Steady-state hook allocations delta 0 while the layer runs | pass | pass | pass | - | `hook_stats ... delta=0` through recording, listing, capture browser |
| console.log: panic/abort/crash/Unmapped | 0 | 0 | 0 | 11 (one early-boot fault, see below) | `tr_eden4.py scan` |

Findings: (1) `compat::menu::apply` was never called, so the typed menu (Macro Source) never left the factory value; fixed by `apply_snapshot` from
`published::publish` (s4-05 shows `named_source=true` after a publish). (2) s4-07: the combined tree's install line passed 8192 bytes and was cut before
`result=`; `install.rs` formats it on the heap (integrator-3 fixes the same in work/implementation). (3) s4-07: after loading a cross-fighter macro and a
settings publish from the diag thread, the game and all plugin threads stopped (fps 0, no modal, no fault); ui-screens reproduces the same signature on a
menu close (publish from the menu worker) and owns the root cause; s4-08 has a stall watchdog (`[NamedMacroDiag] stall ... step= publish=`). (4) s4-07 at
7.15 s: one stack overflow of a 64 KiB thread (SP just below a 64 KiB stack, jump to 0); handed to the stability audit.
Not run: a capture written after a real match (needs a VS match and the game's own replay save); the clip preview (package 3's preview restore is
not provided); the Deluxe Plugins page row (L1).

### s4-08 (2026-11-02 13:12Z-13:22Z, Yoga, solo, profile t4, display :94, muted): baseline before design C
Build stream/pkg4 4286dc8: diag 27719bbc (run), release 1d7247b5. Plugins: cpu_pilot e3033426, tile_cursor b8313ca2. Settings boot-s48 (Macro Source Named,
recording duration 60), seed seed-s4-03. memory.peak 6522310656 bytes; console scan 0 matches of 6082 lines. Rows 22/27 (eden/runs/s4-08/results.json):
- pass: install line (named_macros installed, macros_port boundary, result=installed heap_allocations=0, 12 rows guarded), live adapter ready (capture on),
  named record chord lockout, stop chord ends the capture (241 frames), library lists and loads, **named playback reaches input 0 at the restored pose with
  no refusal** (named_playback_start x=45 y=27.09 facing=-1, then named_playback_complete; sync-01 refused every one), Copy to Record Slot 1, Save slot to
  library (evtest = evtest2's 184 frames), Replay Window lists the seeded 12,692-frame capture, opens it in the clip editor, crops a macro into the library,
  Fighter Mismatch prompt.
- FAIL, row expectation: "capture saved with the captured frames" saved 184 of 241: Crop is on (factory), named_recording_complete crop=true; the crop
  trims the idle lead-in. The row now accepts 0 < saved <= stop when crop=true.
- FAIL, harness: keyboard applet row: the applet ran on the real path (NativeKeyboard begin, returned 0x0, consume, applied, "Saved Recording"), but Eden's
  ui.set of line_edit_osk to "kbtest" did not reach the applet's result (it returned the default "Recording"; recording-2.json written). Also: the watchdog
  printed "stall frames=6455 step=28" while the applet was open: service_keyboard (step 28) held the game thread during the applet (design C moves it to the worker).
- FAIL, cross-package finding: "Switch to recorded fighter": the roster provider applied Duck Hunt (MiiOrdinaryCpu ready/completion, "CPU fighter applied"),
  but afterwards the layer's per-CPU-body work stopped: named_service fell from ~1800 to ~1200 calls per 10 s (only the two panel holds), the diag
  commands (run inside that block) stopped answering, and the watchdog printed "stall frames=7330 step=0" (the game itself kept running).
  recorder::is_primary_cpu_body compares the running fighter's accessor with the CPU entry's **first body** (game::natives::entry_body,
  ENTRY_BODY_FIELD); after package 6's body-bank swap the entry holds the group's bodies (GroupSwap bodies=3) and the first body is no longer the active
  fighter. compat::common::try_get_module_accessor and recorder::cpu_body use the same field, so the recorder's own tint/natives would address the wrong
  body after a swap too. Owner question for the lead (package 2's natives vs package 6's body bank).
- FAIL, R5: every hook_stats line after the first row: steady deltas ~100-130 per 10 s, attributed: roster_preload (package 6, ~10/s, 4396 by the end),
  and in this layer live_replay 1 (match load), recorder_poll 10 (finish_named_capture -> named_macros::finish_capture: to_vec, version String, offer_save,
  on the stop chord), named_service 2 (match start) + 81 (the keyboard applet service). The ones in this layer are the hook-side model calls design C
  moves to the worker; input_caller_a_note 27 / input_caller_a_apply 1 / cpu_frame 192 are at the match load (other packages).
## Loading cancellation / held-close regression (pending integrated runtime)

On the final composed candidate, retain a crop and request a track whose exact Mii metadata is still loading. Press physical B while EDITOR is parked in RELOAD: the pending selection must cancel and the previous crop must return unchanged. Repeat cancellation during opening/fighter reload: the editor closes. Other physical controls during loading do not mutate the crop or start preview. Retain the loading view/receipt and resulting editor/crop, not only a no-crash observation.

Close with physical B held past 15 mapping polls, then release and press again. The held closing press must never reach gameplay or a newly opened editor; the later fresh press may. Repeat after scene departure to show close ownership reset. A failed native preview cleanup remains stopped, but after closing its panel the native pause/exit route must be available. All final hook allocation/dropped-event/Unmapped gates still apply. These rows are OPEN; source routing and host policy tests do not establish native acceptance.
