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

## Layout archive recipe (package 5, host tool)

2026-11-02. `assets/info_training.tra1` is built, not hand-written: `tools/layout-builder` reads the player's own stock
`info_training` layout archive (451,604 B on 13.0.5; dumped by the **diag** build into
`sd:/ultimate/TrainingRooms/stock/`, copied under the gitignored `eden/runs/<label>/` or a scratch folder; game data is
never committed), adds the plugin's panes (`src/model/ui/panes.rs`, the one pane table the runtime also reads) to its
`info_training.bflyt` and writes a `TRA1` recipe: copy ranges of the stock archive plus the plugin's own bytes. The
committed recipe holds no complete game archive; the plugin applies it to the bytes the game just decompressed and refuses
an archive whose length and FNV-1a fingerprint differ (`model::archive_recipe`).

    cargo build --release --manifest-path tools/layout-builder/Cargo.toml     # own CARGO_TARGET_DIR
    layout-builder build <stock info_training.arc> <out dir>                  # target.arc (private), info_training.tra1, report.json
    cp <out dir>/info_training.tra1 assets/info_training.tra1

The tool uses the operator's own MIT crate **nx-layout-toolbox** as a git dependency, host only (it is not linked into the plugin):

- origin `https://github.com/crashbandicode/Toolbox-Cli` (crate `nx-layout-toolbox` 0.1.0, MIT, `LICENSE` in the repository); no Forgejo mirror exists (`git.syntaxbyexample.com/intpa/Toolbox-Cli` is not found)
- pinned revision `9010ca890b6339346758fa68bb53075d24aa0f95` (2026-06-01, "Document MeshCodec mesh-framing RE"), in `tools/layout-builder/Cargo.toml` and `Cargo.lock`; the Yoga copy `/home/intpa/toolbox-cli-aux/Toolbox-Cli` used for the first builds is this revision (identical `src/` apart from CRLF line endings)
- reproducible from a clean checkout: 2026-11-02 the recipe built from the git dependency is byte-identical to the committed `assets/info_training.tra1` (sha256 `a1f3ee13...`, target archive fnv `e8a5a9ce3f4ec968`; rebuilt 2026-11-02 after the chip-grid geometry change, in the Ally WSL with `eden/ally/ally-build.sh recipe`: sha256 `225fdfd0...`, target archive fnv `6ba2f66617e6c4ea`, 529,300 B) given the same stock archive.
