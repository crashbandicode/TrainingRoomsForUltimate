# Eden results, package 6

## s6-01, 2026-11-02, Surface (display :98, muted, offline), diag NRO `5455208d` + standalone CPU pilot `e3033426…`

Purpose: does Skyline std's `File::sync_all` succeed on the console, now that the carried `cycle_preload.ini` writer fails a store when it does not (lead ruling 2026-11-02)? Build: `s6-01-diag-5455208d.nro` (commit `91d83f0`, before the integration merge; resident 339,968 B), installed as `libtraining_rooms.nro` next to `libssbu_cpu_pilot.nro` in the owned profile copy `trainingrooms-eden-pkg6/profiles/s6` (the judge-log plugin was set aside). Claim and release in the Yoga ledger (07:21Z / 07:23Z, "on the Surface"); the host check refused once because another stream's line for the Ally mentioned the Surface in passing, and the harness now reads the "on <host>" phrase first.

Result, from `practice-diagnostics.log` (raw lines):

```text
[SyncProbe] write_all Ok(())
[SyncProbe] sync_all Ok(())
[SyncProbe] sync_data Ok(())
[SyncProbe] rename Ok(())
[SyncProbe] remove Ok(())
[SyncProbe] done sync_all_ok=true
```

`sync_all` returns `Ok(())` under Eden (the scratch write, flush, rename and removal all succeed), so the writer's contract (a failed `sync_all` fails the store) does not break the INI store here. This is Eden's filesystem service, not a console; the same row stays on the real-console list. `events.log` install line: every site `stock`, boot clean; `console.log` (11,909 lines) has no panic, abort, crash or Unmapped line; `memory.peak` 5,638,549,504 B. Outputs under `eden/runs/s6-01/` and the profile's `ultimate/TrainingRooms/` (gitignored).
## s6-02 and s6-03, 2026-11-02, Surface (display :98, muted, offline): Eden set C, CPU-as-P1 fallback (Q11)

Builds: merged pkg6 + pkg6b tree (`d0b6faa`) diag `cba0267a` for s6-02; the same tree with the shortened seam fields diag `4b9bc612` (`3c5f246`) for s6-03. Standalone CPU pilot `e3033426…` (v0.2.1), tile cursor `b8313ca2…`. Claims and releases in the Yoga ledger (08:2xZ, "on the Surface"). Routes: pkg2's route (`eden/route_s6.py`, refs and macros copied from `stream/pkg2 eden/`), run with `/usr/bin/python3` (the PowerShell `python3` is `~/.local/bin/python3` and has no PIL).

**k04, pilot present (s6-02).** Install line: `p1_cpu=not_installed reason=standalone_pilot_present`, the five `sites_pkg6` rows `stock/checked`, `[Ui] anchors installed ... pilot_present=true`; no `[CpuPilot]` line (nothing installed, P1 driven once, by the pilot). `console.log`: 0 matches (panic, abort, crash, Unmapped).

**k03, pilot absent, `p1_cpu_control` On / `p1_cpu_level` 3 (s6-03).** Install line: `p1_cpu=installed`, `seams=4 seams_stock=4 seams_other=[] seams_skipped=[] seams_shadow_diff=[] seams_missing=[]` (the four carried `cpu_pilot` rows, read through `seam_install::gate`), `result=installed` present. Diag: `[CpuPilot] main_base 0x802e0000`, `offline_probe_ready`; after the route to Training (stage select tile 6, CSS, ready to fight) `first_input scene=OfflineTraining slot=0`, `first_pair` (P1 entry 0, CPU entry 1), `admission`, and **`menu_control event=armed level=3 rank=21`**: the fallback armed P1 from the two settings rows. Screenshot `eden/s6-03-p1-armed.png`: Steve (P1) has hit the CPU R.O.B. to 46.9% with the input display showing P1's AI inputs, the CPU not touched. `console.log` 11,965 lines, 0 matches; `memory.peak` 7,029,399,552 B.

**Finding, fixed (`3c5f246`):** on the first merged-tree boot (s6-02) the carried seam report's authoritative-group and unchecked-image lists made the one install line 8,188 characters and cut it before `result=` and `heap_allocations=`. The recorded edit of `seam_install.rs` now writes counts and the failing rows only (line 7,030 characters with `result=installed` and the full set of sites); the late report still names the groups and images.

Not run: CP0/CP1/CP3 (hot swap while P1 is armed: needs the roster cycle), Deluxe 0.10.1 coexistence (the SDK is parked, L1), the online 1v1 scope (package 7), the release NRO row of set A. `scene.initialized` reads false in the log because the lobby `game_mode` probe is not initialised yet (the lobby half is declared with the cycle).
## s6-04, 2026-11-02 11:30-11:45Z, Surface (display :98, muted, offline): sets A/B in one boot (third successor)

Build: diag `dcf52de3` (stream/pkg6 `98d6f5b`; resident 3,158,016 B) + standalone CPU pilot `e3033426…` + tile cursor `b8313ca2…`; boot settings `eden/settings/s6-a.json` (Player Swapping on; P1 cycle Kirby, Ice Climbers, Pokemon Trainer, Palutena; CPU cycle Pikachu, Bowser). Route `eden/route_s6_cut.py` (route_s6 with the install gate on the line start, see the first finding); rows `eden/rows_s6.py`, `eden/extra_cpus_s6.py`; results `eden/runs/s6-04/rows.json`, screenshots `eden/runs/s6-04/shots/`. Stage Kongo Jungle (tile 6), P1 Steve, CPU Mythra. `console.log` 12,415 lines: 0 matches (panic, abort, crash, Unmapped); `memory.peak` 7,463,059,456 B.

| Row | Result | Evidence |
| --- | --- | --- |
| Roster cycle started (every `CALLED_ROWS` guard passed) | pass | no `roster cycle not started` line |
| `[ItemTable]` 38 entries, none MISSING; Peach and Daisy turnip variations | pass | `ITEM_VARIATION_{PEACH,DAISY}DAIKON_{1,6,7,8}` = 0, 5, 6, 7 (`itemtable.txt`) |
| Item grant, owner not in the match (`diag-item.req` 11, Link's bomb to Steve) | pass | `[CharacterItem] granted choice=11 kind=120 variation=0` |
| Save with the item, throw it, load: the engine gives it back | pass | `state done ...` then `[CharacterItem] restored choice=11 variation=0 life=-1 fuse=1777` |
| Menu commit reaches the preload engine (`publish_applied`, ruling 2) | pass | `[CyclePreload] applied slots=2 ...` right after `diag_settings_reload` |
| Preload on Player 1 | pass (late) | `[PreloadStatus] state=Loading planned=5 ready=2 ... text="Loading 2/5"`, later `Loaded 5/7`; the row's 60 s cap expired first (harness) |
| P1 hot swap by the cycle chord (ZL + D-pad Right) | pass | `chords_passed bits=0x20`, `request handed ... selection=Cycle`, `HUD refresh owner=0 kind=0x6`; screenshot Kirby (the first needle was wrong, fixed) |
| Kirby hat by the shortcut (Special + L3) | pass | `[KirbyHats] request role=0 kind=0x0 source_slot=2`, `confirmed` (Mario's hat, Mario not in the match) |
| Save/load gives the hat back through the hat engine (ruling 2 wiring) | pass | after the load `[KirbyHats] request role=0 kind=0x0 source_slot=2` (the restore path) |
| Swap to the Ice Climbers pair; save/load finishes (Nana hand-off) | pass | `HUD refresh owner=0 kind=0x4b`; `state done ... next=NoAction` |
| Solo Popo: load ends at Popo | inconclusive | load done and a new save accepted, but the screenshot still shows Nana: `solo_popo_player_cycle` reached the menu, the built Ice Climbers kept both bodies. The no-Nana path is host-tested only (`state01_solo_popo_has_no_handoff`) |
| Swap to Pokemon Trainer (native group), next fighter (Palutena), CPU cycle (ZL + D-pad Left) | pass | `kind=0x24`, `kind=0x36`, `HUD refresh owner=1 kind=0x8` |
| Native Training with an extra CPU (pause menu: No. of CPUs 2, Behavior Walk): save/load leaves it unfrozen | pass | `state done`; three shots 2 s apart, both CPUs move (mean diffs 30.4, 30.1). **Finding:** the extra CPU (HUD: Mythra, entry 2) is drawn with Palutena's body (`shots/*-p1-visibility.png`): the cycle's helper source uses native entry 2 in Training; see the second finding |
| Practice session retired by a hot swap | not observed | no `practice_session held=` line at all: the lease is never acquired in native Training (open, below) |
| R5: hook allocation delta 0 | **fail** | 6,311 allocations inside hook scopes, about 20/s from entering Training with Steve, about 31/s with Mario/Kirby and preload on, 0/s after the swap to Palutena (`hook_stats` lines) |
| Install line | **fail** (not package 6's code) | the one install line is 8,142 characters and is cut before `result=` and `heap_allocations=` (the s6-02 defect again, with the merged packages' rows) |

Findings for the lead: (1) install line cut by the line limit; (2) entry-2 collision between the cycle's helper source and a native extra CPU (the fork's `legacy_training_owner_reserved`: "this feature reserves only native Training entry2"); (3) R5: allocations in hook scopes while specific fighters are active, not yet attributed to a hook (`hook_stats` has no per-scope allocation counter; next diag build splits them); (4) the practice lease is never acquired in native Training.
## s6-05, 2026-11-02 12:30-12:37Z, Surface: the merged tree's NRO (work/implementation 566780a, diag `3d7a9215`, integrator-3's request)
Cold boot: install line about 23 s after `game.load`, main menu about 40 s, no plugin-silent stall; install line 9,207 characters ending `result=installed ... heap_allocations=0`. Pass: roster started, diag settings reload with periodic lines after it, `[CyclePreload] applied` (through `before_publish`), preload status, P1 hot swap (Kirby), record (50 frames) and playback, menu open/close x3 last (closed and alive each time). FAIL: R5, hook allocations rising about 180-210 per 10 s (2,302 at the end). `console.log` 12,414 lines, 0 matches.

## s6-06, 2026-11-02 12:47-13:00Z, Surface: stream/pkg6 diag `a3bcb55e` (R5 attribution, Solo Popo kept keys, F9 research)
- **R5 attributed:** `allocs=...,cpu_frame:192,input_caller_a_note:25,live_replay:5,roster_input:6,roster_preload:3943`: package 6's preload planner (its 1 Hz planning tick builds Vecs: `OrderedFighterRoster::fighters()`, `ordered_kirby_hat_kinds`, collects in `character_roster::preload_targets`). Root fix next.
- **Solo Popo fixed:** `state popo entry=0 nana_removed=true`, load done, a new save accepted; screenshot `shots/*-solo-popo-after-load.png` shows Popo alone.
- **Lease:** `practice_session held=true` in native Training; a Training hot swap keeps the native entry, so the session stays (the fork's design: the lease retires when entries are replaced). The retirement row belongs to the lobby set.
- **F9 research:** pause-menu dumps (`pausemenu-*.bin`): byte `+0xb40` of the pause menu object is No. of CPUs (1, 2, 3, 1 as changed; it changes while paused, before the game activates the parked entry on close).
- Pass again: install line, item grant/save/restore with the owner absent, Kirby hat restore through the hat engine, preload commit, swaps, menu open/close x3 last. `console.log` 12,421 lines, 0 matches.
## s6-07, 2026-11-02 13:18-13:32Z, Surface: stream/pkg6 diag `4938d913` (R5 v1, F9 give-back, F11 active body)
- Pass: install line (`heap_allocations=0`), preload, swaps, Kirby hat, items.
- **R5 still failing:** `roster_preload` was about 20/s (2,253). A bisection by diag settings reload (`eden/r5_bisect_s6.py`) gave:
  - preload slots 0: 0/s
  - hats only: about 3/s
  - fighters only: about 22/s
  - group swap off: about 23/s
  The source is the ordinary-Mii owner lane of the planner (`cycle_preload::mixed_targets`: a `Vec` from `fighters()` and a `MiiPreferences` clone per planning step). It is fixed in 215dbfd (R5 v2).
- **Lease:** not retired by a Training swap, which is the fork's design. The row moves to lobby set D.
- **F9 give-back crashed the game, and it is backed out in 215dbfd.**
  - Diag clock to console clock: diag ms - 526,436,281. This was checked on three settings/save_states writes.
  - The diag log shows `#703 cpu_quantity=3 seen_by_service` and then `#704 give_back slot=3 cpu_quantity=Some(3) helper=0x2 native=0x5` at console 701.61 s. `#705 [TrioBoundary] reload entry=0x11e5710000 owner=3 incoming=[5] active=[2]` follows.
  - At 701.68 s, 70 ms later, the console shows `Cannot execute instruction at unmapped address 0x0`. LR is main+0x6222a0. The call site is main+0x62229c: `ldr x0,[x19,#0x438]; ldr x8,[x0]; ldr x8,[x8,#0x2f0]; blr x8`, a module vtable slot that is null. R27 is the entry we reloaded (0x11e5710000).
  - The stack holds game frames only, with no plugin frame: scene update → main+0x155c954 → main+0x6573f0 (in main+0x655080, an entry-level function: it writes entry +0x597c/+0x5980) → main+0x60fc6c → main+0x6222a0.
  - Reading: the game activates or re-initialises entry 3 for the raised count while our native reload of that entry is in flight. Our reload was started from the entry's loader callback in the same frame the count changed. This matches the "reload under activation" hypothesis.
  - Entry 2's helper was leased by a Kirby hat (`Wait`), so it was not given back. Without a give-back it shows as the extra CPU at count 2 (the open F9 defect).
- **Unexplained, new in this build:** 1,152 `Unmapped Read16 @ 0xfffe7683fffe76a2..a8` (four halfwords: one misaligned 8-byte load), one burst per frame. They run from console 84.18 s to 89.58 s, which is diag 526,520,459 onward: 5 ms after the Training scene's first loader callback (`#85 cpu_quantity=4294967294`, unreadable at scene init) and during its load. They are absent in s6-04/05/06 (same Eden binary). The address is misaligned, so it is not one of our guarded chain reads (every step is 8-aligned). Not attributed yet: s6-08 shows whether they recur without the F9 code.
