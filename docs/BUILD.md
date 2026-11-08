# Building TrainingRooms

2026-11-02, the lead. The recipe is the one the playbook's plugins use on the Yoga (gsp-visualizer,
elite-rematch, ssbu-quiet-elite-switch, ssbu-tile-cursor: each pins `rust-toolchain.toml` to `1.98.1`, has no
`.cargo/config.toml`, and documents plain `cargo skyline build --release`).

## Two toolchains, by design

- **Host gates** (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo build --no-default-features`)
  use the pin in `rust-toolchain.toml`: rustc **1.98.1**.
- **Vendored crates' own tests** (2026-11-02): the root package has no workspace, so `cargo test` does not run
  `crates/*/tests`. Each gate run adds one call per crate: `cargo test --manifest-path crates/smash-match-data/Cargo.toml`
  (its tests use the test-only oracle crate `crates/smash-match-data/tests/oracle`) and
  `cargo test --manifest-path crates/hos-lock/Cargo.toml`; same target dir, `CARGO_INCREMENTAL=0`.
- **Size gate** (2026-11-02): `python3 -I tools/size_gate.py` prints the implementation lines per module (the
  primary's sloc4 counter, `tools/sloc.py`: no blanks, comments or tests, test modules and simulator code apart; `third_party/` excluded) and fails above
  `size-budget.toml`, or when that file records a raise without the lead's approval. `cargo test` runs it
  (`tests/size_budget.rs`, which needs `/usr/bin/python3` 3.11+ and has no skip switch). The budget only ratchets
  down; the plan is `docs/SIZE-PLAN.md`.
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
earlier with a `RUSTUP_TOOLCHAIN=nightly-2026-02-14` override (`IMPLEMENTATION-PROGRESS.md`) (removed after v0.2.0-rc.1; see that tag), so the build is
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

The waiting-room host uses the same pane table and builder:

    layout-builder build-waiting <stock info_waiting.arc> <out dir>
    cp <out dir>/info_waiting.tra1 assets/info_waiting.tra1
    cp <out dir>/panes-recipe.sha256 assets/panes-recipe.sha256              # the same digest from either build

**The recipes follow the pane table (guard, 2026-11-04).** Each build also writes `panes-recipe.sha256` (and
`pane_table_sha256` in `report.json`): the SHA-256 of `tools/layout-builder/src/pane_table.rs`'s `BUILDER` line
(the builder version and the pinned nx-layout-toolbox revision) and every `panes::specs()` entry. The committed copy
`assets/panes-recipe.sha256` is checked by `tests/pane_recipe_guard.rs`: any change to the pane table without a rebuild
of both recipes fails `cargo test`, with these commands in the message. 2026-11-04 the badge port changed the pane
table without a rebuild, and the candidate's menu never opened (`menu_panes=665 missing=354`). Bump `BUILDER` with
any builder change that alters the recipes for the same table.

Current recipes (rebuilt 2026-11-08 from the pane table with the r8 recording pop-up: 12 sublines and the CONNECTIONS column, 28 panes; its CONNECTIONS boxes, pills and tree and the L/R role tabs, 50 panes; the section headers' squares, the order badges and the boxed page tabs, 26 panes; layout-builder 0.1.2; the stocks are unchanged). The parent recipes are not reused: `tools/layout-builder` wrote these bytes from `panes::specs()`.

| File | SHA-256 | Bytes | Stock archive it applies to |
| --- | --- | ---: | --- |
| `assets/info_training.tra1` | `8e687f08908244968634b3e13adbae92ccc6ba8e25063231ad005d5f8b3ebd73` | 370,381 | `info_training.arc` 451,604 B, SHA-256 `90a8dc81751efa9d4521f836be9ccf8a134cb24af9fb5b195211e58203f9317c`, FNV-1a `8833748e9518a461` (`runtime/ui/probe.rs:91`); target 938,900 B, fnv `8fd86329c0c8f21a` |
| `assets/info_waiting.tra1` | `747593d08529f24c74741f7e7a0e9894f1078c7fdf331bb45f91775b672aec53` | 374,419 | `info_waiting.arc` 1,284,116 B, SHA-256 `7dcc45360bd84dadffcfe8b47771f5d6b40e44468c6a54cbd3dbb6b4142eea64`, fnv `f3d2cb3ec6a9f0de`; target 1,767,300 B, fnv `97f6778cbd14bf8f` |
| `assets/panes-recipe.sha256` | `2311dca6de1ee003818c146f0896d472a4edc2162062545c32cb21b7cf9edcf2` (the digest) | | |

Both recipes are embedded in every NRO (`runtime/ui/archive.rs`), so their size is resident size too (the 354 roster
mark panes added about 33 KB to each).

Its menu, notification and input-log roots live beneath `TrModPort`. The runtime
recognizes each archive by its directory member and applies that layout's recipe.
The two boot buffers each fit the larger recipe's stock/output size; both layouts
share the pool and retain the five-second hand-out grace. A vanilla fallback uses
the second buffer as scratch, so reload timing remains an Eden acceptance gate.

Historical (superseded by the table above): the waiting recipe candidate generated with builder `44177d8`: stock 1,284,116 B,
SHA256 `7dcc45360bd84dadffcfe8b47771f5d6b40e44468c6a54cbd3dbb6b4142eea64`;
recipe 65,784 B, SHA256 `599c7d6fe61568efae283356e237eac05784f817c8b2c02db12682b873a35261`;
output 1,378,180 B. All 183 non-layout archive members and all 63 original pane
records remain byte-identical. The builder adds 549 panes including the host.
Raw generation and independent payload/hierarchy checks are retained privately
under `eden/runs/ui-waiting-build-44177d8/`; this is build evidence, not runtime
acceptance. Never commit `target.arc` or the stock archive.

The tool uses the operator's own MIT crate **nx-layout-toolbox** as a git dependency, host only (it is not linked into the plugin):

- origin `https://github.com/crashbandicode/Toolbox-Cli` (crate `nx-layout-toolbox` 0.1.0, MIT, `LICENSE` in the repository); no Forgejo mirror exists (`git.syntaxbyexample.com/intpa/Toolbox-Cli` is not found)
- pinned revision `9010ca890b6339346758fa68bb53075d24aa0f95` (2026-06-01, "Document MeshCodec mesh-framing RE"), in `tools/layout-builder/Cargo.toml` and `Cargo.lock`; the Yoga copy `/home/intpa/toolbox-cli-aux/Toolbox-Cli` used for the first builds is this revision (identical `src/` apart from CRLF line endings)
- reproducible from a clean checkout: 2026-11-02 the recipe built from the git dependency is byte-identical to the committed `assets/info_training.tra1` (sha256 `a1f3ee13...`, target archive fnv `e8a5a9ce3f4ec968`; rebuilt 2026-11-02 after the chip-grid geometry change, in the Ally WSL with `eden/ally/ally-build.sh recipe`: sha256 `225fdfd0...`, target archive fnv `6ba2f66617e6c4ea`, 529,300 B) given the same stock archive.

Item pictures: the builder reads the 38 approved PNGs from assets/item-icons, appends BC7 sRGB textures and corresponding picture materials to the stock combined BNTX, and emits the same TRA1 delta format. Verify source/destination provenance with `python3 -I tools/carry_ui_items.py`; the 39-file identity ledger is docs/provenance/ui-item-assets-ledger.md. Regenerate both recipes whenever the shared pane table changes. The generated target archives stay private.
