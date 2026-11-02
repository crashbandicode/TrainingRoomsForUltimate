# Test provenance, package 1

`Requirement | spec § | host test | Eden row planned | status`. Host tests run with `cargo test` (69 at the
M3 commit). "Eden row" labels are the specs' own; no Eden run has happened (package 1 is not
Switch-compiled yet). Status: **host pass** = test exists and passes; **deferred** = the spec row belongs to a
later package or needs the Switch runtime; **runtime, unbuilt** = code written, not compiled for the Switch.

## SPEC-core-01 (boot, install, sites)

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
| No heap in hooks (counting allocator, `heap_allocations=`) | §8 | `delay::ring_allocates_nothing` (queue); `src/runtime/alloc.rs` | - | host pass (queue); runtime unbuilt |
| `developer.ini` (decision N1) | N1 | `core01_developer_ini` | - | host pass |

## SPEC-core-04 (settings.json)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Wire shape, real fork files | §3.1, §9 | `core04_wire_shape_fixture` (two real files) | core04-E1 | host pass |
| Factory round trip, defaults independent | §9 | `core04_roundtrip_factory`, `core04_defaults_independent` | core04-E2 | host pass |
| Unknown keys, retired keys | §3.2 | `core04_unknown_keys_ignored`, `core04_unknown_keys_kept_on_rewrite` | - | host pass (see note 1) |
| Missing keys take defaults; short/long arrays | §3.2 | `core04_missing_keys_default` | core04-E3 | host pass |
| Corrupt file: factory + field by field + copy name (decision S2) | §6 | `core04_corrupt_file_policy`, `core04_non_utf8_file_is_corrupt`, `core04_slider_rows`, `core01_paths` (copy names) | core04-E4 | host pass; the rename itself is runtime, unbuilt |
| Duplicate key | §3.2 | `core04_duplicate_key_error` | - | host pass |
| Fork file imports unchanged | §6 | `core04_import_upstream_file` | core04-E5 | host pass |
| Publish before write; writer fallback; no write on unplug | §5, §9 | `core04_publish_order`, `core04_writer_fallback`, `core04_no_write_on_unplug` | core04-E6..E8 | host pass (model); writer thread not in package 1 (nothing writes yet) |

## SPEC-input-03 (Input Delay)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Menu row, values, default 0 | §2.1 | `delay::menu_default_zero` | IN03-E01 | host pass (the row itself arrives with the menu) |
| Steady state, initial fill | §5 | `delay::steady_state_offsets`, `delay::initial_fill_neutral` | IN03-E02 | host pass |
| Change up / down | §5 | `delay::change_up_refills`, `delay::change_down_drops_oldest`, `delay::delay_zero_empties_the_queue_and_passes_through` | IN03-E03 | host pass |
| Clear on owner change; save-state load; reset (B8) | §5.1 | `delay::clear_on_input_owner_change` | IN03-E04 | host pass; owners and the reset hub are not wired in the runtime (see progress file) |
| Chords not delayed; display and recorder see delayed input | §5.2 | `delay::ordering_with_chords_display_recorder` | IN03-E05 | host pass |
| Zero allocations | §8 | `delay::ring_allocates_nothing` | - | host pass |
| Hook at `main+0x1750dd0` | §4 | `core01_sites_match_owned_elf` | IN03-E02 | runtime unbuilt |

## SPEC-state-06 (L+R+A Reset)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Register rule | §4 | `state06_lra_register_rule` | E-state-06-lra-off-no-reset | host pass; runtime unbuilt |
| Hub trigger rules | §5.1 | `state06_hub_trigger_rules` | E-state-06-reset-clears-mash | host pass; **hub not wired** |
| Hub order and contents | §5.2 | `state06_hub_order` | - | host pass |
| Counter types | §5.2 | `state06_counter_types` | - | host pass |
| Load calls the hub every PosMove frame | §5.3 | - | E-state-06-load-resets-delays | deferred (package 3) |
| Hub idempotent | §6 | `state06_hub_idempotent` | - | host pass |

## SPEC-state-07 (Stage Hazards)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Flag address arithmetic | §5.1 | `state07_flag_address_arithmetic` | - | host pass |
| Scans find the pinned values on the owned `.text` | §5.1 | `state07_scan_matches_pinned` | - | host pass |
| Write rule | §5.3 | `state07_write_rule` | E-state-07-hazards-off-ps2 | host pass; runtime unbuilt |
| Missing address disables | §9 | `state07_missing_address_disables` | - | host pass |
| Change applies at next stage load | §5.4 | - | E-state-07-toggle-then-reload | Eden only |
| Waiting-room per-request policy | §6.3 | - | E-state-07-lobby-no-global-write | deferred (package 6; `policy.rs` not carried yet) |

Note 1: the spec says unknown keys are dropped on rewrite. This build knows 3 of the ~90 rows, so dropping
them would erase the player's other settings until their packages land; they are kept verbatim instead
(`core04_unknown_keys_kept_on_rewrite`). Retired keys are dropped as the spec says. Lead decision requested.
