# Carry-over ledger (package 1)

## RC9 Kirby copy cleanup object identity — 2026-11-03

Operator approved the HATS defect fix at 23:50Z. The carried
`character_sources::clear_hat_for_switch` resolves the Fighter through its
entry's existing guarded active-cell reader and verifies accessor plus object
id. The old clean-room native wrapper incorrectly treated a boolean activity
predicate as an object pointer (lr14 captured return x0=1). Full native cleanup,
copy-group checks and lease pin/settle/release order are unchanged. Exact carry
transform and regenerated destination hash/ledger preserve reproducibility;
no fork/stage edit. Astra APPROVE-WITH-CHANGES applied: reject layout failure
before lookup/read, unsafe game-thread NonNull Fighter (no static reference),
retire the unused predicate row/constants/data-slot exemption and CALLED_ROWS
dependency, update adapter/stale spec contracts, and add strict resolver/identity
cleanup-call spies. Generic cells 1/2 are not a claim about Kirby's cell0 layout.
Build/review receipts: `docs/RC9-HATS.md`.

## RC7 shared retirement adapter consumption — 2026-11-03

UI consumes ONLY `src/model/lobby/practice_policy.rs` and its exact carry entry
from authorized pkg6 `185e4c8`; both byte/entry identity verified, source SHA256
`9c69bd57bf9ba8dc9419b724d66ded157f275e96bf4334c6abe4aca055c64fcc`.
The shared `depart_session` protocol retires the lease before unconditional
consumer cleanup. UI adds pause-consumer tests, not another producer or native
adapter. Pkg6 owns scene/P1 wiring in `practice.rs`/`training/mod.rs`; lead must
compose that commit. Full247destinations re-derived successfully. No fork edits.

## RC reset option 2 — 2026-11-03

Lead10:42:04 authorized retirement of obsolete optional source-owner, Mii
origin/pool, fixed-body-bank and automation-results observations, plus the
Hitbox diagnostic-only correlation trio. Six carried files (including the
exclusive pool policy) moved from `files` to `not_carried` in the existing
pkg6 provenance manifest; nine surviving carried files have exact appended
transforms. `tools/carry_pkg6.py carry` and `ledger` regenerated the sources,
destination hashes and ledger; full `verify` re-derived 247 files from the
already-approved ours-new M5q source set. This is product carry metadata, not
a new scratch receipt/hash manifest. No new fork implementation was inspected.

Retired caller/declaration/feature/exclusion/seam closures are part of the same
slice. Sixteen Mii observer seams, four results-observer seams, nine Hitbox
observer seams and ten exclusive source-owner call rows are removed; rows
shared with product native banks/Mii remain. `Feature::Probes` is a reserved
id, not a live probe, preserving later feature discriminants. Continuous and
swap/hang traces, unlink guard/probe, pilot product alias, CountingAllocator,
HookScope, stack/headroom and refusal diagnostics are retained. Product Hitbox
requests, parameters, handles, tint/rate and feature controls are unchanged.

New/adjusted host regressions pin retired feature/file absence, retained tools,
product-only call rows and the empty obsolete-exclusion set. They have not run
on this scratch tree: the lead's coherent-candidate host gates precede Eden.
Historical observations remain in git and the existing raw artifact archives.
No product parity or final gate PASS is inferred from retirement/provenance.

Every file or part of a file taken from the earlier fork. Source: stage **m5o** (= C116.2 as shipped),
`/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/stages/m5o/stage`
(called `<stage>` below). Only files whose `origin` is `ours-new` in `docs/provenance/provenance-files.json`
were opened, plus the one mixed file `src/logging.rs`, of which only lines 20-284 were read (by line range).
Nothing under `upstream-ref/` was opened. The stage and the fork worktree were not edited.

"Clean-room change" is any edit made here to a carried file, for any reason, so the lead can put it in
the two-way ledger (fork fixes must land here, clean-room fixes are offered back).

| Here | Fork path (stage m5o) | sha256 of the stage file | Taken | Clean-room changes (reason) |
| --- | --- | --- | --- | --- |
| `src/model/version.rs` | `src/model.rs` (ours-new) | `6bd383566bc0ff79c7cbfe8a9ebf7e0138225344fa9f4e9757130656d138c44d` | whole file | (1) removed `pub use training_mod_consts::{body_bank, character_forms, native_seams};` and its doc: those crates are not part of this plugin yet and the re-export was only a convenience for the fork's runtime. (2) module doc comment rewritten to point here. No logic changed. |
| `src/model/diag_ring.rs` | `src/diag_ring.rs` (ours-new) | `af22459b85e194252cef240bcdacf77c4616844af01421ed8f39d7e5610eda50` | whole file | (1) added `"[TrainingRooms]"` to `SHIPPED_TAGS` (sorted position): the shipped build must admit this plugin's own boot and settings breadcrumbs. (2) module doc paragraph updated (file names). The fork's feature tags stay listed. |
| `tests/carried_model_facade.rs` | `training_mod_consts/tests/model_facade.rs` (ours-new) | `dd6f2e8e3b38efe6081ac61d8608ddba4afeca1588cee97e4c580bef776d5004` | whole file | `#[path]` now `../src/model/version.rs`; removed `the_seam_table_was_read_from_the_supported_game` (it tested the fork's `native_seams` table, which is not carried); owned-ELF path is `<worktree>/../research/...` (one level, not two). |
| `tests/carried_diag_ring.rs` | `training_mod_consts/tests/diag_ring.rs` (ours-new) | `6982645661a70609d7e1e246f045367d59dc76e39c1921c5ec8cb7c4eb20504e` | whole file | `#[path]` now `../src/model/diag_ring.rs`; doc comment. |
| `src/model/guard.rs` | `training_mod_consts/src/native_seams/mod.rs` (ours-new) | `9c16e57af94e9f6424c92a5bd3de8df64508eb3cda01ff7c4eb9c1054c8338e3` | lines 38-130 (constants, `HookForm`, `Placement`, `SeamKind`, `Sharing`), 546-676 (`far_jump_at`, `outcome_for`, `entry_or_body_outcome`, `EntryForm`, `entry_form`, `rest_from`, `entry_far_jump_target`, `outcome_for_own`), 713-719 (`exact_outcome`), 736-837 (`GuardOutcome`, word constants, `words`, `hook_shape`, `blocks`, `guard_outcome`) | the six word constants (`WORDS`, `NOP`, `LDR_X17_LITERAL_8`, `BR_X17`, `B_MASK`, `B_OPCODE`) and `words` were private; made `pub` so the by-path host tests (which cannot see private items of the included module) can use them. New module doc. The fork's `Seam`/`Site` tables, rows, `Scopes` and image rows are not carried. |
| `tests/carried_guard.rs` | `training_mod_consts/src/native_seams/tests.rs` (ours-new) | `fd7c889851be63354f4db71a22b2cfe61024eec5268f2e4dcc455b92ffc5c163` | lines 7-96, 343-373, 401-445 | `use super::*` replaced by a by-path include and `use guard::*`. The fork's table-wide tests are not carried. |
| `src/model/sites.rs` (data) | `training_mod_consts/src/native_seams/rows_a3.rs`, `rows_a5.rs` (ours-new) | `78b74b30208d4d09007bedb5f5ae50ed1dcc4531ffca39700e28bb14bc233925`, `840976a5e6ed534d82850aba0c5fd891d34339c1474e02fced838276bd432a33` | offsets and 64 guard bytes of seven rows: `needle.fim`, `needle.training_reset_check`, `hazard.hook_window`, `hazard.intercept`, `hazard.flag_scan`, `training.vtable_on_init_slot`, `needle.fighter_vtable_on_init` | Re-keyed to this plugin's names and `Site` struct. The eighth row (`is_training_mode`, `main+0x1671650`) has no fork row (the fork's table lists it as `external`); its 64 bytes were read from the owned 13.0.5 ELF at the game's symbol `_ZN3app9smashball16is_training_modeEv` (symbol table of `research/game-13.0.5/main-13.0.5.elf`). `tests/core01.rs::core01_sites_match_owned_elf` re-checks every compared row against that ELF (all pass). |
| `src/runtime/diag_log.rs` | `src/logging.rs` (**mixed**; only the fork-only lines 20-284, extracted with `Get-Content … \| Select-Object -Skip 19 -First 265`, the rest of the file not viewed) | `c63f92841b860b9758203f14e2c46e8dc0addec453d9eeb34d571e5002f5ba33` (whole m5o file; the m5i hash differs because the file is mixed) | lines 20-284 | `pub use log::…` and the `log` imports dropped (no `log` crate here); paths come from `model::storage` (`TrainingRooms`, not `TrainingModpack`); `training_mod_consts::TRAINING_MODPACK_ROOT` -> `storage::ROOT`; the rotation test is `storage::should_rotate`; a comment that named the fork's `hang_trace` now names `start_drain_thread`. The range ends inside `append`: the last lines of `append`, `sync`, `report_drops` and `start_drain_thread` are new code written here from SPEC-core-01 §7. |
| `tests/fixtures/fork-menu-switch-before21.json` | `artifacts/switch-before21/ultimate/TrainingModpack/training_modpack_menu.conf` (a file the fork wrote on the operator's Switch; not in the stage) | `9f8598346282292407161bce7efb1a9fcfb91e2baf3d0c1b6e3a18022f661630` | whole file | none (byte copy) |
| `tests/fixtures/fork-menu-console-crash78.json` | `artifacts/enhancements/console-crash78/logs/TrainingModpack/training_modpack_menu.conf` | `405e37e3789df9f81d137494d128d538545f67e4b285878a6350b94a2f8aaefa` | whole file | none (byte copy) |

Not carried yet (looked at, left for the package that needs it): `src/hazard_manager/policy.rs`
(ours-new, sha256 `8a378ab2499cab66…`, 38 lines): `waiting_hazards` depends on the fork's lobby-practice
scene model (`training_mod_consts::mii::backend::Layout`), which is package 6; package 1's Training-only
hazard rule does not need it. `src/common/game_mode.rs` (ours-new): lobby practice, package 6.
`training_mod_consts/src/menu_json.rs` was read only to learn the settings envelope; it is bound to the
fork's menu structs and is not carried (the settings reader here is new code that follows SPEC-core-04).

## Findings against the specs (for the lead)

- SPEC-core-08 (still so in 5e34c82, §4 first row and §10 item 1) names `training_modpack.json` as the fork's
  settings file. In the fork that file is the updater record (`{"update":{"last_update_version":…}}`, see the
  carried fixtures `…-training_modpack.json`); the settings file is `training_modpack_menu.conf`.
  `storage::file::LEGACY_SETTINGS` uses the real name.

## Read for setting ids only, nothing carried (menu model, 2026-11-02)

The menu model's factory table (`src/model/factory.rs`) needed the fork's own setting ids and option
counts for the rows the fork added. These ours-new files of stage **m5o** were searched for quoted ids and
option lists; no line of them was copied and no file was edited (the ids and counts are facts of the file
format that real fork settings files already carry):

| Fork path (stage m5o) | sha256 | Used for |
| --- | --- | --- |
| `training_mod_consts/src/menu_json.rs` | `b73c5765a3bb3c654a840de212f2c30ce03800814dbf8169942357395a0e6c90` | `trio_practice` is Player Swapping (a test comment), `fighter_prompt`, `macro_source`, `training_stage_rotation`, `lobby_stages`, `trio_practice` |
| `training_mod_consts/src/cycle_preload_config.rs` | `49c8a777d9e7c728aa8ecc7a51df2f7990f3edf458fdf7163be3b82fa9e3bf01` | the `cycle_preload_*` ids; Cycle Preload Off, 1-5; Sources Fighters / Hats / Both; Bodies Off, 2, 3; Ready Fighters 2-12 default 8; the order of the group switches; Preload Samples is `cycle_preload_measurements` |
| `training_mod_consts/src/p1_cpu_menu_tests.rs` | `591d885b1d0b72215fa20e5a929d8440cd2ce35fb3e2e9525f6c0c8f83842634` | `p1_cpu_control`, `p1_cpu_level` |
| `training_mod_consts/src/roster_menu_tests.rs` | `3d0e1477d2513dca2a1fcb279a51c018637245b349ab4a7a08aa41036d9ad56b` | `player_kirby_hats`, `solo_popo_original`, `player_roster`, `cpu_roster` (kept as unknown keys until package 6) |
| `training_mod_consts/src/mii_menu_tests.rs`, `training_mod_tui/src/mii_menu.rs` | `154ece2b…`, `94d7a41f…` | `lobby_practice`, `mii_brawler_config`, `mii_sword_config`, `mii_gunner_config` |
| `training_mod_consts/src/character_items.rs` | `57640dd2a5a9ea201d3a588e05bb950f5f7cecb551d602acfeeb44d0b5a3c2bf` | `character_item_grant` (the give-item row's key) |
| `training_mod_consts/src/lobby_stages.rs`, `training_mod_tui/src/stage_menu.rs` | `86d084f7…`, `7dece73f…` | `lobby_stages` key only |

**Disclosure.** One search for "Player Swapping" ran over whole source directories of the stage and printed
three lines of **mixed** files (`training_mod_tui/src/containers/app.rs`: the name of a
`player_swapping_enabled` method and a call of it; `training_mod_consts/src/lib.rs`: the string
`"Player Swapping"`), besides ours-new docs. Nothing from them was used: the id came from `menu_json.rs`
above, and the row title is the one already in MENU-DRAFT. The lead should decide whether that counts as
opening a mixed file.
## pkg2

Nothing was carried from the fork: package 2 is clean-room code written from `specs/` and the repo docs. No file
of the stage or the fork worktree was opened, no search ran over the stage, and nothing under `upstream-ref/` was
opened.

Facts read from outside the specs, all public game or third-party MIT facts and none from the Training Modpack:

| Source | What was used |
| --- | --- |
| The owned 13.0.5 ELF (`research/game-13.0.5/main-13.0.5.elf`: symbol table, disassembly, 64-byte windows) | every offset, symbol and guard of the new site rows (`tests/cpu_sites.rs`, `core01_sites_match_owned_elf`); the fighter object layout (kind +0xc, entry id +0x10, module accessor +0x20) from `sv_battle_object::entry_id` and `::module_accessor`; the game's constant lookup `lib::lua_bind_get_value<int>` at `main+0x3728e10` |
| Third-party crate `skyline_smash` 0.1.0 (MIT, crates.io), `src/cpp/l2c_value.rs` and the shape of its `lua_const.rs` | the mechanism only: a constant's value is asked from the game by a 64-bit key of its identifier through `lua_bind_get_value`; the identifiers are the game's own names that the specs already use |
| Third-party crate `lua_bind_hash` 1.0.1 (MIT, crates.io) | the key is XXH64 with the seed `u64::MAX` (the crate's mangled implementation was read to recognise the algorithm; `src/model/cpu/hash.rs` is a standard XXH64 written here); its three published test vectors are used as host tests |

Lead decision wanted: SPEC-input-02 open question 1 asks whether the third-party `smash` constants crate may be
used; package 2 did not need it (the constants are resolved at run time by name), so nothing was added to
`Cargo.toml`.

## pkg5 (menu rendering and assets)

Carried files (stage **m5o**, `/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/stages/m5o/stage`;
all `ours-new` in `docs/provenance/provenance-files.json`; nothing under `upstream-ref/` and no mixed fork file was opened):

| Here | Fork path | sha256 of the stage file | Taken | Clean-room changes |
| --- | --- | --- | --- | --- |
| `src/model/archive_name.rs` | `src/training/ui/archive_name.rs` | `5fc664d1ca98fc7fe008af0a398b0de52cc03c16575221b04e7eb7ab673cff3d` | whole file | none (byte copy) |
| `src/model/archive_retire.rs` | `src/training/ui/archive_retirement.rs` | `47ae08193cd91aceab2e32a1a88f1702789eff4cdca3b9ba5e042c16d621fd33` | whole file | `retire_expired` made `pub` (was `pub(super)`) |
| `src/model/archive_recipe.rs` | `src/training/ui/waiting_archive.rs` + `scripts/make_waiting_patch.py` | `2ea3a4f49f5332419a00f1b751cd0f28f1f4188623d1deb1578acac70a5d62f4`, `5d4c546aaebc0016b5cd79bd0c6df4f5beb619d44936da0bb97241a6bde868e3` | the recipe format, the FNV-1a fingerprint and the greedy copy/literal matcher | one source (the stock archive) instead of two, magic `TRA1`, own bound, the matcher in Rust, new tests |
| `src/runtime/ui/registry.rs` | `src/training/ui/layout_registry.rs` | `137d76960fabcd56f604c3f7f71844f2ed6f582e03b46aa4abd2d4a2f55ffea6` | the holder / manager walk | constants from SPEC-core-05 §4.1 instead of the fork's macro crate; `find` by bytes |

Not carried: `anchors.rs` (its pins and decode are written from SPEC-core-05 §4.1: `model::sites_ui`), the fork's roster, stage and item panel code
(package 6 wires them to the model), `layout.arc`, `waiting_ui.patch` (the waiting-room archive is package 6).

Game data and the committed recipe: `assets/info_training.tra1` (about 11 KB) holds copy ranges of the stock archive and
the plugin's own panes only; the dumped stock archive lives only under gitignored `eden/runs/` / scratch folders
(`tests/ui_sites.rs` and the recipe tests never read it). `tools/layout-builder` is a host tool over the operator's MIT `nx-layout-toolbox` (git dependency pinned to a revision, docs/BUILD.md).

## pkg3 (save states), 2026-11-02

Source: stage **m5o**, `/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/stages/m5o/stage`
(origin `ours-new` checked in `docs/provenance/provenance-files.json` before opening; nothing under `upstream-ref/`, no mixed file).

| Here | Fork path (stage m5o) | sha256 of the stage file | Taken | Clean-room changes (reason) |
| --- | --- | --- | --- | --- |
| `src/model/store.rs` (shared: package 4's named macros reuse it, do not carry `library.rs` again) | `training_mod_macros/src/library.rs` (ours-new) | `f023bf6ccc5908417e2b10014cf26e2311b54260822a2472950e546b468c0275` | whole file except the macro-specific parts | (1) the fork's `MacroError` became `StoreError` (only the variants the store itself raises: version, size, id, not found, no free id, parse, I/O); (2) `Library`, `MacroEntry` and `Record for MacroRecording` removed (package 4 adds its own `Record` impl); (3) `Listing<S>` lost its `MacroEntry` default; (4) the `Header` serde derive in `decode` is a hand read of `schema_version` (this crate has no serde derive: resident size); (5) the id slug fallback is `entry`, not `macro`; (6) the fork's `library/tests.rs` is not carried (macro recordings); new host tests with a toy record cover ids, atomic overwrite, version/size/corruption listing, orphaned backup, sort order; (7) 2026-11-02, slim stream: the `#[allow(clippy::derivable_impls)]` on `impl<S> Default for Listing<S>` removed (a lint allow is a workaround; the hand impl stays because a derive would add `S: Default`; clippy `-D warnings` passes without it); (8) 2026-11-02, slim stream: `Store::write_atomic` calls `write_replace` (the same temporary file `<id>.json.tmp`, sync and replace, written out a second time before). |
## pkg6 (carry-over of the fork's engines), S6-0 plan phase, 2026-11-02

Nothing is carried yet. This section records what S6-0 read, so the lead can audit the boundary.

- Stage **m5o** (`/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/stages/m5o/stage`), never edited. `tools/pkg6_inventory.py` hashes (reads bytes of) only paths that `docs/provenance/provenance-files.json` lists as `ours-new` in the package-6 feature groups, plus six paths absent from that scan (a path absent from the scan is not an upstream path): `training_mod_consts/src/notice_queue.rs`, `training_mod_consts/src/body_bank/mii_publish.rs`, `src/log_stress.rs`, `training_mod_consts/src/log_stress.rs`, `docs/skyline-logger-race.md`, `docs/cpu-rolls-apply.md`. It also walks the stage's directory tree for path names only (no content) to list paths missing from the scan.
- `tools/pkg6_deps.py` parses `use` statements and `crate::` / `training_mod_*::` paths of those same ours-new Rust files and resolves each to a file of the scan; it records which MIXED or upstream-identical file and item a carried file names (no content of a mixed or upstream file is read). Small ad-hoc scans over the same ours-new files counted external crate names, `smash::` paths, menu-struct field accesses, writer calls (`fs::write`, `OpenOptions`, …) and input-owner calls.
- Read directly as ours-new files: `src/training/seams.rs` (lines 1-120), `src/native_install.rs` (lines 1-60), the row labels of `training_mod_consts/src/native_seams/rows_{a1,a2,a3,a4,a5,identity}.rs` (label counts by prefix) and the first line of each, `training_mod_mii/Cargo.toml`.
- Campaign evidence (not fork source): `refactor/RC-SUMMARY.md`, `c1162/EDEN-BRIEF-m5o.md`, `c1162/ONLINE-BRIEF-m5o.md`, `stages/m5o/eden/RESULT.md`, `stages/m5m/eden/RESULT.md`.
- No search over mixed files, none over `upstream-ref/`, none over the fork worktree. No accidental exposure to report.
- Not carried by package 6 and why: `src/training/cpu_rolls.rs`, `training_mod_consts/src/cpu_rolls.rs` (W11's CPU-ROLLS-APPLY: package 2's SPEC-cpu-02 §5, clean-room); `training/seams.rs` and `native_seams/rows_a3.rs` (the fork's upstream-hook install table: superseded by `src/model/sites.rs` rows of packages 1-5).

### Disclosure: fork root `Cargo.toml` (mixed) read by line range, lead-authorised 2026-11-02

Stage m5o `Cargo.toml` (sha256 `8f6ae99261740220bd90817c92c439b5d14244f6c8448ae6c496e816f3cd6b82`), lines **82-122 and 124-240 only** (the fork-only ranges the provenance scan lists), read with `Select-Object -Skip/-First`; line 123 and everything else not viewed. Taken: the `[features]` table (feature names and their local dependency lists), written into this crate's `Cargo.toml` with the fork's one-line comments shortened. Sub-crate features (`training_mod_consts/x`, `training_mod_tui/x`) became the same-name features of this crate; `dep:ssbu-cpu-pilot-core` and `dep:deluxe-plugin-sdk` were left out until their crates are carried (C6, decision D3). `layout_arc_from_file` (package 5) was not taken. Source of the default set: `RECEIPTS-m5o1.json`, job `m5o1-switch-root-full-release` (`enhancements-alpha,native-unlink-guard,deluxe-sdk,mii-hud-portrait-alpha,cycle-preload-extended,lobby-preload-extended,native-body-bank-alpha`).

### pkg6 carried files (S6-1, tranche 1, 2026-11-02)

Stage **m5o**, all files `ours-new`. The table (here path, fork path, source sha256, resulting sha256, every clean-room edit with its reason) is generated from the carry manifest: `docs/provenance/pkg6-carry-ledger.md`
(`python3 -I tools/carry_pkg6.py ledger`; manifest `docs/provenance/pkg6-carry-manifest.json`). 22 files: seam table (`native_seams/*` incl. `rows_a3`, `tests.rs`), `native_fields`, `native_unlink_core`, `native_unlink_guard`,
`source_descriptor`, `source_lifetime`, `source_visible_forms`, `native_hud_refresh`, `source_load`, `roster_grid`, `cycle_admission_grace`, `notice_queue`. Mechanism: a deterministic path rewrite (`crate::x`, `training_mod_consts::x`,
`training_mod_tui::x` to `crate::model::<area>::x`) plus exact-text edits recorded in the manifest. Edits so far (all in two files, each offered back to the fork through the fix ledger):
`native_seams/tests.rs` (three path/layout edits: manifest dir is the repo root, the hook-attribute scan covers only the carried runtime areas, the owned-ELF path is one level up; and the whole-tree count of 150 hooks does not apply to the subset, so the ratchet returns while no carried hook exists)
and `native_unlink_guard.rs` (`# Safety` heading instead of the fork's crate-level `allow(clippy::missing_safety_doc)`).

Deps table (decision D3 conditions; versions are those of the fork's m5o `Cargo.lock`; license fields read from the crates' own manifests in the local cargo registry / git checkouts; resident size = `nro_size.py` of the release NRO built with `cargo skyline build --release` on the Surface at this stage's HEAD):

| Crate | Version / source (m5o lock) | License field | Resident before -> after | Status |
| --- | --- | --- | --- | --- |
| serde (feature `derive`) + serde_derive | 1.0.229, crates.io (identical to this repo's lock) | `MIT OR Apache-2.0` (both) | 233,472 -> 233,472 B (release NRO `d2722f4d…` with and without `derive`: byte-identical, nothing in the release build uses it yet) | added (needed by the Mii crate and `trio`/`ordered_roster`) |
| serde_json | 1.0.151, crates.io (this repo's lock had 1.0.154: `cargo update --precise 1.0.151`, offline) | `MIT OR Apache-2.0` | unchanged (same NRO) | pinned |
| parking_lot | 0.12.5, crates.io | `MIT OR Apache-2.0` | (package 1) | already present (B9) |
| byteflags | 0.1.0, git `https://github.com/asimon-1/byteflags.git` rev `e6ac815a9d0dc4665b39fe7849e2baec8bf443cb` | no `license` field; the repository LICENSE file is MIT (lead-accepted 2026-11-02) | **not added** (see "C2a" below): replaced by `model::compat::choice`, clean-room | not added |
| rand / rand_core / rand_chacha (git `skyline-rs/rand` 8e87ec2a, dependencies of byteflags) | 0.7.4 / 0.5.1 / 0.2.3 | `MIT OR Apache-2.0` | not added (only byteflags needed them) | not added |
| getrandom (git Raytwo 23a14191, dependency of rand) | 0.2.99 | `MIT OR Apache-2.0` | not added | not added |
| skyline_smash (lib name `smash`) | 0.1.0, git `https://github.com/ultimate-research/skyline-smash.git` rev `01bf5835cc1140f5052ddfb2b4f9e98008f22679` | `MIT` (`license = "MIT"` in its Cargo.toml; the repository has no LICENSE file) | 0 B while nothing calls it (LTO drops it); measured when `cpu_pilot` links it | **added** (lead 2026-11-02), `[target.'cfg(target_os = "switch")'.dependencies]`; note: it depends on `skyline` from git (`ultimate-research/skyline-rs#6b52d32`, 0.6.0) next to our crates.io `skyline =0.6.0`: two copies in the lock, to be checked in the first NRO that links both |
| ssbu-cpu-pilot-core | 0.1.1, git `git.syntaxbyexample.com/intpa/ssbu-cpu-pilot` tag `v0.2.1` rev `8b03e3b…` | `MIT` (the operator's crate) | not yet | not added yet (C6) |
| deluxe-plugin-sdk | 2.0.0, no source line in the lock (path dependency); not located yet | – | – | to check at C6 |
### pkg6 carried files, tranche 2 (C3a: Mii crate, body bank, swap model, preload and kirby policy)
Generated ledger: `docs/provenance/pkg6-carry-ledger.md` (101 files after this tranche). New in the tool: item-level and longest-prefix module mapping, `include_str!` rewriting through the manifest, `prepend` (by-path test glue) and `delete_from`/`delete_to` edits. Edits to carried files in this tranche (each offered back through the fix ledger): `trio.rs` and `ordered_roster.rs` lose the fork's menu structure (`to_submenu`, the picker/full-menu tests: replaced by package 1's editor rows and package 5's renderer); `draft_store.rs` (re-run-by-name test filter), `practice_policy.rs` (`#[derive(Default)]` for clippy). The fork's menu names the carried code expects (`OnOff`, `TrainingModpackMenu`, `BASE_MENU`) are provided by `engine::compat::menu` (ours, not carried). Carried tests: the 12 `training_mod_mii/tests` integration files that need only the Mii crate (7 carried; `quickplay_backend`, `ldn_backend`, `source_provenance`, `heap_metrics`, `ui_archive_name` and `replay185_headgear` include other root files by path and are carried with those files).

### Disclosure: an `rg` over two mixed fork files (2026-11-02, while carrying the stage menu)

To find how `PAGE_COLUMNS` (carried `training_mod_tui/src/stage_menu.rs`, ours-new) gets its value I ran one `rg 'NX_SUBMENU_COLUMNS|NX_SUBMENU_ROWS'` that named the two MIXED files `training_mod_tui/src/lib.rs` and `training_mod_tui/src/containers/submenu.rs` of stage m5o (my mistake: I should have searched only ours-new paths). It printed three lines of `lib.rs`: the definitions `pub const NX_SUBMENU_ROWS: usize = 8;` (line 14) and `pub const NX_SUBMENU_COLUMNS: usize = 4;` (line 15), and one use at line 64 (`toggles: StatefulTable::with_items(NX_SUBMENU_ROWS, NX_SUBMENU_COLUMNS, toggles),`); it printed nothing for `submenu.rs`. Nothing was copied. The only value used is the stage grid's page width of 4 columns, which is independent of that line: `docs/DECISIONS.md` Q14 and `MENU-DRAFT.md` state the fork's stage paging as 4 x 4 (eight pages), and the carried `PAGE_ROWS = 4` with `PAGE_SIZE = 16` agrees. The compat constant is written as 4 from those documents.

### pkg6 C2a: cycle_preload_config (2026-11-02, `engine::preload::cycle_preload_config`)
Carried `training_mod_consts/src/cycle_preload_config.rs` (ours-new, m5o `49c8a777d9e7`) by the tool. Manifest edits (each a ledger row): the fork menu-macro modules `bank_submenu` / `ready_submenu` removed, the `create_app` Character Extras test removed (package 1's menu table tests cover the rows), the commit round-trip test serialises the typed menu directly. Path rewrites: `TRAINING_MODPACK_ROOT` to `model::storage::ROOT`, `menu_json` to `compat::menu_json`, the storage-root string. **byteflags decision:** the lead accepted the `byteflags` git dependency; it is not added. Its `byteflags!` macro names a struct field after every choice in capitals, so every user crate gets `non_snake_case` warnings that only a lint `allow` (a workaround needing approval) silences, and it pulls three git dependencies (`rand`, `getrandom`, itself) for a `get_random` the carried code never calls. `model::compat::choice` is the same macro over a `[u8; N]` (constants, `ALL_CONSTS`/`ALL_NAMES`, saturating `+ - *`, `to_byte_vec`, `contains`, `Display`, the same JSON array shape), 5 host tests; the tool rewrites `use byteflags::{byteflags, count}` to it (new `byteflags` head in the tool), so the carried text is unchanged. Written without reading the crate's macro beyond its public API doc and source in the local cargo checkout (MIT). `compat::menu` now holds the 13 `cycle_preload_*` fields and the Solo Popo fields as typed values with the fork's serde attributes (`from_snapshot`, `rows` for the D1 write-back), and `compat::menu_json::parse_menu_file` is the stand-in the carried tests load fork-shaped menu files through. Two fork defaults differ from the clean-room factory table (`solo_popo_original` and `cycle_preload_measurements`: fork On, MENU-DRAFT/factory Off): `BASE_MENU` keeps the fork values, `from_snapshot` gives the settings file's; a host test pins the list. Clean-room tests `src/model/preload/ini_transactions_tests.rs` (9): two-load, failed store, interrupted replace, rejected file untouched across boots, oversize/invalid-UTF-8/duplicate envelope, older-file adopt-and-complete; the carried reader/writer passed all of them unchanged (R1/R4 hold for this file). Open nit offered to the fork: `write_new` ignores the `sync_all` result.

### pkg6 gate-1 rulings applied (2026-11-02)
- **`byteflags` replacement accepted** by the lead; the rewrite of the carried `use byteflags::{byteflags, count}` to `model::compat::choice` is a recorded carry transform of the tool (`EXTERNAL` head `byteflags`, manifest `modules.byteflags`), applied on every re-sync.
- **Fresh-install defaults** follow the factory table and MENU-DRAFT: three more recorded edits to `cycle_preload_config.rs` (`Settings::DEFAULT.measurements`, `default_measurements()` and the one test that asserted the old default) make Preload Samples Off; `compat::menu::BASE_MENU` takes `solo_popo_original` Off and `cycle_preload_measurements` Off, so it equals `from_snapshot(&Snapshot::factory())` (host test). Fork defaults that differ from the clean room, as far as package 6 has read them: Solo Popo (original) On in the fork, Off here; Preload Samples On in the fork, Off here (Developer page, inert unless `developer.ini` enables it). No other difference is known; rows of other packages were not compared.
- **`cycle_preload.ini` is a regenerated boot cache** (`model::preload::boot_cache::sync`): settings.json is the only source of the active choice; the INI is written from it on first boot or when stale, rejected or missing, and is never read back to change the choice (6 host tests: first boot writes the player's settings, stale cache never overrides, rejected cache regenerated, failed write keeps the old file and the active choice, malformed rows fail closed).
- **R1 durable publish**: recorded edit to `write_new`: `sync_all` failure now fails the store before the rename (the fork ignored it). Risk to check at the first Eden boot: if the Skyline std `File::sync_all` reports an error on the console, every store would fail; the E8 first-boot row exercises it.
- **Stages unconditional** (`7f7b5f1`): see the commit message; `model::stages::catalog` is the facade for package 5.
### pkg6 D3 checks for C6/C7 (2026-11-02, read-only: a shallow clone of the pilot repo under /tmp, never built there)
| Crate | Source / pin | License evidence | Status |
| --- | --- | --- | --- |
| ssbu-cpu-pilot-core 0.1.1 (path crate `core/` of the pilot workspace; features: `native`, Switch-only raw calls, no Skyline dependency) | git `https://git.syntaxbyexample.com/intpa/ssbu-cpu-pilot`, tag `v0.2.1`, rev `8b03e3bffea8a156628f76cd9ca31c346f52ac70` (HEAD of main is `f503fb5`, newer) | `license = "MIT"` in `core/Cargo.toml`; repository `LICENSE` is MIT, `Copyright (c) 2026 crashbandicode` | accepted by the lead's D3 ruling; not added yet (needed by `cpu_pilot.rs`); resident delta to be measured when `cpu_pilot` is unconditional |
| skyline_smash (lib `smash`) 0.1.0 | git `ultimate-research/skyline-smash` rev `01bf583` (cached) | `MIT` | not added yet (C7) |
| deluxe-plugin-sdk 1.0.0 (+ deluxe-plugin-abi 1.0.0) | git `https://git.syntaxbyexample.com/intpa/ssbu-online-deluxe-rewrite`, rev `a63bc8c` as pinned by the pilot (checkouts here hold `d444f97`, `fa47994`) | **no `license` field and no LICENSE file** in `sdk/sdk/Cargo.toml`, `sdk/abi/Cargo.toml` or the repository | **STOP per D3: reported to the lead** (the repository is the operator's; it needs a license field or file before the `deluxe-sdk` feature's `dep:` entry is added) |
### pkg6 C6a: the pilot core's pure module (`engine::p1`, 2026-11-02)
Carried from `src/cpu_pilot/` (stage m5o, ours-new), unedited: `pilot.rs` (as `src/model/p1/mod.rs`), `config.rs`, `move_observer.rs` + `move_observer_tests.rs`, `tests.rs` and the three `examples/*.json` its tests include. This is the fork's self-contained host-testable `pilot_core` (explicit JSON config, trigger/command model, move observer). Superseded by the C6 section below: the module now loads as a child of the carried `cpu_pilot.rs` by `#[path]`, and its 25 host tests (with the two session files) run by path from `tests/p1_core.rs`.

### pkg6 C2c: preload_metrics and the logging shim (2026-11-02)
Carried `src/preload_metrics.rs` (stage m5o, ours-new, path rewrite only; svcGetInfo on aarch64, a host path elsewhere; 7 carried tests). New `model::compat::logging` (written here): `diagnostic` forwards to `runtime::diag_log::diagnostic` on the Switch (`runtime::diag_log` became `pub(crate)`, one word in `src/runtime.rs`) and records into a bounded capture buffer on the host (`take()` for tests); `heap_profile` is a no-op (the fork's Eden-only `plugin-heap-meter` allocator is not carried). Verified for the Switch by every Yoga build since the merge (release `006371d1`).
| `src/model/state/held_item.rs` | `training_mod_consts/src/character_items.rs` (ours-new) | `57640dd2a5a9ea201d3a588e05bb950f5f7cecb551d602acfeeb44d0b5a3c2bf` | `ITEM_NAMES`, `valid_variation`, `HeldItemSnapshot` + `valid`, the rule of `deserialize_saved_item` (lines 5-83) | renamed `HeldItem`; no serde derive (hand-read `from_json`: damaged field -> `Unsupported`, `null` -> absent; hand-written `write_json`); the menu types `CharacterItemChoice` / `to_submenu` and the menu test are not carried. |
| `src/model/state/record.rs` (`Charge`, `Robin`, `Steve`, `validate`) | `training_mod_consts/src/practice_setup.rs` (ours-new) | `4ec4abf8de44def3b5efc921d8123e82ad2cc1d8bc92e3d1f832254513e7d7a8` | `RobinCharges`, `ChargeState`, `SteveState` and their `validate` rules (lines 44-207) | renamed `Robin`, `Charge`, `Steve`; no serde derive (hand JSON in the fork's field order, absent options written as `null` exactly as the fork's pretty writer does); Steve's tool materials are `u32` code points (the fork's `char`); a Steve record that fails `validate` on read is dropped; the Mii identity is an opaque `serde_json::Value` kept as read (package 6 gives it a type). |
| `tests/state01_json.rs` + `tests/fixtures/ported-json/` | `training_mod_consts/tests/json_ported_fixtures.rs`, `json_save_state_shape.rs`, `tests/fixtures/ported-json/` (ours-new) | fixtures: the five `*-save_states*.json`, five `expected-a-import/*.imported.json`, `PROVENANCE.json` (byte copies; the `*training_modpack*` updater-record fixtures are not carried) | the assertions of both test files | ported to this crate's reader and writer (the same fixtures, the same byte-for-byte rewrite check); new tests for the filesystem rules (two boots, a corrupt file, a failed write). |
| `tools/elf_symbols.py` | written here (not carried) | | | symbol and 64-byte window lookup in the owned ELF for the site rows. |

Not carried (read and judged): `src/training/setup_library.rs` (ours-new, 392 lines) is glue over the named-macro keyboard draft (package 4), lobby practice (package 6), the Mii roster (package 6) and a TUI panel; its file format `practice_setup.rs` (`SetupRecord`, `FighterSetup`, validation, the `Store` instantiation) is carried by `src/model/state/setup.rs` (see below) and the glue is re-written against this plugin's interfaces. `training_mod_consts/src/menu_json.rs` holds the menu-settings reader (the package-1 reader is already written from SPEC-core-04); it has no `save_states.json` code. The `save_states.json` reader/writer is new here because the fork's lives in the mixed `save_states.rs`, which was never opened; the shape is SPEC-state-01 §7.2 plus the fork's real files (the fixtures above).

### pkg3 additions (milestones S3-1 to S3-5, SPEC-state-05)

| Here | Fork path (stage m5o) | sha256 of the stage file | Taken | Clean-room changes (reason) |
| --- | --- | --- | --- | --- |
| `src/model/state/setup.rs` | `training_mod_consts/src/practice_setup.rs` (ours-new) | `4ec4abf8de44def3b5efc921d8123e82ad2cc1d8bc92e3d1f832254513e7d7a8` | `SetupRecord`, `FighterSetup`, validation, the error type, the listing row, the `Record` impl over the shared `Store` | hand-written JSON (no serde derive); this crate's `Charge` / `Steve` / `HeldItem`; the Mii identity is an opaque value (package 6 types it); the UI glue (`setup_library.rs`) is not carried. |
| `src/model/state/items.rs`, `src/runtime/features/state_restore.rs` (item part), `held_item.rs` | `src/character_items.rs` (ours-new) | `b0679903a58021f041bec47d30926f946fc6ef8dbfb679574fd12e597ab78933` | the mechanism: clear the hand, `ItemModule::have_item(kind, variation)`, the turnip special-init action with its power getter, the 16 probability hooks for the face override, a source fighter required for items that need one | re-expressed against the spec's tables and this crate's site rows (guarded `main+offset` calls, no allocation in hooks); the spec's grant paths A to C are superseded by this mechanism. |
| `src/model/state/convert.rs`, `src/model/migration_run.rs` | none (new here) | | | the N6 conversion and the first-boot import are written from SPEC-state-04 and SPEC-core-08; only the fork's folder layout and file names are known from the fixtures. |
| `src/model/state/pokemon.rs`, the `copy_charge` field | none (new here) | | | SPEC-state-05 §5.1, §5.2 only; the new optional record field `copy_charge` is this crate's (lead ruling 2026-11-02): fork-compatible, written only when present. |

## pkg4 (package 4: recording, input display, chords, notifications; stream `stream/pkg4`)

Source: stage **m5o** (`.../parked-reuse175/refactor/stages/m5o/stage`, called `<stage>`). Only files whose `origin` is
`ours-new` in `docs/provenance/provenance-files.json` were opened (`training_mod_consts/src/menu_shortcuts.rs`,
`training_mod_macros/src/capture_settings.rs`, `training_mod_macros/src/test_util.rs` for its `TempDir`, the heads of
`training_mod_macros/src/lib.rs` and `src/capture_plugin.rs`, and the import / dependency lines of the other
`training_mod_macros` files to size the carry). The mixed files (`input_record.rs`, `button_config.rs`, `input_log.rs`,
`ui/display.rs`) and everything under `upstream-ref/` were not opened; the fork's benchmark fixtures
(`scripts/input-display-bench/fixtures/*`, upstream copies) were not opened.

| Here | Fork path (stage m5o) | sha256 of the stage file | Taken | Clean-room changes (reason) |
| --- | --- | --- | --- | --- |
| `src/model/menu_shortcuts.rs` | `training_mod_consts/src/menu_shortcuts.rs` (ours-new) | `b19b20ac365178ff7ddba0b4801698872048de0081e8955ed4df9e87ea2e6d0d` | whole file (parse, passes, tests) | The fork's `ButtonConfig` byteflags struct is this plugin's `Chord` (`u16`, bit order of the chord rows); button-name tables (`FIELDS`, `LABELS`) replace `ButtonConfig::ALL_FIELDS/ALL_NAMES`; `DEFAULT_INI` doc line no longer says "restart" (N7, a change applies at once); `passes` also needs a non-empty chord; added `to_ini_text` (N7 writer: keeps comments and unknown lines, one line per slot), `with_slot`, `slots`, `ini_name`, `FILE_NAME`, `MAX_BYTES`; tests moved to `Chord` plus round-trip tests. |
| `src/model/capture_settings.rs` | `training_mod_macros/src/capture_settings.rs` (ours-new) | `5357f35edcdee093075cd7e27e6c206c8a408b8a05ad46b583493213b6dc6af3` | whole file | none to the carried text except the test `TempDir` (the fork's `test_util::TempDir`, reduced, defined in the test module). Added at the end: `FILE_NAME`, `MAX_BYTES`, `to_ini_text` (the menu's Replay capture toggle, Q7: replaces the recognised line in place), `from_bytes` (fail-closed read) and their tests. Phase2 pkg6 repairs ONLY this clean-room serializer tail after original RED: an explicit choice repairs invalid/duplicate owned keys while retaining unknown lines/comments; the carried prefix through line398 stays byte-identical to RC8. |

Not carried yet (S4-4, waiting on the lead's sequencing: pkg6's `training_mod_mii` first): `training_mod_macros/src/{recording,library,window,replay,live_frame,move_start,live_capture,capture_archive,capture_toggle,shared_capture,ui_anchor}.rs`,
`src/training/named_macros.rs`, `src/replay_*.rs`, `src/live_replay.rs`, `src/capture_plugin.rs`, `src/training/ui/clip_editor.rs`,
`training_mod_tui/src/{clip_*,macro_panel*,replay_panel,resume}.rs`. Dependencies they need (lead rulings 2026-11-02): `training_mod_mii`
(pkg6), `smash-match-data` (vendored under `crates/`), `deluxe_plugin_sdk` (feature `deluxe-sdk`), `serde` with derive.
`library.rs`'s generic `Store` is pkg3's `model::store`; the macro `Record` impl follows `recording.rs`.

New code in this stream with no fork origin: everything else under `src/model/{recording,chords,ini_files,notify,damage_display,input_display,sites_pkg4}.rs`
and `src/runtime/features/{recorder,chords,notices,input_display}.rs`, written from SPEC-input-01/02/04/05 and the owned 13.0.5 ELF.

### pkg4 S4-4 (named-macro / replay / clip layer), carried by `tools/carry_pkg4.py` from stage **m5q** (2026-11-02)

Source: stage **m5q** (`.../parked-reuse175/refactor/stages/m5q/stage`; m5q is the newest frozen stage, F1-F4 landed). Every path was checked as `ours-new` in
`docs/provenance/provenance-files.json` before it was opened (the `training_mod_macros` crate, `training_mod_tui/src/{clip_browser,clip_panel,macro_panel,macro_panel/fighter,replay_panel}.rs`);
nothing under `upstream-ref/`, no mixed file, no stage-wide search. `smash-match-data-design` (the standalone crate the fork's macros crate depends on, git `faac04e`) is read from its own folder.
The table (here path, fork path, source sha256, resulting sha256, every recorded edit with its reason) is generated from the manifest: `docs/provenance/pkg4-carry-ledger.md`
(`python3 -I tools/carry_pkg4.py ledger`; manifest `docs/provenance/pkg4-carry-manifest.json`; `tests/carry_pkg4.rs` fails on a hand edit and on a stale ledger).

- `training_mod_macros` (pure model: macro recording, library, replay window, move-start rules, capture toggle/archive/shared archive, live capture/frame, UI anchor): `src/model/macros/`, declared as `crate::engine::macros`.
  Edits: `lib.rs` (the crate-level `forbid(unsafe_code)` dropped, `capture_settings` is S4-2's `model::capture_settings`, the generic Store names come from `model::store`, the integration test becomes a unit test),
  `library.rs` (the generic Store, `Record`, limits and helpers are `model::store` (package 3): only `MacroEntry`, `Library = Store<MacroRecording>`, `Record for MacroRecording` and `From<StoreError> for MacroError` stay),
  `library/tests.rs` (tests of the Store's private helpers `check_id` / `slug` / `replace_via_backup` dropped: `model::store`'s own tests pin them; the slug fallback is `entry`, not `macro`), `move_crop_continuation.rs` (unit test).
  `training_mod_mii::` paths become `crate::engine::mii::` (the carried Mii crate; `engine::mii` is unconditional now: one-line change in `src/engine/mod.rs`, package 6's area).
- `training_mod_tui` panel models: `src/model/macro_ui/` (`crate::engine::macro_ui`), pure state machines and `View` values (no ratatui, no terminal); package 5 draws them. `resume.rs` is package 6 worker B's, not carried here.
  The constants `NX_SUBMENU_ROWS` (value 8, from pkg6's disclosure above) joins `engine::compat::grid`.
- `smash-match-data` (vendored): `crates/smash-match-data/` as a path crate (`smash-match-data = { path = "crates/smash-match-data" }`), source files copied unchanged. Dropped: the `replay` feature and its private git dependency `smash-replay-core`
  (no consumer; a git dependency would break the offline lock), `src/bin`, `examples/`, `tests/{cli,replay}.rs`. Dependencies: `serde` (derive), `serde_json` with `float_roundtrip`, `sha2 0.10.9` (MIT OR Apache-2.0); the source repository has no
  licence file (the project is the operator's own; the lead's ruling covers vendoring it).
  Its own tests run with `cargo test --manifest-path crates/smash-match-data/Cargo.toml` (22 pass).
- Writes: the carried archives only ever create new capture folders (`.smd-N.tmp` staging, then one rename into a name that does not exist; a failure removes the staging folder) and never overwrite; the named library goes through `model::store`'s transactional write.
### pkg6 C1b: seam_install (2026-11-02)
Carried `src/swap_native/seam_install.rs` as `src/runtime/seams/seam_install.rs` and `seam_drill.rs` (diag only), Switch-only, with three recorded edits: `use crate::model` becomes `model::version as model`; the fork's second install line (`report_install`, `[TrainingModpack] result=installed ...`) is removed; `InstallFields` (new, in the same file) is the seam outcome as fields of package 1's one install line (`seams=<n> seams_stock=<n> seams_other=[..] seams_skipped=[..] seams_shadow_diff=[..] seams_missing=[..] seams_external=[..] seams_unchecked=<n> seams_authoritative=.. seams_images=.. seams_authoritative_images=..`, lead ruling 2026-11-02); rows checked or changed after the line keep the tagged late line (`[TrainingSeams] late ...`, tag already shipped). `model::install_line::InstallLine` gained `seams: Option<Extra>` (boot passes `InstallFields`; today `seams_unchecked` is the whole table because no consumer calls `gate` yet). `engine::seams::native_seams` (the table) is unconditional; `native_fields` and the unlink guards stay test-only until their consumers land. Resident (Yoga, `cargo skyline build --release`): 765,952 B before (`a8a1dfb9`, the merged tree) -> 901,120 B after (`120f2342`): **+135,168 B** for the 620-row table and the installer (budget estimate for the seam rows was 180 KB). Diag build compiles (`20f054dc`). Dead-code warnings (unused `gate`, `site`, `check_image`, ...) on the Switch build until the cycle files call them.

### pkg6 D3 update: one skyline, pilot core added, cpu_pilot.rs staged (2026-11-02)
- **Unified `skyline`** (lead ruling): `skyline_smash` names the skyline-rs and nnsdk-rs git sources; two `[patch."<git url>"]` entries (`skyline = { version = "=0.6.0" }`, `nnsdk = { version = "0.6.2" }`) point them at the registry crates, so `Cargo.lock` holds exactly one `skyline` 0.6.0, one `skyline_macro` 0.2.1 and one `nnsdk` 0.6.2, all from crates.io (`tests/pkg6_deps.rs` fails on a second copy or a git source). `cargo tree -d --target all` now lists only `libc-nnsdk` 0.4.0 (`skyline_smash` asks `^0.4`) and 0.5.0 (`nnsdk`): the two are plain FFI declaration crates with no runtime state, and no patch can join incompatible semver versions; listed here as a known residual. (`syn` 1 / 3 are host proc-macro build dependencies.) The Yoga Switch release builds with the patch: resident 901,120 B, unchanged (the new crates are not linked yet).
- **`ssbu-cpu-pilot-core` added** (git, rev `8b03e3bffea8a156628f76cd9ca31c346f52ac70`, feature `native`, Switch target): MIT (`core/Cargo.toml`, repository LICENSE); size 0 B until `cpu_pilot` links it.
- **Staged, not yet declared (no compile)**: `src/runtime/p1/cpu_pilot.rs` (stage m5o `src/cpu_pilot.rs`) with three recorded edits (the `#[path]` modules become `engine::p1`, `engine::p1_menu_session`, `engine::p1_stage_session`; the approved `allow(dead_code)` on the first is dropped), and the two pure sub-files `menu_session.rs` / `stage_session.rs` (test-only modules, their tests pass). The tool learned the `training_mod_sync` head (to `compat::sync`). The names `cpu_pilot.rs` takes from outside are rewritten to `crate::runtime::p1::compat::{common::{FIGHTER_MANAGER_ADDR, FighterId, game_mode::current, input::MappedInputs, is_fighter, is_grounded, is_native_training_mode, menu::captures_input, try_get_module_accessor}, training::{recording_or_playback, ui::menu::VANILLA_MENU_ACTIVE}, replay_preview::occupied, character_roster::busy, sync::read}`: that module is the thin adapter written against the wire stream's APIs once their commit is named.
- **Guarded-call cross-check** `tests/pkg6_smash_calls.rs`: scans every source under `src/runtime` that uses `smash::` for calls on `*Module`, `FighterManager`, `FighterEntry`, `utility`, `sv_battle_object`, requires each in the `COVERED` call-to-row table and each row to exist in a site table. A re-sync that adds a call fails until the call is guarded.- Correction to the staging note above: `menu_session.rs` / `stage_session.rs` are not declared in `engine` yet (clippy `-D warnings` flags the never-constructed `stage_session::Invalid` without its consumer); their 7 tests run by path from `tests/p1_sessions.rs` (the playbook's by-path include with `allow(dead_code)`, the pattern the Mii tests already use). They are declared as `engine::p1_menu_session` / `engine::p1_stage_session` together with `cpu_pilot.rs`.
### pkg6 re-sync from stage m5q (D10, 2026-11-02)
`python3 -I tools/carry_pkg6.py carry --stage m5q --update` (stage `…/parked-reuse175/refactor/stages/m5q/stage`, the fork's final stage, content sha 8b7df338…): the path lists of m5o and m5q are identical (897 files each, so no new ours-new file), and **three** carried files changed, all recorded edits re-applied cleanly: `training_mod_consts/src/native_seams/rows_a1.rs` (+51 lines: four `group_swap.*` called rows for the asynchronous group build, W20 F3, guards from the owned ELF, checked by the carried table tests), `training_mod_consts/src/body_bank.rs` (+162 lines, F3), `src/cycle_preload_policy.rs` (F2: `merge_player_hats_first`, hats first while P1 is Kirby). `cycle_preload_config.rs` and the notice queue are unchanged; the F1 notice wording and the F4 fork-side re-roll live in root files not carried yet (`cycle_preload.rs`, `character_roster.rs`: they arrive with the C7 cycle from m5q). `verify --stage m5q` reports 124 files identical to a fresh derivation. Host: `cargo test --no-fail-fast` passes except the wire stream's `every_row_matches_the_owned_elf`; clippy `-D warnings` 0.
- **Approved restriction (operator decision 2026-11-02, both trees), for the compliance report's approved-workarounds table:** Pokémon Trainer / Aegis (native) and Ice Climbers pair groups stay **synchronous**, built on the first swap; only the other groups are built asynchronously (F3, phase B′). Date 2026-11-02. Mii groups were in this restriction until the operator lifted it for them on 2026-11-08 (`docs/DECISIONS.md`, "Group speed in rc.6"); they build asynchronously since `p23/rc6-hot-slots` item 2.
- **Open operator decision D-DELAY-REROLL:** the ledge, throw and pummel delays keep the per-grab re-roll rule (recommendation A, the current behaviour); not a package 6 change.
### pkg6 C6: CPU-as-P1 fallback (Q11) compiled and wired (2026-11-02, after the merge of work/implementation a44b7ce)
- **Carried and declared**: `src/runtime/p1/cpu_pilot.rs` (stage m5q `src/cpu_pilot.rs`) with four recorded edits: the three `#[path]` child modules (pilot core, menu session, stage session) now point at the carried files under `src/model/p1/` (the fork's own module structure, so its `pub(super)` items stay visible to `cpu_pilot`; the fork's approved `allow(dead_code)` on the pilot-core declaration is kept byte-identical), and `installed()` (added so package 1's install line can report the feature). The pure files run their 25 host tests by path from `tests/p1_core.rs`.
- **`runtime::p1::compat`** (written here): the names `cpu_pilot.rs` takes from outside, each one call into another package's API: `MappedInputs`/`Owners` (`model::input_delay`, `published::input_owners()`), `recorder::{recording_or_playback, replay_editor_occupied}`, `scene::is_native_training`, `HOT_SWAP_IN_PROGRESS` for `character_roster::busy`, and the fighter natives only through `runtime::game::natives` (`module_accessor`, `is_fighter`, `is_grounded`: their capability gates answer "nothing" until their rows passed). `game_mode::current()` returns `None` until the lobby UI work (worker B) lands, so only the offline Training scope is live; the online 1v1 scope stays package 7's.
- **Call-ins**: `runtime::p1::{check, install, reports}` from boot (after `ui::install`, which decides `PILOT_PRESENT`), `p1::frame` from the CPU core's per-fighter callback (`features::cpu::on_frame`), `p1::input` after the delay in `features::input_delay::apply`, `p1::publish` from `published::publish` (`p1_cpu_control`, `p1_cpu_level`). The fallback installs only when the standalone pilot is absent, the five `sites_pkg6` rows and the prerequisite rows (CPU core, input hooks, save-state natives, `status_kind`, `prev_status_kind`, `pos_x`, `pos_y`, `damage_of`, `utility_get_kind`, `fighter_manager_get_entry`) are usable and the carried seam gate `cpu_pilot` passes; the install line carries `p1_cpu=installed|not_installed reason=standalone_pilot_present|site_guard|prerequisite_feature|seam_gate` and the five rows.
- **Resident (Yoga, `cargo skyline build --release`)**: merged tree 929,792 B (`01160f4a`) -> 1,024,000 B (`006371d1`): **+94,208 B** for the adapter, `smash` natives, pilot core and `native_fields`. Diag build `e5a8edeb`, 1,175,552 B. `native_fields` is `cfg(any(test, switch))` (its consumers are Switch-only).
- Host gates: `cargo test` all pass (6,2xx), clippy `-D warnings` 0, `--no-default-features` builds. Eden set C is next (pilot-absent rows k03 and CP rows; the pilot-present k04 row).
## pkg6b (package 6 worker B: Mii runtime, lobby UI, the cycle's lobby half), 2026-11-02

Stage **m5o**, branch `stream/pkg6b` from `23cdc6c`; carried files are generated by `tools/carry_pkg6.py` from `docs/provenance/pkg6-carry-manifest.json` (rows in `docs/provenance/pkg6-carry-ledger.md`). Playbook followed: `ae87443`.

### Disclosure: an `rg` that printed mixed-file lines (2026-11-02)

While finding which file declares the carried Mii modules, one `rg "^\s*(pub(\([a-z]+\))? )?mod [a-z_0-9]+\s*;|#\[path" src --glob '!lib.rs' ...` over the stage's `src/` did not exclude `src/lib.rs` (the glob `!lib.rs` did not match `src/lib.rs`) and printed its `mod <name>;` / `#[path]` declaration lines (about 55 lines, lines 10-165: module names such as `mod character_roster;`, `mod mii_editor_bridge;`, `pub mod common;`, no cfg attributes, no code) and the single line `src/hazard_manager/mod.rs:8: mod policy;` (both MIXED files). Nothing was copied or used beyond the fact that those module names are root modules of the fork's crate, which the carried files' own `crate::<name>` paths already state. I then restricted further searches to named ours-new files.

### pkg6b tranche 1 (2026-11-02): tool, module map, first Switch-compiled files
Tool (`tools/carry_pkg6.py`): heads `training_mod_sync` and `training_mod_macros` join the path rewrite (`modules.sync`, `modules.macros`); `#[path = "x.rs"]` declarations are rewritten against the manifest (the child keeps resolving wherever the manifest puts it; an unlisted target is left as written and reported on stderr); in a Switch-only file (dest under `src/runtime/`) a `#[path]` child that is a pure carried module becomes `use crate::engine::<area>::<name> as <name>;` (one module, not two copies). `verify` still reproduces the 121 files of the earlier tranches byte for byte.
Carried (all Switch-only, `src/runtime/`): `common/game_mode.rs` (as `lobby/game_mode.rs`), `training_stage_rotation.rs` (no edit: the tool turns its `#[path]` policy module into a `use`), `native_keyboard.rs` + `inline_keyboard.rs`, `mii_identity_binding.rs` (all four compile for the Switch, release build `Finished`); and, not yet declared as modules (they are children of the roster cycle's `character_roster.rs` through `#[path]` and `use super::*`, which package 6's worker A carries): `mii_auto_init`, `mii_backend`, `mii_catalog_request`, `mii_cpu_adapter`, `mii_hud_portrait`, `mii_ldn_qualification`, `mii_managed_helper`, `mii_native_access` (+`mii_native_cpu_scope`), `mii_native_adapter`, `mii_closed_ack`, `mii_ordinary_{cpu,cpu_native,gate(+.S),native,player}`, `mii_replay`, `mii_saved_{alpha,fingerprints,native}`, `mii_selector_native`, `mii_source_provenance`, the whole `mii_editor_bridge/` (52 files incl. `lua_helpers/*.lua|luac|json`; the Lua test scripts under `lua_helpers/tests/` are not carried), `practice_stage_request` (+words), `practice_stage_observer`, `lobby_stage_probe`, `training/{lobby,practice}`.
New clean-room code (not carried): `src/runtime/compat/{common,menu,sync}.rs` (the fork's `common`, the typed `MENU` and `training_mod_sync::read` as the carried files name them, each forwarding to the clean room's provider; `published::publish` feeds the rows through `compat::menu::apply_snapshot` and `compat::menu::apply_kept(&Settings)` adds the saved defaults and the kept stage/Mii keys after publishing, D1; integrator-2 split the former `apply` so the rows have one path), the typed menu gained `lobby_practice`, `training_stage_rotation` and `lobby_stages` (`engine::compat::menu`, host test), and the engine areas the lobby and Mii game files reach (`preload`, `mii`, `mii_root`, `lobby`, `swap`, the stage policies, `native_fields`) are also on the Switch (`cfg(any(test, target_os = "switch"))`).
Superseded, not carried: `training/ui/layout_registry.rs` (package 5's `runtime::ui::registry`), `training/ui/anchors.rs` (package 5's anchors from SPEC-core-05 §4.1), `training/ui/native_menu.rs` (package 4's `features::notices` Combo Display observers), `training/ui/{roster,stage}_grid.rs` (the fork's pane drawing of its own grid layout; package 5 draws the grids from `model::menu::grid` on its own panes).

### pkg6b re-sync to stage m5q (D10, 2026-11-02)
Stage m5q (the fork's final stage, content sha 8b7df338, F1-F4 fixes) via `python3 -I tools/carry_pkg6.py carry --stage m5q --update`: three carried files changed, none in worker B's area (`native_seams/rows_a1.rs`, `body_bank.rs`, `cycle_preload_policy.rs`); every recorded edit re-applied. Host tests: 5,638 pass, the one failure is the wire stream's `input_sites::every_row_matches_the_owned_elf`; the seam table's compared rows still match the owned ELF.
## u4 (run-time game-file texture helper)

Nothing carried. Every u4 file is new clean-room code written from the owned 13.0.5 ELF (reverse engineering with Ghidra over a copy of the project and small ELF scans, no upstream or fork-non-ours-new file opened): `src/runtime/ui/texture.rs`, `src/runtime/ui/texture_probe.rs` (diag), `src/model/ui_texture.rs`, `src/model/sites_u4.rs` (generated by `tools/gen_sites_u4.py`), `tests/u4_sites.rs`, `eden/u4/*` (the harness copies pkg5's `tr_eden.py` / `route_s5.py` / `drive_s5.py` with this stream's paths). pkg5's ours-new `mii_hud_portrait.rs` was read only for how it calls `main+0x379e9c0` (its guard row and the descriptor note).


## RC7 R3-4 native-editor installation split, 2026-11-03
No new fork copy. Two existing carried destinations, `mii_editor_bridge/mod.rs`
and `fingerprints.rs`, gain exact-text manifest edits: retained Training-stage and
catalog base/readiness no longer initialize the unavailable editor's hooks,
native-default adapter, trace writer, lifecycle, keyboard or resume observers.
The retained callback uses its existing guarded Training-state contract; waiting
catalog readiness reads the existing guarded game-mode provider. Fresh/resumed
editor admission remains unavailable. `mii_editor_bridge/install.rs` and its host
installation-spy tests are new clean-room code. `carry_pkg6.py verify` re-derived
all 247 destinations from pinned ours-new stage m5q; the generated ledger records
the two changed destination hashes. This is provenance/source evidence, not native acceptance.

## stack (thread stacks, stream `stream/stack`), 2026-11-02
Nothing new carried. Clean-room edits of carried files, each recorded as an exact-text edit in its manifest (`verify` re-derives
them from stage m5q: 262 + 42 files) and in the generated ledgers: `src/runtime/roster/practice_trace.rs` (the writer thread
through `runtime::threads::spawn`, its local `STACK_BYTES` removed, the session line prints the table's size),
`src/runtime/mii/mii_editor_bridge/mod.rs` (`Trace` is `Copy`, `Trace::EMPTY`: the trace writer's `BoxedSlot` copies it into a
heap buffer in place; moved by value it gave `tr-trace-writer` a 59,712 B frame on 64 KiB, the Eden s4-07 / u4b-02 boot fault),
`src/runtime/macros/replay_store.rs` (`replay-capture-save` and `mii-catalog` through `runtime::threads::spawn`);
one diag `crate::runtime::threads::headroom("<site>")` line at the entry of the deep callbacks in `roster/native_service.rs`,
`roster/character_lifecycle.rs`, `roster/native_source_capacity.rs` (two), `roster/native_body_bank.rs`,
`mii/mii_hud_portrait.rs` (two), `mii/mii_ordinary_gate.rs` (a no-op inline in the shipped build);
`mii/mii_ordinary_gate.rs`: `mii_ordinary_gate_decide`'s three exclusive arms are `#[inline(never)]` functions (its frame was
20,000 B with all three inlined); `mii/mii_ordinary_cpu.rs`, `mii/mii_ordinary_player.rs`: `bank_service` and
`mii_group_record_publication` are `#[inline(never)]` (their locals no longer sit in every `service` call's frame). Package 1's
carried `src/runtime/diag_log.rs` (no manifest): the drain thread through `runtime::threads::spawn` and, in the diag build, the
`[ThreadStack]` high-water report on its 10 s beat. Written here: `src/model/thread_stacks.rs`, `src/runtime/threads.rs`,
`BoxedSlot` in `src/model/lobby/trace_slot.rs`, `tests/thread_stacks.rs`, `tools/stack_depth.py` (+ `tools/test_stack_depth.py`).

## pkg6a C7 (roster cycle declared, joint declaration commit), 2026-11-02
Stage m5q, manifest `docs/provenance/pkg6-carry-manifest.json` (262 files), ledger `docs/provenance/pkg6-carry-ledger.md` (generated).
- Carried in addition: the cycle's remaining `#[path]` children `native_install`, `native_fighter_install`, `process_memory` and the diag probes `body_bank_probe`, `source_lifecycle_probe`, `source_capacity_probe`, `source_ldn_capacity_probe`, `mii_pool_observer`, `mii_origin_observer` (all `ours-new`; under `src/runtime/roster/`), and the pure `mii_pool_observer_policy` (`src/model/mii_root/`).
- Tool: `log` joins the external crate heads (the carried INI loader's `log::info!/warn!/error!` map to `engine::compat::logging::log_*`).
- Module map: `consts.cycle_preload_config`, `consts.DEFAULTS_MENU`, `macros.MAX_RETAINED_CAPTURES` (stub constant, package 4 repoints), `sync.assign` / `sync.lock_write` (the generic `compat::sync` traits), `swap_native::scope_gate` (`roster::scope_gate`, flat).
- Recorded edits (each a ledger row): `character_roster.rs` (`notify` and `selection_label` take/return `Cow<'static, str>`), `kirby_hats.rs` and `character_items.rs` (`notice` takes a `Cow`), `character_items.rs` (`LEGACY_INDICES` is the identity: the item table is `training::character_specific::items::ALL_CHAR_ITEMS`, SPEC-state-04 section 2.1), `mii_ordinary_{player,cpu}.rs` (the two `format!` notices `.into()`), `hang_trace.rs` (the fork's `practice-heartbeat` thread `start` removed: the drain thread is package 1's, the late seam report runs on it through `diag_log::set_tick`, archive retirement is package 5's, the diag hook counts are `hook_stats`), `cpu_pilot.rs` (the two TEMPORARY edits of the staging commit are gone), `engine::lobby::automation::flow` (the fork's unfulfilled `expect(dead_code)` removed). `widen` (`pub(super)` to `pub(crate)`) on `native_hud_refresh`, `source_descriptor`, `source_visible_forms`.
- Not carried, written here: `runtime/roster.rs` (module declarations, feature gates of the fork's `swap_native/mod.rs`, `init` order, `handle_p1_input`, `CALLED_ROWS`), `runtime/lobby/mod.rs` (`init/update/observe/frame`), `runtime/compat/{input,menu_app,replay_store,replay_capture}.rs`, `runtime/lobby/training/character_specific.rs` (the item table is ours), the typed-menu additions (`trio_practice`, characters, rosters, Kirby hats, `ordered_kirby_hat_kinds`), `compat::menu` write-back (`assign(&MENU, ..)` and `lock_write(&MENU)` store the typed view and the menu worker writes the changed rows and kept keys back into the settings, `ui::worker::adopt_typed`), `Cpu::{request_rerolls, forget_hidden_entry, forget_hidden}`.
- Dependency: `paste = "1"` (Switch only, already in the lock through `skyline_smash`; the carried `character_items.rs` builds the turnip power thunk identifiers with `paste::paste!`).
- Interface stubs of package 4's layer, repointed at the merge of stream/pkg4 6e7ea3f (integrator-2): manifest keys `modules.root.replay_store` and `modules.root.replay_capture` now map to `crate::runtime::macros::{replay_store, replay_capture}`, the new key `modules.root.training::named_macros` to `crate::runtime::macros::named_macros` (the lobby facade `lobby::training::named_macros` re-exports it), and `modules.macros.MAX_RETAINED_CAPTURES` to `crate::engine::macros::MAX_RETAINED_CAPTURES`. The five carried files that name them were re-written by `carry` (`mii_catalog_request`, `mii_replay`, `body_bank_probe`, `source_capacity_probe`, `source_ldn_capacity_probe`; `verify` clean); the stub bodies (`compat/replay_store.rs`, `compat/replay_capture.rs` and the inert `named_macros` module) are removed.
- Fork lint attributes in the carried sources (to cite under D9 or fix, compliance list): `#[allow(clippy::large_enum_variant)]` in `character_roster.rs`, `mii_hud_portrait.rs`, `mii_editor_bridge/draft.rs`, `native_keyboard/inline_keyboard.rs`; `cfg_attr(..., allow(dead_code))` in `character_sources.rs`, `native_source_capacity.rs`, `native_keyboard/inline_keyboard.rs` (four).

## deluxe (the Deluxe plugin SDK under decision L1; the Replay capture row on the Deluxe Plugins page, Q7), 2026-11-02

Branch `stream/deluxe` from `27d84f4`. Playbook followed: `ae87443`.

**Licence (L1, operator 2026-11-02).** `intpa/ssbu-online-deluxe-rewrite` (https://git.syntaxbyexample.com/intpa/ssbu-online-deluxe-rewrite) is AGPL-3.0 at its root; its `sdk/` folder is MIT from rev **`20dd1cbf5201d2b23e653b1bd9798122981f53da`** (2026-11-02 06:03 -05:00, "License: the plugin SDK (sdk/) is MIT"; the commit adds `sdk/LICENSE` and `license = "MIT"` to the three crate manifests under `sdk/`). Checked at that rev before carrying: `sdk/LICENSE` is the MIT text, `Copyright (c) 2026 intpa`; `sdk/abi/Cargo.toml` (`deluxe-plugin-abi` 5.0.0) and `sdk/sdk/Cargo.toml` (`deluxe-plugin-sdk` 5.0.0) both say `license = "MIT"`.

**Boundary.** Fetched into a scratch folder (`/tmp/deluxe-sdk-20dd1cb`) as a blobless clone with a non-cone sparse checkout of `/sdk/` only, so no file outside `sdk/` was materialised. Nothing outside `sdk/` was opened, read, linked or copied (the SDK README links `../docs/PLUGINS.md` and `docs/VALIDATION.md`: not followed). Read in `sdk/`: `LICENSE`, `README.md`, `abi/` and `sdk/` (manifests, sources, locks); of `examples/demo` only its manifest's `name`/`version`/`license` lines.

**Carried, unchanged** (path dependency `third_party/deluxe-plugin-sdk/sdk`, optional, enabled by the `deluxe-sdk` feature, which is back in `default`; the upstream `abi/` + `sdk/` sibling layout is kept so the SDK's own `path = "../abi"` resolves with no edit; the crates' `Cargo.lock` files, the README and the demo are not carried):

| Here | Source (rev 20dd1cb) | Crate, version, licence | sha256 (source = copy) |
| --- | --- | --- | --- |
| `third_party/deluxe-plugin-sdk/LICENSE` | `sdk/LICENSE` | MIT, (c) 2026 intpa | `87e1195acf863f50e9a92c752d9ff9f5bb3c251b2183581de2a4761da92f96d7` |
| `third_party/deluxe-plugin-sdk/abi/Cargo.toml` | `sdk/abi/Cargo.toml` | `deluxe-plugin-abi` 5.0.0, MIT | `e484ec4ddb38bb756438f9a24a2839982687423fb672bb219b3a8d28a770c6bb` |
| `third_party/deluxe-plugin-sdk/abi/src/lib.rs` | `sdk/abi/src/lib.rs` | `deluxe-plugin-abi` 5.0.0, MIT | `a599220150f2791fe810e789ffec58437d12ba0d70982209f8231439d3b519cb` |
| `third_party/deluxe-plugin-sdk/sdk/Cargo.toml` | `sdk/sdk/Cargo.toml` | `deluxe-plugin-sdk` 5.0.0, MIT | `b1d9d8c4e464e787c2c59a90a2afeee1c83374d23f19e207d5ee084f5a7d9c3a` |
| `third_party/deluxe-plugin-sdk/sdk/src/lib.rs` | `sdk/sdk/src/lib.rs` | `deluxe-plugin-sdk` 5.0.0, MIT | `fa6271968040a3440a046224b3820dab0de03d972a4b659cadd453d82ccc7fac` |

Dependencies the SDK adds: none new (`skyline =0.6.0`, `nnsdk` 0.6.2, `libc-nnsdk` 0.5.0 are already in this lock, Switch only). The SDK is an `rlib` dependency of this `cdylib` (LTO applies). The pkg6 D3 rows above for `deluxe-plugin-sdk` 1.0.0 / 2.0.0 (`a63bc8c`, "STOP", "to check at C6") are superseded by this section: the fork's 2.0.0 snapshot is not used; 5.0.0 talks to the card's host directly (`IMPLEMENTATION-PROGRESS-deluxe.md`, ABI).

**Not carried, superseded:** the fork's `src/capture_plugin.rs` (ours-new, stage m5q; carried earlier by `tools/carry_pkg4.py` as `src/runtime/macros/capture_plugin.rs` but never declared). It is written against SDK 2.0.0 (`negotiated_descriptor_version`, host persistence of `replay_capture.ini`). Replaced by clean-room code written from the MIT SDK 5.0.0 docs and Q7: `src/model/deluxe_row.rs` (declared `engine::deluxe_row`; host tests against a fake of the card host's table) and `src/runtime/deluxe_row.rs`. pkg4 manifest changes (then `carry`, `verify` and `ledger` re-run): the file entry removed; `modules.root.capture_plugin` now maps to `crate::runtime::deluxe_row` (`live_replay.rs` re-derived: one path); six recorded edits to `replay_store.rs` drop the host-persistence verdict (`PERSISTENCE_COMPATIBLE`, `persistence_compatible()`, its log lines) because the row no longer asks the host to write the INI: the small-file writer thread is the file's only writer (lead ruling R5), so the menu row, the Deluxe row and `replay_capture.ini` agree (Q7).


## u4b (resident effect-texture helper), 2026-11-02
Nothing carried from the fork or from any frozen stage. Everything under `src/model/effect_texture.rs`, `src/model/sites_u4b.rs` (generated by `tools/gen_sites_u4b.py`), `src/runtime/ui/effect_texture.rs` and `src/runtime/ui/effect_probe.rs` is written from the owned 13.0.5 `main` ELF and the W7/W9 result notes (local evidence, cited, never committed); no upstream or fork file was opened. The `EMTR` field layout is W9's inference (`docs/U4B-RESIDENT-EFFECT-TEXTURE.md`).
## pkg6a: probe infrastructure out of the shipped NRO (2026-11-02, after integrator-2's merge `8dfc4fc`)
- `roster.rs` declares the fork's probe files only under their diag feature (D11): `native_unlink_probe` (`native-unlink-probe`), `practice_trace_core` (`console-practice-trace`), `hang_trace` (`hang-trace`; without it a four-probe no-op shim, the fork's disabled build). `effect_hook_chain`, `heap_metrics` and `heap_fields` are carried but not declared (no clean-room consumer: packages 2-4 hook the effect entries themselves, package 1's `runtime::alloc` is the allocator and `hook_stats` reports it).
- **Decision for the lead:** the fork features `console-practice-trace`, `native-unlink-probe` (implies it) and `plugin-heap-meter` each define a global allocator in the carried files (`practice_trace.rs:325`, `heap_metrics.rs:66`); with package 1's allocator the build fails ("cannot define multiple global allocators"). They cannot be enabled; keep them in the feature table (D11) as carried-not-buildable or drop them. `diag,hang-trace` builds.
- Recorded edits: `seam_install` loses `ImageLabels`, `Groups` (fields of the fork's own install line) and `own_hook_installed` (no caller); `turnip_identity::on_unload` removed (the provider has no unload hook; reads re-check the item code); `character_roster` unnecessary `unsafe` around `save_states::is_loading`. `effect_hook_chain.rs:87` is gone with the module's declaration.
- `roster::register_nro_load` registers `turnip_identity::on_load` with the nro-load provider (`add_nro_load_hook`), so a turnip's identity can be read for the save state (without the provider a turnip is saved as unsupported).
- Release warnings 262 to 121 (probe, ImageLabels and friends gone); the rest are consumers owned elsewhere (Mii list screen, Lobby Stages screen, Give-item, pkg4's macro layer) listed in the report.



## pkg6a: one global allocator and the owners of the unused helpers (2026-11-02, third successor)
- **One global allocator (lead ruling 1).** `runtime::alloc` declares the plugin's only `#[global_allocator]`: package 1's `CountingAllocator`, or, with `console-practice-trace`, the recorder's `TraceAllocator`, which forwards to it. `model::hook_stats` gained the byte meter (`HeapBytes`, `heap_bytes()`: live, peak, total, calls, largest request) behind `cfg(any(test, feature = "plugin-heap-meter", feature = "console-practice-trace"))`; without those features the counters compile away. Recorded edits: `practice_trace.rs` (the fork's `#[global_allocator] TraceAllocator` over `System` and its own LIVE/PEAK become the public forwarding `TraceAllocator`; `metadata` reads `heap_bytes()`), `heap_metrics.rs` (`Meter` and its allocator removed; `Heap`/`snapshot` read the byte meter; declared under `plugin-heap-meter`, read by `compat::logging::heap_profile` and a sparse `[HeapMeter]` line next to the `hook_stats` line). `heap_fields.rs` is no longer carried (its `heap_allocations=<calls>` is a field name package 1's install line owns). `tests/pkg6_features.rs` pins one allocator; `tools/feature_matrix.py` compiles the feature combinations (every feature on the default set and alone, the allocator features together; host `cargo check` or `cargo skyline check`).
- **Unused helpers (lead ruling 2):** each is mapped in `docs/provenance/pkg6-carry-ledger.md` ("Deleted fork functions", "Clean-room natives deleted", "Wired to the clean-room owner of the moment", "Carried helpers waiting for a screen"). Deleted by recorded edit: `cpu_pilot::thinking` (+ its flag and scope-contract row), `character_sources::reserved`, `heap_fields`; package 3's seven item natives of `state_natives` (held-item restore is the item engine's). Wired to the owner of the moment: `kirby_hats::{restore, cancel_pending}` and `character_roster::solo_popo_active` from package 3's load (`features::states`), `cycle_preload_config::publish_applied` from `compat::menu::apply_snapshot` (the menu commit). `recorder::phase` is diag-only (its one caller is `macros::diag`) and is gated with `diag`.
## stream/sync (2026-11-02)

Nothing new carried. Recorded edits to carried files (manifest + regenerated ledger): `src/runtime/mii/native_keyboard.rs` (pkg6: `use parking_lot::Mutex` -> `use hos_lock::Mutex`). `crates/hos-lock` is new code of this repository (MIT), not carried. `parking_lot` and `parking_lot_core` left the build (`tests/no_spin_locks.rs`).

## slim (code size), 2026-11-02

Not carried: no module reaches it (no `mod` line or `#[path]` declared these files, so they were never compiled; lead ruling
2026-11-02). Deleted from `src/` and moved from the pkg6 manifest's `files` to its `not_carried` list (the ledger rows removed
with them); stream/pkg6 confirmed its WIP does not reach them.

| Was here | Fork path (stage m5o) | src sha256 | Lines (`tools/sloc.py`) |
| --- | --- | --- | --- |
| `src/runtime/lobby/automation_helper/native.rs` | `src/automation_helper/native.rs` | `62851fc8c4e245ae` | 425 |
| `src/runtime/lobby/automation_helper/plugin.rs` | `src/automation_helper/plugin.rs` | `cf5371be7299d337` | 58 |
| `src/runtime/roster/effect_hook_chain.rs` | `src/effect_hook_chain.rs` | `100929d53a938c4e` | 73 |

`mii_native_adapter.rs`, `mii_cpu_adapter.rs` and `mii_closed_ack.rs` stay: stream/pkg6's `mii_fullconfig_probe` reaches them
through `#[path]`. `runtime/macros/capture_plugin.rs` is stream/deluxe's (deleted there with its carry record). The comment at
`src/runtime/roster.rs:13` still names `effect_hook_chain` as carried and not declared; it changes after stream/f6's mod lines
next to it are merged.

### Seam table: rows not carried (lead override of D6, 2026-11-02)

Not carried: no consumer (lead override of D6, 2026-11-02). 82 rows that no code installs, checks, names or reports beyond
the census count, deleted by recorded edits of the pkg6 manifest (one `delete_from`/`delete_to` per row, from the row's own
generator comment to the next row's), so `carry_pkg6.py verify` still re-derives every file from stage m5q: `images.rs` 37
(`console_load_probe` 28, `console_map_probe` 6, `training` 3), `rows_a2.rs` 25 (`mii_quit_observer` 14,
`mii_preview_observer` 11), `rows_a4.rs` 11 (`automation_helper_native`), `rows_a3.rs` 7 (`training`), `rows_a1.rs` 1
(`kirby_resource_probe`), `rows_a5.rs` 1 (`console_map_probe`). Proof: `python3 -I tools/seam_census.py <tree>` in the slim,
work, pkg6, pkg4, ui-screens, hitbox, deluxe and f6 trees (2026-11-02): each deleted row has no consumer in any of them
(label literal, offset literal, `SITE_<NAME>` constant, or another row naming it), apart from the 11
`automation_helper_native` rows, whose only reader was the never-compiled `automation_helper/native.rs`.

Also recorded: `src/model/seams/native_seams/mod.rs` re-exports `model::guard`'s rule set instead of its copy (five edits).
`tools/carry_pkg6.py ledger` lists a reason shared by many edits of one file once.

### smash-match-data keeps what the plugin uses (lead ruling 2026-11-02)

Recorded pkg4-manifest edits (`tools/carry_pkg4.py verify` re-derives every file from its source): `capture.rs` (the
reference `Recorder`) and `clip.rs` (clip resolution; its inherent `impl Capture` becomes the trait `CaptureClip`) move to
the test-only oracle crate `crates/smash-match-data/tests/oracle` (`smash-match-data-oracle`, a dev-dependency of the crate
and of the plugin); `archive.rs` loses `Capture::load`/`save` and `validate.rs` loses `validate_capture`, both now
`tests/oracle/src/whole.rs` (written here from those removed lines; `CaptureExt` keeps the method syntax); `lib.rs` and the
crate's `Cargo.toml` follow; the crate tests `core.rs`, `review_correctness.rs`, `review_selection.rs` and the plugin's
`shared_capture.rs` tests reach the moved parts through the oracle. New in `shared_capture.rs`'s tests: golden files
(`tests/fixtures/smd-golden`) pin the published capture bytes, read by a plain JSON reader.

## Slim probe retirement (2026-11-02)

Not carried: probe served (pkg6 confirmation, 2026-11-02). The source-capacity experiments and lifecycle attribution have no shipped/test consumer. Their inline sites are exact manifest edits; shared admission, native guards, creation/load calls, source-book release and ownership cleanup remain. Historical seam provenance is retained.

| Retired destination | Authorized stage path | Stage SHA256 |
| --- | --- | --- |
| src/runtime/roster/source_capacity_probe.rs | src/source_capacity_probe.rs | a8a341ab56a3cfe72c053a15fdf8845570faa17b30f1aa56e05d46b88bbcb00e |
| src/runtime/roster/source_ldn_capacity_probe.rs | src/source_ldn_capacity_probe.rs | e585bd004a12d0f60c0f16e48cc76a8d84b66dd6d59953793beaad5fd1837d3c |
| src/runtime/roster/source_lifecycle_probe.rs | src/source_lifecycle_probe.rs | 3c76da55af50bda1788dd55466cd858f4b7e88a3d0874d3613bd1681a9d870a2 |

Cargo also retires eight empty diagnostic aliases: console-map-probe, archive-poison-probe, kirby-resource-probe, lobby-stage-probe, practice-stage-observer, stage-asset-probe, mii-preview-observer and mii-quit-observer. The ten source-capacity features and source-lifecycle-probe are removed with their experiments. Product lobby-stage/practice-stage features remain. The matrix drops only the two exclusions belonging to deleted experiments. Texture-probe remains pending UI proof, as do all unresolved diagnostic tools.

### Slim served Mii experiments and helper scopes (2026-11-02)

Removed seven diagnostic features: `mii-identity-probe`, `mii-saved-file-probe`, `mii-fullconfig-probe`, `mii-native-adapter-probe`, `mii-cpu-adapter-probe`, `mii-identical-reload-probe`, `mii-special-change-probe`. Shared native access, saved/ordinary behavior and the existing exact write-set regression remain. Every carried change is a manifest transform; native helper predicates follow the real feature consumers, with test-only APIs retained under test. No lint suppression added.

| Retired source | Source SHA256 | Former destination | Reason |
| --- | --- | --- | --- |
| src/mii_identity_probe.rs | 239d5fbaf5ba3f9603ea8f2cb8714348d15fd3a72bdc40f51ef4a832adc5187c | src/runtime/mii/mii_identity_probe.rs | not carried: served Mii diagnostic experiment retired per approved slim handover; shared saved/ordinary primitives and write_tokens regression retained |
| src/mii_saved_file_probe.rs | f6281e7b3d09f0e4a261ec5eec717d4e70908d06d01b9c634bb8856a964da126 | src/runtime/mii/mii_saved_file_probe.rs | not carried: served Mii diagnostic experiment retired per approved slim handover; shared saved/ordinary primitives and write_tokens regression retained |
| src/mii_fullconfig_probe.rs | 3e57c55ecd72ac31dd62a7da9c032b12507d57d1847ec97006b35d4c23579b8e | src/runtime/mii/mii_fullconfig_probe.rs | not carried: served Mii diagnostic experiment retired per approved slim handover; shared saved/ordinary primitives and write_tokens regression retained |
| src/mii_native_adapter.rs | a829b0ac0f55068fb7fa2c2e2b2624f25246fbdf2550c09dd7e592912834a5a9 | src/runtime/mii/mii_native_adapter.rs | not carried: served Mii diagnostic experiment retired per approved slim handover; shared saved/ordinary primitives and write_tokens regression retained |
| src/mii_cpu_adapter.rs | 6008e0fe8cbb0e4033ea494a3d67a4e7125c712b73ff4a99836c2da80afa1be2 | src/runtime/mii/mii_cpu_adapter.rs | not carried: served Mii diagnostic experiment retired per approved slim handover; shared saved/ordinary primitives and write_tokens regression retained |
| src/mii_closed_ack.rs | eb6851a41ba1822e1afd3c6d2958af7470e69b660f9b1e2ca3b0f211daadd61b | src/runtime/mii/mii_closed_ack.rs | not carried: served Mii diagnostic experiment retired per approved slim handover; shared saved/ordinary primitives and write_tokens regression retained |

The shipped stage services are renamed lobby_stage_probe.rs -> lobby_stage.rs and practice_stage_observer.rs -> practice_stage.rs through destination/module-map transforms. Historical seam group strings, guards and behavior stay unchanged; lobby declarations and the notice-header test use the new paths.

### Slim helper feature scope (2026-11-02)
The broad dead_code allowance on mii_ordinary_native::helpers is removed through pkg6 manifest edits. Helper::Ready, its token/key imports, accepts arm and parked argument compile with mii-managed-source-coexist-alpha or tests, exactly where Ready constructors exist. Native helper_kind uses the same predicate for the managed/parked calculation and argument. Enabled product and test decision bodies are preserved; disabled coexistence no longer compiles an unconstructible variant. This changes no native ownership or transaction rule.

### Slim macro name payload ownership (2026-11-02)

The pkg4 macro panel now boxes the large macro-only payload instead of the whole
name draft. Name, caret and grid state remain inline, including when parked for
placement; the exact Mii configuration remains owned by the macro payload.
A macro draft still has one payload allocation, while external naming no longer
allocates an outer draft. No validation, confirmation, placement or retry behavior
is removed. The large-enum suppression and its KNOWN row are removed; all 18
transforms are recorded in the pkg4 manifest and the ledger is regenerated.

## Slim keyboard value boundary (lead scoped authorization21:18Z)

The actual Text/Decision/callback-value state moves from the carried native
inline_keyboard adapter into src/model/mii/inline_keyboard.rs, consumed by that
adapter and host probes. Both destinations derive from the same authorized
ours-new m5q source via exact manifest edits. The pure entry precedes the native
entry, so the tool's source-to-destination module mapping still resolves the
native keyboard child to its SDK adapter. The carried mii module adds one
declaration. No upstream or mixed source was opened.

Native Changed/Moved/Entered/Object/Work/InputText/Appear layouts and SDK
declarations remain unchanged. Only value validation/decision handling moves;
lease, locking, UI-thread ownership and Finalize ordering remain in the adapter.
The initial extraction intentionally retains usize metadata and allocating
macro validation to reproduce the original defect with the actual shared code.
It is a regression baseline, not a completed fix or runtime acceptance.

Keyboard root follow-up: pure value carry transforms now use checked u8 metadata and allocation-free UTF-16 encoding/validation. The ten-unit raw Mii contract, 48-scalar Macro contract and cursor boundaries are preserved by original exported-code probes. Only the superseded keyboard large-enum lint row is removed.

Editor-only matrix correction: mii_ordinary_cpu_native original-kind match now applies the same existing coexistence/test cfg as Helper::Ready. This repairs a missing arm condition from the earlier slim feature-scope cleanup; full-feature behavior is unchanged. Actual failed E0599 and corrected carry253 verification retained.

Lead21:59 authorized exact native keyboard Signals mutex replacement with hos_lock const storage; native cold OnceBox allocation proved in78f440d ELF. Only import and guard unwrap transformations; SDK/lease/guard extents unchanged.
## ui-screens — owned Mii list adapter (2026-11-02)

The existing `src/model/mii_root/mii_menu.rs` (ours-new carried from frozen m5q; source sha256 `94d7a41fe2ff5d3c4dcaed6ff0e87df4c96d64bd927a99e4550a198784cb761c`) now supports the approved direct class list with an Add New row and ZR role switching. The original selection, eligibility, exact identity and menu-close action policy remains its owner. Changes are exact-text transforms in `docs/provenance/pkg6-carry-manifest.json`, applied with `tools/carry_pkg6.py carry`; the generated ledger was refreshed and `verify` passed all 264 files against m5q. No fork or stage file was edited.

`src/editors/mii.rs` is new clean-room presentation/worker adapter code. Its tests create synthetic owned saved-Mii records and exercise the real carried policy. Runtime native lifecycle/actions and bust binding are still being wired and are not accepted yet.
The ui-screens portrait transform also extracts the existing native key-builder call in character_roster.rs into one shared helper (waiting-icon preload and roster UI); its manifest/ledger records the edit. Uses owned game assembly and existing seam guards, not a new pathname provider. Runtime evidence pending.

UI portrait admission correction after ui-13: the existing enforcing character_roster seam gate checks roster.kind_hash and roster.icon_hash before publishing BASE. The menu helper now uses that readiness; core game::Granted has neither carried seam and previously refused every portrait. Exact manifest transform, regenerated ledger, full264-file verification; no guard bypass or fork/stage edit. Runtime portrait acceptance pending.

### UI item picture assets (2026-11-02)
38 PNGs and sources.json, identity-copied from frozen m5q scripts/item-icons/assets, all ours-new verified before reading. Full digests/transforms: docs/provenance/ui-item-assets.json; generated ledger: docs/provenance/ui-item-assets-ledger.md. tools/carry_ui_items.py verifies binary files (the existing Rust carry tools rewrite UTF-8 text). No fork/stage mutation. The pinned MIT layout builder appends item textures/materials to both Training and waiting archives; stage/fighter pictures still load at runtime. Runtime/recipe gates pending in stream progress.
## Lead exact-Mii state capture, 2026-11-02
Clean-room runtime/features/states.rs now passes a Copy Configuration from the
existing settled role accessor to the writer, which serializes it off-hook.
model/state/record.rs gains writer-only attach_mii with failure atomicity;
model/state/file.rs projects imported records to a JSON-free game copy while
the writer retains the complete opaque identity. No additional fork source
was copied. The existing Mii Configuration serialization contract was read in
the carried model; current_configuration still requires the separate native
first-lock root. Source contract: manifest-verified ours-new m5i
docs/setup-library.md d178081ae95cf30726273c2c0178df99d542ffd9d977ce75042ce9fe796be17c
and src/training/setup_library.rs 4362b966d43d4725f330228b2b50ce64966ab117bf88ee0b0660efc0252cf9fb.
This is not native acceptance or permission to relax exact identity checks.

## Lead preview checkpoint, 2026-11-02
Clean-room implementation from manifest-verified ours-new m5i docs/named-macros.md (c587e3e7cb61710e30dc7374914000ee5fd3e86dbd8da70be8e5348b1671d299), docs/reference/named-macros.md (11a1c3eac4c921cef8799f4b634aefdbeb6b2706b607dfac4876e02c1a0fa0d4) and docs/named-macros/SCREENSHOTS.md (723ad2c4208ed94d56134ac064e80a2c9855bfa54b1a881e2ff9cd4d21b7343c). No mixed save_states implementation read or copied. Existing capture native reads extracted without changed ordering into read_record; checkpoint/identity and retirement transport are new. Source and gate limitations in IMPLEMENTATION-PROGRESS-state-core.md.

## Lead Setup transaction provider 2026-11-02T23:07:07.668307+00:00
Frozen e12ed5c472f972e8615d7b675086422527486ffe3e28c59b45192f1aecb33b2b,1115files; archive23a50b85ec91476e4ebeef3a5f6f0b32e7c7fb27c29d3a0484106b6dc7ff5119. Target exclusive trainingrooms-state-core-setup-r2, refreshed source mtimes; ext4 TMPDIR. Targeted actual155/3/155 includes seven Setup model regressions and first/repeated gamephase zeroalloc; clippy alltargets-Dwarnings0/44.01s, no-default0/2.92s. All before/after source, live source, logs and three binary hashes independently verified. Logs [('targeted', 'febabd2cce5deb840aa45339d34d07352e09b153eb785efb2eb5de20dea56284', 3), ('clippy', '2d8708ae2fdac9e039b480aa8cba1e99f392a1e183063a1882528781d7140afd', 0), ('nodefault', '71ce1f442b0e59bb541f1d7f51c780407f9a7ad527d438cb542cc1c255acc2df', 0)].
FrozenTarget captures session/slot/enable; worker validates/decodes exactMii into JSON-free Prepared; game rechecks current ownership/idle/enable/stage/kinds/exactMii then binds both halves atomically, optional standard restore uses frozen slot. Y bind works while save-state option disabled. Export snapshots actual saved values/typed savedMii, not live fighters. Setup jobs use single-ticket stable Box exchange with writer merge in globalcapture sequence; no Setup bind enters ordinary dropping8-mailbox. All allocation/serialization/Boxretirement worker-owned. Persist uses existing writer retry; reply means mirrored binding, not synchronous durable fsync.
Session defect found by UI review: practice active-edge clear now invalidates facts only; existing named_macros::waiting_scene_changed(false) retiresSetup epoch under STATE. Same-lobby true preserves originaltarget/draft; actual new lobby invalidates without silentlymintingtarget. This one-line named_macros consumer hunk must be retained by pkg4. Pendingstale job can retire onworker withoutgame callbacks; committedbinding still drainswriter. UI acceptedticket must poll afterpanelclose.
Superseded b503 setup-r1 threehostgates retained as development evidence only; tracked-only incomplete setup-r2 archive retained separately, never built. Runtime Setup provider not Switchcompiled yet, UIadapter notwired here, nativegetter coldlock/stack/lifecycle/full Eden0Unmapped stillOPEN. Exact APIs open/request_snapshot/request_bind/validate_selection/poll published in inbox. No lead claim/Eden/profile after23:06.
## hitbox (SPEC-core-06), 2026-11-02
No file is carried (stage, path, sha256: none). Everything of the visualizer is clean-room, written from `specs/SPEC-core-06-hitbox-visualizer.md` and the owned 13.0.5 ELF
(`docs/test-provenance/core-06.md`). Used from carried code without edits: the seam rows `hitbox_visualizer.*` of `src/model/seams/native_seams/rows_a3.rs` (`ours-new`, stage m5o
sha256 recorded under package 6) were read for the three hook offsets and their bytes, then re-derived and re-checked against the owned ELF by `tests/core06_hitbox.rs`
(the visualizer's own rows are `src/model/sites_hitbox.rs`; the carried seam group stays unconsumed, as before); `src/runtime/lobby/training/practice.rs::visualizes`
(`ours-new`, package 6) is called as the lobby-practice gate. Edits to shared files, each with its reason: `Feature::Hitbox` (`sites.rs`), `HookId::Hitbox*`
(`hook_stats.rs`), the table chain and `CROSS_NEEDS` (`capabilities.rs`, generated), the pass call in `cpu::on_frame`, one `publish` line, `game::natives` additions, boot
(compare, install, report), `tests/capabilities.rs` (unit mapping and one boundary).

## F6 roster input allocation root (2026-11-03)
The two Mii ordinary intercept adapters now use the existing fixed-storage roster
for their four Mii membership predicates. Exact transformations and before/after
identities are recorded in pkg6-carry-manifest.json and its generated ledger.
Setting snapshots, ordering, membership, actor/receipt/epoch checks and transaction
behavior are unchanged. Other roster consumers remain outside this scoped change.
Original host allocation/order regression and native acceptance are recorded in
IMPLEMENTATION-PROGRESS-f6.md. No new fork source or mixed file was read.


## Q15 bounded diagnostic context observer (2026-11-03)
Adopted exact Slim23ec705 code and UI d029583 existing-callback reference after lead source and owned-ELF guard review. Only native_seams/mod.rs carry entry changes: preserve main exact composition transforms and append the two row-table edits. Worker historical transforms differ but their before-state bytes/SHA match main exactly. diag_log gains a diag-only monotonic writer-ready query; clean-room mii/mod.rs installs the diag-only observer. Release rows are empty; neither observer calls the texture constructor or modifies native arguments. IDs remain opaque; same-boot context, native compile/stack/trampoline review and runtime proof are pending. Admission/filter tests: tests/mii_bust_context.rs. UI anchors is clean-room.

## Capacity pristine verification ordering (2026-11-03)

Adopted the bounded Slim 07f80c4 repair: seven appended transforms across character_roster and native_source_capacity preserve the existing main carry history. A non-copyable, base-bound proof verifies full load ranges before lifecycle hooks; activation remains after ordinary Mii validation with strict creation and seam gates. The pure helper and four original probes are clean-room. Native/runtime qualification remains pending; see docs/CAPACITY-PRISTINE-LOAD-CONTRACT.md.

## pkg6 retained sources, fixed CPU seats and exact created destinations

Integrated reviewed cumulative 1dd2629 composition plus 0bb452d native-owned seat admission. Seventeen existing carry rows receive append-only transforms, followed by three corrected rows. Preserved F6, pristine-load, developer measurement gating, Q15 and Hitbox observer changes. Known-empty native-owned seats require fresh exact active identities and epoch/count/entry rechecks; they grant no managed source token. All managed-generation and capacity resource proofs remain. Source only: integrated tests/native callback ABI/runtime and four-owner HUD remain pending.

## pkg4 first-seal enqueue boundary (2026-11-03)

The existing carried diagnostic formatter/filter/timestamp/ring-push body in
`src/runtime/diag_log.rs` is extracted into an enqueue helper. The normal API
retains its pre-writer synchronous drain and filtered-line short circuit. A
diag/test-only enqueue entry lets the first-seal worker transfer its formatted
bytes to the existing ring without invoking that drain. No new state, capacity,
logging policy, or stage source was added/read. The live_replay call is recorded
as an appended exact-text pkg4 carry transformation; the runtime logger has no
carry manifest (see its original scoped mixed-file attribution above).

## pkg6 RC reset: withdraw unproven F9 and exact-created portrait admission (2026-11-03)

Operator RC reset and lead10:42:04 authorize this deferral. Exact base is
`8fe6261`; ordinary roster/group/saved-CPU-Mii paths, parked helper hat/item
requests, native count handling, save/scene cleanup, INSTALL/pristine checks and
achieved allocation repairs remain. Removed the clean-room-only count-hook
module, its change-budget diagnostic module and its dedicated seam table;
removed their declarations/install call and test includes. None was an entry in
the carried-file manifest, and older read-only CPU-quantity guards remain.

The existing pkg6 carry manifest records appended exact-text removals for the
fixed-seat producer and scheduler gate, consumer cutover, active reload,
promoted presentation/provenance and active-seat admission. Ordinary Mii
portrait source selection again uses the backend's parked helper pair rather
than the deferred exclusive created rows. Created exact-Mii scheduling,
eligibility and native preparation all consult the explicit closed RC policy;
the ordinary created-source capacity/pristine/load/retirement paths remain.
No broad Cargo feature toggle, stage mutation, native flag workaround, ABI or
save-format change. Pure historical promotion models are not native authority
and have no remaining runtime caller in this composition.

All carried source changes were generated through `tools/carry_pkg6.py carry`
from these transforms, not by editing a stage. The ledger is regenerated;
`verify` re-derived all253 files. Source parsing and carry verification are not
host/native/runtime acceptance. Four RC regressions in the existing library
test module cover removed registrations, native Mii/count scope, native-vs-created
scheduling and ordinary portrait row exclusion. They await the one coherent
candidate host gate before Eden; no scratch Cargo/native/runtime run was made.

Lead composition: cf161f5 is integrated after Slim7950485. Both count and Hitbox-observer seam tables/declarations are absent (seven retained tables); both count and fixed-bank-probe install calls are removed. Appended exact carry transforms express the reviewed combined delta from8c0ec63, retaining all Slim exclusions and unchanged original source hashes. Regeneration preserved every reviewed destination byte; all247 remaining files re-derived successfully. The worker253-file result above describes its isolated branch, not this combined inventory. Candidate host/native/runtime gates remain pending.

### RC reset pkg4 provenance correction (2026-11-03)
Slim full rederivation on9c65c2e found two DIFFERS: named_macros.rs and replay_capture.rs. The reviewed11ee226 workflow closure had omitted its manifest edits. Recorded exact11ee226-parent-to-product transforms, regenerated the existing ledger, rederived all41files, and compared every resulting destination byte to the reviewed pre-regeneration source. All41match; original source SHA pins unchanged. No behavior change or acceptance inferred from the earlier failed check.

### RC native-editor exposure closure (2026-11-03)
Added an explicit adapter capability: production MiiList hides Add New and cannot create EditorRequest, while ordinary saved selection, role permissions and close actions remain. Generic enabled CatalogMenu behavior/tests are retained. Native request and begin_session independently refuse the disabled RC capability before native work/session publication, including resume bypass. Shared catalog mount/read and ordinary CPU/group services remain. Two carried files have exact recorded transforms; all247files rederived with reviewed destination bytes and original source hash pins preserved. Adapter regressions cover saved CPU selection/close with no extra row and empty unqualified context without editor dispatch. Full candidate Cargo/native/runtime gates remain pending.

## p23/size-cut: `ui_anchor` not carried (2026-11-04, cut option; lead decision pending)

Not carried: no user. No module, test or tool references `crate::model::macros::ui_anchor` (`docs/SIZE-PLAN.md`
"macros / recording", first row: "no user (M)"); the clean room's UI anchors are `model::sites_ui` and
`runtime::ui::anchors`. Deleted from `src/` and moved from the pkg4 manifest's `files` to a new `not_carried` list (the
pkg6 precedent, `e79019b`); `src/model/macros/mod.rs` gains the exact-text edit that drops `pub mod ui_anchor;`, and the
ledger is regenerated. `carry_pkg4.py verify` (against stage m5q): verified 40 files.

| Was here | Fork path (stage m5q) | src sha256 | Lines (`tools/sloc.py`) |
| --- | --- | --- | --- |
| `src/model/macros/ui_anchor.rs` | `training_mod_macros/src/ui_anchor.rs` | `3e4118ca933404e4` | 134 |

## p23/mii-editor: native Mii editor available again (2026-11-05, rc.3)

The operator asked for the fork's Mii editor, which had no entry in the menu, to be brought over (primary 35137355,
2026-11-05 ~14:16Z). Every file of the fork's `src/mii_editor_bridge/` and its Lua helpers was already carried (pkg6
ledger); only the 2026-11-03 closure above kept it shut. `NATIVE_EDITOR_AVAILABLE` is now `true`:

- The Mii lists show *Add New…* again, and `request` / `begin_session` admit Training, Quickplay and Local Wireless
  waiting practice, as in the fork.
- `install::run` runs `init_editor` once the editor's 64-byte seam rows pass (`mii_editor_bridge`, enforcing).
- Save is unchanged from the fork: `edit_decide` sends only a fresh draft to the game's allocator (a new slot) and reuses
  an identical saved Mii. A saved ID never enters, so no saved Mii or protected original is overwritten.
- The capability switch and both refusals stay; with it `false` the closure's behaviour returns.

Two carried files changed only by recorded exact-text edits: the constant and its comment in `mii_root/mii_menu.rs`,
and two comments in `mii_editor_bridge/mod.rs`. The ledger was regenerated, and `carry_pkg6.py verify` re-derived
all 247 files from stage m5q byte for byte. The adapter tests in `src/editors/mii.rs` check Add New after the saved
choices, Add New on an empty catalog for each class (native default seed, no role permission), and the refusal when
the capability is off. `tests/mii_install_boundary.rs` pins the constant at `true`. Not carried, as before: the fork's
Lua test files (`lua_helpers/tests/`) and the diag Mii Test observers. Runtime proof is pending: the rows for
the combined rc.3 Eden session are in `artifacts/phase23/rc3-combined/rows-mii/`; the waiting-room origins and the
resume after a match have no row.

### p23/mii-editor: the rest of the fork's Mii behaviour, ported directly (2026-11-05)

The operator asked whether Mii creation matches the fork (interruption, replay Miis matched against saved ones, and
so on), then that the port be a direct one: it is our own code. A fork-against-here comparison found every editor,
replay-Mii, selection, cycle, HUD-portrait and identity path carried and reachable in a release build. The behaviours
that were missing or different are now ported from the M5q stage, each citing its fork lines:

- Add New with a Player/CPU choice from the same opening drops that opening's reloads and requests the editor before
  the catalog close (`containers/app.rs` 923-931, `common/menu.rs` 483-491): `MiiList::close_for`, `menu_handoff`.
- A portrait pick drops that role's Mii reload (`app.rs` 522-531): `close_for`'s `picked`, from the worker.
- Stored Mii choices are parsed one selector at a time (`training_mod_consts/src/menu_json.rs` 17-37):
  `engine::compat::menu::mii_preferences_of`.
- Mii tiles in Character Cycles, the portrait picks and Kirby Hats follow the catalog's admission: greyed and refused
  when not admitted, numbered when admitted, the Mii slot-0 rule (`app.rs` 563-685 and 745-800,
  `training/ui/roster_grid.rs` 197-206 and 304-308): `editors::roster::MiiTiles`, `MiiList::tiles`.
- Waiting-room Add New needs the installed editor (`ENABLED`, as the fork's `mod.rs` 204-213).
- An opening with no Mii catalog says so instead of "Reading…" forever (the fork showed no Mii pages then).
- `mii_editor_bridge.called_376d990`: an exact 64-byte row for the Colour selector the resume path calls, in the
  editor preflight. The fork had no guard for it.

Carried-file changes are recorded manifest edits. Fork tests carried: `replay185_headgear`, `ldn_backend` and
`quickplay_backend`; `source_provenance` is recorded as not carried, with the reason. `carry_pkg6.py verify`
re-derives all 250 files from stage m5q. The list, with what stays different and why, is `docs/MII.md`.

## p23/seam-self: seam checks that know this plugin's own writes (2026-11-05, rc.4)

A seam check could not tell this plugin's own hook from another plugin's: rc.2's Change Stage pick did nothing because
`practice_stage`'s enforcing group read our own `lobby_stage` hook at `0x25f7874` as `occupied` (`5d19922` restored the
order only). Now the installer records, for every site a hook row of any table writes, the 20 bytes there before the
boot wrote anything (`seam_install::prime`, before the first hook) and the bytes our writes left
(`seam_install::seal`, at the install line). A check puts our own writes back before it compares
(`model::own_patches::unpatch`). Another plugin's patch is never put back: one from before our boot is in the "before"
bytes, and one over ours no longer matches the "after" bytes. A row that still reads `occupied`, `foreign` or
`out_of_region` is named once in the shipped log (`[TrainingSeams] seam=… group=… offset=… outcome=…`), and the seal
writes `[TrainingSeams] own_writes=<n> sites=<m>`. There is no other behaviour change.

Carried-file changes are recorded pkg6 edits: `native_seams/mod.rs` (`PATCH_ROWS`, `PATCH_SITES`), `native_seams/report.rs`
(`names_row`, `RefusedLine`) and `runtime/seams/seam_install.rs`. `carry_pkg6.py verify` re-derives all 247 files.
Host tests (`engine::seams::own_write_tests`, `model::own_patches`) cover the practice stage group checked in the wrong
order (it verifies), every real row with every hook row's site hooked (each reads its 13.0.5 bytes), and another
plugin's hook over ours (`occupied` again). The report is `artifacts/phase23/LANE-SEAM-SELF-REPORT.md`.

## rc.5: the Mii list marks the picked Mii with a character the game draws (2026-11-05)

The Mii list marked the picked Mii with `✓ ` (U+2713). None of the game's 14 fonts in 13.0.5's data.arc maps it (the
menu's Latin face, `nintendoP_RodinNTLG` B003/EB003/UB003, has 8,612 characters each; read with
`artifacts/phase23/rc5-combined/glyphs/font_cmap.py`), so the list drew an empty box before the name
(`docs/media/mii-editor-list.gif`). It now uses `> `, the mark the roster editor, the combo list and the Direct stage use
for their pick. Carried-file change: one recorded pkg6 edit of `src/model/mii_root/mii_menu.rs`; `carry_pkg6.py verify`
re-derives all 250 files. `tests/ui_glyphs.rs` fails on any character in a non-test string or char literal under `src/`
that is neither ASCII nor in its font-proven list (… · Ω — § é, each mapped by all three Rodin weights).
## p23/hot-slots: a capacity refusal holds an extra group instead of failing it (2026-11-05, P-HOT)

The fork fails an extra group when its prefetch start finds too little process headroom or too few free pool bodies
(`fail_group`, `GroupState::Failed`: "never retried in this scene") and the Fighters notice reports it as "failed".
Eden `tr-p23-hot-measure-1` (release `031f1eae`) showed "Loaded 2/5 (3 failed)" for the CPU group `[2,3,4]` refused
for headroom (530.5 MiB needed, 477.6 MiB free) while helper sources still held the look-ahead loaded during a Mii
stretch (`artifacts/phase23/LANE-HOT-SLOTS-REPORT.md` §8). Here the group is held (`GroupState::Held(Hold)`): it
keeps its place in the plan and the body budget, holds no memory or keys, is started again by the lifecycle after every
planned group (held retries rotate, `body_bank::next_start`), and the notice says "(N waiting for memory)" or "(N
waiting for pool space)". A swap that reaches it is handled like a planned group. 36 recorded manifest edits of five
pkg6 files (`cycle_preload_policy.rs`, `body_bank.rs`, `body_bank/memory.rs`, `swap_model/fallback.rs`,
`native_body_bank.rs`); `carry_pkg6.py verify` re-derives all 250 files from stage m5q. Tests:
`frame85_group_refused_by_headroom_then_admitted`, `held_group_counts_apart_from_failed`,
`held_groups_retry_after_planned_and_rotate`, `held_fighters_read_waiting_not_failed`,
`held_group_at_a_swap_is_disposed_like_planned`, `tests/group_hold_wiring.rs`.

### p23/hot-slots: Ready Fighters defaults to 10 (2026-11-05, P-HOT §3.2)

The fork defaults Ready Fighters to 8 (`cycle_preload_config.rs`, row 102 above). P-HOT sets 10, both active fighters
included (DECISIONS 2026-11-02, the operator's "ten total hot fighters"): 3 recorded manifest edits of
`cycle_preload_config.rs` (the constant and its test), `factory.rs` index 8, `tests/core02_menu.rs`. Saved settings keep
their value; only a new or reset profile gets 10.

### p23/hot-slots: Ready Hats, hats off the fighter budget, helper entries by purpose (2026-11-05, P-HOT §5.1)

The fork charges Kirby-hat helpers against Ready Fighters (`GroupBudget::hats_wanted`) and lets any helper target take
any helper entry. P-HOT (DECISIONS 2026-11-02: "Hat sources are not charged against the fighter budget"; operator
decisions "B" and "1", 2026-11-05): `hats_wanted` is gone from `GroupBudget`; the helper plan is budgeted by
`preload::hat_slots::budgeted` (Ready Hats, a new clean-room row, 0..=5, default 5); Training's helper entries take
targets by purpose (`hat_slots::slot_view`: created 7/6/5/4 hats first, entry 2 fighters only, entry 3 the fifth hat
only through the borrow), and the borrow record F9 reads is `character_sources::ENTRY3_BORROW`. 18 recorded manifest
edits of `body_bank.rs`, `character_sources.rs`, `cycle_preload.rs`, `native_body_bank.rs`; `carry_pkg6.py verify`
re-derives all 250 files. Tests: `hat_slots` (3), `hats_are_not_charged_against_ready_fighters`,
`ready_hats_row_defaults_to_five`, `tests/hat_slots_wiring.rs` (2).

### p23/hot-slots: the start-up window built into Training's own start (2026-11-06, P-HOT B)

The fork builds a fighter's body bank only at a visible swap: Training starts every visible role with one body and the
notice says "Swap once to load". The operator asked for the characters to be "just ready from the get go", as the
game does for Pokémon Trainer, and chose B (2026-11-06). The research boot `tr-p23-hot-measure-3` measured the scene
path (`artifacts/phase23/LANE-HOT-SLOTS-REPORT.md` §12.1). P-HOT B, behind a new seam row `body_bank.scene_header`
(64 guarded bytes at `main+0x66de14`) and a Training-only scope contract:
- H1 extends a visible role's resource header with its start-up window;
- H2: the existing served `0x17e5bc0` requests the window's other members for the pending bank;
- H3 rewrites the creation at the `0x678174` observer;
- H4 binds the bank in `prepare_models`;
- `scene_changed` keeps a window the new scene published.

10 recorded manifest edits of `rows_a1.rs`, `swap_model/scope.rs`, `character_lifecycle.rs` and `native_body_bank.rs`;
the values are a new clean-room model, `src/model/swap/entry_window.rs`. `carry_pkg6.py verify` re-derives all 250
files. Tests: `entry_window` (6), `tests/startup_window_wiring.rs` (3). Eden proof: `rc6-combined/rows-hot-slots/`
(grouped session).

### p23/rc6-hot-slots: the start-up window's owner generation; hats through planning and creation (2026-11-08)

The first Eden boot of P-HOT (`rc6-hot-xps1`, XPS, diag `e5120f8a`) found two defects in carried code:
- **The start-up window's owner generation.** The first group exchange from a start-up window fell back with "group
  owner receipt changed". H1 stamps the bank with the owner generation before the scene's own `scene_changed`, whose
  Mii adapter step then opens a new one. `native_body_bank::adopt_scene_generation` (called by
  `character_roster::scene_changed` after `mii_saved_alpha::scene_changed`) moves a window kept for the new scene to
  that generation (`entry_window::scene_generation`).
- **Hats cut to the fighter remainder** (hs06: no hat in 40 s at Ready Fighters 10).
  - `character_sources::preload_limit` cut the already budgeted plan to `helper_budget` again, which was 0 with
    windows and groups holding the fighters.
  - `creation_intent` capped the created entries by the same remainder and waited for entries 2 and 3 to hold
    targets their purpose views exclude.
  - `purpose_view` counted created entries that did not exist yet as busy for the entry-3 borrow.
  - Now: `preload_limit` without `helper_budget`; the created entries by purpose (`hat_slots::created_allowance`,
    `holds_back_creation`); the borrow waits for the allowed created entries (`hat_slots::created_busy`).

6 recorded manifest edits of `character_roster.rs`, `native_body_bank.rs` and `character_sources.rs`; the values are
in the clean-room models `entry_window.rs` and `hat_slots.rs`. `carry_pkg6.py verify` re-derives all 250 files.
Tests: `a_kept_start_up_window_joins_its_scenes_owner_generation`,
`a_cpu_mii_group_exchange_out_of_a_kept_window_after_the_scene_change` (the cov-rest CPU Mii cycle, primary 12:40Z),
`a_hats_only_plan_creates_four_entries_before_the_borrow`, `tests/startup_window_wiring.rs` (+1),
`tests/hat_slots_wiring.rs` (+1).

### p23/rc6-hot-slots: back to the original after a start-up window is native's single form (2026-11-08)

The re-prove boot `rc6-hot-xps2` (XPS, diag `e025ad2c`) crashed a native thread on P1 lap switch 07, back to the
original Mario from the group [Ness, Mario]. Both Mii adapters capture their origin from the live entry once the
scene is up, which since P-HOT B is the start-up window H3 wrote. "Back to the original" staged that window,
`(3, [0, 1, 2])`, as the request:
- the bank planner refused it as a multi-member request, so the original was never served warm;
- native reloaded three ordinary bodies whose resources only the start-up bank requests. A construction job read an
  unloaded resource (`main+0x35c2958`, null `+0x20`/`+0x40`, then a call through address 0); the reload never
  settled, every later swap was refused, and Quit Training hung.

Now the original's request is native's single form H3 started from (`entry_window::original_request`: count 1,
`[original, −1, −1]`, one actor, active 0, every other byte as captured). The adapters also recognise that form as
the original: `mii_ordinary_player::at_origin` and `player_context`, the CPU's Fighter staging, and the CPU commit's
`descriptor_equals_origin`. Native's own words (one body, PT, the IC pair, Aegis, a Mii) stay as they are.

6 recorded manifest edits of `mii_ordinary_player.rs`, `mii_ordinary_cpu.rs` and `mii_ordinary_cpu_native.rs`;
`carry_pkg6.py verify` re-derives all 250 files. Tests: `the_original_request_is_natives_single_form_of_a_start_up_window`,
`tests/startup_window_wiring.rs` `back_to_the_original_stages_natives_single_form`.

### p23/rc6-hot-slots: warm exit from a Mii group (2026-11-08)

Operator decision 2026-11-08 (relayed in `artifacts/phase23/INBOX-opus-rc6-hotslots.md`, 15:58Z), the first of the three
group-speed items.

On the console (release `1e6c6a0e`, scene 1), the CPU went Fox → Mii Gunner (a Mii group exchange) → Pichu. That took a
5.9 s Reload, `#106 reuse declined … descriptor bytes differ at [46, 48, 4a, 4e]`, although Pichu's body was built: the
Mii exchange had detached the window `[Fox, Pichu, Diddy]` as a Built group. The Reload also tore that window and the
Mii group down, so the next lap rebuilt both.
- The bank's exchange could already put the displaced window back (`R` words, entry bytes and the D mirror it saved).
- But `plan()` compared an ordinary request byte-exactly with the live Mii bytes, and both adapters refused to settle a
  bank switch out of a live Mii. Only a Reload undid the Mii publication.

Now:
- **Planner** (`native_body_bank`): `plan` is unchanged (`plan_in(.., false)`). `plan_mii_exit` serves an ordinary
  target in the Built window the live Mii group displaced (`mii_exit_saved`). The rule is
  `body_bank::mii_exit_bytes_match`: the live entry with that window's saved bytes applied; all else exact, except
  the group words and `+0x43`.
- **Adapters** (`mii_ordinary_player`, `mii_ordinary_cpu`):
  - `bank_settle` keeps an adopted Mii group switch's publication (`MiiExit`, in `State`) while the owner's receipt is
    the one it adopted.
  - `begin` asks for `plan_mii_exit` only then; the `BankTrack` carries `exit`.
- **Settle** (`mii_ordinary_native::exit_settled`, `mii_ordinary_cpu_native::exit_settled`):
  1. the bank's exchange receipt proves the descriptor (`group_rebaseline`);
  2. `bank_restore` puts the publication back and reads it back against the receipt from before the Mii;
  3. the owner adopts the window (the Player with no active configuration).

Tests (failed first, 16:04Z): `an_ordinary_request_leaves_a_mii_window_for_the_window_it_displaced`,
`tests/mii_exit_wiring.rs` (3). Recorded manifest edits of the six carried files; `carry_pkg6.py verify` re-derives all
250 files.

Eden `rc6-hot-xps5` (XPS, diag `dde68833`):
- The Player's laps through a Mii group passed. The exit to Link took 85 and 54 ms; the second Mii visit was a warm
  exchange; all 8 presses were `Applied`.
- The CPU's exit settled `NativeSettledUnowned`, "CPU current index mirrors differ". Until the restore, the CPU's
  `E+0x1a8` names the Mii's destination record while FE and D already name the window's index, so `exit_settled`'s
  owner walk before `bank_restore` refused.
- Fixed: the CPU's `exit_settled` reads the descriptor directly under G, as `bank_restore` reads its fields. Test
  (failed first, 16:35Z): `tests/mii_exit_wiring.rs` `the_cpu_exit_reads_no_owner_walk_before_the_restore`.

### p23/rc6-hot-slots: Mii groups build asynchronously (2026-11-08)

The second of the three group-speed items (operator decision 2026-11-08, `docs/DECISIONS.md` "Group speed in rc.6").
It lifts the 2026-11-02 restriction for Mii groups only. On the console, the CPU's first switch was a Mii group's
synchronous build: `#95 mode=sync reason=mii group`, `#96 built us=106311`, max tick gap 141.6 ms. On Eden (covrest-06,
diag `d20ead7b`), every Mii build took 111–140 ms on the game thread after the process's first.

`body_bank::async_build_eligible` admits a Mii group. Nothing else changes, because the queued path already serves a
Mii build:
- its identity is written into `R`, the entry and the D mirror before either mode starts, and stays while the group is
  the window;
- the Phase B Mii setup (`prepare_mii_group`) runs on the game thread before the construction is queued, as state 6 runs
  before native's own queued states 8–0xb for a canonical Mii;
- no prefetch can start on `R` meanwhile, because it needs INSTALL, which the switch holds.

Native and pair groups keep the synchronous build. Comments updated: the `native_body_bank.rs` header and the Phase B′
note in `perform_swap`; `build_split::Step::MiiSetup`.

Test (failed first, 16:30Z): `async_build_admits_ordinary_and_mii_groups_and_names_each_exclusion` (was
`async_build_admits_ordinary_groups_and_names_each_exclusion`). Recorded manifest edits of `body_bank.rs` and
`native_body_bank.rs`; `carry_pkg6.py verify` re-derives all 250 files.

### p23/rc6-cov-rest: F9 fixed seats reload; a Mii Save that reuses a saved Mii says so (2026-11-08)

**F9 (#58).** Eden covrest-04/05/06 (Ally; hot-slots diag `e025ad2c`, then `d20ead7b`): after a raise from one CPU to
three on one close, CPUs 2 and 3 stayed copies of CPU 1 (Fox, Meta Knight, Byleth) for 30 s, instead of the CPU
rotation's next fighter and the one after (Mii Gunner and Donkey Kong on the operator card). At one CPU entries 2 and 3
were never installed (`f9_body_of` read count 0 every time) and their loader byte held residue (6, then 88).
`classify_parked_body` read any non-zero loader byte on an unpublished cell as "in flight", so `reload_if_needed`
armed no seat Reload. Now an unpublished cell is in flight only while this plugin's own install on that slot is (the
source book's Loading phase); otherwise it is empty and both seats reload after the walk. `parked_entries` changes:
`classify_parked_body` (its third argument is now that question, asked only for an unpublished cell) and
`unpublished_body`; `on_count` and `reload_if_needed` are unchanged.

**#52 (operator decision C).** A Save that matches a saved Mii keeps the reuse rule (docs/MII.md rows 4 and 12) and
posts "Same as ‹saved name›, nothing new saved" with the matched saved Mii's own name (`model::notify::save_notice`,
a fixed body; `features::notices::post_body`, the mailbox, after the session lock).

2 recorded manifest edits: `character_sources.rs` (`f9_body_of`) and `mii_editor_bridge/mod.rs` (`edit_decide`);
`carry_pkg6.py verify` re-derives all 250 files. Tests (failed first): `parked_entries`
`an_unpublished_entry_with_a_residue_loader_byte_is_empty` (in its loader-byte form it returned `Loading`),
`tests/mii_same_notice.rs` (did not compile: no `save_notice`).

### p23/rc6-cov-rest: an F9 Mii seat loads with its own Mii configuration (2026-11-08)

**F9 (#58).** Eden covrest-08 (Ally, diag `888badce` of `70617ba`): after a raise, a drop to one CPU and a raise, the
seats parked and were promoted (Mii Gunner as CPU 2, Donkey Kong as CPU 3), but entry 2's Mii Gunner had been
installed as a plain fighter (`install slot=2 kind=0x4a`, no Mii helper). From the moment No. of CPUs reached 2 the
game read null every frame (`Unmapped Read64 @ 0x0`, `main+0x357bf34`; 315852 by the end of the boot). F9 kept a seat
member as a kind only (`cycle_preload.rs` `set_fixed_members`, `f9_gate`), and `purpose_view`'s seat branch made a
`Want` with no configuration when the plan did not list that kind. The members now keep the CPU rotation target whole
(kind and exact-Mii configuration, F9 report §4.2); `f9_gate` arms a Reload with that configuration and arms no Mii
seat without one (`parked_entries::seat_load`); the seat branch loads what was armed, so a Mii seat goes through the
Mii helper as the CPU cycle's own Mii loads on entry 2 do.

2 recorded manifest edits: `character_sources.rs` (`purpose_view`'s seat branch) and `cycle_preload.rs`
(`set_fixed_members`); `carry_pkg6.py verify` re-derives all 250 files. Test (failed first: no `seat_load`):
`parked_entries` `a_mii_seat_loads_with_its_member_configuration_or_not_at_all`.

### p23/rc6-cov-rest: the first F9 raise shows the seats (2026-11-08)

**F9 (#58), reading A (primary 16:04Z).** Eden covrest-07/08/09: the first raise after a scene start showed copies of
CPU 1 on CPUs 2 and 3; the seats appeared only from the second raise. At one CPU `purpose_view`'s seat branch loaded a
member on entry 2 or 3 only when the merged plan listed it with no configuration, and the body bank covered Donkey
Kong. A parked, unborrowed seat now holds its member (kind and exact-Mii configuration, `parked_entries::parked_seat`)
whether or not the plan lists it. The load is the same speculative helper load, so process-memory admission still
holds it (`[ProcessMemory] refused … stays planned`, retried) when headroom refuses. Entry 3 while it holds the
borrowed fifth hat is not a seat (unchanged); entry 2 no longer waits for that borrow.

1 recorded manifest edit: `character_sources.rs` (`purpose_view`'s seat branch); `carry_pkg6.py verify` re-derives all
250 files. Test (failed first: the seat branch did not call `parked_seat`):
`parked_entries` `the_seats_park_at_one_cpu_whatever_the_plan_lists`.

### p23/rc6-cov-rest: diag note of a refused helper proof (2026-11-08)

Eden covrest-10: after an F9 raise and drop, both Mii owners refused their snapshot ("helper row kind/provenance
changed"), which leaves the Mii classes out of both cycles. Diag builds now log the recorded helper proof beside the
row read now (`[MiiHelperProof] refused id=… why=…`, first 12 per boot) in `mii_ordinary_native.rs` `helper_kind`; the
acceptance itself is unchanged. 1 recorded manifest edit; `carry_pkg6.py verify` re-derives all 250 files.

### p23/rc6-cov-rest: a promoted F9 seat's Mii proof is taken again once parked (2026-11-08)

**F9 × Mii (reading B, primary 16:44Z).** Eden covrest-10/11: after a raise promoted the Mii Gunner seat on entry 2
and a drop parked it again, both Mii owners refused every snapshot ("helper row kind/provenance changed") and the Mii
classes left both cycles. The diag note (`[MiiHelperProof] refused id=2 why=mii record`) showed the helper `Proof`
unchanged and the Mii record changed: the activation's state bytes, not the seat's identity. F9 now marks a published
seat (`f9_publish` → `f9_gate::mark_promoted`); once the seat is parked again and parked ready, `f9_reproof` asks
`mii_managed_helper::reproof_parked` to take the Mii proof (record and descriptor) again from the row, only when the
identity is unchanged (the same ready source and configuration, binding, owner objects and scalars, the charinfo and
a resource pair; `parked_entries::reproof`). Any identity change keeps the refusal.

2 recorded manifest edits: `mii_managed_helper.rs` (`reproof_parked`, new) and `character_sources.rs` (`f9_publish`,
`f9_player_frame`, `f9_reproof` new); `carry_pkg6.py verify` re-derives all 250 files. Test (failed first: no
`reproof`): `parked_entries` `a_promoted_seat_is_re_proved_once_parked_only_with_its_identity`.

### p23/rc6-cov-rest: the portrait race, a press waits for the render and the F9 seat load holds back (2026-11-08)

**Mii portrait × F9 (primary 17:28Z, readings A and B).** Eden covrest-12: after the CPU loaded the card's Mii
Gunner, its HUD portrait leased the F9 seat on entry 2 (the same exact Mii), but the render waited ~6 s behind seat 3's
INSTALL and was issued in the frame of the CPU's next press. The press's snapshot refused the row mid-render, the
portrait's after-render proof quarantined helper 2 for the scene, and presses 1-5 loaded nothing.
- A (the race): a Mii owner's begin waits while that role's portrait job holds a helper lease or has its render
  issued: `Blocker::Portrait` ("portrait render on a helper") in the CPU's `begin_blocker`, the same check in the
  Player's `permitted`, through `mii_hud_portrait::render_holds_helper(logical)`. The press runs after the render; it
  is never dropped.
- B (ordering): F9's seat branch of `purpose_view` starts no seat load while a portrait job wants a helper or holds one
  (`portrait_waits`: a portrait channel's request or a Portrait lease; `parked_entries::seat_load_waits`).

5 recorded manifest edits: `source_load.rs` (`Blocker`, `reason`, the test), `mii_hud_portrait.rs`
(`render_holds_helper`), `mii_ordinary_cpu.rs` (`begin_blocker`, `portrait_render_held`), `mii_ordinary_player.rs`
(`permitted`, `portrait_render_held`), `character_sources.rs` (`purpose_view`'s F9 section, `portrait_waits`);
`carry_pkg6.py verify` re-derives all 250 files. Tests (failed first: no `Blocker::Portrait`, no `seat_load_waits`):
`source_load` `a_press_during_a_portrait_render_waits_and_then_applies`, `parked_entries`
`a_seat_load_holds_back_while_a_portrait_render_waits`.

### p23/rc6-cov-rest: a CPU Mii install writes a whole CharInfo (#54, #68; 2026-11-08)

**#54 / #68.** Eden covrest-yoga-03 (Yoga, probe `06f6dc3` with cov-rec's handoff `e276d0f`): Switch on a CPU macro
recorded on an exact Mii (cov-rec's `cov-named-mii`, fixture `rc4-gunner-mii`) installed Mii Gunner with
`resource_before` zero and stayed zero: `terminal ready comparison refusal ... reason=native resource pair absent`,
INSTALL retained, 17 `Unmapped` console lines (cov-rec 16:08Z: `main+0x357a914`, reads at 0x0 and 0x288). The macro's
Mii came from a replay: its CharInfo has no create ID and no name (bytes `0x00..0x26` zero; the replay window's capture
is the same). The Reload's own Mii setup (`0x67e7d0`, `mii::mii_setup`) checks the record's CharInfo
(`0x39c2930/0x39c2940`) before it binds the resource pair, so it bound none; the same canonical Mii from the catalog
(covrest-14's native Gunner, with its create ID and name) bound its pair. The exact-Mii target of `begin` now installs
`engine::mii_install::installable`: the saved native Mii with the same look (MII.md row 12), else the Mii with a
create ID and the "New Mii" name filled in; the look, tokens and canonical identity are the Mii's.

1 recorded manifest edit: `mii_ordinary_cpu.rs` (`begin`'s `mii_target`); `src/model/mii_install.rs` is new (not
carried); `carry_pkg6.py verify` re-derives all 250 files. Tests (failed first against a stub that returned the
configuration unchanged): `mii_install` `a_replay_mii_installs_the_saved_mii_with_the_same_look`,
`a_replay_mii_with_no_saved_twin_gets_a_create_id_and_the_new_mii_name`,
`a_mii_with_its_own_create_id_and_name_is_installed_as_it_is`, `the_cpu_install_writes_the_installable_configuration`.

### p23/rc6-cov-rest: the clip editor's question owns Player 1's presses (#68; 2026-11-08)

`66a9a53` added one guard to the package-4 file `replay_capture.rs` (`on_input` returns while the clip editor's
Fighter Mismatch question is up; Eden covrest-yoga-01: Down and A went to the editor instead of the question). The
edit is recorded in `docs/provenance/pkg4-carry-manifest.json` (edit 59); `carry_pkg4.py carry`, `ledger`, `verify`
(40 files against stage m5q, and dest-only) pass.
