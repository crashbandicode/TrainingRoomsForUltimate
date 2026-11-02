# Eden evidence, package 1 (TrainingRoomsForUltimate)

Dated 2026-11-02 (UTC, host clocks of the Yoga and the Surface). Tester: Claude Sonnet 5.5 (high), spawned by lead
`0caa72b8` (Opus 5.5 high). Worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-work`,
branch `work/implementation`, HEAD `81cea59`. No plugin code was changed; nothing committed or pushed.

## Builds under test (hashes verified on the Surface and again on the Yoga after `scp`)

| Build | File | sha256 |
| --- | --- | --- |
| release | `dist/release/libtraining_rooms.nro` (126,976 B) | `6d6443849b8c9462821afca1b971b735e03dfd450eb99c37494723362f4988f3` |
| diag | `dist/diag/libtraining_rooms.nro` (122,880 B) | `82f60e248115f766cd54f15d936359837a7640f40662051b5e19d207722e2a03` |
| tile cursor (route helper, 0.1.0) | `/home/intpa/ssbu-tile-cursor/dist/libssbu_tile_cursor.nro` | `b8313ca2e66a52f1bcfc4afc764df1b7cdc5aee7e194bdea671cea02746934bc` |

Eden: `/home/intpa/Applications/Eden-GIF/current` = `20261031-b3e672c43f03-nv01-timer` (the playbook text names the
older `…-v8`; `current` is what the playbook's tools launch), AppImage sha256 `d483c082147eb6a5838ca90b8c0988baec8f8e7bc662234215069a4746215570`,
title `Eden | 5a4ed955f1-main-nv01-timer-b3e672c43f03 | … | Super Smash Bros. Ultimate (64-bit) | 13.0.5 | ANV`.
Playbook followed: Surface clone `53fd1f2454cb98a92eaa77f248a06656fdebdbcd` (the Yoga's clone is at `1717ecab5689`, older; tools and docs were read from the Surface copy).

## Host, claims, profile

- Host: the Yoga (`yoga-ubuntu`, reached as `memento-claw-yoga-ubuntu`; its login shell is PowerShell). No Eden ran there, no open claim, MemAvailable
  about 20.9 GiB at each launch (`LAUNCH-*.json`), `df` 283-285 GB free. Display `:93` (never `:61`), GPU lock unit `trp1-<label>-gpu-lock`,
  one Eden at a time. Profile `/home/intpa/trainingrooms-eden-p1/profiles/t1`: copy of `ssbu-desync-1029/profiles/b` (never `a`), game content hard-linked
  (`gv.py prep` method): `du -sh` of source `b` 6.4 G, of the copy counted after it 681 M. Files the run writes (config, saves, plugin folders) are real copies.
- Claim lines (ledger `/tmp/deluxe-surface-1007/OWNER-COORDINATION.md` on the Yoga), owner `Claude 0caa72b8/pkg1-eden`, one CLAIMED before each Up and one
  RELEASED after each Down: tr101 01:23Z/01:33Z, tr102 01:35Z/01:49Z, tr103 01:49Z/01:56Z, tr104 01:56Z/02:00Z; every Down printed `alive: false`, `left: []`.
- Muted (operator rule): every boot ran `settings.set core.volume 0`, `settings.save`, `settings.get` (returned `"value":"0"`) before `game.load`
  (see `runs/tr10N/launch` output in the run notes below); the profile's `qt-config.ini` carries `volume\default=false`, `volume=0`.
- Profile restored after tr104: the two original NROs (`libarcropolis`, `libssbu_desync_detector`) moved back and their sha256 equal source `b`'s
  (`92171B2F4DBD…`, `7603AD747733…`), the test NROs, the per-label set-aside and the `TrainingRooms*` / `ssbu-tile-cursor*` output folders removed. No Eden, display unit or lock left.
- One window only (no second window to Start).

## Labels and settings

`settings.json` written into the copy's `sd:/ultimate/TrainingRooms/` before boot (shape of `src/model/settings.rs`; files in `eden/settings/`; copy byte-identical to the file read back in each run folder).

| Label | NRO | Input Delay | Stage Hazards | L+R+A Reset | Purpose |
| --- | --- | --- | --- | --- | --- |
| tr100 | none | - | - | - | profile copy only (`prep`) |
| tr101 = boot 1 | release | 5 | Off | On | rows of boot 1 |
| tr102 = boot 2 | release | 0 | On | Off | rows of boot 2 |
| tr103 = boot 3 | diag | 5 | Off | On | rows of boot 3 |
| tr104 (extra) | release | 0 | Off | On | continuous-run delay baseline (see "Input delay"); a relaunch because settings are read at boot only |

Route (all gated; `eden/route_local.py`): title, A until the main-menu screenshot matches, macro `tr-main-to-stage-select` (first 454 frames of the existing
`main-to-training-fast01`, sha256 of the original in `eden/macros/`), tile cursor down x3 + right x8 gated on the tile log `settled screen=sss at=41`
and the name panel reading "Pokemon Stadium 2", A, CSS match, L3+Up (tile 78 = Steve), A, "Ready to fight" match, A. The final "training scene" gate (threshold 16)
missed in tr102, tr103 and tr104 (best distances 23-25, the camera and random CPU differ from the reference taken in tr101); each was confirmed by screenshot
(`runs/tr10N/shots/*training-b*.png`: PS2, P1 Steve, CPU Dark Samus/Lucas/Squirtle, 0.0%). Not a retry-worthy precondition failure: the install line and every earlier gate had passed.

## Boot 1 (tr101, release, delay 5 / hazards Off / L+R+A On)

| Row | Result | Evidence |
| --- | --- | --- |
| Install line | **pass** | `runs/tr101/events.log`: `0 plugin=training_rooms/0.1.0 game=13.0.5 build=21450c647b8c5940ed5b68218ea128fd3c5d597d final_input_mapping=stock/far reset_check=stock/near hazard_window=stock/checked hazard_hook=stock/near hazard_flag=stock/checked on_init_slot=stock/far on_init=stock/checked is_training_mode=stock/checked fighter_information=stock/checked is_operation_cpu=stock/checked input_delay=on lra_reset=on stage_hazards=on reset_hub=on result=installed heap_allocations=0` (454 B, one line, version 13.0.5, ten sites, every one `stock`, hooks written `far`/`near`/`near`/`far`, the rest `checked`). Identical bytes in tr102, tr103, tr104. |
| Hazards absent on PS2 | **pass** | PS2 (tile 41, name panel "Pokémon Stadium 2") stayed in its Stadium form for 6 min 40 s of play (screenshots from 01:27:13 to 01:33:30 plus the 23 per-trial baselines of the delay scan, all Stadium form; ten of them in `runs/tr101/hazards-off-sheet.png`). Reference: with Hazards On (tr102) the first transformation showed between 01:38:15 and 01:38:27, i.e. 30-45 s after the Training scene appeared, and the stage changed form several more times in the next 3 min. |
| Input delay observable, frame-counted | **pass** | see "Input delay": continuous onset at macro frame 74 (press at frame 60) against 69 with delay 0 (tr104): +5 frames. Reproduced in tr103 (74). |
| L+R+A resets | **pass** | `runs/tr101/shots/012903-lra0-start.png` (spawn), `012906-lra1-moved.png` (walked right), `012913-lra2-after-chord.png` (back at spawn, same pose as lra0). Also in tr103 with the paused-frame method (`runs/tr103/probes/step-lra/f005,f060,f090,f120,f160.png`: spawn, moved, reset, back at spawn). |
| Save/reset clears the delay queue | **not observable in this build** | No diag line records the hub run or the queue length. An attempt (hold L+R+A and stick right for 80 frames, `runs/tr101/shots/*hold*`) ended with Steve at spawn; that fits a repeated reset while the chord is held and equally fits the queue being untouched, so it proves nothing. Needs a diag line for the hub (lead). |

## Boot 2 (tr102, release, delay 0 / hazards On / L+R+A Off)

| Row | Result | Evidence |
| --- | --- | --- |
| Hazards present | **pass** | `runs/tr102/hazards-on-sheet.png` (30 screenshots, 01:39:04-01:41:51): Stadium form, then flying/wind form, Stadium, a second form, Stadium, ice/rock forms; `runs/tr102/shots/013842-lra3-hold-right-end.png` shows the rock form; `014853-now.png` the ice form. |
| No delay | **pass (by tr104)** | Boot 2 itself only has frame-stepped data (onset 64, `runs/tr102/probes/step-jump`), which is not comparable to the continuous runs (see caveat below). The comparison that counts is tr104, same Input Delay 0: continuous onset 69. |
| L+R+A ignored | **pass** | `runs/tr102/probes/step-lra2/f005,f060,f160.png` (stable ice form): Steve walks right (f060) and is still beside the snowman 90 frames after the 30-frame chord (f160); at spawn he would be back at the hut door as in f005. Earlier try on the rock form (`runs/tr102/shots/*lra*`) was confounded by the stage transformation and is not used. |

## Boot 3 (tr103, diag NRO, delay 5 / hazards Off / L+R+A On)

| Row | Result | Evidence |
| --- | --- | --- |
| Install line (diag build) | **pass** | `runs/tr103/events.log` byte-identical to boot 1 (454 B). |
| Diag lines the specs list | **fail (defect D1) / nothing else exists** | The only diag-specific line in the code is `[PracticeDiagnostics] adapter=ring-diag session_start; …` (`src/runtime/diag_log.rs`). `practice-diagnostics.log` was 0 bytes while Eden ran (about 6 min in Training) and after it stopped, in tr103 and in the release builds (tr101, tr102, tr104), although `[PracticeDiagnostics]` is in `SHIPPED_TAGS`. No per-event, per-poll or hub diag lines exist in this build (`rg 'diagnostic\(' src` lists only boot and settings breadcrumbs). |
| `foreign=` stand-in modes | **N/A: not implemented** | `rg -n 'foreign=|standin|diag\.txt' src` finds no stand-in code and no `diag.txt` reader (only the word in `GuardOutcome`). |
| Behaviour of the diag build | **pass** | delay onset 74 continuous (same as boot 1); hazards absent for about 4.5 min (`runs/tr103/hazards-off-sheet.png`); L+R+A reset works (above). |

## Input delay (SPEC-input-03, IN03-E02) method and numbers

No input display exists in package 1, so the delay is measured in emulated frames. Eden's macro frame counter (`macro.status.frame`) is exact: `game.pause`
stopped the game at exactly the polled frame in every pause (every pause in the `probe.py` and `step_probe.py` runs; `frame_paused` equals the target and `frame_reads_after_pause` is constant).
Macro `tr-jump-probe`: neutral, X pressed at frame 60, released at 70. `probe.py` plays the macro, pauses at target frame k, screenshots the paused frame, stops the macro and
repeats for each k (one macro run per k, **no pause before k**). `eden/analyze_probe.py` measures the mean grey difference over Steve's region against a baseline shot:
onset = first frame above 6.5 that stays above.

| Run | Plugin | Setting | Onset (macro frame) | Noise floor before onset |
| --- | --- | --- | --- | --- |
| tr101 `probes/jump-scan` | release | 5 | **74** | 5.46 |
| tr103 `probes/jump-scan` | diag | 5 | **74** | 4.21 |
| tr104 `probes/jump-scan` | release | 0 | **69** | 3.35 |

So 5 frames of delay for setting 5, exactly (a jump press at 60 shows at 69 undelayed and at 74 delayed), in both builds. This also answers SPEC-input-03 open question 1 for this case: in Training on Eden the mapping is
polled once per frame for P1 (otherwise the shift would be shorter than the setting).

**Caveat on frame stepping.** `step_probe.py` (pause and screenshot at *every* frame in one run) gave onset 64 with delay 0 (tr102) and 65-67 with delay 5 (tr103, six runs: 67, 65, 65, 66, 66, 66),
i.e. earlier than the continuous runs for the same settings. Pausing every frame changes the timing of the macro's inputs relative to the game, so those numbers are not comparable to the
continuous ones and are not used as delay evidence. They are kept in `runs/tr102/probes/step-jump*` and `runs/tr103/probes/step-jump*`.

## Scans

`console.log` searched for `panic|abort|crash|Unmapped` (case-insensitive) with `tr_eden.py scan`, before and after each stop: tr101 5688 lines, tr102 5774, tr103 6034, tr104 5722; **0 matches in all four**
(no `Unmapped` at any address, so none below 0x1000 or in the plugin's window). Log filter as the playbook's `gv.py`: `*:Warning Frontend:Info Service.Sockets:Info Network:Info`.
`run.json` and `memory.peak` kept per run (peaks 6,208,151,552 / 6,245,142,528 / 6,033,592,320 / 6,244,765,696 B; `run.json` state `stopped`, `reason` `interrupted` = the SIGTERM the playbook's stop sends, `exit_code` 1 as in the playbook's runs).
Game fps 59.9-60.0 in Training.

## Defects and observations for the lead

1. **D1 `practice-diagnostics.log` stays empty** in both builds (0 bytes while running and after stop), although `init_diagnostics()` writes `[PracticeDiagnostics] adapter=ring… session_start` (tag in `SHIPPED_TAGS`) and the diag build writes every line.
   `events.log` (opened, written, `sync_data`, closed per call) works. Candidates to check, not verified: the drain thread's long-lived `File` handle is never flushed to the host in Eden, or the drain never wrote. Raw: `Get-Item …/TrainingRooms/*` → `events.log 454`, `practice-diagnostics.log 0`, `settings.json 128`.
2. **D2 install-line tick is `0`** (first token `0 plugin=…`): `SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default()` yields 0 in the Switch build under Eden. Spec wants a tick or unix time; the line is otherwise per spec.
3. **D3 the install line's `lra_reset=on stage_hazards=on input_delay=on` mean "feature installed", not the setting.** With settings Off/On/0 (tr102) the line still says `on`. Worth a doc note or renaming.
4. Heap: `heap_allocations=0` in every boot, but the counter is read at install time, before any hook ran, so it proves nothing about the hooks; no diag line reports it later.
5. `tools/check/compliance.py` (playbook 53fd1f2) on the worktree and release NRO: failing machine rows `supported version 13.0.5 in model | src/model.rs`, `every site guard is 64 bytes | 0 guards in src/`, `counting allocator reports heap_allocations | src/*.rs`,
   `host tests in model | 0 #[test] in src/`, README GIF/build/license rows, `docs/media/*.gif`, `HANDOFF.md`, `working tree clean`. The first four look like the checker reading only `src/model.rs`/`src/*.rs` while the code is in `src/model/*.rs` and `src/runtime/*.rs`; the rest are real or are my untracked `eden/` files.
6. SPEC-state-06 §9 `E-state-06-load-resets-delays` and IN03-E04 (clear on owners) are not testable in package 1 (no load, no owners), as the provenance table already says.

## Files kept (under `eden/`; nothing committed)

`tr_eden.py` (harness run on the Yoga: claim, prep, up, down, scan), `route_local.py` (gated route, run from the Surface over one ssh), `probe.py`, `step_probe.py` (run on the Yoga), `analyze_probe.py`, `analyze_step.py`, `look.ps1`, `seq.py`, `calm_then.py`,
`mkmacros.py` + `macros/` (probe macros and the route segment with sha256 of the original), `settings/` (the four settings files), `refs/` (reference screenshots for the gates), `runs/tr101..tr104/` (events.log, settings.json, console.log, run.json, memory.peak, receipts, shots, probes).
**`eden/runs/` is about 690 MB (screenshots of four runs) and is not in `.gitignore`; add `/eden/runs/` before committing.**

Reproduce a number: `python3 -I eden/analyze_probe.py eden/runs/tr101/probes jump-scan 6.5` (prints `onset_frame=74`), same for tr103 (74) and tr104 (69).
