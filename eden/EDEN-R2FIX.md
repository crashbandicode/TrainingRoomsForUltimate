# Eden evidence, stream `r2fix` (R2-1 native capabilities, R2-8 import errors)

Dated 2026-11-02 (UTC, Yoga clock). Tester: Claude Sonnet 5.5 (high), stream `r2fix` under lead `0caa72b8`. Branch `stream/r2fix`, commits `c3befec` (R2-1)
and `20d6e79` (R2-8) plus the harness commit. Host: the Yoga (claims `Claude 0caa72b8/r2fix-eden`: r2-01 08:37Z-08:40Z, r2-02 08:44Z-08:46Z, both in the Yoga ledger),
display `:93`, work root `/home/intpa/trainingrooms-eden-r2fix`, own profile `r2` (copy of `ssbu-desync-1029/profiles/b`, game content hard-linked; never `a`), muted
(`settings.set core.volume 0`, `settings.save`, `settings.get` = 0 before `game.load`). Other plugins: the CPU pilot (`e3033426...`) and the tile cursor
(`b8313ca2...`). One Eden at a time; the profile was restored after the runs (`tr_eden_r2.py restore`). `console.log`: r2-01 5,636 lines and r2-02 5,634 lines, **0
matching panic/abort/crash/Unmapped**. `memory.peak` 6.05 GB (r2-01), 6.18 GB (r2-02); `run.json` kept. Outputs under `eden/runs/<label>/` (gitignored): `rows.txt`,
`diag.log`, `events.log`, `install-line.txt`, `console.log`, `run.json`, `memory.peak`, screenshots. Route: `route_r2.py` (every gate passed); rows: `rows_r2.py`.

## Build under test

Diag NRO of `c3befec`, built on the Yoga (`/home/intpa/trainingrooms-build-r2fix`, `cargo skyline build --release --features diag`):
`ad9d93dff0c564cfce598e27bb567cd3797552d2813fe1cccae9f1565bf9a7ec`. The release NRO of the same commit: `b119094a31e1865df7a3cbdd56da94608868c08b7b6368c5ec1d9eada6c1cff2`
(resident 806,912 B = 788 KiB by `nro_size.py`; 0 compiler warnings of this stream's code). The diag NRO reads `diag-poison.txt` from the data folder and marks the
named rows foreign before any install decision (`[TrainingRooms] diag_poison <row> found=<bool>`).

## r2-01: poisoned boot (`diag-poison.txt` = `motion_kind transition_term_hook`)

Install line (the two rows and every feature whose closure holds them; all other features stay installed):

    transition_term_hook=foreign/skipped motion_kind=foreign/skipped
    input_delay=installed lra_reset=installed stage_hazards=installed reset_hub=installed cpu_core=installed cpu_defence=installed
    cpu_ledge=not_installed reason=site:transition_term_hook cpu_tech=installed cpu_shield=installed
    cpu_mash=not_installed reason=site:motion_kind cpu_common=not_installed reason=site:motion_kind
    state_core=installed chords=installed recording=not_installed reason=site:transition_term_hook notices=installed ui=installed   result=installed heap_allocations=0

These are the review's two counterexamples: CpuMash (and CpuCommon) are off because `motion_kind` was rejected, and CpuLedge, the recorder and the common-module hooks
(which call `is_enable_transition_term`) are off because the ledge row was rejected. The game then ran in Training: the CPU core read the CPU body's role (`cpu_event
role entry=1`), the save chord saved and the load chord restored a state (P1 x -45.0 -> -45.0), the record chord started nothing (the recorder is dormant), no Frame
Advantage notice appeared, a Mash=Jump/Always setting changed nothing, and the diag clock kept advancing. No crash, no Unmapped line.

## r2-02: clean boot (no poison file)

Every feature of the closure installed (`cpu_common=not_installed reason=no_nro_hook` only: the profile has no `libnro_hook` provider), `transition_term_hook=stock/far`,
`motion_kind=stock/checked`: nothing is over-disabled. Save/load chords, a recorded slot (`recording_complete slot=0 frames=12`) and the Frame Advantage notice
(`notice_draw header="Frame Advantage" body="-16"`) work.

## R2-8 on Eden: what could and could not be shown

The plugin reads `sd:/ultimate/TrainingModpack` through the Switch file API. Eden's emulated file system answers **both** fixtures with "nothing there": a host folder with
mode 000 (r2-01: `macros` listed as empty, `setups/a.json` and `mii-presets.json` imported, `migration=done`, marker written) and a regular file where `macros` should be
(r2-02: `migration=done`, marker written, no `cannot list` line). So no host fixture makes the import meet a non-NotFound error under Eden, and the rows 3-5 of
`r2-01/rows.txt` that predicted the partial result are recorded as FAIL (the prediction, not the plugin, was wrong); `rows_r2.py` now defaults to `probe` rows. The
behaviour itself (enumeration, entry, type and metadata errors, an unreadable settings file and a non-folder source are `unlisted`, the result is partial, other paths are
imported, no `migration.json`, the next boot completes it and keeps what was copied) is proven on real directories by `tests/migration_run.rs`
(`migration_r2_8_*`, six tests, including a permission-denied folder with its premise checked and an ELOOP metadata failure). A real card or console that reports such an
error (device error, damaged FAT) takes the same path; this is an Eden limitation, not shown on hardware.
