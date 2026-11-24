# The ACTION model (design, 2026-11-24)

## What it is

An action is an originating input and the status chain it starts. It owns its release (and so its hold), its angle
and its aims. A step starts only at an originating input, and the grader compares actions: what was pressed, how long
it was held, which angle the game chose and where the special went. One description is read by the strip, the bar,
the menu and the grader. The design and requirement trace are `artifacts/phase23/action-model/DESIGN.md` (base
`e7667f26`); research is in `artifacts/phase23/research/general-input-recording/REPORT.md` and `REPORT-2.md`.

Travel detail and the angle-and-deflection toggle in DESIGN §8–§9 are a separate change and are not described here.

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

`Tracker::clear` retains `buttons` and `stick`, clearing its other state. `Helper::on_clear` closes an open drill
attempt with a graded step as cleared, but discards an open drill attempt with no graded step without recording it;
both start fresh. Hit-combo clearing remains as before.

## Grading

`grade_with` matches events in order and uses `GradeContext` for hold and aim ownership.

| Event | Compared |
| --- | --- |
| Press, C-stick, flick | Kind, direction and offset; an old presses-only link matches button and its four-way aim. |
| Special press owning aims | Kind and offset, not its stick direction; the aim events carry the direction. |
| Release | Same button; direction is ignored. If both holds have owners, compare hold lengths; otherwise compare offsets where applicable. |
| Aim | Part and facing-relative eight-way direction; for an aim-bearing target its timing delta is zero. |

`Grade.band` adds `min(saved_hold / 8, 15)` frames to the chosen On-time tolerance. A drill step is graded at its
deadline: the latest of its reference events' offsets, plus the tolerance. A release with a saved hold counts from
the frame the attempt pressed its button (`Combo::owner_press`, `Combo::nth_press`): its offset there is that press
plus the saved hold and its band. So a button held longer than the saved hold plus the band and the tolerance drops
the step: as a missing release, or, when another event of the step kept it open, as a release that is off. A
reference drill release with no owning press gets no mark; an unmatched attempt event is extra. For hit combos a
release with no counterpart at the hit is not graded.

## One description

`ActionDesc` carries the opening move, an `of` continuation for `Other`, the side-attack angle, a variant and two
directions. `write` names the move and writes the angle and parts through `action::arrow`; the strip's
`append_moves`, combo-bar cell and recording menu use it. Diagonals are two cardinal arrows. The description holds
kinds, never names; `variant` returns zero, leaving move variants for the move-name lane.

`LinkRec` holds both descriptions in three bytes and remains 124 bytes: each side has 12 bits, five for `of`
(0 undescribed, 1 described without `of`, 2 onward the indexed move), two for angle (0 none, 1 up, 2 down) and five
for variant. Follow-up parts come from its aim events. If both links are described, wrong-move compares move, `of`
and angle, not variant or parts; otherwise it compares legacy moves.

`Action` stores the tracker action state. At opening its origin is the newest press at most `INPUT_LOOKBACK` (15)
frames earlier; a hit uses that frame or the action start. `ids::Table::named_move` maps Catch through CatchAttack to
Grab, Throw to Throw and exactly Final to Final Smash. A throw changes a grab's move without opening a new action.
`Move::legacy` maps those three kinds to `Other`.

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

## The runtime reads

On sampled Player 1 frames, `input_display::p1_motion` reads the animation for a recognised attack or named move
behind `ANGLE_ON`, set by the CPU mash unit's guarded row. `p1_position` reads x and y behind `AIM_ON`, set by the
Recording unit's guarded position rows. The capability table names these boundaries in `tests/capabilities.rs`.
No hook, native site, lock or static was added for the ACTION model.

## Files

On input-drill load, a step beginning with a release or `Aim(1)` joins its predecessor if their events fit in 12;
the original file is not rewritten by loading. A release may write unsigned `hold`, checked for type on reading.
Described hit links write both optional objects `from_shown` and `to_shown`: applicable members are `move` (the three
new kinds), `of` (a move key), `angle` (`up` or `down`) and `variant` (0–31). Both objects are needed for a described
link; `{}` is valid. `from` and `to` write `other` for Grab, Throw and Final Smash for older readers.

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
- A saved link stores no travel arrows. The bar and the menu show the follow-up's from its aim events; for the first
  move they show the base special and the angle only, where the live strip also shows its travel arrows.
- The numeric tolerances are defaults to tune in hand tests and a roster sweep.
- A recording take spanning a Training reset is unchanged.

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
