# Budgets

<a id="code-reduction-rc10"></a>

<a id="code-reduction-rc10--rc-10-code-reduction-measurement-and-plan"></a>

## rc.10 code reduction: measurement and plan

| Baseline | Raw non-test lines |
| --- | --- |
| Upstream | 24,325 |

The plan of `TASK-rc10-code-reduction.md`, measured on `main` `b1e27e5a`, 2026-11-12 (UTC), and the result of each
slice as it lands on `rc10`. Done so far: **S3**, **S1**, **S2**, **S4** and **S3b** (below, one section each).

<a id="code-reduction-rc10--the-short-answer"></a>

### The short answer

| Baseline | Raw non-test lines |
| --- | --- |
| Upstream | 24,325 |
| TrainingRooms | 129,593 |
| Estimated TrainingRooms floor | 110,000 to 117,000 |

- The TrainingRooms count includes 122,707 product, 6,755 diagnostic-only and 131 host-only lines.
  The 48,930 test lines are separate.
- **The target of about 24k is not reachable with every feature kept.** The parts of TrainingRooms that do what
  upstream also does are already larger than all of upstream, and the features upstream doesn't have are about 79k
  more.
- The estimated floor was 4.5–4.8 times upstream, a cut of 12–19k lines (9–15%).
  About 12–16k of that is low risk and leaves the release NRO's code unchanged.
- At the planning stage, three changes needed approval: collapsing the Cargo feature table (S2),
  deleting unused diagnostic probes (S4) and moving guard tables into data files (S1).

<a id="code-reduction-rc10--how-the-lines-were-counted"></a>

### How the lines were counted

| Counter | Rule | TrainingRooms | Upstream `077a8e4` | Ratio |
| --- | --- | --- | --- | --- |
| Raw (the task's rule) | non-blank, non-comment lines of `*.rs` outside `target/`, `tests/`, `third_party/`, `vendor/`; test code separated | **129,593** non-test | **24,325** | 5.3× |
| `tools/sloc.py` (the repo's own, rustfmt-normalized `rustfmt-sloc4`) | every file formatted with the pinned rustfmt first, then the same rule | **165,931** | **24,343** | 6.8× |

Two counters, both on the same tree.

- The raw counter is `tools/loc_raw.py` (`python3 -I tools/loc_raw.py . --depth 2`). It classifies each line by the
  innermost `#[cfg]` item around it:
  - **test**: `#[cfg(test)]` items, and files reached only through a test-gated `mod`;
  - **diag**: items that need a feature the shipped NRO never enables (`diag` and the probe/trace/meter family);
  - **host**: items that need `not(target_os = "switch")`;
  - **product**: everything else.
- The task's baseline said about 155,300 lines for `src/`. The exact figure for `src/` on `b1e27e5a` is 173,786
  (564 files), tests included; a plain count of non-blank lines not starting with `//` gives the same number.
  `crates/` is 4,168, which matches.
- Upstream was counted only (its `*.rs` files exported from the local mirror at `077a8e4`); no upstream code was read.
- **The raw rule flatters TrainingRooms.** Its lines hold 11.1 tokens on average and upstream's hold 4.8, so by
  tokens TrainingRooms is 12.3 times upstream. The carried native code is written in very wide lines; rustfmt
  spreads `src/runtime` from 58,684 to 81,829 lines. No slice below uses formatting to move a number.

<a id="code-reduction-rc10--product-diagnostic-and-test-lines-per-area-raw-counter"></a>

#### Product, diagnostic and test lines per area (raw counter)

| Area | Files | Product | Diag-only | Test |
| --- | --- | --- | --- | --- |
| Fighter cycles, hot switching, Kirby hats | 106 | 27,337 | 2,777 | 16,037 |
| Mii fighters | 83 | 21,230 | 804 | 7,358 |
| Recording, macros, replay capture, clips (with `crates/smash-match-data`) | 71 | 19,827 | 734 | 10,403 |
| Guard tables and their installer | 40 | 10,702 | 279 | 1,820 |
| Menu and UI | 47 | 9,491 | 490 | 1,886 |
| Save states | 34 | 7,146 | 185 | 640 |
| Platform: boot, threads, locks, logs, compat, migration | 73 | 6,344 | 682 | 1,913 |
| Hitbox, frame strip, input display, game speed, input delay, notices | 29 | 5,321 | 423 | 2,018 |
| Waiting rooms, lobby speed, stages | 48 | 5,121 | 86 | 3,681 |
| CPU behaviours | 24 | 4,374 | 295 | 243 |
| Combos, drills, grading | 13 | 3,010 | 0 | 2,188 |
| CPU pilot and Player 1 | 10 | 2,434 | 0 | 588 |
| `tools/layout-builder` | 2 | 370 | 0 | 155 |
| **Total** | 580 | **122,707** | **6,755** (+131 host-only) | **48,930** |

The areas are path prefixes, so a few shared files sit in the nearest area.
Of the 6,755 diag-only lines, 926 are the host simulator in `src/model/swap/swap_model/` (`any(test, feature =
"simulator")`), which `tools/sloc.py` reports apart.

The ten largest files hold 18% of the product lines:

| Product lines | File |
| --- | --- |
| 5,234 | `src/runtime/roster/native_body_bank.rs` |
| 2,613 | `src/runtime/roster/character_sources.rs` |
| 2,023 | `src/runtime/roster/character_roster.rs` |
| 1,946 | `src/runtime/mii/mii_hud_portrait.rs` |
| 1,937 | `src/runtime/roster/native_source_capacity.rs` |
| 1,933 | `src/model/seams/native_seams/rows_a2.rs` (generated table) |
| 1,921 | `src/model/recording_menu.rs` |
| 1,832 | `src/runtime/p1/cpu_pilot.rs` |
| 1,594 | `src/model/sites_pkg3.rs` (generated table) |
| 1,504 | `src/model/macro_ui/macro_panel.rs` |

<a id="code-reduction-rc10--where-the-size-is"></a>

### Where the size is

| Finding | Lines | What it means |
| --- | --- | --- |
| Generated guard tables | 8,413 in 21 files | 962 rows (690 `Seam`, 272 `Site`), 8 to 13 lines each: a label, an offset and 64 bytes. This is data written as code. |
| Two guard-table systems | about 700 | `model/seams/native_seams` (carried) and `model/sites*.rs` (new) have their own row struct, projections and outcome entry points over one rule set (`model/guard.rs`). |
| Rows for probes that no longer exist | 86 rows, about 700 | 61 rows name eight features that are not in `Cargo.toml` any more (`mii-fullconfig-probe`, `lobby-stage-probe`, …); 25 name non-shipped features. All 86 are still compared at boot and sit in the NRO. |
| Feature gates that are always on | 910 attribute lines, 406 never-compiled lines | 26 features are in the shipped closure; every gate on them is true in every build that ships, and their `not(feature)` arms never compile. |
| Diagnostic-only code | 6,755 | 2,447 plain `diag`; 1,896 host simulator and its helpers; the rest are probe features. Five probe features have no reference outside `src/` except the feature test and provenance files (about 1,050 lines). |
| Functions used only by tests, or by nothing | at least 1,565 | By name only (836 test-only, 729 unreferenced), so a lower bound. The crate is a `cdylib` with `pub` modules, which hides dead code from rustc's lint. |
| Repeated 6-line windows | 2,739 (2.2%) | 804 of them between `runtime/mii/mii_ordinary_cpu.rs` and `mii_ordinary_player.rs`. The rest is the tables above and small runs. |
| Label tables (`X => "text"`) | 557 arms | Spread over the menu, recording pop-up and macro panel. |
| Lines that are only closing brackets | 22,905 (18.7%) | Normal for Rust (upstream: 30.4%). Not a reduction target. |

Measured over the product lines.

What I looked for and did not find in quantity:
- settled `const … : bool = true` switches (5 in the tree);
- `#[allow]` attributes in product code (4, in `hos-lock` and the layout tool);
- large copy-pasted modules, apart from the two Mii role files.

<a id="code-reduction-rc10--the-slice-plan"></a>

### The slice plan

| # | Slice | Saving (raw lines) | Risk | How its behaviour is verified |
| --- | --- | --- | --- | --- |
| S1 | Guard tables become data | −7,900 | Low | A host test digests every row (label, offset, kind, guard, flags, order) before and after: same digest. The NRO's tables are byte-identical. One Eden boot; the `[TrainingSeams]` install line is the same. |
| S2 | Collapse the always-on feature table | −1,300 | Low | Identical release and diag NRO. `--no-default-features` still builds. |
| S3 | Delete code the compiler proves dead; move test-only code behind `#[cfg(test)]` | −2,000 to −4,000 (**done: −2,890**; about 2,500 more need a decision) | Low | Identical NRO (LTO already drops it). Tests green, coverage ≥ 90%. |
| S4 | Delete probes nothing uses | −1,050, up to −2,500 | Low for the release NRO; the diag NRO changes | Identical release NRO. Diag build, one Eden boot, and every log line the Eden rows expect is still printed. |
| S5 | One guard-table system | −300 | Medium | The S1 digest test, the install-line tests, one Eden boot with a Training entry and a waiting room. |
| S6 | One Mii role owner for Player 1 and the CPU | −800 | High | Model tests; Eden Mii rows for both roles (cycle to a saved Mii, Add New, the Replay window's Mii switch, a Local Wireless room). |
| S7 | Menu and pop-up label and handler tables | −1,000 to −2,000 | Medium | The existing menu, panel and screen tests (0.9–1.5k lines per file); an Eden menu tour compared frame by frame with rc.9. |
| S8 | Fold the `compat` menu shim into `Settings` | −500 | High | Only if asked for. Touches most carried runtime files; needs the full Eden pass. |

Ranked by safe saving. "Identical NRO" means the rc.9 test-cut method: the release and diag NROs differ from the
base only in the build id, the commit tag and panic line numbers.

**Total:** −12,250 to −15,700 for S1–S4; −14,850 to −19,300 with S5–S8.

<a id="code-reduction-rc10--s1-guard-tables-become-data"></a>

#### S1: guard tables become data

- **Now:** `rows_a1…a5.rs`, eight more `rows_*.rs` and nine `sites_*.rs` are generated Rust (their headers say "do
  not edit by hand"): 962 rows in 8,413 lines.
- **After:** the rows live in a data file per table (the generators already write TSV), and the tables are built at
  compile time into the same `static` arrays, in the same order. The six table walkers in `native_seams/mod.rs`
  shrink to one.
- **Needs a decision:** the task lists "generated tables that could be data", and this is the largest safe slice,
  but the lines move out of `*.rs`; they are not deleted. If the primary would count that as gaming the number, the
  alternative is one row per line in Rust, which saves the same lines under the raw rule and nothing under
  `tools/sloc.py`. I recommend the data file.
- **Compliance:** the playbook's row "every site guard is 64 bytes" reads `guard: [...]` literals in `src/`. It
  still passes (the `EMPTY` row remains), but it then sees a few guards, not 962. A host test asserts 64 bytes for
  every data row; I will say so in the slice's report.
- The 86 rows for probes that no longer exist stay in S1, so the install line doesn't change. Dropping them is part
  of S4.

<a id="code-reduction-rc10--s2-collapse-the-always-on-feature-table"></a>

#### S2: collapse the always-on feature table

- **Now:** `Cargo.toml` carries the fork's table (decision D11). 26 of its features are always on in the shipped
  closure, with 910 gate lines on them and 406 lines that compile only when one is off.
- **After:** those gates and dead arms are gone. `diag`, the probe family that survives S4 and `deluxe-sdk` (an
  optional dependency) stay as features.
- **Needs the primary's word:** D11 is a recorded decision, `tests/pkg6_features.rs` and `tools/feature_matrix.py`
  check the table, and a `--no-default-features` build then holds the whole product (today it holds a smaller one
  that never ships).

<a id="code-reduction-rc10--s3-compiler-proven-dead-code"></a>

#### S3: compiler-proven dead code

- **First step, a measurement:** make the module tree `pub(crate)` (a `cdylib` exports nothing by Rust path) and
  read rustc's `dead_code` warnings for the Switch build and for the test build.
- Dead in both: delete. Live only in tests: move behind `#[cfg(test)]`, where it stays tested and counts as test
  code.
- The estimate is soft until that step runs; the by-name lower bound is 1,565 lines. `tests/*.rs` include the model
  by path, so the visibility change has to keep them compiling.

<a id="code-reduction-rc10--s4-probes-nothing-uses"></a>

#### S4: probes nothing uses

- **Group A, no reference outside `src/` but the feature test and provenance files (about 1,050 lines):**
  `mii-ldn-qualification-probe` (576), `native-unlink-probe` (297), `texture-probe` (124), `console-swap-trace`
  (40), `automation-results-native` (2), and their seam rows.
- **Group B, needs the primary's word (about 1,450 lines):** `console-practice-trace` (1,086, used by
  `tests/hook_alloc.rs` and two tools), `named-input-trace` (107), `hang-trace` and `stall_watch` (about 200, named
  in `size-budget.toml`) and `plugin-heap-meter` (60).
- Kept: plain `diag`, `seam-drill` (the playbook's hook-sharing drill) and the host simulator.
- Also here: dropping the 86 seam rows of removed and non-shipped probes. After S1 they are data, so this saves no
  `*.rs` lines; it changes the install line's row count and frees about 9 KB of the resident image.
- **The question for the primary:** do Eden-only probes count as "functionality we have now"? If they do, S4 is
  dropped and the floor rises by 1,050 to 2,500.

<a id="code-reduction-rc10--s5-s8"></a>

#### S5–S8

- **S5** follows S1: one row type and one projection for both table systems.
- **S6:** `runtime/mii/mii_ordinary_cpu.rs` (1,433) and `mii_ordinary_player.rs` (1,220) share 804 lines of
  identical 6-line windows, and `model/mii/ordinary_cpu.rs` and `ordinary_player.rs` mirror each other. This is
  native Mii install and replay code under locks, so it carries the Eden Mii rows for both roles.
- **S7:** `recording_menu.rs`, `macro_panel.rs`, `menu/mod.rs`, `panes.rs`, `runtime/ui/view.rs` and `worker.rs`
  (7,431 lines). A table replaces repeated arms only where the drawn output is unchanged. I measure each file before
  claiming its number.
- **S8:** `model/compat` and `runtime/compat` (1,146 lines) let the carried code read the fork's old menu struct.
  Removing them is a re-architecture of the carried runtime's settings reads; I don't recommend it for rc.10.

<a id="code-reduction-rc10--each-slice-every-time"></a>

#### Each slice, every time

- Gates: clippy 0, `--no-default-features`, release and diag builds, size, the stack table, svc, the game-thread
  wait check, carry, the tag gate, `tools/check/compliance.py`.
- Tests green, ≤ 1,000 test functions, line coverage ≥ 90%; tests move with their code.
- A no-behaviour-change argument and the line delta (both counters) recorded here.
- An Eden check with the GDB stub on where the table above names one; pure model slices rely on tests.
- Cargo under `systemd-run --user --scope -p MemoryMax=…` and `nice 19`, one subcommand per call, in my own worktree
  and target.

<a id="code-reduction-rc10--the-realistic-floor"></a>

### The realistic floor

| Stage | Non-test lines (raw) | Against upstream's 24,325 |
| --- | --- | --- |
| Now | 129,593 | 5.3× |
| After S1–S4 (low risk) | about 114,000–117,000 | 4.7–4.8× |
| After S5–S7 | about 111,000–115,000 | 4.6–4.7× |
| With S8 and what S3 and S7 find beyond their estimates | about 110,000 | 4.5× |

On `tools/sloc.py` the same work gives about 146,000–153,000 from 165,931 (6.0–6.3× upstream's 24,343). That is an
estimate: I have not run the normalized counter per slice yet.

What holds the rest:

1. **The parts that match upstream are already bigger than upstream.** CPU behaviours, save states, the hitbox and
   input displays, the menu and UI and the platform code are 32,676 product lines, before their guard tables.
2. **The features upstream doesn't have are 78,959 lines:** hot fighter switching (27,337), Mii fighters (21,230),
   recording, replay capture and clips (19,827), waiting rooms (5,121), combos (3,010) and the CPU pilot (2,434).
   The task forbids dropping any of them.
3. **The carried native code has no slack per line.** `src/runtime/roster` and `src/runtime/mii` are 31,714 lines
   at 11 or more tokens a line. Each records reverse-engineered game behaviour that an Eden row proved. Shrinking it
   means rewriting and re-qualifying it, which is a different campaign with a different risk.
4. **Two vendored crates stay in step with their origins:** `smash-match-data` (2,786) and `hos-lock` (760).
5. **The playbook's safety model costs lines by design:** a 64-byte guard for every site (962 rows), the pure model
   split from the runtime so it can be tested on the host, and the diagnostic build.

Getting near 24k would mean removing features or the guard and test structure. I don't propose either.

<a id="code-reduction-rc10--s3-done-2026-11-12"></a>

### S3: done (2026-11-12)

| S3 inclusion rule | Decision |
| --- | --- |
| Unused by the Switch and tests | Remove only if the NROs stay unchanged and no warning appears |
| Used only by tests | Gate with `#[cfg(test)]` under the same condition |

**What moved**

| | Items | Raw product lines |
| --- | --- | --- |
| Dead in the default, diag and all-probes Switch builds | 1,050 | |
| Eight files that are test-only as a whole, gated at their `mod` line | | 1,500 |
| Functions, methods, constants and statics deleted (dead in the tests too) | about 108 | about 310 |
| Functions, methods, constants and statics moved behind `#[cfg(test)]` | about 270 | about 1,080 |
| Left: dead types (structs, enums, a trait) outside those files, and the dead code that touches them | about 580 | |
| Left: fields and enum variants | 71 | |

- **Product lines: 122,707 to 119,817 (−2,890).** Non-test lines: 129,593 to 126,703. On `tools/sloc.py`: 165,930 to
  162,692 (−3,238). `size-budget.toml` is ratcheted down to 162,692.
- **Most of the saving is a reclassification, not a deletion.** The gated lines were never in an NRO; they were
  written as product code and only the tests called them. 370 lines were removed and 351 added (308 `#[cfg(test)]`
  lines, 30 test-only `use` statements).
- 130 files under `src/` changed. 51 carried files have the change recorded in the carry manifests as a `line_patch`.
- The eight files: `model/lobby/automation/{adapter,control,flow,popup,recording}.rs`, `model/macro_ui/replay_panel.rs`,
  `model/preload/notice_queue.rs`, `model/swap/swap_model/hud_model.rs`.

**How the items were found**

1. `src/lib.rs`'s root modules were made private for the measurement only (not committed). A `cdylib` exports nothing
   by Rust path, so rustc's `dead_code` lint then sees the whole crate.
2. Four lists of dead items: the default Switch build, the diag build, a build with every probe feature that
   compiles, and the library's unit-test build.
3. Dead in all three Switch builds and in the unit tests: delete. Dead in the three Switch builds only: gate.
4. Every test target was then compiled. A deleted item a test crate under `tests/` still named came back, gated
   (by name, so conservatively).
5. Imports of gated items became `#[cfg(test)] use` statements.

**The NROs are the same**

- Release and diag NROs built from the S3 commit against those built from `f5146184`: `.text` is byte-identical in both.
- Of the 4,136,960 and 4,329,472 bytes, 557 and 563 differ: the 32-byte build id, the 8 characters of the commit in the build tag, and the line numbers of 500 and 506 panic locations, which moved because lines above them were added or removed.

**Why dead types stayed, and what touches them**

- A first attempt also removed or gated the 132 dead types with their impl blocks (−5,428 product lines).
  - Its NROs had the same size and the same sections, and every function was the same once addresses were masked, but 83 functions had another mangled name and the functions were laid out in another order.
  - An impl block, written or derived, is numbered among the impls of its module, and that number is part of the name of everything in the later impls.
  - So removing a type renames code that stays.
- A second attempt kept the types and gated their methods (−4,169). Its `.text` was identical, but the kept types'
  fields and variants were then unused in the Switch builds: 16 new `dead_code` warnings.
- So S3 keeps every dead type, and everything dead that touches one, as it was. A file that is test-only as a whole
  is the exception: impl numbers are per module, so gating the whole module renames nothing that stays.
- What is left is about 2,500 product lines of dead types and their users. Removing it changes the layout of the
  NRO, so it needs a function-by-function proof (the first attempt has one) or an Eden pass. That is a decision for
  the primary.

**What else S3 did not touch, and why**

- **Fields and enum variants** (71): removing one changes a struct's layout or a `match`.
- **Items a macro generates** (the `game_constants!` tables of `model/cpu/ids.rs` and `model/state/ids.rs`): one
  expansion can be dead while another is live.
- **`is_empty` next to a public `len`** (4): clippy asks for it.
- **Items the default build does not use but the diag build does** (about 80): diagnostic code written without a
  `diag` gate. Gating it would move lines from product to diag-only; S3 left it.

**Found on the way**

- `--features named-input-trace` does not compile at `f5146184` (`src/runtime/macros/named_macros.rs:977`: no method
  `bits` on `u32`). S3 left it as it is; it belongs to S4's decision.
- The default Switch build has one `dead_code` warning that was there before S3: `runtime::lobby::lobby_speed::passes`
  is called only from diag blocks.
- `build.rs` still names the build `rc9`.
<a id="code-reduction-rc10--s1-done-2026-11-12"></a>

### S1: done (2026-11-12)

| S1 change | Measured result |
| --- | --- |
| Guard tables | 23 tables converted to data files |
| Raw product lines | 119,817 to 111,612 |
| NRO comparison | Only build identity changed |

The rows of the 23 guard tables are data files, `data/tables/*.tsv` (format: `data/tables/README.md`). `build.rs`
turns each file into the array its source file includes, so the tables are the same Rust values in the same order.

- **Product lines: 119,817 to 111,612 (−8,205).** Non-test lines: 126,703 to 118,493. On `tools/sloc.py`: 162,692 to
  154,519 (−8,173). The budget is ratcheted down to 154,519.
- **The lines moved; they were not deleted.** 958 rows left 22 `*.rs` files (8,526 lines) and are 1,231 lines of
  data (350 KB). `build.rs` grew by about 50 lines.
- **The NROs are the same:** against S3's, 40 bytes differ in each (the build id and the commit in the build tag).
- **Every row is identical.** `src/model/table_digest_tests.rs` pins a SHA-256 over every row's `Debug` text, in
  table order, for the 690 seam rows, the 258 site rows and the two small tables. The digests were taken while the
  rows were still Rust literals and pass unchanged on the data files.
- **Every data row has a 64-byte guard:** a host test reads the files, and `build.rs` fails the build on a guard that
  is not 128 hex digits.
- **The playbook's compliance row "every site guard is 64 bytes"** reads `guard: [..]` literals in `src/`. It still
  passes, on the few literals that remain (the empty row, two test rows), not on the 958 rows.
- The generators (`tools/gen_*.py`) still write a table as Rust; `tools/rows_tsv.py table` turns that into the data
  file (`tools/test_rows_tsv.py`).
- The 86 rows of removed and non-shipped probes are still in the tables, so the install line is unchanged. S4 drops
  the rows of the probes it removes.
<a id="code-reduction-rc10--s2-done-2026-11-12"></a>

### S2: done (2026-11-12)

| S2 change | Measured result |
| --- | --- |
| Always-enabled Cargo features | 25 removed as switches |
| Raw product lines | 111,612 to 110,367 |

The 25 Cargo features that every shipped build enabled are not switches any more (`docs/DECISIONS.md`, "the product
is not behind feature switches", which supersedes D11).

- **Product lines: 111,612 to 110,367 (−1,245).** Non-test lines: 118,493 to 117,248. On `tools/sloc.py`: 154,519 to
  152,990 (−1,529). The budget is ratcheted down to 152,990.
- **1,031 `cfg` uses in 59 files** were evaluated with those features on: an always-true gate is removed and its item
  stays; an always-false gate is removed with its item; a mixed one keeps only what still varies (`diag`, a probe,
  `test`, the target).
- **Run-time tests of a feature** (`cfg!(feature = ..)`) became constants. `x && true`-style operands were dropped by
  exact identities, and 16 conditions were simplified by hand, keeping the same evaluation: the dropped operands are
  pure, or were already short-circuited away. Two dead branches went with them (`runtime/roster/kirby_hats.rs`,
  `runtime/roster/character_items.rs`). Where a constant is only passed along as an argument, it stays `true`.
- **The NROs are not byte-identical to S1's, and cannot be:** the enabled feature set is part of the crate's symbol
  hash, so every symbol is renamed and the code is laid out in another order; the machine outliner then splits a few
  functions differently. Sizes are the same (release: every section; diag: `.text` +32 bytes).
- **The code is the same, function by function.** S1 and S2 release ELFs built with the machine outliner off have the
  same 7,150 functions, and 7,145 have the same instructions once addresses are masked. The other five: one closure
  whose index changed (same body); `prepare_with` in `runtime/roster/character_sources.rs`, whose simplified filter
  makes the same two calls in the other order and is one instruction shorter; and two functions whose only difference
  is an address-dependent encoding (`adrp`+`add` against `nop`+`adr`, a padding word).
- **`Cargo.toml`** keeps `deluxe-sdk` (an optional dependency), `diag`, the diagnostic family and `simulator`.
  `default = ["deluxe-sdk"]`.
- **`cargo build --no-default-features`** is now the whole product without the Deluxe SDK.
- **The guard tables' `feature` column is unchanged**: a row keeps the name it was carried with, and a name that is no
  longer a Cargo feature means "every build". The row digests are unchanged.
- **Tests and tools:** `tests/pkg6_features.rs` checks the new table (the 25 names are not features and nothing
  requires them; the table is exactly the 14 names left). `tools/feature_matrix.py` reads the table and needed only
  its description. Two tests that pin source text follow the new text (`model/parked_entries.rs`,
  `tests/lock_order.rs`: `runtime/compat/filesystem.rs` is now gated by `diag` alone, so the call graph treats its
  thread as diag-only, which it is).
<a id="code-reduction-rc10--s4-done-2026-11-12"></a>

### S4: done (2026-11-12)

| S4 change | Measured result |
| --- | --- |
| Unused probes | Five removed |
| Diagnostic-only lines | 6,750 to 5,692 |
| Installer | 15 fewer seam rows |

The five probes nothing used are deleted (the operator's Group A): `mii-ldn-qualification-probe`,
`native-unlink-probe`, `texture-probe`, `console-swap-trace` and `automation-results-native`. Group B stays
(`console-practice-trace`, `named-input-trace`, `hang-trace` with `stall_watch`, `plugin-heap-meter`).

- **Diag-only lines: 6,750 to 5,692 (−1,058).** Product lines 110,367 to 110,364. Non-test lines: 117,248 to
  116,187. On `tools/sloc.py`: 152,990 to 151,623 (−1,367). The budget is ratcheted down to 151,623.
- **Files deleted:** `runtime/mii/mii_ldn_qualification.rs`, `runtime/roster/native_unlink_probe.rs`,
  `runtime/ui/texture_probe.rs`, `runtime/lobby/automation_helper.rs` (it held only a comment). 43 gated items in 11
  other files went with the features.
- **15 seam rows deleted** from the data tables: 8 of `native-unlink-probe` (`rows_a5`) and 7 of
  `automation-results-native` (`rows_a4`). The seam table has 675 rows, was 690. This changes the release NRO: the
  installer compares 15 fewer sites at boot and the install line counts 15 fewer rows. The seam digest in
  `src/model/table_digest_tests.rs` is updated on purpose; the other three digests are unchanged.
- **Log lines the Eden rows expect:** the deleted code printed lines under `[MiiLdnQualification]`, `[NativeUnlink]`
  and `[Texture]`. No Eden row names `[MiiLdnQualification]`. The rows' `[NativeUnlink] experimental missing-parent
  guard enabled` and `[Texture] install usable=` lines come from product code that stays
  (`runtime/roster/native_unlink_install.rs`, `runtime/ui/texture.rs`). One file expects a deleted line:
  `eden/u4/shoot_u4.py`, the texture probe's own proof script (`[Texture] shown`); it cannot run any more.
- **Tests and tools:** `tests/pkg6_features.rs` (the table is 9 names; the five are asserted gone),
  `tests/lock_order.rs` (the probe's eight hook entries leave the list), `tools/feature_matrix.py` (its combinations).
- The other 71 rows of features that no longer exist stay in the tables: they were not in the operator's Group A.
- **`named-input-trace` compiles again** (Group B, kept): its trace line called `.bits()` on a `u32`. The fix is that
  one word. `cargo check --features named-input-trace` is now a gate of every slice (`docs/BUILD.md`, "Probe check";
  the `named-input-trace` row of `tools/feature_matrix.py`'s core set). Two of the probe's functions have no caller
  (`trace_write`, `trace_close_input`); they are left as they are.
- **Eden boot** (run `rc10-s4-fb028a9b-g2`, the Surface, diag NRO `d2d39f02…` of `fb028a9b`, the GDB stub on before the load, tracing off): the plugin installed (`result=installed heap_allocations=0`).
  - Eleven rows passed: the stub and the load, boot to Training, the old settings, the migration, a chord, the CPU fighter through the menu, five hot switches, a held item, the Replay window and a named Mii.
  - One row failed, `acc-hook` (no `hook_stats` line in the log); it failed the same way on the base build in the integration lane's last run of this set (`final-rc8-surface-1-eca14e4b`).
  - The console has no panic, abort, crash or unmapped-access line.
  - This boot is also S2's check: S2's binary was not byte-identical, and the build under test contains it.
<a id="code-reduction-rc10--s3b-done-2026-11-12"></a>

### S3b: done (2026-11-12)

| S3b change | Acceptance method |
| --- | --- |
| Dead types and their implementations | Remove after function-by-function comparison, not a byte-identity claim |
| Runtime coverage | Include the change in the final rc.10 Eden pass |

The dead code S3 left in place is removed: the dead types, their impl blocks and what touched them. The operator's
and the primary's answer (2026-11-12 05:25Z) was to take it, with a function-by-function proof in place of byte
identity and the rc.10 final Eden pass on top.

- **Product lines: 110,364 to 107,697 (−2,667).** Non-test lines: 116,187 to 113,520. On `tools/sloc.py`: 151,623 to
  147,878 (−3,745). The budget is ratcheted down to 147,878.
- **Method:** S3's, re-measured on `83e91989` (565 items dead in the default, diag and all-probes Switch builds; 140
  of them dead in the unit tests too), without S3's rule that kept dead types and everything touching them. 354
  functions, methods, constants, statics and types moved: deleted where no test uses them, behind `#[cfg(test)]`
  where only tests do. Fields and enum variants are a separate step.
- **Why the NRO is not byte-identical:** an impl block, written or derived, is numbered among the impls of its
  module, and that number is in the mangled name of everything in the later impls. Removing a type renames code that
  stays, and the linker lays functions out by name.

**The proof** (`tools` of the lane: `symcmp.py`; results `symcmp-s3b-release.txt`, `symcmp-s3b-diag.txt` in the
lane's evidence folder). Release and diag ELFs of `83e91989` against `2c728893`:

| | Release | Diag |
| --- | --- | --- |
| NRO size | 4,132,864 B both | 4,325,376 B both |
| `.text`, `.rodata`, `.data`, `.bss` | each the same size | each the same size |
| Function symbols | 13,474 both | 13,888 both |
| Same demangled name and size | all | all but 5 |
| Same instructions, addresses masked | all | all but 5 |
| Function order | differs | differs |

- "Addresses masked" means branch and `adrp` targets, wide hex immediates, and the low-12-bit offset of the
  instruction right after an `adrp`. Everything else in an instruction must match.
- **The five in the diag build** are four derived `Debug::fmt` functions (`mii::capacity::Roots`,
  `lobby::practice_stage::Resource`, `native_source_capacity::InputSlot`, `practice_stage_request_policy::Scene`) and
  one outlined helper. The machine outliner cut them at another place because the functions around them moved: in one
  build two `stp` instructions sit in the function, in the other in its outlined helper. The four differ by 4 or 8
  bytes each and sum to the same size.
- The stack table, svc and game-thread checks pass on both ELFs.
<a id="code-reduction-rc10--s3c-dead-fields-and-variants-tried-and-not-taken-2026-11-12"></a>

### S3c: dead fields and variants, tried and not taken (2026-11-12)

| Trial | Decision |
| --- | --- |
| 16 product lines saved | Patch retained outside rc.10 |
| About 90 affected release functions | Changed instructions or sizes; byte identity did not hold |

After S3b, 34 enum variants and 37 struct fields are still dead in every Switch build. The order was to remove those
in plain Rust types (no `repr`, never across FFI), with the same function-by-function proof.

- **Tried:** ten variants no build constructs and no test uses, and one field nothing reads, with their match arms:
  `Sharing::External`, `ClipEffect::None`, `PlayRefusal::FightersMissing`, `Phase::Idle`, `CellContent::Unsupported`,
  `NativeCall::WritePairRefs` and `SetRequestBit`, `LedgerRefusal::ResetNotExclusive`, `Exception::CrashGuard`, and
  `RecordStop::Saved`'s `start_save`. All gates passed. The saving was 16 product lines.
- **The proof fails, by nature.** Removing a variant renumbers the enum's discriminants and removes arms from the
  code that matches on it; removing a field changes the struct. In the release ELF about 90 functions changed size or
  instructions (`.text` −1,104 bytes), among them `body_bank::group_lifecycle`'s closures and `store::decode`.
- **So it is not on `rc10`.** For 16 lines it would need its own Eden pass. The commit is kept as a patch in the
  lane's evidence folder (`s3c-9c167a0a-not-pushed.patch`).
- **The rest were never candidates:**
  - 30 or so are used by tests (a test constructs the variant or reads the field);
  - the menu editors' five variants (`NamedLibrary`, `SaveSlotToLibrary`, `ComboSave`, `ComboLibrary`,
    `ComboTarget`) have whole panels as their match arms in `runtime/ui`: removing them is a UI change;
  - the fields of the guard-table rows (`note`, `feature`, `scopes`, `read_only`, `per_frame`), of the callback
    contracts (`seam`, `per_frame`, `review`), of the pane table and of the thread table are data and review records;
    the layout builder and the stack tool read some of them.
<a id="code-reduction-rc10--where-rc-10-stands-and-s5-to-s7-re-measured"></a>

### Where rc.10 stands, and S5 to S7 re-measured

| | Start (`f5146184`) | After S3, S1, S2, S4, S3b (`2c728893`) | Change |
| --- | --- | --- | --- |
| Product lines (raw) | 122,707 | 107,697 | −15,010 |
| Diag-only lines (raw) | 6,755 | 5,692 | −1,063 |
| Non-test lines (raw) | 129,593 | 113,520 | −16,073 (−12.4%) |
| `tools/sloc.py` | 165,930 | 147,878 | −18,052 |

Against upstream's 24,325 that is 4.7 times, from 5.3. It is inside the floor this plan gave (110,000–117,000).
Line coverage after S3b is 90.75% (it was 90.18% at the start), with 824 test functions.

The three slices left, measured on `fb028a9b` (before S3b):

| Slice | What is there now | Realistic saving | Risk and cost |
| --- | --- | --- | --- |
| S5, one guard-table system | The tables are data already. What is left is two row structs and their projections (`native_seams/mod.rs`, `sites.rs`) and the two installers that read them. | about −300 | Medium: both installers change; an Eden boot. |
| S6, one Mii role owner | `runtime/mii/mii_ordinary_cpu.rs` and `mii_ordinary_player.rs` share about 350 lines each of identical 6-line windows. | about −300 (one copy stays, plus the plumbing a shared owner needs) | High: native Mii transactions under locks; the Eden Mii rows for both roles. |
| S7, menu and pop-up tables | 549 `X => "text"` arms, at most 65 in one module. An arm is already one line; a table does not make it shorter. | under −300 | Medium: the menu and panel tests, and an Eden menu tour. |

Together that is about 800 lines, 0.7% of what is left, for two Eden passes and a high-risk refactor.

**Decision (the primary, 2026-11-12 06:55Z): S5 to S7 are skipped for rc.10.** S8 was never planned. The code
reduction of rc.10 ends with S3b. (The roughly 2,500 dead lines S3 had left are taken in S3b.)
<a id="code-reduction-rc10--line-deltas-per-slice"></a>

### Line deltas per slice

| Slice | Head | Raw non-test (product) | `tools/sloc.py` | Tests | Coverage |
| --- | --- | --- | --- | --- | --- |
| Plan baseline | `b1e27e5a` | 129,593 (122,707) | 165,931 | 865 (917 with the crates' gates) | 91.54% |
| `rc10` start | `f5146184` | 129,593 (122,707) | 165,930 | 821 (873) | 90.18% |
| S3 | `6c419689` | 126,703 (119,817) | 162,692 | 821 (873) | 90.59% |
| S1 | `13e8ef5d` | 118,493 (111,612) | 154,519 | 824 (876) | not re-measured |
| S2 | `84e39648` | 117,248 (110,367) | 152,990 | 824 (876) | not re-measured |
| S4 | `fb028a9b` | 116,187 (110,364) | 151,623 | 824 (876) | not re-measured |
| S3b | `2c728893` | 113,520 (107,697) | 147,878 | 824 (876) | 90.75% (34,633 of 38,162 lines) |

<a id="size-plan"></a>

<a id="size-plan--source-size-plan-and-measurement-history"></a>

## Source-size plan and measurement history

| Decision relayed by Astra lead | Measurement or requirement |
| --- | --- |
| Scope | Source lines, measured separately from NRO residency. |

[The latest total in the size budget](../size-budget.toml) sets the active normalized ceiling.

The dated entries below record the November 2, 2026 slim-stream plan and subsequent measurements.

<a id="size-plan--source-size-criterion-adopted-november-2-2026-14-2xz"></a>

### Source-size criterion adopted November 2, 2026 (14:2xZ)

| Decision relayed by Astra lead | Measurement or requirement |
| --- | --- |
| Scope | Source lines, measured separately from NRO residency. |
| RC criterion | Reduce the ceiling at integration. Recount pending additions listed in `size-budget.toml`. |
| Following version | Aim at least 15% below the fork. This superseded the original RC target below. |
| Method | Run `tools/sloc-fmt.ps1` on a scratch copy. Retain both normalized and raw sloc4. |
| Excluded reductions | Layout compression, counting-rule changes and behavior loss. |
| `98f9ab0` | 104,350 raw. Normalized 133,508 including 167 tool lines, 133,341 excluding scripts. |
| Formatter | rustfmt 1.9.0-stable, zero failures. |
| Primary base `ebfcb98` | 136,069 normalized excluding scripts. Contribution −2,728. |
| Fork M5q | 131,081 normalized excluding scripts. The candidate exceeded it by 2,260. |


Goal (operator, 2026-11-02): the implementation smaller than the fork's (M5q = 112,500), target **≤ 90,000 at RC**,
without dropping a feature or proven behaviour. Means: delete dead code (not hide it), retire probes that served,
tables instead of repeated arms/structs, shared abstractions for the carried engines' repeated patterns, one
implementation per concept.

Count: `python3 -I tools/sloc.py git <repo> <rev> 3` (the primary's `sloc4.py` rules since 2026-11-02; `third_party/`
excluded and reported apart). Numbers below marked sloc3 were counted before the switch. Gate:
`python3 -I tools/size_gate.py` against `size-budget.toml`, a gate step of its own (docs/BUILD.md); `cargo test`
runs its host tests (`tests/size_budget.rs`). "M" = measured, "E" = estimate. File:line references are at `7bb86fd`.

<a id="size-plan--1-november-2-baseline"></a>

### 1. November 2 baseline

| | Lines | |
| --- | ---: | --- |
| Fork M5q | 112,500 | operator |
| Clean room `7bb86fd` | 106,983 | M |
| Pending in unmerged streams (work tree minus merge base, ~12:10Z) | +7,061 | [Measured contributions across nine unmerged streams](#size-plan--unmerged-stream-contributions) |
| Deluxe SDK under `third_party/` | 599 | M, not counted (vendored) |
| If merged as they are | ~114,044 | above the fork |
| Budget start (`size-budget.toml`) | 114,100 | measured + pending, rounded up |

| Counter finding, November 2 | Measured effect |
| --- | --- |
| sloc3 external test module | Dropped `#[cfg(test)] mod x;` but counted its file. |
| sloc3 compound conditions | Kept `cfg(all(test, feature = …))` and `cfg(any(test, feature = "simulator"))`. |
| Miscounted test-only files | `macros/library/tests.rs` 624, `native_seams/tests.rs` 601, `swap_model/tests.rs` 456. |
| Other test files | `p1/tests.rs` 305, `macros/window/tests.rs` 265, `macros/recording/tests.rs` 202. |
| Other test files | `preload/ini_transactions_tests.rs` 160, `p1/move_observer_tests.rs` 146, `macros/test_util.rs` 64. |
| Other test files | `macros/move_crop_continuation_tests.rs` 59, `lobby/session_lease_tests.rs` 26. Total 2,908. |
| `swap/mii_cycle.rs:315` | 755 test lines behind `cfg(all(test, feature = "mii-saved-alpha"))`. |
| `swap_model/native_model.rs` | 848 lines behind host-only `cfg(any(test, feature = "simulator"))`. |
| Comparison | The fork’s 112,500 also included carried tests. Apply sloc4 equally to both trees. |
| Corrected fork M5q / clean room `21334c0` | 106,629 / 106,801. Both reported 1,118 simulator lines separately. |
| Target at that point | ≤90,000 at RC, stretch ≤85,300. Counter corrections earned no savings. |
| `tools/sloc.py` with sloc4 | `8b73694` 106,099, stream/slim `eb8e034` 103,497. |

| Unreachable files measured by `/tmp/slim_orphans.py` | Lines | Disposition at the time |
| --- | --- | --- |
| `runtime/lobby/automation_helper/native.rs`, `…/plugin.rs` | 425 + 58 | Deleted at `e79019b` and recorded as not carried. |
| `runtime/roster/effect_hook_chain.rs` | 73 | Same deletion and carry record. |
| `runtime/mii/mii_native_adapter.rs`, `mii_cpu_adapter.rs`, `mii_closed_ack.rs` | 218 + 199 + 143 | Retained for stream/pkg6’s new `mii_fullconfig_probe` via `#[path]`, pending its retirement. |
| pkg6 carry manifest subtotal | 1,116 | No `mod` or `#[path]` reached these files in the measured tree. |
| `runtime/macros/capture_plugin.rs` | 84 | Deleted on stream/deluxe at `166d3cd` with its carry record. |

<a id="size-plan--2-per-area"></a>

### 2. Per area

| Area estimates | Reading the rows |
| --- | --- |
| Savings | Counted source lines. E means estimated, M means measured. |
| Risk | Low, medium or high. |
| Rows | Eden scenarios listed for rerun after that proposed change. |

<a id="size-plan--roster-src-runtime-roster-19-551-and-swap-src-model-swap-7-980-carried-pkg6-manifest"></a>

#### roster (`src/runtime/roster` 19,551) and swap (`src/model/swap` 7,980), carried (pkg6 manifest)

At the November 2 checkpoint, EDEN-PACKAGE-6 covered the s6-01 sync probe and s6-02/03 boot and CPU-as-P1. CP0/CP1/CP3 still needed the roster cycle.

- The baseline required package 6 sets A/B/D, R-ID and R-PC before a roster refactor.
- The original plan is in `docs/PACKAGE-6-PLAN.md` §8 at tag `v0.2.0-rc.1` (subsequently removed).

| Finding (examples) | Change | Saves | Risk | Rows |
| --- | --- | ---: | --- | --- |
| [Legacy fingerprints alongside authoritative seam guards](#size-plan--legacy-roster-fingerprints) | delete the legacy words, keep relocated-data checks | ~190 | M | s6-02/03 install line, set A boot |
| `mii_reload_probe` module `character_roster.rs:2434-2835` names modules that do not exist at `7bb86fd` (`crate::mii_fullconfig_probe` …) | becomes compilable when stream/pkg6 lands those probes; retire with them (D11) | ~350 | L (not shipped) | none |
| Identical `trace!` macro defined 11 times (`copy_packages.rs:19`, `special_articles.rs:15`, …) | one crate macro | ~22 | L | none |
| `identity_view.rs:184-343`: 14 near-identical inline hooks | one `macro_rules!` table (site, offset, registers, scope) | ~60 | M (hook code) | R-ID0..9, R-PC1 |
| `cycle_preload_config.rs:66-182`: 4 logs of 11-15 fields | `Display` in the model, text byte-identical | ~45 | L | preload rows |
| [Repeated native wrappers, owner table and kind operations](#size-plan--repeated-native-wrappers) | one roster offsets table, one wrapper per native (extend `tests/native_wrappers.rs` beyond `game.rs`/`state_natives.rs`) | ~30 (rest leaves with the probes) | L | R-PC1/PC2 |
| `cfg(not(alpha))` alternates for builds never shipped (`character_sources.rs:1555-1608` …) | one version via `slot_allowed()`; all branches if D11 is relaxed | 40 / ~205 | L / M | set D, R-PC1 |
| Model functions only tests use (`body_bank.rs:287,508,568,836`, `ledger.rs:261,438,587,633-662`, `lifecycle.rs:306,350,381,399`, `roster_grid.rs:254-467` …), `swap_model/hud_model.rs` 68, `target.rs` 41 | delete with their tests (check roster_grid against the screens first) | ~360 | L-M | none |
| Probes and observers (§4): ~6.4k incl. ~520 probe-gated lines inside product files | retire after the owners confirm (D11 ruling) | ~6,400 | M | none (diag) |

Area: **0.8-1.4k** without a ruling; **+6.4k** with the probe ruling.

<a id="size-plan--mii-src-runtime-mii-15-167-src-model-mii-4-664-src-model-mii_root-1-083-carried"></a>

#### mii (`src/runtime/mii` 15,167, `src/model/mii` 4,664, `src/model/mii_root` 1,083), carried

At the November 2 baseline, Cargo.toml:5,55-62 enabled all `mii-*-alpha` defaults; 187 cfg lines supported subset builds (D11).

- That checkpoint had no Mii Eden run. R-MII1 (sets A/B) and set D were planned.

| Finding | Change | Saves | Risk | Rows |
| --- | --- | ---: | --- | --- |
| Two fingerprint schemes: `mii_saved_fingerprints.rs:3-76,103-120`, `mii_editor_bridge/fingerprints.rs:2-105,200-225`, `native_default.rs:12-27`, `mii_ordinary_gate.rs:7-14,40-45`, `mii_hud_portrait.rs:65-140`; all five groups `AUTHORITATIVE` | delete the word/FNV checks, keep `headgear_valid` | 250-300 | L-M | boot install line; R-MII1 |
| `SEAMS_*` label arrays copying row names (same 5 files) | look rows up by group in `native_seams` | 150-200 | M | boot install line |
| `mii_editor_bridge/lifecycle.rs` vs `ldn_lifecycle.rs` (48 of 95 lines identical) | one owner tracker table | 60-80 | L-M | set D |
| 105 hand-written pointer-guard lines; private `pointer` helpers at `mii_ordinary_native.rs:192`, `cosmetic_resume.rs:250`, `native_default.rs:81` | `native_fields::child`/`chain` (`native_fields.rs:83,95`) | 40-70 | L-M | R-MII1 |
| `settings_resume.rs:134-173` vs `202-242` (`Policy` / `WrapLatch`) | one bounded latch | ~35 | L | host tests |
| `connected_native.rs:59-73` vs `84-106`, `connected.rs:131-153` | one owner type with a kind | 20-25 | L | set D |
| `mii_ordinary_player.rs` vs `mii_ordinary_cpu.rs`: 431 of 1,095 lines in identical blocks (`432-448` = cpu `538-554`) | one role adapter generic over Player/CPU | 150-300 | H | R-MII1 A/B, set D |
| `mii_saved_alpha.rs:193-730` fallback owner (runs only when the ordinary gate fails) | behaviour decision | ≤350 | H | R-MII1 |
| `mii_ldn_qualification.rs` (565, probe feature) | retire if the LDN layout facts are recorded | 565 | L | none |
| [Shipped editor input snapshots wrote about 13.8 KB per session](#size-plan--editor-snapshots) | behind `diag` or delete (pkg6b decision 1) | ~70 | L | none |

Area: **0.85-1.05k** without a ruling (plus the 560 never-compiled lines in §1); **2.5-2.9k** with the rulings.

<a id="size-plan--seams-src-model-seams-6-690-src-runtime-seams-648-preload-2-338-lobby-2-366-1-591-stages-829"></a>

#### seams (`src/model/seams` 6,690, `src/runtime/seams` 648), preload (2,338), lobby (2,366 + 1,591), stages (829)

| Finding | Change | Saves | Risk | Rows |
| --- | --- | ---: | --- | --- |
| `model/guard.rs` (237) repeats the rules of `native_seams/mod.rs:38-130,546-837` (same enums, `guard_outcome`, `outcome_for`) | `native_seams` re-exports `guard` | 200-230 | L | s6-02/03, pkg1 boot install line |
| [644 seam rows plus 49 image rows, including unchecked entries](#size-plan--unchecked-seam-rows) | delete the unchecked rows (keep `needle.set_cpu_controls`, `training.handle_effect`) | 1,300-1,550 | L-M | s6-02/03; owned-ELF host test |
| [Unread fields on all 693 rows and empty notes on 308](#size-plan--unread-seam-fields) | drop the fields | in the next row | L | owned-ELF host test |
| [Redundant legacy shadow comparisons in authoritative groups](#size-plan--legacy-shadow-comparisons) | delete | 130-165 | L | s6-03 `seams_shadow_diff=[]` |
| [INI-overlay functions lacked non-test callers after D7](#size-plan--ini-overlay-functions) | delete | 250-300 | L | s6-01 |
| Same file: 8 on/off switches repeated across struct, defaults, parse, render, menu | one key/bit/field table, INI byte-identical | 150-250 | M | s6-01, INI first-boot row (none yet) |
| Same file `write_new`/`replace_live`/`recover_previous` (L1200-1288) vs `store::write_replace` | one writer (backup name changes) | ~40 | M | s6-01 |
| [Repeated notifications, hash reads, item gates and span queries](#size-plan--repeated-lookups) | one each | 55-70 | L | lobby rows (none run) |
| [Site types overlapped across two tables](#size-plan--overlapping-site-types) | one site type | 150-300 | H | every install-line row; defer |
| [Only unreachable adapters used the lobby automation helper](#size-plan--unreachable-lobby-adapters) | park with them (L1) | ~1,500 | L | none |

Also found: `practice_stage_request::init` runs twice (`lobby/mod.rs:27`, `practice_stage_observer.rs:293`) and logs
`ui_wired=true` while no ticket is ever queued (defect, reported to the lead, not a size item).
Area: **1.9-2.3k** without a ruling; **+1.5k** parking, **+2.5k** compaction (§3).

<a id="size-plan--macros-recording-src-runtime-macros-3-761-src-model-macros-3-515-src-model-macro_ui-2-137-smash-match-data-3-735"></a>

#### macros / recording (`src/runtime/macros` 3,761, `src/model/macros` 3,515, `src/model/macro_ui` 2,137), smash-match-data (3,735)

| Finding | Change | Saves | Risk | Rows |
| --- | --- | ---: | --- | --- |
| `model/macros/ui_anchor.rs` (122) and `model/macro_ui/replay_panel.rs` (137): no user (M) | delete (pkg4 manifest edit) | 259 | L | none |
| Named playback and slot transfer | removed in F7b; the panel, prompts and replay capture remain | measured in F7b | L | F7b-1–9 |
| Two copies of the fork's practice/lobby/save-state facade vs `runtime/lobby/training/{practice.rs,lobby.rs,mod.rs}` | `port` uses the lobby facade (inside the pkg4 wiring) | ~100 | M-H | s4-03/04, lobby rows |
| `InputFrame` (`macros/recording.rs:29`) = `MappedInputs` (`input_delay.rs:17`) | one type, JSON shape identical; F7b removed `StartSnapshot`/`Origin` | 60-70 | L-M | s4-01/02, s4-03/04 |
| [Duplicate constants, neutral checks, macro source and sync helpers](#size-plan--repeated-macro-helpers) | one each | ~40 | L | s4-01 |
| Diag hook counters written 3× (`features/cpu.rs:117-178`, `states.rs:167-241`, `recorder.rs:91-145`) | one counter type beside `hook_stats::ENTRIES` | 120-150 | L | s2-02..04 hook budget, s4-01 |
| smash-match-data `capture.rs` (360), `clip.rs` (417), `validate.rs:1031-1141`, `Capture::load/save`: used only as a test oracle (`shared_capture.rs:482,734,769`) | vendor only what is used (lead ruling: the crate is carried unchanged) | 850-950 | M | s4-03/04 |

Area: **0.37k** without a ruling; **1.5k** with the smash-match-data ruling.

<a id="size-plan--features-menu-ui-model-core-packages-1-5-clean-room"></a>

#### features, menu, ui, model core (packages 1-5, clean room)

| Finding | Change | Saves | Risk | Rows |
| --- | --- | ---: | --- | --- |
| [Consolidated CPU parameters, pane bounds, hashes, constant tables and storage writes](#size-plan--core-consolidations) | | 129 (M) | L | s2-02..04, s3-03 (constants by name) |
| [Seven comparison loops repeated UI and u4 checks](#size-plan--repeated-row-comparisons) | one table registry in install-line order, `usable(name)`, `mark(name)` | 100-150 | M (`blocked_by` order) | tr105, s2-02, s4-01, w-01, u4-01, r2-01/02 |
| Install-and-mark lists `boot.rs:197-216,383-464,488-502`, `state_boot.rs:37-83` (15 names twice) | one table | 40-60 | L-M | s2-02..04, s3-03..10 |
| Game-constant mirrors `features/cpu.rs:49-92,373-411`, `states.rs:87-101,381-411` (one static per constant) | one array indexed by `C` | 70-90 | L-M | s2-02..04, s3-03 |
| `HookId` + `ALL` + `name()` (`hook_stats.rs:26-119`), `Feature` + `name()` (`sites.rs:20-117`) | one `macro_rules!` each, like `cpu/ids.rs` | 50-70 | L | s4-01 hook counts |
| Constants resolved twice per boot (`features/cpu.rs:360-446`, `states.rs:368-422`; 18 names in both lists) | one list (enum clash to solve) | 60-100 | M | s2-02..04, s3-03 |
| Diag probes that served (§4): `runtime/ui/probe.rs` 132, `ui/texture_probe.rs` 124 | retire | 256 | L | one diag boot |

Area: **0.6k** without the site compaction; **2.3-2.6k** more with it (§3).

<a id="size-plan--3-data-tables-written-long-lead-ruling-2026-11-02-no-layout-only-reductions"></a>

### 3. Data tables written long (lead ruling 2026-11-02: no layout-only reductions)

| November 2 ruling | Evidence |
| --- | --- |
| Accepted reductions | Table-driven changes that removed implementation code. Default rustfmt layout set the counting basis. |
| Excluded | Layout-only reductions. |
| Formatting finding | `cargo fmt --all -- --check -l` identified 444 of 508 tracked Rust files. |
| Scratch measurement | Rustfmt on a `git archive` of `e79019b`: 136,814 sloc3 versus 106,785 committed. |
| Historical tool | Dropped that `--normalize` report mode because the primary’s sloc4 did not adopt it. |

| Layout proposal | November 2 evidence |
| --- | --- |
| Input | 239 `sites*.rs` rows, 13 lines each, four guard lines of 16 bytes each. |
| Seams | Five to ten lines per row. |
| Proposed form | One header line and one line containing all 64 guard bytes. |
| Compliance | `guard: [0x.. ×64]` matched `compliance.py`. Byte strings and `include_bytes!` failed. |
| Example / generators | `rows_a1.rs:11-15`; update `tools/gen_sites_*.py` to emit the form. |
| Data parity | Owned-ELF host tests compared compiled tables. |
| Estimated layout difference | 2,390–2,630 site lines and about 2,500 seam lines after deletions. |
| Ruling | Lead and operator excluded layout-only reductions. |

<a id="size-plan--4-dead-code-probes-doubled-implementations"></a>

### 4. Dead code, probes, doubled implementations

| Warnings at `bf836c8` | Evidence |
| --- | --- |
| Release warnings | 103 in integrator-3’s `/tmp/integrator3/warns-release2.tsv`. |
| Later retirement | F7 replaced `replay_preview.rs` with Recording playback. |

| File (count) | Class |
| --- | --- |
| `runtime/macros/port.rs` (24), `named_macros.rs` (4), `replay_preview.rs` (7) | [Pkg4 and screen wiring remained outstanding; several helpers were unused](#size-plan--pending-wiring) |
| `runtime/ui/menu_app.rs` (11) | waiting: ui-screens (Mii list screen) |
| `mii_editor_bridge/editor_origin.rs` (16), `mii_saved_alpha.rs` (11), `mii_catalog_request.rs` (3) | waiting: pkg5/ui-screens Mii list screen (`request`, `menu_ready`) |
| `mii_replay.rs` (4) | wiring gap: macros call the `port.rs` stubs instead; wiring removes ~60 stub lines |
| `mii_auto_init.rs` (11) | [Caller search found callers for every item. Warning text needed review.](#size-plan--mii-initialization-warnings) |
| `runtime/lobby/training/lobby.rs` (5), `practice_stage_request.rs` (4), `game_mode.rs` (2) | waiting: the waiting-room UI (`set_ui_ready`, stage picker); ~630 lines ship inert until then |
| `roster/character_items.rs` (1) | probe-only (`turnip_probe_idle`) |

**Probes** (ask each owner through the lead before retiring; none is in the release NRO):

| Probe (lines, gate) | Served? | Owner |
| --- | --- | --- |
| `source_capacity_probe` 1,634, `source_owner_probe` 489, `source_ldn_capacity_probe` 453, `source_lifecycle_probe` 434 (`source-*-probe`) | headers call them one-shot entry-7/entry-2/LDN experiments; replaced by the shipped `managed-*-source-alpha` features | pkg6 (D11) |
| `body_bank_probe` 234 (`native-body-bank-probe`, `compile_error!` with the shipped bank) | yes | pkg6 |
| `mii_origin_observer` 617, `mii_pool_observer` 276 | the Mii features they informed ship | pkg6 |
| `practice_trace_core` 721 + `practice_trace.rs` (362 gated), thread `tr-practice-trace` | duplicates package 1's `diag_log` ring; its overlay and UI bridge are not carried | pkg6, stack (`slots!` line goes with it) |
| `native_unlink_probe` 276 | the shipped `native-unlink-guard` handles the crash (loses forensics) | pkg6 |
| `hang_trace` 106 | keep its stubs | pkg6 |
| `mii_ldn_qualification` 565 | no in-repo receipt of the recorded facts | pkg6 |
| pkg6's new `mii_fullconfig_probe`, `mii_identity_probe`, `mii_saved_file_probe`, `item_state_probe`, `turnip_face_caller_probe` (+2.4k pending) | being added now | pkg6 |
| `runtime/ui/probe.rs` 132 (diag), thread `tr-probe` | yes (s5-12, ARCropolis route) | pkg5 / stack (`slots!`) |
| `runtime/ui/texture_probe.rs` 124 (`texture-probe`) | yes (u4-01..03; the stage thumbnails use the same calls) | u4 / sync touches it |
| `runtime/sync_probe.rs` 27 (diag) | Eden yes (s6-01); the row stays on the real-console list | pkg6: keep until the console row |
| `seam_drill` 241 + `drill.rs` 57 (`seam-drill`) | no: hook sharing has no Eden run yet | keep |
| `runtime/macros/diag.rs` 186 (diag) | drives s4-03/04 until the screens re-prove them | keep |
| `lobby_stage_probe` 198, `scope_gate`, `process_memory`, `observe`, `preload_metrics`, `mii_source_provenance`, `report.rs` | product code despite the names | keep |

Stack stream note (2026-11-02): the diag threads `tr-state-diag`, `tr-diag-reload`, `tr-probe`, `tr-macro-diag`,
`tr-practice-trace` have static stacks in `runtime/threads.rs` `slots!`; retiring a probe module removes its slot line
too (`tests/thread_stacks.rs`).

| Area | Duplicated implementations identified |
| --- | --- |
| Seams | `model/guard.rs` and `native_seams` rules. `sites::Site` and seam `Site`. |
| Fingerprints | Legacy words and seam rows across roster, Mii and lobby. Two Mii fingerprint schemes. |
| Constants | CPU/state constant tables and CRC32 were consolidated in this stream. |
| Storage | Config INI writer and `store::write_replace`. |
| Lookups | Three `span` implementations, two item gates, three catalog hashes. |
| Interfaces | `port.rs` and lobby facades. `InputFrame` and `MappedInputs`. |
| Instrumentation | Three diagnostic hook counters and duplicate native wrappers in roster probes. |

<a id="size-plan--5-totals-and-the-target"></a>

### 5. Totals and the target

| Step | Saves (E) | Count after (from ~114.0k with every stream merged) |
| --- | ---: | ---: |
| [A. Removing unreachable implementations within existing ownership](#size-plan--step-a-savings) | 5.6-6.8k | ~107-108k |
| [B. Retiring probes and applying D11 rulings](#size-plan--step-b-savings) | ~9-11k | ~97-99k |
| [C. Changing behavior or structure under separate rulings](#size-plan--step-c-savings) | ~2-2.5k | ~95-96k |
| D. Data tables written compactly (§3) | ruled out (layout only) | — |
| E. Counter correction (§1), applied to both trees | ~3.7k (M: 3,738 at `7bb86fd`) | reported as "test-fixed" |

| November 2 projection | Basis |
| --- | --- |
| Steps A–C | Approximately 95–96k, the lead’s reported target, roughly 15% below the fork’s 112.5k. |
| 90k | Required counter correction E, which would lower the fork count too. |
| Pending wiring and screens | Expected to raise the count. Any addition above budget required a recorded raise. |

Done so far (as measured): `7bb86fd` 106,983 → stream head 106,785 on `566780a` (107,470): the lint-allow roots and
one-implementation wins (129) and three never-compiled files (556).

<a id="size-plan--seam-rows-phase-b-slice-1-lead-override-of-d6-2026-11-02"></a>

### Seam rows (Phase B slice 1, lead override of D6, 2026-11-02)

| Seam census rule | Evidence counted by `python3 -I tools/seam_census.py <tree>` |
| --- | --- |
| Source | Rust files outside the rows and test code. |
| References | Literal row label, hexadecimal offset or needle constant `SITE_<NAME>`. |
| Other rows | References via `OwnHook` or `EntryOrBody`. |
| Conservative retention | Even incidental offset matches counted, including another table guarding the same function. |
| Baseline below | Stream/slim `7a9bd4a` held 693 rows. Only rows without any consumer qualified for removal. |

| Group | Rows | Consumed | Deleted |
| --- | ---: | ---: | ---: |
| `console_load_probe` (images) | 28 | 0 | 28 |
| `console_map_probe` (images + rows_a5) | 7 | 0 | 7 |
| `hitbox_visualizer` | 3 | 0 here, read on stream/hitbox | 0 |
| `automation_helper_native` | 18 | 7 | 11 (read only by the deleted, never-compiled `automation_helper/native.rs`) |
| `kirby_resource_probe` | 10 | 9 | 1 |
| `mii_preview_observer` | 15 | 4 | 11 |
| `mii_quit_observer` | 19 | 5 | 14 |
| `training` (rows_a3 + images) | 82 | 72 | 10 |

The same census over the work, pkg6, pkg4, ui-screens, hitbox, deluxe and f6 trees found no consumer for the 71 rows
outside `automation_helper_native`. After the slice the census shows unconsumed rows only in `hitbox_visualizer`. Count:
109,812 → 109,108 (−704). Eden: one boot row checks the install line (`seams=`/`seams_unchecked=` updated,
`result=installed`, nothing named missing).

<a id="size-plan--smash-match-data-phase-b-lead-ruling-2026-11-02"></a>

### smash-match-data (Phase B, lead ruling 2026-11-02)

| November 2 disposition | API or evidence |
| --- | --- |
| Kept for the plugin | `archive` writer, manifest and timeline reader. `model`. `validate_header` and `StreamChecker` in `validate`. |
| Test-only destination | `crates/smash-match-data/tests/oracle`. Other consumers could also use `smash-match-data-design` at `faac04e`. |
| `capture.rs` | `Recorder`, `RecorderOptions`, `Finished`, `CaptureError`. |
| `clip.rs` | `clip_context*`, `clip_events*`, `selection_end*`, `ClipContext`, `Resolved`, `SelectAt`, `Capture` clip methods as `CaptureClip`. |
| `whole.rs`, `CaptureExt` | `Capture::load`, `Capture::save`, `validate_capture`. |
| External format | Parity tests compared the writer with `Recorder`. Golden files pinned published bytes. |
| Count | 109,108 → 108,159 (−949). |

<a id="size-plan--6-phase-b-order-after-stream-sync-and-stream-stack-are-merged"></a>

### 6. Phase B order (after stream/sync and stream/stack are merged)

| November 2 sequence | Required evidence |
| --- | --- |
| 1. Unreachable files and uncompilable probes | Carry-manifest edits. No Eden rerun because the NRO would be identical. |
| 2. Seams | Consolidate `guard.rs`, remove unchecked rows, unread fields and legacy shadow tables. |
| Seam gates | Boot install lines (s6-02/03 shape, r2-01/02, u4-01, tr105, s2-02) and owned-ELF tests. |
| 3. Probes | Owners’ answers, diagnostic boot, removal of matching `slots!` lines. |
| 4. Clean-room core | Registry, install table, constant mirrors, hook counters (s2-02..04, s3-03..10, s4-01, w-01). |
| 5. Macros | After pkg4 wiring, run s4-01..04 and WIRE. |
| 6. Roster and Mii | At planning time neither had a clean-room Eden row. |
| Baseline and reruns | First package 6 sets A/B/D, then R-ID, R-PC, R-MII1 and D after each slice. |

<a id="size-plan--capacity-probe-retirement-1c0104e-2026-11-02"></a>

### Capacity probe retirement — 1c0104e (2026-11-02)

| Capacity retirement at `1c0104e` | November 2 measurement |
| --- | --- |
| Change | pkg6 manifest transforms retired approved capacity/lifecycle experiments and eight empty aliases. |
| Raw sloc4 | 101,224, excluding 1,118 simulator lines; −3,126 from 104,350, −5,728 from `ebfcb98` (106,952). |
| Raw budget | 105,600 included 4,318 pending merge lines, under the same counting rule. |
| Normalized tool | `sloc-fmt.ps1`, rustfmt 1.9.0-stable `48a229cea`, zero failures. |
| Components | 125,979 src + 3,434 crates + 167 tools = 129,580. |
| Excluding tools | 129,413: −3,928 from 133,341 and −6,656 from normalized base 136,069. |
| Fork comparison | 131,081 excluding tools. RC ratchet improved. Next-version 15% reduction remained outstanding. |

<a id="size-plan--served-mii-probes-and-helper-scopes-2026-11-02"></a>

### Served Mii probes and helper scopes (2026-11-02)

| Mii slice | Result recorded November 2 |
| --- | --- |
| Raw sloc4 | 98,956 = 95,449 src + 3,356 crates + 151 tools. Down 2,268 from `1c0104e`. |
| Excluded counts | 1,118 simulator and 28,416 test lines. |
| Experiments | Removed six files and seven features. Kept shared native helpers and exact-write-set regression. |
| Suppressions | Removed twelve through consumer feature/test scopes. Added none. |
| Stage service | Renamed two modules for shipped behavior and retained historical seam metadata. |
| Budget | 98,956 + 4,318 pending, rounded to 103,300 from 105,600. |
| Acceptance at that checkpoint | Normalized measurement and host/Switch gates outstanding. Shared runtime failures remained open. |

| November 2 follow-up | Result |
| --- | --- |
| `f07bda2` normalized | 126,710 including 167 tool lines, 126,543 excluding them. |
| Exclusions and formatter | Simulator 2,102 and tests 44,339 excluded. rustfmt 1.9.0-stable `48a229cea`, zero failures (`sloc-fmt.ps1`). |
| Reduction | 2,870 from `1c0104e`, 9,526 from 136,069. RC ratchet improved; next-version target remained unmet. |
| Helper scopes | Feature/test scopes replaced the last module-wide dead_code allowance. |
| Intermediate count | Raw 98,962 (+6 versus `f07bda2`), ceiling 103,300. No additional reduction claimed. |
| Intermediate status | Normalization, compiler gates and five large-enum rows pending. Two hos-lock allowances approved. |
| `10a8232` normalized | 126,725 including 167 tools, 126,558 excluding. +15 versus `f07bda2`, −9,511 versus 136,069. |
| Formatter and budget | rustfmt 1.9.0-stable `48a229cea`, zero failures. Feature scoping required no budget increase. |

<a id="size-plan--hook-identifier-declaration-2026-11-02"></a>

### Hook identifier declaration (2026-11-02)

| Hook mapping | Result at `8f762e1` |
| --- | --- |
| Declaration | One table in `model/hook_stats.rs` defined 40 HookId variants, ALL ordering and report labels. |
| Review | Removed three maintained mappings. Source comparison preserved ordinals, labels and variant comments. |
| Unaffected code | Allocator, scope, counters and formatter. |
| Raw sloc4 | 98,885 (−77 from `61b0aab`). Ceiling 103,300. |
| Normalized excluding tools | 126,472 (−77). All five compiler gates passed. |
| Resident bytes | Release 3,444,736 (+4,096 versus `61b0aab`). Diagnostic 3,784,704, unchanged. |
| Runtime allocation failures | Still open at this checkpoint. |

<a id="size-plan--feature-declaration-2026-11-02"></a>

### Feature declaration (2026-11-02)

| Feature declaration at `a6e4d07` | November 2 measurement |
| --- | --- |
| Declaration | All 28 Feature variants, names and variant documentation shared one declaration. |
| Parity | Site rows, bytes, rules and later definitions remained byte-identical. |
| Counts | 98,865 raw (−20 from `8f762e1`), 126,452 normalized excluding tools (−20). |
| Compiler gates | All five passed. |
| Resident bytes | Release 3,444,736, diagnostic 3,784,704. Both matched `8f762e1`. |
| Ceiling | 98,865 + 4,318 pending = 103,183, rounded to 103,200. |

<a id="size-plan--keyboard-value-and-validation-root-78f440d"></a>

### Keyboard value and validation root (78f440d)

| Keyboard checkpoint | Measurement or implementation |
| --- | --- |
| `78f440d` | Native adapter and host probes shared the actual value/callback code. |
| Inline values | Checked byte metadata bounded inline values. |
| Allocation | Fixed UTF-16 encoding and streaming scalar/cursor validation removed temporary Vec/String allocations. |
| Counts | Raw 98,877 (+12 versus `a6e4d07`), normalized 126,470 excluding tools (+18). |
| Formatter | rustfmt 1.9.0-stable `48a229ceae`, zero failures. |
| Budget | 98,877 + 4,318 pending = 103,195 against 103,200. Native-stack acceptance remained separate. |
| `5b40bac` | Restored CPU helper arm’s feature condition. Keyboard Signals used existing hos_lock storage. |
| Counts | Raw 98,879 (+2 versus `78f440d`), normalized 126,463 excluding tools (−7). Zero rustfmt failures. |
| Combined change versus `a6e4d07` | +14 raw / +11 normalized. Including pending: 103,197 against 103,200. |
| Acceptance | Compiled and native acceptance tracked separately. No budget increase. |

<a id="size-plan--normalized-gate-migration-operator-lead23-11-2026-11-02"></a>

#### Normalized gate migration (operator/lead23:11, 2026-11-02)

| Normalized gate adopted November 2 | Rule |
| --- | --- |
| Commands | `tools/sloc.py` CLI and `tools/size_gate.py` measured rustfmt-normalized sloc4. |
| Formatter | rustfmt 1.9.0-stable `48a229ceae`, toolchain 1.98.1, edition 2021, matching `sloc-fmt.ps1`. |
| Isolation | Formatted each file via stdin with `skip_children=true`. Source files and module children remained untouched. |
| Failure | Formatting errors or version mismatch failed the gate without a raw fallback. |
| Raw history | The existing `measure()` remained available for raw historical evidence. |
| Total exclusions | Scripts/tools, tests, simulator-only code and vendored code. |

| November 2 budget transition | Amount or decision |
| --- | --- |
| Authorized normalized ceiling | `ebfcb98` 136,069. Fork M5q comparator 131,081. |
| Following version | About 111,400, 15% below fork. RC followed the ratchet. |
| Raw history | Retained separately. Neither the 103,200 ceiling nor 4,318 pending lines represented normalized units. |
| Integration | Recount after combining Setup/UI/F9. No new normalized pending allowance. |
| L2 decision | Kept both approved hos-lock `declare_interior_mutable_const` allowances. |
| Slim working candidate | 130,570 = 127,132 src + 3,438 crates. Final exact-source measurement and gates remained outstanding. |

<a id="size-plan--diag-only-code-is-not-counted-operator-decision-2026-11-14"></a>

#### Diag-only code is not counted (operator decision, 2026-11-14)

| November 14 diagnostic counting rule | Details at adoption |
| --- | --- |
| Excluded | Code compiled only with `diag` or features enabling it through Cargo.toml. |
| Feature closure | `seam-drill`, `arc-probe`, `console-practice-trace`, `hang-trace` (hos-lock `owners`), `plugin-heap-meter`, `named-input-trace`. |
| Predicate evaluation | `tools/sloc.py` used three-valued evaluation of `#[cfg(...)]` against default release features. |
| Counted | Unknown predicates (`target_os`, `deluxe-sdk`), `cfg!`, `cfg_attr` and macro bodies. |
| Extent removed | Items, statements, fields, variants, arms, parameters and files reached exclusively by diagnostic `mod` declarations. |
| Initial compatibility | Existing test/simulator rules gave identical results on code without diagnostic sections. |
| Gate exit 2 | Remaining code failed parsing or a dropped item shared a source line. |
| Measured by `tools/sloc.py` | rc.13 `ed0d7a35` 147,835 → 141,975. `c5191271` 147,798 → 141,917. |
| rc.14 integration | `8d129d20` 149,316 → 143,399. Ceiling remained 147,921. |
| Tests | `DiagOnly` in `tools/test_size_gate.py`. |

| Later on November 14 | Test and simulator rule |
| --- | --- |
| Scope | Primary applied rc.14 scope decision to the same evaluator. |
| Earlier matching | sloc4 recognized three attribute spellings and stopped at the first semicolon or brace. |
| Partial counts | `const X: [T; N] = …`, `-> [T; N] {` and `use a::{b};`. |
| Macro handling | Examined `macro_rules!` templates as code. Macro call bodies stayed opaque. |
| Measured reduction | 648 lines across 44 files in each measured tree. |
| rc.13 | `ed0d7a35` 141,975 → 141,327. `c5191271` 141,917 → 141,269. |
| rc.14 integration | `12dee86d` 143,828 → 143,180. |
| Tests | `TestAndSimulator` in `tools/test_size_gate.py`. |

<a id="size-plan--rc-15-lucas112-native-charge-release-2026-11-16"></a>

#### rc.15 Lucas112 native charge release (2026-11-16)

| rc.15 checkpoint | Canonical Surface gate and decision |
| --- | --- |
| Lucas Frozen | `f02e9790` 150,945 → `4ff7cf28` 150,963 normalized (+18). |
| Approval | Primary approved +18 at 22:48Z under the rc.15 size ruling. |
| Gate | Red against 148,194. Counter, formatter and `size-budget.toml` stayed unchanged. |
| Final ceiling | Operator-approved final integration owned the adjustment. This checkpoint covered only the Lucas allowance. |
| Lucas evidence | `lucas-gate-4ff7cf28-surface/BUDGET-ENTRY.md` and retained canonical gate receipts. |
| Olimar40 and Inkling102 | `3fd57e54` 150,991 (+28 over Lucas, +46 over `f02e9790`). |
| Builds | Clippy and no-default-features clean. Release and diagnostic builds succeeded. |
| Acceptance at checkpoint | Runtime acceptance was pending. Same final-total approval, same ceiling. |
| Combined evidence | `combined-gate-3fd57e54-surface/SUMMARY.md`. |

<a id="size-plan--rc-15-dedede99-real-up-special-entry-2026-11-16"></a>

#### rc.15 Dedede99 real up-special entry (2026-11-16)

| Dedede99 checkpoint | November 16 record |
| --- | --- |
| Canonical Surface count | `3fd57e54` 150,991 → `bd7f4d90` 150,992 (+1). |
| Budget | Same ceiling and evaluator, under rc.15 final-total size approval. |
| Acceptance at checkpoint | Primary review and Eden contact/escape proof remained pending. |
| Evidence | `dedede99-gate-bd7f4d90-surface/SUMMARY.md`, `BUDGET-ENTRY.md` and retained `compliance-v2.log`. |
| Trace correction | Test references corrected without changing product or gate. |

<a id="size-plan--rc-15-ice-climbers113-114-and-zss108-2026-11-17"></a>

#### rc.15 Ice Climbers113/114 and ZSS108 (2026-11-17)

| Ice Climbers and Flip stomp checkpoint | November 17 record |
| --- | --- |
| Canonical gate | `2e36e0bd` 150,992, equal to `bd7f4d90`. |
| Review | Primary approved the size with the IC113/114 Opus pass. |
| Flip stomp108 | Reused SPECIAL_LW admission and native Bury observation; canonical count was pending at this checkpoint. |
| Budget | Counter and ceiling unchanged. Final integration owned the approved final-total adjustment. |

<a id="size-plan--rc15-mash-out-dedede99-native-cancel-continuation"></a>

#### rc15 Mash out Dedede99 native cancel continuation

| Dedede99 continuation | rc.15 checkpoint |
| --- | --- |
| Observation | Native landing hit interrupted real Bury. |
| Adjustment | Applied native SPECIAL once in Held after CPU hitstop via existing followed/controls/drive/revoke. |
| Scope | Kept existing reader, hook, model, clock, guard and constants. |
| Held probes | Seed, stop, one edge, escape and other plan. |
| Budget at checkpoint | Canonical normalized count pending under rc.15 final-total approval. Ceiling unchanged. |
| Evidence | Native evidence and RED/GREEN in `dedede99-interruption` lane artifacts. |

<a id="size-plan--unmerged-stream-contributions"></a>

##### Unmerged stream contributions

- M: pkg6 +2,416 (mostly new probes: `mii_fullconfig_probe`, `mii_identity_probe`, `mii_saved_file_probe`, `item_state_probe`, `turnip_face_caller_probe`), hitbox +1,272, u4b +990, ui-screens +721, stack +640, sync +495 (`crates/hos-lock` 443), f6 +481, deluxe +83, pkg4 −37

<a id="size-plan--legacy-roster-fingerprints"></a>

##### Legacy roster fingerprints

- Old word fingerprints next to the 64-byte seam guards: `character_roster.rs:2304-2383`, `native_body_bank.rs:3813-3927`, `character_lifecycle.rs:27-65,149-158`; the groups are `AUTHORITATIVE` (`seam_install.rs:55-86`) and `gate()` (`seam_install.rs:298`) ignores the legacy result

<a id="size-plan--repeated-native-wrappers"></a>

##### Repeated native wrappers

- Same native wrapped twice: 0x17e2880/0x17e8890/0x353ea90 in `native_source.rs:161-169` and `source_owner_probe.rs:213-222`; `OWNER_TABLE` 0x5324680 in 12 files; `ACQUIRE_KIND/RELEASE_KIND` in `native_body_bank.rs:245-252` and `r_handle.rs:38-40`

<a id="size-plan--editor-snapshots"></a>

##### Editor snapshots

- Editor input-region snapshots in the shipped build (`mii_editor_bridge/native.rs:404-459`, `mod.rs:57,81,94-100`, `connected.rs:199-234`; ~13.8 KB per session to `mii-editor-last.json`)

<a id="size-plan--unchecked-seam-rows"></a>

##### Unchecked seam rows

- 644 seam rows (M: a1 94, a2 281, a3 82, a4 42, a5 127, identity 18) + 49 image rows; rows with no gate/site call and no outside reference (M): `training` 77 of 79, mii_quit 19, mii_preview 15, mii_fullconfig 14, kirby_resource 10, … and 37 image rows of modules not in the clean room

<a id="size-plan--unread-seam-fields"></a>

##### Unread seam fields

- Row fields never read (M): `scopes` (`Scopes::ALL` on all 693), `read_only`, `per_frame`; `note` empty on 308

<a id="size-plan--legacy-shadow-comparisons"></a>

##### Legacy shadow comparisons

- Shadow-compare `legacy` word tables though every group is authoritative: `lobby_stage_probe` ~45, `practice_stage_observer` ~50, `practice_stage_request_words.rs` ~22, rotation ~12, `game_mode` ~6, installer shadow branch ~30

<a id="size-plan--ini-overlay-functions"></a>

##### INI overlay functions

- `cycle_preload_config.rs`: INI-overlay code with no non-test caller since D7 (`adopt_unset` L641, `Unset` L573, `apply_settings` L778, `pack`/`unpack` L459-542, 8 `*_enabled`)

<a id="size-plan--repeated-lookups"></a>

##### Repeated lookups

- Small doubles: three notification fns `training/mod.rs:122-145`; stage-catalog hash read 3×; item gate 2× (`practice_stage_request.rs:171`, `practice_stage_observer.rs:86`); QueryMemory `span` 3× (request, `effect_hook_chain`, `mii_ordinary_native`)

<a id="size-plan--overlapping-site-types"></a>

##### Overlapping site types

- `sites::Site` vs `native_seams::Site`: 69 of 82 rows_a3 rows and 3 hazard rows on the same offsets as `sites*.rs` rows

<a id="size-plan--unreachable-lobby-adapters"></a>

##### Unreachable lobby adapters

- Lobby automation helper pure half (adapter, control, flow, popup, pulse 1,379) used only by the never-compiled `native.rs`/`plugin.rs`

<a id="size-plan--repeated-macro-helpers"></a>

##### Repeated macro helpers

- Constants and `is_neutral` 2× (`model/recording.rs:30-55` vs `macro_panel.rs:16-76,1661`); `MacroSource` 2× (`port.rs:141`, `compat/menu.rs:35`); sync helper 2× (`port.rs:6-23`, `compat/sync.rs`)

<a id="size-plan--core-consolidations"></a>

##### Core consolidations

- **Done in this stream:** six `too_many_arguments` in `model/cpu` (`mash::Step`, `ledge::TermQuery`, `FrameFacts`), pane box `[x, y, w, h]`, crc32/hash40 2× (`cpu/hash.rs` now re-exports `model::hash40`), the constant-table implementation 2× (`cpu/ids.rs`, `state/ids.rs`: one `game_constants!`), `Store::write_atomic` = `write_replace`

<a id="size-plan--repeated-row-comparisons"></a>

##### Repeated row comparisons

- "Compare every row" loop 7× (`boot.rs:127-154`, `state_boot.rs:15-21`, `p1/mod.rs:27-29`, `ui/mod.rs:100-103`, `ui/texture.rs:76`); UI and u4 rows compared twice

<a id="size-plan--pending-wiring"></a>

##### Pending wiring

- waiting: stream/pkg4 wiring and ui-screens (the roster cycle now exists, so the `port.rs:72-139` stubs can go when wired); `Legacy`/`has_captured_frame`/`frame_at`/`capture_active`/`playback_active`/`using_frames` are truly dead

<a id="size-plan--mii-initialization-warnings"></a>

##### Mii initialization warnings

- not explained by a caller search (every item has a caller): re-check against the warning text

<a id="size-plan--step-a-savings"></a>

##### Step A savings

- A. Rulings not needed beyond file ownership: never-compiled files 1.1k, the "without a ruling" rows of §2 (roster 0.8-1.4k, mii 0.85-1.05k, seams/lobby 1.9-2.3k, macros 0.37k, core 0.6k incl. 129 done)

<a id="size-plan--step-b-savings"></a>

##### Step B savings

- B. Probe and D11 rulings: roster/Mii probes and observers ~7k, pkg6's new probes 2.4k if they serve their run and retire, Mii D11 fold 0.2k, automation-helper parking 1.5k

<a id="size-plan--step-c-savings"></a>

##### Step C savings

- C. Behaviour/structure rulings: player/cpu generic, saved_alpha fallback, smash-match-data trim, seam-label lookup, one site type, config switch table

<a id="resource-costs"></a>

<a id="resource-costs--resource-costs-of-the-upstream-origin-features"></a>

## Resource costs of the upstream-origin features

| Archive accounting | Basis |
| --- | --- |
| Requested capacity | Twice the largest stock or output archive length in the [recipe table](BUILD.md#layout-archive-recipe-package-5-host-tool). |

<a id="resource-costs--clean-room-archive-accounting"></a>

### Clean-room archive accounting

| Archive accounting | Basis |
| --- | --- |
| Requested capacity | Twice the largest stock or output archive length in the [recipe table](BUILD.md#layout-archive-recipe-package-5-host-tool). |
| Input unit | Use the archive length. The smaller `.tra1` file stores its recipe. |
| Reuse | A worker checks each second and recycles buffers aged at least five seconds. |
| Retained memory | Recycling preserves each buffer’s allocation. |
| [November 3 decision](DECISIONS.md#2026-11-03-rc-archive-working-buffer-residency) | Original working-buffer budget and acceptance requirements. |
| r13 diagnostic estimate | 6,164,480 image bytes + requested capacity = 9,305,864 bytes. |
| Scope | Excludes bookkeeping and other dynamic allocations. Release size and RSS were outside this estimate. |


| Research record, November 1, 2026 | Method |
| --- | --- |
| Sources | Each number cites a file and line, JSON key or command. |
| Read-only image inspection | Ran `/usr/bin/python3 -I /home/intpa/ssbu-mod-playbook/tools/build/nro_size.py <nro>` on existing images. No new builds. |
| NRO resident bytes | NRO header sum of `.text`, `.rodata`, `.data`, `.bss`. |
| Method reference | Playbook `docs/WORKFLOW.md` §4, lines 117–119. |
| Other computation | One `zlib` estimate for two `layout.arc` files. |

Confidence vocabulary: **measured** (a tool output or a log line in the record), **derived** (arithmetic on
measured values; the derivation is given), **stated** (a document asserts the number without evidence in the
record), **not measured**.

<a id="resource-costs--sources-and-path-abbreviations"></a>

### Sources and path abbreviations

| Abbrev. | Path |
| --- | --- |
| `UP` | `/home/intpa/smash-training-lobby/upstream-ref/UltimateTrainingModpack-upstream-97cbe9b` (upstream merge base `97cbe9b`, 2026-10-07, read-only) |
| `STAGE` | `/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/stages/m5i/stage` (fork final tree M5i, content sha `dd766f46…`) |
| `D175` | `/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175` |
| `ART` | `/home/intpa/smash-training-lobby/artifacts` |
| `FOOT` | `ART/footprint-experiments` (`results.json`, one NRO per variant, build logs, `post-build-check.json`) |
| `PLAYBOOK` | `/home/intpa/ssbu-mod-playbook` |

Builds the numbers belong to are named in each row: the NRO sha256 prefix printed by `nro_size.py`, the fork
revision/candidate label, and the date (an install receipt's `at`, a doc's header date, or the card copy's
file mtime when nothing else dates it; mtimes are marked as such).

<a id="resource-costs--table-a-whole-plugin-image-over-time"></a>

### Table A — whole-plugin image over time

| Table A conventions | Reading the measurements |
| --- | --- |
| File and resident | November 1 header readings unless a row gives another source or date. |
| Tool | `nro_size.py` inspected the listed NRO. |
| Resident | Text + rodata + data + bss. |
| Opt-level | Reported build setting. “Not stated” means the record gave none. |

| build | date | file bytes | resident bytes | text / ro / data / bss | opt-level | NRO (sha256 prefix) and source |
| --- | --- | ---: | ---: | --- | --- | --- |
| card build before candidate 21 (earliest in the record; identity not documented) | 2026-10-08 (mtime) | 2,408,448 | 2,437,120 | 1,503,232 / 782,336 / 122,880 / 28,672 | not stated | `0577c959`, `ART/switch-before21/…/libtraining_modpack.nro`; bytes also in `ART/switch-before21/manifest.json` key `bytes` |
| candidate 21 | 2026-10-08 (mtime) | 2,453,504 | 2,482,176 | 1,531,904 / 794,624 / 126,976 / 28,672 | not stated | `d8df82fd`, `ART/switch-candidate21/install/…/libtraining_modpack.nro` |
| 26w | 2026-10-08 (mtime) | 2,523,136 | 2,551,808 | 1,593,344 / 798,720 / 131,072 / 28,672 | not stated | `fcc05274`, `ART/yoga-training-26w/sdmc/…/libtraining_modpack.nro` |
| 29w | 2026-10-08 (mtime) | 2,523,136 | 2,551,808 | 1,593,344 / 798,720 / 131,072 / 28,672 | not stated | `f53969aa`, `ART/github-shared29w/libtraining_modpack.nro` (same bytes at `ART/switch-before32w/`) |
| 32w release (`bb3a486`) | on the card until 2026-10-13 | 2,494,464 | 2,523,136 | 1,548,288 / 819,200 / 126,976 / 28,672 | 3 (`STAGE/docs/REVISION32_SMALL_PROFILE.md:31` lists opt-level 3 as the shipped profile of the same series) | `4610b842`, `ART/switch-before-r29/libtraining_modpack.nro`; table row `REVISION32_SMALL_PROFILE.md:15`; `ART/switch-r31-smoke-1/NOTES.md:212` |
| `libtraining_upstream33w.nro` (name only; header identical to 32w; no document in the record describes it) | 2026-10-09 (mtime) | 2,494,464 | 2,523,136 | 1,548,288 / 819,200 / 126,976 / 28,672 | not stated | `f0685997`, `ART/libtraining_upstream33w.nro` |
| revision 29 (`31c112e`) | 2026-10-13 (mtime of the card copy) | 3,424,256 | 3,452,928 | 2,416,640 / 864,256 / 143,360 / 28,672 | 3 | `5561e630`, `ART/switch-before-r30/libtraining_modpack.nro`; bytes also `STAGE/docs/REVISION29_DRAW_DECOUPLING.md:264` |
| revision 31 trace build (`68b3334`) | 2026-10-13 | 3,379,200 | 3,407,872 | 2,371,584 / 864,256 / 143,360 / 28,672 | 3 | `381046f2`, `ART/switch-before-r32/libtraining_modpack.nro`; `REVISION32_SMALL_PROFILE.md:16,31` |
| r31 sources, `codegen-units=1` | 2026-10-13 | 3,346,432 | 3,375,104 | 2,351,104 / 856,064 / 139,264 / 28,672 | 3 | `7234aa07`, `FOOT/results.json` key `cgu1.segments` |
| r31 sources, `opt-level="s"` | 2026-10-13 | 2,621,440 | 2,650,112 | 1,626,112 / 847,872 / 147,456 / 28,672 | s | `14a91714`, `FOOT/results.json` key `opt-s.segments` |
| r31 sources, `"s"` + cgu1 | 2026-10-13 | 2,646,016 | 2,674,688 | 1,654,784 / 843,776 / 147,456 / 28,672 | s | `2bc20eb5`, `FOOT/results.json` key `opt-s-cgu1.segments` |
| r31 sources, `opt-level="z"` | 2026-10-13 | 2,211,840 | 2,240,512 | 1,200,128 / 856,064 / 155,648 / 28,672 | z | `149b2f74`, `FOOT/results.json` key `opt-z.segments` |
| revision 32 = r31 sources, `"z"` + cgu1 (profile kept to M5i) | 2026-10-13 | 2,174,976 | 2,203,648 | 1,171,456 / 851,968 / 151,552 / 28,672 | z | `65ed74ab`, `FOOT/results.json` key `opt-z-cgu1.segments`; `ART/switch-before-r33/libtraining_modpack.nro`; `REVISION32_SMALL_PROFILE.md:36` |
| revision 33 (analytics/updater/dialog removed) | 2026-10-13 19:07 (mtime of `FOOT/results.json`) | 1,769,472 | 1,798,144 | 1,036,288 / 667,648 / 65,536 / 28,672 | z + cgu1 | [`2b820346`, r33 working tree compiled with the r32 profile](#resource-costs--revision-33-provenance) |
| card build before 7.1.2 | 2026-10-15 | 1,773,568 | 1,802,240 | 1,040,384 / 667,648 / 65,536 / 28,672 | z + cgu1 | `e1260d99`, `ART/switch-input-display-7.1.2/switch-before-7.1.2.nro`; `switch-before-7.1.2.json` key `plugins[libtraining_modpack.nro].bytes` |
| 7.1.2 (`v7.1.2-practice.35w`, input display renderer cache) | 2026-10-15T13:18Z | 1,785,856 | 1,814,528 | 1,040,384 / 671,744 / 73,728 / 28,672 | z + cgu1 | `96605c5b` / `b20f128e` (same header), `ART/switch-input-display-7.1.2/switch-install-receipt.json` keys `installed_bytes`, `version`, `release`; `ART/training-performance-followup-20261015/input-candidate.nro` |
| console-ui84 physical card build | 2026-10-22 (mtime) | 4,268,032 | 4,296,704 | 1,503,232 / 2,580,480 / 184,320 / 28,672 | z + cgu1 | [`cac99b4c`, embedded roster, stage and item images](#resource-costs--console-ui-assets) |
| 7.1.2 experimental full candidate | not dated in the doc | 4,308,992 | not recorded | — | z + cgu1 | `4eb6ce41`, `STAGE/docs/experimental-release.md:95-96` (**stated**; NRO not located) |
| C108 | 2026-10-30T11:04Z (`installed_at` 1793358289) | 4,370,432 | 4,399,104 | 1,720,320 / 2,396,160 / 253,952 / 28,672 | z + cgu1 | `14cc0698`, `D175/switch-collection/c111-install/payload/…/libtraining_modpack.nro`; `D175/c108-card-installed175.json` keys `installed_at`, `plugins[libtraining_modpack.nro].bytes` |
| C111 | 2026-10-30T18:37Z | 4,485,120 | 4,513,792 | 1,765,376 / 2,404,352 / 315,392 / 28,672 | z + cgu1 | `719ee63d`, `D175/switch-collection/c111-speed-feedback/payload/…`; `D175/SWITCH-INSTALL-C111.json` key `at` |
| C114 | 2026-10-30T20:57Z | 4,485,120 | 4,513,792 | 1,765,376 / 2,404,352 / 315,392 / 28,672 | z + cgu1 | `060cc3f0`, `D175/switch-collection/c115-install/payload/…`; `D175/SWITCH-INSTALL-C114.json` keys `at`, `after_snapshot` |
| C115 | 2026-10-30T22:33Z | 4,489,216 | 4,517,888 | 1,769,472 / 2,404,352 / 315,392 / 28,672 | z + cgu1 | `1fed3cdd`, `D175/switch-collection/c116-install/payload/…`; `D175/SWITCH-INSTALL-C115.json` keys `at`, `bytes` |
| C116 = B122 | 2026-10-31T05:06Z | 4,534,272 | 4,562,944 | 1,806,336 / 2,412,544 / 315,392 / 28,672 | z + cgu1 | [`92cef8a7`, NRO header and ELF measurements](#resource-costs--b122-resident-measurements) |
| B123-pre | 2026-10-31 | 4,440,064 | 4,466,496 ("heap") | — | z + cgu1 | `70d1be45`, `D175/C115-CAMPAIGN.md:237` (**stated**; NRO not located) |
| B123-rc (= refactor baseline WP0) | 2026-10-31 | 4,444,160 | 4,472,832 | 1,728,512 / 2,400,256 / 315,392 / 28,672 | z + cgu1 | `f0965879`, `D175/refactor/INTEGRATION.md:15` (**stated** there as a `nro_size.py` result; NRO not located) |
| M5 / M5b / M5c (release candidate) | 2026-11-01 02:5x–03:28Z | 4,759,552 | 4,829,184 | 1,851,392 / 2,498,560 / 409,600 / 69,632 | z + cgu1 | `924d4cee` / `d5e93709` / `6b63a740`, `D175/refactor/stages/m5/release/`, `…/m5b/release/`, `D175/switch-install-c1161/superseded-m5c-libtraining_modpack.nro`; `D175/refactor/RC-SUMMARY.md:13` |
| **M5i = C116.1 (November 1 Switch install)** | 2026-11-01T13:31Z | **4,804,608** | **4,878,336** | 1,871,872 / 2,510,848 / 421,888 / 73,728 | z + cgu1 | [`17708c2c`, installation receipt and B123-rc comparison](#resource-costs--m5i-installation-receipt) |
| M5i diag build (never shipped) | 2026-11-01 | 4,816,896 | 4,988,928 | 1,880,064 / 2,510,848 / 425,984 / 172,032 | z + cgu1 | `c3e4e52e`, `D175/refactor/stages/m5i/release/libtraining_modpack-diag.nro` |
| rc.7 `b3678dfe` (before FLAG 6's fix) | 2026-11-10 | 4,866,048 | 7,008,256 | 2,768,896 / 1,302,528 / 794,624 / 2,142,208 | s + cgu1 | `b73bc202`, release NRO of `b3678dfe`'s source (`tmp-p23/rc7-nt-fix-gates/g2-nro/release/`); measured 2026-11-11 from the NRO header (`/tmp/opencode/lp/nro-resident.py`) |
| **rc.7 final `722525a5`** (= `3c7d43a1`: FLAG 6's four 320 KiB native-service stacks, +1,310,720 `.bss`) | 2026-11-11 | 4,886,528 | **8,339,456** | 2,789,376 / 1,302,528 / 794,624 / 3,452,928 | s + cgu1 | `b6e60bb9` (integration's `rc6-eden/inputs-int/6c33ef53/release/`, byte-identical); NRO header |
| **rc.8 `p23/rc8-waste` `ea4a6de9`** (§6, then §4/§7: two 272 KiB service stacks; `artifacts/phase23/rc8-waste/DESIGN.md`) | 2026-11-11 | 4,886,528 | **7,585,792** (−753,664) | 2,785,280 / 1,302,528 / 798,720 / 2,699,264 | s + cgu1 | `f4f34f35`, `tmp-p23/rc8-waste-gates/g3-nro/release/`; NRO header |
| [rc.8 pass head `9d6c7e02`, integration](#resource-costs--integration-service-stacks) | 2026-11-11 | — | 7,659,520 | — | s + cgu1 | release `4f67766f`; integration's gates (its 2026-11-11 SHIP READY figure) |
| [rc.8 `21527b70`, R6 comparison at opt-level s](#resource-costs--r6-at-level-s) | 2026-11-11 | 4,890,624 | 6,721,536 | 2,781,184 / 1,310,720 / 798,720 / 1,830,912 | s + cgu1 | `63a7d1df`, scratch target `tmp-p23/rc8-rootz-target` (worktree `rc8-r6-wt` at `21527b70`); NRO header |
| [rc.8 `f86771f7`, R3–R6 and R8 at opt-level z](#resource-costs--r6-at-level-z) | 2026-11-11 | 4,149,248 | **5,914,624** (−1,744,896 against `9d6c7e02`; −2,424,832 against the rc.7 final) | 2,011,136 / 1,327,104 / 811,008 / 1,765,376 | z + cgu1 | `82c4cac5`, `tmp-p23/rc8-waste-gates/g5-nro/release/`; NRO header |

Refactor stage trail between B123-rc and M5i (all measured 2026-11-01 from `D175/refactor/stages/<stage>/release/libtraining_modpack.nro`; the per-stage attribution is `D175/refactor/RC-SUMMARY.md:14-15`):

| stage | NRO | file | resident | bss | what the stage added (RC-SUMMARY.md:15) |
| --- | --- | ---: | ---: | ---: | --- |
| M0 | `2f3c4fd8` | 4,481,024 | 4,509,696 | 28,672 | Mii groups (+36,864 vs B123-rc) |
| M0.5 | `1daaeb2a` | 4,489,216 | 4,558,848 | 69,632 | toolchain + diag ring (+49,152; bss +40,960 = the diagnostics ring, `C115-CAMPAIGN.md:355`, `INTEGRATION.md:447`) |
| M1 | `941047fc` | 4,710,400 | 4,780,032 | 69,632 | 608-row seam table with 64 B guards + WP-H meter (+221,184; `INTEGRATION.md:1680-1683`) |
| M2 | `a4a7cf1b` | 4,763,648 | 4,833,280 | 69,632 | +53,248 |
| M2b | `8337fbf4` | 4,780,032 | 4,849,664 | 69,632 | Mii-setup adapter +16,384 |
| M3 | `21528fd4` | 4,710,400 | 4,780,032 | 69,632 | U17 labels-only −81,920, WP4 ledger +12,288 |
| M4 / M4b | `edca4711` / `8f26d206` | 4,722,688 | 4,792,320 | 69,632 | +12,288 / +0 |
| M4c | `e07856ad` | 4,730,880 | 4,800,512 | 69,632 | +8,192 |
| M4d | `7c0ea27c` | 4,755,456 | 4,825,088 | 69,632 | +24,576 |
| M4e / M5 / M5b / M5c | `eb4aa464` / `924d4cee` / `d5e93709` / `6b63a740` | 4,759,552 | 4,829,184 | 69,632 | +4,096 / 0 / 0 / 0 |
| M5d | `04d125ce` | 4,763,648 | 4,833,280 | 69,632 | FAIL-2 +4,096 (`C115-CAMPAIGN.md:447`) |
| M5e | `5c6ea087` | 4,771,840 | 4,841,472 | 69,632 | +8,192 (`C115-CAMPAIGN.md:458`) |
| M5f | `6733224e` | 4,775,936 | 4,845,568 | 69,632 | GS-PANIC-2 process-memory accounting +4,096 (`C115-CAMPAIGN.md:469,474`) |
| M5g | `f1e52a28` | 4,796,416 | 4,866,048 | 69,632 | +20,480 (`C115-CAMPAIGN.md:485-486`) |
| M5h / M5i | `7be60059` / `17708c2c` | 4,804,608 | 4,878,336 | 73,728 | +12,288 / 0 (`C115-CAMPAIGN.md:501,515`) |

| Upstream-only image | Finding on November 1 |
| --- | --- |
| Measurement | No upstream-only NRO in the record. |
| Earliest card build | Undocumented fork candidate `0577c959`, 2,408,448 bytes. |
| `libtraining_upstream33w.nro` | Matched the fork’s 32w header. |
| `UP/Cargo.toml` release profile | `panic = "abort"`, `lto = true`, no `opt-level`: default level 3. |
| Comparable profile | 32w/r29/r31 used level 3 but included fork code. |
| Proposed isolation | See “Gaps and how to measure” below. |

What the fork's profile and removals did to the image (derived from Table A rows):

| step | file delta | resident delta | text / ro / data | derivation |
| --- | ---: | ---: | --- | --- |
| r31 → r32: `opt-level "z"` + `codegen-units 1`, no source change | −1,204,224 | −1,204,224 | −1,200,128 / −12,288 / +8,192 | `65ed74ab` − `381046f2`; `REVISION32_SMALL_PROFILE.md:36` states the same −1,204,224 |
| `"s"`+cgu1 → `"z"`+cgu1 (the profile the fork kept) | −471,040 | −471,040 | −483,328 / +8,192 / +4,096 | `65ed74ab` − `2bc20eb5`; `STAGE/docs/cycle-preload.md:103` states 471,040 |
| r32 → r33: removed analytics, updater, dialog and dependencies, alongside other edits (B15–16) | −405,504 | −405,504 | −135,168 / −184,320 / −86,016 | `2b820346` − `65ed74ab` |
| 7.1.2 input display renderer cache (card before → installed) | +12,288 | +12,288 | 0 / +4,096 / +8,192 | `e1260d99` → `96605c5b`; release `v7.1.2-practice.35w` may carry other changes (`switch-install-receipt.json` lists only the NRO change) |

<a id="resource-costs--table-b-per-upstream-origin-feature"></a>

### Table B — per upstream-origin feature

| Historical hook accounting | Definition used in the November 1 record |
| --- | --- |
| Inventory | `#[skyline::hook]`, `#[skyline::from_offset]` and `install_hooks!` lists in `UP/src`. |
| Per frame | Ran every game frame while the mode was active. |
| Per call | Replacement called by the game one or more times per frame per fighter. |
| On event | Status change, hit, load or boot. |
| Outside Training | Most hooks paid the trampoline cost and the initial `is_training_mode()` check. |
| Source bytes | `Get-ChildItem` on `UP/src`, November 1. File sizes estimated code share, not image size. |

<a id="resource-costs--feature-measurements"></a>

#### Feature measurements

<a id="resource-costs--1-mash-follow-up-triggers-and-overrides"></a>

##### 1. Mash, follow-up, triggers and overrides

- **Inventory.** Mash / followup / triggers / overrides (`mash.rs` 21,917 B, `attack_angle.rs` 652 B, `full_hop.rs` 1,651 B, `fast_fall.rs` 3,289 B, `throw.rs` 4,436 B source)

- **Memory.** Statics: `CURRENT_AERIAL`, `QUEUE: RwLock<Vec<Action>>` (heap), `FALLING_AERIAL`, `AERIAL_DELAY`, `IS_TRANSITIONING_DASH` (`UP/src/training/mash.rs:20-26`), `FULL_HOP`, `FAST_FALL`+`DELAY`, `THROW_*` (`full_hop.rs:9`, `fast_fall.rs:10-12`, `throw.rs:11-17`); bytes not measured

- **CPU.** Replace hooks called per call per fighter: `ControlModule::get_command_flag_cat` (`training/mod.rs:97`), `get_attack_air_kind` (`:84`), `get_stick_dir` (`:213`, attack angle), `WorkModule::get_param_float/get_param_int` (`:55,69`); per frame per fighter from the vtable `on_frame` replacement: `fast_fall::do_fast_fall` (`training/mod.rs:150`).
- No timing measured

- **Source.** `UP/src/training/mod.rs:115-154, 894-951`

- **Confidence.** hook inventory derived; cost **not measured**

<a id="resource-costs--2-defensive-options"></a>

##### 2. Defensive options

- **Inventory.** Defensive options: DI, SDI, airdodge direction, ledge, tech, shield (hold/tilt/decay), crouch, clatter, dodge staling, hide tech (`tech.rs` 30,711 B, `shield.rs` 12,204 B, `ledge.rs` 7,528 B, `directional_influence.rs` 3,076 B, `sdi.rs` 1,821 B, `air_dodge_direction.rs` 1,878 B, `clatter.rs` 2,648 B, `crouch.rs` 840 B, `shield_tilt.rs` 866 B source)

- **Memory.** Statics are `RwLock` scalars/`Direction`s: `tech.rs:19-28` (incl. a `Vector3f` camera centre), `shield.rs:17-35`, `ledge.rs:11-14`, `sdi.rs:11-12`, `directional_influence.rs:12`, `air_dodge_direction.rs:11`, `clatter.rs:10-11`, `shield_tilt.rs:7`; bytes not measured; no table

- **CPU.** On event: `L2CFighterBase_change_status` (tech, `tech.rs:36`, installed on the `common` NRO load `UP/src/lib.rs:33-37`), `correctDamageVectorCommon` (DI, on hit, `directional_influence.rs:34`), `start_clatter` (`clatter.rs:59`), `CameraModule::req_quake_pos`, `CHANGE_ACTIVE_CAMERA`, `SET_TRAINING_FIXED_CAMERA_VALUES` (hide tech, `tech.rs:402,419,735`), `MotionModule::change_motion` (`training/mod.rs:276`).
- Per call per fighter: `check_hit_stop_delay_command` (SDI, `sdi.rs:31`), `sub_guard_cont` (shield, `shield.rs:190`), `get_stick_x/_y`, `get_stick_x/_y_no_clamp`, `check_button_on/off` (`training/mod.rs:169-262`).
- Per frame per fighter: `tech::once_per_frame`, `clatter::handle_clatter`, `ledge::do_ledge_option`, `shield::handle_shield`, `directional_influence::set_di`, `shield::param_installer`, and for the CPU a `WorkModule::set_flag` write for dodge staling plus `tech::hide_tech` (`training/mod.rs:125-153`).
- No timing measured

- **Confidence.** hook inventory derived; cost **not measured**

<a id="resource-costs--3-frame-advantage"></a>

##### 3. Frame advantage

- **Inventory.** Frame advantage (`combo.rs` 6,042 B, `frame_counter.rs` 3,232 B source)

- **Memory.** `COUNTERS: RwLock<Vec<FrameCounter>>` heap (`frame_counter.rs:3`); `combo.rs:15-22` statics; bytes not measured

- **CPU.** Per frame per fighter: `combo::once_per_frame` (`training/mod.rs:143`), `frame_counter::tick_ingame` for the CPU (`:139`), `frame_counter::tick_real` on every `info_training` draw (`ui/mod.rs:82`); per call: `WorkModule::is_enable_transition_term` (`training/mod.rs:310`).
- No timing measured

- **Confidence.** **not measured**

<a id="resource-costs--4a-save-state-slots-and-persistence"></a>

##### 4a. Save-state slots and persistence

- **Inventory.** Save states — core (`save_states.rs` 29,252 B source)

- **Memory.** `SAVE_STATE_SLOTS: Mutex<SaveStateSlots>` = 2 × 5 `SavedState` (`save_states.rs:140-152`; fields `:85-95`: 4 `f32`, 2 `i32`, a `SaveState` enum, `ChargeState` = 3 `Option<i32>` + 3 `Option<f32>` + `Option<bool>` + `Option<RobinCharges>` (`charge.rs:33-42`), `Option<SteveState>` 15 fields (`:118-134`)).
- The study estimated about 1.6 KB for ten slots from roughly 165 bytes per naturally aligned field list. It did not measure heap allocation.
- Persisted as TOML: `save_states.toml` 3,648 B on the card (`D175/switch-install-c1161-final/port/save_states.toml`, file size); the fork measured the `toml 0.5` crate the persistence pulls in at **110,592 B** of NRO (json-B variant removes it: `JSON-EXPERIMENT.md:28,33`, B122 sources; `.text` −91,568, `.rodata` −16,096, `.data` −2,832 `:29-31`)

- **CPU.** Per frame per fighter: `save_states::save_states` (`training/mod.rs:145`).
- Per call hooks: `get_param_int` (`:69`), `set_dead_rumble` (`:332`), `CameraModule::req_quake` (`:341`), `EffectModule::req_screen` (`:375`), `SoundModule::play_se` (`:457`, death SFX), `EffectModule::req`/`req_follow`/`req_on_joint` (`:546-690`, death GFX), `CAN_FUTTOBI_BACK` (`:743`, star KO).
- File write on save via the writer thread (`UP/src/common/filesystem.rs:10-21`).
- No timing measured

- **Build.** B122 for the TOML number

- **Source.** `UP/src/training/save_states.rs`, `UP/src/training/mod.rs:894-951`, `JSON-EXPERIMENT.md:21-33`

- **Confidence.** TOML crate size **measured** (fork, B122); slot bytes **derived**; CPU **not measured**

<a id="resource-costs--4b-saved-charges"></a>

##### 4b. Saved charges

- **Inventory.** Save states — charges (`charge.rs` 41,338 B source, the largest upstream training file)

- **Memory.** `ChargeState` inside each slot (row 4a); `OFFSET_COPY_SETUP` `from_offset` call for Kirby copy (`charge.rs:11`)

- **CPU.** Per call: `ArticleModule::get_int` (`training/mod.rs:782`), `handle_fighter_effect`/`handle_fighter_joint_effect` (`:947-948`).
- Runs only on save/load

- **Confidence.** **not measured**

<a id="resource-costs--4c-saved-buffs"></a>

##### 4c. Saved buffs

- **Inventory.** Save states — buffs (`buff.rs` 15,705 B source)

- **Memory.** `BUFF_REMAINING_*`, `IS_BUFFING_*`, `BUFF_DELAY_COUNTER` (`buff.rs:13-19`)

- **CPU.** Per call: `CLOUD_ADD_LIMIT` (`training/mod.rs:366`), `check_doyle_summon_dispatch` (`:395`), `REUSED_UI` (`:754`), `ADD_DAMAGE` (`:411`), `PLAY_SE` buff SFX (`:509`, installed as `handle_fighter_play_se` `:928`)

- **Confidence.** **not measured**

<a id="resource-costs--4d-saved-items"></a>

##### 4d. Saved items

- **Inventory.** Save states — items (`character_specific/items.rs` 17,584 B source)

- **Memory.** `TURNIP_CHOSEN: RwLock<Option<u32>>` (`items.rs:14`); 16 generated hook functions (`items.rs:496-511`)

- **CPU.** 16 replace hooks on Peach/Daisy `daikon_N_prob` (`items.rs:478-531`), called when a turnip spawns

- **Confidence.** **not measured**

<a id="resource-costs--4e-fighter-specific-restoration"></a>

##### 4e. Fighter-specific restoration

- **Inventory.** Save states — fighter-specific restore: Kirby (`kirby.rs` 21,974 B), Pokémon Trainer (`ptrainer.rs` 5,598 B), Pikmin (`pikmin.rs` 8,408 B), Steve (`steve.rs` 6,961 B source)

- **Memory.** `CHARACTER_SPECIFIC_STATUS_MAP: LazyLock<HashMap<(i32,i32),Vec<i32>>>` heap (`character_specific/mod.rs:13`); `SWITCH_DELAY_COUNTER` (`ptrainer.rs:12`); `SteveState` in each slot

- **CPU.** Per frame for Kirby: `KIRBY_OPFF` hook (`kirby.rs:17`); on event: `POKEMON_DECIDE` inline (`ptrainer.rs:143`), `ACTIVATE_AUTONOMY` (`pikmin.rs:34`)

- **Confidence.** **not measured**

<a id="resource-costs--5-training-reset-l-r-a"></a>

##### 5. Training reset (L+R+A)

- **Inventory.** Training reset / L+R+A (`reset.rs` 690 B source)

- **Memory.** none beyond code

- **CPU.** Inline hook `TRAINING_RESET_CHECK` (`training/mod.rs:433`, `lra_handle`); `reset::check_reset` runs in the vtable `on_init` replacement, i.e. on fighter init only (`training/mod.rs:156-162`)

- **Confidence.** **not measured**

<a id="resource-costs--6-stage-hazards"></a>

##### 6. Stage hazards

- **Inventory.** Stage hazards (`hazard_manager/mod.rs` 3,785 B source)

- **Memory.** Two `LazyLock<usize>` addresses (`hazard_manager/mod.rs:30-31`); no table

- **CPU.** Boot: two instruction-pattern scans over the whole text segment (`TextIter` state machines, `:33-83`); run time: one inline hook at the stage-load site that writes one byte (`:85-98`)

- **Confidence.** **not measured**

<a id="resource-costs--7-input-recording-and-playback"></a>

##### 7. Input recording and playback

- **Inventory.** Input recording and playback (`input_record.rs` 26,578 B source)

- **Memory.** Static table `P1_FINAL_MAPPING: RwLock<[[MappedInputs; 600]; 5]>` (`input_record.rs:55-56,69-70`); `MappedInputs` = `Buttons: u32` + 4 × `i8` = 8 B (`UP/src/common/input.rs:194,294-300`) → **24,000 B in `.data`** (derived: 5 × 600 × 8; the fork's `llvm-nm` scan calls it "the pre-existing 24 KB input-recording mapping", `ART/switch-r31-smoke-1/NOTES.md:216-217`, and "the largest static object", `REVISION32_SMALL_PROFILE.md:21`)
- plus `P1_FRAME_LENGTH_MAPPING` 40 B (`:71-72`) and eleven `RwLock` scalars (`:57-68`)

- **CPU.** Per frame per player: `handle_final_input_mapping` chain (`training/mod.rs:800-838`, playback at `:837`); per frame for the CPU: `input_record::handle_recording` (`:138`) and the `SET_CPU_CONTROLS` hook after the AI (`input_record.rs:462`).
- No timing measured

- **Confidence.** table bytes **derived**; CPU **not measured**

<a id="resource-costs--8-input-display"></a>

##### 8. Input display

- **Inventory.** Input display raw/smash/status (`input_log.rs` 12,880 B, `ui/input_log.rs` 5,499 B source)

- **Memory.** `P1_INPUT_LOGS: LazyLock<RwLock<[InputLog; 15]>>` heap (`input_log.rs:70,309-310`); each `InputLog` carries a full `Controller` copy (`input_log.rs:81-89`; `Controller` fields `UP/src/common/input.rs:240-268`), bytes not measured.
- Status labels: `STATUS_DISPLAY: LazyLock<HashMap<&str,i32>>` built from `UP/src/common/consts.rs` (318,173 B source, 9,354 lines; 4,617 lines match `STATUS_KIND_`, two per entry → about 2,300 name strings in `.rodata` plus the map on the heap once built); bytes not measured.
- Image delta of the fork's 7.1.2 renderer cache: **+12,288 B** resident (Table A derivation)

- **CPU.** Upstream: `input_log::draw` on every `info_training` layout draw (`ui/mod.rs:81-86`) and `input_log::handle_final_input_mapping` per frame (`training/mod.rs:833`).
- **Measured by the fork on a host fixture (2,897 panes), original renderer → 7.1.2:** unchanged input 212 → 1 pane lookups, 2 → 0 text writes, 25 → 0 allocations;
- held input with frame count 210 → 3 / 2 → 1 / 24 → 1; rapid inputs 214 → 19–24 / 2 → 0–1 / 26 → 1–2; hidden by the menu 210 → 1 / 2 → 0 / 26 → 0 (`STAGE/docs/input-display-performance.md:23-28`).
- Host time per scenario: steady_repeat 399,521 → 421 ns, changing_buttons 489,232 → 5,669 ns, rapid_input_120_frames 27,367,134 → 19,078,139 ns, reregister 602,402 → 725,069 ns (`STAGE/docs/input-display-benchmark.md:89-103`).
- Follow-up (child-list walk): history rotation 20,314 → 34 pane visits, rapid input 20,382–20,469 → 102–189, 120-frame rapid workload 27.0 ms → 0.216 ms host (`STAGE/docs/input-display-followup.md:21-29`).
- The baseline column is the upstream renderer's work per update.
- Not Switch FPS (`input-display-performance.md:57`)

- **Build.** host bench on the 7.1.2 sources (baseline `17c4e3f9`, `input-display-benchmark.md:5`); image delta on the 2026-10-15 card builds

- **Confidence.** renderer work **measured** (host); image delta **derived**; console FPS **not measured**

<a id="resource-costs--9-input-delay"></a>

##### 9. Input delay

- **Inventory.** Input delay (`input_delay.rs` 821 B source)

- **Memory.** `P1_DELAYED_INPUT_MAPPINGS: RwLock<VecDeque<MappedInputs>>` heap = configured delay frames × 8 B (`input_delay.rs:9,15-21`)

- **CPU.** Per frame per player inside the `OFFSET_FIM` hook (`training/mod.rs:830`): one `VecDeque` push/truncate

- **Confidence.** heap **derived** (per-frame formula); CPU **not measured**

<a id="resource-costs--10-button-chords-and-configuration"></a>

##### 10. Button chords and configuration

- **Inventory.** Button chords / button config (`button_config.rs` 10,348 B source)

- **Memory.** `BUTTON_COMBO_REQUESTS: LazyLock<RwLock<HashMap<ButtonCombo,bool>>>` heap, `START_HOLD_FRAMES`, `START_RELEASE_FRAMES` (`button_config.rs:187-197`)

- **CPU.** Per frame per player inside the FIM hook (`training/mod.rs:826`)

- **Confidence.** **not measured**

<a id="resource-costs--11-menu-model-rendering-and-persistence"></a>

##### 11. Menu model, rendering and persistence

- **Inventory.** Menu system: model (`training_mod_consts`, `training_mod_tui`), native rendering (`ui/menu.rs` 24,201 B, `ui/mod.rs` 7,046 B source), persistence

- **Memory.** `BASE_MENU`, `DEFAULTS_MENU`, `MENU` = three `TrainingModpackMenu` copies in `.data` (`UP/training_mod_consts/src/lib.rs:114,208-209`; bytes not measured); `QUICK_MENU_APP: LazyLock<RwLock<training_mod_tui::App>>` heap (`UP/src/common/menu.rs:92`), `GCC_/PROCON_BUTTON_MAPPING` HashMaps (`ui/menu.rs:66,75`), `DIRECTION_HOLD_FRAMES` (`common/menu.rs:100`); persisted menu file `training_modpack_menu.conf` **22,589 B** on the card (`ART/switch-before21/manifest.json` key `bytes`; JSON per `JSON-EXPERIMENT.md:142`).
- Data loader thread 0x20000 = 128 KiB stack, joined at boot (`UP/src/lib.rs:102-108`)

- **CPU.** `menu::menu_condition` per frame per fighter (`training/mod.rs:127`), `menu::handle_final_input_mapping` per frame per player (`:822`), `menu::draw` on every `info_training` draw (`ui/mod.rs:85`); the menu build/draw work itself not measured.
- Diag-only observation in the fork: the heartbeat's `ui_pass`/`tick_pass` counters rise by about 120/s in Training (`REVISION32_SMALL_PROFILE.md:45-46`, **stated**); one r33 Eden log shows `ui_pass=111` → `506` and `tick_pass=113` → `506` between two heartbeats 5 s apart while loading (`ART/switch-r33-eden-train1/practice-diagnostics.log:22,31`, measured)

- **Build.** r33 on Eden for the heartbeat sample

- **Confidence.** **not measured** except the menu file size and the fork-side pass counters

<a id="resource-costs--12-layout-archives-and-hud"></a>

##### 12. Layout archives and HUD

- **Inventory.** Layout archive injection and HUD toggle (`ui/mod.rs:57-187`)

- **Memory.** Embedded `src/static/layout.arc`: upstream file **4,213,268 B** raw (`Get-Item UP/src/static/layout.arc`), stored DEFLATE-compressed by `include_flate::flate!` (`ui/mod.rs:174`); compressed size estimate 175,527 B (zlib level 9) / 193,889 B (level 6) (`python3 -I` `zlib.compress` on the file, 2026-11-01; the fork states "about 195 KB", `REVISION33_STARTUP_MEMORY.md:39`).
- **Inflated copy 4,213,268 B on the game heap**: upstream `include_flate` inflates once on first use and keeps it for the process lifetime (`REVISION33_STARTUP_MEMORY.md:23`, confirmed by the log line `[LayoutArchive] handed out info_training bytes=4213268`, `ART/switch-r33-eden-train1/practice-diagnostics.log:77`; r33 then releases it: `released … after 6000ms`, `:106`).
- A second waiting-room copy (~4.2 MB) existed in the fork from 32w (`REVISION33_STARTUP_MEMORY.md:24`, stated).
- The 5 MiB static `LAYOUT_ARC` buffer exists only under the `layout_arc_from_file` feature (`ui/mod.rs:94-97`), not in release

- **CPU.** Upstream `handle_draw` runs on every layout draw of every layout: `from_c_str` of the layout name + a 5-string `contains` for the HUD toggle + `damage::draw` on each call, then the four `info_training` passes (`ui/mod.rs:57-89`); the fork's replacement observes "once per holder per frame" on the UI worker (`REVISION29_DRAW_DECOUPLING.md:109`).
- `handle_layout_arc_malloc` inline hook runs once per layout archive load; upstream parses the whole archive with `SarcFile::read`, which "cloned every member's texture/animation payload" (fork comment, `STAGE/src/training/ui/mod.rs:307-309`); inflating the 4.2 MB archive "costs a few tens of milliseconds on a loading screen" (`REVISION33_STARTUP_MEMORY.md:38-39`, stated).
- Fork heartbeat `layout_arc=7` then `9` calls by 15–20 s of uptime (`switch-r33-eden-train1/practice-diagnostics.log:22,31`)

- **Build.** r33 on Eden (2026-10-13) for the archive lines

- **Confidence.** archive bytes **measured**; compressed size **derived** (zlib estimate); draw cost **not measured**

<a id="resource-costs--13-hitbox-visualizer"></a>

##### 13. Hitbox visualizer

- **Inventory.** Hitbox visualizer (`hitbox_visualizer/mod.rs` 10,324 B, `raygun_printer.rs` 6,103 B source)

- **Memory.** Static tables `SEGMENT_DICT: [[f32;5];15]` = 300 B, `ALPHABET: [&str;40]`, `SEGMENT_REV: [char;15]` = 60 B (`UP/src/common/raygun_printer.rs:18,52,96`); no heap table

- **CPU.** Per frame per fighter: `hitbox_visualizer::once_per_frame` (`training/mod.rs:144`); per animcmd call: `sv_animcmd::ATTACK`, `CATCH` replace hooks (`hitbox_visualizer/mod.rs:196,269`), `GrabModule::set_rebound` (`:322`).
- No timing measured

- **Confidence.** table bytes **derived**; CPU **not measured**

<a id="resource-costs--14-boot-offset-search-and-logging"></a>

##### 14. Boot, offset search and logging

- **Inventory.** Boot, offset search, logging (`offsets.rs` 10,494 B, `logging.rs` 1,531 B, `filesystem.rs` 1,808 B source)

- **Memory.** 23 static byte needles (`UP/src/common/offsets.rs:60-343`) plus two fixed vtable addresses (`:356,365`); `log` facade + `TrainingModpackLogger` static (`logging.rs:46`); writer thread spawned with `std::thread::spawn`, i.e. the Switch std default **2 MiB stack for the process lifetime** (`UP/src/common/filesystem.rs:10-21`; the 2 MiB default is `REVISION33_STARTUP_MEMORY.md:12-14,22`, stated from `library/std/src/sys/thread/switch.rs`); fork reduced it to 0x10000 = 64 KiB (`STAGE/src/common/filesystem.rs:17`)

- **CPU.** Boot: 23 `memmem` needle searches over the text segment (each "first and last (about 57 MiB)" per the fork's description of the same search, `REVISION29_DRAW_DECOUPLING.md:230-231`, stated), two hazard instruction scans (row 6), three `LookupSymbol` calls (`training/mod.rs:844-870`),
- one `.nop()` text patch and two vtable word patches (`:878-892`), 66 hooks installed (34 in the `training_mods` list, 29 from the module `init` calls `:953-959`, 3 on the `common` NRO load `lib.rs:33-37`; itemised below Table B), menu load on a joined thread.
- Only the needles whose offsets are used are searched (the `OPCF` hook is commented out, `training/mod.rs:793`).
- Run time: logging goes to Skyline's `println` only; no per-frame file writes

- **Confidence.** inventory derived; time **not measured**

<a id="resource-costs--15-analytics-thread-removal"></a>

##### 15. Analytics thread removal

- **Inventory.** Removed: analytics thread (`common/events.rs` 5,470 B source)

- **Memory.** Permanent thread via `std::thread::spawn(events_loop)` (`UP/src/lib.rs:154`) → **2 MiB stack** (`REVISION33_STARTUP_MEMORY.md:20`, stated); `EVENT_QUEUE: RwLock<Vec<Event>>` with 7 `String`s per event (`events.rs:11,95-104`), `SESSION_ID`/`DEVICE_ID`/`USER_ID` strings (`:12,44,65`); dependencies `minreq` (`https-native`), `native-tls` vendored (`UP/Cargo.toml` `[dependencies]`, `[patch.crates-io]`).
- Image cost together with row 16 and the r33 edits: **−405,504 B** resident when removed (Table A derivation, `65ed74ab` → `2b820346`: text −135,168, ro −184,320, data −86,016).
- Upper bound for rows 15+16: the r33 tree also touched `filesystem.rs`, `menu.rs`, `hang_trace.rs`, `named_macros.rs`, `ui/mod.rs`, `training_mod_consts` (`FOOT/post-build-check.json` key `worktree_status_porcelain`)

- **CPU.** Loop: `sleep(10 s)` then `minreq::post` of the queued events (`events.rs:177,187`); network and TLS work every 10 s for the session

- **Build.** r32 → r33 NROs, 2026-10-13

- **Confidence.** image delta **derived** (upper bound); stack **stated**; CPU **not measured**

<a id="resource-costs--16-updater-and-dialog-removal"></a>

##### 16. Updater and dialog removal

- **Inventory.** Removed: auto-update / version check thread and dialog (`common/release.rs` 7,888 B, `common/dialog.rs` 1,735 B source)

- **Memory.** Thread with explicit `stack_size(0x80000)` = **512 KiB**, joined before `main` returns, so transient (`UP/src/lib.rs:110-121`; skipped on emulators `:110`); downloads the GitHub release zip into a `Vec` and extracts it (`release.rs:30-47`); dependencies `zip`, `byte-unit`, `skyline-web` (dialog) (`UP/Cargo.toml`); `CURRENT_VERSION` string (`release.rs:11`).
- Image cost: inside the −405,504 B of row 15 (`REVISION33_STARTUP_MEMORY.md:21,25` lists exactly these removals)

- **CPU.** One HTTPS GET at every console boot (`release.rs:88`, `:176`), plus the dialog and zip install when accepted (`:46-52,154,209`)

- **Build.** r32 → r33 NROs

- **Confidence.** stack **measured in source** (constant); image **derived** (shared with row 15); boot time **not measured**

<a id="resource-costs--17-whole-nro"></a>

##### 17. Whole NRO

- **Inventory.** Whole-NRO image

- **Memory.** See Table A.
- Upstream-origin code share of the M5i image is **not measured**; the fork's own additions dominate the growth from 1,798,144 B (r33) to 4,878,336 B (M5i), of which `.rodata` alone grew from 667,648 to 2,510,848 B (Table A rows; the console-ui84 build shows the `.rodata` step to 2,580,480 B at 2026-10-22)

- **Build.** all

- **Source.** Table A

- **Confidence.** **measured** (whole image), per-feature split **not measured**

| November 1 upstream inventory | Calls or installed hooks | Historical source under `UP/src` |
| --- | --- | --- |
| Fighter vtable replacement | `once_per_frame_per_fighter`: 11 feature calls per fighter per frame, four more for CPUs. | `training/mod.rs:115-154` |
| Player input | One `OFFSET_FIM` per player per frame, six feature calls. | `training/mod.rs:800-838` |
| CPU controls | One `SET_CPU_CONTROLS` per CPU per frame. | `input_record.rs:462` |
| Kirby and layout | One Kirby per-frame hook and one `handle_draw` per layout draw. | `kirby.rs:17`, `ui/mod.rs:57` |
| `training_mods` | 34: FIM, three inline hooks and 30 function replacements. | `training/mod.rs:894-951` |
| Inline hooks | `TRAINING_RESET_CHECK`, `STALE`, `STALE_MENU`. | Same list. |
| Replacements | ControlModule, WorkModule, EffectModule, SoundModule, CameraModule, MotionModule, ArticleModule and fighter functions. | Per fighter per frame or event. |
| Item probability | 16 | `items.rs:514-531` |
| CPU controls and UI | One `set_cpu_controls`, two UI hooks. | `input_record.rs:605`, `ui/mod.rs:201` |
| Fighter-specific | One each for Pikmin, Pokémon Trainer and Kirby. | `pikmin.rs:51`, `ptrainer.rs:158`, `kirby.rs:607` |
| Tech camera and hitbox visualizer | Three each. | `tech.rs:753-757`, `hitbox_visualizer/mod.rs:353` |
| Hazards and NRO load | One hazard inline hook, three hooks on `common` NRO load. | `hazard_manager/mod.rs:85`, `lib.rs:33-37` |
| Release total | 66 hooks, plus two vtable word patches and one nop patch. | `training/mod.rs:878-892` (patches). |
| Debug additions | Six WorkModule hooks with `debug_assertions`. | `training/mod.rs:962-963`, `debug.rs:17-99` |

<a id="resource-costs--table-c-context-from-the-fork-s-own-features-frames-the-whole-plugin-budget"></a>

### Table C — context from the fork's own features (frames the whole-plugin budget)

| Table C measures | Interpretation |
| --- | --- |
| Admission and process memory | Kernel headroom, per-kind charges and admission thresholds. |
| Load performance | Host traces and physical-console samples carry separate platform labels. |
| Plugin footprint | Resident image, archive hand-outs, thread stacks and diagnostic overhead. |

<a id="resource-costs--process-memory-budget-the-kernel-enforces-svcgetinfo-id-6"></a>

#### Process memory budget the kernel enforces (svcGetInfo id 6)

- 3,444,572,160 B = 3285 MiB in every sample of the s19/s21 runs;
- `headroom` = id 6 − id 7
- **Source.** `STAGE/training_mod_consts/src/body_bank/memory.rs:9-10`; `D175/SWITCH-INSTALL-C116.1.json` key `not_verified[0]` ("same 3285 MiB budget", Eden-measured, console not verified)
- **Confidence.** measured (Eden)

<a id="resource-costs--admission-rule"></a>

#### Admission rule

- load admitted only if headroom − outstanding ≥ cost × (1 + 1/8) with a 16 MiB floor
- **Source.** `memory.rs:45,48` (`MARGIN_DIVISOR = 8`, `FLOOR = 16 * MIB`); `D175/C115-CAMPAIGN.md:469` ("margin 1/8 + 16 MiB floor verified against 663 prefetch / 590 build measurements")
- **Confidence.** measured

<a id="resource-costs--per-fighter-costs-process-charge-eden"></a>

#### Per-fighter costs (process charge, Eden)

- 176 kinds in `KIND_COSTS`, e.g.
- Mario `0x00` prefetch 44,433,408 B / build 13,631,488 B, Donkey Kong `0x01` 43,646,976 / 2,293,760, `0x06` 107,741,184 / 19,660,800; range 3–147 MiB prefetch, 0–98 MiB build;
- Game & Watch builds 80–98 MiB alone;
- Simon/Richter ≈ 100 MiB;
- Falcon helper load 153.8 MiB once
- **Source.** `STAGE/training_mod_consts/src/body_bank/memory_table.rs:261-272` (rows; 176 rows counted with `rg -c "^    \(0x"`), `memory.rs:13-14,18-19`; `SWITCH-INSTALL-C116.1.json` key `not_verified[1]`
- **Confidence.** measured (Eden); console not measured

<a id="resource-costs--how-the-table-was-built"></a>

#### How the table was built

- `tools/group_memory_costs.py` reads `[GroupSwap] prefetch … headroom=`, `[PreloadMetrics] v1 record … committed=A->B … headroom=`, `[GroupSwap]/[CharacterSources] memory … phase=… delta=` lines from 298 logs; estimates per kind = largest single observation, bounds from group totals, 10th-percentile lower bound
- **Source.** `STAGE/tools/group_memory_costs.py:1-36,132-170`; `memory_table.rs:1-3`
- **Confidence.** measured

<a id="resource-costs--preload-metrics-record-format"></a>

#### Preload metrics record format

- `[PreloadMetrics] v1 record scene=… op=request target=fighter kind=0x4a hit=0 outcome=ready frames=0 us=1215334 … committed=3021647872->2762059776 delta=-259588096 base_delta=+268435456 headroom=682512384 slab=348160->368640/16777216 slots=5 apply=fighter+hat resident=4 …`;
- `committed` is svcGetInfo id 7 (committed process pages, "NOT allocator live bytes"), time is the Horizon tick
- **Source.** `D175/refactor/eden-sweep/yoga/runs/hlp4/final/ultimate/TrainingModpack/practice-diagnostics.log:6`; `STAGE/src/preload_metrics.rs:1-12`
- **Confidence.** measured

<a id="resource-costs--eden-training-comparison-of-the-preload-fork-feature"></a>

#### Eden Training comparison of the preload (fork feature)

- targets Off/1/2/3/4/5: fighter median 805/520/501/486/477/468 ms, hat median 770/82/82/81/82/81 ms, extra peak guest memory +0/+55/+104/+137/+165/+214 MiB
- **Source.** `STAGE/docs/cycle-preload.md:66-75`
- **Confidence.** measured (Eden)

<a id="resource-costs--physical-switch-headroom-samples-c111"></a>

#### Physical Switch headroom samples (C111)

- minimum headroom 646,795,264 B = 616.83 MiB (sample #182), baseline 852,774,912 B = 813.27 MiB (#64), budget 3,444,572,160 B
- **Source.** `D175/C111-SWITCH-SPEED.md:23`
- **Confidence.** measured (console)

<a id="resource-costs--hot-swap-timings-fork-feature"></a>

#### Hot-swap timings (fork feature)

- C111 console: window build 5.65 s / 6.31 s / 4.35 s, in-window reuse 0.097 s and 0.378 s;
- C116 Eden: warm group swaps ~50 ms, first build into a group ~0.4–1.0 s, IC→X 67–105 ms, X→IC 310–327 ms, first pair build ~2.1 s;
- C108 cold reload baseline mean 684.814 ms (9 reloads, Eden)
- **Source.** `D175/C111-SWITCH-SPEED.md:9-17`; `D175/SWITCH-INSTALL-C116.json` key `contents[2..3]`; `D175/COLD-BASELINE.md:5`
- **Confidence.** measured

<a id="resource-costs--fork-heap-statement-for-the-rc"></a>

#### Fork heap statement for the RC

- +356,352 B resident B123-rc → M5 (+7.97 %), largest item the 608/626-row seam table (+221,184 at M1: ro +106,496, data +118,784, text +40,960), diag ring +40,960 bss, WP-H meter +4,096 text (shipped build unmetered)
- **Source.** `D175/refactor/RC-SUMMARY.md:13-15`; `INTEGRATION.md:1680-1683,447,1388`; `C115-CAMPAIGN.md:355`
- **Confidence.** measured

<a id="resource-costs--diag-build-overhead"></a>

#### Diag build overhead

- shipped 4,521,984 → `full,diag` 4,624,384 (+102,400) → `+plugin-heap-meter` 4,628,480 (+4,096 text) resident at WP-H;
- M5i release 4,878,336 vs diag 4,988,928 (+110,592; bss 73,728 → 172,032)
- **Source.** `INTEGRATION.md:1387-1389`; Table A rows
- **Confidence.** measured

<a id="resource-costs--fork-layout-archives"></a>

#### Fork layout archives

- `STAGE/src/static/layout.arc` 7,820,940 B raw (zlib-9 1,677,918 B); inflated hand-outs seen in logs: `info_training bytes=7635780`, `info_waiting bytes=6667356`, `6737012`, `8509244`, each released 5–9 s later
- **Source.** `Get-Item`/`zlib` 2026-11-01; `ART/enhancements/plan2-mii-runtime-01/practice-diagnostics.log:152,164`; `ART/training-performance-followup-20261015/grid-waiting/practice-diagnostics.log:92,109`; `ART/enhancements/plans14-ldn05-audit01/evidence/window-a-accepted-practice-diagnostics.log:192,217`
- **Confidence.** measured (Eden)

<a id="resource-costs--fork-thread-stacks"></a>

#### Fork thread stacks

- heartbeat/hang-trace thread 0x10000 = 64 KiB, config writer 0x10000 = 64 KiB, data loader 0x20000 = 128 KiB; diagnostics ring 32 KiB (release) / 128 KiB (diag), writer every 50 ms, files capped at 128 KiB
- **Source.** `STAGE/src/hang_trace.rs:151`, `STAGE/src/common/filesystem.rs:17`, `STAGE/src/lib.rs:352`, `STAGE/src/logging.rs:38,42`; `cycle-preload.md:88`
- **Confidence.** measured in source

<a id="resource-costs--eden-host-level-figures"></a>

#### Eden host-level figures

- ~60 FPS idle windows, idle CPU ~0.945/0.951 cores, host peak 5.5–6.7 GiB across game loads (whole emulator; not plugin cost)
- **Source.** `STAGE/docs/experimental-release.md:69-73,79-81`
- **Confidence.** measured (host), not plugin-attributable

<a id="resource-costs--gaps-and-how-to-measure"></a>

### Gaps and how to measure

| Measurement proposed November 1 | Scope and method |
| --- | --- |
| Gap | No single-feature on/off builds and no per-feature console counter in the record. |
| Phase 3 proposal | Use the fork’s instrumentation for before/after budgets. |
| 1. Per-feature image (rows 1–14, 17) | Isolate each upstream-origin feature with a Cargo feature. |
| Build configuration | Playbook `docs/WORKFLOW.md` §§1–3 specified `diag` and separate model/runtime code. |
| Matrix | Build all-on and all-on-minus-X with `cargo skyline build --release` (WORKFLOW §4). |
| Measure | Run `python3 -I PLAYBOOK/tools/build/nro_size.py` on each NRO and retain its SHA-256. |
| Attribution | Compare segments in 4 KiB pages, the stage trail’s page rounding. |
| Missing Table A baseline | Proposed upstream-only `97cbe9b`, opt-level 3 and `lto = true` (`UP/Cargo.toml`). |
| Profile comparison | Repeat with `opt-level = "z"`, `codegen-units = 1` against fork r33’s 1,798,144 bytes. |
| Matrix schema | `FOOT/results.json`: overrides, returncode, nro_sha256, segments. |

2. **Static tables and `.data`/`.bss` attribution.** `llvm-nm --print-size --demangle` on the release `.so` and bucket
   the symbols per module, as the fork did for the `.smd` path (`ART/switch-r31-smoke-1/NOTES.md:205-222`). This
   gives the exact bytes of `P1_FINAL_MAPPING`, the three menu copies, the status-name strings and the raygun tables
   without a per-feature build.

| 3. Heap per feature (rows 4a, 8–12) | November 1 proposal |
| --- | --- |
| Counters | Live, peak, total, calls and largest from `STAGE/src/heap_metrics.rs:17-62`. |
| Playbook | `PLAYBOOK/docs/WORKFLOW.md` §3, lines 86–92. |
| Diagnostics | Add `heap_live=` and `heap_peak=` bytes to install and snapshot lines alongside `heap_allocations=<calls>`. |
| Baseline | Fork release printed `unmetered` (`STAGE/src/swap_native/heap_fields.rs:1-21`). Searched logs contained no numeric Training Modpack result. |
| Snapshots | Boot, first Training load, then each menu toggle. Attribute Rust heap from successive differences. |
| Archive | Log inflated length and lifetime using `[LayoutArchive] handed out/released … bytes=… after …ms`. |

4. **Process-level memory (what the game heap actually loses).** Sample svcGetInfo id 7 (`committed`) and id 6
   before and after each feature's first use, exactly as `STAGE/src/preload_metrics.rs` does (register-only SVC,
   `:36-56`), and in Eden read `memory.peak` from each run's `run.json` (the Eden workers' ledgers under
   `D175/refactor/eden-*` record it; `RC-SUMMARY.md:17`). Only a positive delta is evidence ("a zero delta proves
   nothing", `preload_metrics.rs:7-8`).
5. **Thread stacks (rows 14–16).** Every `std::thread::spawn` without `stack_size` costs 2 MiB
   (`REVISION33_STARTUP_MEMORY.md:12-14`, stated). Measure rather than state it: compare svcGetInfo id 7 before and
   after spawning the thread in a diag build, once with the default and once with an explicit size. The clean-room
   has no analytics or updater thread to measure; the writer/diag threads should be spawned with explicit sizes and
   the chosen sizes recorded in the install line.

| 6. CPU per hook (rows 1–13) | November 1 proposal |
| --- | --- |
| Instrumentation | Count calls per heartbeat and accumulate `cntpct_el0` ticks inside each hook. |
| Tick reader | `preload_metrics.rs:24-37`. |
| Reporter | Heartbeat thread following `[Heartbeat] … ui_pass=<calls>/<ms since last>` from `REVISION31_HANG_TRACE.md:23-24`. |
| Sample | `ART/switch-r33-eden-train1/practice-diagnostics.log:22,31`. |
| Per-feature estimate | Hooks × calls per frame × mean ticks. |
| Timing comparison | Pair Eden frame times with consecutive console per-frame timestamps. |
| Unit | Microseconds per frame against a 16,667 µs budget at 60 Hz. |

7. **UI renderer work (rows 8, 11, 12).** Reuse the host fixture method of `scripts/input-display-bench`
   (`STAGE/docs/input-display-performance.md:17-41`, `input-display-benchmark.md:103`): instrument the pane tree,
   count lookups/visits/writes/allocations and nanoseconds per update for each renderer (input display rows, menu,
   HUD toggle), with the "missing invalidation" control. It is a renderer measurement, not FPS; the console FPS
   effect stays unmeasured until method 6 runs on the card.
8. **Boot cost (row 14, row 6).** Timestamp the install line and each needle/instruction scan with the Horizon tick;
   report the sum. The fork's statement of the search region (about 57 MiB, `REVISION29_DRAW_DECOUPLING.md:231`)
   should be replaced by the measured region length from `getRegionAddress(Text/Rodata)` (`UP/src/common/offsets.rs:11-12`).
9. **Persistence (rows 4a, 11).** The fork measured one format choice: dropping `toml 0.5` saved 110,592 B of NRO on
   B122 (`JSON-EXPERIMENT.md:28`) while `toml 0.8` would have cost +57,344 B (`TOML08-EXPERIMENT.md:30,37`). The
   clean-room should decide the save format before phase 3 and measure its crate cost with method 1.

<a id="resource-costs--contradictions-and-cautions-found-between-sources"></a>

### Contradictions and cautions found between sources

| Comparison | Read the figures on the same basis |
| --- | --- |
| Storage | Distinguish NRO header segments, ELF sections and compressed archive estimates. |
| Execution | Match the scene, platform and source tree before comparing runs. |

- **bss: NRO header vs ELF.** For the same B122 build the NRO header gives `bss=28672` and resident 4,562,944
  (`nro_size.py` on `92cef8a7`), while the ELF-based measurement gives `.bss` 26,432 and "heap cost (NRO + .bss)"
  4,560,704 (`JSON-EXPERIMENT.md:32-33`); `RC-SUMMARY.md:13` compares against the ELF figure ("vs B122
  4,560,704"). The two differ by 2,240 B (page rounding). Use the header figure from `nro_size.py` consistently.
- **"s" versus "z" misstatement.** `D175/switch-install-c1161-final/COMPLIANCE-FINAL.md:9` says
  `footprint-experiments` "measured 's' at +758 KB". The experiments measured `"s"` at −757,760 B relative to
  opt-level 3 (`REVISION32_SMALL_PROFILE.md:33`); relative to the shipped `"z"`+cgu1 profile, `"s"`+cgu1 is
  +471,040 B (`2bc20eb5` − `65ed74ab`; `cycle-preload.md:103`). The +758 KB is not the cost of "s" versus the
  fork's profile.
- **`FOOT/results.json` mixes two source trees.** Five variants were built from the revision 31 sources
  (18:07–18:14 on 2026-10-13; `cgu1` equals REVISION32's row at `:32`), the `default` variant at 19:07 from the
  revision 33 working tree (`build-default.log:13`, `post-build-check.json`). The JSON does not say so; the memory
  note `/home/intpa/.claude/projects/-home-intpa-smash-training-lobby-UltimateTrainingModpack/memory/modpack-footprint-levers.md:26`
  does ("r33 image 1,769,472 B").
- **`libtraining_upstream33w.nro` is not an upstream build.** Its header equals the fork's 32w build byte for byte
  (Table A) and no document in `ART` names it.
- **Heartbeat rates.** `REVISION32_SMALL_PROFILE.md:45-46` states `ui_pass`/`tick_pass` "+120/s" and
  `roster_service` "+240/s"; the one r33 log sampled shows 395 passes in the 5 s between heartbeats #22 and #31
  during the Training load (about 79/s). Rates depend on the scene; neither is a per-hook cost.
- **Compressed archive.** REVISION33’s rough figure of 195 KB (`:39`) was closest to zlib level 6 (193,889 B). Level 9 yielded 175,527 B. The record does not identify `include_flate`’s compression level.
- **Platform.** The per-fighter process charges in Table C came from Eden. `SWITCH-INSTALL-C116.1.json`, key `not_verified[0]`, assumed equal console costs without measuring console timing or heap. C111 recorded console headroom separately (`C111-SWITCH-SPEED.md:23`).

<a id="resource-costs--revision-33-provenance"></a>

##### Revision 33 provenance

- `2b820346`, `FOOT/results.json` key `default.segments`; `ART/switch-before-r34/libtraining_modpack.nro`. **The `default` variant is not an upstream build**: `FOOT/build-default.log:13` compiles `training_modpack v7.1.1` from `UltimateTrainingModpack-single-file`, and `FOOT/post-build-check.json` shows that tree with `src/common/dialog.rs`, `events.rs`, `release.rs` deleted and `docs/REVISION33_STARTUP_MEMORY.md` untracked, i.e. the revision 33 working tree on the revision 32 profile

<a id="resource-costs--console-ui-assets"></a>

##### Console UI assets

- `cac99b4c`, `ART/enhancements/console-ui84/physical-plugins/libtraining_modpack.nro` (the `.rodata` jump to 2.58 MB is the fork's own embedded roster/stage/item image data, see §C)

<a id="resource-costs--b122-resident-measurements"></a>

##### B122 resident measurements

- `92cef8a7`, `D175/switch-collection/c116_1-precheck/payload/…`; `D175/SWITCH-INSTALL-C116.json` keys `at`, `bytes`. The ELF measurement of the same build reports `.bss` 26,432 and "heap cost (NRO + .bss)" 4,560,704 (`ART/enhancements/automation-macros01/toml08-experiment/JSON-EXPERIMENT.md:32-33`); see "Contradictions"

<a id="resource-costs--m5i-installation-receipt"></a>

##### M5i installation receipt

- `17708c2c`, `D175/refactor/stages/m5i/release/libtraining_modpack.nro`; `D175/SWITCH-INSTALL-C116.1.json` keys `bytes`, `resident`, `resident_vs_b123rc` ("about +9.1 %": 4,878,336 / 4,472,832 = 1.0907, derived)

<a id="resource-costs--integration-service-stacks"></a>

##### Integration service stacks

- rc.8 pass head `9d6c7e02` (`p23/rc8-integration`: §6, §4/§7 `729ab045` with two 304 KiB service stacks, the binds)

<a id="resource-costs--r6-at-level-s"></a>

##### R6 at level s

- rc.8 `p23/rc8-waste` `21527b70` at `"s"` (R3, R4 1024-slot replay mailboxes, R5, R8; two 272 KiB stacks): R6's comparison

<a id="resource-costs--r6-at-level-z"></a>

##### R6 at level z

- **rc.8 `p23/rc8-waste` `f86771f7`** (the reopened scope: R3–R6, R8; `opt-level "z"`, 1024-slot replay mailboxes, two 236 KiB service stacks; `artifacts/phase23/rc8-waste/DESIGN.md` §0)
