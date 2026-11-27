# 1 Frame on the game speed bind (design, 2026-11-21)

- **Lane:** tr-speed-frame (Devin, Opus 5.5), brief from tr-primary-2 13:20Z. Branch `rc15/speed-frame` from `5fb35633`,
  worktree `/home/intpa/tr-fixes-20261113/rc15-speed-frame`.
- **Evidence:** static reads of the owned 13.0.5 main ELF (`0789f0a4…`), with `llvm-mc` (`/home/intpa/speed-frame-re/dis.py`)
  and Ghidra (`/home/intpa/speed-frame-re/o0*.txt`); QCF's `RESEARCH-rc8-speed.md` §3, §9 and its `qcf-spd05y` census.
  Addresses are `main+offset`. "Inferred" marks what only Eden can confirm.
- **Recorded decision this reverses:** `docs/design/rc8-binds.md` §3.5 (2026-11-11): "1 Frame is left to the native
  menu". The new order (operator 13:16Z, via tr-primary-2) puts it on the bind. `docs/guide/game-speed.md` changes with it.

## 1. How native Training does 1 Frame (confirmed in the code)

| Fact | Where |
| --- | --- |
| The value is 2 at `PauseMenu+0xb54`; its apply sets no rate (code 0, 60 updates) and no slot | `.155b654`; `rates_slow` row |
| Pausing is a request bitmask `[main+0x52b92ec]`; Training owns **bit 2** (`orr #4` at `.1556c70`, cleared at `.1556b7c`, `.155a70c`, `.155b608`) | stores listed in `g-52b92e8.txt` |
| `FUN_00555da0` makes the paused word `[main+0x52b92e8]` equal the requests and runs the pause/resume side effects; it is the only writer of the paused word (`.555ef4`) | decompile `o02.txt` |
| A paused word skips the slow step (`.135cab4 cbnz w22`) and runs the scheduler paused (`.135cc10`): only the six tasks marked to run while paused, the registry walker among them | `lobby_speed.paused_flag`; census `qcf-spd05y` |
| The Training mode object's update `FUN_015569e0`, every update while `+0x194 == 2`, reads the step input; no input: set bit 2 + `FUN_00555da0` (pause); a press: clear bit 2 + `FUN_00555da0` (one update runs) | `.1556a18`–`.1556c78` |
| **The step input:** `FUN_0177e8f0(…, functor 0x509ebd0, mode)` over all 10 controllers; mode 0 = newly pressed (`just_down & current`, `.177ec20`), mode 1 = held; the functor (`0x177fe20`) returns a fixed mask by controller style from `0x45434f0`: styles 1, 2, 5, 6 **ZL** (`0x400`), 3, 4 and GameCube (7) raw **L** (`0x100`, the GameCube L trigger, this plugin's logical ZL) | `.177e8f0`, `.177fe20`, table `0x45434f0` = `400,400,100,100,400,400,100` |
| A press steps one update; held, a counter `+0x1bc` counts and from 18 updates on the game runs while it is held | `.1556abc`, `.1556b24`–`.1556b34` |

So the native step button is **ZL** (GameCube L), press = one frame, hold = run from the 18th update.

## 2. Design

1. **Scale:** `1 Frame, 1/4, 1/2, 2/3, x1, x1.5` for Training and the lobby (`TRAINING_SPEEDS`, `LOBBY_SPEEDS`); the ends
   stop. The pop-up body is `< 1 Frame >`.
2. **Training: the native 1 Frame, unchanged.** The bind writes value 2 the way it writes the others (store `+0xb54`,
   `0x1bbbd30` text, the HUD refresh, then `training_apply`'s writes: `+0x194 = 2`, the old slot freed, the step clear,
   code 0 / 60). From the next update the game's own `FUN_015569e0` pauses and steps on ZL exactly as after the pause
   menu. The stopwatch: `hud_icon` shows it for any value but x1 and `0x1bbbd30` writes value 2's label, as the menu
   does. The label's button glyph needs the menu's glyph style, which only a menu open sets: `rc15-fix-f5-f3.md` (2026-11-23).
   - **Leaving 1 Frame by the bind:** the apply already clears bit 2 (`.155b608`), but nothing then syncs the paused word
     in closed-menu play (the only syncs are `FUN_015569e0`'s and the pause/resume side's), so the game would stay paused.
     The bind does what the mode object's own run path does (`.1556b64`–`.1556b84`): if the paused word has bit 2, clear
     request bit 2 and call `FUN_00555da0`. New rows: `game_speed.pause_sync` (called) and `game_speed.step_run`.
   - **Inferred, Eden (a):** P1's poll (the input mapping, called from the scene update `FUN_0135f570` at `.1360ad4`,
     outside the scheduler) runs while paused, so the chord can step out of 1 Frame.
   - **ZL and the chords (review of `985c79c6`, tr-primary-2 2026-11-21 16:55Z and 17:38Z):** the mode object reads the
     step button from the controller itself (`.1556a78` newly pressed: `just_down & current`; `.1556ae0` held:
     `current`), after the mapping call, so the chord stage's hide for the mapping call doesn't reach it. P1's poll
     therefore takes the step button out of the controller and doesn't put it back (`lobby_speed::step_take`,
     `StepKeep`), as it does for Plus against the pause check, which is the same reader with mask `0x40000`
     (`.1558e20`).
     - **When:** while the speed hold is open, and for a chord that passed (Save State and the other ZL chords) from
       the poll it passed until its ZL is let go. Neither steps a frame, pressed or held, and ZL doesn't reach the
       fighter either. A plain ZL is never touched.
     - **What:** the bit leaves the current and both edge sets and is marked in previous, so the mode object reads no
       press and no hold. Plus keeps its current set and loses only its edges (6e226733); that isn't enough here,
       because the mode object also reads the hold (`2bd5445d` did that: a chord's ZL kept down ran Training from the
       18th update).
     - **How the poll still knows the button is down:** with the bit out of the current set, the game's refresh makes
       a press of it again every update while it is down (6e226733's trace), and none once it is up. The poll leaves the
       mark in previous: a refresh copies the cleared current there and puts the held bit back in current, while a poll
       repeated before a refresh finds both edges zeroed but current and previous as the poll left them
       (`smash-training-lobby` `artifacts/phase23/1f-save-hold/repoll-boot/`, exact 3448686c trace: 46 of 46 repeats).
       The detector gets the bit as held and not pressed: the chord stays one chord and fires once. The game's release
       check is previous AND just-released, which stays clear; the mark is no release. The mode object's press and hold
       checks see no current bit. The mark counts only without a just-released bit, so a release in a refresh the poll
       did not see stays a release (review-games-astra-3 23:51:14Z). A re-entry (a P1 poll the runtime skipped, or a
       poll on another Controller than the last) starts with no kept step bit and no chord-owned button, closes an
       open speed hold with its pop-up, and leaves pending requests. The reset runs under the poll's one lock; the
       pop-up close is the first thing acted on after the lock is let go, before any notice or event of this poll.
       A session clear on the same Controller keeps the kept bit and chord-owned buttons.
     - **When the taking ends with the button still down** (the speed chord let go first): the bit goes back into the
       current set without its edge. It is a hold (the game's count to 18), never a press.
     - **Inferred, Eden:** P1's poll comes before the mode object's read in an update, as it does for Plus on the card.
       Rows: ZL x3 and ZL held while the speed chord is held at 1 Frame (no step); D-pad Down + ZL at 1 Frame, held
       2 s (one save, no step).
3. **Lobby: a held world through the battle task entry, no global write.**
   - **Why not a slot:** the slow step clamps the target to 100 (`.135cae4`–`.135caec`) and writes the rate `1/N` every
     update (`.135cbb8`–`.135cc00`); the world keeps moving at least 1/100 frame per update, so a slot can't hold still.
   - **Why not the native pause:** bit 2 + `FUN_00555da0` changes the global paused word and runs side effects
     (`0x555f20`, the sound call `0x35a0460`, `[main+0x5335e90]` and `[main+0x52c50f0]` state) that the waiting room
     never sees natively.
   - **The hold:** one new inline hook, `lobby_speed.entry_hold` at `main+0x135ca38`, in the battle task entry
     `FUN_0135ca00` right after it loads the paused word into w22 (`.135ca30`). While the lobby is at 1 Frame, the
     practice is live, the entry is the waiting area's (`[x20]` = vtable `main+0x506d4d8`) and no step is due, the
     callback sets **x22 = 1 for this entry only**. The entry then skips its slow step and runs the scheduler paused,
     exactly as native 1 Frame does, but the paused word stays 0. It writes no game memory.
   - **What still runs:** the six run-while-paused tasks, including the registry walker; its per-object skip reads the
     global word (`FUN_01358ad0`), which stays 0, so **every registry object ticks at x1**: the Quickplay room
     `0x506fec0` and `0x10af089a93` (both `+0xb = 0` anyway in the census), and the mode objects. The update count, the
     rate words `[main+0x593b444]`/`[main+0x523d004]`, the slow table and the room are untouched.
   - **The step:** the same button, read from P1's own sample (P1's practice): ZL, or the GameCube L trigger, by the
     native table (`0x45434f0`, guarded as a data row). A new press lets exactly one entry run unheld: one slow step
     (one logic frame) and one full scheduler pass. Held from the 18th poll on, every entry runs while it is held.
     No step while the speed hold is open (its buttons are hidden and it owns P1's D-pad), while an input owner blocks,
     or while the waiting-room UI is busy. Outside the hold ZL itself still reaches the fighter, as natively.
   - **A chord's ZL never steps (review of `985c79c6`):** the step reads the same sets as the detector, without the
     buttons the chord stage keeps from the game on that poll (`lobby_speed::step_input`): a chord that passed, and
     its buttons until each is released. D-pad Down + ZL saves and doesn't step, pressed or held (`985c79c6` read the
     raw sets and did both).
   - **Inferred, Eden (b):** the input mapping sees the paused word 0 and N 1 during the hold (`.1753598`), so a button
     pressed between steps counts only if it is still held on the step's update (native 1 Frame holds it).
   - **The online match is never touched:** another class (`0x5061460`), checked per call, and the hook acts only while
     the practice's live flag is armed.
3b. **Reuse check (required 2026-11-21 16:37Z: reuse the code already shown to be desync-safe, plus the ZL
   trigger).** Re-read against the entry's code (`.135ca00`–`.135cc80`).
   - **Reused unchanged:** the bind and its pop-up, the lobby's wanted speed and `LobbyPlan` dispatch, the live flag armed
     by Player 1's frame, the slot release, both resets (`practice_changed`, `session_clear`), and the rule that the lobby
     never writes the rate words.
   - **The ZL step is the game's own x1 update:** on a step the hook returns without touching anything; the entry runs one
     slow step, one scheduler pass, the walker once. No plugin code steps the world.
   - **The hold can't be made from the proven seams:**
     - **The slot (1/4, 1/2, 2/3):** the slow step clamps the divisor to 1..100 (`.135cae4`–`.135caf4`), takes a new
       divisor only at a period's end (`.135cb3c`, `.135cbb0`), and every period's end counts a logic frame (`.135cb48`,
       `.135cbb4`). The slowest a slot goes is one frame every 100 updates (1.67 s), by itself; ZL can't end a period
       early, because nothing the proven code writes reaches the sub-counter `main+0x52c518a`.
     - **The waiting-area hook `.151b170`** is called at `.135cc2c`, after this update's slow step and scheduler pass
       (`.135cc18`). It can add a world step (x1.5), not take one away.
     - **The game's own pause:** the global word and `FUN_00555da0`'s side effects (above).
   - **So the hold stays the one new hook** (`.135ca38`, w22 for that call only, no game memory written). Entering 1 Frame
     also frees the slot of the speed before, through the rc.8 release path.
   - **Only Eden can show:** the paused pass skips the tasks not marked run-while-paused; if one feeds the LDN session the
     guest would see the host stall. The pair run holds the host 60 s without a step for that.
4. **HUD:** Training's stopwatch as natively (2.); the lobby has no stopwatch, and the pop-up shows `1 Frame` while the
   chord is held.
5. **Resets to x1:**
   - **Lobby:** a practice start or end (`practice_changed`), the session clear (the practice P1's destroy, its exit, a
     scene change, a busy UI), and a match: its class is not the waiting area's, and the teardown clears the live flag
     first. The hold holds no memory, so nothing can carry into a match.
   - **Training:** a new Training starts at x1 natively (the menu constructor, `.1bb49f4`); the settings reset writes 1
     (`.13e87dc`); leaving Training leaves the native teardown's pause clears (`.1552370`, `.155353c`). A pending bind
     write is dropped on a scene change (`session_clear`).
   - **"A native reset":** read as Training's own settings reset and, in the lobby, the practice reset that restarts
     P1 (the session clear). **tr-primary-2: confirm**, or name the reset meant.
5b. **Rulings (tr-primary-2 14:12Z):** the hook site and register-only hold approved with conditions (card scan, stock
   guard, waiting-area class and 1 Frame only, no lock, room/rate/slot untouched, ZL only at 1 Frame with the chord not
   held). The lobby's session clear also returns 1 Frame to x1 (`session_clear`). The hook decides every other entry on
   one atomic read (`HOLD_CLASS`: 0, or the waiting area's vtable address while armed, refreshed once per game frame by
   `lobby::update`); only the waiting area's armed entry then reads two slow-state bytes and clears one atomic bit.
6. **Guards and hooks:** a new authoritative group `speed_frame`, 13 rows in its own data file
   (`tools/gen_rows_speed_frame.py`, `data/tables/rows_speed_frame.ROWS.tsv`): the hook `entry_hold`, the entry's two
   `cbnz w22` paths, the walker's paused-skip, the pause sync (called), Training's step mode, press, held, repeat (18),
   run and pause paths, the button functor and its table. The gate `FRAME_ROWS` adds the three `lobby_speed` entry rows.
   If any fails, the bind offers no 1 Frame (`Place::offered`) and the hook is not installed. Card scan of every new
   offset before the code is final. No lock in the hook: atomics only.

## 3. Requirement trace

| Requirement clause | Design | Test |
| --- | --- | --- |
| The speed toggle has a single-frame option | §2.1–2.4 | `the_scale_has_one_frame_slowest_and_stops_at_both_ends`, `one_frame_pop_up_reads_one_frame`, `leaving_one_frame_resumes_the_training_pause`, `the_lobby_step_lets_exactly_one_entry_run_per_press`, `holding_the_step_button_runs_from_the_eighteenth_poll`, `the_training_step_is_the_mode_objects_and_its_button_is_zl`, `a_plain_step_press_steps_the_lobby_once`, `a_plain_held_step_button_still_runs_from_the_eighteenth_update`, `a_held_save_state_chord_never_steps`, `a_held_save_state_chord_never_steps_when_a_repeated_poll_finds_the_edges_cleared`, `the_kept_step_button_is_marked_in_the_previous_set`, `a_release_the_poll_did_not_see_is_not_a_held_step_button`, `a_skipped_poll_forgets_the_kept_step_button`, `an_exposed_release_keeps_the_games_release_edge`, `a_bare_step_press_after_a_gap_is_a_press`, `a_kept_step_button_does_not_follow_a_controller_handoff`, `a_session_clear_on_the_same_pad_keeps_the_chords_step_button`, `an_open_speed_hold_does_not_follow_a_controller_handoff`, `a_speed_chord_pressed_after_a_handoff_opens_the_hold_again`, `pending_requests_survive_a_re_entry`, `a_re_entry_close_and_a_refused_save_fall_on_the_same_poll`, `holding_the_speed_chord_never_steps` |
| It does not desync against the unmodified game, and that is tested | §2.3: a held entry, no global or room write; Eden (b) | `the_lobby_path_never_writes_the_update_rate_words`, `the_hold_overrides_only_the_waiting_area_entry_of_the_live_practice`, `a_step_press_under_the_speed_hold_is_not_a_frame_step` |
| It does not stay enabled into a match | §2.5; Eden (c) | `every_reset_returns_the_lobby_to_x1_and_ends_the_hold`, `the_hold_overrides_only_the_waiting_area_entry_of_the_live_practice` |

## 4. Eden proof (planned)

(a) Training: chord to 1 Frame, ZL steps, stopwatch, back out by chord, then leave and enter another mode at x1.
(b) Quickplay room, TrainingRooms host + vanilla guest, accepted on the host: 1 Frame, steps for a while; both rooms'
counters/timers keep pace (room code `main+0x1564088` cadence, trace counters on both), no disconnect.
(c) Match found while at 1 Frame: x1 on both, no slot, the hold not called for the match class, matching state at
matched frames. (d) Roles swapped if cheap.
