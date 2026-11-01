# CPU aerial timing, throws and pummels, attack angle, fast fall, full hop and crouch — behavioural specification

- **Spec id:** `SPEC-cpu-06`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/throw.rs`, `src/training/attack_angle.rs`, `src/training/fast_fall.rs`, `src/training/full_hop.rs`, `src/training/crouch.rs`, the aerial parts of `src/training/mash.rs`, the stick/button hooks and throw call in `src/training/mod.rs`, `training_mod_consts/src/options.rs` (`AttackAngle`, `ThrowOption`, `MedDelay`, `Delay`, `BoolFlag`, `OnOff`), `training_mod_consts/src/lib.rs` (Mash and Defensive tabs) at commit `97cbe9b9` (and `077a8e43` where it differs); fork changes: none in these files (all upstream-identical in the stage); the fork gates the throw call behind its lobby entry filter in `…/stages/m5i/stage/src/training/mod.rs` lines `160-188` from `docs/provenance/provenance-files.md`
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

Shared definitions: `SPEC-cpu-01` (mash queue, buffer entry points, CPU predicate, command-flag hook), `SPEC-cpu-07` (frame counters, reset bundle).

## 1. Purpose (one paragraph)

These settings shape how the CPU executes the actions chosen in SPEC-cpu-01: whether an aerial comes out rising or falling and after how many frames, whether a jump is a full hop or a short hop, whether and when the CPU fast falls, which way angled tilts and smashes are angled, which throw the CPU performs after a grab and after how long, and whether it pummels first; independently, the CPU can be told to crouch whenever it stands still.

## 2. User-visible behaviour

### Menu items

| Row | Setting key | Type | Values | Default |
| --- | --- | --- | --- | --- |
| Attack Angle (Mash) | `attack_angle` | multi-select, weighted | Neutral (`NEUTRAL`), Up (`UP`), Down (`DOWN`) | none |
| Throw Options (Mash) | `throw_state` | multi-select, weighted | None (`NONE`), Forward Throw (`FORWARD`), Backward Throw (`BACKWARD`), Up Throw (`UP`), Down Throw (`DOWN`) | None |
| Throw Delay (Mash) | `throw_delay` | multi-select, weighted | 0, 5, 10, …, 150 frames | none (= 0) |
| Pummel Delay (Mash) | `pummel_delay` | multi-select, weighted | 0, 5, 10, …, 150 frames | none (= no pummelling) |
| Falling Aerials (Mash) | `falling_aerials` | multi-select, weighted | True, False | False |
| Full Hop (Mash) | `full_hop` | multi-select, weighted | True, False | True |
| Aerial Delay (Mash) | `aerial_delay` | multi-select, weighted | 0 … 30 frames | none (= 0) |
| Fast Fall (Mash) | `fast_fall` | multi-select, weighted | True, False | False |
| Fast Fall Delay (Mash) | `fast_fall_delay` | multi-select, weighted | 0 … 30 frames | none (= 0) |
| Crouch (Defensive) | `crouch` | on/off | On, Off | Off |

Help texts (upstream): "For attacks that can be angled, such as some forward tilts"; "Throw to be performed when a grab is landed"; "How many frames to delay the throw option"; "How many frames after a grab to wait before starting to pummel"; "Should aerials be performed when rising or when falling"; "Should the CPU perform a full hop or a short hop when jumping"; "How long to delay an aerial attack"; "Should the CPU fastfall during a jump"; "How many frames the CPU should delay their fastfall"; "Have the CPU crouch when on the ground".

### Where it is live

Training and lobby practice, CPU-operated fighters. Crouch and attack angle apply to whatever the CPU does (including native AI actions); the rest apply to actions driven by the mash queue or to the CPU's grabs.

### Visible outputs

The CPU's movement and attacks; a small flash effect (`sys_smash_flash_s`) when the plugin starts a fast fall, mimicking the game's own fast-fall spark.

## 3. Inputs

- Menu rows above (live).
- CPU predicate; mash queue state (the action being performed; "a menu mash was buffered" events; "an aerial was buffered" events).
- Game state per CPU fighter: status kind, situation kind, motion frame, the transition term `FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_ATTACK_AIR` (an aerial can start now) and `…_CONT_THROW_HI` (a throw can be input now), the flag `FIGHTER_STATUS_WORK_ID_FLAG_RESERVE_DIVE` (fast fall already requested), the vertical gravity speed `KineticModule::get_sum_speed_y(accessor, FIGHTER_KINETIC_ENERGY_ID_GRAVITY)` (negative = falling), position (`PostureModule::pos_x/pos_y`) for the effect.
- The game's queries: `ControlModule::get_stick_dir` (angle for angled tilts/smashes), `get_stick_y` (crouch), `check_button_on/off` for `CONTROL_PAD_BUTTON_JUMP` and `CONTROL_PAD_BUTTON_FLICK_JUMP`, and the category-2 command flags (throws).
- Input-recording interface: is playback. Lobby interface: entry permitted.
- Frame counters (SPEC-cpu-07): aerial delay, fast-fall delay, throw delay, pummel delay (four in-game counters).

## 4. Outputs and native state touched

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| Angle of angled attacks | Replace hook on `ControlModule::get_stick_dir(accessor)`, `main+0x2001030` (leaf thunk; shares a window with `start_clatter` `main+0x2001080`). Call the original; in training, when the fighter is not hanging (situation ≠ cliff) and no playback runs, for a CPU-operated fighter return 1.0 (Up) or −1.0 (Down) per the held attack angle; Neutral/none → original | every stick-direction read | training; not cliff; not playback; CPU-operated |
| Crouch | Replace hook on `ControlModule::get_stick_y(accessor)`, `main+0x2001280` (shared with SPEC-cpu-02 air-dodge direction, which takes precedence). For a CPU-operated fighter with Crouch On whose status is one of `WAIT`, `SQUAT`, `SQUAT_B`, `SQUAT_F`, `SQUAT_RV`, `SQUAT_WAIT`, return −1.0; else original | every stick-y read | training; CPU-operated |
| Jump button held for a full hop | Replace hooks on `ControlModule::check_button_on` `main+0x2001430` / `check_button_off` `main+0x2001440` (shared with SPEC-cpu-05, whose answer takes precedence). For a CPU-operated fighter, for the two jump buttons, when full hop was rolled true, the status is `JUMP_SQUAT` and no playback runs: `on` → true, `off` → false; else original | every button query | training; CPU-operated |
| Fast fall | `WorkModule::set_flag(accessor, true, FIGHTER_STATUS_WORK_ID_FLAG_RESERVE_DIVE)` then `EffectModule::req(accessor, Hash40("sys_smash_flash_s"), &pos, &zero_rot, 2.0, 0, 0, true, EFFECT_SUB_ATTRIBUTE_CONCLUDE_STATUS)` with `pos = (pos_x, pos_y, 0)` | once per fall when the rules in §5 allow | training; CPU-operated |
| Throw command | OR `FIGHTER_PAD_CMD_CAT2_FLAG_THROW_F` / `THROW_B` / `THROW_HI` / `THROW_LW` into the result of the `get_command_flag_cat` hook (`main+0x2001630`, SPEC-cpu-01) for the **category-2** query (open question 1) | when the throw rules in §5 fire | training; CPU-operated; lobby entry permitted |
| Pummel | `StatusModule::change_status_request_from_script(accessor, FIGHTER_STATUS_KIND_CATCH_ATTACK, true)` | while holding, before the throw delay elapses, after the pummel delay | as stated |
| Mash after the throw | SPEC-cpu-01 external buffer entry with a `mash_state` pick | same frame as the throw command | — |

## 5. Timing and frame rules

### Rolls

- When any action is buffered (SPEC-cpu-01 "buffer action", main or follow-up): roll the attack angle from `attack_angle`; if the action is an aerial, roll its aerial delay from `aerial_delay` (empty → 0).
- When a menu mash is buffered (after the queue is filled): roll full hop, fast fall and falling-aerial from their rows (`True`/`False`; an empty row resolves to false).
- Held values persist until the next roll; the attack angle therefore also applies to the CPU's native angled attacks between mashes (open question 4).

### Aerials (used by SPEC-cpu-01's aerial rule)

- Grounded: the jump button bit is produced; with full hop true the attack is withheld until airborne. The full-hop button hold answers the game's jump-button queries during `JUMP_SQUAT` so the jump becomes a full hop; with full hop false the button is not held (short hop).
- Falling aerial true: withhold the attack until "is falling" (gravity y speed < 0).
- Aerial delay: if the held delay is 0 → no wait. If already in `ATTACK_AIR` → no wait. The delay counter counts only on frames where `CONT_ATTACK_AIR` is enabled (frames before that return "wait" without counting); when the count reaches the delay the counter is fully reset and the attack proceeds.
- Completing or resetting the current action clears the aerial delay and its counter (SPEC-cpu-01).

### Fast fall (once-per-frame dispatch, after clatter, before ledge)

For a CPU-operated fighter with fast fall rolled true and airborne:

1. Not yet falling (gravity speed ≥ 0): roll the fast-fall delay from `fast_fall_delay` (empty → 0) and fully reset its counter; return. (So the delay is rolled at the apex, every frame until falling begins.)
2. Falling: proceed only when the status is `FALL` or `ATTACK_AIR` and no playback runs.
3. If `RESERVE_DIVE` is already set, return.
4. If the delay counter says wait, return.
5. Set `RESERVE_DIVE` and show the flash effect.

### Full hop (button hooks)

Only during `JUMP_SQUAT`, only for the two jump buttons, only when the roll is true and no playback runs. The shield hooks (SPEC-cpu-05) answer first; when they return "no opinion" this rule answers.

### Attack angle (stick-direction hook)

Return 1.0 / −1.0 for Up / Down whenever queried (any status), except on the ledge or during playback. Neutral or an empty row → the game's value.

### Crouch (stick-y hook)

For a CPU-operated fighter with Crouch On in one of the six standing/crouching statuses, the stick y reads as −1.0 (full down), so the CPU crouches when idle and stays crouched. Air-dodge direction (SPEC-cpu-02) answers first when it applies.

### Throws and pummels (command-flag hook)

For a CPU-operated fighter:

1. If the status is not `CATCH_WAIT`, `CATCH_PULL` or `CATCH_ATTACK` (not holding anyone): clear the held throw case, throw delay and pummel delay, reset their counters; produce no bits.
2. If `CONT_THROW_HI` is not enabled (cannot throw right now): produce no bits.
3. Roll what is not yet held for this grab: throw delay (`throw_delay`, empty → 0), throw case (`throw_state`), pummel delay (`pummel_delay`).
4. Case None: produce no bits (and keep the case; the CPU neither throws nor pummels).
5. If the throw delay counter says wait: if the pummel delay counter also says wait → nothing; if `pummel_delay` is empty → nothing (no pummelling by default); otherwise, if the status is `CATCH_WAIT`, request `CATCH_ATTACK` (one pummel; the status returns to `CATCH_WAIT` afterwards and the next pummel follows at once since the counter has elapsed). Produce no bits.
6. Throw delay elapsed: produce the case's category-2 flag and buffer a `mash_state` pick through the external entry (the mash after the throw happens regardless of triggers; open question 3).

The three held values are "not set" sentinels until rolled; the throw delay is also reset by the reset bundle (SPEC-cpu-07).

### Setting changes

Live; held rolls persist until their next roll point.

## 6. Edge cases and interactions

- **Ice Climbers**: Popo and Nana both grab and both get throw handling (per-fighter state, open question 5); Nana's native grab release is unaffected.
- **Donkey Kong cargo**: cargo statuses are not in the catch list, so Throw Options do not drive cargo throws (open question 6).
- **Kirby**: inhale is a special, not a grab; no effect. **Mii, Aegis, Steve, Olimar, Rosalina**: no special handling.
- **Pokémon Trainer**: a switch is impossible while holding; nothing special.
- **Tether grabs** (Link, Samus, …): the catch statuses are the same once the grab connects.
- **Playback**: full hop, fast fall and attack angle yield to playback; throws do not check playback but the command-flag hook as a whole is not reached for a possessed CPU in a way that matters (the recording drives category inputs; open question 7).
- **Save states**: the reset bundle resets the throw delay; the held case clears when the grab ends.
- **Lobby practice**: the throw bits are produced only for permitted entries (0 and 4), as the whole command-flag hook is.
- **Hot swap**: session clear resets held values.

## 7. Persistence

Nothing is written. Settings live in the menu JSON.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | see docs/resource-costs.md (W5) | per CPU fighter: attack angle, full hop, fast fall, falling aerial, aerial delay, fast-fall delay, throw case, throw delay, pummel delay, four counter indices: < 48 bytes | `tools/build/nro_size.py` per-feature build |
| per-frame work | see docs/resource-costs.md (W5) | fast fall: one situation and one speed read while airborne; throws: one status read per category query; stick/button hooks: one comparison when the fighter is not CPU-operated | diag counters |
| heap allocations in hooks | upstream: one per weighted pick | 0 | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Roll points: angle + aerial delay per buffered action; full hop / fast fall / falling aerial per menu mash | `cpu06_roll_points` | — | `evidence/cpu-06/host.md` |
| Aerial delay counts only when the aerial transition is enabled; 0 and in-air exceptions | `cpu06_aerial_delay_rules` | `cpu-06-aerial-delay-10` | `evidence/cpu-06/aerial.md` |
| Falling aerial waits for negative gravity speed | `cpu06_falling_aerial` | `cpu-06-falling-nair` | `evidence/cpu-06/aerial.md` |
| Full hop holds jump during jump squat only; short hop otherwise | `cpu06_full_hop_hold` | `cpu-06-full-hop`, `cpu-06-short-hop` | `evidence/cpu-06/jump.md` |
| Fast fall: delay rolled at apex, flag + effect once, only in FALL/ATTACK_AIR, not during playback | `cpu06_fast_fall_rules` | `cpu-06-fast-fall-delay-5` | `evidence/cpu-06/fastfall.md` |
| Attack angle: Up/Down values, not on ledge, not during playback | `cpu06_attack_angle` | `cpu-06-ftilt-up` | `evidence/cpu-06/angle.md` |
| Crouch in the six statuses only; air-dodge direction wins | `cpu06_crouch_statuses` | `cpu-06-crouch` | `evidence/cpu-06/crouch.md` |
| Throw: case/delays once per grab; None does nothing; pummel timing; throw bits on category 2; mash after throw | `cpu06_throw_state_machine` | `cpu-06-uthrow-delay-30-pummel-10`, `cpu-06-throw-none` | `evidence/cpu-06/throw.md` |
| Per-fighter state (Ice Climbers) | `cpu06_per_fighter_state` | `cpu-06-ice-climbers-grab` | `evidence/cpu-06/ic.md` |

## 10. Open questions

1. **Throw bits and the query category.** Upstream ORs the category-2 throw flags into the result of every category query (1 to 4) while the throw rule fires. The throw flags are category-2 bits, so on the category-1 query they are read as unrelated category-1 commands for that frame. This spec delivers them on the category-2 query only; an Eden row must confirm the throw still comes out on the first possible frame (if the game reads category 2 before the plugin's rule fires in the frame, deliver on both 1 and 2 and record it).
2. **Pummel speed.** One pummel is requested per frame in `CATCH_WAIT` once the pummel delay has elapsed; the game's own pummel cooldown limits the rate. Confirm the resulting rhythm is acceptable or whether a repeat interval is wanted.
3. **Mash after the throw ignores triggers.** Upstream always buffers a `mash_state` action after a throw, even with no trigger ticked. Recommendation: keep it (the throw is itself the trigger) but say so in the help text; or add a "Throw" entry to Mash Triggers in the menu redesign.
4. **Attack angle persists.** The held angle applies to the CPU's native angled attacks until the next mash buffers. Recommendation: clear the angle when the queue empties, so only mash-driven attacks are angled.
5. **Per-fighter state.** Upstream shares all of this state between CPU fighters (Ice Climbers). Confirm per-fighter state.
6. **Cargo throws.** Donkey Kong's cargo hold is not driven. Follow-up with Eden evidence if wanted.
7. **Throws during playback.** The throw rule does not check playback. Recommendation: yield to playback like the other rules (decide with SPEC-cpu-02 question 7).
8. **Full Hop default True.** With the default a mash `JUMP` is always a full hop; confirm the default for the redesigned menu.
