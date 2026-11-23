# The speed label's button and the recording indicator (design, 2026-11-23)

## 1. F5: the speed label reads `1 Frame (Press )`

### Cause (confirmed in the owned 13.0.5 main ELF and on the card build in Eden)

| Fact | Where |
| --- | --- |
| The bind writes the label with the game's own text function, `0x1bbbd30(menu, value, pane)`, as the menu setter does | `runtime/features/game_speed.rs` `training_tick`; `docs/design/rc8-binds.md` §2 |
| For value 2 (1 Frame) that function reads the controller glyph style from the Training pause menu's `+0x30`: `ldr w8,[x22,#0x30]` | `.1bbbd78` (jump table `0x44f8a24` entry 0) |
| It derives the button code from it (`0xc`, or `[main+0x44681a0 + 4*(style-3)]` for styles 3..6), stores the style into each text object's `+0x25c`, and sets `mel_training_speed1` (the menu row) and `mel_training_speed_control_frame` (the stopwatch) with the code as the message's one argument: `0x377a280(pane, label, 1, code)` | `.1bbbd78`–`.1bbbd94`, `.1bbbe34`–`.1bbbf78` |
| The game writes `menu+0x30` when its own pause menu opens: the HUD's open step maps the opening controller's style `[[c+0x10]+0x9c]` (1..7) through the table `main+0x4753de0` = `1,2,3,4,5,6,6`, calls `0x1bb8b20(menu, index)` (which stores it too, `.1bb8db4`) and stores it again | `.13e4e08`–`.13e4e88` |
| The menu's constructor leaves 0 there. The menu's code (`0x1bb0000`–`0x1bc8000`) has no other 32-bit store to `+0x30` | `.1bb16b0` `str wzr,[x19,#0x30]`; `re/peek.py find-store` |
| The other speeds' labels take no argument (`0x377a120(pane, label)`), so only 1 Frame shows it | `.1bbbda8`–`.1bbbe30` |

So with the game's pause menu never opened in the Training session, the bind's call draws the button with glyph style 0, which is no glyph. The game never meets this: its own setter runs only with the menu open.

**Eden, the card build, one boot (`rc15-docs-17060a5e-yoga-2`, eggs-offstage-sol):**
- Pause menu never opened in the session, chord to 1 Frame: `1 Frame (Press )`. Still: `smash-training-lobby` `artifacts/phase23/fix-round-20261123/f5-f3/repro/repro-r5/1-frame-top-right-1080p.png`.
- One open and close of the game's pause menu, then chord to x1 and back to 1 Frame: `1 Frame (Press ZL)` with the glyph. Still: `smash-training-lobby` `artifacts/phase23/fix-round-20261123/f5-f3/repro/repro-r5-native-pause/1-frame-top-right-1080p.png`.

### Fix

The bind lends the text call Player 1's glyph style, the value the menu's open step would have stored for the controller that set the speed:

1. Player 1's poll already reads the controller style (`chords::field::STYLE`, the same `+0x9c`). `game_speed::on_hold` keeps it in an atomic.
2. `model::game_speed::glyph_style(style)` is the game's table (1..=6 as they are, 7 the GameCube pad 6, anything else 0).
3. `model::game_speed::label_lend(value, style, rows)` gives `Some(glyph style)` only for 1 Frame, a style with a glyph style, and the label's rows passed at boot.
4. `training_tick` (the UI job thread, where the menu's own code runs): read `menu+0x30`, write the lent style, call `0x1bbbd30`, write the old value back. The field stays the game's: after the call it holds what the game put there (0, or the last opener's style), and the game's next open overwrites it as always. The two text objects keep the style, as they do after the menu's own setter.
5. Without a lend the call is made as before.

Not changed: the stored speed, the stopwatch show and hide, the apply, the pop-up, the waiting room (it has no stopwatch).

### Guards

A new data file, the 31st: `data/tables/rows_speed_label.ROWS.tsv`, written by `tools/gen_rows_speed_label.py`, group `speed_label`, all `Relied`, read-only, not per frame. The historical digests stay unchanged.

| Row | Offset | What it holds |
| --- | --- | --- |
| `speed_label.frame_text` | `0x1bbbd78` | value 2's case: `ldr w8,[x22,#0x30]`, the button code from the style |
| `speed_label.frame_set` | `0x1bbbeb4` | `mel_training_speed1` set with the code (`bl 0x377a280`), the second read of `menu+0x30`, `str w8,[x21,#0x25c]` |
| `speed_label.style_source` | `0x13e4e0c` | the open step: controller `[main+0x5339860 + 8*index]`, its style `[[c+0x10]+0x9c]` (`ldr x8,[x8,#0x10]; ldr w8,[x8,#0x9c]`) |
| `speed_label.style_store` | `0x13e4e4c` | the table `main+0x4753de0`, `bl 0x1bb8b20`, `str w22,[x21,#0x30]` |
| `speed_label.glyph_styles` | `0x4753de0` | the table `1,2,3,4,5,6,6` |

The gate `LABEL_ROWS` is its own: when a row fails, nothing is lent and the label is written as before. The bind and 1 Frame do not depend on it. No hook, no called function added. Card scan (2026-11-23, the playbook's `scan-offsets.py`): none of the card's other 20 plugins references the five offsets or the three functions around them.

## 2. F3: the recording indicator

### Today (card build, Eden `rc15-docs-17060a5e-yoga-2`, `repro/repro-r3-*`)

- A take for the CPU: the CPU's fighter is tinted by phase (lockout blue, standby magenta, recording red) and a one-second notice names the phase.
- A take for Player 1: nothing on screen.
- The take's role is one atomic, `rr_host::CAP_ROLE` (0 none, 1 Player 1, 2 CPU), set at the Record chord that starts it and cleared at the one that stops it. The bridge already publishes it (`recording.capture.role`). `rr_host::capture_take()` hands it to the HUD as `hud_row::Take`.

### Design: the compact HUD row

The top-left compact row that Break the Targets uses (`TrBtt`: a dark plate, a 32-unit picture chip, one text) shows the take:

| State | Text | Text and chip colour | Box |
| --- | --- | --- | --- |
| Player 1's take | `REC P1` | the roster's Player red `[255, 83, 92]` | 140 wide, at the row's corner |
| The CPU's take | `REC CPU` | the roster's CPU blue `[75, 164, 255]` | the same |
| Break the Targets' line, no take | as today | text colour as today, chip white | as today (`btt_hud_box`) |
| A take during Break the Targets | `REC P1 · ` or `REC CPU · ` and then the line | the role's colour | `btt_hud_box` of the whole text |
| Neither | hidden | | |

- **Where:** 16 px from the left and 8 px from the top at 1280 × 720, the corner the notices use; in the waiting room it moves down with the notices and the strip, under the native panels. A notice stacks under the row, as it does under the Break the Targets line.
- **The dot:** the chip is the row's own picture (the target), tinted by its vertex colours like the orbs and eggs. Red tint: an almost solid red disc. Blue tint: a blue disc with dark rings.
- **When:** from the Record chord that opens the take to the one that closes it, for every kind of take that goes through `CAP_ROLE` (Record, Record drill). Hidden while the plugin's menu covers the HUD, like the rest of it.
- **Independent of the notice queue:** Combo Display Off, a refusal or another notice does not remove it.
- **No layout change:** no pane, texture or material is added or changed; the pane recipe guard passes unchanged.
- **Colours:** the two role colours move from `runtime/ui/view.rs` into the model, and the roster grid and the row use the same constants.
- **Bridge:** one line added to `tr.menu/1`, `recording.indicator` = `player`, `cpu` or empty: the role of the row the HUD drew on its last tick. Empty while the row is hidden, and whenever the practice scene is not fresh (the HUD is not being drawn), so a take left open across a scene change does not read as shown.

### Design: the recorded fighter's tint

While its take records, the recorded fighter has its role's hue, the scheme the chip uses: Player 1 red, the CPU blue. The CPU's two phases before its take records keep hues of their own.

| Take | Phase | Fighter | Card build | Now |
| --- | --- | --- | --- | --- |
| Player 1's | open (it has no lockout or standby) | Player 1's | no tint | red `[1, 0, 0]` |
| The CPU's | Lockout (30 frames) | the CPU's | blue | **yellow `[1, 1, 0]`** |
| The CPU's | Standby (until the first input) | the CPU's | magenta | magenta `[1, 0, 1]`, unchanged |
| The CPU's | Recording | the CPU's | red | blue `[0, 0, 1]` |

- **The values** are the pure hues the phases already had: the card build's recording red goes to Player 1 and its lockout blue to the CPU's recording. Lockout takes yellow, a hue none of the others has (red, blue, magenta).
- **The phase notices** keep their tint's hue, as before: `Lockout` light yellow `[255, 224, 112]` (15.2:1 on the notice plate), `Standby` unchanged, `Recording` light blue `[148, 176, 255]` (the value Lockout had, 9.4:1). Their text, length and timing are unchanged.
- **The CPU's tint** is written where it was (the AI hook, every frame of the phase); only `Phase::tint`'s values change.
- **Player 1's tint** is written in Player 1's own fighter frame in a practice scene (`cpu::fighter_frame`, entry 0; Training and the waiting room) by `recorder::tint_player`: on every frame `rr_host::player_tint()` gives the tint, to entry 0's active body (never Nana, as the CPU's). It does not depend on the CPU's AI frame, which a drill, a placement or Mash Out skips, so Record drill is tinted too.
- **Nothing new native:** the same `ModelModule::set_color_rgb` (`model_set_color_rgb`, package 4) and colour-blend constant the CPU's tint uses; no hook, guard row or called function added.
- **Chip and tint together:** the chip names the role for the whole take; during the CPU's take its fighter is yellow, magenta, then blue.

Not changed: the phases, their notices' text and timing, the standby tint, the recording itself.

### Unproven until the fix build's boot

- How the tinted chip and the 13-unit text read on a real screen, on a bright and a dark stage.
- The waiting room's row (same code path; Eden only).
- Player 1's tint on a running game: the write is the CPU's, made at a new place (Player 1's fighter frame).

## 3. Tests (host)

F5:
- `the_one_frame_label_is_lent_player_ones_glyph_style` (`game_speed_tests`): a table of value, controller style and rows against the lend.
- `the_glyph_styles_are_the_games_table`, `the_frame_label_reads_the_menus_glyph_style`, `the_frame_label_sets_both_texts_with_the_button_code`, `the_menus_open_step_stores_the_glyph_style_of_the_controller`, `every_label_row_is_gated` (`speed_label_rows_tests`): each constant the runtime uses decoded from the guard words.
- `the_training_write_lends_the_glyph_style_around_the_text_call` (source test over `runtime/features/game_speed.rs`): the lend is written before the text call and the old value after it, and `on_hold` keeps the style. Red on `17060a5e`.

F3:
- `a_take_shows_rec_in_the_roles_colour`, `no_take_and_no_line_hides_the_row`, `the_targets_line_alone_is_drawn_as_before`, `a_take_during_the_targets_run_shares_the_line`, `the_rec_row_is_short_and_holds_its_text`, `a_notice_stacks_under_the_rec_row`, `the_rec_row_keeps_clear_of_the_lobbys_panels` (`hud_row_tests`).
- `the_hud_draw_shows_the_open_take_in_training_and_the_waiting_room` (source test over `runtime/features/hud_draw.rs` and `runtime/ui/view.rs`): the draw reads the open take (`rr_host::capture_take`), builds the row for both layouts, writes text colour and chip colour, hides it under the menu, and the roster uses the model's role colours. Red on `17060a5e`.
- `the_open_take_is_the_capture_role` (`rr_host` tests): no take, Player 1's take and the CPU's take through the Record chord, and the bridge word equal to `recording.capture.role`'s.
- `the_snapshot_says_which_take_the_hud_shows`, `the_indicator_is_empty_without_a_fresh_practice_scene` (`bridge_state` tests): the new line for player, cpu and none, and empty without a scene.
- `phase_notifications` (`tests/input01.rs`, its pins changed), `a_recorded_fighter_has_its_roles_tint` (the same file): the four tints, each a different hue, the CPU's recording tint the CPU's, and each phase notice in its tint's hue. Red on `17060a5e`.
- `player_one_is_tinted_only_during_its_own_take` (`rr_host` tests): no take, Player 1's take, its stop and the CPU's take through the Record chord.
- `the_runtime_tints_each_recorded_fighter` (source test over `runtime/features/recorder.rs` and `runtime/features/cpu.rs`): the CPU's phase tint and Player 1's take tint go through one writer, Player 1's is written to entry 0's active body from Player 1's frame in a practice scene. Red on `17060a5e`.
- Unchanged and passing: `tests/pane_recipe_guard.rs`, the table digests (plus the 31st file's count), the Break the Targets and notice box tests.
