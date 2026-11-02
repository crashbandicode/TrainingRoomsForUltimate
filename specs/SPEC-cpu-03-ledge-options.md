# CPU ledge options — behavioural specification

- **Spec id:** `SPEC-cpu-03`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/ledge.rs`, the transition-term hook and ledge call in `src/training/mod.rs`, `src/training/input_record.rs` (ledge playback entry only), `training_mod_consts/src/options.rs` (`LedgeOption`, `LongDelay`), `training_mod_consts/src/lib.rs` (Defensive and Override tabs) at commit `97cbe9b9` (and `077a8e43` where it differs); fork changes at `…/stages/m5i/stage/src/training/ledge.rs` lines `49-50, 106-108, 187-192` from `docs/provenance/provenance-files.md`
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

Shared definitions are in `SPEC-cpu-01` (mash queue, weighted picks, CPU predicate) and `SPEC-cpu-07` (frame counters, reset bundle).

## 1. Purpose (one paragraph)

When the practice CPU hangs on a ledge it gets up the way the player wants to drill: neutral getup, ledge roll, ledge jump, getup attack, waiting on the ledge, or playing back a recorded input sequence, each optionally after a delay, and each optionally followed by a mash action chosen per getup kind.

## 2. User-visible behaviour

### Menu items

| Row (Defensive tab) | Setting key | Type | Values | Default |
| --- | --- | --- | --- | --- |
| Ledge Options | `ledge_state` | multi-select, weighted | Neutral Getup (`NEUTRAL`), Roll (`ROLL`), Jump (`JUMP`), Getup Attack (`ATTACK`), Wait (`WAIT`), Playback Slot 1–5 (`PLAYBACK_1`…`PLAYBACK_5`) | Neutral Getup, Roll, Jump, Getup Attack |
| Ledge Delay | `ledge_delay` | **multi-select + range** (draft 9, N3): weighted checklist or a min–max range, one at a time (SPEC-core-02 §2.8) | 0, 10, 20, …, 300 frames (31 steps); range: any step value from min to max, uniform | none (behaves as 0, but see §5 playback buffering) |

Override tab rows read here (weighted Action multi-selects, default none): Ledge Neutral Getup `ledge_neutral_override`, Ledge Roll `ledge_roll_override`, Ledge Jump `ledge_jump_override`, Ledge Attack `ledge_attack_override`. Mash tab: trigger "Ledge Option" in `mash_triggers` and `mash_state`.

Help texts (upstream): "Actions to be taken when on the ledge"; "How many frames to delay the ledge option".

### Where it is live

Training and lobby practice, CPU-operated fighters, whenever `ledge_state` has at least one option and no input playback is running. An empty row leaves the native ledge behaviour untouched.

### Visible outputs

The CPU's getup; a playback option starts the recording spec's playback (its own notification, if any, belongs to that spec).

## 3. Inputs

- Menu rows above (live).
- CPU predicate; the fork's effective-option filter (removes Playback options from `ledge_state` in a waiting scene while practice is not enabled).
- Game state per CPU fighter: situation kind (`SITUATION_KIND_CLIFF` = hanging), status kind, previous status kind, motion frame (`MotionModule::frame`), the work flag `FIGHTER_INSTANCE_WORK_ID_FLAG_CATCH_CLIFF` (false while the grab still needs to be buffered, true once the getup can be taken; distinguishes a regular grab from a tether/lasso grab in the frame rules below), and the transition term `FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_CLIFF_ATTACK` (true once any ledge action can be taken).
- The game's query of the transition term `FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_CLIFF_CLIMB` (the native automatic climb).
- Input-recording interface: "is playback running", "start playback of slot n", "start ledge playback of slot n" (playback with a 5-frame input buffer so the first recorded input is not lost to the ledge-grab window; 4 frames if the CPU is still in `CLIFF_CATCH` when started).
- Frame counter (SPEC-cpu-07): one in-game counter for the ledge delay.

## 4. Outputs and native state touched

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| The getup status | `StatusModule::change_status_request_from_script(accessor, status, true)` with `FIGHTER_STATUS_KIND_CLIFF_CLIMB` (Neutral), `CLIFF_ESCAPE` (Roll), `CLIFF_JUMP1` (Jump), `CLIFF_ATTACK` (Getup Attack) | when the chosen option is performed (§5) | training; CPU-operated; hanging; option set non-empty; not in playback |
| Native automatic climb denied | Replace hook on `WorkModule::is_enable_transition_term(accessor, term)`, `main+0x208dd60` (13.0.5; four-instruction leaf thunk, shares a 64-byte window with `main+0x208de30`/`0x208de50`). Call the original; when training is active, the fighter is CPU-operated, its status is `CLIFF_WAIT`, `ledge_state` is non-empty, a ledge case is held, and `term == FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_CLIFF_CLIMB`, return **false** if the held case is Wait, or the delay has not elapsed (counter < delay), or the held case is a playback option and the playback has not started yet; otherwise return the original | every query of that term | as stated |
| Mash queue | `SPEC-cpu-01` external buffer entry (full reset, then buffer) | right after the getup is requested | — |
| Playback | recording-spec entry points (see §3) | as in §5 | — |

## 5. Timing and frame rules

All of this runs in the once-per-frame dispatch (after the fighter's frame) for each CPU-operated fighter, after fast fall and before shield (SPEC-cpu-01 §5), and only when no input playback is running and `ledge_state` is non-empty.

1. **Not hanging** (situation is not cliff): clear the held ledge case, clear the held delay and reset the delay counter. Return. (So both are re-rolled on the next grab.)
2. **Hanging**: if no delay is held, pick one from `ledge_delay` (an empty row gives 0; a checklist is a weighted pick; a set range is a uniform pick among its step values, N3). If no case is held, pick one from the effective `ledge_state`. If the effective set is empty (fork fix, required), hold nothing and return: the CPU behaves natively this grab and the climb is not denied.
3. Compute, with `frame` = integer motion frame, `delay` = held delay, `catch_flag` = the `CATCH_CLIFF` work flag:
   - `buffer_playback_now` = `delay == 0` and `frame == 13` (18 − 5 frames of input buffer).
   - `buffer_now` = in `CLIFF_WAIT` with previous status `CLIFF_CATCH` (a regular grab): `delay == 0` and `frame == 19` and `!catch_flag`; in `CLIFF_WAIT` otherwise (a tether/lasso grab): `delay == 0` and `frame == 18` and `catch_flag`; else false.
4. **Cannot act yet** (`CONT_CLIFF_ATTACK` not enabled): if `buffer_playback_now`, the case is a playback option and the Ledge Delay row is non-empty, start the ledge playback of that slot and return. Otherwise, if not `buffer_now`, return. (`buffer_now` lets a zero-delay option be requested on the last unactionable frame so it comes out on the first actionable frame, which the transition-term check alone cannot time.)
5. **Wait**: if the held case is Wait, return without clearing it (the climb stays denied; the CPU hangs until the game drops it).
6. **Delay**: if the delay counter says "still waiting" (SPEC-cpu-07 `should_delay(delay)`), return. Frames before `CONT_CLIFF_ATTACK` becomes enabled do not count unless `buffer_now` let the code reach this point.
7. **Perform**: a playback option starts the normal playback of its slot; any other option requests its status (table in §4).
8. **Mash after the getup**: pick the override row for the performed kind (Neutral/Roll/Jump/Attack) if non-empty, else `mash_state` if the Ledge Option trigger is ticked, else nothing; feed it to the external buffer entry of SPEC-cpu-01. Wait and playback options have no override row (only the trigger path applies after a playback option is performed).

The held case is not re-rolled while hanging (so Wait keeps waiting), except when the menu changes the Ledge Options row (B11, below). The delay is held for the whole grab. Both reset when the CPU leaves the ledge (step 1); the reset bundle (L+R+A, save-state load) also resets the delay and its counter but not the case (the case clears through step 1 anyway).

**Setting changes (decided 2026-11-02, B11, option (a)):** when the menu changes the Ledge Options row, the held ledge case is cleared at once, so the CPU rolls again from the new set at its next frame while hanging, in the same hang (like SDI Direction, SPEC-cpu-02 §5); a CPU already parked with Wait takes the new option without leaving the ledge. An edit that empties `ledge_state` stops the feature immediately (step gate), clears the held case, and the climb is no longer denied. The held delay and its counter are not touched by a Ledge Options change; a Ledge Delay change keeps the per-grab rule (B11 names the options only). The fork gets the same change in its next build; the clean room implements it from this spec.

## 6. Edge cases and interactions

- **Tether grabs** (Link, Samus, Joker, Lucas, …) enter `CLIFF_WAIT` without a prior `CLIFF_CATCH`, hence the second `buffer_now` rule and the ledge-playback buffer adjustment.
- **Ledge trump**: the trumped CPU is in `CLIFF_ROBBED` and airborne, so the case clears; the trump is a mash trigger (SPEC-cpu-01).
- **Ice Climbers**: Popo and Nana are both CPU-operated; each gets its own case/delay (per-fighter state, see open question 4) and both can be driven.
- **Pokémon Trainer**: a switch is not possible on the ledge; nothing special.
- **Kirby, Mii, Aegis, Steve, Olimar, Rosalina**: no special handling.
- **Lobby practice** (fork): Playback options are removed from the effective set while the waiting scene has practice disabled; with an empty effective set the CPU gets up natively and the climb is not denied (fork fix). The fork documents an LDN check where a recorded getup attack played from the ledge.
- **Save states**: a load places the CPU in WAIT/FALL (not hanging), so the case clears; the delay and counter are reset by the bundle.
- **Input playback running**: the feature yields entirely (native ledge behaviour, no climb denial except the "playback option not yet started" rule, which exists to hold the CPU until the recording spec starts).
- **Hot swap**: a replaced body is not hanging; session clear resets the delay and case.
- **Menu open**: paused; no change.

## 7. Persistence

Nothing is written. Settings live in the menu JSON.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | see docs/resource-costs.md (W5) | per CPU fighter: held case, held delay, one counter index: < 16 bytes | `tools/build/nro_size.py` per-feature build |
| per-frame work | see docs/resource-costs.md (W5) | one situation read when not hanging; while hanging: status, previous status, motion frame, one flag, one transition term, no allocation | diag counters |
| heap allocations in hooks | upstream: one per weighted pick | 0 | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Case and delay rolled once per grab, cleared when leaving the ledge | `cpu03_case_lifetime` | `cpu-03-regrab-rerolls` | `evidence/cpu-03/lifetime.md` |
| Status per option (climb, escape, jump1, attack) | `cpu03_option_status_table` | `cpu-03-each-getup` | `evidence/cpu-03/options.md` |
| Zero-delay buffer frames: 19 regular, 18 tether, 13 playback | `cpu03_buffer_frames` | `cpu-03-zero-delay-first-frame`, `cpu-03-tether-first-frame` | `evidence/cpu-03/frames.md` |
| Delay counted only from actionability; Wait holds; climb denied while waiting/delaying | `cpu03_delay_and_climb_denial` | `cpu-03-delay-60`, `cpu-03-wait` | `evidence/cpu-03/delay.md` |
| Playback option: early buffer needs Ledge Delay non-empty; normal path otherwise; climb denied until playback starts | `cpu03_playback_paths` | `cpu-03-playback-slot1` | `evidence/cpu-03/playback.md` |
| Post-getup mash: override per kind, else trigger, else none | `cpu03_post_getup_mash` | `cpu-03-roll-then-fsmash` | `evidence/cpu-03/mash.md` |
| Empty effective set → native behaviour (lobby without practice) | `cpu03_empty_effective_set` | `cpu-03-lobby-playback-only` | `evidence/cpu-03/lobby.md` |
| Per-fighter state for Ice Climbers | `cpu03_per_fighter_state` | `cpu-03-ice-climbers` | `evidence/cpu-03/ic.md` |
| **N3** Ledge Delay range: a set range is picked uniformly among its step values once per grab; a checklist stays a weighted pick; a set range counts as a non-empty Ledge Delay row for the early playback buffering (question 1) | `cpu03_ledge_delay_range_pick`, `cpu03_range_counts_as_nonempty_row` | `cpu-03-delay-range-30-60` | `evidence/cpu-03/delay.md` |
| **B11** Changing the Ledge Options row while hanging clears the held case and re-rolls from the new set at the next frame, in the same hang (Wait → new option without leaving the ledge); emptying the row stops the feature and clears the case; the held delay is unchanged | `cpu03_ledge_options_change_rerolls_held_case`, `cpu03_ledge_options_emptied_clears_case` | `cpu-03-wait-then-change-to-roll` (CPU parked with Wait, menu changes to Roll: it rolls without a second grab), `cpu-03-change-to-empty` | `evidence/cpu-03/change.md` |
| **N3** Checklist and range are mutually exclusive on this row; a file with no range loads as a checklist | `cpu03_ledge_delay_exclusive_and_legacy_load` (with `core02_delay_row_exclusion`) | — | — |
| Yields during input playback; reset bundle resets delay | `cpu03_playback_yield_and_reset` | `cpu-03-savestate-on-ledge` | `evidence/cpu-03/reset.md` |

## 10. Open questions

1. **Ledge Delay row required for early playback buffering.** Upstream buffers a playback option at frame 13 only when the Ledge Delay row has a selection (even "0"); with the row empty the playback starts only when the ledge becomes actionable, which can be a few frames late (trumpable). Recommendation: treat an empty row exactly as a selected "0" everywhere, removing the special case.
2. **Tuned frame numbers.** The 13/18/19 frames and the 5/4-frame playback buffer are empirical for 13.0.5. The Eden rows above must confirm them; if a fighter-specific ledge-grab animation differs, record it.
3. **Wait never times out.** With Wait the CPU hangs until the game forces a drop, and the climb denial persists; confirm this is the intended drill (it matches upstream). **Decided 2026-11-01 (B8 batch, kept line): kept as the fork does it.**
4. **Per-fighter state.** Upstream kept one case/delay for all CPU fighters; this spec asks for per-fighter state (Ice Climbers). Confirm. **Decided 2026-11-01 (B6): as recommended;** one state per fighter.
5. **Transition-term hook cost.** `is_enable_transition_term` is one of the most frequently called game functions; the hook must do nothing beyond the original call unless the fighter is CPU-operated and hanging. Measure the per-frame cost on Eden (diag counter) and consider denying the climb through a cheaper site if it is significant.
6. **Ledge options changed while the CPU is already on the ledge (B11, added 2026-11-01; applies to the fork too, where it is unchanged).** A CPU rolls its ledge option once per hang and holds it until it leaves the ledge, and the held roll also survives while the Ledge Options row is empty. A CPU already parked on the ledge (for example with Wait) therefore ignores a newly selected option until it leaves the ledge and grabs again; the throw option behaves the same for a CPU already holding someone (SPEC-cpu-06). Options: (a) re-roll the ledge options (and the throw options) when the menu changes them, like SDI Direction, so the CPU takes the new option during the same hang; (b) apply on the next ledge grab, which the fork did. Recommendation: (a), the same rule as menu question 18; cost: one more roll cleared on a menu change, and a CPU already hanging can act differently the moment the row changes. **Decided 2026-11-02 (B11): (a), as recommended,** for the ledge options and the throw options (SPEC-cpu-06 question 9), in both trees: the fork gets it in its next build, the clean room implements it from §5 and SPEC-cpu-06. Source: the fork's `c1162/di-sdi-apply/PROGRESS.md`, *For the primary* item 2.
