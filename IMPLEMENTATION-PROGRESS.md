# Implementation progress (package 1)

Branch `work/implementation`, worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-work`.
Implementer: Claude Sonnet 5.5 (high), session transcript `agent-a16caf62dc2a0d371` under lead `0caa72b8`.
Playbook followed: `53fd1f2`. Host only; no Eden, no Switch, no push.

## Done
- M1 skeleton: crate (`cdylib` only, profile and toolchain per playbook), `model` / `runtime` split,
  carried version guard, diag ring, 64-byte guard rules, site table (8 sites, bytes checked against the owned
  ELF), install-line formatter, storage paths, `developer.ini`, counting allocator and diag log (runtime).
- M2 settings: `settings.json` reader/writer (S1 name, S2 policy, duplicate keys, unknown rows kept), tested on
  two real fork files.
- M3 pure models and tests for Input Delay, Stage Hazards, L+R+A Reset.
- SPEC-core-08 plan (`model/migration.rs`, pure): copy list, marker-only rerun rule (a pre-installed `developer.ini` does not block), N8 split. Copy loop, sha256 receipt and N6 conversion are not done.
- Merged `review/menu-proposal` (5e34c82) after the first commits; specs re-read: retired key `character_item`, draft 9 fields (kept as unknown rows until their packages), S2 name `settings.invalid-<n>.json` already as implemented.
- M4 (partly): runtime boot and the three hooks written, **not Switch-compiled**.

## Running / blocked
- Nothing running. Switch build (`cargo skyline build --release`, diag build) is the lead's step on another host.

## Gaps and decisions for the lead
- The Training reset hub is not wired: it needs the fighter on-init wrapper to know the CPU-operated fighter
  (FighterManager and FighterInformation accessors, SPEC-state-01 §3). Until then the input-delay queue is not
  cleared on a reset or save-state load (B8) and the two on-init sites are compared but not hooked.
- Input owners other than gameplay (menu, stop chord, native UI, replay editor, vanilla pause) are not wired
  (`Owners::NONE`): they belong to packages 4-5.
- Unknown settings rows are kept on rewrite (see `docs/test-provenance/package-1.md` note 1).

## Exact commands
`$env:CARGO_TARGET_DIR='/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-work'`, then, one per call,
`systemd-run --user --scope -p MemoryMax=6G /usr/bin/timeout 1800 cargo test` / `cargo clippy --all-targets -- -D warnings`
/ `cargo build --no-default-features` (PATH needs `/home/intpa/.cargo/bin`).
