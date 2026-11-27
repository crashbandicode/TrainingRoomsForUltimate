# Single-frame speed in Training and waiting-room practice

On 2026-11-21, rc.15 added **1 Frame** to the speed chord in both modes. See the [game speed guide](../guide/game-speed.md#1-frame).

## Decision and sources

| Record | Detail |
| --- | --- |
| Previous decision | [The rc.8 bind](rc8-binds.md) left 1 Frame in the native menu on 2026-11-11. |
| Change | tr-primary-2’s 13:20Z brief followed the 13:16Z decision. |
| Lane | tr-speed-frame (Devin, Opus 5.5), `rc15/speed-frame` from `5fb35633`. |
| Worktree | `/home/intpa/tr-fixes-20261113/rc15-speed-frame` |
| Game | Owned 13.0.5 main ELF, `0789f0a4…`. Addresses below are main-relative. |
| Disassembly | `llvm-mc`, `/home/intpa/speed-frame-re/dis.py` |
| Decompilation | Ghidra, `/home/intpa/speed-frame-re/o0*.txt` |
| Earlier measurements | QCF’s `RESEARCH-rc8-speed.md` §3 and §9, and `qcf-spd05y` task census. |

## Native Training step

| Native field or function | Behavior |
| --- | --- |
| `PauseMenu+0xb54`, `.155b654`, `rates_slow` | Value 2 selects 1 Frame. Code 0, 60 updates, no slow slot. |
| Request word `0x52b92ec` | Training owns bit 2 (`orr #4` at `.1556c70`). |
| Bit clears | `.1556b7c`, `.155a70c`, `.155b608`; see `g-52b92e8.txt`. |
| `FUN_00555da0`, `.555ef4`, decompile `o02.txt` | Sole writer of paused word `0x52b92e8`. Synchronizes requests and applies pause/resume side effects. |
| Paused scheduler | Skips slow step (`.135cab4 cbnz w22`). Runs six paused tasks including registry walker (`.135cc10`). |
| `FUN_015569e0`, `+0x194 == 2` | Reads step input each update (`.1556a18`–`.1556c78`). |
| No input | Sets bit 2, then synchronizes the pause word. |
| Step press | Clears bit 2 and synchronizes, permitting one update. |
| `FUN_0177e8f0` | Scans ten controllers with functor `0x509ebd0`. |
| Mode 0 / 1 | `just_down & current` at `.177ec20` / held. |
| `0x177fe20`, table `0x45434f0` | Masks by style: `400,400,100,100,400,400,100` (hex). |
| Styles 1, 2, 5, 6 | Raw ZL (`0x400`). |
| Styles 3, 4, GameCube (7) | Raw L (`0x100`); GameCube L is logical ZL. |
| Hold counter `+0x1bc` | Runs continuously from update 18 (`.1556abc`, `.1556b24`–`.1556b34`). |

## Shipped controls

| Context | Behavior |
| --- | --- |
| Speed chord | Both lists run from 1 Frame through 1/4, 1/2, 2/3, x1, x1.5. |
| List ends | Stop at each end (`TRAINING_SPEEDS`, `LOBBY_SPEEDS`). |
| Popup | D-pad arrows flank **1 Frame**. |
| Training | Native stopwatch appears away from x1. |
| Lobby | The popup shows 1 Frame while held. There is no stopwatch. |

**Training apply**

- Store value 2 at `+0xb54`. Refresh text through `0x1bbbd30` and update the HUD. `training_apply` sets `+0x194 = 2`, frees the old slot, clears the step and selects code 0 / 60.
- From the next update `FUN_015569e0` handles ZL as after the native menu. `hud_icon` shows the stopwatch and the game writes the value-2 label. The [button glyph fix](rc15-fix-f5-f3.md) supplies the menu’s glyph style when the menu is closed (2026-11-23).
- Leaving 1 Frame clears request bit 2 (`.155b608`). In closed-menu play the bind also synchronizes the paused word if it still has bit 2. This follows the mode’s run path (`.1556b64`–`.1556b84`), using guards `game_speed.pause_sync` and `game_speed.step_run`.
- Original test (a) checked whether P1’s input mapping, outside the scheduler at `FUN_0135f570` / `.1360ad4`, can exit 1 Frame while paused.

**ZL ownership**

- Review of `985c79c6`, 2026-11-21 16:55Z and 17:38Z: the mode object reads the controller after mapping. It checks `just_down & current` at `.1556a78` and held `current` at `.1556ae0`. Hiding a button just during mapping was insufficient.
- `lobby_speed::step_take` and `StepKeep` remove ZL during the speed hold. They also remove a successful ZL chord’s button until release, preventing a fighter input or frame step. A plain ZL press passes through.
- Clear current and both edge sets. Mark previous. Plus, read by the same reader (`.1558e20`, mask `0x40000`), needs only its edges hidden (`6e226733`). With ZL, preserving current let a held chord run Training from update 18 (`2bd5445d`).
- Refresh copies cleared current into previous, restores held ZL in current and reports another press. A refresh after physical release does not report one (`6e226733` trace).
- Repeated polling before refresh leaves edges zero and current/previous as written. All 46 repeats on `3448686c` showed this (`artifacts/phase23/1f-save-hold/repoll-boot/`, smash-training-lobby).
- The detector sees one held chord. The mode reads no current bit. The game’s release check needs both previous and just-released. A mark counts only without just-released, preserving a release missed by the poll (review-games-astra-3, 23:51:14Z).
- After a skipped P1 poll or controller change, clear the kept bit and chord-owned buttons, and close the speed hold. Preserve pending requests. Reset under the poll lock, then close the popup before processing new notices or events.
- A session clear on the same controller preserves the kept bit and chord-owned buttons. If the speed chord ends before ZL, restore current with no edge. The native counter can reach 18 without an extra step press.
- The original poll-order checks used three presses and a hold under the speed chord, then D-pad Down + ZL for two seconds. Expected results were zero steps and one save.

**Waiting-room hold**

- `speed_frame.entry_hold` at `main+0x135ca38` follows the paused-word load into w22 (`.135ca30`, `FUN_0135ca00`). It runs only for live practice at 1 Frame when the entry has waiting-area vtable `main+0x506d4d8` at `[x20]`.
- Without a step due, set x22 to 1 for this entry only. The entry skips the slow step and runs the paused scheduler. Global paused stays zero and the hook writes no game memory.
- The six paused tasks still run. The walker’s per-object skip reads the global word (`FUN_01358ad0`), so mode objects and Quickplay objects `0x506fec0` and `0x10af089a93` tick at x1. Both objects had `+0xb = 0` in the census.
- Update count, rate words `0x593b444` and `0x523d004`, slow table and room retain their native behavior. The online match class `0x5061460` fails the per-call class check.
- P1’s own sample supplies ZL (GameCube L, guarded table `0x45434f0`). A press lets one normal entry run: one slow step, one logic frame, one full scheduler pass, one walker tick. From the 18th held poll every entry runs.
- The popup, an input owner or busy waiting-room UI blocks stepping. Otherwise a plain ZL still reaches the fighter. `lobby_speed::step_input` ignores successful chords and their hidden buttons until each is released.
- `985c79c6` read raw input and both saved and stepped on D-pad Down + ZL. The shipped chord produces one save without a step, even when held.
- Test (b) considered whether a press between steps survives only while still held at the step. The mapping reads paused zero and N 1 (`.1753598`). Native 1 Frame holds that input.

**Why the entry hook was needed (2026-11-21 16:37Z reuse review)**

| Candidate | Finding at `.135ca00`–`.135cc80` |
| --- | --- |
| Existing bind | Reuse the popup, wanted speed, `LobbyPlan`, P1-frame live flag and slot release. |
| Lifecycle | Reuse `practice_changed`, `session_clear` and the lobby’s existing rate-word rule. |
| Step | Return directly from the hook to run the native x1 entry. |
| Slow slot | Limits divisor to 1–100 (`.135cae4`–`.135caf4`), writing `1/N` every update (`.135cbb8`–`.135cc00`). |
| Period end | Latches divisor (`.135cb3c`, `.135cbb0`) and counts a logic frame (`.135cb48`, `.135cbb4`). |
| Slowest slot | One frame per 100 updates (1.67 s). Existing writes cannot reach sub-counter `0x52c518a`. |
| `.151b170` waiting-area hook | Called at `.135cc2c`, after the slow step and scheduler (`.135cc18`). Can add passes. |
| Global pause | `FUN_00555da0` changes global state, calling `0x555f20` and sound function `0x35a0460`. |
| Other native pause effects | Writes state at `0x5335e90` and `0x52c50f0`, outside normal waiting-room behavior. |
| Selected hold | Single register-only hook at `.135ca38`; entering 1 Frame uses the rc.8 slot release. |
| Original pair check | Hold host for 60 seconds and watch for a stalled guest LDN session. |

**Resets and guards**

- The 14:12Z ruling approved the register-only hold with a card scan, stock guard, waiting-area class check, 1 Frame only, no locks or global room/rate/slot writes, and ZL stepping outside the speed chord.
- `practice_changed` returns the lobby to x1 on practice start/end. `session_clear` handles P1 destruction, exit, scene change and busy UI. The live flag clears before match teardown and match entries have another class.
- New Training starts at x1 (`.1bb49f4`). Native settings reset stores 1 (`.13e87dc`); exit clears pause (`.1552370`, `.155353c`). Scene change drops a pending bind write. The original reset question was settled by Training’s native reset and the lobby session clear.
- `lobby::update` refreshes `HOLD_CLASS` once per game frame: zero or the armed vtable. Other entries cost one atomic read. Only the armed class reads two slow-state bytes and clears one atomic bit.
- `tools/gen_rows_speed_frame.py` generates 13 `speed_frame` guards in `data/tables/rows_speed_frame.ROWS.tsv`. They cover `entry_hold`, both `cbnz w22` branches, walker paused-skip, pause sync, step mode/press/hold/repeat (18)/run/pause, button functor and table.
- `FRAME_ROWS` adds three lobby-speed entry rows. A mismatch removes 1 Frame from `Place::offered` and skips hook installation. The original release checklist called for scanning every new offset. The hook uses atomics only.

## 3. Requirement trace

| Requirement clause | Design | Test |
| --- | --- | --- |
| The speed toggle has a single-frame option | Scale, native Training and waiting-room step | `the_scale_has_one_frame_slowest_and_stops_at_both_ends` |
| 〃 | 〃 | `one_frame_pop_up_reads_one_frame` |
| 〃 | 〃 | `leaving_one_frame_resumes_the_training_pause` |
| 〃 | 〃 | `the_lobby_step_lets_exactly_one_entry_run_per_press` |
| 〃 | 〃 | `holding_the_step_button_runs_from_the_eighteenth_poll` |
| 〃 | 〃 | `the_training_step_is_the_mode_objects_and_its_button_is_zl` |
| 〃 | 〃 | `a_plain_step_press_steps_the_lobby_once` |
| 〃 | 〃 | `a_plain_held_step_button_still_runs_from_the_eighteenth_update` |
| 〃 | 〃 | `a_held_save_state_chord_never_steps` |
| 〃 | 〃 | `a_held_save_state_chord_never_steps_when_a_repeated_poll_finds_the_edges_cleared` |
| 〃 | 〃 | `the_kept_step_button_is_marked_in_the_previous_set` |
| 〃 | 〃 | `a_release_the_poll_did_not_see_is_not_a_held_step_button` |
| 〃 | 〃 | `a_skipped_poll_forgets_the_kept_step_button` |
| 〃 | 〃 | `an_exposed_release_keeps_the_games_release_edge` |
| 〃 | 〃 | `a_bare_step_press_after_a_gap_is_a_press` |
| 〃 | 〃 | `a_kept_step_button_does_not_follow_a_controller_handoff` |
| 〃 | 〃 | `a_session_clear_on_the_same_pad_keeps_the_chords_step_button` |
| 〃 | 〃 | `an_open_speed_hold_does_not_follow_a_controller_handoff` |
| 〃 | 〃 | `a_speed_chord_pressed_after_a_handoff_opens_the_hold_again` |
| 〃 | 〃 | `pending_requests_survive_a_re_entry` |
| 〃 | 〃 | `a_re_entry_close_and_a_refused_save_fall_on_the_same_poll` |
| 〃 | 〃 | `holding_the_speed_chord_never_steps` |
| It does not desync against the unmodified game, and that is tested | Waiting-room hold and paired-console test (b) | `the_lobby_path_never_writes_the_update_rate_words` |
| 〃 | 〃 | `the_hold_overrides_only_the_waiting_area_entry_of_the_live_practice` |
| 〃 | 〃 | `a_step_press_under_the_speed_hold_is_not_a_frame_step` |
| It does not stay enabled into a match | Scene reset and match-found test (c) | `every_reset_returns_the_lobby_to_x1_and_ends_the_hold` |
| 〃 | 〃 | `the_hold_overrides_only_the_waiting_area_entry_of_the_live_practice` |

## Original Eden acceptance plan, 2026-11-21

| Case | Observation requested |
| --- | --- |
| (a) Training | Select 1 Frame, step with ZL, check stopwatch, exit by chord, change modes at x1. |
| (b) Quickplay pair | TrainingRooms host accepts vanilla guest. Step in 1 Frame and check both rooms stay connected. |
| Pair timing | Compare both trace counters and room code cadence at `main+0x1564088`. |
| (c) Match found | Both reach x1. No slot, match-class hold or state difference at matched frames. |
| (d) Roles swapped | Optional repeat with host and guest reversed. |

### Later paired-console result

| Run | Observed result |
| --- | --- |
| `rc15-ldn-xps-2` | 15/15 rows passed with TrainingRooms host and vanilla guest. |
| Practice | Held world froze. Ten taps and held ZL advanced it; chord-owned ZL gave no step. |
| Match | All 2,105 match-class records per console had x22=0. |
| Detector | Both consoles ended at frame 2,426 with `desync_seen=0`. |
| Return | Both reached room fighter select. Waiting-area x1 after return was outside this run. |

Run receipts: `artifacts/phase23/1f-save-hold/ldn-pair/rc15-ldn-xps-2` in the campaign workspace.
