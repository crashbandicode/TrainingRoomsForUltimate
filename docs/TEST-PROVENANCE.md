# Test provenance

<a id="coverage-rc9"></a>

<a id="coverage-rc9--rc-9-line-coverage"></a>

## rc.9 line coverage

| State | Tests (`cargo test`) | Product lines | Covered | Line coverage |
| --- | ---: | ---: | ---: | ---: |
| Before the test cut (`28082c29`) | 41,918 | 40,012 | 36,345 | 90.84% |

- 2026-11-12.
- Line coverage of the `training_rooms` crate's host-compiled product code, measured with `cargo llvm-cov` 0.9.1 (rustc 1.98.1, LLVM 22) at `a25fbbc6`.
- The order was 95% if possible with at most 1,000 tests.
- **Reached: 91.54% with 865 tests.** What blocks the rest is under "Why not 95%".
- **Then reduced on the operator's choice to the cases that add coverage: 90.18% with 821 test functions and 1,689 cases** (the last section, "Option (b)"); the sections before it describe the state before that reduction.

<a id="coverage-rc9--figures"></a>

### Figures

| State | Tests (`cargo test`) | Product lines | Covered | Line coverage |
| --- | ---: | ---: | ---: | ---: |
| Before the test cut (`28082c29`) | 41,918 | 40,012 | 36,345 | 90.84% |
| After the test cut, the baseline of this work (`7a4cba64`) | 942 | 38,601 | 13,215 | 34.23% |
| Now (`a25fbbc6`) | 865 | 39,457 | 36,118 | 91.54% |

- Without the five `src/runtime/` files (see "Scope"): 35,499 of 38,458, **92.31%**.
- Tests now: 865 in 108 binaries; with the two vendored crates' own gates (28 + 24), 917. The limit is 1,000.
- The vendored crates are not counted: `crates/hos-lock` reads 95.35% (328 of 344 lines) from this crate's by-path includes;
  `crates/smash-match-data` and `third_party/` are not instrumented by this run (their own `cargo test` gates are unchanged).

<a id="coverage-rc9--how-it-was-raised"></a>

### How it was raised

| Coverage change | What the suite retained |
| --- | --- |
| Restored cases | Original assertions in table-driven tests |
| Special harness behavior | Tests using `ignore` or `should_panic` stayed standalone |

- **The cut tests came back as cases.** `tools/test_cut.py` (`tabulate_region`) turns the `#[test]` functions of a module into the rows of one `#[test] fn case_table()`.
  - Each case is the test function it was, unchanged, and asserts what it asserted.
  - `src/test_table.rs` runs each case on its own thread, as the harness did, runs every case even after one fails, and names the failed ones.
  - A test under `cfg`, `ignore` or `should_panic`, and one that re-executes the test binary by its own name (`current_exe`), stays a plain test.
  - The unit-test modules of `src/` are back (the 199 of the cut, and the 59 it kept are tables now too): 233 tables in 221 files.
  - 37 integration files are back as one table each: the menu model (`core02_menu`, `menu_nav`), the state detail rows
    (`state02`–`state04`), `input05`, `input_named`, `migration`, the Mii suites, `p1_core` and others. The seven CPU spec-row
    files (`cpu01`–`cpu07`) became tables too, which freed 82 tests.
  - Still cut, because they cover no product line (they read source text) or need a host shim of a Switch-only file: the 30
    `*_wiring` pins, `cpu_role_lifetime`, `ui_install_boundary`, `ui_native_pause`, `mii_install_boundary`,
    `inline_keyboard_native`, `runtime_diag_log`, the UI glyph/palette/icon checks, the Python-suite runners.
- **New cases** (3): the seam tables' compile-time projections recomputed at run time against a plain walk of the row files
  (`native_seams`), the install line's capacity parts against the site tables (`install_capacity`), and a Steve record against
  its parts (`state::steve`).
- The docs lane's files of the same day (`chords.rs`, `game_speed*.rs`, `recording.rs`, `state/quick_reset.rs`,
  `deluxe_overlay.rs`) are as that lane left them: their tests stay plain tests, and the three old Quick Reset unit tests
  (written for the earlier `input` signature) stay cut.

<a id="coverage-rc9--scope-and-exclusions"></a>

### Scope and exclusions

| Counted | Excluded |
| --- | --- |
| Instrumented host-product lines in `src/` | Test code, vendored crates, the layout builder and uncompiled Switch-only paths |

- **Counted:** every line llvm-cov instruments in a `src/` file compiled into a host test build (the lib's test binary and
  the by-path test crates), outside test code.
- **Not counted, with the reason:**
  - test code: lines inside `#[cfg(test)]` modules, test-only files, `src/test_table.rs`, `tests/**` (61,920 lines, 99.1%
    covered). Counting them would raise the figure to about 96%, which would say nothing about the product.
  - vendored crates and the layout builder (reported above).
  - code the host never compiles: `src/runtime/**` and other `cfg(target_os = "switch")` items. llvm-cov has no lines for them.
- **No line or file was excluded to raise the figure.** Five `src/runtime/` files are compiled on the host only inside test
  shims (`ui/view.rs`, `ui/pane.rs`, `hook_tls.rs`, `mii/mii_source_provenance.rs`, `roster/res_unload.rs`: 999 lines, 380
  uncovered, mostly `view.rs` drawing). They are Switch-only by declaration, so the order puts them out of scope; the headline
  figure still counts them, and the figure without them is given beside it.

<a id="coverage-rc9--per-module"></a>

### Per module

| Module | Lines | Before the cut | Baseline (cut) | Now | Uncovered now |
| --- | ---: | ---: | ---: | ---: | ---: |
| `src/model/*.rs` | 12,813 | 91.90% | 47.80% | 91.68% | 1,066 |
| `src/runtime/ui` | 769 | 52.44% | 44.86% | 51.37% | 374 |
| `src/model/macro_ui` | 1,738 | 88.03% | 0.69% | 88.03% | 208 |
| `src/model/swap` | 4,899 | 95.84% | 0.90% | 95.84% | 204 |
| `src/model/macros` | 1,945 | 90.64% | 35.68% | 90.64% | 182 |
| `src/model/state` | 2,127 | 90.98% | 73.98% | 92.43% | 161 |
| `src/model/mii` | 3,443 | 95.53% | 8.27% | 95.50% | 155 |
| `src/model/mii_root` | 821 | 81.73% | 12.55% | 81.73% | 150 |
| `src/model/cpu` | 1,883 | 94.26% | 88.79% | 92.94% | 133 |
| `src/model/lobby` | 1,224 | 90.11% | 4.25% | 89.79% | 125 |
| `src/model/ui` | 1,612 | 93.05% | 40.88% | 92.37% | 123 |
| `src/model/preload` | 1,639 | 92.74% | 22.74% | 92.74% | 119 |
| `src/model/menu` | 1,336 | 91.69% | 30.16% | 91.69% | 111 |
| `src/model/compat` | 437 | 79.63% | 10.98% | 79.63% | 89 |
| `src/editors/*.rs` | 621 | 91.14% | 0.00% | 91.14% | 55 |
| `src/model/library` | 517 | 92.65% | 88.01% | 92.65% | 38 |
| `src/model/seams` | 508 | 71.85% | 52.36% | 97.05% | 15 |
| `src/model/p1` | 251 | 94.82% | — | 94.82% | 13 |
| `src/model/stages` | 417 | 98.08% | 45.91% | 98.08% | 8 |
| `src/model/kirby` | 82 | 95.12% | 0.00% | 95.12% | 4 |
| `src/runtime/roster` | 88 | 95.45% | — | 95.45% | 4 |
| `src/runtime/*.rs` | 13 | 63.49% | 92.31% | 92.31% | 1 |
| `src/runtime/mii` | 129 | 92.06% | — | 99.22% | 1 |
| `src/install_capacity.rs` | 48 | 0.00% | 0.00% | 100.00% | 0 |
| `src/model/items` | 39 | 100.00% | 0.00% | 100.00% | 0 |
| `src/model/p1_session` | 58 | 100.00% | — | 100.00% | 0 |
| **Total** | **39,457** | **90.84%** | **34.23%** | **91.54%** | **3,339** |

A dash: the module was not compiled into that state's host test builds.

<a id="coverage-rc9--why-not-95"></a>

### Why not 95%

| 95% target on 39,457 lines | Observed |
| --- | --- |
| Maximum uncovered | 1,972 lines |
| Actual uncovered | 3,339 lines |
| Gap | 1,367 lines |

95% of 39,457 lines leaves at most 1,972 uncovered; 3,339 are. The other 1,367 lines are not blocked by the 1,000 limit
(135 tests of headroom; 83 with the vendored crates counted). They are blocked by what they are:

- The suite before the cut reached 90.84% with 41,918 tests. Everything above that needs tests nobody had written.
- The uncovered lines are a long tail: 3,339 lines in about 200 files, typically 4 to 12 lines each. They are refusal and error
  branches (an I/O error half way through a write, a malformed header field, a full table), `Display`/label arms of UI models,
  and Switch-facing drawing code compiled through a shim. Each needs its own arranged input and its own assertion; a test that
  only executes them would not meet the order.
- The largest remaining files:

| File | Lines | Uncovered | What is uncovered |
| --- | ---: | ---: | --- |
| `src/runtime/ui/view.rs` | 632 | 287 | drawing of connections, strokes, chips and tiles through the host shim (Switch-only file) |
| `src/model/recording_menu.rs` | 1,436 | 197 | file-row actions (bind, rename), sequence creation and connection lines, token labels |
| `src/model/macro_ui/macro_panel.rs` | 1,208 | 160 | copy-playback, write-over, rename and library paging branches |
| `src/model/mii_root/mii_menu.rs` | 278 | 134 | context hand-over, numbered/unnumbered CPU choices, confirm and back paths |
| `src/model/macros/capture_archive.rs` | 595 | 124 | I/O and validation error branches of list, save and read |
| `src/model/rr_host.rs` | 1,144 | 123 | host atomics paths the worker tick reaches only with a live recorder |
| `src/model/combo_plan.rs` | 491 | 100 | look/label arms of the helper cells and reason texts |
| `src/model/menu/mod.rs` | 1,155 | 100 | popup and grid edge branches |
| `src/runtime/ui/pane.rs` | 137 | 87 | pane resolution through the host shim (Switch-only file) |
| `src/model/lobby/automation/adapter.rs` | 591 | 71 | refusal branches of the automation adapter |

The per-file table of every uncovered line count is reproducible with the commands below (`--files 300`).

<a id="coverage-rc9--reproduce"></a>

### Reproduce

| Reproduction input | Why it matters |
| --- | --- |
| Fresh target directory | Avoid stale binaries with zero counts |
| LCOV export | Include product files reached through by-path test crates |

```text
rustup component add llvm-tools                      # into the project's RUSTUP_HOME (toolchain 1.98.1)
cargo install cargo-llvm-cov --locked                # 0.9.1, into the project's CARGO_HOME
CARGO_TARGET_DIR=<fresh dir> cargo llvm-cov --no-fail-fast --no-report
python3 -I tools/coverage.py export <fresh dir> cov.lcov
python3 -I tools/coverage.py report cov.lcov . --files 40 --tsv cov.tsv
cargo test -- --list                                 # the test count
```

`cargo llvm-cov report` is not used: it drops every path that contains `/tests/`, and with it the lines of `src/` files reached
through a by-path test crate (`tests/../src/model/x.rs`). Its figure for the baseline was 24.43%; the export above gives 34.23%.
A target directory reused from an earlier commit also skews the export (old binaries are listed with zero counts).

<a id="coverage-rc9--product-code"></a>

### Product code

| NRO comparison with `ef4113c2` | Bytes changed per NRO |
| --- | --- |
| Build ID | 32 |
| Commit tag | 7 |
| Panic line numbers | 17 |

- No product behaviour change.
- Release and diag NRO sizes stayed unchanged. Each differed in 56 bytes, including line numbers at 16 panic locations.
- The test cut's re-gates of `swap/mod.rs`, `stages/mod.rs` and `rr_host.rs` and its removed `#[cfg(test)]` helpers are back as they were at `28082c29`, because their tests are back; the import edit in `compat/menu.rs` stays (the two names are imported by its test module).
- `docs/TEST-CUT-LEDGER-rc9.md` describes the cut as it was made; where it says a module's tests are removed, this file says how they returned.

<a id="coverage-rc9--option-b-only-the-cases-that-add-coverage-2026-11-12"></a>

### Option (b): only the cases that add coverage (2026-11-12)

| | Test functions | Cases | Line coverage |
| --- | ---: | ---: | ---: |
| Before (`b1e27e5a`) | 863 | 2,894 | 91.53% (36,116 of 39,457) |
| After | 821 | 1,689 | 90.18% (35,583 of 39,457) |

The table tests above put the cut tests back as rows: 863 test functions, but 2,894 cases. The operator chose to keep only the
cases that add coverage nothing else gives, with at most a 1.5-point drop.

- A case is one row of a `case_table` or one plain `#[test]`. After: 1,113 rows in 245 tables and 576 plain tests.
- With the two vendored crates' own gates the test functions are 873. The limit is 1,000.

<a id="coverage-rc9--method"></a>

#### Method

- **Per-case lines.** `tools/case_cover.py measure` runs every case alone, in its own process with its own profile
  (`TR_TABLE_CASE` selects one row; a plain test runs with `--exact`), and exports the product lines it covers from its own
  binary. The union of the 2,894 per-case sets is the full run's set (36,118 lines against 36,116).
- **Always kept (1,065 cases), whatever they cover:**
  - 942: kept by the test cut.
  - 80: required guard group.
  - 34: rc.8/rc.9 fix case.
  - 6: docs lane / kept file.
  - 3: new case of the coverage work.
  - "Kept by the test cut" is every test of `7a4cba64`, which holds the required groups of the test-cut order. "Required guard
    group" adds restored cases whose path names one of those groups (hand-offs, `nowait_channel`, locks, svc, stacks, build tag,
    seams, chords, game and lobby speed, settings, store, save-state, setup, recording, library and capture files). An "rc.8/rc.9
    fix case" is a restored case whose text names rc.8, rc.9, C3/C4/C10/C13/C14, R1/R2/R7/R14, a freeze, a hang or a spin.
- **Greedy set cover over the rest (1,829 cases).** Starting from the lines the always-kept cases cover (19,157), it adds the
  case with the most new lines until 35,560 lines are covered (90.12%; the allowed floor is 90.0%, 35,512 lines). It kept 565.
- **59 more cases stay** in eight files (`deluxe_row.rs`, `swap/source_load.rs`, `swap_model/copy_registry.rs`, `rr_host.rs`,
  `runtime/roster/observe.rs`, `tests/mii_bust_context.rs`, `tests/capture_completion.rs`, `tests/common/preload_allocations.rs`):
  dropping them would leave a test model half used (unread fields, an unused variant) or a product item without a host user,
  which would need a `cfg` re-gate in product code. Keeping the cases avoids both.
- **Removed: 1,205 cases**, with their code, the test helpers and imports only they used, 20 test modules and 6 integration
  files left without a case (`lobby_items`, `notice_plates`, `pkg4_capture_status`, `pkg6_item_table`, `state04_convert`,
  `timer_recording`).

<a id="coverage-rc9--what-was-dropped-and-why-it-is-redundant"></a>

#### What was dropped, and why it is redundant

- All but a few dropped cases cover only product lines that the kept cases also cover: measured, case by case.
- The few that covered more account for the cost: 533 lines (1.35 points) that only dropped cases covered.
- This is redundancy of coverage, not of assertions: a dropped case may have asserted a different outcome on the same lines.
- None is in a required group or names an rc.8/rc.9 fix.

| Area | Cases dropped | Files | What they were |
| --- | ---: | ---: | --- |
| `src/model/swap` | 282 | 32 | body-bank, source and swap-model policy cases of the carried fork engine: many scenarios over the same planner and ledger paths |
| integration files (`tests/*.rs`) | 202 | 31 | spec-row cases of the menu model, state detail, input notifications, Mii suites and the first-boot plan that walk code other rows already walk |
| other `src/model/*.rs` (combo, frame strip, recording menu, effect, drill and other models) | 180 | 25 | UI-model and feature-model cases repeating a path with another label, row or value |
| `src/model/mii` | 114 | 20 | Mii policy cases (ordinary player/CPU, inventory, selection, saved) over shared admission and recapture paths |
| `src/model/preload` | 74 | 8 | cycle-preload config and policy cases over the same parse and decision paths |
| `src/model/macros` | 61 | 9 | capture window, toggle and live-capture cases; the recording, library and archive file cases are kept |
| `src/model/ui` | 56 | 8 | screen, frame and pane text cases |
| `src/model/macro_ui` | 44 | 6 | clip and macro panel cases |
| `src/model/lobby` | 34 | 10 | waiting-room automation cases |
| `src/model/mii_root` | 33 | 3 | Mii HUD portrait policy and menu cases |
| `src/model/stages` | 26 | 7 | stage rotation and request policy cases |
| `src/editors` | 24 | 7 | roster, stage and clip editor cases |
| `src/model/cpu` | 15 | 1 | cases over paths the kept cases already cover |
| `src/model/compat` | 14 | 4 | cases over paths the kept cases already cover |
| `src/model/p1` | 12 | 2 | cases over paths the kept cases already cover |
| `src/model/mii_fork_tests` | 7 | 2 | cases over paths the kept cases already cover |
| crate root test files (`src/*_tests.rs`) | 6 | 4 | cases over paths the kept cases already cover |
| `src/model/kirby` | 6 | 1 | cases over paths the kept cases already cover |
| `src/runtime` unit tests run through a host include | 5 | 2 | cases over paths the kept cases already cover |
| `src/model/items` | 4 | 1 | cases over paths the kept cases already cover |
| `src/model/p1_session` | 3 | 2 | cases over paths the kept cases already cover |
| `src/model/state` | 2 | 1 | cases over paths the kept cases already cover |
| shared test hosts (`tests/common`, `tests/support`) | 1 | 1 | cases over paths the kept cases already cover |
| **Total** | **1,205** | **187** | |

The full list (file, case, binary, test path) is `artifacts/phase23/rc9-coverage/option-b/dropped.tsv` in the workspace; the kept
cases and why each is kept are in `kept.tsv` beside it. Every dropped case is in git: `git show b1e27e5a:<file>`.

<a id="coverage-rc9--reproduce-1"></a>

#### Reproduce

```text
CARGO_TARGET_DIR=<fresh dir> cargo llvm-cov --no-fail-fast --no-report
python3 -I tools/case_cover.py count <fresh dir>                  # cases, table rows, plain tests
python3 -I tools/case_cover.py measure <fresh dir> <out dir>      # per-case lines (about 12 minutes, 4 jobs)
python3 -I tools/coverage.py export <fresh dir> cov.lcov && python3 -I tools/coverage.py report cov.lcov .
```

<a id="coverage-rc9--product-code-1"></a>

#### Product code

No product behaviour change. The release and diag NROs at `b1e27e5a` and after this change have the same sizes and differ in 53
bytes each (build id, commit characters in the build tag, panic-location line numbers). With test modules removed, every `src/`
file is the text it was, except a `#[cfg(test)] use` in `mii/replay.rs` that only removed cases used.

<a id="test-cut-ledger-rc9"></a>

<a id="test-cut-ledger-rc9--rc-9-test-cut-ledger"></a>

## rc.9 test cut ledger

| | Test binaries | Tests (`cargo test -- --list`) | Distinct test names |
| --- | ---: | ---: | ---: |
| Before (`28082c29`) | 148 | 41,918 | 3,292 |

2026-11-11. Operator order (`artifacts/phase23/TASK-rc9-test-cut.md`): the host tests go to at most 1,000, a hard limit, keeping what
guards real behaviour and the ship gates. This file lists every cut and why, and what is kept. Base: `28082c29` (rc.8 notes on `main`).

- **2026-11-12:** most of the cut tests returned as table-driven tests (one `#[test]` per module) for the coverage order: `docs/COVERAGE-rc9.md`.
- Of those, the cases that add no line coverage were removed again (its "Option (b)" section): 1,205 cases, nearly all covering only lines a kept case covers.
- The count is 821 test functions and 1,689 cases.
- This file describes the cut as it was made at `7a4cba64`.

<a id="test-cut-ledger-rc9--before-and-after"></a>

### Before and after

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

<a id="test-cut-ledger-rc9--what-is-kept-and-what-each-protects"></a>

### What is kept, and what each protects

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

<a id="test-cut-ledger-rc9--cut-integration-test-files-80"></a>

### Cut: integration test files (80)

| Group | Files cut |
| --- | --- |
| Source pins | 30 |
| UI wording, glyphs and palette | 8 |
| Mii by-path suites | 13 |
| Pure model detail | 15 |
| Host shims and diagnostics | 10 |
| Runners and duplicates | 4 |

<a id="test-cut-ledger-rc9--wiring-rs-and-other-source-pins-30-files-115-tests-of-their-own"></a>

#### `*_wiring.rs` and other source pins (30 files, 115 tests of their own)

- Why not necessary: Source-text pin of one past fix: it reads the Switch-only runtime's `.rs` text through the call-graph scanner and asserts that one named call path or line still exists.
- The rule itself is host-tested in the model or was accepted in Eden; the pin fails on a refactor that keeps behaviour and passes on a wrong value at the right call site.
- Not a ship gate.

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

<a id="test-cut-ledger-rc9--ui-wording-glyphs-and-palette-8-files-26-tests-of-their-own"></a>

#### UI wording, glyphs and palette (8 files, 26 tests of their own)

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

<a id="test-cut-ledger-rc9--mii-by-path-suites-13-files-74-tests-of-their-own"></a>

#### Mii by-path suites (13 files, 74 tests of their own)

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

<a id="test-cut-ledger-rc9--spec-row-detail-of-pure-models-15-files-227-tests-of-their-own"></a>

#### Spec-row detail of pure models (15 files, 227 tests of their own)

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

<a id="test-cut-ledger-rc9--host-shims-and-diag-channel-10-files-47-tests-of-their-own"></a>

#### Host shims and diag channel (10 files, 47 tests of their own)

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

<a id="test-cut-ledger-rc9--runners-and-duplicates-4-files-5-tests-of-their-own"></a>

#### Runners and duplicates (4 files, 5 tests of their own)

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

<a id="test-cut-ledger-rc9--cut-unit-test-modules-in-src-by-area"></a>

### Cut: unit-test modules in `src/` (by area)

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

<a id="test-cut-ledger-rc9--non-test-lines-this-touched-and-the-proof-nothing-shipped-changed"></a>

### Non-test lines this touched, and the proof nothing shipped changed

| Switch build | Effect of test-side edits |
| --- | --- |
| `cfg(test)` | Unset; test declarations and helpers do not ship |

`cfg(test)` is never set in the Switch build, so none of these changes it:

- `src/lib.rs`: the `#[cfg(test)]` macro `lib_unit_tests!`; the declarations of the removed test modules.
- `src/model/compat/menu.rs`: two import names (`DEFAULT_CPU_ROSTER`, `DEFAULT_PLAYER_ROSTER`) only the file's removed unit tests used.
- `src/model/mii/replay.rs`: a `#[cfg(test)] use`. `src/model/preload/preload_metrics.rs`: two `#[cfg(test)]` associated items.
  `src/model/rr_host.rs`: the `#[cfg(test)] fn reset_host`.
- Items whose only host users were removed tests go from `cfg(any(test, target_os = "switch"))` to `cfg(target_os = "switch")`:
  `rr_host::capture_role`, `rr_host::take_cpu_frame`, `stages::practice_stage_request_policy`; and three modules of `swap/mod.rs` gain
  that gate (`source_descriptor`, `source_visible_forms`, `native_hud_refresh`). Without it the host test build warns dead code.
- `tools/layout-builder/src/main.rs`: defines `lib_unit_tests!` (pass-through) for the `src/` file it includes by path.

- Proof (release NRO of `28082c29` against this branch, `cargo skyline build --release`, both 4,136,960 B, resident 5,902,336 B): 57 bytes differ: the 32-byte build id, the 8 characters of the commit in the build tag, and 17 bytes of line numbers in 16 panic locations (lines moved where a test module sat above code).
- The code is byte-identical.
- `proof.py` (kept with the lane's evidence, `artifacts/phase23/rc9-test-cut/` in the workspace) compares every `src/`, `crates/` and `third_party/` file with its test modules removed: 671 identical, 14 differ (the files above and wrapped test files).

<a id="test-cut-ledger-rc9--gate-scripts-and-tools-changed"></a>

### Gate scripts and tools changed

| Tool | Role in the cut |
| --- | --- |
| `tools/test_cut.py` | Remove or wrap test-gated modules |
| Carry generators | Record removed or wrapped tests in carry manifests |

- `tools/test_cut.py` (new): removes or wraps test-gated modules, lexer-aware; `python3 -I tools/test_cut.py selftest`.
- `tools/carry_pkg4.py`, `tools/carry_pkg6.py`: the manifest edits `cut_tests`, `wrap_tests`, `wrap_file` and the entry flag `removed`,
  so `carry` and `verify` re-derive every carried file from the fork stage without the cut tests (118 entries edited, 17 removed).
  `verify` against stage `m5q` passes for both; the manifests and the generated carry ledgers are regenerated.
- `tools/rc5_merge_accept.py` (the rc.5 merge check that every branch's test names survive) is historical and not a gate; it would
  now report the cut names as lost. Left as it is.
- No gate counted tests: `size_gate.py`, `stack_depth.py … table`, `svc_clobber_check.py`, `game_thread_wait_check.py` and
  `stack_marks.py` read the tree or the ELF and are unchanged.

<a id="test-cut-ledger-rc9--per-binary-counts"></a>

### Per-binary counts

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

<a id="core-06"></a>

<a id="core-06--test-provenance-spec-core-06-the-hitbox-visualizer"></a>

## Test provenance — SPEC-core-06, the hitbox visualizer

| RC7 R3-2 change | What remained |
| --- | --- |
| Runtime HB1 reader and three exclusive guards removed | Attack slots, pending grab boxes and their cleanup |

Historical review opened 2026-11-02 (`57008f81`). Implementation details and outstanding checks refer to the revisions reviewed below.

<a id="core-06--rc7-r3-2-boundary-2026-11-03"></a>

### RC7 R3-2 boundary — 2026-11-03

| RC7 R3-2 change | What remained |
| --- | --- |
| Runtime HB1 reader and three exclusive guards removed | Attack slots, pending grab boxes and their cleanup |

The RC removes the runtime HB1 enumerator and its three exclusive HitModule
guard/generator rows. Ordinary attack slots and pending grab boxes still use
the same sphere geometry, tints, owned handles and cleanup. Menu help now says
hitboxes/grab boxes only. No independent hurtbox enable path exists in either
default or diagnostic closure.

- `model::hitbox::want_hitboxes/reconcile_frame` is the production planner and dispatcher used by `runtime::features::hitbox::frame`, not a test replacement.
- The three new `core06_rc_*` probes inspect actual attack-field addresses, requests, retention and stop/cleanup paths, plus the real capability closure.
- `tests/capabilities.rs` explicitly follows its four AttackReader callbacks.
- Existing pure HB1 shape/layout and synthetic handle tests below remain as historical facts, not evidence that the RC draws or qualifies hurtboxes.
- Their code remains counted under the unchanged implementation rules; it was not moved behind a new exclusion.
- The three native offsets in the old HB1 layout test are test-local facts, no longer RC guard dependencies.

The dated implementation/history below describes earlier candidates, not the
RC7 runtime. Frozen RC6 R3-2 source failure and original SH03 pool refusals are
preserved; removing HB1 does not prove the stock effect's resource issue fixed.

Stream `hitbox` (branch `stream/hitbox`), 2026-11-02. Spec: `specs/SPEC-core-06-hitbox-visualizer.md`. Code: `src/model/hitbox.rs`
(rules), `src/model/sites_hitbox.rs` (13 rows), `src/runtime/features/hitbox.rs` (hooks, pass, natives calls),
`src/runtime/game.rs` (`natives::` additions). Host tests: `tests/core06_hitbox.rs`; capability closure: `tests/capabilities.rs`.

<a id="core-06--spec-section-9-row-by-row"></a>

### Spec section 9, row by row

| Requirement | Host test | Eden row (`eden/EDEN-HITBOX.md`) | Evidence |
| --- | --- | --- | --- |
| Sphere count and spacing for capsules; single sphere for a point hitbox | `core06_capsule_sampling` | hb-E1 extended hitbox | `eden/runs/hb-*` |
| A grab box set the frame it connects shows that pass, not through the pull | `core06_orbs` `a_grab_box_set_in_the_frame_it_connects_shows_once_not_through_the_pull` | Eden `gbx-samus-t1` (rc.13: tether orbs in 56/58 only) | |
| Colour table by `id mod 8`; grab boxes magenta, a colour no hitbox uses | `core06_colour_table`, `every_grab_box_is_magenta_a_colour_no_hitbox_uses` | hb-E2 multi-hitbox move | |
| Fighter overlay gating (practice, setting, grab range and shield) | `core06_frame_rules_table` | hb-E3 grab and shield | |
| First-frame draw from the `ATTACK` / `CATCH` parameters (indices as spec section 3) | `core06_command_param_indices` | hb-E4 first active frame | |
| Rebound cleared kills the spheres unless shielding | `core06_rebound_rule` | hb-E5 | |
| Training-only gate incl. `077a8e43`'s early return; no effect-sleep calls elsewhere | `core06_training_gate` | hb-E6 Quickplay with the setting On | |
| Three sites guarded (64 B) and installed independently; neighbouring thunk windows compared first | `core06_site_guards`, `core06_rows_match_the_owned_elf`, `tests/core01.rs` `BOOT_PHASES` | hb-E7 install line | |
| Native effects continue with Hurtboxes on (supersedes the historical Q4 restore test) | `preview_visual_core06_visualizer_does_not_sleep_native_effects` | Palutena up smash with Hurtboxes on and off | |
| `AttackData` and Lua stack layouts the plugin reads | `core06_attack_data_layout_matches_the_owned_elf`, `core06_lua_stack_layout_matches_the_owned_elf` | (all rows) | below |
| Every native the feature calls is a granted capability; its closure is computed from the code | `tests/capabilities.rs` (`cross_needs_match_the_call_graph`, `a_poisoned_row_disables_...`, `the_native_call_layer_checks_the_grant`) | hb-E9 poisoned row | `CROSS_NEEDS[Hitbox]` |
| No heap allocation in the hooks and the pass | `HookScope` counters (`HookId::Hitbox*`), `tests/hook_alloc.rs` | hb-E10 `hook_stats` delta 0 | |
| Cost per frame | diag `[Hitbox] stats` line (ticks per pass, hitbox and hurtbox spheres) | hb-E10 | `eden/EDEN-HITBOX.md` |
| Hurtbox shapes per fighter (HB1 (a)): group 0 parts not switched off, capsule from `+0x08` / `+0x14`, size `+0x20`, node `+0x28`, yellow | `core06_hurtbox_shapes_per_fighter`, `core06_hurtbox_layout_matches_the_owned_elf` | hb-E11 screenshot with hurtboxes | |
| Hurtbox cost row | diag `[Hitbox] stats hurtbox_spheres= max_hurtbox_spheres=` | hb-E10 | `eden/EDEN-HITBOX.md` |

<a id="core-06--where-the-layouts-come-from-13-0-5-main-owned-elf-research-game-13-0-5-main-13-0-5-elf"></a>

### Where the layouts come from (13.0.5 `main`, owned ELF `research/game-13.0.5/main-13.0.5.elf`)

| Fact | Evidence (instruction at `main+...`) |
| --- | --- |
| slots are 0x130 bytes from the pointer at module `+0x68` | `attack_data` (vtable slot 0x1e8, `0x3dda80`): `ldr x9,[x0,#0x68]; mov w10,#0x130; madd x0,x8,x10,x9` (`0x3dda9c`..) |
| node (bone `Hash40`) at `+0x48` | `set_node` (slot 0x148, `0x3dd540`): `str x2,[x8,#0x48]`; `set_attack` copies `[x21+0x48] -> [x27+0x48]` (`0x3dc240`) |
| size at `+0x24` | `set_size` (slot 0x118, `0x3dd3b0`): `str s0,[x8,#0x24]` |
| offset vector (x, y, z) at `+0x00`, second vector at `+0x10` | `set_attack` (slot 0x68, `0x3dc180`): `str q0,[x19]` and `str q0,[x27,#0x10]`; `get_offset2` (slot 0x140, `0x3dd500`) calls `attack_data` and loads the vector at `+0x10` |
| active word at `+0x108` | `is_attack` (slot 0xc8, `0x3dd1f0`): `ldr w8,[x8,#0x108]` (non-zero, and the module's `+0x238` byte clear) |
| a second centre is the vector at `+0x10`, zero for a point hitbox | the `ATTACK` command zeroes the argument (`stp xzr,xzr,[sp,#0x30]`, `0x2284bd0`) and stores `q2` there only for a present second centre (`0x2285320`) |

Reproduce with `python3 -I tools/re_hitbox_layout.py <elf>` (it finds the `AttackModule` vtable from the NSO's relocation table, `main+0x4f6a5b8`,
and prints the functions below). `tests/core06_hitbox.rs` re-reads the ELF and asserts each instruction word named here.

`AttackData` (one attack slot; `attack_data(accessor, slot, false)` returns it):

The `ATTACK` command (`0x2284b80`, behind the `0x2284b20` wrapper) builds the same structure at `sp+0x20` (vectors `+0`, `+0x10`, size `+0x24` from
argument 9, node `+0x48` from argument 3) and calls vtable slot 0x68 with it (`ldr x8,[x8,#0x68]; add x3,sp,#0x20; blr x8`, `0x2285ba0`).

- Lua stack (`lua_State*`): top at `+0x10`, a pointer at `+0x20` whose first word is the frame base; argument `i` is the 16-byte value at `base + 0x10 * i` (nil when not below the top); payload at `+0`, tag at `+8`: 3 float, 0x13 integer (`scvtf` when a float is wanted); the node is the integer.
- Instructions: `ATTACK` `0x2284cd0..` (argument 4, `add x8,x8,#0x40`), `0x2284dd8` (9, size), `0x2284e90`/`0x2284f34`/`0x2284fe0` (10..12), `0x2285110`/`0x22851c0`/`0x2285270` (13..15, read when 13 and 14 are not nil); `CATCH` `0x22ac2ec` (argument 3, `add x8,x8,#0x30`).
- `sv_system::battle_object_module_accessor(L)` (`0x227daf0`): `ldur x8,[x0,#-8]; ldr x0,[x8,#0x1a0]`.

Natives (symbol, owned ELF): `EffectModule::req_follow` `0x2017730` (`...Hash40, Hash40, Vector3f const&, Vector3f const&, float, bool, unsigned, int, int, int, int, bool, bool`),
`kill_kind` `0x2017860`, `set_rgb_partial_last` `0x2017c80`, `set_rate_last` `0x2017ce0`, `MotionAnimcmdModule::set_sleep_effect` `0x2069140`,
`AttackModule::is_attack` `0x1fd01c0`, `attack_data` `0x1fd0520`, `GrabModule::set_rebound` `0x2027bb0`; the rows' 64 bytes are generated by
`tools/gen_hitbox_sites.py` and compared by `core06_rows_match_the_owned_elf`.

<a id="core-06--card-scan-playbook-card-scan-before-the-site-is-chosen"></a>

### Card scan (playbook "Card scan before the site is chosen")

| Dated card copy | Scan outcome |
| --- | --- |
| 16 plugins, 2026-11-01 | Only the fork referenced the three hook sites and `req_follow` |

- `python3 -I tools/build/scan-offsets.py <plugins> 2284b20 22ac290 2027bb0 1fd01c0 1fd0520 2017730 2017860 2017c80 2017ce0 2069140 227daf0 2284cd0 22ac2ec 2087720 3728e10 2017a50` over `/home/intpa/ssbu-cpu-pilot-install/switch-mtp-v0101-v021/pilot/after/atmosphere/contents/01006A800016E000/romfs/skyline/plugins` (16 plugins, folder dated 2026-11-01 10:19): only `libtraining_modpack.nro` (the fork, never installed with this plugin) references the three hook sites and `req_follow`; no other plugin references any of the 16 offsets.
- The `Called` rows tolerate another plugin's hook anywhere in their window; the hooks' rows are `Hook/Replace/Entry` like the CPU package's.
- Sharing v1 / chain: N/A (the scan showed nobody at the sites); a later plugin that hooks them is the case `GuardOutcome::HookedByOther` handles (the feature stays off and the install line names the row).

<a id="core-06--decisions-made-without-the-lead-reported-at-the-gate"></a>

### Decisions made without the lead (reported at the gate)

| Observed refusal | What it established |
| --- | --- |
| `req_follow` returned zero | Failure observed; pool exhaustion was not established |

<a id="core-06--lead-refusal-investigation-2026-11-02-17-2xz-not-a-fix"></a>

#### Lead refusal investigation, 2026-11-02 17:2xZ — not a fix

- Owned-native trace: the req_follow thunk dispatches through EffectModule slot 0x80.
- RELA for vtable0x4f6ccb8 maps it to0x44f880; is_exist slot0x188 maps to0x451800 (explicitly rejects handle0), and get_last_handle slot0x3a0 to0x4530a0.
- req_follow calls EffectManager creation0x3563430 at0x44fc30 and returns its zero result unchanged.
- That creation has several refusal paths: missing effect resource mappings, null free-list head at manager+0x19460, and failure of the lower constructor0x356cec0 after a free node was taken.
- Free count+0x19468 is decremented at0x35636e0.
- Thus hb04's zero return does **not** by itself establish pool exhaustion, and no spawn cap/spreading change is justified yet.

- Two new64-byte relied guards cover singleton resolution at0x44fbe4 and free-list layout at0x35636b8.
- All16 NROs in the saved2026-11-01 pilot/after card copy had no reference to either site (playbook scan-offsets.py,17:2xZ; physicalcard untouched).
- Diagnostic-only snapshots report free count/head before a request and after a refusal, limited to8failures perboot.
- No pool write or rendering behavior change.
- Host ELF assertions check these instructions; Cargo/Switch/Eden gates pending.

- Q4 (per-frame restore while Off): done as the spec recommends: one `set_sleep_effect(false)` on the first pass after the setting goes Off (a per-fighter
  record of the fighters put to sleep), no game call per frame while Off.
- Q2 (shield-damage rewrite in the `ATTACK` hook): not here. Package 2 applies the Infinite shield through the game's own parameters
  (`cpu_shield_param_hook`), so the hook does no Lua pop/push at all.
- Q1 / Q3 (projectiles drawn once; effect-pool cost): behaviour as specified (spheres at the command, per-frame pass for fighters only); the cost is measured
  in the diag build (`[Hitbox] stats`), see `eden/EDEN-HITBOX.md`.
- Hurtboxes: operator decision HB1 (a) (spec updated `dcbb2d1`): drawn yellow from `HitModule` (accessor `+0xb0`; vtable `main+0x4f6e1d0` by its relocations, shown by `set_status_all` / `set_status_joint` / `get_status` / `get_part_size` matching their thunk signatures).
  - Group table `+0x60` (0x3c bytes; first `+4`, count `+6`), part table `+0x70` (0x50 bytes; status `+0`, offsets `+0x08` / `+0x14`, size `+0x20`, node `+0x28`), part count `+0x78`; status 3 is off (the part update at `main+0x468490` skips it).
  - Rows `hurtbox_module`, `hurtbox_groups`, `hurtbox_part_shape` (relied).
  - Help text: "On: hitboxes, grab boxes and hurtboxes are drawn."

<a id="core-06--lead-ly-01-lower-constructor-attribution-2026-11-03-02-3xz"></a>

#### Lead ly-01 lower-constructor attribution, 2026-11-03 02:3xZ

- Actual r7 5f0b9c1 / diagnostic64b0be5b on Yoga, Steve/Ridley in Kongo Jungle, reproduced two first-pass refusals.
- Keys33559297/33559298, joint0x578749466, scale0.114; free count56->55->54 with a non-null head each time.
- First Ridley pass kept53/failed2, next reached55.
- This rejects free-list exhaustion for those calls, but does not identify the lower refusal branch.
- Full console SHA6c88c7b7ee75de3540cd0441e176506c2b34051647fb1803bf4c0bbe86330808 has137 Unmapped Read64; the run FAILS acceptance.
- No later combat rows accepted.
- Full26regularfiles and restore/sourceb hashes retained in artifacts/trainingrooms-lead-astra4/ly-01.

- Owned ELF0789f0a4 lower constructor356cec0 compares priority count5334a0c against limit5334a00 at356cfec..cff8, and normal count5334a14 against limit5334a04 at356d03c..d04c.
- These can return zero after a free entry was taken; final356db24..30 can also refuse when no payload was constructed.
- The new diagnostic samples ONLY the four raw signed limit/count words before/after each failed request, in [priority limit, priority count, normal limit, normal count] order.
- Two64-byte relied guards356cfe4/356d030 qualify the actual loads/branches.
- No native state write, spawn cap, retry spreading, capacity increase, or zero-refusal tolerance is introduced.
- New original owned-ELF assertions check both branches separately from the free-list guard.

Saved pilot/after16-NRO card inventory scan-offsets.py found no references to either new window (including fork binary; no fork implementation read, no physical card operation). Exact diagnostic native build/runtime still pending; raw disassembly retained tmp-lead-astra4/hitbox-manager-create.asm and hitbox-emitter-constructor.asm. The table/generator equality and11 instruction words were checked directly; this is not a Cargo execution claim.

<a id="package-1"></a>

<a id="package-1--test-provenance-package-1"></a>

## Test provenance, package 1

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Version guard matrix (display, build id, missing note) | §3, §9 | `core01_verdict_matrix`; carried `the_verdict_needs_both_the_version_and_the_build_id` | core01-E1 | host pass |

Package 1 test record opened 2026-11-01 (`39fd9e0f`). Each status records the evidence available at that stage.

- `Requirement | spec § | host test | Eden row planned | status`.
- Host tests run with `cargo test` (unit tests inside the model modules are also compiled into each by-path test binary, so the per-binary counts overlap; `cargo test` in total runs 7 + 8 + 6 + 17 lib unit tests plus the spec-named tests in `tests/`).
- "Eden row" labels are the specs' own; no Eden run has happened (package 1 is not Switch-compiled yet).
- Status: **host pass** = test exists and passes; **deferred** = the spec row belongs to a later package or needs the Switch runtime; **runtime, unbuilt** = code written, not compiled for the Switch.

<a id="package-1--spec-core-01-boot-install-sites"></a>

### SPEC-core-01 (boot, install, sites)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Version guard matrix (display, build id, missing note) | §3, §9 | `core01_verdict_matrix`; carried `the_verdict_needs_both_the_version_and_the_build_id` | core01-E1 | host pass |
| Build-id note finder | §3 | `core01_build_id_scan_tail`; carried `the_guard_finds_the_note_at_the_end_of_rodata`, `the_owned_1305_elf_carries_the_build_id_where_the_model_says` | - | host pass |
| Unsupported version installs nothing, logs one line | §2, §9 | `core01_unsupported_logs_and_returns` | core01-E2 | host pass; runtime unbuilt |
| Every site has a 64-byte guard; windows compared before any install | §4.2, §9 | `core01_site_table_guards_present`, `core01_overlapping_windows_checked_first`, `core01_site_offsets_named_by_hooks_are_table_rows`, `core01_sites_match_owned_elf` | core01-E3 | host pass (8 of the spec's ~40 sites: those of package 1) |
| 64-byte rule, chaining onto another plugin's hook | §4.2, §9 | `core01_chain_entry_classification`; carried `carried_guard.rs` (8 tests) | core01-E5 | host pass |
| Stale pair order | §9 | - | core01-E3 | deferred (site not in package 1) |
| Missing providers -> logged skip | §9 | - | core01-E4 | deferred (providers are used from package 2) |
| Welcome and chord notifications | §9 | - | core01-E6 | deferred (SPEC-input-05) |
| Data folder and paths | §7, §9 | `core01_paths` | core01-E7 | host pass |
| Install line format, `events.log` | §7, §9 | `core01_install_line_format` | core01-E1 | host pass; runtime unbuilt |
| Diagnostics ring admission and rotation | §7, §9 | `core01_diag_ring_admission`, `core01_diag_ring_rotation`, `runtime_diag_log::shipped_lines_enter_the_ring_and_others_do_not`; carried `carried_diag_ring.rs` (7 tests) | core01-E8 | host pass |
| Per-frame driver order, accessor none-paths | §5.2, §9 | - | core01-E9, E10 | deferred (packages 2-3) |
| No network symbols | §9 | `core01_no_network_symbols` (Cargo.lock) | core01-E11 | host pass |
| No heap in hooks (counting allocator, `heap_allocations=`) | §8 | `delay::ring_allocates_nothing` (queue); detector proven by `hook_alloc::the_detector_reports_a_stub_that_allocates_and_passes_one_that_does_not` (R5, below); `src/model/hook_stats.rs` | - | host pass; Eden: diag `hook_stats` lines, see the R1-fix section |
| `developer.ini` (decision N1), fail-closed (R4) | N1 | `core01_developer_ini`, `core01_developer_ini_fails_closed`, `core01_developer_ini_load_from_files`, `install_line::the_developer_gate_outcome_is_on_the_line` | - | host pass; Eden: `developer=off reason=…` on the install line, see the R1-fix section |

<a id="package-1--spec-core-04-settings-json"></a>

### SPEC-core-04 (settings.json)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Wire shape, real fork files | §3.1, §9 | `core04_wire_shape_fixture` (two real files) | core04-E1 | host pass |
| Factory round trip, defaults independent | §9 | `core04_roundtrip_factory`, `core04_defaults_independent` | core04-E2 | host pass |
| Unknown keys, retired keys | §3.2 | `core04_unknown_keys_ignored`, `core04_unknown_keys_kept_on_rewrite` | - | host pass (see note 1) |
| Missing keys take defaults; short/long arrays | §3.2 | `core04_missing_keys_default` | core04-E3 | host pass |
| Corrupt file: factory + field by field + copy name (decision S2) | §6 | `core04_corrupt_file_policy`, `core04_non_utf8_file_is_corrupt`, `core04_slider_rows`, `core01_paths` (copy names) | core04-E4 | host pass |
| Recovery is a boot-time storage transaction: untouched backup, sanitized complete settings (defaults, unknown keys) written to the active path before the original is retired; write failure leaves the original active (R1) | §6, S2 | `core04_recovery_two_boots_keep_the_recovered_settings_and_the_backup`, `core04_recovery_write_failure_leaves_the_original_active_and_logs_it`, `core04_recovery_without_a_free_backup_name_keeps_the_original_active`, `core04_recovery_of_a_file_that_is_not_json_keeps_it_and_writes_a_valid_one`, `core04_recovery_of_the_forks_old_file_name_retires_it_after_the_write`, `core04_a_clean_file_and_a_missing_file_are_left_alone`, `core04_an_interrupted_replace_is_restored_before_loading`, `core04_save_publishes_through_the_same_replace` (`tests/core04_recovery.rs`, real files in a temp dir, through `model::settings_file::load`, the code the runtime calls) | core04-E4 | host pass; Eden: two boots with a partially bad file, see the R1-fix section |
| Duplicate key | §3.2 | `core04_duplicate_key_error` | - | host pass |
| Fork file imports unchanged | §6 | `core04_import_upstream_file` | core04-E5 | host pass |
| Publish before write; writer fallback; no write on unplug | §5, §9 | `core04_publish_order`, `core04_writer_fallback`, `core04_no_write_on_unplug` | core04-E6..E8 | host pass (model); writer thread not in package 1 (nothing writes yet) |

<a id="package-1--spec-input-03-input-delay"></a>

### SPEC-input-03 (Input Delay)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Menu row, values, default 0 | §2.1 | `delay::menu_default_zero` | IN03-E01 | host pass (the row itself arrives with the menu) |
| Steady state, initial fill | §5 | `delay::steady_state_offsets`, `delay::initial_fill_neutral` | IN03-E02 | host pass |
| Change up / down | §5 | `delay::change_up_refills`, `delay::change_down_drops_oldest`, `delay::delay_zero_empties_the_queue_and_passes_through` (R3: the transition poll returns the current record), `delay::delay_zero_transition_with_press_and_release_records`, `delay::change_down_matches_the_observable_rule` | IN03-E03 | host pass |
| Clear on owner change; save-state load; reset (B8) | §5.1 | `delay::clear_on_input_owner_change` | IN03-E04 | host pass; the reset hub clears the queue (Eden `hub_run`); the menu, chord, replay and waiting-room owners arrive with their packages |
| The native Training menu is an input owner (R2; signal: the game's `L_staying_help` layout state, SPEC-core-03 §2.4): begin sets the bit and empties the queue even if polls stop, polls while paused are neutral, end releases, first *d* polls after resume neutral; driven through the real producer | §4, §5.1, question 3 | `input_pause::a_pause_empties_the_queue_owns_the_input_and_nothing_old_reaches_gameplay`, `input_pause::takeover_empties_the_queue_even_when_polls_stop_while_paused`, `input_pause::the_unwired_producer_lets_the_stale_and_menu_records_through`, `input_pause::other_owners_still_work_beside_the_pause_and_survive_its_release`, `input_pause::delay_zero_and_repeated_transitions_are_harmless`, `input_pause::the_ui_flag_edges_drive_the_owner_and_nothing_old_reaches_gameplay` (the UI-flag path through the real adapter) | IN03-E06 (diag pause lane) | host pass; Eden: see the R1-fix section |
| Chords not delayed; display and recorder see delayed input | §5.2 | `delay::ordering_with_chords_display_recorder` | IN03-E05 | host pass |
| Zero allocations | §8 | `delay::ring_allocates_nothing` | - | host pass |
| Hooks in the two callers of the final input mapping (note before each call, apply right after it returns; no hook in or on the function, whose inner points are the CPU pilot's) | §4 | `input_mapping_callers::the_sites_are_in_the_callers_and_not_near_the_pilot`, `input_mapping_callers::the_function_has_two_callers_and_the_registers_are_still_ours_at_the_sites`, `core01_sites_match_owned_elf` | IN03-E02 (run with and without the pilot NRO, both load orders) | host pass; Switch-built |

<a id="package-1--spec-state-06-l-r-a-reset"></a>

### SPEC-state-06 (L+R+A Reset)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Register rule | §4 | `state06_lra_register_rule` | E-state-06-lra-off-no-reset | host pass; runtime unbuilt |
| Hub trigger rules and the facts they rest on (entry id at fighter+0x10, manager slot, the two thunks) | §5.1 | `state06_hub_trigger_rules`, `state06_hub_trigger_facts_match_the_guarded_words` | E-state-06-reset-clears-mash | host pass; runtime unbuilt |
| Hub order and contents | §5.2 | `state06_hub_order` | - | host pass |
| Counter types | §5.2 | `state06_counter_types` | - | host pass |
| Load calls the hub every PosMove frame | §5.3 | - | E-state-06-load-resets-delays | deferred (package 3) |
| Hub idempotent | §6 | `state06_hub_idempotent` | - | host pass |

<a id="package-1--spec-state-07-stage-hazards"></a>

### SPEC-state-07 (Stage Hazards)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Flag address arithmetic | §5.1 | `state07_flag_address_arithmetic` | - | host pass |
| Scans find the pinned values on the owned `.text` | §5.1 | `state07_scan_matches_pinned` | - | host pass |
| Write rule | §5.3 | `state07_write_rule` | E-state-07-hazards-off-ps2 | host pass; runtime unbuilt |
| Missing address disables | §9 | `state07_missing_address_disables` | - | host pass |
| Change applies at next stage load | §5.4 | - | E-state-07-toggle-then-reload | Eden only |
| Waiting-room per-request policy | §6.3 | - | E-state-07-lobby-no-global-write | deferred (package 6; `policy.rs` not carried yet) |

<a id="package-1--spec-core-08-first-boot-import-pure-plan-and-draft-9-settings"></a>

### SPEC-core-08 (first-boot import, pure plan) and draft 9 settings

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Plan lists only the routed files; skips existing and excluded | §4, §9 | `migration::plan_copies_listed_files_only`, `migration::plan_skips_existing_and_excluded` | - | host pass |
| Only `migration.json` blocks a rerun (a pre-installed `developer.ini` does not) | §2, §9, lead rule 2026-11-02 | `migration::marker_blocks_rerun`, `migration::only_the_marker_means_already_imported` | CORE08-E02 | host pass |
| No source: marker with `none`; io error: partial; too large | §4, §9 | `migration::no_source_writes_none_marker`, `migration::io_error_is_partial`, `migration::too_large_skipped` | CORE08-E04, E05 | host pass |
| S1 items 1-2: settings renamed, INI files copied, old name accepted once | §9 | `migration::settings_renamed_on_copy`, `migration::ini_files_copied`, `migration::loader_accepts_old_name_once` | CORE08-E06 | host pass (see note 2) |
| N8: captures without cap, `.part` then rename, blocking/background split, marker last | §5, §9 | `migration::captures_have_no_size_cap`, `migration::captures_part_then_rename`, `migration::plan_splits_blocking_and_background`, `migration::marker_after_background_part`, `migration::captures_originals_untouched` | CORE08-E08, E09 | host pass (plan only) |
| N6: `character_item` retired and ignored on load | §4, core04 | `migration::character_item_ignored_after_import`, `core04_character_item_retired` | CORE08-E07 | host pass |
| N6: item-on-load conversion of save-state records | §4 | - | CORE08-E07 | deferred (needs SPEC-state-01/-04 records; package 3) |
| Receipt shape with sha256 per file; the copy loop, thread, `.part` writes | §4, §9 | - | CORE08-E01 | deferred (runtime, sha256 not written yet) |
| Draft 9 fields default when absent; per-field default on a bad field | core04 §3.1, §9 | `core04_draft9_fields_default`, `core04_per_field_default_on_bad_field` | core04-E9 | host pass (rows themselves arrive with their packages) |

Note 2: SPEC-core-08 §4 names `training_modpack.json` as the fork's settings file. It is the fork's
updater record; the settings are in `training_modpack_menu.conf` (CARRY-OVER.md, findings). The plan and
`storage::file::LEGACY_SETTINGS` use the real name; copying the updater file as `settings.json` would have
loaded factory settings over the player's own. Lead: please correct the spec.

- Note 3 (replaced by review R4, 2026-11-02): `developer.ini` compares the key and the value exactly (`developer_page`, lower-case `on`; lead, 2026-11-02), skips a leading UTF-8 BOM, and **failed closed** in R4.
- The loader reads at most 8 KiB + 1 byte and an over-limit file hides the page, a read error hides it, bytes that are not UTF-8 anywhere hide it, and a repeated `developer_page` key (in any order, any values) hides it.
- The earlier last-line-wins and prefix-truncation behaviour is gone.
- The install line says why: `developer=off reason=missing|unreadable|too_large|not_utf8|duplicate_key|key_absent|not_on`, or `developer=on reason=on`.

Note 1: the spec says unknown keys are dropped on rewrite. This build knows 3 of the ~90 rows, so dropping
them would erase the player's other settings until their packages land; they are kept verbatim instead
(`core04_unknown_keys_kept_on_rewrite`). Retired keys are dropped as the spec says. Lead decision requested.

<a id="package-1--menu-model-spec-core-02-spec-core-03-2-5-spec-core-04-last_page-spec-input-04-n7-tests-core02_menu-rs"></a>

### Menu model (SPEC-core-02, SPEC-core-03 §2.5, SPEC-core-04 `last_page`, SPEC-input-04 N7), `tests/core02_menu.rs`

| Requirement | § | Host test | Eden row planned | Status |
| --- | --- | --- | --- | --- |
| Factory table: every row of §2.7 with kind, option count, default; draft 9 rows; the two sliders; the seven delay ranges; `character_item` and `update_policy` not in the table | core02 §2.7, §9 | `core02_factory_table_matches_spec`, `every_menu_row_with_an_id_binds_to_its_row_or_is_an_editor_key` | core02-E5 | host pass |
| Layout: 5 pages + hidden Developer; rows per page 31 / 13 / 17 / 20 / 11 (+3); 92 rows without the pilot, 91 with it; 4 direction grids, 7 delay rows, 13 multi-selects, 2 sliders | MENU-DRAFT §2 counts | `layout_row_counts_match_the_draft`, `layout_the_pilot_swaps_the_fallback_rows_for_one_info_row` | core02-E5 | host pass |
| Two-line help text for every row, no stale chord text | core02 §2.6, Q10 | `layout_every_row_has_two_line_help` | core03-E7 | host pass |
| Advanced group collapsed by default, opened with A on its header | MENU-DRAFT §2 | `layout_advanced_group_is_collapsed_until_opened` | core02-E1 | host pass |
| List navigation wraps and skips non-focusable headers; grid cursor skips empty cells (adapted from the 8 x 4 grid) | core02 §2.3, §9 | `core02_table_nav_wrap_skip`, `core02_table_carriage_return` | core02-E1 | host pass (adapted) |
| Page machine: A opens, B closes the sub-screen first, B on the page closes; L / R round the ring | core02 §2.3, §9 | `core02_page_machine` | core02-E1 | host pass |
| Toggle flips; dropdown keeps exactly one option, Left / Right cycle in place with wrap, A opens the list on the current option (Q2); weight wraps at the largest; Y clears weighted only (adapted) | core02 §2.2-2.3, core03 §2.5 | `core02_toggle_flips_on_a`, `core02_toggle_single_exclusive`, `core02_toggle_wrap`, `core02_y_clears_weighted_only` | core02-E2 | host pass (adapted) |
| Clear-toggle hint rule | core02 §4, §9 | `core02_clear_hint_rule` | core02-E2 | host pass |
| Yes / No / Random over a weighted `[True, False]` row; number rows step and clamp | MENU-DRAFT §2 | `dropdown_yes_no_random_maps_to_the_weighted_row`, `number_row_steps_and_clamps` | core02-E2 | host pass |
| Multi-select: A ticks, ZL all / none; situation-override editor | MENU-DRAFT §2, Q4 | `multi_select_ticks_and_all_none`, `situation_overrides_editor_picks_a_situation_then_actions` | core02-E2 | host pass |
| Slider: states, steps (slow 1, fast 10), clamp; greyed unless Random | core02 §2.4, §9 | `core02_slider_states`, `core02_slider_clamp_steps`, `damage_range_is_greyed_unless_random` | core02-E3 | host pass |
| Greyed rows with the reason: Respawn on platform while Auto-load on death is On (N2); swap-group dependencies | N2, MENU-DRAFT §2 | `respawn_invincibility_is_greyed_while_auto_load_is_on`, `swap_group_entries_grey_with_the_reason` | core02-E2 | host pass (dependencies inferred, see progress file) |
| Direction grids: chip layout and navigation, weights, ZL all / none, nine-dot preview; Shield Tilt exactly one chip | Q12, MENU-DRAFT §1 | `direction_grid_navigation_follows_the_chip_layout`, `direction_grid_weights_all_none_and_preview`, `shield_tilt_grid_holds_exactly_one_chip` | core03-E7 | host pass |
| N3 delay rows: checklist or range exclusion, Y clears the active one, bounds and steps, round trip, file without a range, slot sets have no range, reset / save-defaults and bulk update treat them as one row | N3, core02 §2.8, §9 | `core02_delay_row_exclusion`, `core02_delay_row_y_clear`, `core02_delay_range_bounds`, `core02_delay_range_roundtrip`, `core02_delay_no_range_loads_checklist`, `core02_slot_sets_have_no_range`, `core02_delay_row_reset_and_save_defaults`, `core02_delay_row_bulk_update_clears_range`, `delay_range_pick_is_uniform_over_the_step_values`, `core03_delay_popup_render_plan` | core02-E8, E9, core03-E14 | host pass |
| X resets the page to the defaults snapshot (saved defaults replace the factory ones); serialization shape and round trip; bulk update clears before applying | core02 §2.5, §5, §9 | `core02_defaults_and_reset`, `core02_serialize_roundtrip`, `core02_update_from_shorter_array` | core02-E4 | host pass (see gap: no key saves defaults) |
| Up / Down hold repeat quickens; stick Left / Right and held jumps repeat steadily | operator 2026-11-22, menu lists | `model::adapter::tests::case_table` | core02-E6 superseded | host pass |
| N1 Developer page in the ring only while the flag is on; rows serialized, inert while hidden | N1, core02 §2.8 | `core02_hidden_page_ring`, `core02_hidden_page_rows_serialized` | core02-E7 | host pass |
| Q5 opens on the last page used, CPU on the first open ever, a saved page not in the ring opens CPU; `last_page` round trip | Q5, core02 §2.8, core04 §3.1 | `core02_first_open_is_cpu`, `core02_open_on_last_page`, `core02_last_page_missing_falls_back`, `core04_last_page_roundtrip` | core02-E10 | host pass |
| Re-roll events (Q18 A: SDI Direction, Shield Tilt; B11: Ledge Options, Throw options only; delay rows and DI do not) | Q18, B11 | `menu_change_to_cpu_choices_emits_reroll_events` | CPU package rows | host pass (model only; the CPU package consumes `Outbox::rerolls`) |
| N7 shortcut rows open the chord editor and show the chord text; permanent chord is a read-only info row; a confirmed change applies at once and emits the `button_config.ini` request, only then | N7, input04 §10 | `core03_shortcut_rows_open_editor`, `core03_permanent_shortcut_row_info`, `chords_shortcut_rows_open_editor`, `chords_permanent_chord_fixed`, `chords_shortcut_change_applies_immediately`, `chords_ini_written_only_on_confirmed_change`, `button_chords_editor_writes_the_chord_setting`, `chord_text_and_row_conversion` | core03-E13, IN04-E08 | host pass (the INI text and the write are package 4 / the runtime) |
| Replay capture toggle emits its INI request | Q7 | `replay_capture_toggle_writes_its_ini` | - | host pass |
| N5 paging by console mode: items docked 5 x 8 one page, handheld 4 x 4 pages 16 / 16 / 6; stages docked 3 pages 40 / 40 / 38, handheld 8 pages; every tile once; layout fixed at open | N5, core03 §2.5, §9 | `core03_item_grid_layout_docked`, `core03_item_grid_layout_handheld`, `core03_item_grid_layout_fixed_at_open`, `core03_item_grid_pages_cover_38_items_once`, `core03_stage_grid_layout_docked`, `core03_stage_grid_layout_handheld`, `core03_stage_grid_pages_docked_40_40_38`, `core03_stage_grid_pages_handheld_8`, `core03_stage_grid_pages_cover_118_tiles_once`, `core03_stage_grid_rotation_and_form_same_in_both_layouts`, `grid_screen_pages_with_l_r_and_keeps_the_cursor_on_a_tile` | core03-E12, E15 | host pass (rotation badges and the form tag are drawn per tile: package 5) |
| Dropdown lists read back and have unique labels | MENU-DRAFT §2 | `dropdown_options_are_unique_and_row_values_read_back` | - | host pass |

`src/model/menu/` (pure; no rendering, no hooks) and the factory table `src/model/factory.rs`. Rows that the
new page layout replaced (the 8 x 4 submenu grid, tabs, R-confirm reset) keep their spec test name where the
requirement survives in the new structure, adapted as noted. Names with the `chords::` prefix in SPEC-input-04
§10 are written `chords_...` here.

Not covered by the model (later packages): the fork's grid contents (portraits, hats, Mii lists, stage and
item actions), the named-macro and setup libraries, rendering of every screen, the INI text writer.
<a id="package-1--review-r1-fixes-2026-11-02-stream-r1fix-base-4895ea3-review-artifacts-trainingrooms-reviews-r1-astra-pkg1-review-md"></a>

### Review R1 fixes (2026-11-02, stream `r1fix`, base 4895ea3; review `artifacts/trainingrooms-reviews/R1-astra-pkg1/REVIEW.md`)

| Finding | Root cause | Fix | Regression |
| --- | --- | --- | --- |
| R1 settings recovery lost the recovered file | `keep_copy` renamed the active file away and nothing wrote the sanitized settings back | `model::settings_file::load`: write the sanitized complete settings to `settings.json.tmp`, write and read back the backup `settings.invalid-<n>.json`, only then replace the active file (`store::replace_file`); any failure leaves the original active and is logged | `tests/core04_recovery.rs` (two boots, write failure, no free backup name, non-JSON, old file name, orphaned `.bak`) |
| R2 native pause never owned the input | `published::INPUT_OWNERS` was never written. The scene's `StatePause` (first candidate) is not used by the Training menu: Eden probes r1-03 to r1-06 never fired at its begin routine, enter slots or exit | `InputAdapter::pause_flag` over `pause_begin/pause_end`, fed by the UI tick from `ui::state::PAUSE_ACTIVE` (the game's `L_staying_help` layout state, SPEC-core-03 §2.4; lead ruling); the `StatePause` hooks and sites were removed | `tests/input_pause.rs` (six tests, the UI-flag path through the real adapter), Eden pause lane |
| R3 delay 0 played a stale record | the oldest queued record was read before the queue was trimmed | `DelayQueue::step` handles 0 first (clear, return the current record); the decreasing-nonzero transition satisfies the spec's observable rule and is pinned | `tests/input_delay.rs` (three tests) |
| R4 developer gate accepted duplicates, oversize, read errors | last-line-wins parse of an 8 KiB prefix, `read_to_end` error ignored | `developer_ini::load` fails closed (above) | `tests/core01.rs` (three tests), install-line test |
| R5 no post-exercise allocation observation | the counter was read only at boot | `model::hook_stats` (allocator, per-hook counters), diag-only `hook_stats` line from the heartbeat thread, Eden gate `eden/hook_stats.py` | `tests/hook_alloc.rs` (allocating stub), `tests/eden/test_analyze.py` (gate) |

One commit per finding on `stream/r1fix`; `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo build --no-default-features` green after each.

The Eden rows (labels `r1-NN`) are in `eden/EDEN-PACKAGE-1.md`, section "R1-fix".

<a id="package-2"></a>

<a id="package-2--test-provenance-package-2-cpu-behaviour"></a>

## Test provenance, package 2 (CPU behaviour)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| The game's constant table is keyed by XXH64 (seed `u64::MAX`) of the identifier; Hash40 is CRC-32 and length | cpu-01 §3 | `cpu_ids_hash_matches_the_published_vectors`, `cpu_ids_hash40_is_the_crc_and_the_length` | `cpu-01-install-line` (constants missing = 0) | host pass; runtime, unrun |

Package 2 test record opened 2026-11-01 (`fa8b77d3`). Each status records the evidence available at that stage.

- `Requirement | spec § | host test | Eden row planned | status`.
- Host tests run with `cargo test` (`tests/cpu_core.rs`, `tests/cpu02.rs`, `tests/cpu03.rs`, `tests/cpu_sites.rs`; the model's unit tests are compiled into each by-path binary, so per-binary counts overlap).
- Eden row labels are the specs' own and are batched per boot in `eden/EDEN-PACKAGE-2-PLAN.md`; no Eden run has happened.
- Status: **host pass** = test exists and passes; **runtime, unrun** = the hook is written and Switch-compiled but not run on Eden; **blocked** = needs an input the implementer does not have (reason in the row); **later** = the spec row belongs to a later milestone of this package.

- Design facts the tests rest on (all in `src/model/cpu/`): one `Cpu` value (settings, the plugin's own generator, the constant table, the role table, one state per fighter body).
- Each hook reads the game first, takes the one lock for a pure decision, releases it and applies the answer.
- No status, term or work id is written down: they are asked from the game's constant table by name at run time (`hash.rs`, `ids.rs`).

<a id="package-2--shared-by-the-cpu-specs-spec-cpu-01-3-5-spec-cpu-07-5-decisions-b4-b5-n4-b6"></a>

### Shared by the CPU specs (SPEC-cpu-01 §3/§5, SPEC-cpu-07 §5, decisions B4, B5/N4, B6)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| The game's constant table is keyed by XXH64 (seed `u64::MAX`) of the identifier; Hash40 is CRC-32 and length | cpu-01 §3 | `cpu_ids_hash_matches_the_published_vectors`, `cpu_ids_hash40_is_the_crc_and_the_length` | `cpu-01-install-line` (constants missing = 0) | host pass; runtime, unrun |
| Constant names unique and hash apart; resolve by key; unresolved matches nothing | cpu-01 §3 | `cpu_ids_names_are_unique_and_hash_apart`, `cpu_ids_resolve_asks_by_hash_and_lists_the_missing` | same | host pass |
| Situation predicates (hitstun end-exclusive, tumble, footstool, shield, shieldstun, tech, trump, landing, clatter) | cpu-01 §3 | `cpu01_situation_predicates` | - | host pass (synthetic numbering; real values come from the game) |
| Weighted pick: weight 0 never, proportional, empty = none, no game RNG | cpu-01 §3 | `cpu01_weighted_pick_distribution` | - | host pass |
| Picks (weighted, delay checklist, delay range) never allocate | cpu-01 §8 | `cpu01_weighted_pick_no_alloc` (counting allocator) | - | host pass |
| Delay rows: empty = 0, checklist weighted, range uniform and inclusive, min = max fixed, a set range is a selection | N3 | `cpu_core_delay_rows_pick_checklists_and_ranges` | - | host pass |
| CPU settings read from the snapshot (factory values, a changed snapshot, ranges, tilt, override-playback default Off) | cpu-01..06 §2 | `cpu_settings_read_the_factory_rows`, `cpu_settings_read_a_changed_snapshot` | - | host pass |
| B4: CPU role only (entry 0 never, entry 1 in Training, entry 4 in the lobby) | cpu-01 Q7 | `cpu01_cpu_role_only_when_p1_ai` | `cpu-01-p1-ai-untouched` | host pass; runtime, unrun |
| Role table: accessor to body, replaced, forgotten, cleared | cpu-01 §6 | `cpu_core_role_table_finds_bodies_by_accessor` | - | host pass |
| B6: Popo and Nana each have their own state | cpu-01..06 | `cpu_core_bodies_keep_separate_state`, `cpu02_per_fighter_state`, `cpu03_per_fighter_state` | `cpu-02-ice-climbers-sdi`, `cpu-03-ice-climbers` | host pass; runtime, unrun |
| B5/N4: covered settings yield to a playing recording unless *Settings override playback* is On | cpu-02 §6 | `cpu_core_covered_settings_yield_to_playback_by_default` | - | host pass |
| `should_delay` semantics | cpu-07 §5 | `cpu07_should_delay` | - | host pass |
| Tick advances counting counters; reset clears them | cpu-07 §5 | `cpu07_tick_types_and_reset` | - | host pass (only in-game counters exist so far; the Real and no-reset types come with the menu and frame advantage) |
| One in-game tick per simulation frame with two bodies | cpu-07 §5 | `cpu07_single_tick_per_frame` | `cpu-07-ice-climbers-delay-exact` | host pass; runtime, unrun |
| Reset hub steps clear mash, SDI roll, counters, ledge delay, in hub order | state-06 §5.2 | `state06_hub_order_clears_cpu_state` | `E-state-06-reset-clears-mash` | host pass; runtime, unrun (combo, throw and shield-tilt steps come with their specs) |
| Menu re-roll events: SDI / shield tilt left pending for the next CPU frame, ledge case cleared at once (Q18 A, B11) | cpu-02 §5, cpu-03 §5 | `cpu_core_publish_consumes_the_menu_rerolls` | `cpu-02-sdi-change-no-reset`, `cpu-03-wait-then-change-to-roll` | host pass; runtime, unrun (the menu is package 5: `published::publish` takes the outbox bits) |
| A frame with nothing to do asks for nothing (staling flag once) | cpu-02 §8 | `cpu_core_frame_with_no_work_asks_for_nothing` | - | host pass |
| Mash queue: main then follow-up, completion pops, refuse when non-empty | cpu-01 §5 | `cpu01_queue_main_then_followup` | `cpu-01-followup-nair-then-jump` | host pass (the queue and its buffer entry; the per-frame trigger evaluation and command flags are P2-7) |
| Playback Mash Interrupt | cpu-01 §5 | `cpu01_playback_interrupt` | `cpu-01-playback-interrupt` | host pass (buffer rule only) |
| Lobby effective-action filter | cpu-01 §3 | `cpu01_lobby_effective_actions` | `cpu-01-lobby-mash` | host pass (the lobby scene is package 6) |
| Roll points at buffering (attack angle, aerial delay, full hop, fast fall, falling aerial) | cpu-06 §5 | `cpu06_roll_points` | - | host pass |

<a id="package-2--spec-cpu-02-di-sdi-air-dodge-direction-clatter-dodge-staling"></a>

### SPEC-cpu-02 (DI, SDI, air-dodge direction, clatter, dodge staling)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Direction to angle table, Neutral none, Left/Right absolute | §2 | `cpu02_direction_angles` | - | host pass |
| Mirroring when the player is to the CPU's right, never for Left/Right (B7: opponent-relative on both sides) | §2 | `cpu02_mirror_rule`, `cpu02_airdodge_opponent_relative_both_sides` | `cpu-02-di-out-both-sides`, `cpu-02-airdodge-out-left-and-right` | host pass; runtime, unrun |
| DI case held for the hitstun episode, cleared outside it; Neutral writes (0, 0); empty row writes nothing; the case clears in the frame dispatch | §5 | `cpu02_di_case_lifetime`, `cpu02_di_case_clears_in_the_frame_dispatch` | `cpu-02-di-multihit`, `cpu-02-di-neutral` | host pass; **blocked** for the runtime: the DI hook sits in the `common` script module and its 64-byte guard cannot be taken (see below) |
| SDI period 8 / 6 / 4, None never, input frame writes the vector | §5 | `cpu02_sdi_period` | `cpu-02-sdi-high-up` | host pass; runtime, unrun |
| SDI re-rolls on the reset bundle and on a row change, not per hitlag episode (Q18 A) | §5 | `cpu02_sdi_reroll`, `cpu02_sdi_rolls_on_row_change_and_reset`, `cpu02_sdi_roll_not_per_hitlag` | `cpu-02-sdi-two-directions`, `cpu-02-sdi-change-no-reset` | host pass; runtime, unrun |
| N9: SDI not set means no SDI (empty direction, Strength None, Neutral; the original is never called) | §5 | `cpu02_sdi_unset_means_no_sdi` | `cpu-02-sdi-unset-none` | host pass; runtime, unrun |
| Air-dodge direction: one pick per aerial hitstun episode, consistent x and y, only airborne in hitstun | §5 | `cpu02_airdodge_direction_consistency` | `cpu-02-airdodge-down-out` | host pass; runtime, unrun |
| Clatter period 8 / 5 / 2, step from the start call, effect on input frames | §5 | `cpu02_clatter_period_and_step` | `cpu-02-clatter-grab-high`, `cpu-02-clatter-freeze` | host pass; runtime, unrun |
| Dodge staling flag equals the negated row, written on the first frame and on change, repeated after a forget | §5 | `cpu02_stale_flag_follows_setting` | `cpu-02-dodge-staling-off` | host pass; runtime, unrun (open question 5: Eden must show the game does not clear the flag) |
| B5/N4: DI, SDI and air-dodge overrides skipped while a playback possesses the CPU; applied with *Settings override playback* On | §6 | `cpu02_playback_wins_default`, `cpu02_playback_override_on` | `cpu-02-di-during-playback-off`, `cpu-02-di-during-playback-on` | host pass; runtime, unrun (the recorder is package 4: `Cpu::playback` is set by it) |
| Common-module hook installs only after its guard matches the loaded image | §9 | - | `cpu-02-install-line` | **blocked**: no 13.0.5 `common` image bytes to take a guard from |

<a id="package-2--spec-cpu-03-ledge-options"></a>

### SPEC-cpu-03 (ledge options)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Case and delay rolled once per grab, cleared when leaving the ledge | §5 | `cpu03_case_lifetime` | `cpu-03-regrab-rerolls` | host pass; runtime, unrun |
| Status per option (climb, escape, jump1, attack) | §4 | `cpu03_option_status_table` | `cpu-03-each-getup` | host pass; runtime, unrun |
| Zero-delay buffer frames 19 regular, 18 tether, 13 playback | §5 | `cpu03_buffer_frames` | `cpu-03-zero-delay-first-frame`, `cpu-03-tether-first-frame` | host pass; runtime, unrun (the frame numbers are the spec's tuned values; Eden confirms, open question 2) |
| Delay counted only from actionability; Wait holds; climb denied while waiting or delaying | §4, §5 | `cpu03_delay_and_climb_denial` | `cpu-03-delay-60`, `cpu-03-wait` | host pass; runtime, unrun |
| Playback option: early buffer needs Ledge Delay non-empty; normal path otherwise; climb denied until the playback starts | §5 | `cpu03_playback_paths` | `cpu-03-playback-slot1` | host pass; runtime, unrun (the recorder is package 4; the request is carried in `FrameOut::playback` and not yet delivered) |
| Post-getup mash: override per kind, else trigger, else none | §5 | `cpu03_post_getup_mash` | `cpu-03-roll-then-fsmash` | host pass; runtime, unrun |
| Empty effective set: native behaviour, climb not denied | §5 | `cpu03_empty_effective_set` | `cpu-03-lobby-playback-only` | host pass (the lobby scene is package 6) |
| N3: a set range is picked uniformly once per grab; a set range counts as a non-empty row | §5 | `cpu03_ledge_delay_range_pick`, `cpu03_range_counts_as_nonempty_row` | `cpu-03-delay-range-30-60` | host pass; runtime, unrun |
| N3: checklist and range exclusive, legacy file loads as a checklist | §9 | covered by `core02_delay_row_exclusion` (package 1 menu tests) and `cpu_settings_read_a_changed_snapshot` | - | host pass |
| B11: changing Ledge Options clears the held case and re-rolls in the same hang; emptying the row stops the feature; the held delay is unchanged | §5 | `cpu03_ledge_options_change_rerolls_held_case`, `cpu03_ledge_options_emptied_clears_case` | `cpu-03-wait-then-change-to-roll`, `cpu-03-change-to-empty` | host pass; runtime, unrun |
| Yields during input playback; reset bundle resets the delay and its counter | §5 | `cpu03_playback_yield_and_reset` | `cpu-03-savestate-on-ledge` | host pass; runtime, unrun |

<a id="package-2--spec-cpu-04-tech-missed-tech-hide-tech"></a>

### SPEC-cpu-04 (tech, missed tech, hide tech)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Ground tech rewrite per option; No Tech leaves DOWN; no tech without the PASSIVE term and the script's check; Ganondorf's aerial slam not techable; no roll unless a tech situation applies | §5 | `cpu04_ground_tech_table` | `cpu-04-tech-each-option`, `cpu-04-ganon-side-b` | host pass; **blocked** for the runtime: the script status-change hook is in the `common` module |
| Wall and ceiling tech rewrites | §5 | `cpu04_wall_ceiling_tech` | `cpu-04-wall-tech-jump`, `cpu-04-ceiling-tech` | host pass; blocked (same hook) |
| Motion substitution follows the held roll directions; tech and getup animations by their own rows | §5 | `cpu04_motion_substitution` | `cpu-04-roll-back-animation` | host pass; runtime, unrun (`cpu_motion_change_hook`) |
| Mistech per option from down-wait (both kinds) and slip-wait | §5 | `cpu04_mistech_table` | `cpu-04-mistech-each-option`, `cpu-04-banana-slip` | host pass; runtime, unrun (per-frame, `change_status_force`) |
| Mistech no longer depends on Tech Options (B8, question 1) | §10 | `cpu04_mistech_gate_is_its_own_row` | - | host pass |
| Lay-down lock-out formula and its counter; other statuses reset it | §5 | `cpu04_laydown_lockout` | `cpu-04-snake-dthrow` | host pass; runtime, unrun |
| Tech and Mistech triggers buffer the override or the mash | §5 | `cpu04_tech_triggers_mash` | `cpu-04-tech-then-usmash` | host pass; runtime, unrun for mistech, blocked for tech |
| Hide from frame 6, show in the last 5, cursor 800 then restored; restore on status end, toggle Off mid-tech and session end | §5 | `cpu04_hide_tech_lifecycle`, `cpu04_hide_tech_toggle_off_mid_tech` | `cpu-04-hide-tech`, `cpu-04-hide-tech-toggle-off-mid-tech` | host pass; runtime, unrun |
| Quake suppressed only in DOWN with hide On | §4 | `cpu04_quake_rule` | `cpu-04-mistech-no-shake` | host pass; runtime, unrun (`cpu_quake_pos_hook`) |
| Camera centre per stage class | §5 | `cpu04_camera_table` | `cpu-04-fixed-camera-bf`, `cpu-04-fixed-camera-smashville` | host pass for the table; **blocked** for the runtime: the stage-id to class mapping needs the game's `StageID` numbers, which no spec or owned source gives, and the camera-manager needle sites are not in this table |
| Primary body only; Nana visible; the record dies with the session | §6 | `cpu04_primary_body_only` | `cpu-04-ice-climbers-hide` | host pass; runtime, unrun |
| B6 per-fighter held roll directions | §10 | `cpu04_per_fighter_state` | - | host pass |
| B5/N4 recording wins by default; override On applies | §6 | `cpu04_playback_wins_default`, `cpu04_playback_override_on` | `cpu-04-tech-during-playback-off/on` | host pass |
| Common-module hook guard | §9 | - | `cpu-04-install-line` | **blocked** (no `common` image) |

<a id="package-2--spec-cpu-05-shield-out-of-shield-oos-offset-reaction-time-tilt"></a>

### SPEC-cpu-05 (shield, out-of-shield, OoS offset, reaction time, tilt)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Hold predicate (playback, standby, loading, was-airborne, queue request, toggles), guard-button answers | §5 | `cpu05_should_hold_shield` | `cpu-05-hold-shield-stands` | host pass; runtime, unrun (`cpu_check_button_on/off_hook`) |
| Decay override per toggle; Hold flips after the first hit and back when not guarding; None native | §5 | `cpu05_decay_override_rules` | `cpu-05-infinite-no-decay`, `cpu-05-hold-decays-after-hit`, `cpu-05-none-native-decay` | host pass; runtime, unrun (`cpu_shield_param_hook`); the flip on leaving shieldstun needs the `common` script hook (blocked), so Hold never flips in the runtime |
| Damage multiplier 0 for Infinite, restored otherwise and outside Training, cached across session clears | §4 | `cpu05_damage_mul_restore` | `cpu-05-leave-training-restores-mul` | host pass; **blocked** for the runtime: it needs the param-hook plugin's pointer (SPEC-cpu-05 question 6, the lead decides) |
| OoS offset: N hits shielded then act; reset when idle or in hitstun | §5 | `cpu05_oos_offset_count` | `cpu-05-oos-offset-2` | host pass; blocked (out-of-shield hook) |
| Reaction time counted after shieldstun; parry blocks; not shielding resets | §5 | `cpu05_reaction_time` | `cpu-05-reaction-10`, `cpu-05-parry-no-oos` | host pass; blocked |
| Each OoS action: dodge or roll by status, jump, aerial, up special and up smash by native release, grab without drop, the rest with suspension | §5 | `cpu05_oos_action_table` | `cpu-05-oos-each-action` | host pass; blocked |
| Shieldstun override vs trigger | §5 | `cpu05_shieldstun_override` | `cpu-05-shieldstun-override-nair` | host pass; blocked |
| Shield tilt; a changed tilt applies at the next CPU frame and on the reset (Q18 A); In / Out opponent-relative (B7) | §5 | `cpu05_shield_tilt`, `cpu05_shield_tilt_applies_next_frame`, `cpu05_shield_tilt_opponent_relative` | `cpu-05-tilt-up`, `cpu-05-tilt-change-no-reset`, `cpu-05-tilt-in-left-and-right` | host pass; runtime, unrun (`cpu_stick_x/y_no_clamp_hook`) |
| N3: OoS Offset and Reaction Time ranges | §9 | `cpu05_oos_offset_range_pick`, `cpu05_reaction_time_range_pick` | `cpu-05-oos-range-1-3`, `cpu-05-reaction-range-5-15` | host pass |
| B6 per-fighter state | §10 | `cpu05_per_fighter_state` | `cpu-05-ice-climbers` | host pass |
| Common-module hook guard; param-hook absent path logs once | §9 | - | `cpu-05-install-line` | **blocked** |

<a id="package-2--spec-cpu-01-the-mash-run-spec-cpu-06-aerials-throws-angle-fast-fall-full-hop-crouch-spec-cpu-07-frame-advantage"></a>

### SPEC-cpu-01 (the mash run), SPEC-cpu-06 (aerials, throws, angle, fast fall, full hop, crouch), SPEC-cpu-07 (frame advantage)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Trigger priority tech > clatter > tumble > hitstun > parry > footstool > trump > landing > generic | cpu-01 §5 | `cpu01_trigger_priority` | `cpu-01-tumble-vs-hitstun` | host pass; runtime, unrun (`cpu_command_flag_hook`) |
| Override replaces mash_state without its trigger; an empty override falls back | cpu-01 §5 | `cpu01_override_precedence` | `cpu-01-hitstun-override-only` | host pass; runtime, unrun |
| Distance thresholds 16 / 37 / 64, infinity without a player (3-D distance from x, y, z) | cpu-01 §3 | `cpu01_distance_thresholds` | `cpu-01-distance-mid` | host pass; runtime, unrun |
| One roll per situation entry (B8), a new one at the next entry | cpu-01 Q1 | `cpu01_one_roll_per_situation` | `cpu-01-tumble-vs-hitstun` | host pass; runtime, unrun |
| Each action's command bits and expected status; jab only grounded; grab blocked by the invalid-catch frame; shield airborne returns 0 and stays queued | cpu-01 §5 | `cpu01_action_flags_table` | `cpu-01-mash-each-action` | host pass; runtime, unrun |
| Dash attack: dash first, one dash frame, then the dash attack; no re-buffer while beginning it | cpu-01 §5 | `cpu01_dash_attack_sequence` | `cpu-01-dash-attack` | host pass; runtime, unrun |
| Character-specific completion (Bowser, Kirby range; Sora and K. Rool rows are in the table) | cpu-01 §4 | `cpu01_character_specific_completion` | `cpu-01-kirby-hat-neutral-b`, `cpu-01-bowser-up-b` | host pass; runtime, unrun |
| A menu save that empties Mash Toggles and Triggers empties the queue once | cpu-01 §5 | `cpu01_menu_disable_clears_queue` | `cpu-01-menu-disable` | host pass; runtime, unrun |
| Reset bundle empties the queue and latch; no choice while a save state loads | cpu-01 §5 | `cpu01_reset_clears_queue` | `cpu-01-savestate-reset` | host pass |
| Nana receives bits; an unresolved table asks for nothing | cpu-01 §6 | `cpu01_nana_and_missing_accessor` | `cpu-01-ice-climbers` | host pass |
| Playback Mash Interrupt stops the playback and triggers again after it | cpu-01 §5 | `cpu01_playback_stops_on_a_triggered_mash`, `cpu01_playback_interrupt` | `cpu-01-playback-interrupt` | host pass; the request goes to `published::CPU_STOP_PLAYBACK` for package 4 |
| Aerial delay counts only when CONT_ATTACK_AIR is enabled; 0 and in-ATTACK_AIR wait for nothing | cpu-06 §5 | `cpu06_aerial_delay_rules` | `cpu-06-aerial-delay-10` | host pass; runtime, unrun |
| Falling aerial waits for negative gravity speed | cpu-06 §5 | `cpu06_falling_aerial` | `cpu-06-falling-nair` | host pass; runtime, unrun |
| Full hop holds the jump buttons during JUMP_SQUAT only, not during playback | cpu-06 §5 | `cpu06_full_hop_hold` | `cpu-06-full-hop`, `cpu-06-short-hop` | host pass; runtime, unrun |
| Fast fall: delay rolled at the apex, flag and flash once, only in FALL / ATTACK_AIR, not during playback | cpu-06 §5 | `cpu06_fast_fall_rules` | `cpu-06-fast-fall-delay-5` | host pass; runtime, unrun |
| Attack angle: up and down, not on the ledge, not in playback, cleared when the queue empties (B8) | cpu-06 §5 | `cpu06_attack_angle` | `cpu-06-ftilt-up` | host pass; runtime, unrun (`cpu_stick_dir_hook`) |
| Crouch in the six statuses only | cpu-06 §5 | `cpu06_crouch_statuses` | `cpu-06-crouch` | host pass; runtime, unrun (`cpu_stick_y_hook`; the air-dodge direction answers first) |
| Throw: case and delays once per grab; None does nothing; pummel timing; throw bits on category 2; the mash after the throw ignores the triggers | cpu-06 §5 | `cpu06_throw_state_machine` | `cpu-06-uthrow-delay-30-pummel-10`, `cpu-06-throw-none` | host pass; runtime, unrun |
| B11: a changed Throw Options row re-rolls the held case in the same hold | cpu-06 Q9 | `cpu06_throw_options_change_rerolls_held_case` | `cpu-06-throw-change-while-holding` | host pass; runtime, unrun |
| B5 / N4: throws yield to a recording by default | cpu-06 §6 | `cpu06_throw_yields_to_playback` | `cpu-06-throw-during-playback-off/on` | host pass |
| N3 ranges for the four delay rows | cpu-06 §9 | `cpu06_range_picks_and_exclusive_rows` | `cpu-06-aerial-range-5-15`, `cpu-06-throw-range-20-40`, `cpu-06-pummel-range-on` | host pass |
| B6 per-fighter state | cpu-06 Q5 | `cpu06_per_fighter_state` | `cpu-06-ice-climbers-grab` | host pass |
| Frame advantage sign, lock-in, one measurement per exchange, colours and text | cpu-07 §5 | `cpu07_frame_advantage_math` | `cpu-07-jab-on-shield-minus`, `cpu-07-plus-frames` | host pass; runtime, unrun (published to `published::FRAME_ADVANTAGE_*` for the UI package) |
| No work when Off; aborted by a mash set | cpu-07 §5 | `cpu07_frame_advantage_gates` | `cpu-07-off-no-notification` | host pass |
| The reset bundle and a session clear clear the measurement | cpu-07 §5 | `cpu07_reset_bundle_clears_the_measurement` | `cpu-07-lra-reset` | host pass |

**Spec budget row (cpu-07 §8).** Counters are per fighter body (decision B6, accepted 2026-11-02): 7 counters x 16 bodies, about 450 bytes of
state against the spec's roughly 128 bytes for one shared table; the clock still ticks once per simulation frame.

<a id="package-2--sites-spec-cpu-01-05-4"></a>

### Sites (SPEC-cpu-01..05 §4)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Every row is the symbol it claims (31 lua_bind functions and the constant lookup, checked against the ELF's symbol table); 64 guard bytes equal the owned 13.0.5 `main` (`core01_sites_match_owned_elf`) | §4 | `cpu_sites_are_the_symbols_they_name`, `core01_sites_match_owned_elf` | `cpu-0N-install-line` | host pass |
| Every hooked thunk is a 16-aligned leaf thunk that fits Skyline's 16-byte far jump without touching the next function | playbook | `cpu_sites_every_replace_hook_is_a_thunk_that_fits_the_far_jump` | `cpu-0N-install-line` | host pass |
| No site or guard window within 0x40 of a site of the CPU pilot; only our own rows overlap each other | playbook | `cpu_sites_are_not_near_the_cpu_pilot`, `cpu_sites_overlapping_windows_are_all_our_own` | pilot coexistence rows | host pass |
| Features need the core's natives; the slot row records its original | §4 | `cpu_sites_features_need_the_core`, `cpu_sites_the_frame_slot_holds_the_original_it_replaces` | - | host pass |
| Card scan (2026-11-01 10:19 MTP preflight copy, 16 plugins): only `libtraining_modpack.nro` (the fork, never co-installed, B1) references any of the 19 offsets; `libssbu_cpu_pilot.nro` and every other plugin: no reference | playbook | `tools/build/scan-offsets.py` (output in `IMPLEMENTATION-PROGRESS.md`) (removed after v0.2.0-rc.1; see that tag) | - | pass |

<a id="package-2--not-in-this-milestone"></a>

### Not in this milestone

| Package-2 milestone | Status at handoff |
| --- | --- |
| Fixed camera stage table | Open |
| Shield damage multiplier | Open |

The original handoff is under “Still open” in `IMPLEMENTATION-PROGRESS-pkg2.md` at `v0.2.0-rc.1`.
The November 2 update records the later results.

<a id="package-2--update-2026-11-02-successor-worker-eden-runs-s2-01-s2-04-on-the-ally-the-two-open-items-closed"></a>

### Update 2026-11-02 (successor worker): Eden runs s2-01..s2-04 on the Ally, the two open items closed

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Shield damage multiplier 0 for Infinite, restored otherwise, no provider plugin | cpu-05 §4, question 6 | `cpu05_damage_mul_restore`, `cpu_sites_common_params_fill_stores_shield_damage_mul_at_the_field_offset` | `cpu-05-leave-training-restores-mul` | host pass; **Eden pass** for Infinite and back to None (`write=0.000 was=1.190`, `write=1.190 was=0.000`); leaving Training not run |
| Fixed camera centre per stage class, stage ids from the game's table | cpu-04 §5 | `cpu04_camera_table`, `cpu04_stage_class_follows_the_native_ids_and_the_record_form`, `cpu04_camera_setup_and_mode_apply_the_stage_value_and_restore_the_default`, `cpu_sites_stage_table_records_are_the_stages_the_camera_rule_names`, `cpu_sites_fixed_camera_setup_loads_the_manager_through_the_slot_and_stores_the_centre_at_the_field` | `cpu-04-fixed-camera-bf`, `cpu-04-fixed-camera-smashville` | host pass; Eden: setup write issued for Final Destination (`write=Some([0,50,230])`, Off: `None`); the on-screen effect, camera mode 4 and Smashville not run |
| Common-module hook guards and install | cpu-02/04/05 §9 | `cpu_common_guards_match_the_owned_module` | `cpu-0N-install-line` | **Eden pass**: `cpu_common_load module=common … cpu_common=installed` (loader name `common`) |
| DI | cpu-02 §4 | `cpu02_di_case_lifetime` | `cpu-02-di-out-both-sides` | **Eden pass** for one side (`di_before role=true`, `di x=1.0 y=0.0`, `DAMAGE_FLY` hits only) |
| SDI, air-dodge direction, dodge staling | cpu-02 §5 | `cpu02_sdi_period`, `cpu02_airdodge_direction_consistency`, `cpu02_stale_flag_follows_setting` | `cpu-02-sdi-high-up`, `cpu-02-airdodge-down-out`, `cpu-02-dodge-staling-off` | **Eden pass** |
| Menu re-roll bits on a whole-settings replacement | cpu-02/03 §5, B11, Q18 A | `cpu03_whole_settings_replacement_yields_the_menus_reroll_bits` | `cpu-02-sdi-change-no-reset`, `cpu-05-tilt-change-no-reset` | **Eden pass** for the reload (`rerolls=0x3` and others); the ledge B11 row not run |
| Ledge Roll with a delay | cpu-03 §4, §5 | `cpu03_delay_and_climb_denial` | `cpu-03-delay-60` | **Eden pass** (one grab: climb denied, then `CLIFF_ESCAPE` requested) |
| Mash, each action | cpu-01 §5 | `cpu01_action_flags_table` | `cpu-01-mash-each-action` | **Eden pass** for 16 of 18 actions |
| Playback request word | cpu-03 §5 | `cpu03_playback_request_word_round_trips_for_every_slot` | - | host pass |
| Install line fits its buffer | core-01 §7 | `the_install_line_of_every_site_and_feature_fits_its_buffer_with_room_to_spare` | `cpu-0N-install-line` | host pass; found by Eden (line cut at 2048 bytes) |

Evidence and raw lines: `eden/EDEN-PACKAGE-2.md`. Rows whose status changed:

<a id="package-3"></a>

<a id="package-3--test-provenance-package-3-save-states-charges-buffs-items-fighter-specific-restore-first-boot-import"></a>

## Test provenance, package 3 (save states, charges, buffs, items, fighter-specific restore, first-boot import)

| Requirement | Host test | Eden row | Status |
| --- | --- | --- | --- |
| Chord passes on hold + press, not while a menu is open | not here: the chord engine is package 4's (`states::request_save/load` are its call surface; the diag build drives them from request files) | s3-01 (request files) | host n/a, Eden pass via request files |

Package 3 test record opened 2026-11-02 (`00d84b9e`). Each status records the evidence available at that stage.

- `Requirement | spec § | host test | Eden row | status`.
- Host tests run with `cargo test` (`tests/state01_json.rs`, `state01_machine.rs`, `state01_setup.rs`, `state02_charges.rs`, `state03_buffs.rs`, `state04_convert.rs`, `state04_items.rs`, `state05_restore.rs`, `state_sites.rs`, `migration_run.rs`; each by-path binary also runs the model's unit tests, so per-binary counts overlap).
- Eden labels `s3-NN` (results in `eden/EDEN-PACKAGE-3.md`).
- Status: **host pass**, **Eden pass**, **runtime, unrun** (written and Switch-compiled, no Eden row yet), **not built** (reported).

<a id="package-3--spec-state-01-save-load-core"></a>

### SPEC-state-01 save / load core

| Requirement | Host test | Eden row | Status |
| --- | --- | --- | --- |
| Chord passes on hold + press, not while a menu is open | not here: the chord engine is package 4's (`states::request_save/load` are its call surface; the diag build drives them from request files) | s3-01 (request files) | host n/a, Eden pass via request files |
| Save captures x, y, lr, percent, situation, kind into Save State Slot only | `state01_save_captures_into_the_save_slot_only`, `state01_record_json_shape` | s3-01: Save -> `save_states.json` slot 1 written (x -45, y 27.085, kind 88) | Eden pass |
| Load request marks both records, stops playback | `state01_load_request_marks_both` | s3-01: `load_request`, `kill` both | Eden pass |
| Kill -> WaitForAlive -> PosMove -> NoAction, grounded | `state01_sequence_ground` | s3-01: P1 walked to x -30.1, load back to -45.0 | Eden pass |
| Airborne state ends in FALL; ledge state: per-fighter raise (+6/+4/+2 groups) | `state01_sequence_air`, `state01_a_cliff_state_ends_in_fall_or_a_ledge_status_and_raises_by_kind` | not run (ledge row open) | host pass; Eden open |
| Mirroring None / Alternate / Random, stage offsets | `state01_mirror_sign_rules`, `state01_stage_offset_table` | s3-01: Alternate over three loads: x 45 / -45 / 45, lr -1 / 1 / -1 | Eden pass (None, Alternate) |
| Damage Default / Save State / Random within (Lower, Upper) per role | `state01_damage_modes` | s3-01: Random 50-50 gave 50.0 on both, Default 0.0, Save State 10.0 (fixture) | Eden pass |
| Random slot drawn once per request, latched | `state01_random_slot_latch` | - | host pass |
| Auto-load on death on / off | `state01_autoload_on_death` | s3-01: walked off stage, `load_request`, both placed | Eden pass |
| Nana hand-off, Nana never saves | `state01_nana_handoff` | - (needs Ice Climbers) | host pass |
| Suppression predicates, param overrides | `state01_suppression_predicates`, `state01_param_overrides` | s3-01 hook counts, no quake / star KO | Eden pass (counts) |
| N2 respawn invincibility Off = fork, On = overrides only while loading | `state01_respawn_on_overrides_only_while_loading`, `state01_respawn_row_greyed_with_autoload_and_load_unaffected_by_the_toggle`, `state01_respawn_off_is_fork` via `state01_param_overrides` | s3-01: Off: respawn drops from above, no platform; On: P1 on the respawn platform (`runs/s3-01/shots/respawn-sheet.png`) | Eden pass |
| Load request ignored mid-load, Enable Off cancels | `state01_ignore_request_mid_load`, `state01_disable_cancels` | - | host pass |
| Recording trigger / playback once; `on_state_restored` once | `state01_playback_once` | s3-01: `recorder_restored` once per load (adapter logs only until pkg4) | host pass, Eden pass (adapter line) |
| File written after both saves, lazy/boot read, marker reset, missing/malformed file | `state01_file_roundtrip_marker_reset`, `state01_file_missing_defaults_and_unusable_files_report_why`, `state01_two_boots_read_what_the_first_wrote`, `state01_a_corrupt_file_is_copied_not_moved_and_survives_every_boot`, `state01_a_failed_write_leaves_the_previous_file_and_the_next_load_reads_it`, `state01_an_oversize_or_unreadable_file_is_left_alone` | s3-01: the fixture file loaded at boot (slot 3 Steve), the Save wrote slot 1 | Eden pass |
| The fork's real files load and rewrite byte for byte | `state01_ported_files_load_and_rewrite_as_the_fork_wrote_them`, `state01_legacy_shaped_json_loads_with_defaults`, `state01_damaged_fields_do_not_discard_adjacent_slots` | - | host pass |
| Setup library file format (carried) | `tests/state01_setup.rs` (8) | - | host pass; the UI glue is not built (named-macro draft, lobby practice, Mii roster are packages 4 and 6) |
| Sites: every row is the ELF's bytes, thunks fit the far jump, no pilot site nearby | `tests/state_sites.rs` | s3-01/s3-02 install line | host pass, Eden pass |

<a id="package-3--spec-state-02-charges-spec-state-03-buffs"></a>

### SPEC-state-02 charges, SPEC-state-03 buffs

| Requirement | Host test | Eden row | Status |
| --- | --- | --- | --- |
| Read table per kind, write only present fields | `state02_read_table_per_kind`, `state02_write_only_present_and_matching_kind` | s3-02 (Samus) | see EDEN-PACKAGE-3 |
| Full-charge visuals, Sheik needles, G&W, Pac-Man, Robin, Incineroar, Hero | `state02_full_charge_visuals`, `state02_sheik_needle_groups`, `state02_gw_bucket_clear_flash`, `state02_pacman_key_flag`, `state02_robin_tomes`, `state02_incineroar_revenge`, `state02_hero_clear_then_set` | s3-02 (Samus glow) | Samus: see EDEN-PACKAGE-3; the others runtime, unrun |
| Olimar line, Sora spell | `charge::line_order`, `prev_colour`, `spawn_colour`, `sora_written_kind` (in the charge model, exercised by the state02 table) | - | runtime, unrun |
| Buff steps: Hero, Cloud/Joker/Mac, Sephiroth, Wario, Shulk, Wii Fit, Min Min, unselected, per-record bookkeeping | `tests/state03_buffs.rs` (12) | s3-04 (Cloud Limit, after the s3-03 crash fix) | Eden pass for Cloud (and Wii Fit, buffed as the CPU in s3-04); the other buff fighters are host pass, runtime unrun |

<a id="package-3--spec-state-04-items-spec-state-05-fighter-specific-restore"></a>

### SPEC-state-04 items, SPEC-state-05 fighter-specific restore

| Requirement | Host test | Eden row | Status |
| --- | --- | --- | --- |
| Item table for the 38 choices, sources, common items, snapshot precedence, timers rule, turnip face hooks, choice of a held item | `tests/state04_items.rs` (7) | s3-02 (Bob-omb snapshot) | see EDEN-PACKAGE-3 |
| N6 conversion of the retired `character_item` (all values, never overwrites, no entry leaves absent) | `tests/state04_convert.rs` (7), `migration_item_on_load_*` | s3-09, s3-10 (import copy; N6: 5 snapshots) | host pass; Eden pass |
| Steve materials and tools restored, bar refreshed | `steve` layout in the model (round trip) and the fixture | s3-01: slot 3 fixture, wood 77 / sword 11.5 etc. read back exactly | Eden pass |
| Keep-list, Pikmin autonomy gate | `Table::keeps_article`; the gate is a hook | - | runtime, unrun |
| Pokemon Trainer decide mapping (rotation, descriptor validation, non-Pokemon untouched) | `state05_pt_decide_mapping` | s3-05, s3-07 (PT row) (fixture `s3-pt-states.json`: saved Ivysaur, Squirtle out) | host pass; Eden pass (s3-05 to s3-07) |
| Trainer wait: 5 frames, rate 1.0 in RUN, airborne trainer grounded, per-role counter | `state05_pt_wait_sequence` | s3-05, s3-07 (PT row) | host pass; Eden pass (s3-05 to s3-07) |
| Pokemon dead test (switching frame) and switch effect / sound suppressions | `state05_pt_dead_test`, `state05_pt_suppressions` | s3-05, s3-07 (PT row) (no plate, no switch sound) | host pass; Eden pass (s3-05 to s3-07) |
| Pre-death rule (flag off, rates 1000, ball hidden) | `state05_pt_predeath` (in `state01_machine.rs`: the flag, the raise and the wait marker); the calls are runtime | s3-05, s3-07 (PT row) | host pass; Eden pass (s3-05 to s3-07) |
| Kirby acceptance rule (equal, Pokemon family, Pyra/Mythra) | `state05_kirby_accept_rule` | s3-06 (Kirby row) (`diag-copy.req`, then Save, Load) | host pass; Eden pass (s3-05 to s3-07) |
| New optional record field `copy_charge` (Kirby with a copy only; same shape as `charge`; the copied fighter's own HAD / pull-throw flag; written only when present, so fork-compatible files are byte-identical and the fork's loader ignores the key; lead ruling 2026-11-02, to be added to SPEC-state-05 and SPEC-state-01 §7): round trip, absent field, fork-written file, damaged value | `state05_copy_charge_round_trip_absent_and_fork_files`, the ported-files round trip (`state01_ported_files_load_and_rewrite_as_the_fork_wrote_them`), and the flag cases of `state05_kirby_charge_table` | s3-06 (saved file for a copy without a flag has no `copy_charge` key) | host pass; the key with a flag (Mewtwo, Lucario, Pac-Man copy) not Eden-run |
| Kirby copied charge table, copy flag and kind saved | `state05_kirby_charge_table` | s3-06 (Kirby row) (dump line: copy flag, chara, module kind) | host pass; Eden pass (s3-05 to s3-07) |
| Kirby copy sound and Sora jingle silences by status | `state05_sfx_status_rules` | s3-05, s3-06 (se_silenced, se_passed=0) | host pass; Eden pass for the trainer-switch and Kirby copy sounds; Sora's jingle not Eden-run |
| Kirby copy-setup with the opponent's real entry id; copy hook writes charge and ends the wait | runtime only (`states::kirby_copy`, `state_kirby_opff_hook`) | s3-06 (Kirby row) | Eden pass (s3-05 to s3-07) |

<a id="package-3--spec-core-08-first-boot-import-s3-5"></a>

### SPEC-core-08 first-boot import (S3-5)

| Requirement | Host test | Eden row | Status |
| --- | --- | --- | --- |
| Plan splits blocking and background; originals untouched; receipt shape and hashes | `migration_plan_splits_blocking_and_background`, `migration_full_import_receipt_and_originals`, `migration_receipt_hashes_the_converted_copy_separately` | s3-09, s3-10 | host pass; Eden pass |
| N6 on the copy only, never over a snapshot or an existing save | `migration_item_on_load_converts_player_records_in_the_copy`, `migration_item_on_load_never_overwrites_a_snapshot_or_an_existing_save` | s3-09, s3-10 | host pass; Eden pass |
| `developer.ini` does not mark imported, left byte-identical | `migration_developer_ini_does_not_mark_imported` | s3-09, s3-10 | host pass; Eden pass |
| Marker blocks a second boot; no source writes `none` | `migration_marker_blocks_a_second_boot`, `migration_no_source_writes_a_none_marker` | s3-09, s3-10 | host pass; Eden pass |
| Existing destinations never overwritten; size caps; captures uncapped | `migration_existing_destinations_are_never_overwritten`, `migration_captures_have_no_size_cap_and_the_rest_do` | s3-09, s3-10 | host pass; Eden pass |
| Staging folders skipped, never read, counted | `migration_staging_folders_are_skipped_and_never_touched` | s3-09, s3-10 | host pass; Eden pass |
| I/O error partial, unreadable source partial, interrupted background rerun, marker write failure reruns | `migration_io_error_is_partial_and_the_rest_is_copied`, `migration_unreadable_source_file_is_partial`, `migration_interrupted_background_copy_reruns_and_completes`, `migration_marker_write_failure_leaves_the_import_to_run_again` | s3-09, s3-10 | host pass; Eden pass |
| SHA-256 vectors | `sha256::tests` | - | host pass |

<a id="package-4"></a>

<a id="package-4--test-provenance-package-4-recording-input-display-chords-notifications"></a>

## Test provenance, package 4 (recording, input display, chords, notifications)

| Question | Decision | Where |
| --- | --- | --- |
| SPEC-input-01 question 2 (stick scale) | one scale, 1/120, clamped to +/-1; clamped fields drop values below 0.2 | `recording::inject_values`, `injected_stick_values` |

Package 4 test record opened 2026-11-01 (`b9d6497d`). Each status records the evidence available at that stage.

Stream `stream/pkg4` (Claude Sonnet 5.5 high under lead `0caa72b8`). Playbook followed: `009c9ac`. Host tests are in
`tests/input01.rs` (SPEC-input-01), `tests/input02.rs` (SPEC-input-02), `tests/input04.rs` (SPEC-input-04),
`tests/input05.rs` (SPEC-input-05) and `tests/input_sites.rs` (sites against the owned 13.0.5 ELF); the carried files
carry their own unit tests (`src/model/menu_shortcuts.rs`, `src/model/capture_settings.rs`).

<a id="package-4--lead-decisions-applied-2026-11-02"></a>

### Lead decisions applied (2026-11-02)

| Question | Decision | Where |
| --- | --- | --- |
| SPEC-input-01 question 2 (stick scale) | one scale, 1/120, clamped to +/-1; clamped fields drop values below 0.2 | `recording::inject_values`, `injected_stick_values` |
| SPEC-input-01 question 3 (transition-term ranges) | inclusive at both ends; mapped statuses use their term, and other starts wait until the fighter can act (`cpu::actionable`). The side-special term is not enabled for every fighter that can act: Greninja answers no while idle and after hitstun, Mario answers yes while idle (F8 trace 2026-11-23). | `Recorder::term_for_status`, `starting_status_to_term`, `other_status_has_no_specific_transition_term` |
| SPEC-input-01 question 4 (Recording Slot re-read every frame) | latch the slot and the duration at the lockout | `Recorder::start_recording` |
| End check and tint | run in the AI control hook, directly after the injection (same game frame as the on-frame wrapper) | `runtime/features/recorder.rs` |
| Replay capture default | the INI is the file of record; absent means off | `model::capture_settings`, menu `Env::default` |
| SPEC-input-02 question 1 (status names) | numeric ids until the names' source is decided (parked) | `input_display::StatusText` |

<a id="package-4--host-rows"></a>

### Host rows

| Spec row | Test | State |
| --- | --- | --- |
| IN01 menu defaults, rows read | `menu_defaults_match_spec`, `settings_rows_are_read`, `override_playback_default_off` | pass |
| IN01 lockout, standby, recording; P1 neutralised | `recorder_lockout_then_standby_then_cpu`, `standby_ends_on_non_neutral`, `starting_status_captured_frame_one` | pass |
| IN01 neutral rules, crop, bounds, empty slot | `neutral_frame_rules`, `crop_trims_trailing_neutral`, `frame_bounds_store_and_play_full_length`, `playback_refuses_empty_slot` | pass |
| IN01 chord pick, mirror, Shulk dial, stick values, R6 | `playback_slot_pick_and_lockout_refusal`, `mirror_multiplier_and_shulk_dial`, `injected_stick_values`, `control_write_touches_only_verified_fields` | pass |
| IN01 ledge buffer, hitstun timing, term table | `ledge_buffer_schedule`, `hitstun_timing_modes`, `starting_status_to_term` | pass |
| IN01 Playback Slot run, mash interrupt, stop, notices, import refusals | `a_playback_slot_plays_once`, `mash_interrupt_stops`, `session_clear_stops`, `phase_notifications`, `named_contract_export_import_refusals` | See F7b precheck |
| IN01 B4, N4 rows | N4 `override_playback_default_off`; B4's Record refusal was removed on 2026-11-25 (`input01::record_starts_while_an_ai_drives_p1`) | N4 host, B4 Eden (see EDEN-PACKAGE-4.md) |
| IN02 mode rules, binning, icons, fade, ring, counts | `difference_rules_smash_raw_status`, `bin_stick_values`, `icon_order_and_colours`, `frame_count_cap_99`, `frame_counts_count_in_game_frames_not_polls`, `fade_alpha_curve`, `ring_rotation_physical_index` | pass |
| IN02 renderer plan: hidden, refresh, registration, cache, budget, no heap | `hidden_under_menus`, `refresh_after_hide_and_reregister`, `unchanged_held_input_writes_nothing`, `rapid_changes_write_only_the_new_row_and_moves`, `bench_counts` | pass |
| IN04 rule, mapping, glyphs, latch, suppression, Plus | `hold_press_rule`, `single_button_semantics`, `button_mapping_by_style`, `glyph_table`, `request_latch_and_clear`, `suppressed_under_menus`, `plus_passthrough_and_suppression`, `detection_allocates_nothing` | pass |
| IN04 N7: editor rows, permanent chord, applies at once, INI written only on a confirmed change, fail-closed reads, write failure keeps the old file | `shortcut_rows_open_editor_and_permanent_chord_fixed`, `shortcut_change_applies_immediately`, `ini_written_only_on_confirmed_change`, `ini_reads_fail_closed`, `ini_write_failure_keeps_the_old_file`, `ini_missing_file_is_created_with_the_default_text`, `menu_shortcuts::tests::*` | pass (fs-backed, incl. a read-only folder) |
| IN05 queue, clear, hidden, first draw, no heap | `queue_order_and_lengths`, `clear_by_header_and_all`, `hidden_and_combo_display_off`, `first_draw_writes`, `tick_allocates_nothing` | pass |
| IN05 damage digits, pane rules, missing accessor | `damage_digits`, `pane_rules_fixture`, `missing_accessor_zero` | pass |
| Sites | `tests/input_sites.rs` (12 rows equal the ELF; the 6 natives are the symbols they name; the AI epilogue is the function's only return and no branch lands in it; the Combo Display pins are the spec's words; no window near a pilot site) | pass |
| Q7 replay capture INI | `capture_settings::toggle_tests::*` plus the carried tests | pass |

<a id="package-4--not-testable-on-the-host-or-not-built-in-this-stream-stated-not-implied"></a>

### Not testable on the host or not built in this stream (stated, not implied)

| Package-4 test boundary | Dependency at handoff |
| --- | --- |
| Pane writes | Package 5 layout and anchors |
| Mash-triggered playback | Package 2 trigger evaluation |
| Record-on-load | Package 3 restore completion |

- Pane writes of the input display and the notice slot, the Combo Display consumer and the damage helper's pane writes wait for
  package 5's layout and anchors (SPEC-core-05); the plan and the model are built and tested, `write_plan` / `write_panes`
  report to the diagnostics only.
- Mash-triggered playback and the mash interrupt need package 2's per-frame mash trigger evaluation (not built there); the
  recorder side (`mash_start`, `interrupt_on_mash`, the queue pop) is built and host-tested.
- Save-state playback and record-on-load need package 3's restore completion; the diag build's stand-in (the save-state chord) calls
  `on_state_restored`.
- S4-4 (named macros, replay import, record-slot export, clip editor, live replay) waits for `training_mod_mii` (package 6).

<a id="package-4--online-review-read-only-rules-hook-by-hook-spec-input-01-4-playbook-checklist-online"></a>

### Online review (read-only rules, hook by hook; SPEC-input-01 §4, playbook CHECKLIST "Online")

| Hook | Reads | Writes | Draws game RNG / skips native code |
| --- | --- | --- | --- |
| P1 poll note (`input_caller_*_note`, shared with Input Delay) | the player's `Controller` | Plus bits of that controller only for a modal owner or an accepted Plus shortcut (never in an online match: no Training) | no |
| P1 poll apply (`input_caller_*_apply`) | the mapped record | the record (delay, recorder neutralising) in Training only | no |
| `ai_epilogue` (`main+0x2dbc38`) | AI object, CPU entry, status, facing | the CPU entry's own controller fields (buttons, stick, clamped stick), the cliff rest timer, the model colour blend: Training only, CPU entry only | no (the plugin's own generator picks slots) |
| `combo_display_a/b` | w20 / w8 (the byte the game just loaded) | the plugin's own atomic | no |
| `stale_moves_toggle` | the menu object pointer | one byte at `PauseMenu+0xb60` in Training only | no |

The package's hooks act only when `is_training_mode()` is true and, for the recorder, only on the CPU entry's own controller; in an online match every hook
returns before it reads or writes anything beyond that predicate.

Replay A/B with the desync detector and the two-console online rows are not run (lead's to schedule); the plugin never writes simulated state outside Training.

<a id="package-4--checklist-rows-package-4"></a>

### Checklist rows (package 4)

| Requirement | Result | Evidence |
| --- | --- | --- |
| cdylib only, release profile, skyline =0.6.0, toolchain 1.98.1, `model`/`runtime` split | pass | `compliance.py` machine rows |
| Gates: `cargo test` (24 binaries, 1,227 tests, 0 failed, Surface), clippy `--all-targets -D warnings` (0 warnings, Yoga gate folder with `SKIP_OWNED_ELF_FACT=1` because the owned ELF lives on the Surface; the ELF tests ran on the Surface), `cargo build --no-default-features`, Switch release and diag builds warning-free | pass | commands in `IMPLEMENTATION-PROGRESS-pkg4.md` (removed after v0.2.0-rc.1; see that tag) |
| 64-byte guard on every site, card scan | pass | `tests/input_sites.rs`; scan over the 2026-11-01 10:19 preflight copy: only the fork NRO and the CPU pilot (`0x2da180`, the AI function's entry, not our site) reference our offsets; the Combo Display / stale-moves sites and the natives are referenced by the fork only |
| Hook placement (after the prologue / most specific instruction) | pass | `ai_epilogue` is the function's only return path, not its entry; the poll hooks are the caller-side sites of package 1 |
| Hook sharing with the CPU pilot (the pilot owns the AI function's entry) | pass | both load orders in Eden (s4-01, s4-02); no foreign hook at our rows (`stock`) |
| Shipped NRO writes only its install line | pass | `strings` of the release NRO: no `PracticeRecording` / `standin`; per-event lines and the writer thread exist only in the diag build; the INI is written only on a confirmed shortcut change and created once when absent |
| No heap in hooks | pass | `heap_allocations=0` at install and after 600-frame reports (s4-01, s4-02) |
| Eden: claim/release, one per host, profile copy, waits gated, console scan, run.json/memory.peak kept, profile restored | pass | `eden/EDEN-PACKAGE-4.md` |
| Workarounds | none shipped; open decision: status names (SPEC-input-02 question 1); the `CONT_SPECIAL_S` placeholder was superseded by F8 |
| Slip | seven builds on the Yoga beside pkg5's Eden (disclosed in `IMPLEMENTATION-PROGRESS-pkg4.md`) (removed after v0.2.0-rc.1; see that tag) |

<a id="package-4--s4-4-named-macros-replay-store-live-capture-adapter-replay-window-2026-11-02"></a>

### S4-4: named macros, replay store, live capture adapter, replay window (2026-11-02)

| Requirement | Host test | Eden row (label) |
| --- | --- | --- |
| Playback Slot frames and fixed-start mirroring | `tests/input01.rs`, `rr_host` slot case table | F7b-1–4 NOT FLOWN |
| The carried macros crate and panel models keep their tests (library, recording, window, move crop, capture archive, shared capture, macro panel, clip panel) | `cargo test --lib engine::macros engine::macro_ui` | - |
| The vendored match-data crate's own tests | `cargo test --manifest-path crates/smash-match-data/Cargo.toml` (22) | - |
| Carried files equal the manifest, ledger generated | `tests/carry_pkg4.rs` | - |
| Every game call of the layer is a guarded row | `tests/pkg6_smash_calls.rs`, `tests/input_sites.rs` (23 package-4 rows against the owned ELF) | install line `control_get_button=stock` ... |
| Stick direction bits of the controller record | `tests/input04.rs` `the_button_set_reads_the_dpad_and_both_sticks_digital_directions` | - |
| Typed menu rows fighter_prompt, macro_source | `compat::menu` test `the_macro_rows_follow_the_snapshot_and_default_to_ask_and_numbered_slots` | - |
| HUD plan diag line is coalesced | `model::input_display::plan_log_gate_tests` (3) | R5 delta in s4-04 |
| Live capture adapter installs only on guards and capture on | seam table tests (package 6) | s4-03: `guarded live adapter ready capture=on save=entry` |
| Replay window lists a capture and crops a macro | carried `ClipEditor` / `SharedCaptureArchive` tests | s4-04 (seeded fork capture) |

Online review (read-only rules): the layer runs only in Training and the practice session (`scene::is_training`, `macros::ready`); the live capture hooks observe the game's native replay recorder
(no write to simulated state; they do not draw or reorder checked randoms). Not yet reviewed in a two-console run (the hooks are installed only when `replay_capture.ini` says on).

<a id="package-5"></a>

<a id="package-5--test-provenance-package-5-menu-rendering-and-assets"></a>

## Test provenance, package 5 (menu rendering and assets)

| Requirement (spec) | Host test | Eden row | Result |
| --- | --- | --- | --- |
| UI site rows match the owned ELF; anchor pins; manager slot decode (SPEC-core-05 §9) | `tests/ui_sites.rs` (4), `model::sites_ui::tests` | install line `ui_*=stock/near`, `[Ui] anchors installed` (s5-02..09) | pass |

Package 5 test record opened 2026-11-02 (`d1de600e`). Each status records the evidence available at that stage.

| Requirement (spec) | Host test | Eden row | Result |
| --- | --- | --- | --- |
| UI site rows match the owned ELF; anchor pins; manager slot decode (SPEC-core-05 §9) | `tests/ui_sites.rs` (4), `model::sites_ui::tests` | install line `ui_*=stock/near`, `[Ui] anchors installed` (s5-02..09) | pass |
| Archive recipe rebuilds the target, refuses a changed stock archive and any partial or corrupted recipe | `model::archive_recipe::tests` | `[LayoutArchive] handed out ... bytes=467860 stock=451604` | pass |
| SARC directory lookup (carried) and archive retirement (carried) | `model::archive_name::tests`, `model::archive_retire::tests` | handed out line | pass |
| Pane table: unique names, indexes, linear colour | `model::panes::tests` | `resolved ... missing=0` | pass |
| Page frame: groups, rows, values, help (SPEC-core-03 §2.2, §2.5) | `model::frame::tests` | s5-07/09 screenshots | pass |
| Sub-screens (list, checklist, delay incl. grey reason, slider, swap, overrides, chords, chord editor), grids by console mode | `model::screens::tests` | s5-09 p02-p05; s5-10 every sub-screen and both grid layouts (docked 5 x 8, handheld 4 x 4 by flipping the Eden operation mode live) | pass; mockup differences listed in `eden/EDEN-PACKAGE-5.md` |
| Input adapter: chord edge rule, stick repeat, controller mapping (SPEC-core-03 §2.1, §3) | `model::adapter::tests` | s5-07 open / navigate / close | pass |
| Close fade curve | `model::frame::tests::the_fade_is_logarithmic...` | close screenshot | pass |
| P1 output empty while open and 15 frames after (owner bit) | `model::input_delay` poll tests (package 1) | s5-07: P1 did not act | partial: no adapter-level lane for the producer yet |
| HUD hidden while the menu is open | - | s5-08 / s5-09 screenshots | pass |
| `hud` Off in a running match | `Snapshot::hud` default | s5-10 `hud-on-vs-off.png`: percentages, names and cursors hidden, the Training panel stays | pass |
| Pilot absent: the two fallback rows; Developer page in the ring only with `developer.ini` | `model::menu` tests | s5-11 (`20-developer.png`, `cpu-p1-fallback-rows.png`) | pass |
| Run-time file route (SPEC-core-07 U1-U3) | - | s5-12 `[Probe]` lines: `arcrop_load_file` from a plugin thread returns the mod's file; `mods:/` serves only mod files; `arc:/` the vanilla view | answered |

<a id="package-6"></a>

<a id="package-6--test-provenance-package-6-the-fork-s-engines-carried-hot-swap-preload-kirby-hats-items-mii-lobby-practice-cpu-as-p1"></a>

## Test provenance, package 6 (the fork's engines carried: hot swap, preload, Kirby hats, items, Mii, lobby practice, CPU-as-P1)

| Requirement | Host test | Eden row | Status |
| --- | --- | --- | --- |
| The plugin has one `#[global_allocator]` (`runtime::alloc`: the counting allocator, or the recorder's forwarding `TraceAllocator` under `console-practice-trace`) | `pkg6_features::the_plugin_has_one_global_allocator` | - | host pass |

Package 6 test record opened 2026-11-02 (`c008bcfc`). Each status records the evidence available at that stage.

- `Requirement | host test | Eden row | status`.
- The carried engines bring their own host tests (the fork's unit and integration tests, run in every by-path test binary; counts overlap) and are not listed one by one: see `docs/provenance/pkg6-carry-ledger.md` and `tests/carry_pkg6.rs` (a hand edit of a carried file fails).
- This file lists what the clean-room work of package 6 added, and the Eden rows (labels `s6-NN`, results in `eden/EDEN-PACKAGE-6.md`).
- Status: **host pass**, **Eden pass**, **runtime, unrun** (written and Switch-compiled, no Eden row yet), **not built** (reported).

<a id="package-6--allocator-and-features-lead-ruling-2026-11-02-one-global-allocator-every-feature-builds"></a>

### Allocator and features (lead ruling 2026-11-02: one global allocator, every feature builds)

| Requirement | Host test | Eden row | Status |
| --- | --- | --- | --- |
| The plugin has one `#[global_allocator]` (`runtime::alloc`: the counting allocator, or the recorder's forwarding `TraceAllocator` under `console-practice-trace`) | `pkg6_features::the_plugin_has_one_global_allocator` | - | host pass |
| The byte meter of the one allocator counts live, peak, total, calls and largest request (`plugin-heap-meter`, `console-practice-trace`) | `hook_alloc::the_byte_meter_counts_what_the_one_global_allocator_serves` | diag boot with `diag,plugin-heap-meter`: `[HeapMeter]` lines | host pass |
| Every feature of the table builds, alone and on the default set, and the allocator features together | `pkg6_features::the_feature_matrix_lists_every_feature` (the list); `python3 -I tools/feature_matrix.py --run` (the builds: host `cargo check`, Switch `cargo skyline check`) | - | see `IMPLEMENTATION-PROGRESS-pkg6.md` (removed after v0.2.0-rc.1; see that tag) |
| `default` is the shipped list (minus `deluxe-sdk`, L1); no diagnostic feature in it; every probe requires `diag` | `pkg6_features::*` | - | host pass |

<a id="package-6--owners-of-the-moments-the-fork-called-the-unused-helpers-from-lead-ruling-2026-11-02"></a>

### Owners of the moments the fork called the unused helpers from (lead ruling 2026-11-02)

| Requirement | Host test | Eden row | Status |
| --- | --- | --- | --- |
| Solo Popo (no Nana): a load of Popo out of REBIRTH ends at Popo, no `NanaPosMove` marker | `state01_machine::state01_solo_popo_has_no_handoff` | native Training, Solo Popo on, save and load | host pass; runtime, unrun |
| The Ice Climbers hand-off with Nana is unchanged | `state01_machine::state01_nana_handoff` | native Training, Ice Climbers, save and load | host pass |
| Kirby's saved hat comes back through the hat engine without the hat's owner in the match; a cancelled load drops the queued restore | none host-side (game memory) | Kirby hats row | runtime, unrun |
| A menu change of the 13 preload rows reaches the engine's INI and cache (`publish_applied` from the menu commit) | `cycle_preload_config` INI transaction tests (carried, 9 clean-room tests) | change a preload row in the menu or by `diag-settings-next.json`: `[CyclePreload] applied` | runtime, unrun |
| Held item save/restore is the item engine's alone; the duplicate natives are gone | `pkg6_item_table`, `state04_items` | item save/restore with the owner not in the match (`diag-item.req`) | runtime, unrun |
| An automation never touches a helper body of the cycle (entries 2, 3, 5, 6): by role, not by a reserved test | `pkg2` role-table tests (`model::cpu::role`) | managed Kirby / third source row | host pass; runtime, unrun |

<a id="rc15-colour-legend"></a>

<a id="rc15-colour-legend--hitbox-colour-legend-evidence-2026-11-15"></a>

## Hitbox colour legend evidence — 2026-11-15

| Original PNG, copied without modification | Dimensions | SHA-256 |
| --- | --- | --- |
| [Normal colours](media/rc15-colour-legend-normal.png) | 1280 × 674 | `9d0b41973ab7d299a0a5bcdf708f4d25244fb80536c1f5b74a488739c5941a7f` |

These rc.15 captures show the former seven-item Hitbox Visualization checklist and its two colour schemes.

- Tested merged source: `3526828736909e797fafe5b7e192edac5616e4bb` (rc.15).
- Diagnostic NRO SHA-256: `1660a9e420016760535cd1641d33b6905c7db627bca351a43745f479e1b02d5c`.
- Eden run: `rc15-integration-35268287-remaining1`, Yoga; row `colour-legend-both-schemes`.
- Row: PASS, 2026-11-15 20:46:02–20:46:26 UTC; starts and ends at the Training HUD.
- The row's `requested_source=c1d9d2b6` names the donor implementation; it is not the merged binary identity.

- The normal capture has Frame data colours unticked and the other six items ticked.
- Its thirteen keys show all eight hitbox colours, magenta grab boxes, yellow fighter/article hurtboxes, deep-blue windboxes and wind areas, and white direction lines.
- After Down three times and A, Frame data colours is visibly ticked.
- Nine keys show strong red and weak pink hitboxes, purple grabs, white normal hurtboxes, blue unhittable hurtboxes, green no-damage hurtboxes, and the wind and direction keys.
- Both complete checklists, swatches and labels were visually reviewed: no overlap or clipping.

The existing per-press Ui trace records Checklist cursor 0 on opening and cursor 3 on the A that changes the scheme.
The runner receipt and its start/end HUD gate captures are retained with the run. Automated collection alone is not
visual acceptance; the separate image review establishes the observations above.

| Original PNG, copied without modification | Dimensions | SHA-256 |
| --- | --- | --- |
| [Normal colours](media/rc15-colour-legend-normal.png) | 1280 × 674 | `9d0b41973ab7d299a0a5bcdf708f4d25244fb80536c1f5b74a488739c5941a7f` |
| [Frame data colours](media/rc15-colour-legend-frame-data.png) | 1280 × 674 | `42be49c942d872fb9dce16303c23e1764625f97fa2829f80002d560e2960e589` |

- Host coverage: `menu_swatches_match_the_renderer_in_both_schemes` checked actual renderer colours in both schemes and all 128 selection masks; `hitbox_checklist_colour_key_updates_and_clears_with_the_screen` checked menu updates and cleanup.
- The merged XPS gate passed tests, Clippy with warnings denied, no-default-features, release, diag and size checks.
- Its normalized production total is 148,194, with 56 lines contributed by the legend.
- Switch builds retain an inherited unused-function warning; host Clippy has none.
- No strict aggregate Cargo test count is claimed.

- This documentation was added after that build; it changes no product source.
- The captures establish the legend's menu layout and scheme change in Eden, not hardware behavior or every gameplay status.
- The image named for a final-build footer did not visibly show the version text and is not used as footer proof.
- Other integration rows and run closure have their own receipts; this record makes no whole-run or release-readiness claim.
