# rc.9 test cut ledger

2026-11-11. Operator order (`artifacts/phase23/TASK-rc9-test-cut.md`): the host tests go to at most 1,000, a hard limit, keeping what
guards real behaviour and the ship gates. This file lists every cut and why, and what is kept. Base: `28082c29` (rc.8 notes on `main`).

**2026-11-12:** most of the cut tests returned as table-driven tests (one `#[test]` per module) for the coverage order:
`docs/COVERAGE-rc9.md`. Of those, the cases that add no line coverage were removed again (its "Option (b)" section): 1,205 cases,
nearly all covering only lines a kept case covers. The count is 821 test functions and 1,689 cases. This file describes the
cut as it was made at `7a4cba64`.

## Before and after

| | Test binaries | Tests (`cargo test -- --list`) | Distinct test names |
| --- | ---: | ---: | ---: |
| Before (`28082c29`) | 148 | 41,918 | 3,292 |
| After | 68 | 942 | 939 |

- With the two vendored crates' own gates (`cargo test --manifest-path crates/hos-lock/Cargo.toml`: 28; `crates/smash-match-data`: 24;
  unchanged), the three host gates run 994 tests.
- Where 41,918 came from: the crate is a `cdylib`, so 69 test crates included `src/model.rs` by path and each re-ran its ~509 unit
  tests (about 35,000 runs of the same functions); 11 more included `src/model/mii/mod.rs` (~166 each); one included the engine
  (1,828). The lib's own binary ran 1,913, about 1,300 of them the carried fork engines' tests.
- Now each test function runs once. The unit tests of `src/` files are wrapped in `lib_unit_tests! { ... }`: `src/lib.rs` passes the
  body through, and a test crate that includes a `src/` file by path defines the macro empty. The call-graph scanner's own three
  tests run in `tests/hook_locks.rs` only (`callgraph_self_tests!`).
- No test was merged into a table to lower the count: every remaining `#[test]` is the function it was.
- To bring a cut test back: `git show 28082c29:<path>` (the paths are below).

## What is kept, and what each protects

| Kept | Tests | Protects |
| --- | ---: | --- |
| `tests/game_thread_handoffs.rs` | 3 | no std/crossbeam channel in non-test code: a game thread never spins on a peer (the rc.8 "Add New…" hang) |
| `model::nowait_channel` and the lock-free hand-offs (`macro_handoff`, `box_handoff`, `close_handoff`, `catalog_mailbox`, `catalog_reply`, `pooled_reply`, `owned_ring`, `drop_ring`, `frame_queue`, `seq_cell`, `reusable_snapshot`, `replay_jobs`, `stall_watch`), `tests/catalog_reply.rs`, `tests/menu_writeback.rs` | 10 | the wait-free primitives every game-thread hand-off uses return "not ready" instead of waiting |
| `tests/hook_locks.rs`, `tests/lock_order.rs`, `tests/lockfree_wiring.rs`, `tests/no_spin_locks.rs` | 48 | no hook blocks on a lock a plugin thread can hold; lock order; the lock-free inventory only shrinks; no spinning lock crate |
| `tests/svc_clobber_abi.rs` | 2 | every `svc` asm declares `clobber_abi("C")` (with `tools/svc_clobber_check.py` on the ELF) |
| `tests/thread_stacks.rs`, `tests/stack_marks.rs`, `tests/native_service_wiring.rs` | 21 | the stack table covers every thread's measured need; a measured high-water past its row fails; the native service's game-stack bound |
| `tests/build_tag.rs` | 1 | the menu footer's build name and commit |
| `tests/size_budget.rs`, `tests/carry_pkg4.rs`, `tests/carry_pkg6.rs`, `tests/pkg6_deps.rs`, `tests/pkg6_features.rs`, `tests/lint_allows.rs`, `tests/pane_recipe_guard.rs` | 23 | the size gate; carried files match their manifests; one Skyline runtime in the lock; the shipped feature closure has no diag feature; no unlisted lint `allow`; the layout recipes match the pane table |
| Seam guards: `engine::seams::*`, `model::sites`, `model::sites_ui`, `model::own_patches`, `tests/carried_guard.rs`, `tests/carried_model_facade.rs`, `tests/core01.rs`, `tests/capabilities.rs`, the site tables against the owned 13.0.5 ELF (`cpu_sites`, `cpu_common`, `input_sites`, `state_sites`, `u4_sites`, `u4b_sites`, `pkg6_sites`, `ui_sites`, `pause_helper_sites`, `input_mapping_callers`), `tests/pkg6_smash_calls.rs`, `tests/native_wrappers.rs` | 53 | the version guard and the 64-byte rule stop a bind when the game's bytes differ; every site row is the ELF's bytes; every called game function is guarded and wrapped once; a rejected capability disables every entry point |
| `model::chords`, `tests/ui_chord_capture.rs`, `tests/input04.rs` | 33 | the chord rule from physical input, chord capture, button mapping, the INI shortcuts file (fail-closed reads, transactional writes) |
| `model::game_speed`, `model::lobby_speed`, their `engine::*_rows_tests`, `engine::stages::lobby_stage_cycle` | lib | game-speed and lobby-speed state logic and their compared native rows; the waiting-room entry stage choice |
| Settings and files: `tests/core04_settings.rs`, `tests/core04_recovery.rs`, `tests/migration_run.rs`, `tests/findings_menu_close.rs`, `model::settings`, `model::store`, `model::storage`, `model::migration`, `model::capture_settings`, `model::sha256`, `model::archive_*`, `engine::preload::ini_transactions_tests` | 62 | `settings.json` shape, defaults, recovery of a partly bad file across two boots, atomic writes, the first-boot import on real directories, the menu-close commit rule, `cycle_preload.ini` load/store sequences across a reboot |
| Save states and setups: `tests/state01_json.rs`, `tests/state01_machine.rs`, `tests/state01_setup.rs`, `tests/state05_restore.rs`, `tests/state06_07.rs`, `model::state::setup_transaction`, `model::lra_reset` | 65 | `save_states.json` and the setup library's file format, the save/load machine, Pokémon Trainer and Kirby restore rules, L+R+A reset, the stage-hazard instruction scan |
| Recording files: `engine::macros::recording`, `engine::macros::library`, `engine::macros::capture_archive`, `engine::library`, `model::recording` | lib | the recording file format, the macro library's files, index and migration, the capture archive |
| Recorder and input: `tests/input01.rs`, `tests/input_delay.rs`, `tests/input_pause.rs`, `tests/input_pending.rs`, `model::input_delay`, `tests/ui_readiness.rs`, `tests/ui_owner.rs` | 59 | the recorder's state machine, Input Delay, input ownership (pause, menu, an unavailable menu never owns Player 1) |
| No heap in hooks: `tests/hook_alloc.rs`, `tests/hook_tls.rs`, `tests/playback_hook.rs`, `tests/p1_pilot_hooks.rs`, `model::hook_stats` | 21 | the counting allocator proves hooks, the playback machine and the CPU-as-P1 fallback allocate nothing |
| CPU options: `tests/cpu_core.rs`, `tests/cpu01.rs` … `tests/cpu07.rs` | 43 | the CPU training behaviours (mash, DI, ledge, tech, shield, aerials, frame advantage) by spec row |
| `tests/core06_hitbox.rs`, `tests/entry_cells.rs`, `tests/ui_view_indices.rs`, `model::install_line`, `install_capacity`, `model::scene`, `lobby_flag_tests`, `lib.rs::grid_matches_catalog` | 38 | the hitbox visualizer's rules, sites and layout facts; the F6 entry-cell reader; the pane-index panic (P0); the install line and its buffer; the scene predicate; the waiting-room online flag; grid sizes equal the catalog's |

The lib's binary (`src/lib.rs`) holds 331 of the 942; rows marked `lib`, and the `model::`/`engine::` names in the other
rows, are counted there. The integration rows' numbers are their own files' tests.

## Cut: integration test files (80)

### `*_wiring.rs` and other source pins (30 files, 115 tests of their own)

Why not necessary: Source-text pin of one past fix: it reads the Switch-only runtime's `.rs` text through the call-graph scanner and asserts that one named call path or line still exists. The rule itself is host-tested in the model or was accepted in Eden; the pin fails on a refactor that keeps behaviour and passes on a wrong value at the right call site. Not a ship gate.

| File | Own tests | Listed before | What it covered |
| --- | ---: | ---: | --- |
| `tests/background_build_wiring.rs` | 5 | 5 | rc.6 background builds: eligibility and notice mapping call sites |
| `tests/background_mii_wiring.rs` | 6 | 6 | rc.6 Mii groups built in the background: call sites |
| `tests/background_native_wiring.rs` | 8 | 8 | rc.6 Pokémon Trainer / Pyra-Mythra background groups: call sites |
| `tests/background_pair_wiring.rs` | 7 | 7 | rc.6 Ice Climbers pair group: call sites |
| `tests/body_window_wiring.rs` | 3 | 3 | rc.6 body window teardown reads only built members |
| `tests/combo_lobby_wiring.rs` | 8 | 520 | lobby-practice combos reach the model through the named paths |
| `tests/combo_share_wiring.rs` | 3 | 6 | shareable combo files: runtime paths to the host-tested loader |
| `tests/combo_switch_wiring.rs` | 5 | 8 | a combo's fighter question uses the replay macro's switch path |
| `tests/cpu_cycle_wiring.rs` | 5 | 8 | every CPU cycle walker uses the one member list |
| `tests/cycle_change_wiring.rs` | 9 | 9 | a mid-session cycle change re-plans and releases groups |
| `tests/give_item_wiring.rs` | 2 | 5 | the Give item request's hand-off path |
| `tests/group_hold_wiring.rs` | 2 | 2 | a refused group is held, not counted failed |
| `tests/hat_slots_wiring.rs` | 3 | 3 | helper entries are budgeted by purpose |
| `tests/lobby_persist_wiring.rs` | 4 | 513 | waiting-room partners and CPU action persist |
| `tests/mii_exit_wiring.rs` | 6 | 6 | warm exit from a Mii group |
| `tests/mii_switch_installed_wiring.rs` | 1 | 4 | comparisons use what the install wrote |
| `tests/momentum_wiring.rs` | 3 | 6 | Play applies recorded start speeds by the Momentum key |
| `tests/player_cycle_wiring.rs` | 3 | 6 | every Player cycle walker uses the one member list |
| `tests/preload_seat_wiring.rs` | 2 | 2 | parked F9 seats keep their own members |
| `tests/rc3_review_wiring.rs` | 5 | 8 | the rc.3 review's fixes stay wired |
| `tests/recording_switch_mii_wiring.rs` | 1 | 4 | Switch on an exact-Mii macro reaches the replay path |
| `tests/replay_window_wiring.rs` | 3 | 6 | Replay Window retry and exact-Mii named macros are wired |
| `tests/seat_notice_wiring.rs` | 2 | 5 | the readiness notice counts held seats |
| `tests/setup_restore_wiring.rs` | 2 | 5 | a setup restore leaves the bound save alone |
| `tests/startup_window_wiring.rs` | 5 | 5 | the window built into Training's own start |
| `tests/window_admission_wiring.rs` | 2 | 2 | window admission checks headroom before prepare |
| `tests/online_flag.rs` | 4 | 4 | the waiting-room clock's call sites (the decision itself: `lobby_flag_tests`, kept) |
| `tests/kirby_hat_cpu.rs` | 3 | 3 | the CPU Kirby hat request runs without Player 1 being Kirby |
| `tests/timer_recording.rs` | 2 | 5 | a record continues across a stretch with no live play (model test in `live_capture`, also cut) |
| `tests/stage_request_init_order.rs` | 1 | 1 | the practice observer checks its seam group before the lobby hook installs |

### UI wording, glyphs and palette (8 files, 26 tests of their own)

Why not necessary: On-screen wording, glyph, colour or layout geometry: incidental presentation, checked by eye in the Eden docs GIFs.

| File | Own tests | Listed before | What it covered |
| --- | ---: | ---: | --- |
| `tests/ui_glyphs.rs` | 2 | 2 | every character the plugin draws is in the game's font |
| `tests/recording_icons.rs` | 1 | 1 | the floppy and loop glyphs are in the committed layout archive |
| `tests/notice_clearance.rs` | 3 | 11 | the notice slot does not overlap the frame strip |
| `tests/notice_plates.rs` | 7 | 516 | dark plates behind notice text |
| `tests/ui_notice_palette.rs` | 1 | 510 | notice text contrast |
| `tests/core07_assets.rs` | 2 | 511 | SDI chip streak length; no baked fighter portraits |
| `tests/intro_once.rs` | 6 | 515 | the Training intro notices show once per install |
| `tests/mii_same_notice.rs` | 4 | 513 | a duplicate Mii save posts "Same as ..." |

### Mii by-path suites (13 files, 74 tests of their own)

Why not necessary: Mii policy probes that included `src/model/mii/mod.rs` by path (each binary re-ran its ~166 unit tests). The carried Mii engine is pinned by the carry manifest (`tests/carry_pkg6.rs`); its acceptance is the Eden Mii rows. Not a ship gate.

| File | Own tests | Listed before | What it covered |
| --- | ---: | ---: | --- |
| `tests/mii_bank_groups.rs` | 5 | 171 | bank group planning |
| `tests/mii_bank_publication.rs` | 3 | 169 | bank publication |
| `tests/mii_bust_context.rs` | 3 | 3 | bust context policy |
| `tests/mii_configuration.rs` | 9 | 178 | persisted identity across representations |
| `tests/mii_helper147_headgear.rs` | 3 | 169 | helper callback hat admission |
| `tests/mii_install_boundary.rs` | 5 | 5 | editor install dispatch (shim) |
| `tests/mii_ldn_handshake.rs` | 9 | 175 | Local Wireless round-handshake policy |
| `tests/mii_player146_headgear_ownership.rs` | 3 | 169 | fresh-O admission per Training row |
| `tests/mii_replay185_headgear.rs` | 11 | 177 | replay source candidates |
| `tests/mii_role_adapter.rs` | 8 | 174 | role adapter |
| `tests/mii_setup_preflight.rs` | 7 | 173 | setup preflight |
| `tests/inline_keyboard.rs` | 7 | 176 | inline keyboard callbacks |
| `tests/inline_keyboard_native.rs` | 1 | 167 | native keyboard adapter against an SDK double |

### Spec-row detail of pure models (15 files, 227 tests of their own)

Why not necessary: Per-row spec detail of a pure model (menu rows, per-fighter tables, notification queue). No thread, lock, seam or file is involved; a wrong row shows in the menu or in the Eden rows of that feature.

| File | Own tests | Listed before | What it covered |
| --- | ---: | ---: | --- |
| `tests/core02_menu.rs` | 79 | 588 | SPEC-core-02/03 menu model rows (paging, shortcut rows, delay popup); chords, chord capture, the INI shortcuts file and the menu-close commit rule stay (`ui_chord_capture`, `input04`, `findings_menu_close`) |
| `tests/menu_nav.rs` | 22 | 531 | ZL + Up/Down section jumps and the remembered menu place |
| `tests/core06_orbs.rs` | 9 | 518 | own orbs sink and board (the hitbox visualizer's rules and sites stay in `core06_hitbox`) |
| `tests/state02_charges.rs` | 11 | 520 | which fields each fighter's charge lives in |
| `tests/state03_buffs.rs` | 12 | 521 | the per-fighter ApplyBuff step |
| `tests/state04_convert.rs` | 9 | 518 | one-time conversion of the retired `character_item` setting |
| `tests/state04_items.rs` | 7 | 516 | the item of a snapshot's choice |
| `tests/input05.rs` | 25 | 534 | notification queue and damage display helper |
| `tests/input_named.rs` | 8 | 517 | the named layer's frame source (the recorder's machine stays in `input01`) |
| `tests/migration.rs` | 16 | 525 | the first-boot import's pure plan, row by row; `migration_run` (kept) still drives that plan end to end on real directories (copy loop, receipt, interrupted and failed copies, second boot) |
| `tests/frame_strip_settings.rs` | 5 | 514 | the Input Display row and its migration |
| `tests/lobby_items.rs` | 9 | 521 | the Lobby Items row, defaults and import |
| `tests/developer_features.rs` | 9 | 518 | the hidden Developer rows |
| `tests/phase2_pkg6_storage.rs` | 3 | 512 | the Q7 persisted capture choice |
| `tests/pkg6_item_table.rs` | 3 | 518 | the character-item table order |

### Host shims and diag channel (10 files, 47 tests of their own)

Why not necessary: Host shim of a Switch-only adapter or of the diag channel (a stand-in `skyline`/`nn` crate around one runtime file). It proves the shim's protocol, not the console's; the native side is qualified in Eden.

| File | Own tests | Listed before | What it covered |
| --- | ---: | ---: | --- |
| `tests/cpu_role_lifetime.rs` | 14 | 1,828 | the CPU role retirement adapter with native-call spies (also re-ran 1,828 by-path tests) |
| `tests/ui_install_boundary.rs` | 4 | 513 | the anchor installer's installation-spy contract |
| `tests/ui_native_pause.rs` | 6 | 518 | layout callback, pane walker and input-delay adapter on a host shim |
| `tests/ui_pane_boundary.rs` | 6 | 6 | pane traversal over host-owned lists |
| `tests/runtime_diag_log.rs` | 1 | 510 | the diag log channel on temporary files |
| `tests/carried_diag_ring.rs` | 7 | 7 | the diagnostics ring (diag builds only) |
| `tests/capture_completion.rs` | 4 | 4 | the capture completion trace |
| `tests/pristine_load.rs` | 4 | 4 | the pristine-load policy |
| `tests/pkg4_capture_status.rs` | 1 | 1 | next-record preference versus active arming |
| `tests/p1_core.rs` | 0 | 25 | the carried CPU-as-P1 helpers' unit tests, run by path (the hooks' no-allocation gate stays: `p1_pilot_hooks`) |

### Runners and duplicates (4 files, 5 tests of their own)

Why not necessary: Runner or duplicate of another gate.

| File | Own tests | Listed before | What it covered |
| --- | ---: | ---: | --- |
| `tests/eden_harness.rs` | 1 | 1 | ran `tests/eden/test_harness.py` (the Eden claim ledger parser) from cargo; the Python suite is unchanged and runs directly |
| `tests/eden_scripts.rs` | 2 | 2 | ran `tests/eden/test_analyze.py` from cargo; unchanged, runs directly |
| `tests/f6_split.rs` | 2 | 10 | the diag F6 split model and its evidence-table script (`tools/test_f6_split_table.py` still runs directly) |
| `tests/macro_mailbox.rs` | 0 | 6 | re-ran `crates/hos-lock`'s mailbox unit tests in the root gate; they run in that crate's own `cargo test` (docs/BUILD.md) |

Own tests in the 80 files: 494. "Listed before" also counts the by-path unit tests each binary re-ran.

Their helpers went with them: `tests/common/preload_allocations.rs`, `tests/common/res_unload_host.rs`,
`tests/support/mii_cycle_compact.rs`. The Python suites under `tests/eden/` and `tools/test_*.py` are unchanged.

## Cut: unit-test modules in `src/` (by area)

| Area | Test modules before | Tests before | Modules kept | Tests kept | Why the rest is not necessary |
| --- | ---: | ---: | ---: | ---: | --- |
| other `src/model/*.rs` | 61 | 427 | 38 | 170 | See the per-file list below. |
| engine::swap | 37 | 423 | 0 | 0 | Carried with the fork's code (packages 4 and 6): policy-table and decision tests of pure code the carry manifests pin byte for byte. None is in a required group (game-thread hand-off, lock, SVC, stack, build tag, seam guard, chord, game speed, settings or recording file); the features' acceptance is their Eden rows. Kept here where a row shows it: the `cycle_preload.ini` transactions (a settings file) and the waiting-room stage cycle. |
| engine::mii | 26 | 166 | 0 | 0 | Carried with the fork's code (packages 4 and 6): policy-table and decision tests of pure code the carry manifests pin byte for byte. None is in a required group (game-thread hand-off, lock, SVC, stack, build tag, seam guard, chord, game speed, settings or recording file); the features' acceptance is their Eden rows. Kept here where a row shows it: the `cycle_preload.ini` transactions (a settings file) and the waiting-room stage cycle. |
| engine::macros | 14 | 124 | 4 | 41 | Capture-window, live-capture, toggle and replay policy detail. The recording file format, the library's files and the capture archive keep their tests (`recording`, `library`, `capture_archive`). |
| engine::preload | 10 | 118 | 1 | 9 | Carried with the fork's code (packages 4 and 6): policy-table and decision tests of pure code the carry manifests pin byte for byte. None is in a required group (game-thread hand-off, lock, SVC, stack, build tag, seam guard, chord, game speed, settings or recording file); the features' acceptance is their Eden rows. Kept here where a row shows it: the `cycle_preload.ini` transactions (a settings file) and the waiting-room stage cycle. |
| engine::macro_ui | 6 | 82 | 0 | 0 | Menu, panel and screen text or layout models: presentation detail, seen in the menu. |
| model UI (screens, frame, flow, adapter, panes) | 8 | 75 | 0 | 0 | Menu, panel and screen text or layout models: presentation detail, seen in the menu. |
| runtime (Switch-only files) | 19 | 71 | 0 | 0 | Switch-only files: their unit tests compiled only inside the by-path Mii and preload suites that are cut; `cargo test` never built them otherwise. |
| engine::seams | 9 | 66 | 8 | 57 | Only the seam *report* formatting (pending/again lines) went; every guard test stays. |
| engine::rr_host / rr_runtime | 2 | 64 | 0 | 0 | Carried with the fork's code (packages 4 and 6): policy-table and decision tests of pure code the carry manifests pin byte for byte. None is in a required group (game-thread hand-off, lock, SVC, stack, build tag, seam guard, chord, game speed, settings or recording file); the features' acceptance is their Eden rows. Kept here where a row shows it: the `cycle_preload.ini` transactions (a settings file) and the waiting-room stage cycle. |
| crate root (`src/*.rs`) | 14 | 60 | 5 | 22 | See the per-file list below. |
| engine::lobby | 10 | 55 | 0 | 0 | Carried with the fork's code (packages 4 and 6): policy-table and decision tests of pure code the carry manifests pin byte for byte. None is in a required group (game-thread hand-off, lock, SVC, stack, build tag, seam guard, chord, game speed, settings or recording file); the features' acceptance is their Eden rows. Kept here where a row shows it: the `cycle_preload.ini` transactions (a settings file) and the waiting-room stage cycle. |
| engine::mii_root | 9 | 55 | 0 | 0 | Carried with the fork's code (packages 4 and 6): policy-table and decision tests of pure code the carry manifests pin byte for byte. None is in a required group (game-thread hand-off, lock, SVC, stack, build tag, seam guard, chord, game speed, settings or recording file); the features' acceptance is their Eden rows. Kept here where a row shows it: the `cycle_preload.ini` transactions (a settings file) and the waiting-room stage cycle. |
| engine::stages | 8 | 48 | 1 | 11 | Carried with the fork's code (packages 4 and 6): policy-table and decision tests of pure code the carry manifests pin byte for byte. None is in a required group (game-thread hand-off, lock, SVC, stack, build tag, seam guard, chord, game speed, settings or recording file); the features' acceptance is their Eden rows. Kept here where a row shows it: the `cycle_preload.ini` transactions (a settings file) and the waiting-room stage cycle. |
| editors | 8 | 46 | 0 | 0 | Menu, panel and screen text or layout models: presentation detail, seen in the menu. |
| engine::p1 / p1_session | 4 | 25 | 0 | 0 | Carried with the fork's code (packages 4 and 6): policy-table and decision tests of pure code the carry manifests pin byte for byte. None is in a required group (game-thread hand-off, lock, SVC, stack, build tag, seam guard, chord, game speed, settings or recording file); the features' acceptance is their Eden rows. Kept here where a row shows it: the `cycle_preload.ini` transactions (a settings file) and the waiting-room stage cycle. |
| engine::compat | 5 | 24 | 0 | 0 | Carried with the fork's code (packages 4 and 6): policy-table and decision tests of pure code the carry manifests pin byte for byte. None is in a required group (game-thread hand-off, lock, SVC, stack, build tag, seam guard, chord, game speed, settings or recording file); the features' acceptance is their Eden rows. Kept here where a row shows it: the `cycle_preload.ini` transactions (a settings file) and the waiting-room stage cycle. |
| model::cpu (CPU level picker) | 1 | 20 | 0 | 0 | A pure table lookup with no failure mode beyond a wrong menu value. |
| engine::library | 1 | 15 | 1 | 15 | all kept |
| model::state | 3 | 13 | 1 | 7 | Preview and quick-reset detail; the setup transaction (a file write) stays. |
| engine::kirby | 1 | 9 | 0 | 0 | Carried with the fork's code (packages 4 and 6): policy-table and decision tests of pure code the carry manifests pin byte for byte. None is in a required group (game-thread hand-off, lock, SVC, stack, build tag, seam guard, chord, game speed, settings or recording file); the features' acceptance is their Eden rows. Kept here where a row shows it: the `cycle_preload.ini` transactions (a settings file) and the waiting-room stage cycle. |
| engine::items | 1 | 6 | 0 | 0 | Carried with the fork's code (packages 4 and 6): policy-table and decision tests of pure code the carry manifests pin byte for byte. None is in a required group (game-thread hand-off, lock, SVC, stack, build tag, seam guard, chord, game speed, settings or recording file); the features' acceptance is their Eden rows. Kept here where a row shows it: the `cycle_preload.ini` transactions (a settings file) and the waiting-room stage cycle. |
| engine::mii_install | 1 | 5 | 0 | 0 | Carried with the fork's code (packages 4 and 6): policy-table and decision tests of pure code the carry manifests pin byte for byte. None is in a required group (game-thread hand-off, lock, SVC, stack, build tag, seam guard, chord, game speed, settings or recording file); the features' acceptance is their Eden rows. Kept here where a row shows it: the `cycle_preload.ini` transactions (a settings file) and the waiting-room stage cycle. |
| **Total** | **258** | **1997** | **59** | **332** | |

The cut modules of the two "mixed" areas, each with its test count:

- combo and drill model detail (plan, record, share files, notices): feature logic with Eden rows; no ship gate: `engine/mod.rs::sequence_drill_contract` (7), `model/combo.rs::tests` (41), `model/combo_plan.rs::tests` (9), `model/combo_record.rs::tests` (5), `model/combo_record.rs::share_tests` (8), `model/combo_record.rs::notice_tests` (3), `model/drill.rs::tests` (4).
- feature or menu model detail; no thread, lock, seam or file involved: `lib.rs::preload_allocation_tests` (20), `lib.rs::res_unload_host` (0), `lib.rs::f9_seat_tests` (1), `lib.rs::planner_alloc_tests` (1), `lib.rs::clip_input_tests` (2), `lib.rs::kirby_hat_cpu_tests` (4), `lib.rs::numbered_picker_tests` (3), `model/body_activation.rs::tests` (2), `model/build_split.rs::tests` (8), `model/deluxe_row.rs::tests` (9), `model/developer_ini.rs::tests` (1), `model/effect_chips.rs::tests` (7), `model/effect_texture.rs::tests` (12), `model/frame_strip.rs::tests` (23), `model/frame_strip_plan.rs::tests` (14), `model/lobby_items.rs::tests` (8), `model/lobby_persist.rs::tests` (6), `model/menu_shortcuts.rs::tests` (10), `model/parked_entries.rs::tests` (20), `model/pause_helper.rs::tests` (8), `model/playback.rs::tests` (8), `model/recording_menu.rs::tests` (41), `model/ui_texture.rs::tests` (7).
- other small models with no thread, lock, seam or file involved: `engine/mod.rs::mii_fork_tests` (0), `model/stage_hazards.rs::tests` (3).

31 test-only files went with their modules (`*_tests.rs`, `tests.rs`, and three files declared only under `#[cfg(test)]`:
`sequence_drill_contract.rs`, `swap_model/order_replay.rs`, `swap_model/pair_after_objects.rs`), and `mii_fork_tests/`.

## Non-test lines this touched, and the proof nothing shipped changed

`cfg(test)` is never set in the Switch build, so none of these changes it:

- `src/lib.rs`: the `#[cfg(test)]` macro `lib_unit_tests!`; the declarations of the removed test modules.
- `src/model/compat/menu.rs`: two import names (`DEFAULT_CPU_ROSTER`, `DEFAULT_PLAYER_ROSTER`) only the file's removed unit tests used.
- `src/model/mii/replay.rs`: a `#[cfg(test)] use`. `src/model/preload/preload_metrics.rs`: two `#[cfg(test)]` associated items.
  `src/model/rr_host.rs`: the `#[cfg(test)] fn reset_host`.
- Items whose only host users were removed tests go from `cfg(any(test, target_os = "switch"))` to `cfg(target_os = "switch")`:
  `rr_host::capture_role`, `rr_host::take_cpu_frame`, `stages::practice_stage_request_policy`; and three modules of `swap/mod.rs` gain
  that gate (`source_descriptor`, `source_visible_forms`, `native_hud_refresh`). Without it the host test build warns dead code.
- `tools/layout-builder/src/main.rs`: defines `lib_unit_tests!` (pass-through) for the `src/` file it includes by path.

Proof (release NRO of `28082c29` against this branch, `cargo skyline build --release`, both 4,136,960 B, resident 5,902,336 B):
57 bytes differ: the 32-byte build id, the 8 characters of the commit in the build tag, and 17 bytes of line numbers in 16 panic
locations (lines moved where a test module sat above code). The code is byte-identical. `proof.py` (kept with the lane's evidence, `artifacts/phase23/rc9-test-cut/` in the workspace) compares every
`src/`, `crates/` and `third_party/` file with its test modules removed: 671 identical, 14 differ (the files above and wrapped test files).

## Gate scripts and tools changed

- `tools/test_cut.py` (new): removes or wraps test-gated modules, lexer-aware; `python3 -I tools/test_cut.py selftest`.
- `tools/carry_pkg4.py`, `tools/carry_pkg6.py`: the manifest edits `cut_tests`, `wrap_tests`, `wrap_file` and the entry flag `removed`,
  so `carry` and `verify` re-derive every carried file from the fork stage without the cut tests (118 entries edited, 17 removed).
  `verify` against stage `m5q` passes for both; the manifests and the generated carry ledgers are regenerated.
- `tools/rc5_merge_accept.py` (the rc.5 merge check that every branch's test names survive) is historical and not a gate; it would
  now report the cut names as lost. Left as it is.
- No gate counted tests: `size_gate.py`, `stack_depth.py … table`, `svc_clobber_check.py`, `game_thread_wait_check.py` and
  `stack_marks.py` read the tree or the ELF and are unchanged.

## Per-binary counts

| Binary | Before | After |
| --- | ---: | ---: |
| `src/lib.rs` | 1,913 | 331 |
| `tests/cpu_role_lifetime.rs` | 1,828 | — |
| `tests/playback_hook.rs` | 977 | 3 |
| `tests/core02_menu.rs` | 588 | — |
| `tests/core06_hitbox.rs` | 541 | 29 |
| `tests/input01.rs` | 540 | 31 |
| `tests/migration_run.rs` | 540 | 31 |
| `tests/cpu_core.rs` | 537 | 28 |
| `tests/state01_machine.rs` | 536 | 27 |
| `tests/input05.rs` | 534 | — |
| `tests/thread_stacks.rs` | 532 | 16 |
| `tests/menu_nav.rs` | 531 | — |
| `tests/input04.rs` | 529 | 20 |
| `tests/capabilities.rs` | 527 | 15 |
| `tests/core01.rs` | 527 | 18 |
| `tests/core04_settings.rs` | 527 | 18 |
| `tests/cpu04.rs` | 527 | 18 |
| `tests/cpu02.rs` | 526 | 17 |
| `tests/migration.rs` | 525 | — |
| `tests/cpu03.rs` | 524 | 15 |
| `tests/p1_pilot_hooks.rs` | 524 | 7 |
| `tests/cpu05.rs` | 522 | 13 |
| `tests/state01_json.rs` | 522 | 13 |
| `tests/ui_chord_capture.rs` | 522 | 13 |
| `tests/cpu01.rs` | 521 | 12 |
| `tests/lobby_items.rs` | 521 | — |
| `tests/state03_buffs.rs` | 521 | — |
| `tests/combo_lobby_wiring.rs` | 520 | — |
| `tests/cpu06.rs` | 520 | 11 |
| `tests/input_delay.rs` | 520 | 11 |
| `tests/state02_charges.rs` | 520 | — |
| `tests/cpu_sites.rs` | 519 | 10 |
| `tests/state06_07.rs` | 519 | 10 |
| `tests/core04_recovery.rs` | 518 | 9 |
| `tests/core06_orbs.rs` | 518 | — |
| `tests/developer_features.rs` | 518 | — |
| `tests/hook_alloc.rs` | 518 | 9 |
| `tests/pkg6_item_table.rs` | 518 | — |
| `tests/state04_convert.rs` | 518 | — |
| `tests/ui_native_pause.rs` | 518 | — |
| `tests/input_named.rs` | 517 | — |
| `tests/input_sites.rs` | 517 | 8 |
| `tests/state01_setup.rs` | 517 | 8 |
| `tests/u4b_sites.rs` | 517 | 8 |
| `tests/notice_plates.rs` | 516 | — |
| `tests/state04_items.rs` | 516 | — |
| `tests/state05_restore.rs` | 516 | 7 |
| `tests/input_pause.rs` | 515 | 6 |
| `tests/intro_once.rs` | 515 | — |
| `tests/pkg6_sites.rs` | 515 | 6 |
| `tests/u4_sites.rs` | 515 | 6 |
| `tests/frame_strip_settings.rs` | 514 | — |
| `tests/state_sites.rs` | 514 | 5 |
| `tests/ui_readiness.rs` | 514 | 5 |
| `tests/cpu_common.rs` | 513 | 4 |
| `tests/findings_menu_close.rs` | 513 | 4 |
| `tests/lobby_persist_wiring.rs` | 513 | — |
| `tests/mii_same_notice.rs` | 513 | — |
| `tests/ui_install_boundary.rs` | 513 | — |
| `tests/ui_sites.rs` | 513 | 4 |
| `tests/cpu07.rs` | 512 | 3 |
| `tests/phase2_pkg6_storage.rs` | 512 | — |
| `tests/pkg6_smash_calls.rs` | 512 | 3 |
| `tests/ui_view_indices.rs` | 512 | 3 |
| `tests/core07_assets.rs` | 511 | — |
| `tests/hook_tls.rs` | 511 | 2 |
| `tests/input_mapping_callers.rs` | 511 | 2 |
| `tests/runtime_diag_log.rs` | 510 | — |
| `tests/ui_notice_palette.rs` | 510 | — |
| `tests/ui_owner.rs` | 510 | 1 |
| `tests/mii_configuration.rs` | 178 | — |
| `tests/mii_replay185_headgear.rs` | 177 | — |
| `tests/inline_keyboard.rs` | 176 | — |
| `tests/mii_ldn_handshake.rs` | 175 | — |
| `tests/mii_role_adapter.rs` | 174 | — |
| `tests/mii_setup_preflight.rs` | 173 | — |
| `tests/mii_bank_groups.rs` | 171 | — |
| `tests/mii_bank_publication.rs` | 169 | — |
| `tests/mii_helper147_headgear.rs` | 169 | — |
| `tests/mii_player146_headgear_ownership.rs` | 169 | — |
| `tests/inline_keyboard_native.rs` | 167 | — |
| `tests/catalog_reply.rs` | 33 | 7 |
| `tests/p1_core.rs` | 25 | — |
| `tests/lock_order.rs` | 21 | 18 |
| `tests/entry_cells.rs` | 12 | 6 |
| `tests/lockfree_wiring.rs` | 12 | 9 |
| `tests/hook_locks.rs` | 11 | 11 |
| `tests/input_pending.rs` | 11 | 5 |
| `tests/notice_clearance.rs` | 11 | — |
| `tests/pkg6_features.rs` | 11 | 11 |
| `tests/f6_split.rs` | 10 | — |
| `tests/no_spin_locks.rs` | 10 | 10 |
| `tests/pane_recipe_guard.rs` | 10 | 2 |
| `tests/cycle_change_wiring.rs` | 9 | — |
| `tests/background_native_wiring.rs` | 8 | — |
| `tests/carried_guard.rs` | 8 | 8 |
| `tests/combo_switch_wiring.rs` | 8 | — |
| `tests/cpu_cycle_wiring.rs` | 8 | — |
| `tests/rc3_review_wiring.rs` | 8 | — |
| `tests/background_pair_wiring.rs` | 7 | — |
| `tests/carried_diag_ring.rs` | 7 | — |
| `tests/background_mii_wiring.rs` | 6 | — |
| `tests/carried_model_facade.rs` | 6 | 6 |
| `tests/combo_share_wiring.rs` | 6 | — |
| `tests/macro_mailbox.rs` | 6 | — |
| `tests/mii_exit_wiring.rs` | 6 | — |
| `tests/momentum_wiring.rs` | 6 | — |
| `tests/native_wrappers.rs` | 6 | 3 |
| `tests/player_cycle_wiring.rs` | 6 | — |
| `tests/replay_window_wiring.rs` | 6 | — |
| `tests/ui_pane_boundary.rs` | 6 | — |
| `tests/background_build_wiring.rs` | 5 | — |
| `tests/give_item_wiring.rs` | 5 | — |
| `tests/mii_install_boundary.rs` | 5 | — |
| `tests/seat_notice_wiring.rs` | 5 | — |
| `tests/setup_restore_wiring.rs` | 5 | — |
| `tests/startup_window_wiring.rs` | 5 | — |
| `tests/timer_recording.rs` | 5 | — |
| `tests/capture_completion.rs` | 4 | — |
| `tests/mii_switch_installed_wiring.rs` | 4 | — |
| `tests/online_flag.rs` | 4 | — |
| `tests/pristine_load.rs` | 4 | — |
| `tests/recording_switch_mii_wiring.rs` | 4 | — |
| `tests/body_window_wiring.rs` | 3 | — |
| `tests/game_thread_handoffs.rs` | 3 | 3 |
| `tests/hat_slots_wiring.rs` | 3 | — |
| `tests/kirby_hat_cpu.rs` | 3 | — |
| `tests/lint_allows.rs` | 3 | 3 |
| `tests/menu_writeback.rs` | 3 | 3 |
| `tests/mii_bust_context.rs` | 3 | — |
| `tests/native_service_wiring.rs` | 3 | 3 |
| `tests/carry_pkg4.rs` | 2 | 2 |
| `tests/eden_scripts.rs` | 2 | — |
| `tests/group_hold_wiring.rs` | 2 | — |
| `tests/pause_helper_sites.rs` | 2 | 2 |
| `tests/pkg6_deps.rs` | 2 | 2 |
| `tests/preload_seat_wiring.rs` | 2 | — |
| `tests/size_budget.rs` | 2 | 2 |
| `tests/stack_marks.rs` | 2 | 2 |
| `tests/svc_clobber_abi.rs` | 2 | 2 |
| `tests/ui_glyphs.rs` | 2 | — |
| `tests/window_admission_wiring.rs` | 2 | — |
| `tests/build_tag.rs` | 1 | 1 |
| `tests/carry_pkg6.rs` | 1 | 1 |
| `tests/eden_harness.rs` | 1 | — |
| `tests/pkg4_capture_status.rs` | 1 | — |
| `tests/recording_icons.rs` | 1 | — |
| `tests/stage_request_init_order.rs` | 1 | — |
| **Total** | **41,918** | **942** |
