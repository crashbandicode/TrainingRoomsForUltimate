# CPU defensive behaviour: DI, SDI, air-dodge direction, clatter mashing, dodge staling — behavioural specification

- **Spec id:** `SPEC-cpu-02`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/directional_influence.rs`, `src/training/sdi.rs`, `src/training/air_dodge_direction.rs`, `src/training/clatter.rs`, the dodge-staling and stick-hook parts of `src/training/mod.rs`, `src/lib.rs` (install of the common-module hook), `training_mod_consts/src/options.rs` (`Direction`, `SdiFrequency`, `ClatterFrequency`, `OnOff`), `training_mod_consts/src/lib.rs` (Defensive tab) at commit `97cbe9b9` (and `077a8e43` where it differs); fork changes: these five upstream files are upstream-identical in the stage; fork-only context in `…/stages/m5i/stage/src/training/lobby.rs` (`prepare_cpu_options`, fork-owned) and `…/training/mod.rs` lines `289-296` from `docs/provenance/provenance-files.md`
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

Shared definitions (CPU predicate, situation predicates, weighted picks, the per-frame dispatch order) are in `SPEC-cpu-01` §3 and §5 and are not repeated.

## 1. Purpose (one paragraph)

The practice CPU influences its own launch (DI), shifts itself during hitlag (SDI), angles its air dodge out of hitstun, mashes out of grabs, buries, freezes and similar holds at a chosen rate, and optionally ignores the game's dodge-staling penalty. All of these let the player practise against a defender who escapes or drifts the way a real opponent would.

## 2. User-visible behaviour

### Menu items

The `Direction` option list, used by DI, SDI and Airdodge Direction (and Shield Tilt in SPEC-cpu-05):

| Display name | Internal | Angle (radians, before mirroring) |
| --- | --- | --- |
| Out | `OUT` | 0 |
| Up Out | `UP_OUT` | π/4 |
| Up | `UP` | π/2 |
| Up In | `UP_IN` | 3π/4 |
| In | `IN` | π |
| Down In | `DOWN_IN` | 5π/4 |
| Down | `DOWN` | 3π/2 |
| Down Out | `DOWN_OUT` | 7π/4 |
| Neutral | `NEUTRAL` | none (stick at rest) |
| Left | `LEFT` | π, absolute (never mirrored) |
| Right | `RIGHT` | 0, absolute (never mirrored) |

"Out" and "In" are relative to the opponent: an angle `a` is mirrored to `π − a` when the player entry's x position is greater than the CPU's x position (the player stands to the CPU's right) and the direction is not Left/Right. The stick vector is `(cos a, sin a)`. An empty row (nothing selected) means "do not interfere".

| Row (Defensive tab) | Setting key | Type | Values | Default |
| --- | --- | --- | --- | --- |
| DI Direction | `di_state` | multi-select, weighted | Direction list | none |
| SDI Direction | `sdi_state` | multi-select, weighted | Direction list | none |
| SDI Strength | `sdi_strength` | single-select | None, Normal, Medium, High | None |
| Airdodge Direction | `air_dodge_dir` | multi-select, weighted | Direction list | none |
| Clatter Strength | `clatter_strength` | single-select | None, Normal, Medium, High | None |
| Dodge Staling | `stale_dodges` | on/off | On, Off | On |

Help texts (upstream, may be rewritten): DI "Direction to angle the directional influence during hitlag"; SDI "Direction to angle the smash directional influence during hitlag"; SDI Strength "Relative strength of the smash directional influence inputs"; Airdodge "Direction to angle airdodges"; Clatter "Configure how rapidly the CPU will mash out of grabs, buries, etc."; Dodge Staling "Controls whether the CPU's dodges will worsen with repetitive use (Note: this setting can cause combo behavior not possible in the original game)".

Strength to period (one input every N frames): SDI None → never, Normal → 8, Medium → 6, High → 4. Clatter None → never, Normal → 8, Medium → 5, High → 2. (Upstream's README text says Medium 6 / High 4 for clatter; the code uses 5 / 2; the code values are the behaviour to reproduce, see open question 6.)

### Where it is live

Training mode and lobby practice, for CPU-operated fighters only. No chords. Visible outputs: the CPU's trajectory, its SDI steps with the game's own `sys_clatter` effect on each simulated clatter input, and no HUD text.

## 3. Inputs

- Menu rows above, read live.
- CPU predicate and the player/CPU accessors (SPEC-cpu-01 §3).
- Positions (`PostureModule::pos_x`) of the player and CPU entries for the mirroring rule.
- Status kind, situation kind (airborne), "in hitstun" and "in clatter" predicates (SPEC-cpu-01 §3).
- For the air dodge direction's alternative mirroring rule (open question 2): the CPU's damage-launch x speed, `KineticEnergy::get_speed_x(KineticModule::get_energy(accessor, FIGHTER_KINETIC_ENERGY_ID_DAMAGE))`.
- Clatter: the `manual_recovery_rate` argument of the game's clatter start call (how many clatter-time units one real input removes; 8 for most holds, less for some, e.g. freeze 4).
- Hitlag SDI query: the game's "check hit-stop delay command" call, which asks the control module whether an SDI input happened this hitlag frame and where.
- Reset bundle and settings-change events (SPEC-cpu-07): SDI direction re-rolls there.

## 4. Outputs and native state touched

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| DI stick floats of the damage status | Replace hook on the common fighter-script function `L2CFighterCommon::FighterStatusDamage__correctDamageVectorCommon(fighter, value)` in the `common` script module (symbol `_ZN7lua2cpp16L2CFighterCommon46FighterStatusDamage__correctDamageVectorCommonEN3lib8L2CValueE`; the module is loaded at run time, so the hook is installed when the `common` NRO loads and its 64-byte guard is checked against that image, not `main`). Before calling the original: when training is active, `di_state` is non-empty and the fighter (accessor from the script's Lua state) is CPU-operated, write `WorkModule::set_float(accessor, x, FIGHTER_STATUS_DAMAGE_WORK_FLOAT_VECOR_CORRECT_STICK_X)` and `…_STICK_Y` with the held DI vector (`(0,0)` for Neutral). Then call the original, which reads those floats | each time the game corrects a damage vector (start of knockback, once per hit) | training; CPU-operated; row non-empty |
| SDI input and direction during hitlag | Replace hook on `FighterControlModuleImpl::check_hit_stop_delay_command(accessor, out: *mut Vector2f) -> u64`, `main+0x20aeda0` (13.0.5). For a CPU-operated fighter in training: run the period counter; on an input frame with a direction, write `out.x = cos a`, `out.y = sin a`, return 1; otherwise return 0. Non-CPU or outside training: call the original | every hitlag frame of the CPU | training; CPU-operated |
| Air-dodge stick | Replace hooks on `ControlModule::get_stick_x(accessor)` `main+0x2001260` and `ControlModule::get_stick_y(accessor)` `main+0x2001280`. Call the original; for a CPU-operated fighter that is airborne and in hitstun, with `air_dodge_dir` non-empty, return `cos a` / `sin a` of the held air-dodge direction; otherwise the original (the y hook then falls through to Crouch, SPEC-cpu-06) | every stick read | training; CPU-operated; airborne and in hitstun |
| Clatter step | Replace hook on `ControlModule::start_clatter(accessor, initial_time: f32, auto_recovery_rate: f32, manual_recovery_rate: f32, i8, i32, bool, bool) -> u64`, `main+0x2001080`. Remember `manual_recovery_rate` as the step for CPU-operated fighters in training (initial value 8.0); always call the original | start of each clatter situation | training; CPU-operated |
| Clatter time | `ControlModule::add_clatter_time(accessor, -step, 0)` | on each simulated input frame while in a clatter status | training; CPU-operated; in clatter |
| Clatter effect | `EffectModule::req_on_joint(accessor, Hash40("sys_clatter"), Hash40("hip"), &zero, &zero, 1.0, &zero, &zero, true, EFFECT_SUB_ATTRIBUTE_NO_JOINT_SCALE \| EFFECT_SUB_ATTRIBUTE_FOLLOW \| EFFECT_SUB_ATTRIBUTE_CONCLUDE_STATUS, 0, 0)` | same frame as the clatter time change | same |
| Dodge-staling penalty flag | `WorkModule::set_flag(accessor, !stale_dodges, FIGHTER_INSTANCE_WORK_ID_FLAG_DISABLE_ESCAPE_PENALTY)` | see §5 | training; CPU-operated |

The two `get_stick_*` thunks and the clatter thunk are four-instruction leaf thunks; `main+0x2001260`/`0x2001280` lie within one 64-byte window of each other, and `main+0x2001030`/`0x2001080` share a window with other hooked thunks (`get_stick_dir`, SPEC-cpu-06). Check every guard before installing any of them.

## 5. Timing and frame rules

### DI

- One DI case per hitstun episode: on the first damage-vector correction while no case is held, pick from `di_state`; keep it while the CPU stays in hitstun. In the once-per-frame dispatch (after the fighter's frame), clear the held case whenever the CPU is **not** in hitstun. So every hit while still in hitstun keeps the same DI; a hit from neutral rolls anew.
- Mirroring is evaluated at the moment of the correction (the player's position then).
- Neutral: write `(0, 0)` (an explicit no-DI, overriding the native CPU's DI). Empty row: do not write anything (native CPU DI).

### SDI

- Period counter per CPU: on every hitlag query, `counter = (counter + 1) mod period`; the input frame is when `counter == period − 1`. With Strength None the period is effectively infinite (never inputs). The counter is reset to 0 whenever the direction is rolled.
- Direction roll: **once per hitlag episode** (first hitlag query after a frame in which the CPU was not in hitstun), plus the reset bundle and a menu save (SPEC-cpu-07). Upstream rolled only on reset and save-state load (its own comment marks this as a bug); see open question 1.
- Mirroring as for DI, evaluated on the input frame.
- Empty `sdi_state` with a non-None strength: no direction → return 0 (no SDI). Non-empty direction with Strength None: never inputs. Note that for a CPU-operated fighter in training the hook **replaces** the native answer (returns 0 when it does not input); see open question 3.

### Air-dodge direction

- Roll one direction per aerial hitstun episode (first stick read while airborne in hitstun after a frame outside that condition) and return a consistent `(cos a, sin a)` for every x/y read of that episode. Upstream rolled on every single stick read, so x and y could come from different picks (open question 2).
- The override is active for every stick read while airborne and in hitstun. The game reads the stick when the air dodge starts (and for drift once actionable), so this sets the directional air dodge out of hitstun and does not affect grounded behaviour.
- Mirroring: use the opponent-relative rule shared with DI (open question 2 records upstream's launch-direction variant and asks for a decision).

### Clatter

- Per frame in the once-per-frame dispatch, for a CPU-operated fighter in training whose status is in the clatter list: `counter = (counter + 1) mod period`; when `counter == period − 1`, apply the clatter-time reduction and the effect. Outside clatter the counter is left as is.
- The step defaults to 8.0 until the first clatter start call sets it; it is overwritten at every clatter start for the CPU.
- Leaving clatter is a mash trigger (SPEC-cpu-01: Clatter trigger / `clatter_override`).

### Dodge staling

- The flag must equal `!stale_dodges` for every CPU-operated fighter while training is active. Upstream writes it every frame; the clean-room may write it on fighter (re)initialisation, save-state load, scene entry and whenever the setting changes, provided Eden shows the game does not clear it in between (open question 5). Default On means the flag is false (staling applies, as in the unmodified game).

### Setting changes

Rows are read live. Changing SDI Direction, Airdodge Direction or DI Direction while a case is held does not change the held case; the next episode uses the new rows. A menu save re-rolls the SDI direction (SPEC-cpu-07 settings-change bundle).

### Reset and save states

The reset bundle (SPEC-cpu-07) re-rolls the SDI direction and resets its counter. The DI and air-dodge cases clear by themselves (the CPU is not in hitstun after a load). Clatter state needs no reset.

## 6. Edge cases and interactions

- **Nana** is CPU-operated and gets her own DI/SDI/air dodge/clatter handling with separate per-fighter state (the per-fighter counters and cases must be keyed by fighter, not global; upstream used globals shared by all CPU fighters, so Popo and Nana shared one SDI counter: open question 4).
- **Pokémon Trainer**: a switch ends hitstun, so held cases clear; nothing else.
- **Kirby, Mii, Aegis, Steve, Olimar**: no special handling. **Rosalina/Luma**: Luma is a weapon, never CPU-operated.
- **Captured by Kirby/Yoshi/Wario, DK cargo, Steve minecart** etc. are in the clatter list; upstream's own note says swallowed and cargo-carry statuses use a different recovery path (`set_dec_time`-style) that it does not drive; open question 6.
- **Input playback**: upstream applies DI, SDI and the air-dodge stick override during playback as well (the recording drives buttons and sticks, but the hitstun stick override wins while airborne in hitstun). Open question 7.
- **Save-state load** while in hitstun is not possible (loading forces WAIT/FALL); cases clear.
- **Menu open**: the game is paused; nothing advances.
- **Lobby practice**: entry 4 is the CPU; the fork re-rolls SDI direction (and shield tilt) when settings change while practice is active; the real match never runs these hooks (`is_training_mode` is false outside Training/active lobby practice).
- **Hot swap**: a new body starts with no held case; per-fighter state must be dropped when the body is replaced (fork session clear).
- **CPU-as-P1**: as in SPEC-cpu-01 open question 7.

## 7. Persistence

Nothing is written. Settings live in the menu JSON.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | see docs/resource-costs.md (W5) | per-fighter state: DI case, SDI direction + counter, air-dodge direction, clatter counter + step: < 32 bytes per CPU fighter | `tools/build/nro_size.py` per-feature build |
| per-frame work | see docs/resource-costs.md (W5) | DI: nothing per frame except one "in hitstun" check; SDI: one counter step per hitlag query; air dodge: one predicate per stick read while airborne in hitstun; clatter: one status check per frame; staling: no per-frame write (see §5) | diag counters |
| heap allocations in hooks | upstream: one per weighted pick | 0 | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Direction → angle table, Neutral none, Left/Right absolute | `cpu02_direction_angles` | — | `evidence/cpu-02/host.md` |
| Mirroring when the player is to the CPU's right, never for Left/Right | `cpu02_mirror_rule` | `cpu-02-di-out-both-sides` | `evidence/cpu-02/di.md` |
| DI case held for the whole hitstun, cleared outside hitstun; Neutral writes (0,0); empty row writes nothing | `cpu02_di_case_lifetime` | `cpu-02-di-multihit`, `cpu-02-di-neutral` | `evidence/cpu-02/di.md` |
| SDI period 8/6/4, None never; input frame writes vector and returns 1 | `cpu02_sdi_period` | `cpu-02-sdi-high-up` | `evidence/cpu-02/sdi.md` |
| SDI direction re-rolls per hitlag episode and on reset bundle | `cpu02_sdi_reroll` | `cpu-02-sdi-two-directions` | `evidence/cpu-02/sdi.md` |
| Air-dodge direction: consistent x/y per episode, only airborne in hitstun | `cpu02_airdodge_direction_consistency` | `cpu-02-airdodge-down-out` | `evidence/cpu-02/airdodge.md` |
| Clatter period 8/5/2, step from the start call, effect on input frames | `cpu02_clatter_period_and_step` | `cpu-02-clatter-grab-high`, `cpu-02-clatter-freeze` | `evidence/cpu-02/clatter.md` |
| Dodge staling flag equals !setting; takes effect without a reset | `cpu02_stale_flag_follows_setting` | `cpu-02-dodge-staling-off` | `evidence/cpu-02/staling.md` |
| Per-fighter state for Popo and Nana | `cpu02_per_fighter_state` | `cpu-02-ice-climbers-sdi` | `evidence/cpu-02/ic.md` |
| Common-module hook installs only after its guard matches the loaded image | `cpu02_common_guard_gate` | `cpu-02-install-line` | `evidence/cpu-02/install.md` |

## 10. Open questions

1. **SDI re-roll.** Upstream rolls the SDI direction only on L+R+A reset and save-state load (and, in the fork, on a lobby settings change), so with several directions selected every hit of a combo uses the same one until a reset. Upstream's own source marks this as a bug. This spec requires a roll per hitlag episode. Confirm with the lead; the alternative (keep upstream's behaviour) is a one-line change.
2. **Air-dodge "Out/In" convention.** Upstream mirrors relative air-dodge directions by the CPU's launch x speed, but only on frames where the player is to the CPU's right; when the player is to the left, no mirroring happens at all, so the convention flips between stage sides. This spec uses the opponent-relative rule shared with DI. Alternative: mirror by launch direction on both sides ("Out" = continue along the launch). Decision needed; both are a one-line rule.
3. **SDI hook replaces the native CPU answer.** For a CPU-operated fighter in training the hook never calls the original, so a CPU with SDI Direction empty or Strength None never SDIs even where the native CPU would. Recommendation: call the original when the feature is unset (empty direction or Strength None) and only replace the answer when the player configured SDI.
4. **Shared counters.** Upstream keeps one SDI counter, one clatter counter and one DI/air-dodge case for all CPU-operated fighters, so Popo and Nana share them. This spec requires per-fighter state. Confirm the extra bytes are acceptable (two fighters at most in Training; up to the lobby's practice set otherwise).
5. **Dodge-staling write frequency.** Upstream sets the penalty flag every frame. Writing only on change and on (re)initialisation is cheaper; Eden must confirm the game does not clear the flag during a session (if it does, fall back to the per-frame write).
6. **Clatter coverage and rates.** Upstream notes that swallowed and cargo-carried holds use a different recovery function it does not drive, so Clatter Strength has no effect there even though those statuses are in the clatter list. The README's clatter rates (6/4) also disagree with the code (5/2). Recommendation: keep 8/5/2, state them in the help text, and track the uncovered holds as a follow-up with Eden evidence.
7. **Playback interaction.** The DI, SDI and air-dodge overrides stay active while a recording plays back on the CPU, so a recorded air dodge direction can be overridden. Recommendation: honour the recording (skip the overrides while playback possesses the CPU), consistent with how the shield and stick-direction hooks already yield to playback.
8. **DI write before the original.** The DI hook writes the two stick floats and then runs the game's correction, which may itself overwrite them for a human-controlled fighter; for the CPU the native AI leaves them, so the write survives. If an Eden row shows the native CPU overwriting the floats, the fallback is to write after the original and recompute the corrected vector; record it then.
