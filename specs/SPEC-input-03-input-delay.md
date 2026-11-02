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

No other game state is read or written. The queue is plugin state: at most 30 records of 8 bytes.

## 5. Timing and frame rules

Per P1 poll, with delay *d* (0–30) read from the menu and a queue ordered newest-first:

1. Let `actual` = the poll's current `out`.
2. If queue length < *d*: `out` ← neutral. Else if the queue is non-empty: `out` ← the oldest queued record. (With *d* = 0 the queue is always empty after step 4, so `out` is unchanged.)
3. Push `actual` at the newest end.
4. Drop the oldest records until the length is ≤ *d*.

Observable rules that follow:

- Steady state: `out` at poll *t* equals the input polled at *t − d*.
- Increasing *d* from *a* to *b* mid-session: the next *b − a* polls output neutral (the queue refills), then delayed input resumes; nothing is lost.
- Decreasing *d* from *a* to *b*: the *a − b* oldest records are discarded unplayed; the output jumps forward in time by *a − b* frames.
- Queue cleared (§5.1): the next *d* polls output neutral.
- Setting the delay to 0 empties the queue on the next poll and passes input through unchanged.

### 5.1 Clearing the queue (fork fix, fold in)

Upstream never clears the queue; input pressed before the mod menu opened could still be delivered after it closed if the menu was open for fewer than *d* polls, and stale records survived scene changes. The fork clears the queue, and for that poll outputs neutral, when: the mod menu or a modal panel captures input or has closed within the last 15 frames; the named stop chord was consumed this poll; the native waiting-room UI became busy; the practice session is cleared (scene change, hot swap, lobby activation change); the replay editor owns the poll. Fold in as: **the queue is emptied whenever something other than gameplay owns P1's input, and on session clear.**

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
7. **Game pause (vanilla menu):** polls may continue while paused; the queue then fills with the paused-state inputs (typically neutral or the pause button) and the first *d* polls after unpausing deliver them. Upstream behaviour; open question 3.

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
| Zero heap allocations in the hook | `delay::ring_allocates_nothing` | — | install line |

## 10. Open questions

1. **Poll multiplicity.** If the mapping function runs more than once per game frame, advance the queue only when the in-game frame stamp changed (the input display already de-duplicates this way). Needs the diag measurement of SPEC-input-01 open question 16 first.
2. **Clear on save-state load and L+R+A reset?** Not upstream. Recommendation: clear on both (a load repositions the fighters; stale presses are never what the player meant). **Decided 2026-11-01 (B8 batch): accepted** (the queue is cleared on a save-state load and on a reset).
3. **Polls during the vanilla pause.** Recommendation: treat the vanilla menu as an input owner (clear and output neutral while it is open), consistent with §5.1.
4. **Range.** 0–30 matches upstream. The menu redesign may prefer a slider; the behaviour does not change.
