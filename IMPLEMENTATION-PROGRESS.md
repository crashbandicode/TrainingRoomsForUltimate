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
- M4: runtime boot, the three hooks and the reset hub; **Switch-compiled 2026-11-02** on the Surface (below). Not run on Eden or a Switch.
- Switch build: `cargo-skyline skyline build --release` and `… --features diag` succeed with the campaign nightly launcher (`RUSTUP_TOOLCHAIN=nightly-2026-02-14-x86_64-unknown-linux-gnu`, `RUSTUP_HOME`/`CARGO_HOME` under `/home/intpa/smash-training-lobby/toolchain/tools`, skyline-v3 std, `CARGO_TARGET_DIR=/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-work`, flock on the campaign compiler lock). With the repo pin (`1.98.1`, a stable toolchain) cargo-skyline fails: `error: the option `Z` is only accepted on the nightly compiler` (it passes `-Zunstable-options` / `-Z build-std` to the cargo on PATH). `rust-toolchain.toml` was not changed. NROs (gitignored, `dist/`): release sha256 `6d6443849b8c9462821afca1b971b735e03dfd450eb99c37494723362f4988f3` (file 126,976 B, resident 172,032 B), diag `82f60e248115f766cd54f15d936359837a7640f40662051b5e19d207722e2a03` (file 122,880 B, resident 270,336 B), from `tools/build/nro_size.py`.
- Card-site scan (`tools/build/scan-offsets.py`) over the newest local card-plugin copy, the MTP preflight copy `/home/intpa/ssbu-cpu-pilot-install/switch-mtp-v0101-v021/pilot/after/atmosphere/contents/01006A800016E000/romfs/skyline/plugins` (16 plugins, folder dated 2026-11-01 10:19), offsets 1750dd0 1378e50 2311bec 2311be4 17408c4 4f80240 60dfa0 1671650 2140fc0 20c9e30: only `libtraining_modpack.nro` (the fork; B1 says never install both) references the hook sites (all of them); `is_training_mode` (1671650, a called function) is referenced by libelite_stages_counterpick, libsmash_gamepad_overlay, libsmash_replay_to_mp4, libssbu_cpu_pilot, libssbu_online_deluxe_rewrite (movz/movk or constants: calls or hooks; the Called row tolerates hooks anywhere in its 64 bytes). Nobody else references the input mapping, reset check, hazard sites, on-init slot or the two thunks.

## Running / blocked
- Nothing running. Switch build (`cargo skyline build --release`, diag build) is the lead's step on another host.

## Gaps and decisions for the lead
- The Training reset hub is not wired: it needs the fighter on-init wrapper to know the CPU-operated fighter
  (FighterManager and FighterInformation accessors, SPEC-state-01 §3). Until then the input-delay queue is not
  cleared on a reset or save-state load (B8) and the two on-init sites are compared but not hooked.
- Input owners: `published::INPUT_OWNERS` (an atomic of `Owners` bits) is read by the input-delay hook on every poll. Nothing sets a bit yet: the menu (package 5), stop chord and replay editor (4), native UI busy (6) and the game's pause menu each need their own detection facts (open/close state, chord consumption, the pause-menu flag), which are not in package 1's specs.
- Unknown settings rows are kept on rewrite (see `docs/test-provenance/package-1.md` note 1).

## Exact commands
`$env:CARGO_TARGET_DIR='/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-work'`, then, one per call,
`systemd-run --user --scope -p MemoryMax=6G /usr/bin/timeout 1800 cargo test` / `cargo clippy --all-targets -- -D warnings`
/ `cargo build --no-default-features` (PATH needs `/home/intpa/.cargo/bin`).

## Eden fixes D1-D4 (2026-11-02)
New NROs (same launcher): release sha256 `9bad6541a3b1d7497724e6c1db108083017b4a1aef3c9df49ea39a7786cc0ac5` (file 126,976 B, resident 172,032 B), diag `db7a4dd980b17b275ee439e0e644b6090b3ed6aede297f165fd8c1e93423e475` (122,880 B, 270,336 B). They replace the hashes of the first builds above.

## Review round 2 (2026-11-02)
- Input Delay hooks left the function entry (cfd6f4b): inline at main+0x1750dfc (`mov x19, x2`, after the prologue and the player-index checks) and main+0x1751ce8 (the function's one epilogue, after the call that completes the record). **Finding:** the CPU pilot plugin hooks the same two instructions (its `fim_valid` 0x1750df0 and `fim_exit` 0x1751ce8), so on a card with the pilot the second installer is refused (W5 in docs/compliance-package-1.md). Needs a lead decision (keep the entry hook, chain, or share).
- is_training_mode (main+0x1671650): no plugin hooks it. elite_stages_counterpick, smash_gamepad_overlay, smash_replay_to_mp4 and online_deluxe_rewrite compare the looked-up symbol with main+0x1671650 and check the function's first words (capstone disassembly of the 2026-11-01 10:19 preflight copy); the pilot keeps a guarded called-function record for it.
- Host tests for the Eden analysis scripts (tests/eden/test_analyze.py, run by cargo test); review/menu-proposal ecb574d merged (59a159b).
- New NROs at this HEAD: release sha256 `e6736713a0c2c8c4f16cede9d5679e399545aa3c6202680ba04898522c2a2f05` (126,976 B file, 172,032 B resident), diag `2b9f0828ecb8ef275aaa732c598278bf06b68b02eabc0327c6a370fe0897f30d` (122,880 B, 270,336 B).
## Input Delay hooks in the callers (2026-11-02, after the pilot collision)
The CPU pilot hooks the final input mapping's inner points (0x1750df0, 0x1751ce8), so cfd6f4b's in-function sites were replaced: Input Delay hooks the function's two callers (the only references to it in the ELF: direct `bl` at 0x17546f4 in the 8-slot loop and 0x1755250 in the single-slot wrapper). `note` hooks at 0x17546e0 and 0x175523c (before each call: out record and slot), `apply` hooks at 0x1754720 and 0x1755254 (after the call returns). No site or guard window is within 0x40 of a pilot site or a card reference. NROs: release `26a5fef0383d2252f44868257bd7e95ccf17c7157b4e1c00a314e377016303f9` (172,032 B resident), diag `4792c91a4f631e797c8fcee41f3629dd5a39c0f0544437fd0fe5ac1d80c9e888` (274,432 B).