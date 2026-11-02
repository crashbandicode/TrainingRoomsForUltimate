# Test provenance, package 1

`Requirement | spec § | host test | Eden row planned | status`. Host tests run with `cargo test` (unit tests inside the model modules are also compiled into each by-path test binary, so the per-binary counts overlap; `cargo test` in total runs 7 + 8 + 6 + 17 lib unit tests plus the spec-named tests in `tests/`). "Eden row" labels are the specs' own; no Eden run has happened (package 1 is not
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
| No heap in hooks (counting allocator, `heap_allocations=`) | §8 | `delay::ring_allocates_nothing` (queue); detector proven by `hook_alloc::the_detector_reports_a_stub_that_allocates_and_passes_one_that_does_not` (R5, below); `src/model/hook_stats.rs` | - | host pass; Eden: diag `hook_stats` lines, see the R1-fix section |
| `developer.ini` (decision N1), fail-closed (R4) | N1 | `core01_developer_ini`, `core01_developer_ini_fails_closed`, `core01_developer_ini_load_from_files`, `install_line::the_developer_gate_outcome_is_on_the_line` | - | host pass; Eden: `developer=off reason=…` on the install line, see the R1-fix section |

## SPEC-core-04 (settings.json)

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

## SPEC-input-03 (Input Delay)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Menu row, values, default 0 | §2.1 | `delay::menu_default_zero` | IN03-E01 | host pass (the row itself arrives with the menu) |
| Steady state, initial fill | §5 | `delay::steady_state_offsets`, `delay::initial_fill_neutral` | IN03-E02 | host pass |
| Change up / down | §5 | `delay::change_up_refills`, `delay::change_down_drops_oldest`, `delay::delay_zero_empties_the_queue_and_passes_through` (R3: the transition poll returns the current record), `delay::delay_zero_transition_with_press_and_release_records`, `delay::change_down_matches_the_observable_rule` | IN03-E03 | host pass |
| Clear on owner change; save-state load; reset (B8) | §5.1 | `delay::clear_on_input_owner_change` | IN03-E04 | host pass; the reset hub clears the queue (Eden `hub_run`); the menu, chord, replay and waiting-room owners arrive with their packages |
| The game's pause is an input owner (R2): begin sets the bit and empties the queue even if polls stop, polls while paused are neutral, end releases, first *d* polls after resume neutral; driven through the real producer | §4, §5.1, question 3 | `input_pause::a_pause_empties_the_queue_owns_the_input_and_nothing_old_reaches_gameplay`, `input_pause::takeover_empties_the_queue_even_when_polls_stop_while_paused`, `input_pause::the_unwired_producer_lets_the_stale_and_menu_records_through`, `input_pause::other_owners_still_work_beside_the_pause_and_survive_its_release`, `input_pause::delay_zero_and_repeated_transitions_are_harmless` | IN03-E06 (diag pause lane) | host pass (producer and poll); **Eden: the Training pause menu does not run the hooked `StatePause`, so the producer is not driven in Training yet** (R1-fix section) |
| The pause sites: the `StatePause` chain in the owned ELF, guards, no pilot overlap, card scan | §4 | `pause_sites::statepause_chain_from_the_state_name_to_the_two_hooked_instructions`, `pause_sites::the_rows_are_the_two_pause_sites_and_clear_of_everything_else` | IN03-E06 | host pass |
| Chords not delayed; display and recorder see delayed input | §5.2 | `delay::ordering_with_chords_display_recorder` | IN03-E05 | host pass |
| Zero allocations | §8 | `delay::ring_allocates_nothing` | - | host pass |
| Hooks in the two callers of the final input mapping (note before each call, apply right after it returns; no hook in or on the function, whose inner points are the CPU pilot's) | §4 | `input_mapping_callers::the_sites_are_in_the_callers_and_not_near_the_pilot`, `input_mapping_callers::the_function_has_two_callers_and_the_registers_are_still_ours_at_the_sites`, `core01_sites_match_owned_elf` | IN03-E02 (run with and without the pilot NRO, both load orders) | host pass; Switch-built |

## SPEC-state-06 (L+R+A Reset)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Register rule | §4 | `state06_lra_register_rule` | E-state-06-lra-off-no-reset | host pass; runtime unbuilt |
| Hub trigger rules and the facts they rest on (entry id at fighter+0x10, manager slot, the two thunks) | §5.1 | `state06_hub_trigger_rules`, `state06_hub_trigger_facts_match_the_guarded_words` | E-state-06-reset-clears-mash | host pass; runtime unbuilt |
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

## SPEC-core-08 (first-boot import, pure plan) and draft 9 settings

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

Note 3 (replaced by review R4, 2026-11-02): `developer.ini` compares the key and the value exactly (`developer_page`, lower-case `on`; lead, 2026-11-02), skips a leading UTF-8 BOM, and now **fails closed**: the loader reads at most 8 KiB + 1 byte and an over-limit file hides the page, a read error hides it, bytes that are not UTF-8 anywhere hide it, and a repeated `developer_page` key (in any order, any values) hides it. The earlier last-line-wins and prefix-truncation behaviour is gone. The install line says why: `developer=off reason=missing|unreadable|too_large|not_utf8|duplicate_key|key_absent|not_on`, or `developer=on reason=on`.

Note 1: the spec says unknown keys are dropped on rewrite. This build knows 3 of the ~90 rows, so dropping
them would erase the player's other settings until their packages land; they are kept verbatim instead
(`core04_unknown_keys_kept_on_rewrite`). Retired keys are dropped as the spec says. Lead decision requested.

## Menu model (SPEC-core-02, SPEC-core-03 §2.5, SPEC-core-04 `last_page`, SPEC-input-04 N7), `tests/core02_menu.rs`

`src/model/menu/` (pure; no rendering, no hooks) and the factory table `src/model/factory.rs`. Rows that the
new page layout replaced (the 8 x 4 submenu grid, tabs, R-confirm reset) keep their spec test name where the
requirement survives in the new structure, adapted as noted. Names with the `chords::` prefix in SPEC-input-04
§10 are written `chords_...` here.

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
| Greyed rows with the reason: Respawn invincibility while Auto-load on death is On (N2); swap-group dependencies | N2, MENU-DRAFT §2 | `respawn_invincibility_is_greyed_while_auto_load_is_on`, `swap_group_entries_grey_with_the_reason` | core02-E2 | host pass (dependencies inferred, see progress file) |
| Direction grids: chip layout and navigation, weights, ZL all / none, nine-dot preview; Shield Tilt exactly one chip | Q12, MENU-DRAFT §1 | `direction_grid_navigation_follows_the_chip_layout`, `direction_grid_weights_all_none_and_preview`, `shield_tilt_grid_holds_exactly_one_chip` | core03-E7 | host pass |
| N3 delay rows: checklist or range exclusion, Y clears the active one, bounds and steps, round trip, file without a range, slot sets have no range, reset / save-defaults and bulk update treat them as one row | N3, core02 §2.8, §9 | `core02_delay_row_exclusion`, `core02_delay_row_y_clear`, `core02_delay_range_bounds`, `core02_delay_range_roundtrip`, `core02_delay_no_range_loads_checklist`, `core02_slot_sets_have_no_range`, `core02_delay_row_reset_and_save_defaults`, `core02_delay_row_bulk_update_clears_range`, `delay_range_pick_is_uniform_over_the_step_values`, `core03_delay_popup_render_plan` | core02-E8, E9, core03-E14 | host pass |
| X resets the page to the defaults snapshot (saved defaults replace the factory ones); serialization shape and round trip; bulk update clears before applying | core02 §2.5, §5, §9 | `core02_defaults_and_reset`, `core02_serialize_roundtrip`, `core02_update_from_shorter_array` | core02-E4 | host pass (see gap: no key saves defaults) |
| Hold repeat every 21 frames on sticks | core02 §2.3, §9 | `core02_hold_repeat_period` | core02-E6 | host pass |
| N1 Developer page in the ring only while the flag is on; rows serialized, inert while hidden | N1, core02 §2.8 | `core02_hidden_page_ring`, `core02_hidden_page_rows_serialized` | core02-E7 | host pass |
| Q5 opens on the last page used, CPU on the first open ever, a saved page not in the ring opens CPU; `last_page` round trip | Q5, core02 §2.8, core04 §3.1 | `core02_first_open_is_cpu`, `core02_open_on_last_page`, `core02_last_page_missing_falls_back`, `core04_last_page_roundtrip` | core02-E10 | host pass |
| Re-roll events (Q18 A: SDI Direction, Shield Tilt; B11: Ledge Options, Throw options only; delay rows and DI do not) | Q18, B11 | `menu_change_to_cpu_choices_emits_reroll_events` | CPU package rows | host pass (model only; the CPU package consumes `Outbox::rerolls`) |
| N7 shortcut rows open the chord editor and show the chord text; permanent chord is a read-only info row; a confirmed change applies at once and emits the `button_config.ini` request, only then | N7, input04 §10 | `core03_shortcut_rows_open_editor`, `core03_permanent_shortcut_row_info`, `chords_shortcut_rows_open_editor`, `chords_permanent_chord_fixed`, `chords_shortcut_change_applies_immediately`, `chords_ini_written_only_on_confirmed_change`, `button_chords_editor_writes_the_chord_setting`, `chord_text_and_row_conversion` | core03-E13, IN04-E08 | host pass (the INI text and the write are package 4 / the runtime) |
| Replay capture toggle emits its INI request | Q7 | `replay_capture_toggle_writes_its_ini` | - | host pass |
| N5 paging by console mode: items docked 5 x 8 one page, handheld 4 x 4 pages 16 / 16 / 6; stages docked 3 pages 40 / 40 / 38, handheld 8 pages; every tile once; layout fixed at open | N5, core03 §2.5, §9 | `core03_item_grid_layout_docked`, `core03_item_grid_layout_handheld`, `core03_item_grid_layout_fixed_at_open`, `core03_item_grid_pages_cover_38_items_once`, `core03_stage_grid_layout_docked`, `core03_stage_grid_layout_handheld`, `core03_stage_grid_pages_docked_40_40_38`, `core03_stage_grid_pages_handheld_8`, `core03_stage_grid_pages_cover_118_tiles_once`, `core03_stage_grid_rotation_and_form_same_in_both_layouts`, `grid_screen_pages_with_l_r_and_keeps_the_cursor_on_a_tile` | core03-E12, E15 | host pass (rotation badges and the form tag are drawn per tile: package 5) |
| Dropdown lists read back and have unique labels | MENU-DRAFT §2 | `dropdown_options_are_unique_and_row_values_read_back` | - | host pass |

Not covered by the model (later packages): the fork's grid contents (portraits, hats, Mii lists, stage and
item actions), the named-macro and setup libraries, rendering of every screen, the INI text writer.
## Review R1 fixes (2026-11-02, stream `r1fix`, base 4895ea3; review `artifacts/trainingrooms-reviews/R1-astra-pkg1/REVIEW.md`)

One commit per finding on `stream/r1fix`; `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo build --no-default-features` green after each.

| Finding | Root cause | Fix | Regression |
| --- | --- | --- | --- |
| R1 settings recovery lost the recovered file | `keep_copy` renamed the active file away and nothing wrote the sanitized settings back | `model::settings_file::load`: write the sanitized complete settings to `settings.json.tmp`, write and read back the backup `settings.invalid-<n>.json`, only then replace the active file (`store::replace_file`); any failure leaves the original active and is logged | `tests/core04_recovery.rs` (two boots, write failure, no free backup name, non-JSON, old file name, orphaned `.bak`) |
| R2 native pause never owned the input | `published::INPUT_OWNERS` was never written | the producer and the poll (`InputAdapter::pause_begin/pause_end/poll`) are built and tested through the real code; the scene's `StatePause` begin/end sites (`sites_pause.rs`) are guarded and installed, **but Eden shows the Training pause menu does not run that state** (probes r1-03 to r1-06: 0 hits at the begin routine, both enter slots and the exit, control hook fired), so the Training signal is **still open** (reported to the lead) | `tests/input_pause.rs`, `tests/pause_sites.rs`; Eden pause lane (negative) |
| R3 delay 0 played a stale record | the oldest queued record was read before the queue was trimmed | `DelayQueue::step` handles 0 first (clear, return the current record); the decreasing-nonzero transition satisfies the spec's observable rule and is pinned | `tests/input_delay.rs` (three tests) |
| R4 developer gate accepted duplicates, oversize, read errors | last-line-wins parse of an 8 KiB prefix, `read_to_end` error ignored | `developer_ini::load` fails closed (above) | `tests/core01.rs` (three tests), install-line test |
| R5 no post-exercise allocation observation | the counter was read only at boot | `model::hook_stats` (allocator, per-hook counters), diag-only `hook_stats` line from the heartbeat thread, Eden gate `eden/hook_stats.py` | `tests/hook_alloc.rs` (allocating stub), `tests/eden/test_analyze.py` (gate) |

The Eden rows (labels `r1-NN`) are in `eden/EDEN-PACKAGE-1.md`, section "R1-fix".
