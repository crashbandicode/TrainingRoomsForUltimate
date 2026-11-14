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
  - Grab boxes (`CATCH` animation commands) are drawn when they are created, all in one colour no hitbox uses: magenta
    `1.0, 0.0, 1.0` (rc.13, operator: "grabs being a different color"; before it was the colour for `(id + 3) mod 8`).
    A grab box whose command ran in a status the fighter has already left by its pass (the grab connected the frame its
    box was set, as Samus's tether does) is drawn that pass only; before rc.13 it stayed through the whole pull.
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
| 5 | 0.0, 0.4, 1.0 | blue `#0066ff` (rc.13; was `0.4, 0.4, 1.0`) |
| 6 | 0.0, 1.0, 0.6 | spring green `#00ff99` (rc.13; was purple `0.8, 0.0, 1.0`) |
| 7 | 1.0, 1.0, 1.0 | white `#ffffff` (rc.13; was pink `1.0, 0.2, 0.8`) |

rc.13 took purple and pink out of the hitbox table so the grab boxes' magenta is theirs alone (every hitbox tint is at
least 1.0 from it in RGB, `every_grab_box_is_magenta_a_colour_no_hitbox_uses`).

### 2a. Rendering: own orbs for fighters (decision HB-OWN-ORBS, option 1, operator 2026-11-05)

The operator decided "go with discs for all" after the A/B (`docs/decisions/HB-OWN-ORBS-ab.png`, `docs/DECISIONS.md`
HB-OWN-ORBS): with two Ridleys the game's shared effect pool refused 22 of one fighter's 55 spheres on every pass; own
orbs do not use that pool. This supersedes the upstream effect rendering for **fighters** (hit, hurt and grab spheres):

- The sphere set (§2 colours, hurtboxes of decision HB1, grab boxes) is unchanged except for capsules (below); the per-frame pass
  keeps each fighter's spheres (`HandleSet`, ruling Q3) through an own-orb sink that never refuses and ends an orb when the
  fighter's status changes (the effect's `CONCLUDE_STATUS` rule).
- Each held sphere is placed on its bone with `ModelModule::joint_global_position_with_offset(…, false)` and published
  per fighter in fixed storage. The flag is meant to make the call a read (no model update first); the guarded thunk
  passes it to the implementation behind ModelModule's vtable slot `0x158`, which is not read yet (review HB-3: a
  follow-up Eden tracepoint reads that slot's target).
- **Pills (decision HB-PILLS, `docs/DECISIONS.md`, 2026-11-14):** this fork draws a shape with two centres as one pill
  and replaces §2's sampling rule (`n` spheres along the segment) for its own orbs. A shape is held as its two end
  spheres, its centre and its second centre (sphere keys 0 and 1); a point hitbox is one sphere. The board folds the two
  ends into one orb (centre and tail), whatever order the held set keeps them in.
- The layout tick projects both ends of every orb, and one radius point along world x and y at each, with the game's
  own camera (`world_to_screen`'s projection, render camera), and draws the orb as a pill of the plugin's panes: a
  half-disc cap on each end (a radius wide, its flat side on the end, turned along the projected axis, the far cap by a
  further 180°) and a band between the ends (the projected length, rotated the same way). The width is the mean of
  the two ends' projected diameters. The caps and the band come from one plugin-drawn disc profile (the cap is its left
  half, the band its centre column), tinted per colour, so they meet without a seam (Eden `rc13-pill-7v2`, 4×
  zooms of the joins) and nothing in one pill is drawn twice. A point is two caps back to back: the disc.
  The panes' container is RootPane's first child, so the HUD, the vanilla pause menu and the plugin's menu draw over the
  orbs as they drew over the 3D effects; nothing is hidden for the menus.
- **How many (review HB-2, 2026-11-05; HB-PILLS):** a fighter holds at most 128 spheres (`MAX_HELD`; more are counted
  as dropped, as before). The layout has 512 orb panes, 340 caps and 172 bands: 170 pills. A pill takes two caps and,
  when its ends project apart, a band. When the panes run out the cut is deterministic and takes whole pills, never a
  cap without its band: every hitbox and grab box is drawn before any hurtbox, each in board-slot order, and the log
  says once `[Hitbox] more orbs than panes` (`drawn` and `cut` count pills).
- When the practice ends while its layout stays (the waiting room after its request ends; Training's layout outside
  Training), the tick hides every orb it showed and the container (review HB-1).
- The pills are flat and drawn over the stage and the fighters (no depth hiding); the effect spheres' glow extended past
  each sphere's radius, a pill stops at it.
- **Known limitation, camera lag:** while the camera pans, the pills trail the spheres' true screen position by about one
  to two frames of motion (measured in Eden run `tr-p23-hitbox-ab2`); still, they coincide. Projecting from a render-side
  camera was not attempted (no cheap, guarded site).
- **Articles and thrown items** (rc.13) are own orbs too (`Articles`, `ARTICLES` = 4 at once; a fifth replaces the
  oldest). An article whose `ATTACK` or `CATCH` ran, or an item with a live attack slot, is drawn by every fighter pass
  from its live attack slots on its bones while its attack lasts (§6, Q1). An article's `CATCH` (Olimar's Pikmin) waits
  in the article's own `HandleSet` with the status it ran in and is drawn as a fighter's grab box is.
- A failed guard of any row of the feature's closure leaves the whole feature uninstalled (install line
  `hitbox=not_installed site=<row>`): nothing is drawn, no partial path. A layout without the orb panes draws nothing and
  says so once (`[Hitbox] orb panes missing`).

### rc.14: the Frame data colours ranking rule (spots lane; operator 2026-11-14; defaults SP1 and SP2 approved ~18:07Z, the colours changed ~18:15Z to Ultimate Frame Data's legend)

The rule behind the *Frame data colours* item of the checklist below (it began as its own row, `frame_data_colours`, and is an item of
`hitbox_vis` since the checklist). Unticked it is exactly the drawing above.
Ticked, a hitbox is **red** when it is the strongest hitbox of its frame ("Sweet") or cannot be ranked, and **pink** (`#ff8fc8`) when another
hitbox out on that frame is stronger ("Sour") or when the frame's strongest hitbox is weaker than the strongest seen earlier in the same
action ("Late"). Damage is each active slot's `AttackData` damage (`layout::POWER`); no per-fighter table. An action is one status, and the
motion frame going back (the same status entered again, as a jab's next hit does) starts a new one. Grab boxes and hurtboxes keep their colours (the legend lane extends the one colour
function, `hitbox::legend_colour`). The shipped build reads each active slot's damage a second time per pass, only while the row is On.

### rc.14: the Hitbox Visualization checklist, the frame data legend and launch angles (operator 2026-11-14; plan approved by the primary)

`hitbox_vis` is a checklist of five items (`legend::ITEMS`): Hitboxes, Grab boxes, Hurtboxes, Frame data colours, Launch angles; nothing
ticked by default, which draws nothing and keeps the pass and the hooks idle, as Off did. Each item draws on its own: the layout tick
filters the orbs by kind (`legend::shows`), so any mix works; the HitModule is read only while Hurtboxes is ticked. A file from before
the checklist migrates (`legend::migrate`): On becomes the three boxes, Off none.

*Frame data colours* recolours what is drawn through one function, `hitbox::legend_colour`: a hitbox red when it is the strongest of its
frame or cannot be ranked and pink when it is weaker (the spots lane's rule, by `AttackData` damage); a grab box purple; a hurtbox by the
game's own status of its part (`legend::part_status`, as `HitModule::get_status` at `main+0x46a800`: the highest of the part's word, the
group's whole status `+0x1c` taken as at least 0, and the group's global status `+0x20`): 0 white, 1 (invincible) green, 2 (intangible)
and 3 (the whole group off: the revival platform) sky blue. Eden `hbl-trace-8fadea57-1`: a spot dodge sets whole 2, the revival platform
whole 3, the time after it global 1, with every part word 0. **Windboxes keep the hit colours:** `ATTACK` has no windbox argument; the
attribute rule was refuted (F.L.U.D.D.'s water push is `collision_attr_water`); two bool arguments (26 at `+0x70`, 27 at `+0x6e`) are set on
both known windboxes and on neither Sing nor seven damaging hits, but the game's use of `+0x6e` (skipping a hit-feedback block of sound and
effect calls) does not define "pushes without hurting", so no rule is wired.

*Launch angles* adds white marks of a hitbox's own orbs, from `AttackData +0x28` (i32 degrees counter-clockwise from forward, mirrored by
`PostureModule::lr`): a line from the centre to the edge for 0 to 360; two dashes for 361 at 38 degrees (the launch the trace measured on a
target in the air, `t-hbl-361b`); a centre dot for 362 and above. Up to two marks per hitbox (`BOARD_ORBS`); drawn last, cut first.
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

- **Projectiles and articles**: the per-frame pass runs for fighters only (the fighter vtable driver), so projectile hitboxes are not redrawn per frame; they are drawn once when their `ATTACK` command runs (any battle object executing the command, since the hook reads the Lua state's accessor) and then follow their bone with `CONCLUDE_STATUS` until the article's status ends. Multi-hit projectiles therefore keep stale spheres between hits. Q1. Fork (rc.13): articles and thrown items are own orbs redrawn from their attack slots every pass (§2a); no effect sphere is requested for them.
- **Shielding fighter**: nothing is drawn and effects are not slept (the shield visual is itself a `sys_shield` effect; killing it would remove the player's shield bubble — this is why shielding is exempt).
- **Grab statuses**: no hit spheres, no sleeping, so the grab animation effects show; grab boxes drawn at `CATCH` persist until rebound. Fork: the grabber's hurt orbs stay (§2a).
- **Missed tech / footstool**: effects kept awake (`DOWN`, tread statuses) so those visual cues remain.
- **Kirby copy, Pokémon Trainer switch, Ice Climbers**: each fighter body runs the pass independently; Nana's hitboxes are drawn too. Pokémon Trainer's trainer object is not a fighter accessor in the driver.
- **Mii fighters, Aegis, Steve**: no special handling upstream.
- **Save-state load**: the save-state feature kills effects independently (SPEC-state-*); the visualizer redraws next frame.
- **Menu open**: unaffected (the game keeps running underneath upstream; spheres keep drawing).
- **Lobby practice (fork)**: `visualizes(accessor)` gates per fighter so opponents' hitboxes in a waiting room are never drawn and their effects never slept — carry over the gate hook.
- **CPU-as-P1 / hot swap (fork)**: `is_training_mode()` false during a swap → nothing drawn that frame.
- **Shield-damage rewrite inside the `ATTACK` hook**: a different feature (Infinite shield) piggybacks on this hook; it pops and re-pushes 36 Lua values on every attack command while Infinite is selected. Record as Q2 (ownership and cost).
- **Effect budget**: the game's effect pool is finite; 8 slots × up to 16 spheres × fighters, respawned every frame, can exhaust it and drop other effects. Q3. Fork: resolved by the own orbs (§2a), for fighters and, since rc.13, articles and thrown items.
- **Fork, own orbs and the menus (§2a)**: the orbs are drawn under every pane of the layout (HUD, vanilla pause menu, plugin menu), as the 3D effects were; shielding draws no hurt orbs (step order above, unchanged). A grab status (`CATCH..=CATCH_TURN`) keeps the grabber's hurt orbs and drops only its hit spheres: the grabber stays hittable, so a missed grab can be punished (the hurt read was put after the upstream grab stop in HB1 and hid them until this fix).
- **`077a8e43` difference**: at `97cbe9b9` the per-frame pass ran in every mode (online too), calling `set_sleep_effect(false)` on every fighter every frame and, with the setting On, drawing spheres in online matches (a desync-safe but visible and unwanted side effect, and a per-frame call outside Training). Fold in the early return.

## 7. Persistence

Only the `hitbox_vis` setting (SPEC-core-04), a five-item checklist since rc.14.

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
| A capsule is its two end spheres (keys 0 and 1), one pill (HB-PILLS; replaces the `n = ceil(d/(1.75 s))+1` sampling); a single sphere for a point hitbox | `core06_capsule_is_two_ends`, `a_capsules_two_ends_are_one_orb`, `projected_orbs_map_onto_the_layout_canvas`, `the_pill_panes_are_cut_whole_hurtboxes_first` | `core06-E1` screenshot of an extended hitbox (e.g. a sword tilt) | `docs/EDEN-RESULTS.md` |
| Colour table by `id mod 8`; grab boxes magenta, a colour no hitbox uses | `core06_colour_table`, `every_grab_box_is_magenta_a_colour_no_hitbox_uses` | `core06-E2` multi-hitbox move screenshot | `docs/EDEN-RESULTS.md` |
| Per-frame step order and status exemptions (grab range, shield, tread, DOWN) | `core06_frame_rules_table` | `core06-E3` grab, shield, missed tech cases | — |
| First-frame draw from the `ATTACK`/`CATCH` parameters (indices as §3) | `core06_command_param_indices` (model: parameter extraction) | `core06-E4` frame-step the first active frame | — |
| Rebound cleared → spheres killed unless shielding | `core06_rebound_rule` | `core06-E5` | — |
| Training-only gate incl. `077a8e43` early return; no effect-sleep calls online | `core06_training_gate` | `core06-E6` Quickplay match with setting On: no spheres, effects normal | `docs/EDEN-RESULTS.md` |
| Three sites guarded (64 B) and installed independently; neighbouring thunk windows checked first | `core06_site_guards` | `core06-E7` install line `handle_attack=stock/near …` | — |
| Off restores effects next frame | `core06_off_restores` | `core06-E8` | — |
| rc.14 Frame data colours: Sweet red, Sour and Late pink, an unrankable hitbox red; Off is today's colours; one function chooses every tint | `the_hit_spheres_take_the_frame_data_colours`, `off_is_exactly_the_old_drawing`, `one_function_chooses_every_tint`, `the_strongest_hitbox_of_a_frame_is_sweet_and_a_weaker_one_sour`, `a_hitbox_weaker_than_an_earlier_one_of_the_action_is_late` | rc14 spots, Marth forward smash and Captain Falcon forward air at 1/4 speed | `artifacts/rc14-spots/` |
| rc.14 checklist and legend: each item draws on its own, old On/Off migrates, grab purple, hurtboxes by the game's status, windboxes uncoloured, launch angles (line, dashes at 38, dot) mirrored by facing | `legend_checklist_items_draw_independently`, `legend_old_hitbox_vis_migrates`, `legend_hitbox_visualization_is_one_checklist`, `legend_part_status_is_the_games_max`, `legend_hurtbox_status_colours`, `legend_grab_is_purple_while_frame_data_colours_is_on`, `legend_off_keeps_todays_tints`, `legend_windboxes_keep_the_hit_colours`, `legend_angle_line_points_along_the_angle_mirrored_by_facing`, `legend_special_angles_are_dashed_or_a_dot`, `legend_marks_follow_their_hitbox_and_draw_last`, `legend_facts_match_the_owned_elf` | `hbl-trace-8fadea57-1`, `hbl-final-252e200b-1`, the checklist boot | `artifacts/rc14-hb-legend/` |

## 10. Open questions

1. **Projectile hitboxes** are drawn once and never refreshed, so multi-hit articles show stale spheres. Recommendation: either run the per-frame pass for weapons too (hook or iterate articles) or kill the article's spheres at each `ATTACK` for that object; decide after measuring the effect-pool cost. Fork: resolved in rc.13 by the first option without the pool: articles and thrown items are own orbs redrawn from their attack slots every pass (§2a).
2. **Shield-damage rewrite** (Infinite shield) lives in the `ATTACK` hook and re-pushes 36 Lua values per attack command. Recommendation: move the behaviour to SPEC-cpu-* (shield) as a shared hook consumer of the same site, with a fixed-size buffer; the `ATTACK` site then has two consumers in one plugin (no sharing issue).
3. **Effect pool exhaustion.** Respawning every sphere every frame is wasteful. Recommendation: investigate keeping the follow effects alive while the attack slot stays active (effect handles from `req_follow`) and only killing on change; measure before changing the visible behaviour.
4. **Per-frame `set_sleep_effect(false)` when Off.** Upstream calls it every frame for every fighter as the "restore" path. Recommendation: call it once on the On→Off transition (per fighter) and otherwise not at all.
5. **Colour comment mismatch** (orange `#ff9900` vs components `1.0, 0.4, 0.0`). Recommendation: pick the components (what the player saw); the designer may retune the eight tints for colour-blind contrast.
