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
