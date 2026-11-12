# rc.10 code reduction: measurement and plan

The plan of `TASK-rc10-code-reduction.md`, measured on `main` `b1e27e5a`, 2026-11-12 (UTC), and the result of each
slice as it lands on `rc10`. Done so far: **S3**, **S1**, **S2** and **S4** (below, one section each).

## The short answer

- **Upstream is 24,325 lines. TrainingRooms is 129,593 non-test lines** (122,707 product, 6,755 diagnostic-only,
  131 host-only), plus 48,930 test lines.
- **The target of about 24k is not reachable with every feature kept.** The parts of TrainingRooms that do what
  upstream also does are already larger than all of upstream, and the features upstream doesn't have are about 79k
  more.
- **The realistic floor is about 110–117k non-test lines** (4.5–4.8 times upstream), a cut of 12–19k (9–15%).
  About 12–16k of that is low risk and leaves the release NRO's code unchanged.
- Three items need the primary's or the operator's word before I start them: collapsing the Cargo feature table
  (S2), deleting unused diagnostic probes (S4), and whether the guard tables may live in a data file (S1).

## How the lines were counted

Two counters, both on the same tree.

| Counter | Rule | TrainingRooms | Upstream `077a8e4` | Ratio |
| --- | --- | --- | --- | --- |
| Raw (the task's rule) | non-blank, non-comment lines of `*.rs` outside `target/`, `tests/`, `third_party/`, `vendor/`; test code separated | **129,593** non-test | **24,325** | 5.3× |
| `tools/sloc.py` (the repo's own, rustfmt-normalized `rustfmt-sloc4`) | every file formatted with the pinned rustfmt first, then the same rule | **165,931** | **24,343** | 6.8× |

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

### Product, diagnostic and test lines per area (raw counter)

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

## Where the size is

Measured over the product lines.

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

What I looked for and did not find in quantity:
- settled `const … : bool = true` switches (5 in the tree);
- `#[allow]` attributes in product code (4, in `hos-lock` and the layout tool);
- large copy-pasted modules, apart from the two Mii role files.

## The slice plan

Ranked by safe saving. "Identical NRO" means the rc.9 test-cut method: the release and diag NROs differ from the
base only in the build id, the commit tag and panic line numbers.

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

**Total:** −12,250 to −15,700 for S1–S4; −14,850 to −19,300 with S5–S8.

### S1: guard tables become data

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

### S2: collapse the always-on feature table

- **Now:** `Cargo.toml` carries the fork's table (decision D11). 26 of its features are always on in the shipped
  closure, with 910 gate lines on them and 406 lines that compile only when one is off.
- **After:** those gates and dead arms are gone. `diag`, the probe family that survives S4 and `deluxe-sdk` (an
  optional dependency) stay as features.
- **Needs the primary's word:** D11 is a recorded decision, `tests/pkg6_features.rs` and `tools/feature_matrix.py`
  check the table, and a `--no-default-features` build then holds the whole product (today it holds a smaller one
  that never ships).

### S3: compiler-proven dead code

- **First step, a measurement:** make the module tree `pub(crate)` (a `cdylib` exports nothing by Rust path) and
  read rustc's `dead_code` warnings for the Switch build and for the test build.
- Dead in both: delete. Live only in tests: move behind `#[cfg(test)]`, where it stays tested and counts as test
  code.
- The estimate is soft until that step runs; the by-name lower bound is 1,565 lines. `tests/*.rs` include the model
  by path, so the visibility change has to keep them compiling.

### S4: probes nothing uses

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

### S5–S8

- **S5** follows S1: one row type and one projection for both table systems.
- **S6:** `runtime/mii/mii_ordinary_cpu.rs` (1,433) and `mii_ordinary_player.rs` (1,220) share 804 lines of
  identical 6-line windows, and `model/mii/ordinary_cpu.rs` and `ordinary_player.rs` mirror each other. This is
  native Mii install and replay code under locks, so it carries the Eden Mii rows for both roles.
- **S7:** `recording_menu.rs`, `macro_panel.rs`, `menu/mod.rs`, `panes.rs`, `runtime/ui/view.rs` and `worker.rs`
  (7,431 lines). A table replaces repeated arms only where the drawn output is unchanged. I measure each file before
  claiming its number.
- **S8:** `model/compat` and `runtime/compat` (1,146 lines) let the carried code read the fork's old menu struct.
  Removing them is a re-architecture of the carried runtime's settings reads; I don't recommend it for rc.10.

### Each slice, every time

- Gates: clippy 0, `--no-default-features`, release and diag builds, size, the stack table, svc, the game-thread
  wait check, carry, the tag gate, `tools/check/compliance.py`.
- Tests green, ≤ 1,000 test functions, line coverage ≥ 90%; tests move with their code.
- A no-behaviour-change argument and the line delta (both counters) recorded here.
- An Eden check with the GDB stub on where the table above names one; pure model slices rely on tests.
- Cargo under `systemd-run --user --scope -p MemoryMax=…` and `nice 19`, one subcommand per call, in my own worktree
  and target.

## The realistic floor

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

## S3: done (2026-11-12)

Code that no Switch build uses is deleted, or sits behind `#[cfg(test)]` when only tests use it, wherever that leaves
the NROs unchanged and adds no warning.

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

Release and diag NROs built from the S3 commit against those built from `f5146184`: `.text` is byte-identical in
both. Of the 4,136,960 and 4,329,472 bytes, 557 and 563 differ: the 32-byte build id, the 8 characters of the commit
in the build tag, and the line numbers of 500 and 506 panic locations, which moved because lines above them were
added or removed.

**Why dead types stayed, and what touches them**

- A first attempt also removed or gated the 132 dead types with their impl blocks (−5,428 product lines). Its NROs
  had the same size and the same sections, and every function was the same once addresses were masked, but 83
  functions had another mangled name and the functions were laid out in another order. An impl block, written or
  derived, is numbered among the impls of its module, and that number is part of the name of everything in the later
  impls. So removing a type renames code that stays.
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
## S1: done (2026-11-12)

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
## S2: done (2026-11-12)

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
## S4: done (2026-11-12)

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
- **Eden boot** (run `rc10-s4-fb028a9b-g2`, the Surface, diag NRO `d2d39f02…` of `fb028a9b`, the GDB stub on before
  the load, tracing off): the plugin installed (`result=installed heap_allocations=0`); 11 rows passed: the stub and
  the load, boot to Training, the old settings, the migration, a chord, the CPU fighter through the menu, five hot
  switches, a held item, the Replay window and a named Mii. One row failed, `acc-hook` (no `hook_stats` line in the
  log); it failed the same way on the base build in the integration lane's last run of this set
  (`final-rc8-surface-1-eca14e4b`). The console has no panic, abort, crash or unmapped-access line. This boot is also
  S2's check: S2's binary was not byte-identical, and the build under test contains it.
## After S4: where rc.10 stands, and S5 to S7 re-measured

| | Start (`f5146184`) | After S3, S1, S2, S4 (`fb028a9b`) | Change |
| --- | --- | --- | --- |
| Product lines (raw) | 122,707 | 110,364 | −12,343 |
| Diag-only lines (raw) | 6,755 | 5,692 | −1,063 |
| Non-test lines (raw) | 129,593 | 116,187 | −13,406 (−10.3%) |
| `tools/sloc.py` | 165,930 | 151,623 | −14,307 |

Against upstream's 24,325 that is 4.8 times, from 5.3. It is inside the floor this plan gave (110,000–117,000).

The three slices left, measured again on `fb028a9b`:

| Slice | What is there now | Realistic saving | Risk and cost |
| --- | --- | --- | --- |
| S5, one guard-table system | The tables are data already. What is left is two row structs and their projections (`native_seams/mod.rs`, `sites.rs`) and the two installers that read them. | about −300 | Medium: both installers change; an Eden boot. |
| S6, one Mii role owner | `runtime/mii/mii_ordinary_cpu.rs` and `mii_ordinary_player.rs` share about 350 lines each of identical 6-line windows. | about −300 (one copy stays, plus the plumbing a shared owner needs) | High: native Mii transactions under locks; the Eden Mii rows for both roles. |
| S7, menu and pop-up tables | 549 `X => "text"` arms, at most 65 in one module. An arm is already one line; a table does not make it shorter. | under −300 | Medium: the menu and panel tests, and an Eden menu tour. |

Together that is about 800 lines, 0.7% of what is left, for two Eden passes and a high-risk refactor. My
recommendation is not to do S5 to S7 for rc.10. The larger remaining saving is the roughly 2,500 dead lines S3 left
in place (dead types and what touches them), which needs a decision on its proof, not new engineering.
## Line deltas per slice

| Slice | Head | Raw non-test (product) | `tools/sloc.py` | Tests | Coverage |
| --- | --- | --- | --- | --- | --- |
| Plan baseline | `b1e27e5a` | 129,593 (122,707) | 165,931 | 865 (917 with the crates' gates) | 91.54% |
| `rc10` start | `f5146184` | 129,593 (122,707) | 165,930 | 821 (873) | 90.18% |
| S3 | `6c419689` | 126,703 (119,817) | 162,692 | 821 (873) | 90.59% |
| S1 | `13e8ef5d` | 118,493 (111,612) | 154,519 | 824 (876) | not re-measured |
| S2 | `84e39648` | 117,248 (110,367) | 152,990 | 824 (876) | not re-measured |
| S4 | `fb028a9b` | 116,187 (110,364) | 151,623 | 824 (876) | not re-measured |
