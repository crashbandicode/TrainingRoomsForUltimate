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

- SPEC-core-08 (as it stood in the worktree before 5e34c82) named `training_modpack.json` as the fork's
  settings file. In the fork that file is the updater record (`{"update":{"last_update_version":…}}`, see the
  carried fixtures `…-training_modpack.json`); the settings file is `training_modpack_menu.conf`.
  `storage::file::LEGACY_SETTINGS` uses the real name.
