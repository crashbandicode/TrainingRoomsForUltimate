# The ACTION model (design, 2026-11-24)

## What it is

An action is an originating input and the status chain it starts. It owns its release (and so its hold), its angle
and its aims. A step starts only at an originating input, and the grader compares actions: what was pressed, how long
it was held, which angle the game chose and where the special went. One description is read by the strip, the bar,
the menu and the grader. The design and requirement trace are `artifacts/phase23/action-model/DESIGN.md` (base
`e7667f26`); research is in `artifacts/phase23/research/general-input-recording/REPORT.md` and `REPORT-2.md`.

Travel detail and the angle-and-deflection toggle of DESIGN §8–§9 are described under "Travel parts and the stick
toggle".

## The five defects it removes

| Defect | Cause | Rule |
| --- | --- | --- |
| Reset swallowed the next input | The cleared tracker forgot the held pad state | Clear keeps buttons and stick; the drill attempt resets. |
| A late first aim split a step | Every event after an idle gap opened a step | Only an originating input does; the first aim appends. |
| A held button's release split a step | A release after the gap opened a step | A release appends to the active step. |
| A release was graded on its stick | Release direction was compared like a press | Compare the owned hold length instead. |
| Bar and menu omitted a link's arrow | Each site named the move independently | All read the same description and formatter. |

## Ownership and steps

`action::originates` accepts a press, C-stick input or flick. While grouping a recording,
`InputCapture::step_with` starts a step when one of these arrives more than 20 frames after the last retained event;
the drill helper instead grades against the reference's step schedule. A second or later aim starts its own step,
and a full 12-event step continues in another. Other events append to the active step: a release or first aim does not
itself split on idle. `Combo::unreleased` discards a release when the capture has no unmatched press for that button.
A button already held when a take starts is a press.

## Clear

`Tracker::clear` retains `buttons` and `stick`, clearing its other state. A drill step opened by a press still held
at clear, with no release after that press, belongs to the clear gesture. A result is provisional while all presses
that opened its steps remain held, even after the attempt is forgotten. A clear withdraws the leading provisional
results and restores their displaced history, streak and best; an open gesture-only attempt is discarded. Releasing
an opening button makes its result and older provisional results final. Load after waits for a provisional result to
become final and is dropped if the clear withdraws it. Otherwise an ungraded final gesture step is removed and an
open attempt with a graded step is cleared; an open attempt without a graded step is discarded. Hit-combo clearing
remains as before.

The game acts on its Training reset chord about eight frames before it initialises the fighters again (Eden
`i4-final-yoga-01`: the chord's first status request at frame 58324, the fighter gone at 58332, initialised at
58336), so the drill takes the reset from the chord. On a poll where Player 1's raw controller holds L+R+A in
native Training with L+R+A Reset On, the runtime sets `RESET_COMMAND` in that poll's buttons; it rides in the record
through Input Delay (`input_delay::with_reset_command`, `split_reset_command`), so the game never gets the bit. On a
frame that carries it the drill clears the attempt as at a clear, after the frame's presses are taken, and leaves
the hit-combo capture alone. With L+R+A Reset Off the chord resets nothing and its presses are ordinary input.

## Grading

`grade_with` matches events in order and uses `GradeContext` for hold and aim ownership.

| Event | Compared |
| --- | --- |
| Press, C-stick, flick | Kind, direction and offset; an old presses-only link matches button and its four-way aim. |
| Special press owning aims | Kind and offset, not its stick direction; the aim events carry the direction. |
| Release | In a drill, only a reference release marked as reacted is graded, by its owned hold; direction is ignored. A hit link compares releases as before. |
| Aim | Part and facing-relative eight-way direction; for an aim-bearing target its timing delta is zero. |

`Grade.band` adds `min(saved_hold / 8, 15)` frames to the chosen On-time tolerance. A drill step is graded at its
deadline: the latest reference event's offset other than an unreacted release, plus the tolerance. A reacted release
with a saved hold counts from
the frame the attempt pressed its button (`Combo::owner_press`, `Combo::nth_press`): its offset there is that press
plus the saved hold and its band. So a button held longer than the saved hold plus the band and the tolerance drops
the step: as a missing release, or, when another event of the step kept it open, as a release that is off. A
reference drill release with no owning press gets no mark; an unmatched attempt event is extra. For hit combos a
release with no counterpart at the hit is not graded. `Tracker::late` reports the originating press of the action
whose status changed, before any new action opens. Its release becomes `LetGo` if that change occurs on the release
frame or within the selected On-time tolerance afterwards. A change with no known origin upgrades nothing, and a
status left while Player 1 could already act belongs to no action: a release that starts a walk long after its move
ended is not a reaction. An ordinary `Release` stays recorded but contributes no mark or deadline in a drill.

## One description

`ActionDesc` carries the opening move, an `of` continuation for `Other`, the side-attack angle, a variant and two
directions. `write` names the move and writes the angle and parts through `action::arrow`; the strip's
`append_moves`, combo-bar cell and recording menu use it. Diagonals are two cardinal arrows. The description holds
kinds, never names; `variant` identifies engine-common motion variants and returns zero when none is known.
The ledge, get-up and trip attack animations use `Normal` numbers 26, 27 and 28 respectively.

`LinkRec` holds both descriptions in three bytes and remains 124 bytes: each side has 12 bits, five for `of`
(0 undescribed, 1 described without `of`, 2 onward the indexed move), two for angle (0 none, 1 up, 2 down) and five
for variant. Follow-up parts come from its aim events. If both links are described, wrong-move compares move, `of`
and angle, not variant or parts; otherwise it compares legacy moves.

`Action` stores the tracker action state. At opening its origin is the newest press at most `INPUT_LOOKBACK` (15)
frames earlier; a hit uses that frame or the action start. `ids::Table::named_move` maps Catch through CatchAttack to
Grab, Throw to Throw and exactly Final to Final Smash. It maps `CLIFF_ATTACK`, `DOWN_STAND_ATTACK` and
`SLIP_STAND_ATTACK` to Ledge attack, Getup attack and Trip attack. Each opens its own action from a wait, even one
that is not actionable, when the tracked move is different. A throw changes a grab's move without opening a new
action. `Move::legacy` maps these six kinds to `Other`; the lane's colours are unchanged.

## The outcome rule

`aim::Rule` decides an aim from what the fighter did and what the stick asked for. It reads no fighter kind and holds
no table of statuses. The letters are those of REPORT-2 §2.2 and of the operator's decisions:

- **A, inside a special:** from an engine-common special start status (neutral, side, up or down) through the
  fighter's own statuses to the next common status. The start gives the special's own direction: up for an up
  special, down for a down special, either way along the ground for a side special, none for a neutral special.
- **B, onset:** an entry into an own status opens a candidate at the position of the last frame before it that was
  not in hitstop, with the velocity the fighter had then. Within `W` frames not in hitstop the fighter must be at
  least `MOVED` from that position; a status change drops the candidate. The movement is an onset when that status
  already had one in this special (a re-entry is the next part), or when its velocity is new: it differs from the old
  velocity by at least `DV`, it is not the old velocity carried on (the old speed at least `CARRIED`, the same way
  within `FOLLOW`, no faster than the old speed plus `DV`), and it turns by more than `FOLLOW` from the onset before
  it.
- **C, stick-follow:** some left-stick sample since the onset before it (or since the originating press) is off
  centre (`DEAD`) and within `FOLLOW` of the heading. The newest such sample is found by searching backwards from the
  onset frame. Ground clause: when the fighter stood on the ground before the entry and the heading is level within
  `FOLLOW`, a stick pointing downward on the same horizontal side also follows it.
- **D, not the special's own direction:** the heading is more than `FOLLOW` from the direction rule A gave. A
  straight-up up special has no aim.
- **E, part number:** the onset's ordinal in the special. Every onset counts, also one that rule C or D refuses;
  only the first two can be parts. An accepted part is reported once, in the nearest of eight screen directions.

| Constant | Value |
| --- | --- |
| `MOVED` | 1 game unit |
| `W` | 3 non-hitstop frames |
| `DV` | 1 unit/frame |
| `FOLLOW` | 33.75 degrees |
| `DEAD` | 24 raw stick units |
| `CARRIED` | 0.3 unit/frame |

Hitstop does not consume the movement window or update the previous positions. The strip places an emitted mark at
the candidate's entry frame; `Tracker::aim_pending` exposes a candidate whose movement is still undecided. The rule
does not read fighter kind or use a fighter-specific status-number table: it does compare status identities to detect
entry and re-entry. `src/model/aim_f13_fixture.rs`, generated by `tools/gen_aim_f13_fixture.py` from trace boot
`f13-trace-0a17-1`, has Pikachu's 14 casts, whose parts the rule reproduces. Other fighters have no dense traces here,
and this change has not run in a game.

## The angle from the animation

`Tracker::late` reports an action opening and a changed side-tilt or side-smash angle, with its originating press when
known. `InputCapture::step_late` rewrites the matching C-stick event, or Attack press if there was no C-stick event,
using `action::angled`; forward or back is retained and the animation supplies up, level or down. An undecided
direction can wait up to `INPUT_LOOKBACK` frames. `Helper::regrade` updates an already graded step while its attempt
is open and resolves a newly unclean attempt.

## Travel parts and the stick toggle

`action::Travel` is what one travel part measured: the deciding stick (angle in tenths of a degree, deflection in
percent), the heading and straight-line distance of the whole part (tenths of a degree, tenths of a unit) and eight
path points in game units from where the part began. Everything is facing-relative: 0° is forward, 90° is up.

- **Measured by the rule.** When `aim::Rule` reports an onset as a part it opens that part: its base position, the
  special's facing and the deciding stick, which is the newest stick sample since the onset before it (or the press)
  that points along the heading (`aim::follows`, `action::stick_of`). It keeps up to 16 positions; when they are full
  it keeps every second one and samples half as often. The part ends on the first frame whose status is not the
  part's. `Rule::ended` and `Tracker::late().travel` hold the part number and its `Travel` on that frame only.
- **Path.** The line from the base through the kept positions to the last one, resampled at eighths of its length,
  mirrored for a left facing, rounded and saturated to ±127 units.
- **Kept with the drill.** `InputCapture::step_with` attaches the travel to the newest `Aim` event of that part
  (`Combo::attach_travel`). A combo has room for `DETAILS` (4) travels, 24 bytes each; a fifth is dropped. The join on
  load and a crop move the entries with their events.
- **Graded in input drills.** When the reference's and the attempt's aim both carry a travel and their eight-way
  directions agree, `action::path_off` fails the step with `Mark::Path` if any of the eight points is farther apart
  than `max(PATH_ABS, PATH_REL × the reference path's length)` (6 units, 0.30). With *Match angle and deflection* On,
  `action::stick_off` then fails it with `Mark::Angle` when the stick's angle is more than `ANGLE_TOL` (10°) off, else
  with `Mark::Deflection` when its deflection is more than `DEFLECTION_TOL` (15 points) off. The reasons are
  `AimPath`, `AimAngle` and `AimDeflection` ("S1 1st path off", "S1 1st angle 12 off", "S1 1st deflection 30% low"),
  reported after an extra aim and before a timing fault. The angle mark and reason use the same tenth of a degree
  that was graded: a 10.2-degree miss shows `10.2 off`, not `10 off`.
- **Late travel.** A part can end after its step was graded; `Helper::regrade` grades that step again, as for a late
  angle.
- **The toggle.** Recording › Options, *Match angle and deflection* (`combo_stick`, Off by default), read into
  `combo::Settings::match_stick`. A step keeps the grade of the setting in force when it was graded or graded again;
  flipping the toggle during a drill does not grade its earlier steps again.

## A special's own travel

Rule D gives a special that goes its own way no aim event, so a steered special (an up special bent a little to one
side) left nothing to grade. The rule now also measures the special's own travel, as part 0.

- **What it is.** Player 1's positions from the frame before the special's start status until the special ends, a
  new special starts or its first aim part opens, in whatever status the fighter moves. While an entry into an own
  status may still turn out to be an aim part, it does not sample. It is kept only if some position is at least
  `MOVED` from where the special began; a special that stands still keeps nothing.
- **The stick kept with it** is the one held for the most frames of that travel, by eight-way direction, as its
  newest sample; neutral when no direction was held longest (a tie goes to a direction).
- **Entry velocity** is the displacement between the two unstopped positions immediately *before* the special
  starts, with X relative to the fighter's facing. The preceding position supplies the magnitude of the change
  between the last two velocities. Both velocity and change are stored to 1/256 of a unit. A missing third position
  gives no timing allowance; a missing velocity is unknown, not zero. Aim parts have no entry.
- **Handed over** as `Late::travel` with part 0 and `Late::press`, the press that began the special, on the frame the
  travel ends. The capture attaches it to that press's event (a Special press or a C-stick event), in the same
  four-entry table. When the table is full an aim's travel takes the place of the last one kept on a press; a press's
  travel never takes an aim's.
- **Graded in input drills,** when the reference's and the attempt's press both carry one: the path as for an aim
  part (`Mark::Path`, "S1 Special path off"). Then, only when some path point is more than `STEER_FLOOR` (2 units)
  apart **and** both entries are comparable, grade the stick's eight-way direction (`Mark::Steer`, "S1 Special
  steer wrong"), and with *Match angle and
  deflection* On its angle and deflection ("S1 Special angle 12 off", "S1 Special deflection 30% low"). Inside the
  floor or with incomparable or unknown entry, the stick is not compared. A steered special at incomparable entry speed
  gets no steering, angle or deflection fault; a path outside the independent wide-path tolerance still says path off.
- Entries compare up to an inclusive vector distance of 1/16 unit/frame plus the larger observed entry change
  times the selected On-time tolerance in frames. Either entry can be the later one in an accelerating fall. A
  zero-frame tolerance or two constant-speed approaches keep the original 1/16 threshold.
- The optional JSON pair `entry_x`, `entry_y` uses game units/frame. An old press path without them retains the path
  comparison but does not grade the stick. `entry_change` is absent when zero. One axis without the other, change
  without the pair, bad types, or unrepresentable values are refused.
- The last step of a drill waits for the special to end when the drill keeps any travel, as it does for an aim.

## The runtime reads

On sampled Player 1 frames, `input_display::p1_motion` reads the animation for a recognised attack or named move
behind `ANGLE_ON`, set by the CPU mash unit's guarded row. `p1_position` reads x and y behind `AIM_ON`, set by the
Recording unit's guarded position rows. The capability table names these boundaries in `tests/capabilities.rs`.
No hook, native site, lock or static was added for the ACTION model.

## Files

On input-drill load, a step beginning with a release or `Aim(1)` joins its predecessor if their events fit in 12;
the original file is not rewritten by loading. Optional event keys include `hold` and `reacted` for releases and
`stick`, `heading`, `distance` and `path` for aim travel. A release may write unsigned `hold`, checked for type on reading.
This build records reactions using the selected On-time tolerance and writes `reacted: true` for an input-drill
`LetGo` and `reacted: false` for an ordinary release; a non-boolean is rejected. For a legacy release without
`reacted`, an owned hold under eight frames stays ungraded,
while a hold of eight or more is graded in its band. An unowned release remains ungraded. Re-recording a drill gives
it the causal rule.
Described hit links write both optional objects `from_shown` and `to_shown`: applicable members are `of` (a
move key), `angle` (`up` or `down`) and `variant` (0–31), and `move` for a Final Smash only. Both objects are
needed for a described link; `{}` is valid. `from` and `to` write `grab`, `throw`, `ledge_attack`,
`getup_attack` and `trip_attack` by name (the operator's order of 2026-11-25, which reverses that day's rule
that the six kinds belong in the description, for these five). It is not backward compatible: install 4
(`a945b0c1`) and earlier builds refuse a file with one of the five in `from` or `to`. A Final Smash still
writes `other` with `move`, and the reader refuses `final_smash` straight in `from` or `to`. The reader still
takes the older form of the five (`other` with `move`) and reads it to the same link; a file read from disk
keeps its own JSON, and a crop or a new record writes the names. A named `from` or `to` with a `move` member is
refused. Without both objects a link is compared by its endpoints: `other` stands for any move older files
call other, a named endpoint for that move only.

An input drill's aim event that has a travel also writes `stick` (`{"angle": 135.0, "deflection": 100}`, only when a
stick was kept), `heading`, `distance` and `path` (eight `[dx, dy]` pairs). On reading, an aim event with `path` gets
a travel: `path` must be eight pairs of integers from −127 to 127, `heading` a number from 0 to under 360,
`distance` a number of 0 or more, and `stick`, when present, an angle in the same range and a deflection from 0 to
100. Without `path` the other three keys are ignored. No schema version changes. A Special press or a C-stick event
that keeps the special's own travel writes and reads the same four keys; such a drill stays version 2.

Versions 1 and 3 are hit combos, 2 and 4 are input drills; 3 and 4 allow aim events. The writer selects 1 or 2
when the combo has no aim, 3 or 4 when it has one. A source-backed write retains its original fields but writes the
selected version and name.

## Changes against the build before it

- A special's own direction does not produce an aim mark or event.
- An own status that never moves, or whose movement the stick never followed, produces no aim.
- A later travel is an aim only if the stick pointed along it after the onset before it and it is off the special's
  own direction; before, every entry into the travel status that moved was a part.
- In a new take, stick flicks inside any fighter's special are not events; before, only inside the listed fighters'
  specials. An attempt against a drill that has no aim events still counts them, as that drill was saved.
- A fighter-own status continuing a special is named by that special, with or without travel.
- Described links distinguish an angled side tilt or smash from a level one.

## Limits

- At most two onset parts per special are emitted.
- The re-entry shortcut of rule B remembers two statuses per special.
- The stick search of rule C reaches back no further than the tracker keeps frames (`SNAP_RING`, 96).
- Hit-combo aim events retain eight-way direction grading.
- A saved link stores no travel arrows. The bar and the menu show the follow-up's from its aim events. A link after
  the first shows its first move's from the link before it, whose follow-up is the same action (`Combo::descs`). The
  combo's first link shows only the base special and the angle for its first move, where the live strip also shows
  its travel arrows.
- A drill keeps four travels in all, aim parts before the travels kept on presses; hit-combo links keep none.
- A special's own travel is told apart from a steered one only beyond `STEER_FLOOR`; path points are whole units. The
  floor and what counts as steered were set from one fighter's sampled casts (Mario's up special) and not flown.
- A different approach can move the path outside the wide-path tolerance even when the stick is not comparable.
  The result then reports the path, not wrong steering.
- The entry tolerance was checked against one fighter's traced fall (Pikachu, 44 samples) and not flown. An attempt
  whose entry does not compare has its stick left ungraded, so it can pass without the recorded steer.
- Every stored combo carries the 120-byte travel table, also where no travel can occur (four 30-byte details).
- The numeric tolerances are defaults to tune in hand tests and a roster sweep.
- A recording take spanning a Training reset is unchanged.
- An origin identifies a press frame, not a button: simultaneous presses share that frame and cannot be distinguished
  by the reaction rule. Hit-combo link grading remains unchanged.
- A result under a newer final result is final too, even if its own opening buttons remain held.
- Older drill files lack reaction evidence, so their hold-length compatibility rule cannot establish causality.

## Host checks

- Ownership and clear: `a_cast_after_a_training_reset_is_graded`,
  `a_specials_first_aim_stays_in_its_step_however_late`, `a_second_aim_part_keeps_its_own_step`,
  `a_held_buttons_release_stays_in_the_step_of_its_press`, `buttons_held_through_a_clear_are_not_presses`,
  `a_release_without_its_press_is_not_an_event`, `a_button_held_when_a_take_starts_is_a_press`,
  `a_clear_mid_attempt_is_cleared_and_the_next_cast_is_graded`, `unrelated_inputs_still_split_on_idle`.
- Grading and files: `a_release_has_no_direction_to_grade`, `a_special_that_owns_aims_is_graded_by_its_aims_not_its_stick`,
  `a_hold_is_graded_on_its_length_within_a_band`, `a_short_hold_keeps_the_on_time_tolerance`,
  `a_press_inside_the_tolerance_keeps_its_whole_hold_band`,
  `a_repeated_button_uses_its_matching_press_for_the_hold_deadline`,
  `a_reference_release_without_its_press_is_not_graded`, `a_split_step_file_reads_as_the_new_recorder_saves_it`,
  `a_joined_file_is_written_back_unchanged`, `a_release_records_its_hold_in_the_file`.
- Smoke regressions: `a_tap_is_not_graded_on_its_length`, `a_charge_release_is_graded_on_its_hold`,
  `a_held_special_that_ignores_its_release_is_not_graded`, `a_status_change_outside_the_tolerance_is_no_reaction`,
  `an_older_files_tap_is_not_graded_and_its_long_hold_is`, `a_new_file_says_whether_the_game_reacted`,
  `a_reacted_release_round_trips`, `a_reset_chord_alone_leaves_no_result`,
  `a_reset_before_the_deadline_leaves_no_result`, `a_real_attempt_then_a_reset_records_as_today`,
  `a_failed_attempt_is_not_taken_back_by_a_later_reset`, `only_the_owning_press_reacts_to_its_actions_change`,
  `a_press_does_not_react_to_the_opening_of_its_own_action`, `a_held_moves_end_is_a_reaction_for_its_own_press`,
  `a_change_after_the_action_ended_is_no_reaction`, `a_recording_uses_the_selected_tolerance`,
  `the_recording_host_passes_its_selected_reaction_window`,
  `a_staggered_chord_takes_back_both_results`, `a_released_press_makes_its_result_final`,
  `a_load_after_waits_for_a_provisional_result`, `a_hit_combo_load_is_not_deferred`,
  `two_provisional_results_restore_a_distinct_full_history_in_order`,
  `a_release_finalizes_a_result_after_capture_overflows`, `a_new_final_result_keeps_an_older_deferred_load`.
- Description and angle: `the_bar_and_the_menu_name_a_link_as_the_strip_does`,
  `a_hit_combo_tells_an_angled_tilt_from_a_level_one`, `a_link_without_descriptions_grades_as_before`,
  `link_descriptions_round_trip_in_optional_keys`, `a_link_record_keeps_its_size`,
  `the_description_holds_no_names`, `a_side_tilts_drill_direction_comes_from_its_animation`,
  `the_strip_mark_and_the_drill_direction_agree`, `a_buffered_tilt_waits_for_its_animation_before_it_is_graded`,
  `kirbys_angled_tilt_still_saves_as_version_two`, `the_tracker_reports_an_action_when_it_opens`,
  `the_action_takes_its_origin_when_it_opens`, `a_throw_stays_in_the_action_of_its_grab`,
  `the_status_predicates_use_the_resolved_constants`,
  `a_link_keeps_both_moves_directions_and_the_special_a_travel_continues`.
- Outcome: `the_outcome_rule_gives_f13s_pikachu_parts`, `an_aim_in_the_specials_own_direction_is_no_event`,
  `a_carried_or_drifting_status_is_no_onset`, `an_onset_the_stick_never_pointed_at_is_no_aim`,
  `a_ground_travel_follows_the_sticks_side`, `a_reentered_status_is_the_next_part_and_a_third_is_none`,
  `a_delayed_aim_uses_the_stick_since_the_press`, `hitstop_frames_do_not_use_up_the_onset_window`,
  `the_design_tolerances_are_the_stated_defaults`, `stick_flicks_inside_any_special_are_not_events`,
  `any_fighters_special_can_aim`, `no_fighter_table_is_left`,
  `the_runtime_reads_the_angle_and_the_travel_behind_their_gates`.
- Travel and the stick toggle: `a_travel_part_records_its_stick_heading_distance_and_path`,
  `the_deciding_stick_is_the_last_one_that_pointed_along_the_heading`,
  `a_path_is_eight_points_by_arc_length_facing_relative`, `the_tracker_hands_over_a_travel_when_its_part_ends`,
  `a_steered_part_is_graded_on_its_path_within_the_tolerance`, `a_short_warp_fails_a_long_warp_drill`,
  `the_angle_and_deflection_toggle_grades_the_stick`, `the_toggle_row_reads_its_setting_and_is_off_by_default`,
  `a_travel_that_ends_after_its_step_was_graded_amends_the_grade`, `aim_details_round_trip_in_optional_keys`,
  `a_drill_keeps_details_for_its_first_four_aims`, `the_travel_table_is_packed_into_120_bytes_a_combo`,
  `a_later_links_first_move_borrows_its_travel_arrows`, `a_failed_stick_angle_shows_the_difference_that_was_graded`,
  `a_travel_that_ends_the_frame_after_its_step_was_graded_amends_the_grade`,
  `the_stick_toggle_flipped_mid_drill_applies_from_the_next_grade`.
- A special's own travel: `a_specials_own_travel_is_measured_from_its_start_to_its_end`,
  `a_special_that_does_not_move_has_no_own_travel`, `the_own_travel_ends_where_the_first_aim_part_begins`,
  `the_stick_most_held_is_kept_with_the_own_travel`, `the_tracker_hands_over_a_specials_own_travel_with_its_press`,
  `a_steered_special_must_be_steered`, `a_special_that_ignores_the_stick_cannot_fault`,
  `a_wide_steer_fails_on_its_path`, `carried_momentum_inside_the_floor_is_not_a_steer_fault`,
  `a_specials_own_travel_is_kept_on_its_press`, `the_stick_toggle_also_grades_a_steered_specials_stick`,
  `a_press_keeps_its_own_travel_in_optional_keys`, `an_aims_detail_is_not_displaced_by_a_press_path`,
  `a_steered_specials_faults_reach_both_card_panes`.
- Entry velocity: `entry_motion_does_not_turn_an_airborne_fixed_kick_into_a_steer`,
  `entry_motion_on_a_steer_must_match_to_judge_the_stick`,
  `entry_motion_round_trips_and_old_press_paths_do_not_grade_the_stick`,
  `entry_motion_one_frame_apart_compares_on_all_44_pikachu_fall_samples`.
- The new reasons and marks of both are also drawn by
  `every_reason_the_grader_can_give_is_shown_in_full_at_the_cards_width` and
  `every_mark_under_a_step_is_shown_in_full`.
- Edge attacks: `the_edge_attack_statuses_are_the_games`, `an_edge_attack_opens_its_own_action_at_its_press`,
  `an_edge_attack_keeps_its_move_one_frame_past_its_end`, `an_edge_attack_links_on_both_sides_and_reads_alike`,
  `the_grader_tells_an_edge_attack_from_another_move`, `edge_attacks_round_trip_in_files_and_in_the_packed_link`,
  `an_old_file_grades_an_edge_attack_as_before`, `an_edge_attack_step_is_decided_when_its_action_opens`,
  `an_edge_attack_takes_its_name_number_from_its_animation`,
  `a_new_link_names_a_grab_a_throw_and_the_edge_attacks_and_round_trips`,
  `an_old_file_with_other_and_a_description_reads_as_before`, `a_file_mixing_old_and_new_links_reads_both`,
  `the_combo_and_drill_views_name_them_the_same`, `a_final_smash_is_still_refused_straight_in_from_or_to`,
  `a_named_endpoint_takes_no_move_in_its_description`, `a_named_endpoint_without_descriptions_grades_by_its_name`,
  `a_crop_of_an_old_file_writes_the_names`, `the_format_docs_say_which_builds_read_a_named_link`.
