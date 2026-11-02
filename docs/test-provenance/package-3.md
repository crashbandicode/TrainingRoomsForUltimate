# Test provenance, package 3 (save states, charges, buffs, items, fighter-specific restore, first-boot import)

`Requirement | spec § | host test | Eden row | status`. Host tests run with `cargo test` (`tests/state01_json.rs`,
`state01_machine.rs`, `state01_setup.rs`, `state02_charges.rs`, `state03_buffs.rs`, `state04_convert.rs`,
`state04_items.rs`, `state_sites.rs`, `migration_run.rs`; each by-path binary also runs the model's unit tests, so
per-binary counts overlap). Eden labels `s3-NN` (results in `eden/EDEN-PACKAGE-3.md`). Status: **host pass**,
**Eden pass**, **runtime, unrun** (written and Switch-compiled, no Eden row yet), **not built** (reported).

## SPEC-state-01 save / load core

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

## SPEC-state-02 charges, SPEC-state-03 buffs

| Requirement | Host test | Eden row | Status |
| --- | --- | --- | --- |
| Read table per kind, write only present fields | `state02_read_table_per_kind`, `state02_write_only_present_and_matching_kind` | s3-02 (Samus) | see EDEN-PACKAGE-3 |
| Full-charge visuals, Sheik needles, G&W, Pac-Man, Robin, Incineroar, Hero | `state02_full_charge_visuals`, `state02_sheik_needle_groups`, `state02_gw_bucket_clear_flash`, `state02_pacman_key_flag`, `state02_robin_tomes`, `state02_incineroar_revenge`, `state02_hero_clear_then_set` | s3-02 (Samus glow) | Samus: see EDEN-PACKAGE-3; the others runtime, unrun |
| Olimar line, Sora spell | `charge::line_order`, `prev_colour`, `spawn_colour`, `sora_written_kind` (in the charge model, exercised by the state02 table) | - | runtime, unrun |
| Buff steps: Hero, Cloud/Joker/Mac, Sephiroth, Wario, Shulk, Wii Fit, Min Min, unselected, per-record bookkeeping | `tests/state03_buffs.rs` (12) | s3-03 (Cloud Limit) | see EDEN-PACKAGE-3; the others runtime, unrun |

## SPEC-state-04 items, SPEC-state-05 fighter-specific restore

| Requirement | Host test | Eden row | Status |
| --- | --- | --- | --- |
| Item table for the 38 choices, sources, common items, snapshot precedence, timers rule, turnip face hooks, choice of a held item | `tests/state04_items.rs` (7) | s3-02 (Bob-omb snapshot) | see EDEN-PACKAGE-3 |
| N6 conversion of the retired `character_item` (all values, never overwrites, no entry leaves absent) | `tests/state04_convert.rs` (7), `migration_item_on_load_*` | s3-04 (import copy) | host pass |
| Steve materials and tools restored, bar refreshed | `steve` layout in the model (round trip) and the fixture | s3-01: slot 3 fixture, wood 77 / sword 11.5 etc. read back exactly | Eden pass |
| Keep-list, Pikmin autonomy gate | `Table::keeps_article`; the gate is a hook | - | runtime, unrun |
| Pokemon Trainer pre-death / decide hook / waits / effects, Kirby copy and copied charge | - | - | **not built** (reported) |

## SPEC-core-08 first-boot import (S3-5)

| Requirement | Host test | Eden row | Status |
| --- | --- | --- | --- |
| Plan splits blocking and background; originals untouched; receipt shape and hashes | `migration_plan_splits_blocking_and_background`, `migration_full_import_receipt_and_originals`, `migration_receipt_hashes_the_converted_copy_separately` | s3-04 | host pass |
| N6 on the copy only, never over a snapshot or an existing save | `migration_item_on_load_converts_player_records_in_the_copy`, `migration_item_on_load_never_overwrites_a_snapshot_or_an_existing_save` | s3-04 | host pass |
| `developer.ini` does not mark imported, left byte-identical | `migration_developer_ini_does_not_mark_imported` | s3-04 | host pass |
| Marker blocks a second boot; no source writes `none` | `migration_marker_blocks_a_second_boot`, `migration_no_source_writes_a_none_marker` | s3-04 | host pass |
| Existing destinations never overwritten; size caps; captures uncapped | `migration_existing_destinations_are_never_overwritten`, `migration_captures_have_no_size_cap_and_the_rest_do` | s3-04 | host pass |
| Staging folders skipped, never read, counted | `migration_staging_folders_are_skipped_and_never_touched` | s3-04 | host pass |
| I/O error partial, unreadable source partial, interrupted background rerun, marker write failure reruns | `migration_io_error_is_partial_and_the_rest_is_copied`, `migration_unreadable_source_file_is_partial`, `migration_interrupted_background_copy_reruns_and_completes`, `migration_marker_write_failure_leaves_the_import_to_run_again` | s3-04 | host pass |
| SHA-256 vectors | `sha256::tests` | - | host pass |
