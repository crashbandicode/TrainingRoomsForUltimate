# Carry-over ledger (package 1)

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
| `src/model/store.rs` (shared: package 4's named macros reuse it, do not carry `library.rs` again) | `training_mod_macros/src/library.rs` (ours-new) | `f023bf6ccc5908417e2b10014cf26e2311b54260822a2472950e546b468c0275` | whole file except the macro-specific parts | (1) the fork's `MacroError` became `StoreError` (only the variants the store itself raises: version, size, id, not found, no free id, parse, I/O); (2) `Library`, `MacroEntry` and `Record for MacroRecording` removed (package 4 adds its own `Record` impl); (3) `Listing<S>` lost its `MacroEntry` default; (4) the `Header` serde derive in `decode` is a hand read of `schema_version` (this crate has no serde derive: resident size); (5) the id slug fallback is `entry`, not `macro`; (6) the fork's `library/tests.rs` is not carried (macro recordings); new host tests with a toy record cover ids, atomic overwrite, version/size/corruption listing, orphaned backup, sort order. |
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
### pkg6 C6a: the pilot core's pure module (ngine::p1, 2026-11-02)
Carried from src/cpu_pilot/ (stage m5o, ours-new), unedited: pilot.rs (as src/model/p1/mod.rs), config.rs, move_observer.rs + move_observer_tests.rs, 	ests.rs and the three xamples/*.json its tests include. This is the fork's self-contained host-testable pilot_core (explicit JSON config, trigger/command model, move observer); 18 host tests pass. Test-only until the adapter cpu_pilot.rs (hooks, game natives; waits for the interfaces and the ssbu-cpu-pilot-core dependency) is carried. Full clippy for this commit is deferred until the Surface is free (lead's rule while the M5q Eden worker holds it).

### pkg6 C2c: preload_metrics and the logging shim (2026-11-02)
Carried src/preload_metrics.rs (stage m5o, ours-new, path rewrite only; svcGetInfo on aarch64, a host path elsewhere; 7 carried tests). New model::compat::logging (written here): diagnostic forwards to 
untime::diag_log::diagnostic on the Switch (
untime::diag_log became pub(crate), one word in src/runtime.rs) and records into a bounded capture buffer on the host (	ake() for tests); heap_profile is a no-op (the fork's Eden-only plugin-heap-meter allocator is not carried). **Not yet compiled for the Switch** (the Surface is held by the M5q Eden worker: no NRO builds); the first Switch build after its RELEASED line verifies compat::logging and the pub(crate).
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
| `src/model/capture_settings.rs` | `training_mod_macros/src/capture_settings.rs` (ours-new) | `5357f35edcdee093075cd7e27e6c206c8a408b8a05ad46b583493213b6dc6af3` | whole file | none to the carried text except the test `TempDir` (the fork's `test_util::TempDir`, reduced, defined in the test module). Added at the end: `FILE_NAME`, `MAX_BYTES`, `to_ini_text` (the menu's Replay capture toggle, Q7: replaces the recognised line in place), `from_bytes` (fail-closed read) and their tests. |

Not carried yet (S4-4, waiting on the lead's sequencing: pkg6's `training_mod_mii` first): `training_mod_macros/src/{recording,library,window,replay,live_frame,move_start,live_capture,capture_archive,capture_toggle,shared_capture,ui_anchor}.rs`,
`src/training/named_macros.rs`, `src/replay_*.rs`, `src/live_replay.rs`, `src/capture_plugin.rs`, `src/training/ui/clip_editor.rs`,
`training_mod_tui/src/{clip_*,macro_panel*,replay_panel,resume}.rs`. Dependencies they need (lead rulings 2026-11-02): `training_mod_mii`
(pkg6), `smash-match-data` (vendored under `crates/`), `deluxe_plugin_sdk` (feature `deluxe-sdk`), `serde` with derive.
`library.rs`'s generic `Store` is pkg3's `model::store`; the macro `Record` impl follows `recording.rs`.

New code in this stream with no fork origin: everything else under `src/model/{recording,chords,ini_files,notify,damage_display,input_display,sites_pkg4}.rs`
and `src/runtime/features/{recorder,chords,notices,input_display}.rs`, written from SPEC-input-01/02/04/05 and the owned 13.0.5 ELF.

### pkg6 C1b: seam_install (2026-11-02)
Carried `src/swap_native/seam_install.rs` as `src/runtime/seams/seam_install.rs` and `seam_drill.rs` (diag only), Switch-only, with three recorded edits: `use crate::model` becomes `model::version as model`; the fork's second install line (`report_install`, `[TrainingModpack] result=installed ...`) is removed; `InstallFields` (new, in the same file) is the seam outcome as fields of package 1's one install line (`seams=<n> seams_stock=<n> seams_other=[..] seams_skipped=[..] seams_shadow_diff=[..] seams_missing=[..] seams_external=[..] seams_unchecked=<n> seams_authoritative=.. seams_images=.. seams_authoritative_images=..`, lead ruling 2026-11-02); rows checked or changed after the line keep the tagged late line (`[TrainingSeams] late ...`, tag already shipped). `model::install_line::InstallLine` gained `seams: Option<Extra>` (boot passes `InstallFields`; today `seams_unchecked` is the whole table because no consumer calls `gate` yet). `engine::seams::native_seams` (the table) is unconditional; `native_fields` and the unlink guards stay test-only until their consumers land. Resident (Yoga, `cargo skyline build --release`): 765,952 B before (`a8a1dfb9`, the merged tree) -> 901,120 B after (`120f2342`): **+135,168 B** for the 620-row table and the installer (budget estimate for the seam rows was 180 KB). Diag build compiles (`20f054dc`). Dead-code warnings (unused `gate`, `site`, `check_image`, ...) on the Switch build until the cycle files call them.

### pkg6 D3 update: one skyline, pilot core added, cpu_pilot.rs staged (2026-11-02)
- **Unified `skyline`** (lead ruling): `skyline_smash` names the skyline-rs and nnsdk-rs git sources; two `[patch."<git url>"]` entries (`skyline = { version = "=0.6.0" }`, `nnsdk = { version = "0.6.2" }`) point them at the registry crates, so `Cargo.lock` holds exactly one `skyline` 0.6.0, one `skyline_macro` 0.2.1 and one `nnsdk` 0.6.2, all from crates.io (`tests/pkg6_deps.rs` fails on a second copy or a git source). `cargo tree -d --target all` now lists only `libc-nnsdk` 0.4.0 (`skyline_smash` asks `^0.4`) and 0.5.0 (`nnsdk`): the two are plain FFI declaration crates with no runtime state, and no patch can join incompatible semver versions; listed here as a known residual. (`syn` 1 / 3 are host proc-macro build dependencies.) The Yoga Switch release builds with the patch: resident 901,120 B, unchanged (the new crates are not linked yet).
- **`ssbu-cpu-pilot-core` added** (git, rev `8b03e3bffea8a156628f76cd9ca31c346f52ac70`, feature `native`, Switch target): MIT (`core/Cargo.toml`, repository LICENSE); size 0 B until `cpu_pilot` links it.
- **Staged, not yet declared (no compile)**: `src/runtime/p1/cpu_pilot.rs` (stage m5o `src/cpu_pilot.rs`) with three recorded edits (the `#[path]` modules become `engine::p1`, `engine::p1_menu_session`, `engine::p1_stage_session`; the approved `allow(dead_code)` on the first is dropped), and the two pure sub-files `menu_session.rs` / `stage_session.rs` (test-only modules, their tests pass). The tool learned the `training_mod_sync` head (to `compat::sync`). The names `cpu_pilot.rs` takes from outside are rewritten to `crate::runtime::p1::compat::{common::{FIGHTER_MANAGER_ADDR, FighterId, game_mode::current, input::MappedInputs, is_fighter, is_grounded, is_native_training_mode, menu::captures_input, try_get_module_accessor}, training::{recording_or_playback, ui::menu::VANILLA_MENU_ACTIVE}, replay_preview::occupied, character_roster::busy, sync::read}`: that module is the thin adapter written against the wire stream's APIs once their commit is named.
- **Guarded-call cross-check** `tests/pkg6_smash_calls.rs`: scans every source under `src/runtime` that uses `smash::` for calls on `*Module`, `FighterManager`, `FighterEntry`, `utility`, `sv_battle_object`, requires each in the `COVERED` call-to-row table and each row to exist in a site table. A re-sync that adds a call fails until the call is guarded.- Correction to the staging note above: `menu_session.rs` / `stage_session.rs` are not declared in `engine` yet (clippy `-D warnings` flags the never-constructed `stage_session::Invalid` without its consumer); their 7 tests run by path from `tests/p1_sessions.rs` (the playbook's by-path include with `allow(dead_code)`, the pattern the Mii tests already use). They are declared as `engine::p1_menu_session` / `engine::p1_stage_session` together with `cpu_pilot.rs`.