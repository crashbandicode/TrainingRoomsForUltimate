# CPU shield behaviour: hold/infinite/constant shield, out-of-shield options, OoS offset, reaction time, shield tilt — behavioural specification

- **Spec id:** `SPEC-cpu-05`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/shield.rs`, `src/training/shield_tilt.rs`, the button, param and stick hooks in `src/training/mod.rs`, `src/lib.rs` (common-module hook install), `training_mod_consts/src/options.rs` (`Shield`, `Delay`, `Direction`), `training_mod_consts/src/lib.rs` (Mash and Defensive tabs) at commit `97cbe9b9` (and `077a8e43` where it differs: the shield parameter restore must also run outside training); fork changes at `…/stages/m5i/stage/src/training/shield.rs` lines `34-43, 168-170, 173, 190-193` and `…/training/lobby.rs` (`prepare_cpu_options`) from `docs/provenance/provenance-files.md`
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

Shared definitions: `SPEC-cpu-01` (mash queue, external buffer entry, CPU predicate, predicates "shielding", "in shieldstun", "in parry", "was airborne", "idle", "in hitstun"), `SPEC-cpu-07` (frame counters, reset bundle). The `Direction` list and its angles are in `SPEC-cpu-02` §2.

## 1. Purpose (one paragraph)

The practice CPU holds its shield so the player can practise pressure, with the shield optionally immune to damage or decay; after a chosen number of hits on shield and a chosen reaction time, it acts out of shield with the mash action (or a shieldstun-specific override), including rolls, spot dodges, jumps, aerials, up specials and up smashes that need the shield dropped or cancelled; and it can tilt its shield in a fixed direction.

## 2. User-visible behaviour

### Menu items

| Row | Setting key | Type | Values | Default |
| --- | --- | --- | --- | --- |
| Shield Toggles (Defensive) | `shield_state` | single-select | None (`NONE`), Infinite (`INFINITE`), Hold (`HOLD`), Constant (`CONSTANT`) | None |
| Shield Tilt (Defensive) | `shield_tilt` | single-select | Direction list | none |
| OoS Offset (Mash) | `oos_offset` | **multi-select + range** (draft 9, N3): weighted checklist or a min–max range, one at a time (SPEC-core-02 §2.8) | 0 … 30 (hits); range: any value from min to max, uniform | none (= 0) |
| Reaction Time (Mash) | `reaction_time` | **multi-select + range** (draft 9, N3), same | 0 … 30 frames; range as above | none (= 0) |

Also read: trigger "Shieldstun" in `mash_triggers` (default ticked), `mash_state`, Override row Shieldstun `shieldstun_override`.

Meaning of the shield toggles (upstream README): Infinite = no shield damage and no decay; Hold = the CPU holds shield with no decay until the shield is hit for the first time, then normal decay; Constant = no decay (damage applies); None = the CPU shields only when a mash action asks for it. Help texts: "CPU Shield Behavior"; "Direction to tilt the shield"; "How many times the CPU shield can be hit before performing a Mash option"; "How many frames to delay before performing a Mash option out of shield".

### Where it is live

Training and lobby practice, CPU-operated fighters. Nothing during input playback or standby, nothing while a save state loads, nothing in the air.

### Visible outputs

The CPU's shield, its tilt and its out-of-shield action. No HUD text.

## 3. Inputs

- Menu rows above (live).
- CPU predicate, "shielding", "in shieldstun", "in parry", "idle", "in hitstun", "was airborne" predicates; status and previous status (`FIGHTER_STATUS_KIND_GUARD`, `GUARD_DAMAGE`, `GUARD_OFF`, `GUARD_ON`).
- Transition term `FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_ESCAPE` (a dodge/roll can start now).
- The game's shield parameter reads: `WorkModule::get_param_float(accessor, param_type, param_hash)` with `param_type == hash40("common")` and `param_hash` in {`hash40("shield_dec1")` (shield decay per frame), `hash40("shield_recovery1")` (shield regeneration per frame)}.
- The game's common parameter block (`CommonParams`), field `shield_damage_mul` (float; vanilla 1.19): the pointer is supplied by the param-hook plugin (`libparam_hook`, hook `params_main` → `ParamsInfo::get::<CommonParams>`). If the plugin is absent, the Infinite shield's damage immunity is unavailable and the fact is logged once (fork fix: never panic).
- The game's button queries for the CPU: `ControlModule::check_button_on(accessor, button)` and `check_button_off(accessor, button)` for `CONTROL_PAD_BUTTON_GUARD` and `CONTROL_PAD_BUTTON_GUARD_HOLD`.
- The game's shield-script continuation `L2CFighterCommon::sub_guard_cont()` (runs every frame the fighter is in a guard status and decides shield drop / out-of-shield transitions).
- The game's unclamped stick reads `ControlModule::get_stick_x_no_clamp` / `get_stick_y_no_clamp` (used for shield tilt).
- Mash queue: current action and "request shield" (current action is `SHIELD`, or `AIR_DODGE` while grounded).
- Input-recording interface: is playback, is standby/lockout. Save-state interface: is loading.
- One in-game frame counter (SPEC-cpu-07) for the reaction time.

## 4. Outputs and native state touched

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| Shield decay and regeneration parameters | Replace hook on `WorkModule::get_param_float`, `main+0x208de50` (leaf thunk; shares a window with `main+0x208de30` `get_param_int`, hooked by the save-state spec, and `main+0x208dd60`). Call the original; in training, for a CPU-operated fighter not in playback, when the decay override is active (§5): `common`/`shield_dec1` → 0.0, `common`/`shield_recovery1` → 999.0; everything else → original | every parameter read | training; CPU-operated; not playback |
| Shield damage multiplier (global) | Write `CommonParams.shield_damage_mul` through the param-hook pointer: 0.0 while training is active and `shield_state == INFINITE`; otherwise the value cached from the first observation (the game's boot value). Must run every frame including outside training so leaving Training restores the game value (upstream `077a8e43` and the fork agree) | once per frame (first fighter callback of the frame is enough) | pointer non-null |
| Shield button held | Replace hooks on `ControlModule::check_button_on`, `main+0x2001430`, and `check_button_off`, `main+0x2001440` (adjacent leaf thunks, one window). Call the original; for a CPU-operated fighter in training, for the two guard buttons, when "should hold shield" (§5) and the shield is not suspended: `on` → true; `off` → false unless the current mash action needs a native shield drop (§5), then the original. Other fighters/buttons: original (then SPEC-cpu-06 full hop) | every button query | training; CPU-operated |
| Out-of-shield decision | Replace hook on the common fighter-script function `L2CFighterCommon::sub_guard_cont()` (symbol `_ZN7lua2cpp16L2CFighterCommon14sub_guard_contEv`; installed when the `common` NRO loads; guard against that image). Call the original first, then apply §5 "Out of shield" for a CPU-operated fighter in training | every frame in a guard status | training; CPU-operated |
| Dodge/roll out of shield | `L2CFighterBase::change_status(FIGHTER_STATUS_KIND_ESCAPE / ESCAPE_F / ESCAPE_B as Lua int, true)` on the fighter script object (the hook's argument) | when the current action is a spot dodge or roll and `CONT_ESCAPE` is enabled | as stated |
| Shield tilt | Replace hooks on `ControlModule::get_stick_x_no_clamp`, `main+0x2001600`, and `get_stick_y_no_clamp`, `main+0x2001610` (adjacent leaf thunks). Call the original; for a CPU-operated fighter in training with a tilt direction set, return `cos a` / `sin a` of the tilt angle; else original | every unclamped stick read | training; CPU-operated |

Plugin state per CPU fighter: multi-hit offset, reaction delay, "was in shieldstun" latch, "shield suspended" flag, "decay enabled" flag; global: cached `shield_damage_mul`.

## 5. Timing and frame rules

### Holding the shield

"Should hold shield" is true when: not in playback, not in standby/lockout, no save state loading, the previous situation was not airborne, and (the mash queue requests shield, or `shield_state` is Hold, Infinite or Constant). While it is true and the shield is not suspended, the button hooks report the guard buttons as pressed-and-held, so the CPU shields whenever the game lets it (standing, landing). "Suspended" is set from the current mash action (below) so an out-of-shield action can drop the shield.

### Decay override

Active when `shield_state` is Infinite or Constant, or when it is Hold and the "decay enabled" flag is false. The flag is set true when the CPU's script leaves shieldstun while Hold is selected, and set false (in the once-per-frame dispatch) whenever the status is not `GUARD`; so Hold has no decay until the first hit on this shield, then normal decay until the shield is dropped. With `shield_state == NONE` the override must be inactive (native decay); see open question 1 for upstream's deviation.

### Hits on shield and reaction time

- The multi-hit offset is reset to `oos_offset pick + 1` whenever the CPU is idle (`WAIT`) or in hitstun (once-per-frame dispatch). The `+ 1` exists because the offset is decremented as soon as a shield hit is observed but checked only later in the same frame's script continuation.
- On the first frame of each shieldstun ("was in shieldstun" latch false → true; the latch clears when not in shieldstun): roll the reaction delay from `reaction_time` and decrement the offset once (saturating). Upstream observes this inside the parameter-read hook; any placement that observes the shieldstun entry before the same frame's out-of-shield decision is acceptable (open question 3).
- "Out of shield allowed" = offset is 0.

### Out of shield (script continuation hook, after the original)

For a CPU-operated fighter whose previous status is `GUARD_DAMAGE` (it just left shieldstun this frame):

1. If Hold is selected, set the decay-enabled flag.
2. If out-of-shield is not allowed (offset > 0), stop.
3. If the fighter is not shielding any more, fully reset the reaction counter and stop.
4. If the reaction counter says wait (`should_delay(reaction delay)`), stop.
5. If in parry (motion `just_shield_off`), stop (let the parry resolve).
6. If the Shieldstun trigger is ticked: buffer `shieldstun_override` (non-empty) else `mash_state` through the external entry (SPEC-cpu-01).
7. Take the current mash action:
   - spot dodge / roll forwards / roll backwards with `CONT_ESCAPE` enabled → change the status directly on the script object and stop;
   - jump, any aerial, up special, up smash → stop (the button-off hook will report the shield released natively so the game performs the out-of-shield option);
   - air dodge → if Hold/Infinite/Constant, suspend the shield; stop;
   - shield → stop; if Hold/Infinite/Constant the shield stays held, otherwise it is dropped;
   - anything else → set "suspended" = the action is not `U_SMASH`, `GRAB`, `SHIELD` or empty (grab and up smash work from shield; every other action needs the shield dropped).

The suspension is also re-evaluated for the new front action every time the mash queue pops (SPEC-cpu-01 "completion").

### Shield tilt

The tilt direction is a single selection. **Decided 2026-11-01 (B7): the opponent-relative rule shared with DI and SDI** (SPEC-cpu-02 §2): In is toward the opponent and Out away on both sides, mirrored when the player stands on the CPU's other side; Left and Right stay absolute (open question 2). Upstream re-reads the selection only on the reset bundle (and the fork on a lobby settings change). **Decided 2026-11-01 (menu question 18 A, the fork's M5o rule):** a changed Shield Tilt applies at the next CPU frame, in Training as in lobby practice, and is also re-read by the reset bundle (read live or refresh on every row change; no reset is needed). Empty selection → native stick.

### Ordering

Per frame: the parameter installer (damage multiplier) first in the dispatch; the shield state reset (offset reset when idle/hitstun, decay flag when not guarding) after ledge and before the DI case reset. Button and parameter hooks are answered whenever the game asks.

### Reset and save states

The reset bundle (SPEC-cpu-07) re-reads the shield tilt. A save-state load disables holding while loading (predicate). The fork's session clear resets all per-fighter shield state (offset 0, delay 0, latches false, counter reset) but keeps the cached damage multiplier so the restore still works.

## 6. Edge cases and interactions

- **Ice Climbers**: Popo and Nana both hold shield; state must be per fighter (upstream shared it; open question 4). Nana's shieldstun and OoS run on her own script.
- **Pokémon Trainer**: switch ends the guard; state resets through the idle rule.
- **Kirby, Mii, Aegis, Steve, Olimar, Rosalina**: no special handling (Rosalina's Luma is not a fighter).
- **Parry**: while the parry animation plays, no out-of-shield action is taken.
- **Mash `SHIELD` with `shield_state == NONE`**: the shield is held only while the queue's front action is `SHIELD` (completion at `GUARD_ON` pops it, see SPEC-cpu-01 open question 4).
- **Save states / playback / standby**: shield never held; parameter override inactive during playback.
- **Lobby practice**: the fork re-reads the shield tilt when settings change while practice is active; the damage-multiplier write is admitted in every scope because it restores the native value outside Training.
- **Hot swap**: session clear resets state; a parked body never receives button answers (CPU predicate refuses reserved bodies).
- **Leaving Training**: the multiplier must be restored on the next frame outside training; this is the one CPU-behaviour write that outlives the mode.
- **param-hook plugin missing**: Infinite loses its damage immunity only; log once; decay/regeneration overrides still work.

## 7. Persistence

Nothing is written. Settings live in the menu JSON.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | see docs/resource-costs.md (W5) | per CPU fighter < 24 bytes; one cached float | `tools/build/nro_size.py` per-feature build |
| per-frame work | see docs/resource-costs.md (W5); upstream compares two hashes on every parameter read of the CPU and writes the multiplier every fighter callback | parameter hook: one CPU check and two 64-bit compares only when the override is active; multiplier: one compare per frame, write only on change | diag counters |
| heap allocations in hooks | upstream: one per weighted pick | 0 | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Hold predicate (playback, standby, loading, was-airborne, mash request, toggles) | `cpu05_should_hold_shield` | `cpu-05-hold-shield-stands` | `evidence/cpu-05/hold.md` |
| Decay override per toggle; Hold flips after first hit; None native | `cpu05_decay_override_rules` | `cpu-05-infinite-no-decay`, `cpu-05-hold-decays-after-hit`, `cpu-05-none-native-decay` | `evidence/cpu-05/decay.md` |
| Damage multiplier 0 for Infinite, restored otherwise and outside Training | `cpu05_damage_mul_restore` | `cpu-05-leave-training-restores-mul` | `evidence/cpu-05/mul.md` |
| OoS offset: N hits shielded, then act; reset when idle/hitstun | `cpu05_oos_offset_count` | `cpu-05-oos-offset-2` | `evidence/cpu-05/oos.md` |
| Reaction time counted after shieldstun; parry blocks | `cpu05_reaction_time` | `cpu-05-reaction-10`, `cpu-05-parry-no-oos` | `evidence/cpu-05/oos.md` |
| Each OoS action: roll/spotdodge via status, jump/aerial/upB/usmash via native release, grab/usmash without drop, others with suspension | `cpu05_oos_action_table` | `cpu-05-oos-each-action` | `evidence/cpu-05/actions.md` |
| Shieldstun override vs trigger | `cpu05_shieldstun_override` | `cpu-05-shieldstun-override-nair` | `evidence/cpu-05/mash.md` |
| Shield tilt angle applied immediately, empty → native | `cpu05_shield_tilt` | `cpu-05-tilt-up` | `evidence/cpu-05/tilt.md` |
| **Q18 A** A changed Shield Tilt applies at the next CPU frame with no reset (the fork's R-APPLY rows) | `cpu05_shield_tilt_applies_next_frame` | `cpu-05-tilt-change-no-reset` | `evidence/cpu-05/tilt.md` |
| **B7** Shield Tilt In / Out are opponent-relative on both sides; Left / Right absolute | `cpu05_shield_tilt_opponent_relative` | `cpu-05-tilt-in-left-and-right` | `evidence/cpu-05/tilt.md` |
| **N3** OoS Offset and Reaction Time ranges: a set range is picked uniformly where the checklist is picked today (OoS Offset: at each offset reset while idle or in hitstun; Reaction Time: on the first frame of each shieldstun), a checklist stays a weighted pick; exclusive; a file with no range loads as a checklist | `cpu05_oos_offset_range_pick`, `cpu05_reaction_time_range_pick`, `cpu05_delay_rows_exclusive_and_legacy_load` | `cpu-05-oos-range-1-3`, `cpu-05-reaction-range-5-15` | `evidence/cpu-05/oos.md` |
| Per-fighter state (Ice Climbers) | `cpu05_per_fighter_state` | `cpu-05-ice-climbers` | `evidence/cpu-05/ic.md` |
| Common-module hook guard; param-hook absent path logs once | `cpu05_guards_and_missing_provider` | `cpu-05-install-line` | `evidence/cpu-05/install.md` |

## 10. Open questions

1. **Decay override with Shield None.** In upstream the override condition is satisfied whenever the decay flag is false, which with None is always; so the CPU's shield in Training never decays and regenerates instantly even with Shield Toggles = None. This spec requires None to be native. Confirm with the lead and verify on Eden that the native Training CPU's shield decays again (it is a visible behaviour change against the fork). **Decided 2026-11-01 (B8 batch): accepted.**
2. **Shield tilt convention.** Upstream tilts by absolute angle (Out = right), unlike DI/SDI (opponent-relative). Recommendation: use the opponent-relative rule shared with SPEC-cpu-02, or reduce the row to Up/Down/Left/Right and the four diagonals; decide with the menu redesign. **Decided 2026-11-01 (B7): as recommended;** opponent-relative (the direction grid labels its chips In and Out, so they mean the same on every row).
3. **Offset observation point.** Upstream decrements the hit counter inside the parameter-read hook (the first parameter read of the shieldstun frame). Recommendation: observe shieldstun entry in the script-continuation hook before the out-of-shield decision, which is the only consumer; the `+1` compensation can then go. Needs an Eden row proving "OoS Offset N = exactly N shielded hits".
4. **Per-fighter state.** Upstream shares all shield state between Popo and Nana; this spec asks for per-fighter state. Confirm. **Decided 2026-11-01 (B6): as recommended;** one state per fighter.
5. **Shield mash completion** interacts with holding (SPEC-cpu-01 open question 4).
6. **Param-hook dependency.** The damage multiplier needs `libparam_hook`'s pointer. Alternative: locate `CommonParams` through a guarded needle and drop the plugin dependency. The lead decides whether the dependency stays.
7. **Upstream HEAD `077a8e43`** keeps the multiplier restore outside training (adopted).
