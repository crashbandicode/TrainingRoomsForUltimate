# Package 3 (save states) progress

Branch `stream/pkg3`, worktree `TrainingRoomsForUltimate-pkg3`. Not pushed. Updated 2026-11-02.

## Milestones

| Milestone | State | Evidence |
| --- | --- | --- |
| S3-1 save/load core, JSON saves, setup library | done; Eden pass (s3-01) | `docs/test-provenance/package-3.md`, `eden/EDEN-PACKAGE-3.md` |
| S3-2 charges, buffs | host pass; Eden: Samus charge and Bob-omb snapshot (s3-02); Cloud buff (s3-03) queued | same |
| S3-3 items, fighter-specific restore (Steve, Olimar, Sora, Pokemon Trainer, Kirby) | host pass; Switch-compiled; Steve Eden pass; the rest queued for s3-03 | same |
| S3-5 first-boot import (SPEC-core-08) | host pass (13 filesystem tests); Eden s3-04 queued | same |
| S3-4 Switch builds, batched Eden rows, compliance | builds done; Eden s3-03 and s3-04 wait for the Yoga queue; compliance run after them | below |

## State of the code

- The model (`src/model/state/`) is pure and host-tested: records and JSON (fork order, absent options as `null`,
  new optional `copy_charge` only when present), the load machine, placement rules, charges (own table and
  Kirby's copied table), buffs, items, Steve, Olimar line, Pokemon Trainer decide and wait, Kirby acceptance,
  setup library, N6 conversion.
- The runtime (`src/runtime/features/states.rs`, `state_effects.rs`, `state_restore.rs`, `state_recorder.rs`,
  `import.rs`) calls the game only through guarded site rows (`src/model/sites_pkg3.rs`, generated from the owned
  ELF by `tools/gen_sites_pkg3.py`): 114 rows, 31 replace hooks, one inline hook (Pokemon decide).
- Persistence follows review R1 (atomic replace, copy-aside of a corrupt file, fs-backed two-boot tests).
- Hooks allocate nothing; the diag build logs `state_hook_counts` (now with `se_silenced`, `se_passed`,
  `switch_fx`, `decide`, `copy_hook`).

## Builds (Yoga, `cargo skyline build --release`)

Built at 4fdf560 in `/home/intpa/trainingrooms-build-pkg3/dist/{release,diag}`:

| Build | sha256 | Resident |
| --- | --- | --- |
| release | `30ae799adf2167d583e7136b1de90363eda93594c3c83b1311e4c93b735c52c4` | 421888 B (412 KiB) |
| diag | `e4912885c2cd7fa6c9eb8b4eebdd38a5683c1bab9369e889d2eceb07799755ed` | 548864 B (536 KiB) |

Gates at 4fdf560: `cargo test --no-fail-fast` all pass (29 binaries); `cargo clippy --all-targets -- -D warnings`
and `cargo build --no-default-features` clean (run on the Yoga); runtime code is `cfg(target_os = "switch")`
and is compiled only by the Switch build (four dead-code warnings for `request_save`, `request_load`,
`is_loading`, `is_killing`, which package 4's chord engine consumes).

## Known gaps and decisions

- Chord engine and the recorder are package 4's: `states::request_save/load` and `state_recorder` adapters log
  only until the merge.
- Kirby's copied-kind source is `INT_COPY_CHARA`; the diag dump compares the module offset `+0x17398`.
- The decide descriptor layout is a native check; a rejection in normal play is a defect to root-cause.
- Kirby and Sora silences apply only while loading (spec open question 4).
- Held-item capture limits, the setup-library UI glue (packages 4 and 6), and Eden rows for fighters other than
  Steve, Samus, Cloud, Pokemon Trainer and Kirby are not run.
