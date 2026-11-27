# Source-size plan and measurement history

[The latest total in the size budget](../size-budget.toml) sets the active normalized ceiling.

The dated entries below record the November 2, 2026 slim-stream plan and subsequent measurements.

## Source-size criterion adopted November 2, 2026 (14:2xZ)

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

## 1. November 2 baseline

| | Lines | |
| --- | ---: | --- |
| Fork M5q | 112,500 | operator |
| Clean room `7bb86fd` | 106,983 | M |
| Pending in unmerged streams (work tree minus merge base, ~12:10Z) | +7,061 | [Measured contributions across nine unmerged streams](#unmerged-stream-contributions) |
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

## 2. Per area

| Area estimates | Reading the rows |
| --- | --- |
| Savings | Counted source lines. E means estimated, M means measured. |
| Risk | Low, medium or high. |
| Rows | Eden scenarios listed for rerun after that proposed change. |

### roster (`src/runtime/roster` 19,551) and swap (`src/model/swap` 7,980), carried (pkg6 manifest)

At the November 2 checkpoint, EDEN-PACKAGE-6 covered the s6-01 sync probe and s6-02/03 boot and CPU-as-P1. CP0/CP1/CP3 still needed the roster cycle.

- The baseline required package 6 sets A/B/D, R-ID and R-PC before a roster refactor.
- The original plan is in `docs/PACKAGE-6-PLAN.md` §8 at tag `v0.2.0-rc.1` (subsequently removed).

| Finding (examples) | Change | Saves | Risk | Rows |
| --- | --- | ---: | --- | --- |
| [Legacy fingerprints alongside authoritative seam guards](#legacy-roster-fingerprints) | delete the legacy words, keep relocated-data checks | ~190 | M | s6-02/03 install line, set A boot |
| `mii_reload_probe` module `character_roster.rs:2434-2835` names modules that do not exist at `7bb86fd` (`crate::mii_fullconfig_probe` …) | becomes compilable when stream/pkg6 lands those probes; retire with them (D11) | ~350 | L (not shipped) | none |
| Identical `trace!` macro defined 11 times (`copy_packages.rs:19`, `special_articles.rs:15`, …) | one crate macro | ~22 | L | none |
| `identity_view.rs:184-343`: 14 near-identical inline hooks | one `macro_rules!` table (site, offset, registers, scope) | ~60 | M (hook code) | R-ID0..9, R-PC1 |
| `cycle_preload_config.rs:66-182`: 4 logs of 11-15 fields | `Display` in the model, text byte-identical | ~45 | L | preload rows |
| [Repeated native wrappers, owner table and kind operations](#repeated-native-wrappers) | one roster offsets table, one wrapper per native (extend `tests/native_wrappers.rs` beyond `game.rs`/`state_natives.rs`) | ~30 (rest leaves with the probes) | L | R-PC1/PC2 |
| `cfg(not(alpha))` alternates for builds never shipped (`character_sources.rs:1555-1608` …) | one version via `slot_allowed()`; all branches if D11 is relaxed | 40 / ~205 | L / M | set D, R-PC1 |
| Model functions only tests use (`body_bank.rs:287,508,568,836`, `ledger.rs:261,438,587,633-662`, `lifecycle.rs:306,350,381,399`, `roster_grid.rs:254-467` …), `swap_model/hud_model.rs` 68, `target.rs` 41 | delete with their tests (check roster_grid against the screens first) | ~360 | L-M | none |
| Probes and observers (§4): ~6.4k incl. ~520 probe-gated lines inside product files | retire after the owners confirm (D11 ruling) | ~6,400 | M | none (diag) |

Area: **0.8-1.4k** without a ruling; **+6.4k** with the probe ruling.

### mii (`src/runtime/mii` 15,167, `src/model/mii` 4,664, `src/model/mii_root` 1,083), carried

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
| [Shipped editor input snapshots wrote about 13.8 KB per session](#editor-snapshots) | behind `diag` or delete (pkg6b decision 1) | ~70 | L | none |

Area: **0.85-1.05k** without a ruling (plus the 560 never-compiled lines in §1); **2.5-2.9k** with the rulings.

### seams (`src/model/seams` 6,690, `src/runtime/seams` 648), preload (2,338), lobby (2,366 + 1,591), stages (829)

| Finding | Change | Saves | Risk | Rows |
| --- | --- | ---: | --- | --- |
| `model/guard.rs` (237) repeats the rules of `native_seams/mod.rs:38-130,546-837` (same enums, `guard_outcome`, `outcome_for`) | `native_seams` re-exports `guard` | 200-230 | L | s6-02/03, pkg1 boot install line |
| [644 seam rows plus 49 image rows, including unchecked entries](#unchecked-seam-rows) | delete the unchecked rows (keep `needle.set_cpu_controls`, `training.handle_effect`) | 1,300-1,550 | L-M | s6-02/03; owned-ELF host test |
| [Unread fields on all 693 rows and empty notes on 308](#unread-seam-fields) | drop the fields | in the next row | L | owned-ELF host test |
| [Redundant legacy shadow comparisons in authoritative groups](#legacy-shadow-comparisons) | delete | 130-165 | L | s6-03 `seams_shadow_diff=[]` |
| [INI-overlay functions lacked non-test callers after D7](#ini-overlay-functions) | delete | 250-300 | L | s6-01 |
| Same file: 8 on/off switches repeated across struct, defaults, parse, render, menu | one key/bit/field table, INI byte-identical | 150-250 | M | s6-01, INI first-boot row (none yet) |
| Same file `write_new`/`replace_live`/`recover_previous` (L1200-1288) vs `store::write_replace` | one writer (backup name changes) | ~40 | M | s6-01 |
| [Repeated notifications, hash reads, item gates and span queries](#repeated-lookups) | one each | 55-70 | L | lobby rows (none run) |
| [Site types overlapped across two tables](#overlapping-site-types) | one site type | 150-300 | H | every install-line row; defer |
| [Only unreachable adapters used the lobby automation helper](#unreachable-lobby-adapters) | park with them (L1) | ~1,500 | L | none |

Also found: `practice_stage_request::init` runs twice (`lobby/mod.rs:27`, `practice_stage_observer.rs:293`) and logs
`ui_wired=true` while no ticket is ever queued (defect, reported to the lead, not a size item).
Area: **1.9-2.3k** without a ruling; **+1.5k** parking, **+2.5k** compaction (§3).

### macros / recording (`src/runtime/macros` 3,761, `src/model/macros` 3,515, `src/model/macro_ui` 2,137), smash-match-data (3,735)

| Finding | Change | Saves | Risk | Rows |
| --- | --- | ---: | --- | --- |
| `model/macros/ui_anchor.rs` (122) and `model/macro_ui/replay_panel.rs` (137): no user (M) | delete (pkg4 manifest edit) | 259 | L | none |
| Named playback and slot transfer | removed in F7b; the panel, prompts and replay capture remain | measured in F7b | L | F7b-1–9 |
| Two copies of the fork's practice/lobby/save-state facade vs `runtime/lobby/training/{practice.rs,lobby.rs,mod.rs}` | `port` uses the lobby facade (inside the pkg4 wiring) | ~100 | M-H | s4-03/04, lobby rows |
| `InputFrame` (`macros/recording.rs:29`) = `MappedInputs` (`input_delay.rs:17`) | one type, JSON shape identical; F7b removed `StartSnapshot`/`Origin` | 60-70 | L-M | s4-01/02, s4-03/04 |
| [Duplicate constants, neutral checks, macro source and sync helpers](#repeated-macro-helpers) | one each | ~40 | L | s4-01 |
| Diag hook counters written 3× (`features/cpu.rs:117-178`, `states.rs:167-241`, `recorder.rs:91-145`) | one counter type beside `hook_stats::ENTRIES` | 120-150 | L | s2-02..04 hook budget, s4-01 |
| smash-match-data `capture.rs` (360), `clip.rs` (417), `validate.rs:1031-1141`, `Capture::load/save`: used only as a test oracle (`shared_capture.rs:482,734,769`) | vendor only what is used (lead ruling: the crate is carried unchanged) | 850-950 | M | s4-03/04 |

Area: **0.37k** without a ruling; **1.5k** with the smash-match-data ruling.

### features, menu, ui, model core (packages 1-5, clean room)

| Finding | Change | Saves | Risk | Rows |
| --- | --- | ---: | --- | --- |
| [Consolidated CPU parameters, pane bounds, hashes, constant tables and storage writes](#core-consolidations) | | 129 (M) | L | s2-02..04, s3-03 (constants by name) |
| [Seven comparison loops repeated UI and u4 checks](#repeated-row-comparisons) | one table registry in install-line order, `usable(name)`, `mark(name)` | 100-150 | M (`blocked_by` order) | tr105, s2-02, s4-01, w-01, u4-01, r2-01/02 |
| Install-and-mark lists `boot.rs:197-216,383-464,488-502`, `state_boot.rs:37-83` (15 names twice) | one table | 40-60 | L-M | s2-02..04, s3-03..10 |
| Game-constant mirrors `features/cpu.rs:49-92,373-411`, `states.rs:87-101,381-411` (one static per constant) | one array indexed by `C` | 70-90 | L-M | s2-02..04, s3-03 |
| `HookId` + `ALL` + `name()` (`hook_stats.rs:26-119`), `Feature` + `name()` (`sites.rs:20-117`) | one `macro_rules!` each, like `cpu/ids.rs` | 50-70 | L | s4-01 hook counts |
| Constants resolved twice per boot (`features/cpu.rs:360-446`, `states.rs:368-422`; 18 names in both lists) | one list (enum clash to solve) | 60-100 | M | s2-02..04, s3-03 |
| Diag probes that served (§4): `runtime/ui/probe.rs` 132, `ui/texture_probe.rs` 124 | retire | 256 | L | one diag boot |

Area: **0.6k** without the site compaction; **2.3-2.6k** more with it (§3).

## 3. Data tables written long (lead ruling 2026-11-02: no layout-only reductions)

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

## 4. Dead code, probes, doubled implementations

| Warnings at `bf836c8` | Evidence |
| --- | --- |
| Release warnings | 103 in integrator-3’s `/tmp/integrator3/warns-release2.tsv`. |
| Later retirement | F7 replaced `replay_preview.rs` with Recording playback. |

| File (count) | Class |
| --- | --- |
| `runtime/macros/port.rs` (24), `named_macros.rs` (4), `replay_preview.rs` (7) | [Pkg4 and screen wiring remained outstanding; several helpers were unused](#pending-wiring) |
| `runtime/ui/menu_app.rs` (11) | waiting: ui-screens (Mii list screen) |
| `mii_editor_bridge/editor_origin.rs` (16), `mii_saved_alpha.rs` (11), `mii_catalog_request.rs` (3) | waiting: pkg5/ui-screens Mii list screen (`request`, `menu_ready`) |
| `mii_replay.rs` (4) | wiring gap: macros call the `port.rs` stubs instead; wiring removes ~60 stub lines |
| `mii_auto_init.rs` (11) | [Caller search found callers for every item. Warning text needed review.](#mii-initialization-warnings) |
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

## 5. Totals and the target

| Step | Saves (E) | Count after (from ~114.0k with every stream merged) |
| --- | ---: | ---: |
| [A. Removing unreachable implementations within existing ownership](#step-a-savings) | 5.6-6.8k | ~107-108k |
| [B. Retiring probes and applying D11 rulings](#step-b-savings) | ~9-11k | ~97-99k |
| [C. Changing behavior or structure under separate rulings](#step-c-savings) | ~2-2.5k | ~95-96k |
| D. Data tables written compactly (§3) | ruled out (layout only) | — |
| E. Counter correction (§1), applied to both trees | ~3.7k (M: 3,738 at `7bb86fd`) | reported as "test-fixed" |

| November 2 projection | Basis |
| --- | --- |
| Steps A–C | Approximately 95–96k, the lead’s reported target, roughly 15% below the fork’s 112.5k. |
| 90k | Required counter correction E, which would lower the fork count too. |
| Pending wiring and screens | Expected to raise the count. Any addition above budget required a recorded raise. |

Done so far (as measured): `7bb86fd` 106,983 → stream head 106,785 on `566780a` (107,470): the lint-allow roots and
one-implementation wins (129) and three never-compiled files (556).

## Seam rows (Phase B slice 1, lead override of D6, 2026-11-02)

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

## smash-match-data (Phase B, lead ruling 2026-11-02)

| November 2 disposition | API or evidence |
| --- | --- |
| Kept for the plugin | `archive` writer, manifest and timeline reader. `model`. `validate_header` and `StreamChecker` in `validate`. |
| Test-only destination | `crates/smash-match-data/tests/oracle`. Other consumers could also use `smash-match-data-design` at `faac04e`. |
| `capture.rs` | `Recorder`, `RecorderOptions`, `Finished`, `CaptureError`. |
| `clip.rs` | `clip_context*`, `clip_events*`, `selection_end*`, `ClipContext`, `Resolved`, `SelectAt`, `Capture` clip methods as `CaptureClip`. |
| `whole.rs`, `CaptureExt` | `Capture::load`, `Capture::save`, `validate_capture`. |
| External format | Parity tests compared the writer with `Recorder`. Golden files pinned published bytes. |
| Count | 109,108 → 108,159 (−949). |

## 6. Phase B order (after stream/sync and stream/stack are merged)

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

## Capacity probe retirement — 1c0104e (2026-11-02)

| Capacity retirement at `1c0104e` | November 2 measurement |
| --- | --- |
| Change | pkg6 manifest transforms retired approved capacity/lifecycle experiments and eight empty aliases. |
| Raw sloc4 | 101,224, excluding 1,118 simulator lines; −3,126 from 104,350, −5,728 from `ebfcb98` (106,952). |
| Raw budget | 105,600 included 4,318 pending merge lines, under the same counting rule. |
| Normalized tool | `sloc-fmt.ps1`, rustfmt 1.9.0-stable `48a229cea`, zero failures. |
| Components | 125,979 src + 3,434 crates + 167 tools = 129,580. |
| Excluding tools | 129,413: −3,928 from 133,341 and −6,656 from normalized base 136,069. |
| Fork comparison | 131,081 excluding tools. RC ratchet improved. Next-version 15% reduction remained outstanding. |

## Served Mii probes and helper scopes (2026-11-02)

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

## Hook identifier declaration (2026-11-02)

| Hook mapping | Result at `8f762e1` |
| --- | --- |
| Declaration | One table in `model/hook_stats.rs` defined 40 HookId variants, ALL ordering and report labels. |
| Review | Removed three maintained mappings. Source comparison preserved ordinals, labels and variant comments. |
| Unaffected code | Allocator, scope, counters and formatter. |
| Raw sloc4 | 98,885 (−77 from `61b0aab`). Ceiling 103,300. |
| Normalized excluding tools | 126,472 (−77). All five compiler gates passed. |
| Resident bytes | Release 3,444,736 (+4,096 versus `61b0aab`). Diagnostic 3,784,704, unchanged. |
| Runtime allocation failures | Still open at this checkpoint. |

## Feature declaration (2026-11-02)

| Feature declaration at `a6e4d07` | November 2 measurement |
| --- | --- |
| Declaration | All 28 Feature variants, names and variant documentation shared one declaration. |
| Parity | Site rows, bytes, rules and later definitions remained byte-identical. |
| Counts | 98,865 raw (−20 from `8f762e1`), 126,452 normalized excluding tools (−20). |
| Compiler gates | All five passed. |
| Resident bytes | Release 3,444,736, diagnostic 3,784,704. Both matched `8f762e1`. |
| Ceiling | 98,865 + 4,318 pending = 103,183, rounded to 103,200. |

## Keyboard value and validation root (78f440d)

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

### Normalized gate migration (operator/lead23:11, 2026-11-02)

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

### Diag-only code is not counted (operator decision, 2026-11-14)

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

### rc.15 Lucas112 native charge release (2026-11-16)

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

### rc.15 Dedede99 real up-special entry (2026-11-16)

| Dedede99 checkpoint | November 16 record |
| --- | --- |
| Canonical Surface count | `3fd57e54` 150,991 → `bd7f4d90` 150,992 (+1). |
| Budget | Same ceiling and evaluator, under rc.15 final-total size approval. |
| Acceptance at checkpoint | Primary review and Eden contact/escape proof remained pending. |
| Evidence | `dedede99-gate-bd7f4d90-surface/SUMMARY.md`, `BUDGET-ENTRY.md` and retained `compliance-v2.log`. |
| Trace correction | Test references corrected without changing product or gate. |

### rc.15 Ice Climbers113/114 and ZSS108 (2026-11-17)

| Ice Climbers and Flip stomp checkpoint | November 17 record |
| --- | --- |
| Canonical gate | `2e36e0bd` 150,992, equal to `bd7f4d90`. |
| Review | Primary approved the size with the IC113/114 Opus pass. |
| Flip stomp108 | Reused SPECIAL_LW admission and native Bury observation; canonical count was pending at this checkpoint. |
| Budget | Counter and ceiling unchanged. Final integration owned the approved final-total adjustment. |

### rc15 Mash out Dedede99 native cancel continuation

| Dedede99 continuation | rc.15 checkpoint |
| --- | --- |
| Observation | Native landing hit interrupted real Bury. |
| Adjustment | Applied native SPECIAL once in Held after CPU hitstop via existing followed/controls/drive/revoke. |
| Scope | Kept existing reader, hook, model, clock, guard and constants. |
| Held probes | Seed, stop, one edge, escape and other plan. |
| Budget at checkpoint | Canonical normalized count pending under rc.15 final-total approval. Ceiling unchanged. |
| Evidence | Native evidence and RED/GREEN in `dedede99-interruption` lane artifacts. |

#### Unmerged stream contributions

- M: pkg6 +2,416 (mostly new probes: `mii_fullconfig_probe`, `mii_identity_probe`, `mii_saved_file_probe`, `item_state_probe`, `turnip_face_caller_probe`), hitbox +1,272, u4b +990, ui-screens +721, stack +640, sync +495 (`crates/hos-lock` 443), f6 +481, deluxe +83, pkg4 −37

#### Legacy roster fingerprints

- Old word fingerprints next to the 64-byte seam guards: `character_roster.rs:2304-2383`, `native_body_bank.rs:3813-3927`, `character_lifecycle.rs:27-65,149-158`; the groups are `AUTHORITATIVE` (`seam_install.rs:55-86`) and `gate()` (`seam_install.rs:298`) ignores the legacy result

#### Repeated native wrappers

- Same native wrapped twice: 0x17e2880/0x17e8890/0x353ea90 in `native_source.rs:161-169` and `source_owner_probe.rs:213-222`; `OWNER_TABLE` 0x5324680 in 12 files; `ACQUIRE_KIND/RELEASE_KIND` in `native_body_bank.rs:245-252` and `r_handle.rs:38-40`

#### Editor snapshots

- Editor input-region snapshots in the shipped build (`mii_editor_bridge/native.rs:404-459`, `mod.rs:57,81,94-100`, `connected.rs:199-234`; ~13.8 KB per session to `mii-editor-last.json`)

#### Unchecked seam rows

- 644 seam rows (M: a1 94, a2 281, a3 82, a4 42, a5 127, identity 18) + 49 image rows; rows with no gate/site call and no outside reference (M): `training` 77 of 79, mii_quit 19, mii_preview 15, mii_fullconfig 14, kirby_resource 10, … and 37 image rows of modules not in the clean room

#### Unread seam fields

- Row fields never read (M): `scopes` (`Scopes::ALL` on all 693), `read_only`, `per_frame`; `note` empty on 308

#### Legacy shadow comparisons

- Shadow-compare `legacy` word tables though every group is authoritative: `lobby_stage_probe` ~45, `practice_stage_observer` ~50, `practice_stage_request_words.rs` ~22, rotation ~12, `game_mode` ~6, installer shadow branch ~30

#### INI overlay functions

- `cycle_preload_config.rs`: INI-overlay code with no non-test caller since D7 (`adopt_unset` L641, `Unset` L573, `apply_settings` L778, `pack`/`unpack` L459-542, 8 `*_enabled`)

#### Repeated lookups

- Small doubles: three notification fns `training/mod.rs:122-145`; stage-catalog hash read 3×; item gate 2× (`practice_stage_request.rs:171`, `practice_stage_observer.rs:86`); QueryMemory `span` 3× (request, `effect_hook_chain`, `mii_ordinary_native`)

#### Overlapping site types

- `sites::Site` vs `native_seams::Site`: 69 of 82 rows_a3 rows and 3 hazard rows on the same offsets as `sites*.rs` rows

#### Unreachable lobby adapters

- Lobby automation helper pure half (adapter, control, flow, popup, pulse 1,379) used only by the never-compiled `native.rs`/`plugin.rs`

#### Repeated macro helpers

- Constants and `is_neutral` 2× (`model/recording.rs:30-55` vs `macro_panel.rs:16-76,1661`); `MacroSource` 2× (`port.rs:141`, `compat/menu.rs:35`); sync helper 2× (`port.rs:6-23`, `compat/sync.rs`)

#### Core consolidations

- **Done in this stream:** six `too_many_arguments` in `model/cpu` (`mash::Step`, `ledge::TermQuery`, `FrameFacts`), pane box `[x, y, w, h]`, crc32/hash40 2× (`cpu/hash.rs` now re-exports `model::hash40`), the constant-table implementation 2× (`cpu/ids.rs`, `state/ids.rs`: one `game_constants!`), `Store::write_atomic` = `write_replace`

#### Repeated row comparisons

- "Compare every row" loop 7× (`boot.rs:127-154`, `state_boot.rs:15-21`, `p1/mod.rs:27-29`, `ui/mod.rs:100-103`, `ui/texture.rs:76`); UI and u4 rows compared twice

#### Pending wiring

- waiting: stream/pkg4 wiring and ui-screens (the roster cycle now exists, so the `port.rs:72-139` stubs can go when wired); `Legacy`/`has_captured_frame`/`frame_at`/`capture_active`/`playback_active`/`using_frames` are truly dead

#### Mii initialization warnings

- not explained by a caller search (every item has a caller): re-check against the warning text

#### Step A savings

- A. Rulings not needed beyond file ownership: never-compiled files 1.1k, the "without a ruling" rows of §2 (roster 0.8-1.4k, mii 0.85-1.05k, seams/lobby 1.9-2.3k, macros 0.37k, core 0.6k incl. 129 done)

#### Step B savings

- B. Probe and D11 rulings: roster/Mii probes and observers ~7k, pkg6's new probes 2.4k if they serve their run and retire, Mii D11 fold 0.2k, automation-helper parking 1.5k

#### Step C savings

- C. Behaviour/structure rulings: player/cpu generic, saved_alpha fallback, smash-match-data trim, seam-label lookup, one site type, config switch table
