# Input delay — behavioural specification

- **Spec id:** `SPEC-input-03`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude (Fable 5.1), spec-writer for area `input`, 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/input_delay.rs`, the `Delay` option in `training_mod_consts/src/options.rs`, the Misc tab row in `training_mod_consts/src/lib.rs`, the hook order in `src/training/mod.rs`, all at commit `97cbe9b9` (unchanged at `077a8e43`). Fork changes at `…/stages/m5i/stage/src/training/input_delay.rs` lines 10–13 (session clear) and the clear/empty call sites in `…/stage/src/training/mod.rs` lines 1032, 1056, 1075, 1145–1152, `…/stage/src/training/lobby.rs` line 32.
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

The player can add 0 to 30 frames of artificial delay to their own inputs in Training, to practise under online-like conditions. Every mapped input P1 produces is held back by the chosen number of frames before the game, the input display and the recorder see it.

## 2. User-visible behaviour

### 2.1 Menu item (Misc Settings tab, id `misc`)

| Item | Setting id | Type | Values | Default |
| --- | --- | --- | --- | --- |
| Input Delay | `input_delay` | single choice | 0, 1, 2, …, 30 (frames) | 0 |

Help text: "Frames to delay player inputs by". An empty selection means 0.

### 2.2 Visible effect

With delay *d* > 0, P1's fighter reacts *d* frames late to every press, release and stick movement. For the first *d* polls after the delay becomes active (or after the queue was cleared), P1's fighter receives neutral input. No HUD element.

## 3. Inputs

- The Input Delay setting, read on every P1 input poll.
- P1's final mapped input record for the poll (`MappedInputs`, SPEC-input-01 §3.1), as produced by the game's mapping function and after the mod-menu and chord stages of the poll (SPEC-input-01 §5.1).
- Fork: the "menu captures input" condition (mod menu open, the named-macro panel visible, or within 15 frames after the menu closed), the named stop-chord consumption, native waiting-room UI busy, replay-editor ownership — all of which clear the queue.

## 4. Outputs and native state touched

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| P1's final mapped input record (`out`) in the final-input-mapping hook at `main+0x1750dd0` (SPEC-input-01 §4, same hook; this stage runs after chord detection and before the input display and the recorder) | overwrite `out` with the delayed record, or with neutral while the queue is shorter than the delay | each P1 poll (`player_idx == 0`) in Training or an active practice session | hook resolved at the documented site; Training/practice; `player_idx == 0` |
| **Native pause state** (review R2 of package 1, 2026-11-02; read from the owned 13.0.5 `main`): the begin and the end of the Training scene's `StatePause`. The scene state machine registered at `main+0x14d1ec0` (it also registers `StateResetTraining`) maps the name `StatePause` to the factory `main+0x14d3180`; the object's vtable is `main+0x506bd18`; slot 2 (`main+0x14d47b0`) is the state's enter, which loads the pause manager and calls the game's pause-begin routine `main+0x150f0c0` (raises three flag bytes, value 1: `+0xa9` of the object behind the slot at `main+0x5336e10`; `+0x568e` of the object at `+0x11a0` of the one behind the slot at `main+0x5335e90`; and `+0x2d` of the object reached through `+0x1190` and `+0x28` of that same one); slot 3 (`main+0x14d47d0`) is the state's exit and clears exactly those flags, every path leaving through one epilogue at `main+0x14d4868` | two inline hooks, read-only for the game: **pause begin** at `main+0x14d47c0` (the enter's `ldr x0, [x8]`, just before the call) sets the `VANILLA_PAUSE` bit of the input-owner mask and empties the queue in the same locked step; **pause end** at `main+0x14d4868` (the exit's `ldp x29, x30, [sp], #0x10`) releases the bit. The queue is empty from the moment the pause begins even if no mapping poll runs while paused | each begin and each end of the game's pause in the Training scene (the same scene class serves Versus; the mask only matters in native Training) | 64-byte guards at both sites (`sites_pause::SITES`: `pause_enter`, `pause_exit`), both installed or neither; card scan of the 2026-11-01 10:19 MTP preflight copy: no plugin references either site; no site within 0x40 of a site of the CPU pilot |

No other game state is read or written. The queue is plugin state: at most 30 records of 8 bytes.

## 5. Timing and frame rules

Per P1 poll, with delay *d* (0–30) read from the menu and a queue ordered newest-first:

0. **If *d* = 0: empty the queue and leave `out` unchanged (the poll's own current record). Stop.** The queue is emptied *before* anything is read from it, so selecting zero never plays a record queued at the previous delay (review R3 of package 1: the earlier text read the oldest queued record first and returned one stale record on the transition poll).
1. Let `actual` = the poll's current `out`.
2. If queue length < *d*: `out` ← neutral. Else `out` ← the oldest queued record (the queue is non-empty here because *d* ≥ 1).
3. Push `actual` at the newest end.
4. Drop the oldest records until the length is ≤ *d*.

Observable rules that follow:

- Steady state: `out` at poll *t* equals the input polled at *t − d*.
- Increasing *d* from *a* to *b* mid-session: the next *b − a* polls output neutral (the queue refills), then delayed input resumes; nothing is lost.
- Decreasing *d* from *a* to *b* (both nonzero): the transition poll still plays the oldest record (*a* polls old); the *a − b* records between it and the new steady state are discarded unplayed, and from the next poll the output is *b* polls old, so the output jumps forward in time by *a − b* frames. (Reviewed under R3: steps 1–4 as written satisfy this; the transition poll is the last one at the old delay.)
- Queue cleared (§5.1): the next *d* polls output neutral.
- Setting the delay to 0 empties the queue and passes input through unchanged **on the transition poll itself** (step 0): the record the game sees is that poll's own, and the next poll's is its own again.

### 5.1 Clearing the queue (fork fix, fold in)

Upstream never clears the queue; input pressed before the mod menu opened could still be delivered after it closed if the menu was open for fewer than *d* polls, and stale records survived scene changes. The fork clears the queue, and for that poll outputs neutral, when: the mod menu or a modal panel captures input or has closed within the last 15 frames; the named stop chord was consumed this poll; the native waiting-room UI became busy; the practice session is cleared (scene change, hot swap, lobby activation change); the replay editor owns the poll. Fold in as: **the queue is emptied whenever something other than gameplay owns P1's input, and on session clear.** The game's own pause is such an owner (§10 question 3, applied as the working default, `docs/DECISIONS.md` rule for unanswered technical recommendations; review R2 of package 1): the pause-begin hook of §4 sets the `VANILLA_PAUSE` owner bit and empties the queue at once, even if the mapping poll does not run while paused; polls that do run while it is set give the game neutral and keep the queue empty; the pause-end hook releases the bit; the first *d* polls after the resume are neutral, so no record queued before the pause and no record produced by the pause menu's input ever reaches gameplay.

### 5.2 Ordering within the poll

Chord detection (SPEC-input-04) reads the raw controller, so chords are never delayed. The input display (SPEC-input-02) and the recorder (SPEC-input-01) run after this stage and therefore see delayed input. The mod menu's own input handling runs before this stage on the raw controller and is not delayed.

### 5.3 Poll multiplicity

The queue advances once per poll, not once per game frame. If the game polls P1 more than once per frame in some situations (SPEC-input-01 open question 16), the effective delay in frames is shorter than the setting. Open question 1.

## 6. Edge cases and interactions

1. **Recording** (SPEC-input-01): recordings capture delayed input; playback is not delayed again (the CPU injection path does not pass through this stage).
2. **Save-state load, L+R+A reset:** not cleared upstream (the delayed presses before the load are delivered after it). Fold in the session-clear rule only for the fork's listed owners; whether a save-state load should clear is open question 2.
3. **CPU-as-P1:** the delay applies to the human controller's mapping for `player_idx 0` regardless of who drives P1; an AI-driven P1 is unaffected.
4. **Lobby practice:** active in the waiting room for P1 when the practice session is active; cleared when the native UI (dialogs, pause) takes over.
5. **Ice Climbers, Pokémon Trainer, hot swap:** P1's mapping index does not change; no special case. A hot swap clears the queue through session clear.
6. **Menu open during the delay window:** see §5.1; the menu's own 15-frame post-close input kill also runs before this stage, so those polls push neutral records.
7. **Game pause (vanilla menu):** the game's pause is an input owner (§5.1; the begin and end hooks of §4). Upstream let polls continue while paused, so the queue filled with the paused-state inputs and the first *d* polls after unpausing delivered them; this plugin does not (open question 3, recommendation applied).

## 7. Persistence

None beyond the menu setting (menu spec).

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | queue ≤ 30 × 8 B on the heap plus a growable container header; code not isolated — see `docs/resource-costs.md` (W5) | fixed 31-slot ring (248 B) in static data | `tools/build/nro_size.py` per-feature build |
| per-frame work | one 8-byte push, one truncate, one copy per P1 poll | same | diag counters |
| heap allocations in hooks | upstream uses a growable deque: allocates on first growth and when capacity doubles (up to 30) | 0 | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Menu item, values, default | `delay::menu_default_zero` | IN03-E01 | `evidence/input/IN03-E01.md` |
| Steady state: output equals input *d* polls earlier, for d = 0, 1, 15, 30 | `delay::steady_state_offsets` | IN03-E02 (visible reaction delay with the input display) | |
| First *d* polls after activation are neutral | `delay::initial_fill_neutral` | IN03-E02 | |
| Increase and decrease mid-session (§5) | `delay::change_up_refills`, `delay::change_down_drops_oldest` | IN03-E03 | |
| Clear on menu capture, modal owner, native UI busy, session clear (§5.1) | `delay::clear_on_input_owner_change` | IN03-E04 (press during menu, close, nothing fires) | |
| Chords are not delayed; display and recorder see delayed input (§5.2) | `delay::ordering_with_chords_display_recorder` | IN03-E05 (chord at d = 30 fires at once) | |
| Delay 0 passes the current record through on the transition poll (step 0); press and release records; the decreasing-nonzero transition (§5) | `delay::delay_zero_empties_the_queue_and_passes_through`, `delay::delay_zero_transition_with_press_and_release_records`, `delay::change_down_matches_the_observable_rule` | IN03-E03 | |
| The game's pause owns the input: pause begin sets the owner bit and empties the queue (even if polls stop); polls while paused give neutral; pause end releases; the first *d* polls after the resume are neutral; no pre-pause or menu record reaches gameplay (driven through the real producer, not a hand-passed owner value) | `input_pause::a_pause_empties_the_queue_owns_the_input_and_nothing_old_reaches_gameplay`, `input_pause::takeover_empties_the_queue_even_when_polls_stop_while_paused`, `input_pause::the_unwired_producer_lets_the_stale_and_menu_records_through` | IN03-E06 (delay 5, queue an input, open the native pause, give menu input, resume; diag `pause_owner` lines) | `eden/EDEN-PACKAGE-1.md`, R1-fix section |
| The pause sites: guards from the ELF, the chain `StatePause` → factory → vtable → enter/exit → pause flags, card scan, no pilot overlap | `pause_sites::statepause_chain_from_the_state_name_to_the_two_hooked_instructions`, `pause_sites::the_rows_are_the_two_pause_sites_and_clear_of_everything_else` | IN03-E06 | install line `pause_enter=stock/near pause_exit=stock/near` |
| Zero heap allocations in the hook | `delay::ring_allocates_nothing`; the detector itself: `hook_alloc::the_detector_reports_a_stub_that_allocates_and_passes_one_that_does_not` | diag `hook_stats` line after the lane: nonzero hook coverage, delta 0 (review R5) | install line; `eden/hook_stats.py` |

## 10. Open questions

1. **Poll multiplicity.** If the mapping function runs more than once per game frame, advance the queue only when the in-game frame stamp changed (the input display already de-duplicates this way). Needs the diag measurement of SPEC-input-01 open question 16 first.
2. **Clear on save-state load and L+R+A reset?** Not upstream. Recommendation: clear on both (a load repositions the fighters; stale presses are never what the player meant). **Decided 2026-11-01 (B8 batch): accepted** (the queue is cleared on a save-state load and on a reset).
3. **Polls during the vanilla pause.** Recommendation: treat the vanilla menu as an input owner (clear and output neutral while it is open), consistent with §5.1. **Applied 2026-11-02 as the working default** (unanswered technical recommendation, `docs/DECISIONS.md`), with the authoritative native transition of §4; review R2 of package 1.
4. **Range.** 0–30 matches upstream. The menu redesign may prefer a slider; the behaviour does not change.
