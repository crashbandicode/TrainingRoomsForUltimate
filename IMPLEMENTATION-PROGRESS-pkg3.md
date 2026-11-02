# Package 3 (save states) progress

Branch `stream/pkg3`, worktree `TrainingRoomsForUltimate-pkg3`. Not pushed. Updated 2026-11-02.

## Milestones

| Milestone | State | Evidence |
| --- | --- | --- |
| S3-1 save/load core, JSON saves, setup library | done; Eden pass (s3-01) | `docs/test-provenance/package-3.md`, `eden/EDEN-PACKAGE-3.md` |
| S3-2 charges, buffs | host pass; Eden: Samus charge and Bob-omb snapshot (s3-02), Cloud Limit buff (s3-04, after the s3-03 crash fix) | same |
| S3-3 items, fighter-specific restore (Steve, Olimar, Sora, Pokemon Trainer, Kirby) | host pass; Eden pass: Steve (s3-01), Pokemon Trainer (s3-05, s3-07), Kirby hat (s3-06); Olimar and Sora not Eden-run | same |
| S3-5 first-boot import (SPEC-core-08) | host pass (13 filesystem tests); Eden s3-08 waits for the Yoga (a foreign M5q Eden holds it) | same |
| S3-4 Switch builds, batched Eden rows, compliance | builds done; Eden s3-03 to s3-07 run; s3-08 pending; compliance run after it | below |

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

Built at 13be87c in `/home/intpa/trainingrooms-build-pkg3/dist/{release,diag}`:

| Build | sha256 | Resident |
| --- | --- | --- |
| release | `aefbcad56be10d8fa16c39b3ae2be7e9a2a874b3984235f1788489d4a2685948` | 421888 B (412 KiB) |
| diag | `e39444a8d39d98f0942bcacfa3f401e5cd8a93370bcf78a0f283d28f5c4a90f2` | 548864 B (536 KiB) |

Gates (host tests, clippy, no-default-features were green at 4fdf560; the later commits changed one native
argument and removed a `Vec`, and 13be87c was re-run through the host tests locally before the Yoga build): `cargo test --no-fail-fast` all pass (29 binaries); `cargo clippy --all-targets -- -D warnings`
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
