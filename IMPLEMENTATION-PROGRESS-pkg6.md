# Implementation progress, package 6 (stream `pkg6`)

Branch `stream/pkg6`, worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-pkg6`, base `473da26`, merged
`work/implementation` `fa8b77d` (`5f98056`). Target `/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-pkg6`,
Yoga folder `/home/intpa/trainingrooms-build-pkg6`, Eden labels `s6-NN`. Claude Sonnet 5.5 high (hidden watcher in the
native temp dir, bound to the harness PID; model gate from the session transcript). Playbook followed: `009c9ac`.
Stream rules: `/home/intpa/smash-training-lobby/TRAININGROOMS-STREAM-RULES.md`.

## Done
- S6-0: `docs/PACKAGE-6-PLAN.md`; generated inventory (`docs/provenance/pkg6-inventory.{json,md}`), dependency census
  (`pkg6-deps*.json`, `pkg6-edges.json`), carry layers (`pkg6-layers.md`) by `tools/pkg6_inventory.py` and
  `tools/pkg6_deps.py` (`python3 -I tools/pkg6_inventory.py . m5o`, then `pkg6_deps.py . m5o`).

## Running / blocked
- Nothing running. Decisions D1-D10 of the plan wait for the lead; C0/C1 can start on D2/D3 (module layout and
  dependencies).

## Next
- S6-1: `tools/carry_pkg6.py` + manifest, `model::compat`, seam guards (C1), pure halves of cycle preload / Kirby hats /
  character items (C2); then C3-C5.
## S6-1 tranche 1 (2026-11-02)
- Merged `work/implementation` `fa8b77d` (`5f98056`). Lead rulings D1-D11 recorded in the plan section 12.
- `tools/carry_pkg6.py` (carry / verify / ledger) and manifest; 22 files carried (seam table + unlink guards, source lifetime and load,
  roster grid, preload grace and notice queue) under `src/model/{seams,swap,preload}`, `cfg(test)`-only until their consumers land.
- Cargo features: the fork's `[features]` table (lead-authorised read of lines 82-122 and 124-240 (line 123 not printed), disclosed in CARRY-OVER.md), `default` = the M5o release list.
- Gates (host, Surface, target `cargo-targets/trainingrooms-pkg6`): `cargo test` all binaries pass (the carried unit tests run in every by-path test binary),
  `cargo clippy --all-targets -- -D warnings` clean, `cargo build --no-default-features` builds. Switch builds: not run for this tranche (no runtime change; resident
  size unchanged at the package-2 value because the carried areas are test-only).
- Not blocked. Next: C2 remainder, C3 (D3 dependency checks first: serde derive, byteflags), then C4/C5; `src/runtime/scene.rs` (D4) before C7.
