# Recording menu and wizard visibility

2026-11-22. Four changes to the Recording menu and the HUD.

## The mixed page

A pop-up over a kept sub-screen drew the pop-up before the sub-screen's rows, but took the buttons itself. The pop-up
now owns the whole frame and its jump inputs while open; the kept screen remains for the next menu opening.
Tests: `recording_popup_over_a_kept_game_screen_is_the_popup_alone`,
`recording_popup_is_the_popup_alone_over_every_kept_screen`, `recording_popup_takes_jumps_over_a_kept_screen`,
`kept_game_screen_survives_a_popup_between_two_menu_openings`.

## The shared wizard state

Each wizard reports entry and exit through `runtime::published::WIZARDS.set_up(model::wizard::Kind::X, up)`.
The HUD hides the strip, card and helper bar while any kind is up. Each wizard lowers its own bit on its existing
exit or frame paths. Tests: `set_up_reports_whether_the_wizard_was_up`, `the_strip_stays_hidden_until_the_last_wizard_leaves`,
`the_strip_asks_the_shared_wizard_state`, `every_wizard_is_lowered_by_the_menu_and_by_its_own_frame`.

## The page role tabs

The Recording page uses the pop-up's role plate and its own ZR cap, with the lit tab showing the page view. A file
screen, picker or keyboard keeps the plate but hides the cap; the pop-up retains L/R. Tests:
`the_recording_page_shows_its_role_like_the_popup`, `a_file_screen_keeps_the_role_tabs_without_the_zr_cap`,
`role_caps_reuse_the_two_cap_panes`, `the_page_tabs_end_left_of_the_role_plate`, `the_role_tabs_draw_the_models_caps`.

## Roles on the card

The pop-up's Record role and the page's view remain independent. The worker saves changes to live settings and requests
the existing settings writer; a new model reads the top-level `recording_roles` key before its page is synchronized.
Older files default to Player. Tests: `an_old_file_without_recording_roles_reads_player`,
`recording_roles_round_trip_and_ignore_bad_values`, `recording_popup_reopens_on_the_role_picked_last`,
`a_role_change_is_saved_without_a_menu_close`, `a_held_model_keeps_its_roles_at_a_menu_opening`,
`recording_page_role_is_saved_and_restores_its_view`, `popup_and_page_roles_are_saved_separately`,
`recording_roles_reach_the_card_from_one_place`.

At boot, `worker::remember` adopts the saved pop-up role through `rr_host::adopt_saved_role`, so the first Record chord
after a restart records on that role without a menu visit. Tests: `a_cold_start_records_on_the_saved_popup_role`,
`the_saved_popup_role_is_adopted_at_boot`.
