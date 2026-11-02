# Size plan: a clean room smaller than the fork (2026-11-02, slim stream)

## Current operator criterion (2026-11-02 14:2xZ; relayed by Astra lead)
This supersedes the historical RC target below: measure a scratch rustfmt-normalized copy with
the operator's tools/sloc-fmt.ps1 (sloc4 semantics). At least 15% under the fork is a NEXT-version
goal; RC size must ratchet down. Keep raw sloc4 alongside normalized measurements. No layout
compression, counting-rule change or behavior loss counts as a reduction.
At98f9ab0: raw104350; normalized133508 including tools167 (133341 excluding scripts),
rustfmt1.9.0-stable,0 formatting failures. Primary's normalized base ebfcb98=136069 excluding
scripts; fork M5q=131081 excluding scripts. Same-basis contribution=-2728 lines; current is
still2260 above fork. This is source size, not NRO resident size. Pending additions remain
itemized in size-budget.toml; integration must remeasure and ratchet.


Goal (operator, 2026-11-02): the implementation smaller than the fork's (M5q = 112,500), target **≤ 90,000 at RC**,
without dropping a feature or proven behaviour. Means: delete dead code (not hide it), retire probes that served,
tables instead of repeated arms/structs, shared abstractions for the carried engines' repeated patterns, one
implementation per concept.

Count: `python3 -I tools/sloc.py git <repo> <rev> 3` (the primary's `sloc4.py` rules since 2026-11-02; `third_party/`
excluded and reported apart). Numbers below marked sloc3 were counted before the switch. Gate: `python3 -I tools/size_gate.py` against `size-budget.toml`, run by `cargo test`
(`tests/size_budget.rs`). "M" = measured, "E" = estimate. File:line references are at `7bb86fd`.

## 1. Where the count stands

| | Lines | |
| --- | ---: | --- |
| Fork M5q | 112,500 | operator |
| Clean room `7bb86fd` | 106,983 | M |
| Pending in unmerged streams (work tree minus merge base, ~12:10Z) | +7,061 | M: pkg6 +2,416 (mostly new probes: `mii_fullconfig_probe`, `mii_identity_probe`, `mii_saved_file_probe`, `item_state_probe`, `turnip_face_caller_probe`), hitbox +1,272, u4b +990, ui-screens +721, stack +640, sync +495 (`crates/hos-lock` 443), f6 +481, deluxe +83, pkg4 −37 |
| Deluxe SDK under `third_party/` | 599 | M, not counted (vendored) |
| If merged as they are | ~114,044 | above the fork |
| Budget start (`size-budget.toml`) | 114,100 | measured + pending, rounded up |

**Counter finding (operator decision, not a saving).** `sloc3.py` drops a `#[cfg(test)] mod x;` line but counts the
file it points to, and keeps blocks gated by `cfg(all(test, feature = …))` or `cfg(any(test, feature = "simulator"))`.
Test-only files counted as implementation (M): `macros/library/tests.rs` 624, `native_seams/tests.rs` 601,
`swap_model/tests.rs` 456, `p1/tests.rs` 305, `macros/window/tests.rs` 265, `macros/recording/tests.rs` 202,
`preload/ini_transactions_tests.rs` 160, `p1/move_observer_tests.rs` 146, `macros/test_util.rs` 64,
`macros/move_crop_continuation_tests.rs` 59, `lobby/session_lease_tests.rs` 26 = **2,908**; plus `swap/mii_cycle.rs:315`
tests under `cfg(all(test, feature = "mii-saved-alpha"))` **755**, and `swap_model/native_model.rs` (848) under
`cfg(any(test, feature = "simulator"))` (host-only feature). Many of these are carried, so the fork's 112,500 has
them too: a corrected counter must be applied to both trees, or the comparison is unfair. **Resolved 2026-11-02:** the
primary's `sloc4.py` counts these as tests and the simulator apart, in both trees: fork M5q 106,629, clean room
`21334c0` 106,801, simulator 1,118 in both; target <= 90,000 at RC, stretch <= 85,300. `tools/sloc.py` follows sloc4:
`8b73694` 106,099; stream/slim at `eb8e034` 103,497.

**Never compiled but counted (M, `/tmp/slim_orphans.py`: no `mod` line or `#[path]` reaches them).**
`runtime/lobby/automation_helper/native.rs` 425, `…/plugin.rs` 58, `runtime/mii/mii_native_adapter.rs` 218,
`mii_cpu_adapter.rs` 199, `mii_closed_ack.rs` 143, `runtime/roster/effect_hook_chain.rs` 73 = **1,116**, all in the
pkg6 carry manifest; `runtime/macros/capture_plugin.rs` 84. Status: `native.rs`, `plugin.rs` and
`effect_hook_chain.rs` deleted (`e79019b`, recorded as not carried); the three Mii adapter files stay because
stream/pkg6's new `mii_fullconfig_probe` reaches them by `#[path]` (they leave with that probe if it retires);
`capture_plugin.rs` is deleted on stream/deluxe (`166d3cd`) with its carry record.

## 2. Per area

Savings are counted lines (E unless marked). Risk: L/M/H. "Rows" = Eden rows that must pass again after the change.

### roster (`src/runtime/roster` 19,551) and swap (`src/model/swap` 7,980), carried (pkg6 manifest)

No clean-room Eden row has exercised the roster cycle yet (EDEN-PACKAGE-6: s6-01 sync probe, s6-02/03 boot and
CPU-as-P1; "Not run: CP0/CP1/CP3 … needs the roster cycle"). Rows below are the planned ones of
docs/PACKAGE-6-PLAN.md §8 (sets A/B/D, R-ID, R-PC) and must exist as a baseline before a roster refactor.

| Finding (examples) | Change | Saves | Risk | Rows |
| --- | --- | ---: | --- | --- |
| Old word fingerprints next to the 64-byte seam guards: `character_roster.rs:2304-2383`, `native_body_bank.rs:3813-3927`, `character_lifecycle.rs:27-65,149-158`; the groups are `AUTHORITATIVE` (`seam_install.rs:55-86`) and `gate()` (`seam_install.rs:298`) ignores the legacy result | delete the legacy words, keep relocated-data checks | ~190 | M | s6-02/03 install line, set A boot |
| `mii_reload_probe` module `character_roster.rs:2434-2835` names modules that do not exist at `7bb86fd` (`crate::mii_fullconfig_probe` …) | becomes compilable when stream/pkg6 lands those probes; retire with them (D11) | ~350 | L (not shipped) | none |
| Identical `trace!` macro defined 11 times (`copy_packages.rs:19`, `special_articles.rs:15`, …) | one crate macro | ~22 | L | none |
| `identity_view.rs:184-343`: 14 near-identical inline hooks | one `macro_rules!` table (site, offset, registers, scope) | ~60 | M (hook code) | R-ID0..9, R-PC1 |
| `cycle_preload_config.rs:66-182`: 4 logs of 11-15 fields | `Display` in the model, text byte-identical | ~45 | L | preload rows |
| Same native wrapped twice: 0x17e2880/0x17e8890/0x353ea90 in `native_source.rs:161-169` and `source_owner_probe.rs:213-222`; `OWNER_TABLE` 0x5324680 in 12 files; `ACQUIRE_KIND/RELEASE_KIND` in `native_body_bank.rs:245-252` and `r_handle.rs:38-40` | one roster offsets table, one wrapper per native (extend `tests/native_wrappers.rs` beyond `game.rs`/`state_natives.rs`) | ~30 (rest leaves with the probes) | L | R-PC1/PC2 |
| `cfg(not(alpha))` alternates for builds never shipped (`character_sources.rs:1555-1608` …) | one version via `slot_allowed()`; all branches if D11 is relaxed | 40 / ~205 | L / M | set D, R-PC1 |
| Model functions only tests use (`body_bank.rs:287,508,568,836`, `ledger.rs:261,438,587,633-662`, `lifecycle.rs:306,350,381,399`, `roster_grid.rs:254-467` …), `swap_model/hud_model.rs` 68, `target.rs` 41 | delete with their tests (check roster_grid against the screens first) | ~360 | L-M | none |
| Probes and observers (§4): ~6.4k incl. ~520 probe-gated lines inside product files | retire after the owners confirm (D11 ruling) | ~6,400 | M | none (diag) |

Area: **0.8-1.4k** without a ruling; **+6.4k** with the probe ruling.

### mii (`src/runtime/mii` 15,167, `src/model/mii` 4,664, `src/model/mii_root` 1,083), carried

The default features include every `mii-*-alpha` (Cargo.toml:5, 55-62); 187 cfg lines exist only for subset builds
(D11). No Eden row in this repo exercises Mii; R-MII1 (sets A/B) and set D are planned, not run.

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
| Editor input-region snapshots in the shipped build (`mii_editor_bridge/native.rs:404-459`, `mod.rs:57,81,94-100`, `connected.rs:199-234`; ~13.8 KB per session to `mii-editor-last.json`) | behind `diag` or delete (pkg6b decision 1) | ~70 | L | none |

Area: **0.85-1.05k** without a ruling (plus the 560 never-compiled lines in §1); **2.5-2.9k** with the rulings.

### seams (`src/model/seams` 6,690, `src/runtime/seams` 648), preload (2,338), lobby (2,366 + 1,591), stages (829)

| Finding | Change | Saves | Risk | Rows |
| --- | --- | ---: | --- | --- |
| `model/guard.rs` (237) repeats the rules of `native_seams/mod.rs:38-130,546-837` (same enums, `guard_outcome`, `outcome_for`) | `native_seams` re-exports `guard` | 200-230 | L | s6-02/03, pkg1 boot install line |
| 644 seam rows (M: a1 94, a2 281, a3 82, a4 42, a5 127, identity 18) + 49 image rows; rows with no gate/site call and no outside reference (M): `training` 77 of 79, mii_quit 19, mii_preview 15, mii_fullconfig 14, kirby_resource 10, … and 37 image rows of modules not in the clean room | delete the unchecked rows (keep `needle.set_cpu_controls`, `training.handle_effect`) | 1,300-1,550 | L-M | s6-02/03; owned-ELF host test |
| Row fields never read (M): `scopes` (`Scopes::ALL` on all 693), `read_only`, `per_frame`; `note` empty on 308 | drop the fields | in the next row | L | owned-ELF host test |
| Shadow-compare `legacy` word tables though every group is authoritative: `lobby_stage_probe` ~45, `practice_stage_observer` ~50, `practice_stage_request_words.rs` ~22, rotation ~12, `game_mode` ~6, installer shadow branch ~30 | delete | 130-165 | L | s6-03 `seams_shadow_diff=[]` |
| `cycle_preload_config.rs`: INI-overlay code with no non-test caller since D7 (`adopt_unset` L641, `Unset` L573, `apply_settings` L778, `pack`/`unpack` L459-542, 8 `*_enabled`) | delete | 250-300 | L | s6-01 |
| Same file: 8 on/off switches repeated across struct, defaults, parse, render, menu | one key/bit/field table, INI byte-identical | 150-250 | M | s6-01, INI first-boot row (none yet) |
| Same file `write_new`/`replace_live`/`recover_previous` (L1200-1288) vs `store::write_replace` | one writer (backup name changes) | ~40 | M | s6-01 |
| Small doubles: three notification fns `training/mod.rs:122-145`; stage-catalog hash read 3×; item gate 2× (`practice_stage_request.rs:171`, `practice_stage_observer.rs:86`); QueryMemory `span` 3× (request, `effect_hook_chain`, `mii_ordinary_native`) | one each | 55-70 | L | lobby rows (none run) |
| `sites::Site` vs `native_seams::Site`: 69 of 82 rows_a3 rows and 3 hazard rows on the same offsets as `sites*.rs` rows | one site type | 150-300 | H | every install-line row; defer |
| Lobby automation helper pure half (adapter, control, flow, popup, pulse 1,379) used only by the never-compiled `native.rs`/`plugin.rs` | park with them (L1) | ~1,500 | L | none |

Also found: `practice_stage_request::init` runs twice (`lobby/mod.rs:27`, `practice_stage_observer.rs:293`) and logs
`ui_wired=true` while no ticket is ever queued (defect, reported to the lead, not a size item).
Area: **1.9-2.3k** without a ruling; **+1.5k** parking, **+2.5k** compaction (§3).

### macros / recording (`src/runtime/macros` 3,761, `src/model/macros` 3,515, `src/model/macro_ui` 2,137), smash-match-data (3,735)

| Finding | Change | Saves | Risk | Rows |
| --- | --- | ---: | --- | --- |
| `model/macros/ui_anchor.rs` (122) and `model/macro_ui/replay_panel.rs` (137): no user (M) | delete (pkg4 manifest edit) | 259 | L | none |
| `named_macros.rs:337-345,870-888` (`Legacy`), `:734-775`, `:492-503` superseded by the recorder state | delete | ~60 | L | s4-03/04 |
| Two copies of the fork's practice/lobby/save-state facade: `runtime/macros/port.rs:155-286` vs `runtime/lobby/training/{practice.rs:17-41,lobby.rs:21-57,mod.rs:64-127}` | `port` uses the lobby facade (inside the pkg4 wiring) | ~100 | M-H | s4-03/04, lobby rows |
| `InputFrame` (`macros/recording.rs:29`) = `MappedInputs` (`input_delay.rs:17`); `StartSnapshot`/`Origin` vs `StartState` | one type, JSON shape identical | 60-70 | L-M | s4-01/02, s4-03/04 |
| Constants and `is_neutral` 2× (`model/recording.rs:30-55` vs `macro_panel.rs:16-76,1661`); `MacroSource` 2× (`port.rs:141`, `compat/menu.rs:35`); sync helper 2× (`port.rs:6-23`, `compat/sync.rs`) | one each | ~40 | L | s4-01 |
| Diag hook counters written 3× (`features/cpu.rs:117-178`, `states.rs:167-241`, `recorder.rs:91-145`) | one counter type beside `hook_stats::ENTRIES` | 120-150 | L | s2-02..04 hook budget, s4-01 |
| smash-match-data `capture.rs` (360), `clip.rs` (417), `validate.rs:1031-1141`, `Capture::load/save`: used only as a test oracle (`shared_capture.rs:482,734,769`) | vendor only what is used (lead ruling: the crate is carried unchanged) | 850-950 | M | s4-03/04 |

Area: **0.37k** without a ruling; **1.5k** with the smash-match-data ruling.

### features, menu, ui, model core (packages 1-5, clean room)

| Finding | Change | Saves | Risk | Rows |
| --- | --- | ---: | --- | --- |
| **Done in this stream:** six `too_many_arguments` in `model/cpu` (`mash::Step`, `ledge::TermQuery`, `FrameFacts`), pane box `[x, y, w, h]`, crc32/hash40 2× (`cpu/hash.rs` now re-exports `model::hash40`), the constant-table implementation 2× (`cpu/ids.rs`, `state/ids.rs`: one `game_constants!`), `Store::write_atomic` = `write_replace` | | 129 (M) | L | s2-02..04, s3-03 (constants by name) |
| "Compare every row" loop 7× (`boot.rs:127-154`, `state_boot.rs:15-21`, `p1/mod.rs:27-29`, `ui/mod.rs:100-103`, `ui/texture.rs:76`); UI and u4 rows compared twice | one table registry in install-line order, `usable(name)`, `mark(name)` | 100-150 | M (`blocked_by` order) | tr105, s2-02, s4-01, w-01, u4-01, r2-01/02 |
| Install-and-mark lists `boot.rs:197-216,383-464,488-502`, `state_boot.rs:37-83` (15 names twice) | one table | 40-60 | L-M | s2-02..04, s3-03..10 |
| Game-constant mirrors `features/cpu.rs:49-92,373-411`, `states.rs:87-101,381-411` (one static per constant) | one array indexed by `C` | 70-90 | L-M | s2-02..04, s3-03 |
| `HookId` + `ALL` + `name()` (`hook_stats.rs:26-119`), `Feature` + `name()` (`sites.rs:20-117`) | one `macro_rules!` each, like `cpu/ids.rs` | 50-70 | L | s4-01 hook counts |
| Constants resolved twice per boot (`features/cpu.rs:360-446`, `states.rs:368-422`; 18 names in both lists) | one list (enum clash to solve) | 60-100 | M | s2-02..04, s3-03 |
| Diag probes that served (§4): `runtime/ui/probe.rs` 132, `ui/texture_probe.rs` 124 | retire | 256 | L | one diag boot |

Area: **0.6k** without the site compaction; **2.3-2.6k** more with it (§3).

## 3. Data tables written long (lead ruling 2026-11-02: no layout-only reductions)

Ruling: reformatting rows to cut lines is not a reduction; the count assumes rustfmt's default layout. Table-driven rows
that remove real code are welcome. Kept below as the finding it was.

**Layout finding.** The tree is not rustfmt-formatted (`cargo fmt --all -- --check -l`: 444 of 508 tracked `.rs`
files). On a scratch `git archive` copy of `e79019b` laid out by rustfmt the count is 136,814 against 106,785 as
committed (sloc3). The `--normalize` report mode that measured this was dropped: the primary's sloc4 did not adopt it.

Site rows (M): 239 rows in `sites*.rs`, 13 lines each, guard as 4 lines of 16 bytes; seam rows 5-10 lines each. One
header line plus the 64 guard bytes on one line keeps `compliance.py`'s `guard: [0x.. ×64]` row passing (a byte string
or `include_bytes!` would fail it) and is already the form of `rows_a1.rs:11-15`. Saves 2,390-2,630 (sites) and
~2,500 (seams after the row deletions). The values do not change (the owned-ELF host tests compare compiled tables);
the generators `tools/gen_sites_*.py` must emit the new form. It is a change of layout, not of code: the lead and the
operator decide whether it counts.

## 4. Dead code, probes, doubled implementations

**Release dead-code warnings at `bf836c8` (103, integrator-3's `/tmp/integrator3/warns-release2.tsv`):**

| File (count) | Class |
| --- | --- |
| `runtime/macros/port.rs` (24), `named_macros.rs` (4), `replay_preview.rs` (7) | waiting: stream/pkg4 wiring and ui-screens (the roster cycle now exists, so the `port.rs:72-139` stubs can go when wired); `Legacy`/`has_captured_frame`/`frame_at`/`capture_active`/`playback_active`/`using_frames` are truly dead |
| `runtime/ui/menu_app.rs` (11) | waiting: ui-screens (Mii list screen) |
| `mii_editor_bridge/editor_origin.rs` (16), `mii_saved_alpha.rs` (11), `mii_catalog_request.rs` (3) | waiting: pkg5/ui-screens Mii list screen (`request`, `menu_ready`) |
| `mii_replay.rs` (4) | wiring gap: macros call the `port.rs` stubs instead; wiring removes ~60 stub lines |
| `mii_auto_init.rs` (11) | not explained by a caller search (every item has a caller): re-check against the warning text |
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

**Doubled implementations:** `model/guard.rs` vs `native_seams` rules; `sites::Site` vs the seam `Site`; legacy words
vs seam rows (roster, Mii, lobby); two fingerprint schemes (Mii); cpu/state constant tables and crc32 (done here); the
config INI writer vs `store::write_replace`; `span` 3×, item gate 2×, catalog hash 3×; `port.rs` facade vs the lobby
facade; `InputFrame` vs `MappedInputs`; diag hook counters 3×; natives wrapped twice in roster probes.

## 5. Totals and the target

| Step | Saves (E) | Count after (from ~114.0k with every stream merged) |
| --- | ---: | ---: |
| A. Rulings not needed beyond file ownership: never-compiled files 1.1k, the "without a ruling" rows of §2 (roster 0.8-1.4k, mii 0.85-1.05k, seams/lobby 1.9-2.3k, macros 0.37k, core 0.6k incl. 129 done) | 5.6-6.8k | ~107-108k |
| B. Probe and D11 rulings: roster/Mii probes and observers ~7k, pkg6's new probes 2.4k if they serve their run and retire, Mii D11 fold 0.2k, automation-helper parking 1.5k | ~9-11k | ~97-99k |
| C. Behaviour/structure rulings: player/cpu generic, saved_alpha fallback, smash-match-data trim, seam-label lookup, one site type, config switch table | ~2-2.5k | ~95-96k |
| D. Data tables written compactly (§3) | ruled out (layout only) | — |
| E. Counter correction (§1), applied to both trees | ~3.7k (M: 3,738 at `7bb86fd`) | reported as "test-fixed" |

**Is 90k reachable?** Not by removals alone: A, B and C give about **95-96k** as measured (about 15 % under the
fork's 112.5k); the target the lead reports is that number. 90k would need a measurement decision (E, which also
lowers the fork's number), not a code change. Stream additions still to come (wiring, screens) push the other way;
every addition above the budget needs a recorded raise.

Done so far (as measured): `7bb86fd` 106,983 → stream head 106,785 on `566780a` (107,470): the lint-allow roots and
one-implementation wins (129) and three never-compiled files (556).

## Seam rows (Phase B slice 1, lead override of D6, 2026-11-02)

`python3 -I tools/seam_census.py <tree>` lists, per group, the rows with a consumer: a Rust file outside the row files and
outside test code holding the row's label as a literal, its offset as a hex literal, the `SITE_<NAME>` constant of a
needle row, or another row naming it (`OwnHook`/`EntryOrBody`). An offset match is counted as a consumer even when it is a
coincidence (another table guarding the same function), so the deleted set is the provable one, not the largest.
Before the slice, on stream/slim at `7a9bd4a` (693 rows), the groups with unconsumed rows:

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

The library keeps what the plugin uses: `archive` (writer, manifest, timeline reader), `model`, `validate`
(`validate_header`, `StreamChecker`). Moved to the test-only oracle crate `crates/smash-match-data/tests/oracle`, where a
consumer that needs them takes them (or from the upstream `smash-match-data-design` crate, git `faac04e`): `capture.rs`
(`Recorder`, `RecorderOptions`, `Finished`, `CaptureError`), `clip.rs` (`clip_context*`, `clip_events*`,
`selection_end*`, `ClipContext`, `Resolved`, `SelectAt`, the `Capture` clip methods as `CaptureClip`), `Capture::load`,
`Capture::save` and `validate_capture` (`whole.rs`, `CaptureExt`). The plugin's parity tests still compare its writer with
the `Recorder`; golden files pin the published bytes for outside readers. Count 109,108 → 108,159 (−949).

## 6. Phase B order (after stream/sync and stream/stack are merged)

1. Never-compiled files and the uncompilable probe code (no Eden: the NRO does not change; carry-manifest edits).
2. Seams: `guard.rs` as the one rule set, unchecked rows and unread fields, legacy shadow tables; boot install-line rows
   (s6-02/03 shape, r2-01/02, u4-01, tr105, s2-02) plus the owned-ELF host tests.
3. Probes after the owners' answers (diag boot only; `slots!` lines with them).
4. Clean-room core: table registry, install table, constant mirrors, hook counters (s2-02..04, s3-03..10, s4-01, w-01).
5. Macros after the pkg4 wiring (s4-01..04, WIRE).
6. Roster and Mii refactors only after package 6's Eden sets A/B/D exist as a baseline (no clean-room row exercises
   the roster cycle or Mii today), then R-ID, R-PC, R-MII1 and set D after each slice.

## Capacity probe retirement — 1c0104e (2026-11-02)

Approved capacity/lifecycle experiments and eight empty aliases retired through pkg6 manifest transforms. Raw sloc4 is101224 (simulator1118 excluded), down3126 from104350 and5728 from authorized baseebfcb98=106952. Budget105600 retains4318pending merge allowance; no counting rule change.

Operator sloc-fmt.ps1 on git1c0104e: rustfmt1.9.0-stable48a229cea, zero failures; src125979+crates3434+tools167=129580. Excluding tools:129413, down3928 from133341 and6656 from normalized base136069. Fork reference131081 excludingtools; next-version15% reduction goal still pending. This is an RC ratchet contribution, not completion of next-version target.

## Served Mii probes and helper scopes (2026-11-02)

Raw sloc4:98,956 (src95,449/crates3,356/tools151); simulator1,118 excluded, tests28,416 excluded. This slice removes2,268 implementation lines after capacity1c0104e. Six Mii experiment files/seven features retired, shared native helpers and exact-write-set regression retained; twelve dead-code suppressions removed through consumer feature/test scopes, with no new suppression. Two stage service module names now describe shipped behavior; historical seam metadata remains. Budget103,300 =98,956+4,318pending rounded up, down from105,600. Normalized measurement and host/Switch gates pending; shared runtime failures remain open.

Normalizedf07bda2 via operator sloc-fmt.ps1:126710incltools167 =126543excltools; simulator2102excluded, tests44339excluded; rustfmt1.9.0-stable48a229cea,0failures. Reduction2870from1c0104e,9526fromauthorizedbase136069. The next-version15%target remains unmet; current RC criterion is monotone ratchet.

Helper feature-scope follow-up: raw98,962 (+6 versusf07bda2; ceiling103,300 unchanged), removing the last pending module-wide dead_code allow with actual feature/test scopes. Normalized count and compiler gates pending; no claim of further line reduction for this lint root. Five large-enum ratchet rows remain pending alongside the two approved hos-lock allows.

Normalized10a8232=126725includingtools167 (126558excludingtools), +15 versusf07bda2, -9511 versusauthorizedbase136069; rustfmt1.9.0-stable48a229cea,0failures. This is the explicit feature-scope cost of retiring a broad dead-code allowance; budget not raised.
