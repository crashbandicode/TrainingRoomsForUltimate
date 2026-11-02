# Test provenance, package 4 (recording, input display, chords, notifications)

Stream `stream/pkg4` (Claude Sonnet 5.5 high under lead `0caa72b8`). Playbook followed: `009c9ac`. Host tests are in
`tests/input01.rs` (SPEC-input-01), `tests/input02.rs` (SPEC-input-02), `tests/input04.rs` (SPEC-input-04),
`tests/input05.rs` (SPEC-input-05) and `tests/input_sites.rs` (sites against the owned 13.0.5 ELF); the carried files
carry their own unit tests (`src/model/menu_shortcuts.rs`, `src/model/capture_settings.rs`).

## Lead decisions applied (2026-11-02)

| Question | Decision | Where |
| --- | --- | --- |
| SPEC-input-01 question 2 (stick scale) | one scale, 1/120, clamped to +/-1; clamped fields drop values below 0.2 | `recording::inject_values`, `injected_stick_values` |
| SPEC-input-01 question 3 (transition-term ranges) | inclusive at both ends; every other starting status keeps the described behaviour, `CONT_SPECIAL_S`, for parity with the fork | `Recorder::term_for_status`, `starting_status_to_term`, `other_status_keeps_the_placeholder_term` (pins it: a later change is deliberate) |
| SPEC-input-01 question 4 (Recording Slot re-read every frame) | latch the slot and the duration at the lockout | `Recorder::start_recording` |
| End check and tint | run in the AI control hook, directly after the injection (same game frame as the on-frame wrapper) | `runtime/features/recorder.rs` |
| Replay capture default | the INI is the file of record; absent means off | `model::capture_settings`, menu `Env::default` |
| SPEC-input-02 question 1 (status names) | numeric ids until the names' source is decided (parked) | `input_display::StatusText` |

## Host rows

| Spec row | Test | State |
| --- | --- | --- |
| IN01 menu defaults, rows read | `menu_defaults_match_spec`, `settings_rows_are_read`, `override_playback_default_off` | pass |
| IN01 lockout, standby, recording; P1 neutralised | `recorder_lockout_then_standby_then_cpu`, `standby_ends_on_non_neutral`, `starting_status_captured_frame_one` | pass |
| IN01 neutral rules, crop, bounds, empty slot | `neutral_frame_rules`, `crop_trims_trailing_neutral`, `frame_bounds_store_and_play_full_length`, `playback_refuses_empty_slot` | pass |
| IN01 chord pick, mirror, Shulk dial, stick values, R6 | `playback_slot_pick_and_lockout_refusal`, `mirror_multiplier_and_shulk_dial`, `injected_stick_values`, `control_write_touches_only_verified_fields` | pass |
| IN01 ledge buffer, hitstun timing, term table | `ledge_buffer_schedule`, `hitstun_timing_modes`, `starting_status_to_term` | pass |
| IN01 loop, mash interrupt, save-state triggers, per-slot origin, stop, notices, named contract, size | `loop_restarts_playback`, `mash_interrupt_stops`, `save_state_triggers_once`, `per_slot_origin`, `session_clear_stops`, `phase_notifications`, `named_contract_export_import_refusals`, `recorder_state_is_the_24000_byte_slot_table_and_a_little_more` | pass |
| IN01 B4, N4 rows | N4 `override_playback_default_off`; B4 is a runtime check (`RecordRefusal::P1DrivenByAi`) | N4 host, B4 Eden (see EDEN-PACKAGE-4.md) |
| IN02 mode rules, binning, icons, fade, ring, counts | `difference_rules_smash_raw_status`, `bin_stick_values`, `icon_order_and_colours`, `frame_count_cap_99`, `frame_counts_count_in_game_frames_not_polls`, `fade_alpha_curve`, `ring_rotation_physical_index` | pass |
| IN02 renderer plan: hidden, refresh, registration, cache, budget, no heap | `hidden_under_menus`, `refresh_after_hide_and_reregister`, `unchanged_held_input_writes_nothing`, `rapid_changes_write_only_the_new_row_and_moves`, `bench_counts` | pass |
| IN04 rule, mapping, glyphs, latch, suppression, Plus | `hold_press_rule`, `single_button_semantics`, `button_mapping_by_style`, `glyph_table`, `request_latch_and_clear`, `suppressed_under_menus`, `plus_passthrough_and_suppression`, `detection_allocates_nothing` | pass |
| IN04 N7: editor rows, permanent chord, applies at once, INI written only on a confirmed change, fail-closed reads, write failure keeps the old file | `shortcut_rows_open_editor_and_permanent_chord_fixed`, `shortcut_change_applies_immediately`, `ini_written_only_on_confirmed_change`, `ini_reads_fail_closed`, `ini_write_failure_keeps_the_old_file`, `ini_missing_file_is_created_with_the_default_text`, `menu_shortcuts::tests::*` | pass (fs-backed, incl. a read-only folder) |
| IN05 queue, clear, hidden, first draw, no heap | `queue_order_and_lengths`, `clear_by_header_and_all`, `hidden_and_combo_display_off`, `first_draw_writes`, `tick_allocates_nothing` | pass |
| IN05 damage digits, pane rules, missing accessor | `damage_digits`, `pane_rules_fixture`, `missing_accessor_zero` | pass |
| Sites | `tests/input_sites.rs` (12 rows equal the ELF; the 6 natives are the symbols they name; the AI epilogue is the function's only return and no branch lands in it; the Combo Display pins are the spec's words; no window near a pilot site) | pass |
| Q7 replay capture INI | `capture_settings::toggle_tests::*` plus the carried tests | pass |

## Not testable on the host or not built in this stream (stated, not implied)

- Pane writes of the input display and the notice slot, the Combo Display consumer and the damage helper's pane writes wait for
  package 5's layout and anchors (SPEC-core-05); the plan and the model are built and tested, `write_plan` / `write_panes`
  report to the diagnostics only.
- Mash-triggered playback and the mash interrupt need package 2's per-frame mash trigger evaluation (not built there); the
  recorder side (`mash_start`, `interrupt_on_mash`, the queue pop) is built and host-tested.
- Save-state playback and record-on-load need package 3's restore completion; the diag build's stand-in (the save-state chord) calls
  `on_state_restored`.
- S4-4 (named macros, replay import, record-slot export, clip editor, live replay) waits for `training_mod_mii` (package 6).
