# Recording menu and wizard visibility

The [Recording menu](../menu/recording.md) includes these four changes from the 2026-11-22 design.

## The mixed page

| State | Shipped behavior |
| --- | --- |
| Pop-up over a kept screen | The pop-up owns drawing and jump input. |
| Next menu opening | The kept sub-screen returns. |
| Original defect | The sub-screen drew over the pop-up while the pop-up still took input. |

Regression tests:

- `recording_popup_over_a_kept_game_screen_is_the_popup_alone`
- `recording_popup_is_the_popup_alone_over_every_kept_screen`
- `recording_popup_takes_jumps_over_a_kept_screen`
- `kept_game_screen_survives_a_popup_between_two_menu_openings`

## The shared wizard state

| State | Shipped behavior |
| --- | --- |
| Wizard opens | Its kind sets its bit with `runtime::published::WIZARDS.set_up`. |
| Any wizard up | The HUD hides its strip, card and helper bar. |
| Wizard exits | Its frame or exit path lowers its bit. |

Regression tests:

- `set_up_reports_whether_the_wizard_was_up`
- `the_strip_stays_hidden_until_the_last_wizard_leaves`
- `the_strip_asks_the_shared_wizard_state`
- `every_wizard_is_lowered_by_the_menu_and_by_its_own_frame`

## The page role tabs

| State | Shipped behavior |
| --- | --- |
| Recording page | The pop-up’s role plate lights the page role and displays a ZR cap. |
| File screen, picker or keyboard | Keep the plate and hide its cap. |
| Pop-up | ZR switches role. L/R changes tabs. |

Regression tests:

- `the_recording_page_shows_its_role_like_the_popup`
- `a_file_screen_keeps_the_role_tabs_without_the_zr_cap`
- `role_caps_reuse_the_two_cap_panes`
- `the_page_tabs_end_left_of_the_role_plate`
- `the_role_tabs_draw_the_models_caps`

## Roles on the card

| State | Shipped behavior |
| --- | --- |
| Pop-up | Saves the Record role independently of the page view. |
| Page | Saves its own Player or CPU view. |
| Settings change | The worker updates live settings and requests the existing settings writer. |
| Opening a fresh model | Read top-level `recording_roles` before synchronizing the page. |
| Older settings files | Both roles default to Player. |
| Boot | The worker adopts the saved pop-up role through `rr_host::adopt_saved_role`. |
| First Record after restart | Records on the saved role before any menu visit. |

Regression tests:

- `an_old_file_without_recording_roles_reads_player`
- `recording_roles_round_trip_and_ignore_bad_values`
- `recording_popup_reopens_on_the_role_picked_last`
- `a_role_change_is_saved_without_a_menu_close`
- `a_held_model_keeps_its_roles_at_a_menu_opening`
- `recording_page_role_is_saved_and_restores_its_view`
- `popup_and_page_roles_are_saved_separately`
- `recording_roles_reach_the_card_from_one_place`
- `a_cold_start_records_on_the_saved_popup_role`
- `the_saved_popup_role_is_adopted_at_boot`
