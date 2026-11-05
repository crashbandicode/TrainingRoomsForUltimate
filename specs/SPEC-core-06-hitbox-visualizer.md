# Hitbox visualizer — behavioural specification

- **Spec id:** `SPEC-core-06`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), spec-writer role, 2026-11-01
- **Upstream reference read (spec writer only):** `src/hitbox_visualizer/mod.rs` at commit `97cbe9b9` and `077a8e43` (differs by +4 lines: an early return when not in Training mode at the top of the per-frame function), `src/training/mod.rs` (the per-frame call site; `077a8e43` +8/−1), `training_mod_consts/src/lib.rs` (`hitbox_vis` default), `src/static/img/hitbox_vis.svg` (icon, SPEC-core-07); fork changes at stage M5i `src/hitbox_visualizer/mod.rs` lines `224-225, 272, 328, 354-367` and the seam rows `hitbox_visualizer.*` in `training_mod_consts/src/native_seams/rows_a3.rs`, from `docs/provenance/provenance-files.md`.
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

With the setting on, every active hitbox of a fighter in Training is drawn as a translucent coloured sphere (or a chain of spheres for an extended, capsule-shaped hitbox) that follows the bone it is attached to, coloured by hitbox id so that overlapping hitboxes can be told apart; grab boxes are drawn the same way with shifted colours. While the visualizer is on, the fighter's ordinary visual effects are silenced so the spheres stay readable, except around grabs, tech situations and knockdown where the game's own effects carry information the player needs.

## 2. User-visible behaviour

- Menu item **Hitbox Visualization** (`hitbox_vis`, Misc tab, single choice On/Off, default **Off**). Upstream help: "Display a visual representation for active hitboxes (hides other visual effects)". Fork help (route B, 2026-11-04): "On: hitboxes, grab boxes and hurtboxes are drawn."
- Button chords: none.
- Visible output when On, in Training only:
  - For each fighter, each frame, each of the fighter's eight attack slots that is active is drawn with the game's `sys_shield` effect (a translucent sphere) attached to the hitbox's bone at the hitbox position, scaled to `size × 19/200`, tinted with the colour for `id mod 8`, playing at 8× rate so the effect does not pulse. Hitboxes with a second centre (extended/capsule) are drawn as `n` spheres evenly spaced along the segment, `n = ceil(distance / (size × 1.75)) + 1` clamped to 2..16.
  - Grab boxes (`CATCH` animation commands) are drawn when they are created, with the colour for `(id + 3) mod 8`.
  - The frame a hitbox is created by the attack animation command, it is drawn immediately from the command's parameters (so the first active frame is visible even before the per-frame pass).
  - When a grab ends (rebound cleared) and the fighter is not shielding, the spheres are hidden and killed.
  - While On and the fighter is not in a grab status range and not shielding, the fighter's animation-command effects are put to sleep (no hit sparks, dust, etc.), except that effects stay on in the grab/capture statuses, the tread (footstool) statuses, and the `DOWN` knockdown status (so missed-tech and footstool effects remain). Turning the setting Off restores effects the next frame.
- Colour table (RGB tint multipliers; at least one component is 1.0 so the tint is fully saturated):

| id mod 8 | tint (r, g, b) | approximate colour |
| --- | --- | --- |
| 0 | 1.0, 0.0, 0.0 | red `#ff0000` |
| 1 | 1.0, 0.4, 0.0 | orange `#ff9900`-ish (upstream comment says `#ff9900`; the components give `#ff6600`) |
| 2 | 0.8, 1.0, 0.0 | yellow-green `#ccff00` |
| 3 | 0.2, 1.0, 0.2 | green `#33ff33` |
| 4 | 0.0, 0.8, 1.0 | sky blue `#00ccff` |
| 5 | 0.4, 0.4, 1.0 | blue `#6666ff` |
| 6 | 0.8, 0.0, 1.0 | purple `#cc00ff` |
| 7 | 1.0, 0.2, 0.8 | pink `#ff33cc` |

### 2a. Rendering: own orbs for fighters (decision HB-OWN-ORBS, option 1, operator 2026-11-05)

The operator decided "go with discs for all" after the A/B (`docs/decisions/HB-OWN-ORBS-ab.png`, `docs/DECISIONS.md`
HB-OWN-ORBS): with two Ridleys the game's shared effect pool refused 22 of one fighter's 55 spheres on every pass; own
orbs do not use that pool. This supersedes the upstream effect rendering for **fighters** (hit, hurt and grab spheres):

- The sphere set (§2 colours, capsule sampling, hurtboxes of decision HB1, grab boxes) is unchanged; the per-frame pass
  keeps each fighter's spheres (`HandleSet`, ruling Q3) through an own-orb sink that never refuses and ends an orb when the
  fighter's status changes (the effect's `CONCLUDE_STATUS` rule).
- Each held sphere is placed on its bone with `ModelModule::joint_global_position_with_offset(…, false)` and published
  per fighter in fixed storage. The flag is meant to make the call a read (no model update first); the guarded thunk
  passes it to the implementation behind ModelModule's vtable slot `0x158`, which is not read yet (review HB-3: a
  follow-up Eden tracepoint reads that slot's target).
- The layout tick projects every orb with the game's own camera (`world_to_screen`'s projection, render camera) and draws
  it as one of the plugin's disc panes (one plugin-drawn disc texture tinted per colour, size from the projected radius).
  The panes' container is RootPane's first child, so the HUD, the vanilla pause menu and the plugin's menu draw over the
  orbs as they drew over the 3D effects; nothing is hidden for the menus.
- **How many (review HB-2, 2026-11-05):** a fighter holds at most 128 spheres (`MAX_HELD`; more are counted as dropped,
  as before). The layout has 512 orb panes: Training's most fighter objects, Player 1 and the CPU each possibly Ice
  Climbers with Nana (four), times 128. More orbs than panes happen only while a gone fighter's last pass is still on the
  board (a few frames). Then the cut is deterministic: every hitbox and grab box takes a pane before any hurtbox, each in
  board-slot order, and the log says once `[Hitbox] more orbs than panes`.
- When the practice ends while its layout stays (the waiting room after its request ends; Training's layout outside
  Training), the tick hides every orb it showed and the container (review HB-1).
- The discs are flat and drawn over the stage and the fighters (no depth hiding); the effect spheres' glow extended past
  each sphere's radius, a disc stops at it.
- **Known limitation, camera lag:** while the camera pans, the discs trail the spheres' true screen position by about one
  to two frames of motion (measured in Eden run `tr-p23-hitbox-ab2`); still, they coincide. Projecting from a render-side
  camera was not attempted (no cheap, guarded site).
- **Articles** (projectiles) keep the `sys_shield` effect spheres drawn at their `ATTACK` command (§6, Q1).
- A failed guard of any row of the feature's closure leaves the whole feature uninstalled (install line
  `hitbox=not_installed site=<row>`): nothing is drawn, no partial path. A layout without the orb panes draws nothing and
  says so once (`[Hitbox] orb panes missing`).

## 3. Inputs

- `hitbox_vis` setting; `shield_state` setting (see §6, the shield-damage rewrite that lives in the same hook).
- Per fighter per frame: `StatusModule::status_kind`, `is_shielding` (status in `GUARD_ON..=GUARD_DAMAGE`), `AttackModule::is_attack(slot, false)` and `AttackModule::attack_data(slot, false)` for slots 0..7 (fields used: `x, y, z` centre, `x2, y2, z2` second centre or zero, `size`, `node` bone hash).
- At the `ATTACK` animation command (Lua stack, 1-based indices): `1` id (int), `3` joint (hash40), `9` size (float), `10..12` x, y, z, `13..15` x2, y2, z2 (float or absent); `4..8` are damage, angle, KBG, FKB, BKB (read upstream, unused); index `20` is the shield damage (rewritten by the shield feature, §6). The attack command has 36 parameters.
- At the `CATCH` animation command: `1` id, `2` joint, `3` size, `4..6` x, y, z, `7..9` x2, y2, z2 (optional).
- The battle-object module accessor of the Lua state (`sv_system::battle_object_module_accessor`).
- `is_training_mode()` (fork: `practice::visualizes(module_accessor)`, per-fighter: in lobby practice only the practice fighters are visualized — fork carry-over hook).

## 4. Outputs and native state touched

| Native state | How it is reached (`main+0x…`, 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| `ATTACK` animation command | replace hook at `main+0x2284b20` (`sv_animcmd::ATTACK`, entry); the hook runs the plugin part first (Training only), then the original | every hitbox creation | 64-byte guard (fork row `hitbox_visualizer.handle_attack`), chain if the entry is owned |
| `CATCH` animation command | replace hook at `main+0x22ac290` (`sv_animcmd::CATCH`, entry); plugin part then original | every grab-box creation | 64-byte guard (`hitbox_visualizer.handle_catch`) |
| `GrabModule::set_rebound` | replace hook at `main+0x2027bb0` (16-byte lua_bind thunk; the neighbouring thunks sit inside its 64-byte window) | every rebound change | 64-byte guard (`hitbox_visualizer.handle_set_rebound`), window checked before any install |
| Effect spawn | `EffectModule::req_follow(accessor, hash "sys_shield", bone hash, pos, zero rot, size×0.095, follow=true, attributes NO_JOINT_SCALE \| FOLLOW \| CONCLUDE_STATUS, 0, 0, 0, 0, true, true)` then `set_rgb_partial_last(r,g,b)` and `set_rate_last(8.0)` | per sphere per frame (per-frame pass) and at creation (command hooks) | called game functions (no hook) |
| Effect removal | `EffectModule::set_visible_kind(accessor, "sys_shield", false)` then `kill_kind(accessor, "sys_shield", false, true)` | every frame before redrawing; on rebound cleared | called |
| Effect sleep | `MotionAnimcmdModule::set_sleep_effect(accessor, bool)`: `false` at the start of every per-frame pass (restores), `true` when visualizing outside the exempt statuses | per fighter per frame | called |
| Own orbs: bone position (§2a) | `ModelModule::joint_global_position_with_offset` lua_bind thunk `main+0x2057640`, last argument `false` | per held fighter sphere per pass | 64-byte guard (`orb_joint_position`) |
| Own orbs: projection (§2a) | the projection behind `sv_camera_manager::world_to_screen`, `main+0x1341280` (`(out_x*, out_y*, pos&, false)`), and its screen-size words read at `main+0x13415f0` | per orb per layout tick | 64-byte guards (`orb_world_to_screen`, `orb_screen_size`) |
| Own orbs: panes (§2a) | the plugin's own `TrOrbs` container and `tr_oa_*`/`tr_ob_*` pictures in `info_training` / `info_waiting` (position, size, colour, visibility) | per layout tick | the layout recipe digest |
| Lua stack rewrite (shield feature) | pop the 36 parameters, push them back with index 20 replaced by `-999.0` when `shield_state == Infinite` | at `ATTACK`, Training only | — (SPEC-cpu-* shield owns the behaviour) |

No memory is written outside these calls.

## 5. Timing and frame rules

- **Per fighter, per frame** (from the `on_frame` driver, SPEC-core-01 §5.2, after combo and before save states): (1) `set_sleep_effect(false)`; (2) if Off → stop; (3) if status in `CATCH..=CATCH_TURN` → stop; (4) if shielding → stop; (5) `set_sleep_effect(!(status in CATCH..=TREAD_FALL) && status != DOWN)`; (6) hide and kill `sys_shield`; (7) for each active attack slot spawn the spheres. The newest upstream (`077a8e43`) adds (0) if not Training → stop **before** step (1), so a non-Training scene never has its effect sleep flag touched by the plugin; required.
- **At hitbox creation** (`ATTACK` hook): draw once from the command parameters before the original runs, Training only. The per-frame pass redraws (and first kills) on the following frames.
- **At grab-box creation** (`CATCH` hook): draw once; grab boxes are not redrawn per frame (the per-frame pass only reads attack slots), so they persist as `CONCLUDE_STATUS` effects until the status ends or the rebound hook kills them.
- **Rebound cleared**: hide and kill `sys_shield` unless shielding.
- The spheres are respawned every frame (kill + spawn), so there is no persistent effect handle to manage; the cost is proportional to the number of active hitboxes.
- A setting change applies on the next per-frame pass; Off restores effects immediately through step (1).

## 6. Edge cases and interactions

- **Projectiles and articles**: the per-frame pass runs for fighters only (the fighter vtable driver), so projectile hitboxes are not redrawn per frame; they are drawn once when their `ATTACK` command runs (any battle object executing the command, since the hook reads the Lua state's accessor) and then follow their bone with `CONCLUDE_STATUS` until the article's status ends. Multi-hit projectiles therefore keep stale spheres between hits. Q1.
- **Shielding fighter**: nothing is drawn and effects are not slept (the shield visual is itself a `sys_shield` effect; killing it would remove the player's shield bubble — this is why shielding is exempt).
- **Grab statuses**: no drawing, no sleeping, so the grab animation effects show; grab boxes drawn at `CATCH` persist until rebound.
- **Missed tech / footstool**: effects kept awake (`DOWN`, tread statuses) so those visual cues remain.
- **Kirby copy, Pokémon Trainer switch, Ice Climbers**: each fighter body runs the pass independently; Nana's hitboxes are drawn too. Pokémon Trainer's trainer object is not a fighter accessor in the driver.
- **Mii fighters, Aegis, Steve**: no special handling upstream.
- **Save-state load**: the save-state feature kills effects independently (SPEC-state-*); the visualizer redraws next frame.
- **Menu open**: unaffected (the game keeps running underneath upstream; spheres keep drawing).
- **Lobby practice (fork)**: `visualizes(accessor)` gates per fighter so opponents' hitboxes in a waiting room are never drawn and their effects never slept — carry over the gate hook.
- **CPU-as-P1 / hot swap (fork)**: `is_training_mode()` false during a swap → nothing drawn that frame.
- **Shield-damage rewrite inside the `ATTACK` hook**: a different feature (Infinite shield) piggybacks on this hook; it pops and re-pushes 36 Lua values on every attack command while Infinite is selected. Record as Q2 (ownership and cost).
- **Effect budget**: the game's effect pool is finite; 8 slots × up to 16 spheres × fighters, respawned every frame, can exhaust it and drop other effects. Q3. Fork: resolved for fighters by the own orbs (§2a); articles still use the pool.
- **Fork, own orbs and the menus (§2a)**: the orbs are drawn under every pane of the layout (HUD, vanilla pause menu, plugin menu), as the 3D effects were; shielding and grab statuses draw no hurt orbs (step order above, unchanged).
- **`077a8e43` difference**: at `97cbe9b9` the per-frame pass ran in every mode (online too), calling `set_sleep_effect(false)` on every fighter every frame and, with the setting On, drawing spheres in online matches (a desync-safe but visible and unwanted side effect, and a per-frame call outside Training). Fold in the early return.

## 7. Persistence

Only the `hitbox_vis` setting (SPEC-core-04).

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | not measured separately (small: three hooks, one colour table) | see `docs/resource-costs.md` (W5) | `tools/build/nro_size.py` per-feature build |
| per-frame work (Off) | one `set_sleep_effect(false)` + one setting read per fighter (upstream); `077a8e43`: plus the Training check first | one setting read per fighter; no game call when Off (Q4) | diag counters |
| per-frame work (On) | kill_kind + up to 8 × 16 `req_follow` + 2 setter calls each, per fighter | same unless Q1/Q3 change the model | diag counters, effect-pool occupancy |

§8 cost rows stay **pending** until the Eden census (rows-C C1/C4: the `[Hitbox] hurt census` and `[Hitbox] stats`
lines record the HitModule read's cost and the pass's own cost on the integrated candidate's diag build).
| heap allocations in hooks | upstream's shield rewrite collects 36 Lua values into a `Vec` per attack command; the visualizer itself allocates nothing | 0 (fixed-size array for the rewrite) | counting allocator |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Sphere count and spacing for capsules (`n = ceil(d/(1.75 s))+1` clamped 2..16; t = i/(n−1)); single sphere for a point hitbox | `core06_capsule_sampling` | `core06-E1` screenshot of an extended hitbox (e.g. a sword tilt) | `docs/EDEN-RESULTS.md` |
| Colour table by `id mod 8`; grab boxes `(id+3) mod 8` | `core06_colour_table` | `core06-E2` multi-hitbox move screenshot | `docs/EDEN-RESULTS.md` |
| Per-frame step order and status exemptions (grab range, shield, tread, DOWN) | `core06_frame_rules_table` | `core06-E3` grab, shield, missed tech cases | — |
| First-frame draw from the `ATTACK`/`CATCH` parameters (indices as §3) | `core06_command_param_indices` (model: parameter extraction) | `core06-E4` frame-step the first active frame | — |
| Rebound cleared → spheres killed unless shielding | `core06_rebound_rule` | `core06-E5` | — |
| Training-only gate incl. `077a8e43` early return; no effect-sleep calls online | `core06_training_gate` | `core06-E6` Quickplay match with setting On: no spheres, effects normal | `docs/EDEN-RESULTS.md` |
| Three sites guarded (64 B) and installed independently; neighbouring thunk windows checked first | `core06_site_guards` | `core06-E7` install line `handle_attack=stock/near …` | — |
| Off restores effects next frame | `core06_off_restores` | `core06-E8` | — |

## 10. Open questions

1. **Projectile hitboxes** are drawn once and never refreshed, so multi-hit articles show stale spheres. Recommendation: either run the per-frame pass for weapons too (hook or iterate articles) or kill the article's spheres at each `ATTACK` for that object; decide after measuring the effect-pool cost.
2. **Shield-damage rewrite** (Infinite shield) lives in the `ATTACK` hook and re-pushes 36 Lua values per attack command. Recommendation: move the behaviour to SPEC-cpu-* (shield) as a shared hook consumer of the same site, with a fixed-size buffer; the `ATTACK` site then has two consumers in one plugin (no sharing issue).
3. **Effect pool exhaustion.** Respawning every sphere every frame is wasteful. Recommendation: investigate keeping the follow effects alive while the attack slot stays active (effect handles from `req_follow`) and only killing on change; measure before changing the visible behaviour.
4. **Per-frame `set_sleep_effect(false)` when Off.** Upstream calls it every frame for every fighter as the "restore" path. Recommendation: call it once on the On→Off transition (per fighter) and otherwise not at all.
5. **Colour comment mismatch** (orange `#ff9900` vs components `1.0, 0.4, 0.0`). Recommendation: pick the components (what the player saw); the designer may retune the eight tints for colour-blind contrast.
