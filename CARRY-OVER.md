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
## Package 2 (CPU behaviour), 2026-11-02

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
| byteflags | 0.1.0, git `https://github.com/asimon-1/byteflags.git` branch `rand-0.7.4` rev `e6ac815a9d0dc4665b39fe7849e2baec8bf443cb` | **no `license` field** in its `Cargo.toml`; the repository `LICENSE` file (sha256 `ec79c3434bb77c3a5671a9ef22b29488c94f7655c6984c3287ae32344b88bc82`) is the MIT text, `Copyright (c) 2023 asimon-1`. **Accepted by the lead 2026-11-02** (MIT by its LICENSE file) | measured when the runtime uses it | to be added with `cycle_preload_config.rs` (4 `byteflags!` groups), which stays byte-identical |
| rand (git, dependency of byteflags) + rand_core + rand_chacha | 0.7.4 / 0.5.1 / 0.2.3, git `https://github.com/skyline-rs/rand` rev `8e87ec2a30ef2283888b2456b8ea65ea5110f2bd` | `MIT OR Apache-2.0` (all three manifests); `LICENSE-MIT` sha256 `209fbbe0ad52d9235e37badf9cadfe4dbdc87203179c0899e738b39ade42177b`, `LICENSE-APACHE` sha256 `aaff376532ea30a0cd5330b9502ad4a4c8bf769c539c87ffe78819d188a18ebf` | with byteflags | with byteflags |
| getrandom (git, dependency of rand) | 0.2.99, git `https://github.com/Raytwo/getrandom` rev `23a1419108a1f7539839dc201e5b73fc3515f71f` | `MIT OR Apache-2.0` | with byteflags | with byteflags |
| skyline_smash (lib name `smash`) | 0.1.0, git `https://github.com/ultimate-research/skyline-smash.git` rev `01bf583…` | `MIT` | not yet | not added yet (needed by C7) |
| ssbu-cpu-pilot-core | 0.1.1, git `git.syntaxbyexample.com/intpa/ssbu-cpu-pilot` tag `v0.2.1` rev `8b03e3b…` | `MIT` (the operator's crate) | not yet | not added yet (C6) |
| deluxe-plugin-sdk | 2.0.0, no source line in the lock (path dependency); not located yet | – | – | to check at C6 |
### pkg6 carried files, tranche 2 (C3a: Mii crate, body bank, swap model, preload and kirby policy)
Generated ledger: `docs/provenance/pkg6-carry-ledger.md` (101 files after this tranche). New in the tool: item-level and longest-prefix module mapping, `include_str!` rewriting through the manifest, `prepend` (by-path test glue) and `delete_from`/`delete_to` edits. Edits to carried files in this tranche (each offered back through the fix ledger): `trio.rs` and `ordered_roster.rs` lose the fork's menu structure (`to_submenu`, the picker/full-menu tests: replaced by package 1's editor rows and package 5's renderer); `draft_store.rs` (re-run-by-name test filter), `practice_policy.rs` (`#[derive(Default)]` for clippy). The fork's menu names the carried code expects (`OnOff`, `TrainingModpackMenu`, `BASE_MENU`) are provided by `engine::compat::menu` (ours, not carried). Carried tests: the 12 `training_mod_mii/tests` integration files that need only the Mii crate (7 carried; `quickplay_backend`, `ldn_backend`, `source_provenance`, `heap_metrics`, `ui_archive_name` and `replay185_headgear` include other root files by path and are carried with those files).

### Disclosure: an `rg` over two mixed fork files (2026-11-02, while carrying the stage menu)

To find how `PAGE_COLUMNS` (carried `training_mod_tui/src/stage_menu.rs`, ours-new) gets its value I ran one `rg 'NX_SUBMENU_COLUMNS|NX_SUBMENU_ROWS'` that named the two MIXED files `training_mod_tui/src/lib.rs` and `training_mod_tui/src/containers/submenu.rs` of stage m5o (my mistake: I should have searched only ours-new paths). It printed three lines of `lib.rs`: the definitions `pub const NX_SUBMENU_ROWS: usize = 8;` (line 14) and `pub const NX_SUBMENU_COLUMNS: usize = 4;` (line 15), and one use at line 64 (`toggles: StatefulTable::with_items(NX_SUBMENU_ROWS, NX_SUBMENU_COLUMNS, toggles),`); it printed nothing for `submenu.rs`. Nothing was copied. The only value used is the stage grid's page width of 4 columns, which is independent of that line: `docs/DECISIONS.md` Q14 and `MENU-DRAFT.md` state the fork's stage paging as 4 x 4 (eight pages), and the carried `PAGE_ROWS = 4` with `PAGE_SIZE = 16` agrees. The compat constant is written as 4 from those documents.
