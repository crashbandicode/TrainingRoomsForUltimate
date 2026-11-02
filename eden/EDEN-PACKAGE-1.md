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

Reproduce a number: `python3 -I eden/analyze_probe.py eden/runs/tr101/probes jump-scan 6.5` (prints `onset_frame=74`), same for tr103 (74) and tr104 (69); frame-stepped numbers: `python3 -I eden/analyze_step.py eden/runs/tr102/probes step-jump 6.5` (prints `onset_frame=64`); console.log counts and matches: `python3 -I eden/tr_eden.py scan <label>`. The scripts have host tests on synthetic data (a planted 5-frame shift is measured as 5, no shift as 0, a flicker is not an onset, planted panic/Unmapped lines are found): `python3 -I tests/eden/test_analyze.py`, also run by `cargo test` (`tests/eden_scripts.rs`).

## Re-run after the D1-D4 fixes (2026-11-02, labels tr105 and tr106)

Builds (hashes verified on the Surface, again after scp to the Yoga): release 9bad6541a3b1d7497724e6c1db108083017b4a1aef3c9df49ea39a7786cc0ac5, diag db7a4dd980b17b275ee439e0e644b6090b3ed6aede297f165fd8c1e93423e475 (worktree HEAD 5247578).
Same rules as above: claim before and release after each Down (ledger 02:09Z/02:11Z and 02:11Z/02:13Z, owner Claude 0caa72b8/pkg1-eden), own profile copy 	1, muted via the CLI (olume.get 0), display :93.
Settings both boots: Input Delay 5, L+R+A On, Hazards Off (settings/boot1-delay5-hazoff-lraon.json). The route's last gate threshold was raised from 16 to 30 (the right scene scored 23-25 before; other screens score 75); it matched at 10.9 and 25.5.
Note: 
uns/tr105/events.log and practice-diagnostics.log were re-created from the raw lines printed during the run (a later fetch had copied tr106's files over them, the profile folder being shared); the lines below are the verbatim ones.

| Row | Result | Evidence |
| --- | --- | --- |
| tr105 install line: non-zero tick | **pass** | first token 483391898 (boot-relative milliseconds; it was  ) |
| <feature>=installed per feature | **pass** | … input_delay=installed lra_reset=installed stage_hazards=installed reset_hub=installed … |
| separate settings report | **pass** | … settings: input_delay=5 lra_reset=on stage_hazards=off result=installed heap_allocations=0; the ten site outcomes are unchanged (stock, ar/
ear/checked as before) |
| tr105 practice-diagnostics.log non-empty | **pass** | 128 B: 483391857 #1 [PracticeDiagnostics] adapter=ring session_start; max_files=2 max_bytes_each=131072 ring_bytes=32768 line_max=9216 (was 0 B) |
| tr105 delay regression | **pass** | continuous-run onset at macro frame **74** (
uns/tr105/probes/jump-scan, noise floor 4.32; first frame above 6.5 is 7.29), against 69 for delay 0 (tr104, previous build; same method) |
| tr106 (diag) hub_run on reset | **pass** | 483626945 #2 [TrainingRooms] hub_run cause=TrainingReset input_delay_cleared=5 (on entering Training, before any chord), then #3 … input_delay_cleared=5 after the walk-then-L+R+A macro and #4 … input_delay_cleared=5 after the L+R+A + stick-right hold; 
 is the number of queued records (5 = the delay), so n>0 was seen. Holding the chord for 30 and 80 frames gave one hub_run each, not a repeat. |
| tr106 diag session line | **pass** | 483562974 #1 [PracticeDiagnostics] adapter=ring-diag session_start; … ring_bytes=131072 line_max=12288 |
| release NRO has no hub_run | **pass** | byte search of dist/release/libtraining_rooms.nro: hub_run=False input_delay_cleared=False; the diag NRO has both (True) |
| Unmapped / panic scan | **pass** | console.log panic\|abort\|crash\|Unmapped: 0 matches in tr105 (5726 lines) and tr106 (5694 lines); 
un.json and memory.peak kept (6,072,090,624 and 6,077,005,824 B) |
| Released, profile restored | **yes** | both Downs live: false, left: []; the profile's plugins are the two original NROs, sha256 equal to source  (a restore step deleted the set-aside copies once; both were re-copied from  and verified); no display unit or lock left |

## Input Delay at the caller-side sites, with and without the CPU pilot (2026-11-02, labels tr107 to tr110)

Builds (hashes verified on the Surface and again after `scp` to the Yoga; worktree HEAD `e20416a`): release `26a5fef0383d2252f44868257bd7e95ccf17c7157b4e1c00a314e377016303f9`,
diag `4792c91a4f631e797c8fcee41f3629dd5a39c0f0544437fd0fe5ac1d80c9e888`. Pilot: `libssbu_cpu_pilot.nro`, v0.2.1, sha256 `e30334261370a09453d5eeb36513a44c945440694d86d916375dfd83ef9c7582` (73,728 B), read-only copy from
`/home/intpa/ssbu-cpu-pilot-install/switch-mtp-v0101-v021/pilot/after/…/skyline/plugins/` (the card's copy of the 2026-11-01 MTP preflight). Tile cursor as before.
Same rules: Yoga, claim before and release after each Down (ledger 02:35Z/02:38Z, 02:38Z/02:41Z, 02:41Z/02:44Z, 02:44Z/02:46Z), own profile copy `t1`, muted (`volume.get` 0 each boot), display `:93`.
Settings in every boot: Input Delay 5, L+R+A On, Hazards Off. Harness changes (`tr_eden.py`): `--nro path:installed-name`, the pilot's output folder is archived per label, and a `restore` command that returns
the plugin set to source `b`'s by sha256 before it deletes anything.

| Row | Label, set | Result | Evidence |
| --- | --- | --- | --- |
| A install line | tr107, release, no pilot | **pass** | `484967633 plugin=training_rooms/0.1.0 game=13.0.5 build=21450c64… input_caller_a_note=stock/near input_caller_a_apply=stock/near input_caller_b_note=stock/near input_caller_b_apply=stock/near reset_check=stock/near … input_delay=installed lra_reset=installed stage_hazards=installed reset_hub=installed settings: input_delay=5 lra_reset=on stage_hazards=off result=installed heap_allocations=0` (four caller sites replace the old `final_input_mapping`; every site `stock`); `practice-diagnostics.log` holds the session_start line |
| A delay | tr107 | **pass** | continuous-run onset at macro frame **74** (`runs/tr107/probes/jump-scan`, noise floor 4.34, first frame above 6.5 is 8.58), baseline 69 from tr104 (same method, delay 0, the earlier release build, no pilot): +5 frames |
| A L+R+A resets | tr107 | **pass** | `runs/tr107/lra-montage.png`: spawn, walked right, back at spawn after the chord |
| B both install lines | tr108, release + pilot, default names | **pass** | ours as in A (`input_delay=installed … result=installed`); pilot `plugin=ssbu-cpu-pilot/0.2.1 level=3 … fim_valid=stock/near fim_exit=stock/near hud=stock/near ai_begin=stock/near guests=0 ai_end=stock/near guests=0 results_rematch=owner:stock/near … result=installed heap_allocations=0 menu=host_missing menu_allocations=0` (`runs/tr108/pilot-events.log`) |
| B delay | tr108 | **pass** | onset **74** (noise floor 4.16), same as without the pilot |
| B pilot still works | tr108 | install line only | no quick in-Training observable is documented (the pilot's source and docs are not on these hosts; its menu row needs the Deluxe host, `menu=host_missing`); its `fim_valid` and `fim_exit` sites installed (`stock/near`) beside our caller-side hooks |
| C other load order | tr109, our NRO installed as `0libtraining_rooms.nro` | **pass** | both install lines identical in content to B; delay onset **74** (noise floor 4.46) |
| Load-order evidence | tr108 vs tr109 | | birth time of each plugin's `events.log` (written at the end of its `main`; the plugins' ticks use different clocks and are not comparable): tr108 pilot `…45.315512119` before ours `…45.388512839` (pilot loaded first, 73 ms earlier); tr109 ours `…42.369255998` before pilot `…42.392256225` (ours first, 23 ms earlier). Files in `runs/tr108` and `runs/tr109/load-order-birth-times.txt`. Skyline loads in directory order, here by file name. |
| D `hub_run` with the pilot | tr110, diag + pilot | **pass** | `485574919 #2 [TrainingRooms] hub_run cause=TrainingReset input_delay_cleared=5` (Training entry), `#3` after the walk-then-L+R+A macro, `#4` after the chord-plus-stick hold, all `input_delay_cleared=5`; pilot line `plugin=ssbu-cpu-pilot/0.2.1 … result=installed` |
| Release has no `hub_run` | `dist/release/libtraining_rooms.nro` | **pass** | byte search: `hub_run=False`; diag `hub_run=True` |
| Unmapped / panic scan | tr107-tr110 | **pass** | `console.log` `panic\|abort\|crash\|Unmapped`: 0 matches in all four (5726, 5728, 5746, 5714 lines); `run.json` and `memory.peak` kept (6,061,506,560 / 6,008,090,624 / 6,022,520,832 / 6,024,269,824 B) |
| Released, profile restored | | **yes** | every Down `alive: false`, `left: []`; `restore` matched the plugin set to source `b` by sha256 (`libarcropolis` 92171b2f4dbd, `libssbu_desync_detector` 7603ad747733) before it removed the per-label set-aside copies; no display unit or lock left |

## Final package-1 build (menu model and full settings factory table), release + pilot and diag + pilot (2026-11-02, labels tr111 to tr113)

Builds (hashes verified on the Surface and after `scp` to the Yoga; worktree HEAD `68a072b`): release `434d81761869c12f21cf902f3730c3b324a979bee0c90e9948aaa8c52d4a6aae`, diag `3192a97b2d23ea822ec37ba4e0213a488e45eb9b9c4085f562474440ac215bc7`;
pilot `e30334261370a09453d5eeb36513a44c945440694d86d916375dfd83ef9c7582` (same file as tr108), tile cursor as before. Yoga only (the Surface ran another agent's Eden), one claim and release per boot (ledger 03:18Z to 03:23Z), own profile copy, muted (`volume.get` 0), default load order (pilot before ours).
tr113 is a third boot I added for the unknown-keys check (the lead's two labels cover the other rows).

| Row | Label, set | Result | Evidence |
| --- | --- | --- | --- |
| install line, sites, settings field | tr111, release + pilot, delay 5 / L+R+A On / Hazards Off | **pass** | `487535251 plugin=training_rooms/0.1.0 game=13.0.5 build=21450c64… input_caller_a_note=stock/near input_caller_a_apply=stock/near input_caller_b_note=stock/near input_caller_b_apply=stock/near reset_check=stock/near … input_delay=installed lra_reset=installed stage_hazards=installed reset_hub=installed settings: input_delay=5 lra_reset=on stage_hazards=off result=installed heap_allocations=0`; pilot line `plugin=ssbu-cpu-pilot/0.2.1 … fim_valid=stock/near fim_exit=stock/near … result=installed`; `practice-diagnostics.log` has the session_start line |
| delay onset | tr111 | **pass** | continuous-run onset at macro frame **74** (`runs/tr111/probes/jump-scan`, noise floor 4.36, first frame above 6.5 is 8.41) against 69 for delay 0 (tr104, same method, earlier build): +5 |
| L+R+A resets | tr111 | **pass** | `runs/tr111/lra-montage.png`: spawn, walked right, back at spawn |
| hub_run lines | tr112, diag + pilot, same settings | **pass** | `487784861 #2 [TrainingRooms] hub_run cause=TrainingReset input_delay_cleared=5` (Training entry), `#3` and `#4` after the two L+R+A macros, all `input_delay_cleared=5`; our line `… settings: input_delay=5 lra_reset=on stage_hazards=off result=installed`, pilot `result=installed`; `adapter=ring-diag` session line present |
| release has no `hub_run` | `dist/release` | **pass** | byte search `hub_run=False` (diag `True`) |
| extra unknown keys and `last_page` load | tr113, release + pilot, `settings/boot-extra-keys-delay5-hazoff-lraon.json` (the three rows plus `menu.unknown_future_row`, top-level `last_page":"misc"`, `mii_preferences`, `some_later_build_key`) | **pass** | install line `settings: input_delay=5 lra_reset=on stage_hazards=off result=installed heap_allocations=0` (same as the plain file); the folder holds only `events.log`, `practice-diagnostics.log`, `settings.json` (no `settings.invalid-<n>.json`, no `settings issue` breadcrumb); `settings.json` sha256 first 12 `E26E9F62599C` before and after boot (not rewritten) |
| Unmapped / panic scan | tr111-tr113 | **pass** | `console.log` `panic\|abort\|crash\|Unmapped`: 0 matches in all three (5746, 5714, 5714 lines); `run.json` and `memory.peak` kept (6,039,302,144 / 5,998,366,720 / 4,901,920,768 B) |
| Released, profile restored | | **yes** | each Down `alive: false`, `left: []`; `restore` matched the plugin set to source `b` by sha256 before removing the per-label set-aside copies; no display unit or lock left |

## R1-fix (2026-11-02, stream `r1fix`, labels r1-01 to r1-09)

Tester: Claude Sonnet 5.5 (high), spawned by lead `0caa72b8`. Worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-r1fix`, branch `stream/r1fix`.
Yoga only; harness `/home/intpa/trainingrooms-eden-r1` (deployed from `eden/` with `tr_eden.r1fix.config.json` as `tr_eden.config.json`), own profile `r1` (copy of `ssbu-desync-1029/profiles/b`, never `a`), display `:98` (never `:61`),
claim before Up and release after Down (ledger on the Yoga; every Down `alive: false`, `left: []`), muted (`settings.set core.volume 0`, `settings.save`, `settings.get` returned 0 before every `game.load`), queue per `TRAININGROOMS-EDEN-QUEUE.md`.
Pilot: `libssbu_cpu_pilot.nro` `e3033426...` in every run (default load order, pilot first); tile cursor for the route. `console.log` `panic|abort|crash|Unmapped`: **0 matches in every run** (r1-01 5615 lines, r1-02 5646, r1-03 5689, r1-07 and r1-08 clean, r1-09 5744).

### Builds

| Build | Commit | File | sha256 | Resident (`nro_size.py`) |
| --- | --- | --- | --- | --- |
| first build (r1-01 to r1-06), pre-merge, with the since-removed `StatePause` hooks | `009f6c8` + scene wrapper | release / diag | `2dbc56a0fd74...` / `0c0925821f24...` | 249,856 B / 356,352 B |
| merged build (r1-07 to r1-09) | `e2effe9` (merge `ffc93e4` of work/implementation `c5863bd`, R2 by the UI flag) | release `dist/release/libtraining_rooms.nro` | `11f749169554eb846f2dbcdbb064a188ee4a1eb446cf5f55a4706ca08ead9cd8` | 778,240 B |
| | | diag `dist/diag/libtraining_rooms.nro` | `9c4ea58462f19ba7e999e2ebb4812542e588d84a93823b4f73adbae7b4b90ee3` | 933,888 B |

Resident cost of the five fixes over `4895ea3` (233,472 B, bisected build by build): R4 +4,096 B (the loader), R1 +12,288 B (the JSON serializer for the sanitized file and the transaction); R3, R5 and R2 +0.

### R1 (settings recovery) and R4 (developer.ini): boots

`settings.json` of `eden/settings/r1-partial-bad-delay5-hazon.json`: Input Delay 5, Stage Hazards On, `lra_reset` malformed (`"bad"`), saved defaults Input Delay 3, an unknown top-level key. Boots 2 and 3 run over boot 1's data folder (`--keep-data`).

| Row | Label, build | Result | Evidence |
| --- | --- | --- | --- |
| R1 boot 1: recovery | r1-01 (first build) and r1-07 (merged), release + pilot, `developer.ini` = `duplicate.ini` | **pass** | install line `... developer=off reason=duplicate_key settings: input_delay=5 lra_reset=on stage_hazards=on ... result=installed heap_allocations=0` (`lra_reset=on` is the factory value of the malformed field); `practice-diagnostics.log`: `settings issue at=menu.lra_reset reason=not an array`, `settings recovered; the original is kept as settings.invalid-1.json`; `settings.invalid-1.json` 581 B, sha256 `ef178b927ee9...` = the file that was installed (`EF178B927EE9`); `settings.json` rewritten to 25,918 B (sha256 `5cd6687e7941...`): menu input delay index 5, hazards `[1,0]`, saved defaults input delay index 3, `future_top_level_key` kept |
| R1 boot 2: the recovered fields survive | r1-02 (first build), r1-08 (merged), release + pilot, `developer.ini` = `oversized-on-prefix.ini` (9,019 B, `developer_page=on` first) | **pass** | install line again `settings: input_delay=5 lra_reset=on stage_hazards=on`; no `settings issue` line; no second backup (`settings.invalid-1.json` only); `settings.json` sha256 unchanged (`5cd6687e794174...`, a clean file is not rewritten) |
| R4 duplicate keys | r1-01, r1-07 | **pass** | `developer=off reason=duplicate_key` (`developer_page=off` then `developer_page=on`) |
| R4 oversized, enabled prefix | r1-02, r1-08 | **pass** | `developer=off reason=too_large` (9,019 B file starting `developer_page=on`) |
| R4 clean file | r1-09 | **pass** | `developer=on reason=on` |
| Files after the three merged boots | r1-09 data folder | | `developer.ini`, `events.log` (3 install lines), `migration.json` (pkg3's first-boot import, `result=none`), `practice-diagnostics.log` (+ `.previous`), `settings.invalid-1.json`, `settings.json` |

Evidence: `eden/runs/r1-0N/` (`data/`, `data-hashes.txt`, `console.log`, `run.json`, `memory.peak`, receipts). The write-failure path and the no-free-backup-name path are host-tested only (`tests/core04_recovery.rs`).

### R2: what the investigation found, and the lane that proves the producer

Probes (diag builds r1-04 to r1-06, hooks at the game's pause-begin routine `main+0x150f0c0`, both `StatePause` enter slots, the exit, and about a dozen candidate natives, with a hook on `is_training_mode` as the control): **the native Training pause menu (Plus in Training) never runs the scene's `StatePause`**; the control fired in every run; the fighters' per-frame hooks (`cpu_hook_counts frames=600` every 9.9 s) did not stall while the menu was open. The `StatePause` hooks were therefore removed. Lead ruling: the producer takes the game's layout state of SPEC-core-03 section 2.4 (`L_staying_help` off its rest position, `ui::state::PAUSE_ACTIVE`).

| Row | Label | Result | Evidence |
| --- | --- | --- | --- |
| R2 pause lane: delay 5, a held stick queued, Training menu opened, menu input, closed with the stick released | r1-09 (merged build, diag) | **pass** | macro `tr-pause-lane` (stick right from frame 10, Plus at 70, D-pad down/up while the menu is open, Plus with the stick released at 420); `practice-diagnostics.log`: `502917297 #3318 [TrainingRooms] pause_owner begin queue_dropped=5 owners=0x10` (the queue held 5 records and was emptied at the edge), `502923596 #3321 pause_owner end queued_at_begin=5 owned_polls=386 owned_non_neutral=0 owners=0x00` (386 mapping polls were owned and none gave the game input), `502923664 #3326 pause_owner resume_checked delay=5 leaked=0` (none of the first 5 gameplay polls after the resume carried a queued record); screenshots `eden/runs/r1-09/shots/073440-pause0-before.png`, `073445-pause1-menu-open.png` (the native "Quit Training" menu), `073452-pause2-after-resume.png` |
| (harness note) the lane's own gate printed FAIL | r1-09 | | the diag log rotated at 128 KiB mid-lane and the lane counted lines; fixed (`lanes_r1.py` now reads the previous and current file and selects by millisecond stamp); the lines above were read from `practice-diagnostics.previous.log` of the run |

### R5: hook observation (diag), and the package-1 rows on the merged build

| Row | Label | Result | Evidence |
| --- | --- | --- | --- |
| R5 `hook_stats` after the delay, reset and hazard lanes | r1-09 | **pass** | `python3 -I eden/lanes_r1.py r1-09 stats input_caller_a_apply,reset_check,reset_hub,hazard_rule` -> `GATE ok hook_stats: nonzero coverage ... and allocation delta 0 on every line`; the last line: `hook_stats allocations=0 delta=0 hook_calls=27834 hooks=input_caller_a_note:21304,input_caller_a_apply:86448,reset_check:10481,hazard_rule:1,reset_hub:2,...` (every line `delta=0`, `allocations=0`) |
| install line still ends `heap_allocations=0` | r1-07 to r1-09 | **pass** | every install line (merged build: 49+ site tokens plus pkg3/pkg4/pkg5 rows, 6,614 B, the 8 KiB buffer holds it) |
| Input Delay +5 (recovered setting) | r1-09 | **pass** | continuous-run jump probe `eden/runs/r1-09/probes/jump-scan`: onset at macro frame **74** (`analyze_probe.py ... 6.5`, noise floor 4.98, first frame above 6.5 is 7.96) against 69 for delay 0 (tr104, same method) |
| Stage Hazards On (recovered setting) | r1-09 | **pass** | `lane hazards`: the stage region changes up to 25.3 (mean grey difference to the first shot) over 75 s, shots `073322-haz00.png` to `073432-haz09.png` (Stadium form, then the rock form and back) |
| L+R+A resets | r1-09 | **pass (by eye)** | `073313-lra0-start.png` (rock form, Steve squatting at the right of the left platform), `073321-lra2-after-chord.png` (back in Stadium form, both fighters at spawn, P1 marker); the lane's grey-difference gate FAILed because Hazards On changes the stage form (it compares a fixed box), so it is judged by eye |
| `pause_enter` / `pause_exit` rows | r1-03 | removed | the first build's install line had `pause_enter=stock/near pause_exit=stock/near pause_owner=installed`; both never fired in Training and were removed with the ruling |

### Release and profile

Each Down printed `alive: false`, `left: []`; the claims are in the ledger (07:30Z to 07:35Z for r1-07 to r1-09). The profile `r1` keeps the three boots' data; nothing under `profiles/b` or `a` was touched. A first attempt of the merged batch failed on a script argument bug before any Eden started (the ledger shows RELEASED lines for r1-07 to r1-09 at 07:29Z with no run behind them).
