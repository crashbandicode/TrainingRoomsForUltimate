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

## Online review (read-only rules, hook by hook; SPEC-input-01 §4, playbook CHECKLIST "Online")

The package's hooks act only when `is_training_mode()` is true and, for the recorder, only on the CPU entry's own controller; in an online match every hook
returns before it reads or writes anything beyond that predicate.

| Hook | Reads | Writes | Draws game RNG / skips native code |
| --- | --- | --- | --- |
| P1 poll note (`input_caller_*_note`, shared with Input Delay) | the player's `Controller` | Plus bits of that controller only for a modal owner or an accepted Plus shortcut (never in an online match: no Training) | no |
| P1 poll apply (`input_caller_*_apply`) | the mapped record | the record (delay, recorder neutralising) in Training only | no |
| `ai_epilogue` (`main+0x2dbc38`) | AI object, CPU entry, status, facing | the CPU entry's own controller fields (buttons, stick, clamped stick), the cliff rest timer, the model colour blend: Training only, CPU entry only | no (the plugin's own generator picks slots) |
| `combo_display_a/b` | w20 / w8 (the byte the game just loaded) | the plugin's own atomic | no |
| `stale_moves_toggle` | the menu object pointer | one byte at `PauseMenu+0xb60` in Training only | no |

Replay A/B with the desync detector and the two-console online rows are not run (lead's to schedule); the plugin never writes simulated state outside Training.

## Checklist rows (package 4)

| Requirement | Result | Evidence |
| --- | --- | --- |
| cdylib only, release profile, skyline =0.6.0, toolchain 1.98.1, `model`/`runtime` split | pass | `compliance.py` machine rows |
| Gates: `cargo test` (24 binaries, 1,227 tests, 0 failed, Surface), clippy `--all-targets -D warnings` (0 warnings, Yoga gate folder with `SKIP_OWNED_ELF_FACT=1` because the owned ELF lives on the Surface; the ELF tests ran on the Surface), `cargo build --no-default-features`, Switch release and diag builds warning-free | pass | commands in `IMPLEMENTATION-PROGRESS-pkg4.md` |
| 64-byte guard on every site, card scan | pass | `tests/input_sites.rs`; scan over the 2026-11-01 10:19 preflight copy: only the fork NRO and the CPU pilot (`0x2da180`, the AI function's entry, not our site) reference our offsets; the Combo Display / stale-moves sites and the natives are referenced by the fork only |
| Hook placement (after the prologue / most specific instruction) | pass | `ai_epilogue` is the function's only return path, not its entry; the poll hooks are the caller-side sites of package 1 |
| Hook sharing with the CPU pilot (the pilot owns the AI function's entry) | pass | both load orders in Eden (s4-01, s4-02); no foreign hook at our rows (`stock`) |
| Shipped NRO writes only its install line | pass | `strings` of the release NRO: no `PracticeRecording` / `standin`; per-event lines and the writer thread exist only in the diag build; the INI is written only on a confirmed shortcut change and created once when absent |
| No heap in hooks | pass | `heap_allocations=0` at install and after 600-frame reports (s4-01, s4-02) |
| Eden: claim/release, one per host, profile copy, waits gated, console scan, run.json/memory.peak kept, profile restored | pass | `eden/EDEN-PACKAGE-4.md` |
| Workarounds | none shipped; open decisions: status names (SPEC-input-02 question 1), `CONT_SPECIAL_S` placeholder (kept by the lead's ruling for parity) |
| Slip | seven builds on the Yoga beside pkg5's Eden (disclosed in `IMPLEMENTATION-PROGRESS-pkg4.md`) |

## S4-4: named macros, replay store, live capture adapter, replay window (2026-11-02)

| Requirement | Host test | Eden row (label) |
| --- | --- | --- |
| Named lockout, capture from frame 0, buffer read-back, stop chord, named playback, mirroring, maximum length | `tests/input_named.rs` (8) | s4-04 named rows |
| The carried macros crate and panel models keep their tests (library, recording, window, move crop, capture archive, shared capture, macro panel, clip panel) | `cargo test --lib engine::macros engine::macro_ui` | - |
| The vendored match-data crate's own tests | `cargo test --manifest-path crates/smash-match-data/Cargo.toml` (22) | - |
| Carried files equal the manifest, ledger generated | `tests/carry_pkg4.rs` | - |
| Every game call of the layer is a guarded row | `tests/pkg6_smash_calls.rs`, `tests/input_sites.rs` (23 package-4 rows against the owned ELF) | install line `control_get_button=stock` ... |
| Stick direction bits of the controller record | `tests/input04.rs` `the_button_set_reads_the_dpad_and_both_sticks_digital_directions` | - |
| Typed menu rows fighter_prompt, macro_source | `compat::menu` test `the_macro_rows_follow_the_snapshot_and_default_to_ask_and_numbered_slots` | - |
| HUD plan diag line is coalesced | `model::input_display::plan_log_gate_tests` (3) | R5 delta in s4-04 |
| Save slot to library writes the slot's frames; load follows selected.txt | `named_macros` / `Library` (carried tests) | s4-04 |
| Live capture adapter installs only on guards and capture on | seam table tests (package 6) | s4-03: `guarded live adapter ready capture=on save=entry` |
| Replay window lists a capture and crops a macro | carried `ClipEditor` / `SharedCaptureArchive` tests | s4-04 (seeded fork capture) |

Online review (read-only rules): the layer runs only in Training and the practice session (`scene::is_training`, `macros::ready`); the live capture hooks observe the game's native replay recorder
(no write to simulated state; they do not draw or reorder checked randoms). Not yet reviewed in a two-console run (the hooks are installed only when `replay_capture.ini` says on).
