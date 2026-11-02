# Building TrainingRooms

2026-11-02, the lead. The recipe is the one the playbook's plugins use on the Yoga (gsp-visualizer,
elite-rematch, ssbu-quiet-elite-switch, ssbu-tile-cursor: each pins `rust-toolchain.toml` to `1.98.1`, has no
`.cargo/config.toml`, and documents plain `cargo skyline build --release`).

## Two toolchains, by design

- **Host gates** (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo build --no-default-features`)
  use the pin in `rust-toolchain.toml`: rustc **1.98.1**.
- **Switch builds** (`cargo skyline build --release`, `cargo skyline build --release --features diag`) do not use
  the pin. cargo-skyline 3.5.0 runs `rustup run skyline-v3 cargo build -Zbuild-std=core,alloc,std,panic_abort` with
  `RUSTFLAGS=--cfg skyline_std_v3` (strings of the binary, identical on both hosts). The `skyline-v3` toolchain is
  rustc 1.95.0-nightly `a423f68a0` (2026-02-13) with the skyline std: on the Yoga
  `~/.rustup/toolchains/skyline-v3` → `~/.cargo/skyline/toolchain/skyline`; on the Surface in the campaign home
  `/home/intpa/smash-training-lobby/toolchain/tools/rustup/toolchains/skyline-v3`.

The rustup home used for a Switch build must contain `skyline-v3`. The Surface's default `~/.rustup` does not
(toolchains: stable, nightly, nightly-2026-02-14, 1.75, 1.98.1), which is why the first Switch build here failed with
"the option `Z` is only accepted on the nightly compiler". No `RUSTUP_TOOLCHAIN` override is needed.

## Commands

Yoga (`memento-claw-yoga-ubuntu`): `cargo skyline build --release`, then `cargo skyline build --release --features diag`.

Surface: the same two commands with
`RUSTUP_HOME=/home/intpa/smash-training-lobby/toolchain/tools/rustup`,
`CARGO_HOME=/home/intpa/smash-training-lobby/toolchain/tools/cargo`,
cargo-skyline from `/home/intpa/smash-training-lobby/toolchain/tools/skyline-install/bin`, and this copy's own
`CARGO_TARGET_DIR`; each call under `systemd-run --user --scope -p MemoryMax=6G /usr/bin/timeout 1800`.

## Evidence

2026-11-02: a detached clean checkout of `e8dbcc9`, pin unchanged, no `RUSTUP_TOOLCHAIN`, the Surface command above
(script `/tmp/tr-pin-probe.ps1`): `Finished release … in 16.54s`, `libtraining_rooms.nro` sha256
`26a5fef0383d2252f44868257bd7e95ccf17c7157b4e1c00a314e377016303f9` — byte-identical to the release NRO built
earlier with a `RUSTUP_TOOLCHAIN=nightly-2026-02-14` override (`IMPLEMENTATION-PROGRESS.md`), so the build is
reproducible and the override is not needed. Resident size: `nro_size.py` → 172,032 B.
