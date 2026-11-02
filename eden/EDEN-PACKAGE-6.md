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