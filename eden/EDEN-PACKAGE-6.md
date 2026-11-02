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