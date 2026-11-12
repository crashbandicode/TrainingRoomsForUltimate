# rc.9 line coverage

2026-11-12. Line coverage of the `training_rooms` crate's host-compiled product code, measured with `cargo llvm-cov` 0.9.1
(rustc 1.98.1, LLVM 22) at `a25fbbc6`. The order was 95% if possible with at most 1,000 tests. **Reached: 91.54% with 865 tests.**
What blocks the rest is under "Why not 95%". **Then reduced on the operator's choice to the cases that add coverage: 90.18% with 821 test
functions and 1,689 cases** (the last section, "Option (b)"); the sections before it describe the state before that reduction.

## Figures

| State | Tests (`cargo test`) | Product lines | Covered | Line coverage |
| --- | ---: | ---: | ---: | ---: |
| Before the test cut (`28082c29`) | 41,918 | 40,012 | 36,345 | 90.84% |
| After the test cut, the baseline of this work (`7a4cba64`) | 942 | 38,601 | 13,215 | 34.23% |
| Now (`a25fbbc6`) | 865 | 39,457 | 36,118 | 91.54% |

- Without the five `src/runtime/` files (see "Scope"): 35,499 of 38,458, **92.31%**.
- Tests now: 865 in 108 binaries; with the two vendored crates' own gates (28 + 24), 917. The limit is 1,000.
- The vendored crates are not counted: `crates/hos-lock` reads 95.35% (328 of 344 lines) from this crate's by-path includes;
  `crates/smash-match-data` and `third_party/` are not instrumented by this run (their own `cargo test` gates are unchanged).

## How it was raised

- **The cut tests came back as cases.** `tools/test_cut.py` (`tabulate_region`) turns the `#[test]` functions of a module into
  the rows of one `#[test] fn case_table()`. Each case is the test function it was, unchanged, and asserts what it asserted.
  `src/test_table.rs` runs each case on its own thread, as the harness did, runs every case even after one fails, and names
  the failed ones. A test under `cfg`, `ignore` or `should_panic`, and one that re-executes the test binary by its own name
  (`current_exe`), stays a plain test.
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

## Scope and exclusions

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

## Per module

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

## Why not 95%

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

## Reproduce

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

## Product code

No product behaviour change. The release and diag NROs built at the commit before this work (`ef4113c2`) and at this commit have
the same size and differ in 56 bytes each: the 32-byte build id, 7 characters of the commit in the build tag and 17 bytes of line
numbers in 16 panic locations. The test cut's re-gates of `swap/mod.rs`, `stages/mod.rs` and `rr_host.rs` and its removed
`#[cfg(test)]` helpers are back as they were at `28082c29`, because their tests are back; the import edit in `compat/menu.rs` stays
(the two names are imported by its test module). `docs/TEST-CUT-LEDGER-rc9.md` describes the cut as it was made; where it says a
module's tests are removed, this file says how they returned.

## Option (b): only the cases that add coverage (2026-11-12)

The table tests above put the cut tests back as rows: 863 test functions, but 2,894 cases. The operator chose to keep only the
cases that add coverage nothing else gives, with at most a 1.5-point drop.

| | Test functions | Cases | Line coverage |
| --- | ---: | ---: | ---: |
| Before (`b1e27e5a`) | 863 | 2,894 | 91.53% (36,116 of 39,457) |
| After | 821 | 1,689 | 90.18% (35,583 of 39,457) |

- A case is one row of a `case_table` or one plain `#[test]`. After: 1,113 rows in 245 tables and 576 plain tests.
- With the two vendored crates' own gates the test functions are 873. The limit is 1,000.

### Method

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

### What was dropped, and why it is redundant

All but a few dropped cases cover only product lines that the kept cases also cover: measured, case by case. The few that
covered more account for the cost: 533 lines (1.35 points) that only dropped cases covered. This is redundancy of coverage,
not of assertions: a dropped case may have asserted a different outcome on the same lines. None is in a required group or
names an rc.8/rc.9 fix.

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

### Reproduce

```text
CARGO_TARGET_DIR=<fresh dir> cargo llvm-cov --no-fail-fast --no-report
python3 -I tools/case_cover.py count <fresh dir>                  # cases, table rows, plain tests
python3 -I tools/case_cover.py measure <fresh dir> <out dir>      # per-case lines (about 12 minutes, 4 jobs)
python3 -I tools/coverage.py export <fresh dir> cov.lcov && python3 -I tools/coverage.py report cov.lcov .
```

### Product code

No product behaviour change. The release and diag NROs at `b1e27e5a` and after this change have the same sizes and differ in 53
bytes each (build id, commit characters in the build tag, panic-location line numbers). With test modules removed, every `src/`
file is the text it was, except a `#[cfg(test)] use` in `mii/replay.rs` that only removed cases used.
