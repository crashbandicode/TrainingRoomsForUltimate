# Test provenance, package 2 (CPU behaviour)

`Requirement | spec § | host test | Eden row planned | status`. Host tests run with `cargo test`
(`tests/cpu_core.rs`, `tests/cpu02.rs`, `tests/cpu03.rs`, `tests/cpu_sites.rs`; the model's unit tests are
compiled into each by-path binary, so per-binary counts overlap). Eden row labels are the specs' own and are
batched per boot in `eden/EDEN-PACKAGE-2-PLAN.md`; no Eden run has happened. Status: **host pass** = test exists
and passes; **runtime, unrun** = the hook is written and Switch-compiled but not run on Eden; **blocked** = needs
an input the implementer does not have (reason in the row); **later** = the spec row belongs to a later milestone
of this package.

Design facts the tests rest on (all in `src/model/cpu/`): one `Cpu` value (settings, the plugin's own generator,
the constant table, the role table, one state per fighter body); every hook reads the game first, takes the one
lock for a pure decision, releases it and applies the answer; no status, term or work id is written down: they are
asked from the game's constant table by name at run time (`hash.rs`, `ids.rs`).

## Shared by the CPU specs (SPEC-cpu-01 §3/§5, SPEC-cpu-07 §5, decisions B4, B5/N4, B6)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| The game's constant table is keyed by XXH64 (seed `u64::MAX`) of the identifier; Hash40 is CRC-32 and length | cpu-01 §3 | `cpu_ids_hash_matches_the_published_vectors`, `cpu_ids_hash40_is_the_crc_and_the_length` | `cpu-01-install-line` (constants missing = 0) | host pass; runtime, unrun |
| Constant names unique and hash apart; resolve by key; unresolved matches nothing | cpu-01 §3 | `cpu_ids_names_are_unique_and_hash_apart`, `cpu_ids_resolve_asks_by_hash_and_lists_the_missing` | same | host pass |
| Situation predicates (hitstun end-exclusive, tumble, footstool, shield, shieldstun, tech, trump, landing, clatter) | cpu-01 §3 | `cpu01_situation_predicates` | - | host pass (synthetic numbering; real values come from the game) |
| Weighted pick: weight 0 never, proportional, empty = none, no game RNG | cpu-01 §3 | `cpu01_weighted_pick_distribution` | - | host pass |
| Picks (weighted, delay checklist, delay range) never allocate | cpu-01 §8 | `cpu01_weighted_pick_no_alloc` (counting allocator) | - | host pass |
| Delay rows: empty = 0, checklist weighted, range uniform and inclusive, min = max fixed, a set range is a selection | N3 | `cpu_core_delay_rows_pick_checklists_and_ranges` | - | host pass |
| CPU settings read from the snapshot (factory values, a changed snapshot, ranges, tilt, override-playback default Off) | cpu-01..06 §2 | `cpu_settings_read_the_factory_rows`, `cpu_settings_read_a_changed_snapshot` | - | host pass |
| B4: CPU role only (entry 0 never, entry 1 in Training, entry 4 in the lobby) | cpu-01 Q7 | `cpu01_cpu_role_only_when_p1_ai` | `cpu-01-p1-ai-untouched` | host pass; runtime, unrun |
| Role table: accessor to body, replaced, forgotten, cleared | cpu-01 §6 | `cpu_core_role_table_finds_bodies_by_accessor` | - | host pass |
| B6: Popo and Nana each have their own state | cpu-01..06 | `cpu_core_bodies_keep_separate_state`, `cpu02_per_fighter_state`, `cpu03_per_fighter_state` | `cpu-02-ice-climbers-sdi`, `cpu-03-ice-climbers` | host pass; runtime, unrun |
| B5/N4: covered settings yield to a playing recording unless *Settings override playback* is On | cpu-02 §6 | `cpu_core_covered_settings_yield_to_playback_by_default` | - | host pass |
| `should_delay` semantics | cpu-07 §5 | `cpu07_should_delay` | - | host pass |
| Tick advances counting counters; reset clears them | cpu-07 §5 | `cpu07_tick_types_and_reset` | - | host pass (only in-game counters exist so far; the Real and no-reset types come with the menu and frame advantage) |
| One in-game tick per simulation frame with two bodies | cpu-07 §5 | `cpu07_single_tick_per_frame` | `cpu-07-ice-climbers-delay-exact` | host pass; runtime, unrun |
| Reset hub steps clear mash, SDI roll, counters, ledge delay, in hub order | state-06 §5.2 | `state06_hub_order_clears_cpu_state` | `E-state-06-reset-clears-mash` | host pass; runtime, unrun (combo, throw and shield-tilt steps come with their specs) |
| Menu re-roll events: SDI / shield tilt left pending for the next CPU frame, ledge case cleared at once (Q18 A, B11) | cpu-02 §5, cpu-03 §5 | `cpu_core_publish_consumes_the_menu_rerolls` | `cpu-02-sdi-change-no-reset`, `cpu-03-wait-then-change-to-roll` | host pass; runtime, unrun (the menu is package 5: `published::publish` takes the outbox bits) |
| A frame with nothing to do asks for nothing (staling flag once) | cpu-02 §8 | `cpu_core_frame_with_no_work_asks_for_nothing` | - | host pass |
| Mash queue: main then follow-up, completion pops, refuse when non-empty | cpu-01 §5 | `cpu01_queue_main_then_followup` | `cpu-01-followup-nair-then-jump` | host pass (the queue and its buffer entry; the per-frame trigger evaluation and command flags are P2-7) |
| Playback Mash Interrupt | cpu-01 §5 | `cpu01_playback_interrupt` | `cpu-01-playback-interrupt` | host pass (buffer rule only) |
| Lobby effective-action filter | cpu-01 §3 | `cpu01_lobby_effective_actions` | `cpu-01-lobby-mash` | host pass (the lobby scene is package 6) |
| Roll points at buffering (attack angle, aerial delay, full hop, fast fall, falling aerial) | cpu-06 §5 | `cpu06_roll_points` | - | host pass |

## SPEC-cpu-02 (DI, SDI, air-dodge direction, clatter, dodge staling)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Direction to angle table, Neutral none, Left/Right absolute | §2 | `cpu02_direction_angles` | - | host pass |
| Mirroring when the player is to the CPU's right, never for Left/Right (B7: opponent-relative on both sides) | §2 | `cpu02_mirror_rule`, `cpu02_airdodge_opponent_relative_both_sides` | `cpu-02-di-out-both-sides`, `cpu-02-airdodge-out-left-and-right` | host pass; runtime, unrun |
| DI case held for the hitstun episode, cleared outside it; Neutral writes (0, 0); empty row writes nothing; the case clears in the frame dispatch | §5 | `cpu02_di_case_lifetime`, `cpu02_di_case_clears_in_the_frame_dispatch` | `cpu-02-di-multihit`, `cpu-02-di-neutral` | host pass; **blocked** for the runtime: the DI hook sits in the `common` script module and its 64-byte guard cannot be taken (see below) |
| SDI period 8 / 6 / 4, None never, input frame writes the vector | §5 | `cpu02_sdi_period` | `cpu-02-sdi-high-up` | host pass; runtime, unrun |
| SDI re-rolls on the reset bundle and on a row change, not per hitlag episode (Q18 A) | §5 | `cpu02_sdi_reroll`, `cpu02_sdi_rolls_on_row_change_and_reset`, `cpu02_sdi_roll_not_per_hitlag` | `cpu-02-sdi-two-directions`, `cpu-02-sdi-change-no-reset` | host pass; runtime, unrun |
| N9: SDI not set means no SDI (empty direction, Strength None, Neutral; the original is never called) | §5 | `cpu02_sdi_unset_means_no_sdi` | `cpu-02-sdi-unset-none` | host pass; runtime, unrun |
| Air-dodge direction: one pick per aerial hitstun episode, consistent x and y, only airborne in hitstun | §5 | `cpu02_airdodge_direction_consistency` | `cpu-02-airdodge-down-out` | host pass; runtime, unrun |
| Clatter period 8 / 5 / 2, step from the start call, effect on input frames | §5 | `cpu02_clatter_period_and_step` | `cpu-02-clatter-grab-high`, `cpu-02-clatter-freeze` | host pass; runtime, unrun |
| Dodge staling flag equals the negated row, written on the first frame and on change, repeated after a forget | §5 | `cpu02_stale_flag_follows_setting` | `cpu-02-dodge-staling-off` | host pass; runtime, unrun (open question 5: Eden must show the game does not clear the flag) |
| B5/N4: DI, SDI and air-dodge overrides skipped while a playback possesses the CPU; applied with *Settings override playback* On | §6 | `cpu02_playback_wins_default`, `cpu02_playback_override_on` | `cpu-02-di-during-playback-off`, `cpu-02-di-during-playback-on` | host pass; runtime, unrun (the recorder is package 4: `Cpu::playback` is set by it) |
| Common-module hook installs only after its guard matches the loaded image | §9 | - | `cpu-02-install-line` | **blocked**: no 13.0.5 `common` image bytes to take a guard from |

## SPEC-cpu-03 (ledge options)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Case and delay rolled once per grab, cleared when leaving the ledge | §5 | `cpu03_case_lifetime` | `cpu-03-regrab-rerolls` | host pass; runtime, unrun |
| Status per option (climb, escape, jump1, attack) | §4 | `cpu03_option_status_table` | `cpu-03-each-getup` | host pass; runtime, unrun |
| Zero-delay buffer frames 19 regular, 18 tether, 13 playback | §5 | `cpu03_buffer_frames` | `cpu-03-zero-delay-first-frame`, `cpu-03-tether-first-frame` | host pass; runtime, unrun (the frame numbers are the spec's tuned values; Eden confirms, open question 2) |
| Delay counted only from actionability; Wait holds; climb denied while waiting or delaying | §4, §5 | `cpu03_delay_and_climb_denial` | `cpu-03-delay-60`, `cpu-03-wait` | host pass; runtime, unrun |
| Playback option: early buffer needs Ledge Delay non-empty; normal path otherwise; climb denied until the playback starts | §5 | `cpu03_playback_paths` | `cpu-03-playback-slot1` | host pass; runtime, unrun (the recorder is package 4; the request is carried in `FrameOut::playback` and not yet delivered) |
| Post-getup mash: override per kind, else trigger, else none | §5 | `cpu03_post_getup_mash` | `cpu-03-roll-then-fsmash` | host pass; runtime, unrun |
| Empty effective set: native behaviour, climb not denied | §5 | `cpu03_empty_effective_set` | `cpu-03-lobby-playback-only` | host pass (the lobby scene is package 6) |
| N3: a set range is picked uniformly once per grab; a set range counts as a non-empty row | §5 | `cpu03_ledge_delay_range_pick`, `cpu03_range_counts_as_nonempty_row` | `cpu-03-delay-range-30-60` | host pass; runtime, unrun |
| N3: checklist and range exclusive, legacy file loads as a checklist | §9 | covered by `core02_delay_row_exclusion` (package 1 menu tests) and `cpu_settings_read_a_changed_snapshot` | - | host pass |
| B11: changing Ledge Options clears the held case and re-rolls in the same hang; emptying the row stops the feature; the held delay is unchanged | §5 | `cpu03_ledge_options_change_rerolls_held_case`, `cpu03_ledge_options_emptied_clears_case` | `cpu-03-wait-then-change-to-roll`, `cpu-03-change-to-empty` | host pass; runtime, unrun |
| Yields during input playback; reset bundle resets the delay and its counter | §5 | `cpu03_playback_yield_and_reset` | `cpu-03-savestate-on-ledge` | host pass; runtime, unrun |

## SPEC-cpu-04 (tech, missed tech, hide tech)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Ground tech rewrite per option; No Tech leaves DOWN; no tech without the PASSIVE term and the script's check; Ganondorf's aerial slam not techable; no roll unless a tech situation applies | §5 | `cpu04_ground_tech_table` | `cpu-04-tech-each-option`, `cpu-04-ganon-side-b` | host pass; **blocked** for the runtime: the script status-change hook is in the `common` module |
| Wall and ceiling tech rewrites | §5 | `cpu04_wall_ceiling_tech` | `cpu-04-wall-tech-jump`, `cpu-04-ceiling-tech` | host pass; blocked (same hook) |
| Motion substitution follows the held roll directions; tech and getup animations by their own rows | §5 | `cpu04_motion_substitution` | `cpu-04-roll-back-animation` | host pass; runtime, unrun (`cpu_motion_change_hook`) |
| Mistech per option from down-wait (both kinds) and slip-wait | §5 | `cpu04_mistech_table` | `cpu-04-mistech-each-option`, `cpu-04-banana-slip` | host pass; runtime, unrun (per-frame, `change_status_force`) |
| Mistech no longer depends on Tech Options (B8, question 1) | §10 | `cpu04_mistech_gate_is_its_own_row` | - | host pass |
| Lay-down lock-out formula and its counter; other statuses reset it | §5 | `cpu04_laydown_lockout` | `cpu-04-snake-dthrow` | host pass; runtime, unrun |
| Tech and Mistech triggers buffer the override or the mash | §5 | `cpu04_tech_triggers_mash` | `cpu-04-tech-then-usmash` | host pass; runtime, unrun for mistech, blocked for tech |
| Hide from frame 6, show in the last 5, cursor 800 then restored; restore on status end, toggle Off mid-tech and session end | §5 | `cpu04_hide_tech_lifecycle`, `cpu04_hide_tech_toggle_off_mid_tech` | `cpu-04-hide-tech`, `cpu-04-hide-tech-toggle-off-mid-tech` | host pass; runtime, unrun |
| Quake suppressed only in DOWN with hide On | §4 | `cpu04_quake_rule` | `cpu-04-mistech-no-shake` | host pass; runtime, unrun (`cpu_quake_pos_hook`) |
| Camera centre per stage class | §5 | `cpu04_camera_table` | `cpu-04-fixed-camera-bf`, `cpu-04-fixed-camera-smashville` | host pass for the table; **blocked** for the runtime: the stage-id to class mapping needs the game's `StageID` numbers, which no spec or owned source gives, and the camera-manager needle sites are not in this table |
| Primary body only; Nana visible; the record dies with the session | §6 | `cpu04_primary_body_only` | `cpu-04-ice-climbers-hide` | host pass; runtime, unrun |
| B6 per-fighter held roll directions | §10 | `cpu04_per_fighter_state` | - | host pass |
| B5/N4 recording wins by default; override On applies | §6 | `cpu04_playback_wins_default`, `cpu04_playback_override_on` | `cpu-04-tech-during-playback-off/on` | host pass |
| Common-module hook guard | §9 | - | `cpu-04-install-line` | **blocked** (no `common` image) |

## SPEC-cpu-05 (shield, out-of-shield, OoS offset, reaction time, tilt)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Hold predicate (playback, standby, loading, was-airborne, queue request, toggles), guard-button answers | §5 | `cpu05_should_hold_shield` | `cpu-05-hold-shield-stands` | host pass; runtime, unrun (`cpu_check_button_on/off_hook`) |
| Decay override per toggle; Hold flips after the first hit and back when not guarding; None native | §5 | `cpu05_decay_override_rules` | `cpu-05-infinite-no-decay`, `cpu-05-hold-decays-after-hit`, `cpu-05-none-native-decay` | host pass; runtime, unrun (`cpu_shield_param_hook`); the flip on leaving shieldstun needs the `common` script hook (blocked), so Hold never flips in the runtime |
| Damage multiplier 0 for Infinite, restored otherwise and outside Training, cached across session clears | §4 | `cpu05_damage_mul_restore` | `cpu-05-leave-training-restores-mul` | host pass; **blocked** for the runtime: it needs the param-hook plugin's pointer (SPEC-cpu-05 question 6, the lead decides) |
| OoS offset: N hits shielded then act; reset when idle or in hitstun | §5 | `cpu05_oos_offset_count` | `cpu-05-oos-offset-2` | host pass; blocked (out-of-shield hook) |
| Reaction time counted after shieldstun; parry blocks; not shielding resets | §5 | `cpu05_reaction_time` | `cpu-05-reaction-10`, `cpu-05-parry-no-oos` | host pass; blocked |
| Each OoS action: dodge or roll by status, jump, aerial, up special and up smash by native release, grab without drop, the rest with suspension | §5 | `cpu05_oos_action_table` | `cpu-05-oos-each-action` | host pass; blocked |
| Shieldstun override vs trigger | §5 | `cpu05_shieldstun_override` | `cpu-05-shieldstun-override-nair` | host pass; blocked |
| Shield tilt; a changed tilt applies at the next CPU frame and on the reset (Q18 A); In / Out opponent-relative (B7) | §5 | `cpu05_shield_tilt`, `cpu05_shield_tilt_applies_next_frame`, `cpu05_shield_tilt_opponent_relative` | `cpu-05-tilt-up`, `cpu-05-tilt-change-no-reset`, `cpu-05-tilt-in-left-and-right` | host pass; runtime, unrun (`cpu_stick_x/y_no_clamp_hook`) |
| N3: OoS Offset and Reaction Time ranges | §9 | `cpu05_oos_offset_range_pick`, `cpu05_reaction_time_range_pick` | `cpu-05-oos-range-1-3`, `cpu-05-reaction-range-5-15` | host pass |
| B6 per-fighter state | §10 | `cpu05_per_fighter_state` | `cpu-05-ice-climbers` | host pass |
| Common-module hook guard; param-hook absent path logs once | §9 | - | `cpu-05-install-line` | **blocked** |

## SPEC-cpu-01 (the mash run), SPEC-cpu-06 (aerials, throws, angle, fast fall, full hop, crouch), SPEC-cpu-07 (frame advantage)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Trigger priority tech > clatter > tumble > hitstun > parry > footstool > trump > landing > generic | cpu-01 §5 | `cpu01_trigger_priority` | `cpu-01-tumble-vs-hitstun` | host pass; runtime, unrun (`cpu_command_flag_hook`) |
| Override replaces mash_state without its trigger; an empty override falls back | cpu-01 §5 | `cpu01_override_precedence` | `cpu-01-hitstun-override-only` | host pass; runtime, unrun |
| Distance thresholds 16 / 37 / 64, infinity without a player (3-D distance from x, y, z) | cpu-01 §3 | `cpu01_distance_thresholds` | `cpu-01-distance-mid` | host pass; runtime, unrun |
| One roll per situation entry (B8), a new one at the next entry | cpu-01 Q1 | `cpu01_one_roll_per_situation` | `cpu-01-tumble-vs-hitstun` | host pass; runtime, unrun |
| Each action's command bits and expected status; jab only grounded; grab blocked by the invalid-catch frame; shield airborne returns 0 and stays queued | cpu-01 §5 | `cpu01_action_flags_table` | `cpu-01-mash-each-action` | host pass; runtime, unrun |
| Dash attack: dash first, one dash frame, then the dash attack; no re-buffer while beginning it | cpu-01 §5 | `cpu01_dash_attack_sequence` | `cpu-01-dash-attack` | host pass; runtime, unrun |
| Character-specific completion (Bowser, Kirby range; Sora and K. Rool rows are in the table) | cpu-01 §4 | `cpu01_character_specific_completion` | `cpu-01-kirby-hat-neutral-b`, `cpu-01-bowser-up-b` | host pass; runtime, unrun |
| A menu save that empties Mash Toggles and Triggers empties the queue once | cpu-01 §5 | `cpu01_menu_disable_clears_queue` | `cpu-01-menu-disable` | host pass; runtime, unrun |
| Reset bundle empties the queue and latch; no choice while a save state loads | cpu-01 §5 | `cpu01_reset_clears_queue` | `cpu-01-savestate-reset` | host pass |
| Nana receives bits; an unresolved table asks for nothing | cpu-01 §6 | `cpu01_nana_and_missing_accessor` | `cpu-01-ice-climbers` | host pass |
| Playback Mash Interrupt stops the playback and triggers again after it | cpu-01 §5 | `cpu01_playback_stops_on_a_triggered_mash`, `cpu01_playback_interrupt` | `cpu-01-playback-interrupt` | host pass; the request goes to `published::CPU_STOP_PLAYBACK` for package 4 |
| Aerial delay counts only when CONT_ATTACK_AIR is enabled; 0 and in-ATTACK_AIR wait for nothing | cpu-06 §5 | `cpu06_aerial_delay_rules` | `cpu-06-aerial-delay-10` | host pass; runtime, unrun |
| Falling aerial waits for negative gravity speed | cpu-06 §5 | `cpu06_falling_aerial` | `cpu-06-falling-nair` | host pass; runtime, unrun |
| Full hop holds the jump buttons during JUMP_SQUAT only, not during playback | cpu-06 §5 | `cpu06_full_hop_hold` | `cpu-06-full-hop`, `cpu-06-short-hop` | host pass; runtime, unrun |
| Fast fall: delay rolled at the apex, flag and flash once, only in FALL / ATTACK_AIR, not during playback | cpu-06 §5 | `cpu06_fast_fall_rules` | `cpu-06-fast-fall-delay-5` | host pass; runtime, unrun |
| Attack angle: up and down, not on the ledge, not in playback, cleared when the queue empties (B8) | cpu-06 §5 | `cpu06_attack_angle` | `cpu-06-ftilt-up` | host pass; runtime, unrun (`cpu_stick_dir_hook`) |
| Crouch in the six statuses only | cpu-06 §5 | `cpu06_crouch_statuses` | `cpu-06-crouch` | host pass; runtime, unrun (`cpu_stick_y_hook`; the air-dodge direction answers first) |
| Throw: case and delays once per grab; None does nothing; pummel timing; throw bits on category 2; the mash after the throw ignores the triggers | cpu-06 §5 | `cpu06_throw_state_machine` | `cpu-06-uthrow-delay-30-pummel-10`, `cpu-06-throw-none` | host pass; runtime, unrun |
| B11: a changed Throw Options row re-rolls the held case in the same hold | cpu-06 Q9 | `cpu06_throw_options_change_rerolls_held_case` | `cpu-06-throw-change-while-holding` | host pass; runtime, unrun |
| B5 / N4: throws yield to a recording by default | cpu-06 §6 | `cpu06_throw_yields_to_playback` | `cpu-06-throw-during-playback-off/on` | host pass |
| N3 ranges for the four delay rows | cpu-06 §9 | `cpu06_range_picks_and_exclusive_rows` | `cpu-06-aerial-range-5-15`, `cpu-06-throw-range-20-40`, `cpu-06-pummel-range-on` | host pass |
| B6 per-fighter state | cpu-06 Q5 | `cpu06_per_fighter_state` | `cpu-06-ice-climbers-grab` | host pass |
| Frame advantage sign, lock-in, one measurement per exchange, colours and text | cpu-07 §5 | `cpu07_frame_advantage_math` | `cpu-07-jab-on-shield-minus`, `cpu-07-plus-frames` | host pass; runtime, unrun (published to `published::FRAME_ADVANTAGE_*` for the UI package) |
| No work when Off; aborted by a mash set | cpu-07 §5 | `cpu07_frame_advantage_gates` | `cpu-07-off-no-notification` | host pass |
| The reset bundle and a session clear clear the measurement | cpu-07 §5 | `cpu07_reset_bundle_clears_the_measurement` | `cpu-07-lra-reset` | host pass |

**Spec budget row (cpu-07 §8).** Counters are per fighter body (decision B6, accepted 2026-11-02): 7 counters x 16 bodies, about 450 bytes of
state against the spec's roughly 128 bytes for one shared table; the clock still ticks once per simulation frame.

## Sites (SPEC-cpu-01..05 §4)

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Every row is the symbol it claims (31 lua_bind functions and the constant lookup, checked against the ELF's symbol table); 64 guard bytes equal the owned 13.0.5 `main` (`core01_sites_match_owned_elf`) | §4 | `cpu_sites_are_the_symbols_they_name`, `core01_sites_match_owned_elf` | `cpu-0N-install-line` | host pass |
| Every hooked thunk is a 16-aligned leaf thunk that fits Skyline's 16-byte far jump without touching the next function | playbook | `cpu_sites_every_replace_hook_is_a_thunk_that_fits_the_far_jump` | `cpu-0N-install-line` | host pass |
| No site or guard window within 0x40 of a site of the CPU pilot; only our own rows overlap each other | playbook | `cpu_sites_are_not_near_the_cpu_pilot`, `cpu_sites_overlapping_windows_are_all_our_own` | pilot coexistence rows | host pass |
| Features need the core's natives; the slot row records its original | §4 | `cpu_sites_features_need_the_core`, `cpu_sites_the_frame_slot_holds_the_original_it_replaces` | - | host pass |
| Card scan (2026-11-01 10:19 MTP preflight copy, 16 plugins): only `libtraining_modpack.nro` (the fork, never co-installed, B1) references any of the 19 offsets; `libssbu_cpu_pilot.nro` and every other plugin: no reference | playbook | `tools/build/scan-offsets.py` (output in `IMPLEMENTATION-PROGRESS.md`) (removed after v0.2.0-rc.1; see that tag) | - | pass |

## Not in this milestone

The fixed camera (stage table) and the shield damage multiplier are open (IMPLEMENTATION-PROGRESS-pkg2.md (removed after v0.2.0-rc.1; see that tag), "Still open").

## Update 2026-11-02 (successor worker): Eden runs s2-01..s2-04 on the Ally, the two open items closed

Evidence and raw lines: `eden/EDEN-PACKAGE-2.md`. Rows whose status changed:

| Requirement | § | Host test | Eden row | Status |
| --- | --- | --- | --- | --- |
| Shield damage multiplier 0 for Infinite, restored otherwise, no provider plugin | cpu-05 §4, question 6 | `cpu05_damage_mul_restore`, `cpu_sites_common_params_fill_stores_shield_damage_mul_at_the_field_offset` | `cpu-05-leave-training-restores-mul` | host pass; **Eden pass** for Infinite and back to None (`write=0.000 was=1.190`, `write=1.190 was=0.000`); leaving Training not run |
| Fixed camera centre per stage class, stage ids from the game's table | cpu-04 §5 | `cpu04_camera_table`, `cpu04_stage_class_follows_the_native_ids_and_the_record_form`, `cpu04_camera_setup_and_mode_apply_the_stage_value_and_restore_the_default`, `cpu_sites_stage_table_records_are_the_stages_the_camera_rule_names`, `cpu_sites_fixed_camera_setup_loads_the_manager_through_the_slot_and_stores_the_centre_at_the_field` | `cpu-04-fixed-camera-bf`, `cpu-04-fixed-camera-smashville` | host pass; Eden: setup write issued for Final Destination (`write=Some([0,50,230])`, Off: `None`); the on-screen effect, camera mode 4 and Smashville not run |
| Common-module hook guards and install | cpu-02/04/05 §9 | `cpu_common_guards_match_the_owned_module` | `cpu-0N-install-line` | **Eden pass**: `cpu_common_load module=common … cpu_common=installed` (loader name `common`) |
| DI | cpu-02 §4 | `cpu02_di_case_lifetime` | `cpu-02-di-out-both-sides` | **Eden pass** for one side (`di_before role=true`, `di x=1.0 y=0.0`, `DAMAGE_FLY` hits only) |
| SDI, air-dodge direction, dodge staling | cpu-02 §5 | `cpu02_sdi_period`, `cpu02_airdodge_direction_consistency`, `cpu02_stale_flag_follows_setting` | `cpu-02-sdi-high-up`, `cpu-02-airdodge-down-out`, `cpu-02-dodge-staling-off` | **Eden pass** |
| Menu re-roll bits on a whole-settings replacement | cpu-02/03 §5, B11, Q18 A | `cpu03_whole_settings_replacement_yields_the_menus_reroll_bits` | `cpu-02-sdi-change-no-reset`, `cpu-05-tilt-change-no-reset` | **Eden pass** for the reload (`rerolls=0x3` and others); the ledge B11 row not run |
| Ledge Roll with a delay | cpu-03 §4, §5 | `cpu03_delay_and_climb_denial` | `cpu-03-delay-60` | **Eden pass** (one grab: climb denied, then `CLIFF_ESCAPE` requested) |
| Mash, each action | cpu-01 §5 | `cpu01_action_flags_table` | `cpu-01-mash-each-action` | **Eden pass** for 16 of 18 actions |
| Playback request word | cpu-03 §5 | `cpu03_playback_request_word_round_trips_for_every_slot` | - | host pass |
| Install line fits its buffer | core-01 §7 | `the_install_line_of_every_site_and_feature_fits_its_buffer_with_room_to_spare` | `cpu-0N-install-line` | host pass; found by Eden (line cut at 2048 bytes) |
