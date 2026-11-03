# Resource costs of the upstream-origin features

## Current clean-room RC archive accounting

Primary decision relayed in TRAININGROOMS-LEAD-INBOX.md under PRIMARY DECISION ~05:5xZ; operator informed and may override. Keep the existing two preallocated archive working buffers for RC to preserve zero allocations in the archive hook. Current recipe maximum is 1,570,692 bytes per buffer: 3,141,384 bytes (about 3.0 MiB) requested capacity total, plus allocator/bookkeeping overhead. Five-second release means logical hand-out retirement and return to the pool; it does not mean heap or RSS reclamation.

Mandatory acceptance remains zero hook allocations, at most two buffers, at least five seconds of hand-out grace, repeated-load correctness and measured headroom. Count the reserved capacity separately from NRO image residency and include it in the 10-fighter plus 5-hat hot-slot memory measurement. Logical payload lengths, requested allocation capacity, allocator residency and host RSS are different measurements; do not add the buffer cost again to a measured heap total that already includes it.

Freeing between loads through qualified off-hook preparation becomes next-version memory work, not an RC blocker. core05.12 closes only on the corrected requirement plus actual runtime evidence; this decision alone is not a PASS.


Historical research tables below retain their original scope and do not describe the current clean-room runtime. Current r13 diagnostic NRO image residency is 6,164,480 bytes; adding the separately requested archive capacity gives 9,305,864 bytes before archive bookkeeping and all other dynamic allocations. This arithmetic is not total process/native RSS or a final release measurement. UI18 runtime hand-out/grace/re-entry/headroom evidence remains pending.


Compiled 2026-11-01 (research worker, read-only). Every number below carries the file and line (or JSON key)
it was read from, or the command that produced it. Nothing was built or run except
`/usr/bin/python3 -I /home/intpa/ssbu-mod-playbook/tools/build/nro_size.py <nro>` on NRO files already in the
record (resident = `.text`+`.rodata`+`.data`+`.bss` from the NRO header; playbook `docs/WORKFLOW.md` §4 lines
117–119) and one `zlib` size estimate of two `layout.arc` files.

Confidence vocabulary: **measured** (a tool output or a log line in the record), **derived** (arithmetic on
measured values; the derivation is given), **stated** (a document asserts the number without evidence in the
record), **not measured**.

## Sources and path abbreviations

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

## Table A — whole-plugin image over time

All `file`/`resident` values in this table were produced on 2026-11-01 by `nro_size.py` on the listed NRO copy
unless the source column says otherwise. `resident` = text+ro+data+bss. The opt-level column is what the record
states for that build; "not stated" means no document in the record names it.

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
| revision 33 (analytics/updater/dialog removed) | 2026-10-13 19:07 (mtime of `FOOT/results.json`) | 1,769,472 | 1,798,144 | 1,036,288 / 667,648 / 65,536 / 28,672 | z + cgu1 | `2b820346`, `FOOT/results.json` key `default.segments`; `ART/switch-before-r34/libtraining_modpack.nro`. **The `default` variant is not an upstream build**: `FOOT/build-default.log:13` compiles `training_modpack v7.1.1` from `UltimateTrainingModpack-single-file`, and `FOOT/post-build-check.json` shows that tree with `src/common/dialog.rs`, `events.rs`, `release.rs` deleted and `docs/REVISION33_STARTUP_MEMORY.md` untracked, i.e. the revision 33 working tree on the revision 32 profile |
| card build before 7.1.2 | 2026-10-15 | 1,773,568 | 1,802,240 | 1,040,384 / 667,648 / 65,536 / 28,672 | z + cgu1 | `e1260d99`, `ART/switch-input-display-7.1.2/switch-before-7.1.2.nro`; `switch-before-7.1.2.json` key `plugins[libtraining_modpack.nro].bytes` |
| 7.1.2 (`v7.1.2-practice.35w`, input display renderer cache) | 2026-10-15T13:18Z | 1,785,856 | 1,814,528 | 1,040,384 / 671,744 / 73,728 / 28,672 | z + cgu1 | `96605c5b` / `b20f128e` (same header), `ART/switch-input-display-7.1.2/switch-install-receipt.json` keys `installed_bytes`, `version`, `release`; `ART/training-performance-followup-20261015/input-candidate.nro` |
| console-ui84 physical card build | 2026-10-22 (mtime) | 4,268,032 | 4,296,704 | 1,503,232 / 2,580,480 / 184,320 / 28,672 | z + cgu1 | `cac99b4c`, `ART/enhancements/console-ui84/physical-plugins/libtraining_modpack.nro` (the `.rodata` jump to 2.58 MB is the fork's own embedded roster/stage/item image data, see §C) |
| 7.1.2 experimental full candidate | not dated in the doc | 4,308,992 | not recorded | — | z + cgu1 | `4eb6ce41`, `STAGE/docs/experimental-release.md:95-96` (**stated**; NRO not located) |
| C108 | 2026-10-30T11:04Z (`installed_at` 1793358289) | 4,370,432 | 4,399,104 | 1,720,320 / 2,396,160 / 253,952 / 28,672 | z + cgu1 | `14cc0698`, `D175/switch-collection/c111-install/payload/…/libtraining_modpack.nro`; `D175/c108-card-installed175.json` keys `installed_at`, `plugins[libtraining_modpack.nro].bytes` |
| C111 | 2026-10-30T18:37Z | 4,485,120 | 4,513,792 | 1,765,376 / 2,404,352 / 315,392 / 28,672 | z + cgu1 | `719ee63d`, `D175/switch-collection/c111-speed-feedback/payload/…`; `D175/SWITCH-INSTALL-C111.json` key `at` |
| C114 | 2026-10-30T20:57Z | 4,485,120 | 4,513,792 | 1,765,376 / 2,404,352 / 315,392 / 28,672 | z + cgu1 | `060cc3f0`, `D175/switch-collection/c115-install/payload/…`; `D175/SWITCH-INSTALL-C114.json` keys `at`, `after_snapshot` |
| C115 | 2026-10-30T22:33Z | 4,489,216 | 4,517,888 | 1,769,472 / 2,404,352 / 315,392 / 28,672 | z + cgu1 | `1fed3cdd`, `D175/switch-collection/c116-install/payload/…`; `D175/SWITCH-INSTALL-C115.json` keys `at`, `bytes` |
| C116 = B122 | 2026-10-31T05:06Z | 4,534,272 | 4,562,944 | 1,806,336 / 2,412,544 / 315,392 / 28,672 | z + cgu1 | `92cef8a7`, `D175/switch-collection/c116_1-precheck/payload/…`; `D175/SWITCH-INSTALL-C116.json` keys `at`, `bytes`. The ELF measurement of the same build reports `.bss` 26,432 and "heap cost (NRO + .bss)" 4,560,704 (`ART/enhancements/automation-macros01/toml08-experiment/JSON-EXPERIMENT.md:32-33`); see "Contradictions" |
| B123-pre | 2026-10-31 | 4,440,064 | 4,466,496 ("heap") | — | z + cgu1 | `70d1be45`, `D175/C115-CAMPAIGN.md:237` (**stated**; NRO not located) |
| B123-rc (= refactor baseline WP0) | 2026-10-31 | 4,444,160 | 4,472,832 | 1,728,512 / 2,400,256 / 315,392 / 28,672 | z + cgu1 | `f0965879`, `D175/refactor/INTEGRATION.md:15` (**stated** there as a `nro_size.py` result; NRO not located) |
| M5 / M5b / M5c (release candidate) | 2026-11-01 02:5x–03:28Z | 4,759,552 | 4,829,184 | 1,851,392 / 2,498,560 / 409,600 / 69,632 | z + cgu1 | `924d4cee` / `d5e93709` / `6b63a740`, `D175/refactor/stages/m5/release/`, `…/m5b/release/`, `D175/switch-install-c1161/superseded-m5c-libtraining_modpack.nro`; `D175/refactor/RC-SUMMARY.md:13` |
| **M5i = C116.1 (installed on the Switch)** | 2026-11-01T13:31Z | **4,804,608** | **4,878,336** | 1,871,872 / 2,510,848 / 421,888 / 73,728 | z + cgu1 | `17708c2c`, `D175/refactor/stages/m5i/release/libtraining_modpack.nro`; `D175/SWITCH-INSTALL-C116.1.json` keys `bytes`, `resident`, `resident_vs_b123rc` ("about +9.1 %": 4,878,336 / 4,472,832 = 1.0907, derived) |
| M5i diag build (never shipped) | 2026-11-01 | 4,816,896 | 4,988,928 | 1,880,064 / 2,510,848 / 425,984 / 172,032 | z + cgu1 | `c3e4e52e`, `D175/refactor/stages/m5i/release/libtraining_modpack-diag.nro` |

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

**Upstream-only image: not measured.** No NRO in the record was built from upstream sources alone; the earliest
card build (`0577c959`, 2,408,448 B) is an undocumented fork candidate and `libtraining_upstream33w.nro` has the
fork's 32w header. Upstream's `Cargo.toml` release profile is `panic = "abort"`, `lto = true` with no `opt-level`
(`UP/Cargo.toml` `[profile.release]`), so an upstream build would be an opt-level-3 image like the 32w/r29/r31 rows,
which carry fork code on top. How to get the number is in §Gaps.

What the fork's profile and removals did to the image (derived from Table A rows):

| step | file delta | resident delta | text / ro / data | derivation |
| --- | ---: | ---: | --- | --- |
| r31 → r32: `opt-level "z"` + `codegen-units 1`, no source change | −1,204,224 | −1,204,224 | −1,200,128 / −12,288 / +8,192 | `381046f2` − `65ed74ab`; `REVISION32_SMALL_PROFILE.md:36` states the same −1,204,224 |
| `"s"`+cgu1 → `"z"`+cgu1 (the profile the fork kept) | −471,040 | −471,040 | −483,328 / +8,192 / +4,096 | `2bc20eb5` − `65ed74ab`; `STAGE/docs/cycle-preload.md:103` states 471,040 |
| r32 → r33: analytics thread, version check, update dialog and their deps removed (plus other r33 edits, see Table B rows 15–16) | −405,504 | −405,504 | −135,168 / −184,320 / −86,016 | `65ed74ab` − `2b820346` |
| 7.1.2 input display renderer cache (card before → installed) | +12,288 | +12,288 | 0 / +4,096 / +8,192 | `e1260d99` → `96605c5b`; release `v7.1.2-practice.35w` may carry other changes (`switch-install-receipt.json` lists only the NRO change) |

## Table B — per upstream-origin feature

Hook counts come from reading `UP/src` (attributes `#[skyline::hook]`, `#[skyline::from_offset]`,
`install_hooks!` lists). "Per frame" means the hook or function runs every game frame while the mode is active;
"per call" means a replaced game function that the game calls one or more times per frame per fighter; "on
event" means status change, hit, load, boot. Upstream gates most hooks with `is_training_mode()` at the top of
the hook body, so outside Training the per-call cost is the hook trampoline plus that check. Source byte sizes are
the upstream file sizes (`Get-ChildItem` on `UP/src`, 2026-11-01) and are a code-share proxy only, not image bytes.

| # | feature | memory (resident code/data, static tables, heap, thread stacks) | CPU (hooks and measured numbers) | build measured on | source | confidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | Mash / followup / triggers / overrides (`mash.rs` 21,917 B, `attack_angle.rs` 652 B, `full_hop.rs` 1,651 B, `fast_fall.rs` 3,289 B, `throw.rs` 4,436 B source) | Statics: `CURRENT_AERIAL`, `QUEUE: RwLock<Vec<Action>>` (heap), `FALLING_AERIAL`, `AERIAL_DELAY`, `IS_TRANSITIONING_DASH` (`UP/src/training/mash.rs:20-26`), `FULL_HOP`, `FAST_FALL`+`DELAY`, `THROW_*` (`full_hop.rs:9`, `fast_fall.rs:10-12`, `throw.rs:11-17`); bytes not measured | Replace hooks called per call per fighter: `ControlModule::get_command_flag_cat` (`training/mod.rs:97`), `get_attack_air_kind` (`:84`), `get_stick_dir` (`:213`, attack angle), `WorkModule::get_param_float/get_param_int` (`:55,69`); per frame per fighter from the vtable `on_frame` replacement: `fast_fall::do_fast_fall` (`training/mod.rs:150`). No timing measured | — | `UP/src/training/mod.rs:115-154, 894-951` | hook inventory derived; cost **not measured** |
| 2 | Defensive options: DI, SDI, airdodge direction, ledge, tech, shield (hold/tilt/decay), crouch, clatter, dodge staling, hide tech (`tech.rs` 30,711 B, `shield.rs` 12,204 B, `ledge.rs` 7,528 B, `directional_influence.rs` 3,076 B, `sdi.rs` 1,821 B, `air_dodge_direction.rs` 1,878 B, `clatter.rs` 2,648 B, `crouch.rs` 840 B, `shield_tilt.rs` 866 B source) | Statics are `RwLock` scalars/`Direction`s: `tech.rs:19-28` (incl. a `Vector3f` camera centre), `shield.rs:17-35`, `ledge.rs:11-14`, `sdi.rs:11-12`, `directional_influence.rs:12`, `air_dodge_direction.rs:11`, `clatter.rs:10-11`, `shield_tilt.rs:7`; bytes not measured; no table | On event: `L2CFighterBase_change_status` (tech, `tech.rs:36`, installed on the `common` NRO load `UP/src/lib.rs:33-37`), `correctDamageVectorCommon` (DI, on hit, `directional_influence.rs:34`), `start_clatter` (`clatter.rs:59`), `CameraModule::req_quake_pos`, `CHANGE_ACTIVE_CAMERA`, `SET_TRAINING_FIXED_CAMERA_VALUES` (hide tech, `tech.rs:402,419,735`), `MotionModule::change_motion` (`training/mod.rs:276`). Per call per fighter: `check_hit_stop_delay_command` (SDI, `sdi.rs:31`), `sub_guard_cont` (shield, `shield.rs:190`), `get_stick_x/_y`, `get_stick_x/_y_no_clamp`, `check_button_on/off` (`training/mod.rs:169-262`). Per frame per fighter: `tech::once_per_frame`, `clatter::handle_clatter`, `ledge::do_ledge_option`, `shield::handle_shield`, `directional_influence::set_di`, `shield::param_installer`, and for the CPU a `WorkModule::set_flag` write for dodge staling plus `tech::hide_tech` (`training/mod.rs:125-153`). No timing measured | — | as cited | hook inventory derived; cost **not measured** |
| 3 | Frame advantage (`combo.rs` 6,042 B, `frame_counter.rs` 3,232 B source) | `COUNTERS: RwLock<Vec<FrameCounter>>` heap (`frame_counter.rs:3`); `combo.rs:15-22` statics; bytes not measured | Per frame per fighter: `combo::once_per_frame` (`training/mod.rs:143`), `frame_counter::tick_ingame` for the CPU (`:139`), `frame_counter::tick_real` on every `info_training` draw (`ui/mod.rs:82`); per call: `WorkModule::is_enable_transition_term` (`training/mod.rs:310`). No timing measured | — | as cited | **not measured** |
| 4a | Save states — core (`save_states.rs` 29,252 B source) | `SAVE_STATE_SLOTS: Mutex<SaveStateSlots>` = 2 × 5 `SavedState` (`save_states.rs:140-152`; fields `:85-95`: 4 `f32`, 2 `i32`, a `SaveState` enum, `ChargeState` = 3 `Option<i32>` + 3 `Option<f32>` + `Option<bool>` + `Option<RobinCharges>` (`charge.rs:33-42`), `Option<SteveState>` 15 fields (`:118-134`)); about 1.6 KB heap for 10 slots (derived: ≈165 B per slot from the field list with natural alignment; not measured). Persisted as TOML: `save_states.toml` 3,648 B on the card (`D175/switch-install-c1161-final/port/save_states.toml`, file size); the fork measured the `toml 0.5` crate the persistence pulls in at **110,592 B** of NRO (json-B variant removes it: `JSON-EXPERIMENT.md:28,33`, B122 sources; `.text` −91,568, `.rodata` −16,096, `.data` −2,832 `:29-31`) | Per frame per fighter: `save_states::save_states` (`training/mod.rs:145`). Per call hooks: `get_param_int` (`:69`), `set_dead_rumble` (`:332`), `CameraModule::req_quake` (`:341`), `EffectModule::req_screen` (`:375`), `SoundModule::play_se` (`:457`, death SFX), `EffectModule::req`/`req_follow`/`req_on_joint` (`:546-690`, death GFX), `CAN_FUTTOBI_BACK` (`:743`, star KO). File write on save via the writer thread (`UP/src/common/filesystem.rs:10-21`). No timing measured | B122 for the TOML number | `UP/src/training/save_states.rs`, `UP/src/training/mod.rs:894-951`, `JSON-EXPERIMENT.md:21-33` | TOML crate size **measured** (fork, B122); slot bytes **derived**; CPU **not measured** |
| 4b | Save states — charges (`charge.rs` 41,338 B source, the largest upstream training file) | `ChargeState` inside each slot (row 4a); `OFFSET_COPY_SETUP` `from_offset` call for Kirby copy (`charge.rs:11`) | Per call: `ArticleModule::get_int` (`training/mod.rs:782`), `handle_fighter_effect`/`handle_fighter_joint_effect` (`:947-948`). Runs only on save/load | — | as cited | **not measured** |
| 4c | Save states — buffs (`buff.rs` 15,705 B source) | `BUFF_REMAINING_*`, `IS_BUFFING_*`, `BUFF_DELAY_COUNTER` (`buff.rs:13-19`) | Per call: `CLOUD_ADD_LIMIT` (`training/mod.rs:366`), `check_doyle_summon_dispatch` (`:395`), `REUSED_UI` (`:754`), `ADD_DAMAGE` (`:411`), `PLAY_SE` buff SFX (`:509`, installed as `handle_fighter_play_se` `:928`) | — | as cited | **not measured** |
| 4d | Save states — items (`character_specific/items.rs` 17,584 B source) | `TURNIP_CHOSEN: RwLock<Option<u32>>` (`items.rs:14`); 16 generated hook functions (`items.rs:496-511`) | 16 replace hooks on Peach/Daisy `daikon_N_prob` (`items.rs:478-531`), called when a turnip spawns | — | as cited | **not measured** |
| 4e | Save states — fighter-specific restore: Kirby (`kirby.rs` 21,974 B), Pokémon Trainer (`ptrainer.rs` 5,598 B), Pikmin (`pikmin.rs` 8,408 B), Steve (`steve.rs` 6,961 B source) | `CHARACTER_SPECIFIC_STATUS_MAP: LazyLock<HashMap<(i32,i32),Vec<i32>>>` heap (`character_specific/mod.rs:13`); `SWITCH_DELAY_COUNTER` (`ptrainer.rs:12`); `SteveState` in each slot | Per frame for Kirby: `KIRBY_OPFF` hook (`kirby.rs:17`); on event: `POKEMON_DECIDE` inline (`ptrainer.rs:143`), `ACTIVATE_AUTONOMY` (`pikmin.rs:34`) | — | as cited | **not measured** |
| 5 | Training reset / L+R+A (`reset.rs` 690 B source) | none beyond code | Inline hook `TRAINING_RESET_CHECK` (`training/mod.rs:433`, `lra_handle`); `reset::check_reset` runs in the vtable `on_init` replacement, i.e. on fighter init only (`training/mod.rs:156-162`) | — | as cited | **not measured** |
| 6 | Stage hazards (`hazard_manager/mod.rs` 3,785 B source) | Two `LazyLock<usize>` addresses (`hazard_manager/mod.rs:30-31`); no table | Boot: two instruction-pattern scans over the whole text segment (`TextIter` state machines, `:33-83`); run time: one inline hook at the stage-load site that writes one byte (`:85-98`) | — | as cited | **not measured** |
| 7 | Input recording and playback (`input_record.rs` 26,578 B source) | Static table `P1_FINAL_MAPPING: RwLock<[[MappedInputs; 600]; 5]>` (`input_record.rs:55-56,69-70`); `MappedInputs` = `Buttons: u32` + 4 × `i8` = 8 B (`UP/src/common/input.rs:194,294-300`) → **24,000 B in `.data`** (derived: 5 × 600 × 8; the fork's `llvm-nm` scan calls it "the pre-existing 24 KB input-recording mapping", `ART/switch-r31-smoke-1/NOTES.md:216-217`, and "the largest static object", `REVISION32_SMALL_PROFILE.md:21`) plus `P1_FRAME_LENGTH_MAPPING` 40 B (`:71-72`) and eleven `RwLock` scalars (`:57-68`) | Per frame per player: `handle_final_input_mapping` chain (`training/mod.rs:800-838`, playback at `:837`); per frame for the CPU: `input_record::handle_recording` (`:138`) and the `SET_CPU_CONTROLS` hook after the AI (`input_record.rs:462`). No timing measured | — | as cited | table bytes **derived**; CPU **not measured** |
| 8 | Input display raw/smash/status (`input_log.rs` 12,880 B, `ui/input_log.rs` 5,499 B source) | `P1_INPUT_LOGS: LazyLock<RwLock<[InputLog; 15]>>` heap (`input_log.rs:70,309-310`); each `InputLog` carries a full `Controller` copy (`input_log.rs:81-89`; `Controller` fields `UP/src/common/input.rs:240-268`), bytes not measured. Status labels: `STATUS_DISPLAY: LazyLock<HashMap<&str,i32>>` built from `UP/src/common/consts.rs` (318,173 B source, 9,354 lines; 4,617 lines match `STATUS_KIND_`, two per entry → about 2,300 name strings in `.rodata` plus the map on the heap once built); bytes not measured. Image delta of the fork's 7.1.2 renderer cache: **+12,288 B** resident (Table A derivation) | Upstream: `input_log::draw` on every `info_training` layout draw (`ui/mod.rs:81-86`) and `input_log::handle_final_input_mapping` per frame (`training/mod.rs:833`). **Measured by the fork on a host fixture (2,897 panes), original renderer → 7.1.2:** unchanged input 212 → 1 pane lookups, 2 → 0 text writes, 25 → 0 allocations; held input with frame count 210 → 3 / 2 → 1 / 24 → 1; rapid inputs 214 → 19–24 / 2 → 0–1 / 26 → 1–2; hidden by the menu 210 → 1 / 2 → 0 / 26 → 0 (`STAGE/docs/input-display-performance.md:23-28`). Host time per scenario: steady_repeat 399,521 → 421 ns, changing_buttons 489,232 → 5,669 ns, rapid_input_120_frames 27,367,134 → 19,078,139 ns, reregister 602,402 → 725,069 ns (`STAGE/docs/input-display-benchmark.md:89-103`). Follow-up (child-list walk): history rotation 20,314 → 34 pane visits, rapid input 20,382–20,469 → 102–189, 120-frame rapid workload 27.0 ms → 0.216 ms host (`STAGE/docs/input-display-followup.md:21-29`). The baseline column is the upstream renderer's work per update. Not Switch FPS (`input-display-performance.md:57`) | host bench on the 7.1.2 sources (baseline `17c4e3f9`, `input-display-benchmark.md:5`); image delta on the 2026-10-15 card builds | as cited | renderer work **measured** (host); image delta **derived**; console FPS **not measured** |
| 9 | Input delay (`input_delay.rs` 821 B source) | `P1_DELAYED_INPUT_MAPPINGS: RwLock<VecDeque<MappedInputs>>` heap = configured delay frames × 8 B (`input_delay.rs:9,15-21`) | Per frame per player inside the `OFFSET_FIM` hook (`training/mod.rs:830`): one `VecDeque` push/truncate | — | as cited | heap **derived** (per-frame formula); CPU **not measured** |
| 10 | Button chords / button config (`button_config.rs` 10,348 B source) | `BUTTON_COMBO_REQUESTS: LazyLock<RwLock<HashMap<ButtonCombo,bool>>>` heap, `START_HOLD_FRAMES`, `START_RELEASE_FRAMES` (`button_config.rs:187-197`) | Per frame per player inside the FIM hook (`training/mod.rs:826`) | — | as cited | **not measured** |
| 11 | Menu system: model (`training_mod_consts`, `training_mod_tui`), native rendering (`ui/menu.rs` 24,201 B, `ui/mod.rs` 7,046 B source), persistence | `BASE_MENU`, `DEFAULTS_MENU`, `MENU` = three `TrainingModpackMenu` copies in `.data` (`UP/training_mod_consts/src/lib.rs:114,208-209`; bytes not measured); `QUICK_MENU_APP: LazyLock<RwLock<training_mod_tui::App>>` heap (`UP/src/common/menu.rs:92`), `GCC_/PROCON_BUTTON_MAPPING` HashMaps (`ui/menu.rs:66,75`), `DIRECTION_HOLD_FRAMES` (`common/menu.rs:100`); persisted menu file `training_modpack_menu.conf` **22,589 B** on the card (`ART/switch-before21/manifest.json` key `bytes`; JSON per `JSON-EXPERIMENT.md:142`). Data loader thread 0x20000 = 128 KiB stack, joined at boot (`UP/src/lib.rs:102-108`) | `menu::menu_condition` per frame per fighter (`training/mod.rs:127`), `menu::handle_final_input_mapping` per frame per player (`:822`), `menu::draw` on every `info_training` draw (`ui/mod.rs:85`); the menu build/draw work itself not measured. Diag-only observation in the fork: the heartbeat's `ui_pass`/`tick_pass` counters rise by about 120/s in Training (`REVISION32_SMALL_PROFILE.md:45-46`, **stated**); one r33 Eden log shows `ui_pass=111` → `506` and `tick_pass=113` → `506` between two heartbeats 5 s apart while loading (`ART/switch-r33-eden-train1/practice-diagnostics.log:22,31`, measured) | r33 on Eden for the heartbeat sample | as cited | **not measured** except the menu file size and the fork-side pass counters |
| 12 | Layout archive injection and HUD toggle (`ui/mod.rs:57-187`) | Embedded `src/static/layout.arc`: upstream file **4,213,268 B** raw (`Get-Item UP/src/static/layout.arc`), stored DEFLATE-compressed by `include_flate::flate!` (`ui/mod.rs:174`); compressed size estimate 175,527 B (zlib level 9) / 193,889 B (level 6) (`python3 -I` `zlib.compress` on the file, 2026-11-01; the fork states "about 195 KB", `REVISION33_STARTUP_MEMORY.md:39`). **Inflated copy 4,213,268 B on the game heap**: upstream `include_flate` inflates once on first use and keeps it for the process lifetime (`REVISION33_STARTUP_MEMORY.md:23`, confirmed by the log line `[LayoutArchive] handed out info_training bytes=4213268`, `ART/switch-r33-eden-train1/practice-diagnostics.log:77`; r33 then releases it: `released … after 6000ms`, `:106`). A second waiting-room copy (~4.2 MB) existed in the fork from 32w (`REVISION33_STARTUP_MEMORY.md:24`, stated). The 5 MiB static `LAYOUT_ARC` buffer exists only under the `layout_arc_from_file` feature (`ui/mod.rs:94-97`), not in release | Upstream `handle_draw` runs on every layout draw of every layout: `from_c_str` of the layout name + a 5-string `contains` for the HUD toggle + `damage::draw` on each call, then the four `info_training` passes (`ui/mod.rs:57-89`); the fork's replacement observes "once per holder per frame" on the UI worker (`REVISION29_DRAW_DECOUPLING.md:109`). `handle_layout_arc_malloc` inline hook runs once per layout archive load; upstream parses the whole archive with `SarcFile::read`, which "cloned every member's texture/animation payload" (fork comment, `STAGE/src/training/ui/mod.rs:307-309`); inflating the 4.2 MB archive "costs a few tens of milliseconds on a loading screen" (`REVISION33_STARTUP_MEMORY.md:38-39`, stated). Fork heartbeat `layout_arc=7` then `9` calls by 15–20 s of uptime (`switch-r33-eden-train1/practice-diagnostics.log:22,31`) | r33 on Eden (2026-10-13) for the archive lines | as cited | archive bytes **measured**; compressed size **derived** (zlib estimate); draw cost **not measured** |
| 13 | Hitbox visualizer (`hitbox_visualizer/mod.rs` 10,324 B, `raygun_printer.rs` 6,103 B source) | Static tables `SEGMENT_DICT: [[f32;5];15]` = 300 B, `ALPHABET: [&str;40]`, `SEGMENT_REV: [char;15]` = 60 B (`UP/src/common/raygun_printer.rs:18,52,96`); no heap table | Per frame per fighter: `hitbox_visualizer::once_per_frame` (`training/mod.rs:144`); per animcmd call: `sv_animcmd::ATTACK`, `CATCH` replace hooks (`hitbox_visualizer/mod.rs:196,269`), `GrabModule::set_rebound` (`:322`). No timing measured | — | as cited | table bytes **derived**; CPU **not measured** |
| 14 | Boot, offset search, logging (`offsets.rs` 10,494 B, `logging.rs` 1,531 B, `filesystem.rs` 1,808 B source) | 23 static byte needles (`UP/src/common/offsets.rs:60-343`) plus two fixed vtable addresses (`:356,365`); `log` facade + `TrainingModpackLogger` static (`logging.rs:46`); writer thread spawned with `std::thread::spawn`, i.e. the Switch std default **2 MiB stack for the process lifetime** (`UP/src/common/filesystem.rs:10-21`; the 2 MiB default is `REVISION33_STARTUP_MEMORY.md:12-14,22`, stated from `library/std/src/sys/thread/switch.rs`); fork reduced it to 0x10000 = 64 KiB (`STAGE/src/common/filesystem.rs:17`) | Boot: 23 `memmem` needle searches over the text segment (each "first and last (about 57 MiB)" per the fork's description of the same search, `REVISION29_DRAW_DECOUPLING.md:230-231`, stated), two hazard instruction scans (row 6), three `LookupSymbol` calls (`training/mod.rs:844-870`), one `.nop()` text patch and two vtable word patches (`:878-892`), 66 hooks installed (34 in the `training_mods` list, 29 from the module `init` calls `:953-959`, 3 on the `common` NRO load `lib.rs:33-37`; itemised below Table B), menu load on a joined thread. Only the needles whose offsets are used are searched (the `OPCF` hook is commented out, `training/mod.rs:793`). Run time: logging goes to Skyline's `println` only; no per-frame file writes | — | as cited | inventory derived; time **not measured** |
| 15 | Removed: analytics thread (`common/events.rs` 5,470 B source) | Permanent thread via `std::thread::spawn(events_loop)` (`UP/src/lib.rs:154`) → **2 MiB stack** (`REVISION33_STARTUP_MEMORY.md:20`, stated); `EVENT_QUEUE: RwLock<Vec<Event>>` with 7 `String`s per event (`events.rs:11,95-104`), `SESSION_ID`/`DEVICE_ID`/`USER_ID` strings (`:12,44,65`); dependencies `minreq` (`https-native`), `native-tls` vendored (`UP/Cargo.toml` `[dependencies]`, `[patch.crates-io]`). Image cost together with row 16 and the r33 edits: **−405,504 B** resident when removed (Table A derivation, `65ed74ab` → `2b820346`: text −135,168, ro −184,320, data −86,016). Upper bound for rows 15+16: the r33 tree also touched `filesystem.rs`, `menu.rs`, `hang_trace.rs`, `named_macros.rs`, `ui/mod.rs`, `training_mod_consts` (`FOOT/post-build-check.json` key `worktree_status_porcelain`) | Loop: `sleep(10 s)` then `minreq::post` of the queued events (`events.rs:177,187`); network and TLS work every 10 s for the session | r32 → r33 NROs, 2026-10-13 | as cited | image delta **derived** (upper bound); stack **stated**; CPU **not measured** |
| 16 | Removed: auto-update / version check thread and dialog (`common/release.rs` 7,888 B, `common/dialog.rs` 1,735 B source) | Thread with explicit `stack_size(0x80000)` = **512 KiB**, joined before `main` returns, so transient (`UP/src/lib.rs:110-121`; skipped on emulators `:110`); downloads the GitHub release zip into a `Vec` and extracts it (`release.rs:30-47`); dependencies `zip`, `byte-unit`, `skyline-web` (dialog) (`UP/Cargo.toml`); `CURRENT_VERSION` string (`release.rs:11`). Image cost: inside the −405,504 B of row 15 (`REVISION33_STARTUP_MEMORY.md:21,25` lists exactly these removals) | One HTTPS GET at every console boot (`release.rs:88`, `:176`), plus the dialog and zip install when accepted (`:46-52,154,209`) | r32 → r33 NROs | as cited | stack **measured in source** (constant); image **derived** (shared with row 15); boot time **not measured** |
| 17 | Whole-NRO image | See Table A. Upstream-origin code share of the M5i image is **not measured**; the fork's own additions dominate the growth from 1,798,144 B (r33) to 4,878,336 B (M5i), of which `.rodata` alone grew from 667,648 to 2,510,848 B (Table A rows; the console-ui84 build shows the `.rodata` step to 2,580,480 B at 2026-10-22) | — | all | Table A | **measured** (whole image), per-feature split **not measured** |

Upstream per-frame hook summary (CPU proxy, derived from `UP/src`): one vtable replacement runs
`once_per_frame_per_fighter` for every fighter every frame (`training/mod.rs:115-154`: 11 feature calls per
fighter per frame plus 4 CPU-only calls); one `OFFSET_FIM` hook runs per player per frame with 6 feature calls
(`training/mod.rs:800-838`); 1 `SET_CPU_CONTROLS` hook per CPU per frame (`input_record.rs:462`); 1 Kirby per-frame
hook (`kirby.rs:17`); 1 `handle_draw` per layout draw (`ui/mod.rs:57`). Installed hooks in total: 34 in the
`training_mods` list (`training/mod.rs:894-951`: the FIM hook, 3 inline hooks `TRAINING_RESET_CHECK`/`STALE`/
`STALE_MENU`, and 30 replacements of `ControlModule`/`WorkModule`/`EffectModule`/`SoundModule`/`CameraModule`/
`MotionModule`/`ArticleModule`/fighter functions that the game calls per fighter per frame or per event), 16
item-probability hooks (`items.rs:514-531`), 1 `set_cpu_controls` (`input_record.rs:605`), 2 UI hooks
(`ui/mod.rs:201`), 1 Pikmin, 1 Pokémon Trainer, 1 Kirby (`pikmin.rs:51`, `ptrainer.rs:158`, `kirby.rs:607`), 3 tech
camera hooks (`tech.rs:753-757`), 3 hitbox-visualizer hooks (`hitbox_visualizer/mod.rs:353`), 1 hazard inline hook
(`hazard_manager/mod.rs:85`), 3 hooks on the `common` NRO load (`lib.rs:33-37`) = 66 hooks, plus 2 vtable word
patches and 1 nop patch (`training/mod.rs:878-892`). Upstream builds with `debug_assertions` add 6 `WorkModule`
debug hooks (`training/mod.rs:962-963`, `debug.rs:17-99`), not in release.

## Table C — context from the fork's own features (frames the whole-plugin budget)

| item | value | source | confidence |
| --- | --- | --- | --- |
| Process memory budget the kernel enforces (svcGetInfo id 6) | 3,444,572,160 B = 3285 MiB in every sample of the s19/s21 runs; `headroom` = id 6 − id 7 | `STAGE/training_mod_consts/src/body_bank/memory.rs:9-10`; `D175/SWITCH-INSTALL-C116.1.json` key `not_verified[0]` ("same 3285 MiB budget", Eden-measured, console not verified) | measured (Eden) |
| Admission rule | load admitted only if headroom − outstanding ≥ cost × (1 + 1/8) with a 16 MiB floor | `memory.rs:45,48` (`MARGIN_DIVISOR = 8`, `FLOOR = 16 * MIB`); `D175/C115-CAMPAIGN.md:469` ("margin 1/8 + 16 MiB floor verified against 663 prefetch / 590 build measurements") | measured |
| Per-fighter costs (process charge, Eden) | 176 kinds in `KIND_COSTS`, e.g. Mario `0x00` prefetch 44,433,408 B / build 13,631,488 B, Donkey Kong `0x01` 43,646,976 / 2,293,760, `0x06` 107,741,184 / 19,660,800; range 3–147 MiB prefetch, 0–98 MiB build; Game & Watch builds 80–98 MiB alone; Simon/Richter ≈ 100 MiB; Falcon helper load 153.8 MiB once | `STAGE/training_mod_consts/src/body_bank/memory_table.rs:261-272` (rows; 176 rows counted with `rg -c "^    \(0x"`), `memory.rs:13-14,18-19`; `SWITCH-INSTALL-C116.1.json` key `not_verified[1]` | measured (Eden); console not measured |
| How the table was built | `tools/group_memory_costs.py` reads `[GroupSwap] prefetch … headroom=`, `[PreloadMetrics] v1 record … committed=A->B … headroom=`, `[GroupSwap]/[CharacterSources] memory … phase=… delta=` lines from 298 logs; estimates per kind = largest single observation, bounds from group totals, 10th-percentile lower bound | `STAGE/tools/group_memory_costs.py:1-36,132-170`; `memory_table.rs:1-3` | measured |
| Preload metrics record format | `[PreloadMetrics] v1 record scene=… op=request target=fighter kind=0x4a hit=0 outcome=ready frames=0 us=1215334 … committed=3021647872->2762059776 delta=-259588096 base_delta=+268435456 headroom=682512384 slab=348160->368640/16777216 slots=5 apply=fighter+hat resident=4 …`; `committed` is svcGetInfo id 7 (committed process pages, "NOT allocator live bytes"), time is the Horizon tick | `D175/refactor/eden-sweep/yoga/runs/hlp4/final/ultimate/TrainingModpack/practice-diagnostics.log:6`; `STAGE/src/preload_metrics.rs:1-12` | measured |
| Eden Training comparison of the preload (fork feature) | targets Off/1/2/3/4/5: fighter median 805/520/501/486/477/468 ms, hat median 770/82/82/81/82/81 ms, extra peak guest memory +0/+55/+104/+137/+165/+214 MiB | `STAGE/docs/cycle-preload.md:66-75` | measured (Eden) |
| Physical Switch headroom samples (C111) | minimum headroom 646,795,264 B = 616.83 MiB (sample #182), baseline 852,774,912 B = 813.27 MiB (#64), budget 3,444,572,160 B | `D175/C111-SWITCH-SPEED.md:23` | measured (console) |
| Hot-swap timings (fork feature) | C111 console: window build 5.65 s / 6.31 s / 4.35 s, in-window reuse 0.097 s and 0.378 s; C116 Eden: warm group swaps ~50 ms, first build into a group ~0.4–1.0 s, IC→X 67–105 ms, X→IC 310–327 ms, first pair build ~2.1 s; C108 cold reload baseline mean 684.814 ms (9 reloads, Eden) | `D175/C111-SWITCH-SPEED.md:9-17`; `D175/SWITCH-INSTALL-C116.json` key `contents[2..3]`; `D175/COLD-BASELINE.md:5` | measured |
| Fork heap statement for the RC | +356,352 B resident B123-rc → M5 (+7.97 %), largest item the 608/626-row seam table (+221,184 at M1: ro +106,496, data +118,784, text +40,960), diag ring +40,960 bss, WP-H meter +4,096 text (shipped build unmetered) | `D175/refactor/RC-SUMMARY.md:13-15`; `INTEGRATION.md:1680-1683,447,1388`; `C115-CAMPAIGN.md:355` | measured |
| Diag build overhead | shipped 4,521,984 → `full,diag` 4,624,384 (+102,400) → `+plugin-heap-meter` 4,628,480 (+4,096 text) resident at WP-H; M5i release 4,878,336 vs diag 4,988,928 (+110,592; bss 73,728 → 172,032) | `INTEGRATION.md:1387-1389`; Table A rows | measured |
| Fork layout archives | `STAGE/src/static/layout.arc` 7,820,940 B raw (zlib-9 1,677,918 B); inflated hand-outs seen in logs: `info_training bytes=7635780`, `info_waiting bytes=6667356`, `6737012`, `8509244`, each released 5–9 s later | `Get-Item`/`zlib` 2026-11-01; `ART/enhancements/plan2-mii-runtime-01/practice-diagnostics.log:152,164`; `ART/training-performance-followup-20261015/grid-waiting/practice-diagnostics.log:92,109`; `ART/enhancements/plans14-ldn05-audit01/evidence/window-a-accepted-practice-diagnostics.log:192,217` | measured (Eden) |
| Fork thread stacks | heartbeat/hang-trace thread 0x10000 = 64 KiB, config writer 0x10000 = 64 KiB, data loader 0x20000 = 128 KiB; diagnostics ring 32 KiB (release) / 128 KiB (diag), writer every 50 ms, files capped at 128 KiB | `STAGE/src/hang_trace.rs:151`, `STAGE/src/common/filesystem.rs:17`, `STAGE/src/lib.rs:352`, `STAGE/src/logging.rs:38,42`; `cycle-preload.md:88` | measured in source |
| Eden host-level figures | ~60 FPS idle windows, idle CPU ~0.945/0.951 cores, host peak 5.5–6.7 GiB across game loads (whole emulator; not plugin cost) | `STAGE/docs/experimental-release.md:69-73,79-81` | measured (host), not plugin-attributable |

## Gaps and how to measure

Every "not measured" row above shares one cause: the record never built the plugin with a single upstream-origin
feature on or off, and the console has no per-feature counter. The fork's tooling already contains every
instrument needed; phase 3 can set a before/after budget per feature with the following methods.

1. **Per-feature image bytes (rows 1–14, 17).** Give the clean-room crate one cargo feature per upstream-origin
   feature (the playbook already requires a `diag` feature and a `model`/`runtime` split, `PLAYBOOK/docs/WORKFLOW.md`
   §1–3). Build the matrix "all on", "all on minus X" for each X with `cargo skyline build --release` (WORKFLOW §4)
   and run `python3 -I PLAYBOOK/tools/build/nro_size.py` on each NRO; the difference per segment is the feature's
   resident cost, page-rounded (4 KiB pages, as the stage trail shows). Record the sha256 the tool prints. Do the
   same once on upstream `97cbe9b` itself to obtain the missing upstream-only image row of Table A (an opt-level-3
   build with `lto = true`, `UP/Cargo.toml`), and once on the same sources with `opt-level = "z"`,
   `codegen-units = 1` so the comparison with the fork's 1,798,144 B r33 image is like for like. The
   `FOOT/results.json` layout (overrides, returncode, nro_sha256, segments) is a usable schema for the matrix.
2. **Static tables and `.data`/`.bss` attribution.** `llvm-nm --print-size --demangle` on the release `.so` and bucket
   the symbols per module, as the fork did for the `.smd` path (`ART/switch-r31-smoke-1/NOTES.md:205-222`). This
   gives the exact bytes of `P1_FINAL_MAPPING`, the three menu copies, the status-name strings and the raygun tables
   without a per-feature build.
3. **Heap per feature (rows 4a, 8, 9, 10, 11, 12).** Ship the counting allocator the playbook asks for and the fork
   already has (`STAGE/src/heap_metrics.rs:17-62`: live, peak, total, calls, largest; `PLAYBOOK/docs/WORKFLOW.md`
   §3 lines 86-92). Extend the install line and a diag snapshot line with `heap_live=`/`heap_peak=` bytes, not only
   `heap_allocations=<calls>` (the fork's shipped field is `unmetered`, `STAGE/src/swap_native/heap_fields.rs:1-21`;
   no numeric `heap_allocations=` line for the Training Modpack exists in the logs searched). Take one snapshot at
   boot, one after the first Training load, one after each feature is toggled on in the menu; the deltas are the
   feature's Rust heap. For the layout archive specifically, log the inflated size and lifetime as the fork does
   (`[LayoutArchive] handed out/released … bytes=… after …ms`).
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
6. **CPU per hook (rows 1–13).** Add diag counters per hook: calls per heartbeat interval and accumulated
   `cntpct_el0` ticks inside the hook body (`preload_metrics.rs:24-37` shows the tick read), printed by a heartbeat
   thread like the fork's `[Heartbeat] … ui_pass=<calls>/<ms since last>` line (`REVISION31_HANG_TRACE.md:23-24`;
   sample `ART/switch-r33-eden-train1/practice-diagnostics.log:22,31`). Report per feature: hooks × calls per frame
   × mean ticks. Pair it with Eden's frame-time counter and, on the console, with the time between two consecutive
   per-frame entries, so a feature's cost is expressed in microseconds per frame at 60 Hz (16,667 µs budget).
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

## Contradictions and cautions found between sources

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
- **REVISION33's compressed-archive figure** ("about 195 KB", `:39`) sits between the zlib-6 (193,889 B) and zlib-9
  (175,527 B) estimates of the upstream archive; `include_flate`'s actual level is not recorded.
- **Eden versus console.** Every memory cost in Table C is Eden process charge; `SWITCH-INSTALL-C116.1.json` key
  `not_verified[0]` records that the console's timing and heap were not measured and the Eden costs were assumed
  equal. The only console memory numbers in the record are the C111 headroom samples (`C111-SWITCH-SPEED.md:23`).
