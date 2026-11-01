# Training reset hook and L+R+A reset — behavioural specification

- **Spec id:** `SPEC-state-06`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/reset.rs`, `src/training/mod.rs` (`fighter_vtable_on_init`, `lra_handle`, `training_mods` install), `src/training/frame_counter.rs` (`FrameCounterType`, `reset_all`), `src/common/offsets.rs` (`TRAINING_RESET_CHECK`, `FIGHTER_VTABLE_ON_INIT`), `training_mod_consts/src/lib.rs` (L+R+A Reset row) at commit `97cbe9b9` and `077a8e43` (the latter changes `fighter_vtable_on_init`); fork changes at `…/stages/m5i/stage/src/training/reset.rs` lines 12-34 and `…/stage/src/training/mod.rs` lines 77-109, 342-347, 626-636, 1073-1077 from `docs/provenance/provenance-files.md`
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

Training mode's own L+R+A reset puts both fighters back at their start positions; the plugin lets the player turn that chord off (so it cannot collide with practice button chords) and, whenever the game does reset Training, clears every per-session plugin state that would otherwise leak into the fresh setup: queued mash actions, the random SDI and shield-tilt directions, in-game frame counters, the combo counter, and the ledge and throw delays. The same clearing runs when a save state places a fighter.

## 2. User-visible behaviour

- Menu item (Misc tab): **L+R+A Reset**, setting id `lra_reset`, toggle On/Off, default On. Help: "Reset Training Room when pressing L+R+A".
- With it Off, pressing L+R+A in Training does nothing. With it On, the game's reset behaves as usual, and the plugin's per-session state is cleared along with it.
- No notification, no HUD output.
- The fork adds a waiting-room quick reset on the same chord (lobby practice; carried over, §6.4).

## 3. Inputs

- `lra_reset` (read inside the reset-check hook on every evaluation).
- The game's own Training reset check (the hook sits inside it; the game has already established Training mode there).
- The fighter on-init callback (shared runtime entry; SPEC-state-01 §4.1): the fighter's module accessor and whether it is CPU-operated.
- Fork: whether a hot swap is in progress (reset must not run while the fork replaces a fighter) and whether the scene is native Training.

## 4. Outputs and native state touched

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| Result of the game's L+R+A check | inline hook at `main+0x1378e50` (inside the Training reset check routine, eight instructions after a compare on `w8` and a branch): when `lra_reset` is Off, set register `x8` to 0 so the game's check fails and no reset happens | every evaluation of the check | the game checks Training mode before this point; the fork additionally returns early unless the scene is native Training (folded: cheap and prevents a stray write in other scenes should the site be reached) |
| Plugin per-session state | `on_reset()`: mash full reset, SDI direction re-roll, in-game frame counters reset, combo reset, ledge delay reset, throw delay reset, shield tilt direction re-roll (each owned by its own feature; this spec requires the hub to call them in this order) | on the Training reset of the CPU-operated fighter (§5.1) and on every PosMove frame of a save-state load (SPEC-state-01 §5.3 step 6) | — |
| Log line | "Resetting Training Mode..." at info level | §5.1 | — |

No game memory other than the one register at the hook site is written by this feature.

## 5. Timing and frame rules

### 5.1 Detecting a Training reset

The game re-initialises every fighter on an L+R+A reset through the fighter on-init callback (vtable slot `main+0x4f80240`, original at `main+0x60dfa0`, shared runtime). After the original on-init, if the fighter is CPU-operated (the `FighterInformation::is_operation_cpu` test through the FighterManager, SPEC-state-01 §3) run `on_reset()`. Only the CPU's init triggers it so the hub runs once per reset, not once per fighter. The same callback fires when fighters are first created on entering Training, so the state is also fresh at the start of a session.

Required gating (fork, folded): run only in native Training mode, and not while a fighter replacement (hot swap) is in progress. Upstream `97cbe9b9` runs it unconditionally in every mode (so a CPU being created in any mode cleared Training state, harmless but wasteful); upstream `077a8e43` wraps it in `if !is_training_mode()`, which runs it only **outside** Training and never on a Training reset (open question 1).

### 5.2 `on_reset()` contents and ordering

1. Mash: discard the queued and current mash actions (mash spec).
2. SDI: draw a new random SDI direction (SDI spec).
3. Frame counters: every counter of type "in-game" is set to 0 and stops counting; counters of type "in-game, no reset" and "real" are untouched (frame counter spec).
4. Combo: reset the combo counter and frame advantage bookkeeping.
5. Ledge: reset the ledge delay.
6. Throw: reset the throw delay.
7. Shield tilt: draw a new random shield-tilt direction.

### 5.3 Relationship to save states

Every PosMove frame of a load calls `on_reset()` (SPEC-state-01 §5.3 step 6), so a load behaves like a reset for every feature above; the buff delay counter and the Pokémon switch delay counter are in-game counters and are therefore also reset by a load or an L+R+A (they only begin counting after placement, so this is safe).

### 5.4 Setting changed mid-chord

`lra_reset` is read at the moment the game evaluates the chord; turning it Off while L+R+A are held stops the next evaluation from resetting.

## 6. Edge cases and interactions

1. **L+R+A during a save-state load**: the game's reset re-inits both fighters; the load's markers persist and continue on the re-initialised fighters (they are dead → alive → PosMove). Upstream accepts this. Recommendation: cancel markers on a detected reset (open question 3).
2. **Chord overlap**: a user may bind Save/Load to combinations including L, R or A; with `lra_reset` On both fire. The menu should warn (menu spec).
3. **Robin's Levin Sword** is not refilled by the game's reset (upstream comment); the save state's `levin_recharge` field is the workaround (open question 2).
4. **Fork waiting-room quick reset** (carried over, interface only): on player 1's raw controller, the chord "(L or digital L) and (R or digital R) and A" held, while lobby practice is active and owns both practice fighters, `lra_reset` is On, no plugin menu, macro panel, replay capture or preview is open, requests a quick reset: both practice fighters go through the death/rebirth restore of SPEC-state-01 to the fresh idle spawn pose observed at their first WAIT in this entry lifetime (a temporary pair, never a numbered slot), with damage restored to that observation; the chord's inputs are swallowed and the input-delay queue cleared; if the pair is not yet observed the notification "Quick Reset" / "Wait for both practice fighters to finish loading" (120 frames) is shown. The core must expose `request_quick_reset()` and the temporary-pair accessors.
5. **Fork session clear** (carried over, interface only): on scene departure or practice retirement the fork runs a wider `clear_session_state()` (practice lease, hats, items, menu, chords, input delay, input log, combo, shield, buffs, mash, ledge, throw, pummel, frame counters, save-state markers, quick reset, reset latch, notifications, named macros, setup library, preview, playback). The reset hub of §5.2 is the Training-time subset; the session clear is the superset the fork calls.
6. **CPU-as-P1** (fork): "CPU-operated" is read from the game's FighterInformation, so when P1 is under CPU control both fighters would trigger the hub; harmless (idempotent) but twice.
7. **Extra Training CPUs**: each CPU's init triggers the hub; idempotent.

## 7. Persistence

`lra_reset` in the menu file (carried over). Nothing else.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | see docs/resource-costs.md (W5) | minimal | `tools/build/nro_size.py` per-feature build |
| per-frame work | see docs/resource-costs.md (W5) | 0 (event-driven) | diag counters |
| heap allocations in hooks | 0 | 0 | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| `lra_reset` Off → register cleared; On → untouched | `state06_lra_register_rule` | `E-state-06-lra-off-no-reset` | |
| Hub runs once per reset (CPU init), in native Training only, not during swap | `state06_hub_trigger_rules` | `E-state-06-reset-clears-mash` | |
| Hub order and contents | `state06_hub_order` | — | |
| In-game counters reset, no-reset and real counters untouched | `state06_counter_types` | — | |
| Load calls the hub every PosMove frame | `state06_load_calls_hub` | `E-state-06-load-resets-delays` | |
| Hub idempotent when called twice in a frame | `state06_hub_idempotent` | — | |

## 10. Open questions

1. Upstream HEAD `077a8e43` changed the on-init wrapper to run the reset check only when **not** in Training mode (`if !is_training_mode()`), which looks inverted (the companion change gates the per-frame body on `is_training_mode()`). Recommendation: implement "native Training only" as the fork does and tell upstream.
2. Robin's Levin Sword recharge is not refilled by the game's L+R+A; upstream suggests forcing it full "if we find Robin's reset function". Recommendation: out of scope; the save-state field covers the practical case.
3. A Training reset during a load leaves the load markers set. Recommendation: the hub should also cancel pending save-state markers (`cancel_pending()`), since the game has already moved both fighters.
4. The reset-check needle in upstream matches eight instructions at `main+0x1378e50`; the fork pins the same address. Confirm the inline hook's register (`x8`) is the compare operand on the owned binary before freezing the site table.
