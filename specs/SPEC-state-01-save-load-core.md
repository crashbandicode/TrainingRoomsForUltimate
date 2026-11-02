# Save states: save and load core — behavioural specification

- **Spec id:** `SPEC-state-01`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/save_states.rs`, `src/training/mod.rs` (the save-state hooks: `handle_get_param_int`, `handle_set_dead_rumble`, `handle_req_quake`, `handle_se`, `handle_fighter_play_se`, `handle_effect`, `handle_fighter_effect`, `handle_effect_follow`, `handle_fighter_joint_effect`, `handle_star_ko`, `once_per_frame_per_fighter`), `src/common/mod.rs` (`is_dead`, `is_operation_cpu`, `try_get_module_accessor`), `src/common/button_config.rs`, `training_mod_consts/src/options.rs` (`SaveStateMirroring`, `SaveDamage`, `DamagePercent`, `SaveStateSlot`, `OnOff`, `PlaybackSlot`, `RecordTrigger`), `training_mod_consts/src/lib.rs` (Save States tab), `training_mod_consts/src/files.rs`, `README.md` (Save States section, FAQ 12) at commit `97cbe9b9` (and `077a8e43`: no change to these files); fork changes at `…/stages/m5i/stage/src/training/save_states.rs` lines 2, 25, 95-102, 116-118, 171-172, 175-179, 183-504, 508-510, 515-516, 525-529, 538-541, 547-550, 563-569, 578-581, 589-602, 621-624, 770-790, 817-828, 838-847, 864-867, 893-897, 914-950, 962-973, 1003-1006, 1047-1053, 1066, 1089-1094, 1104, 1110-1120, 1147-1149, 1158-1159, 1167-1169, 1173-1181, 1196-1200 and `…/stage/src/training/mod.rs` lines 242-337, 526-538, 665-923 from `docs/provenance/provenance-files.md`
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

The player presses a button chord to remember where both fighters are (position, facing, damage, situation, special-move charges, fighter kind) in one of five numbered slots, and presses another chord to put both fighters back there. Loading goes through the game's own death and respawn so that every status, article, item and effect is cleaned up for free, then teleports each fighter to its saved spot, restores damage and charges, optionally mirrors the setup across the stage centre, optionally gives the player an item, applies buffs, and can start input recording or playback. The slots survive game restarts. This is the practice loop the whole plugin is built around and it replaces the game's own L+R+A reset.

## 2. User-visible behaviour

### 2.1 Menu items (Save States tab unless noted)

| Item | Setting id | Type | Values | Default |
| --- | --- | --- | --- | --- |
| Mirroring | `save_state_mirroring` | single-choice | None, Alternate, Random | None |
| Auto Save States | `save_state_autoload` | toggle | On, Off | Off |
| Save Dmg (CPU) | `save_damage_cpu` | single-choice | Default, Save State, Random Value | Default |
| Dmg Range (CPU) | `save_damage_limits_cpu` | slider (two handles) | Lower 0–150, Upper 0–150 (percent) | 0 / 150 |
| Save Dmg (Player) | `save_damage_player` | single-choice | Default, Save State, Random Value | Default |
| Dmg Range (Player) | `save_damage_limits_player` | slider (two handles) | Lower 0–150, Upper 0–150 | 0 / 150 |
| Enable Save States | `save_state_enable` | toggle | On, Off | On |
| Save State Slot | `save_state_slot` | single-choice | Slot 1 … Slot 5 | Slot 1 |
| Randomize Slots | `randomize_slots` | multi-choice with weights (1–5 per slot) | Slot 1 … Slot 5 | none selected |
| ~~Character Item~~ | `character_item` | **dropped 2026-11-01 (N6)**: the row and its setting are gone; the first-boot import converts older saves once (SPEC-core-08 §4, SPEC-state-04) | — | — |
| **Respawn invincibility** (draft 9, N2; Save states group, after Auto Save States) | `respawn_invincibility` (proposed id) | toggle | On, Off | Off |
| Buff Options | `buff_state` | multi-choice (no weights) | see SPEC-state-03 | none selected |
| Save State Playback (Input Recording tab) | `save_state_playback` | multi-choice with weights | Slot 1 … Slot 5 | none selected |
| Recording Trigger (Input Recording tab) | `record_trigger` | single-choice | Button Combination, Save State Load | Button Combination |
| Save State Save (Button Config tab) | `save_state_save` | multi-choice of buttons | A, B, X, Y, Pro L, Pro R / GCC Z, Pro ZL / GCC L, Pro ZR / GCC R, D-pad Up/Down/Left/Right, Plus, Minus, Left Stick press, Right Stick press | ZL + D-pad Down |
| Save State Load (Button Config tab) | `save_state_load` | multi-choice of buttons | same list | ZL + D-pad Up |

**Respawn invincibility (N2, decided 2026-11-01; B8).** Off (default) is the fork and upstream: a natural death in Training respawns the fighter at once, with no respawn invincibility (the §4.1 parameter overrides apply to every death). On: after a **natural** death (a blast-zone or similar death that is not part of a save-state load) the fighter, **P1 or the CPU**, respawns with the game's own respawn invincibility and, where the game's own respawn path can be used, on the respawn platform as in a match: the §4.1 parameter overrides are then applied only while a load is in progress. The row is greyed while *Auto Save States* (Auto-load on death) is On, because a death then loads a save state and the load's own instant respawn is in force (the stored value is kept and takes effect again when Auto Save States is turned Off). If the platform cannot be had through the game's own respawn path, that limit goes to the operator before this ships (open question 3). The row is read live; turning it On or Off takes effect at the next death.

Help texts (upstream, for the menu writer): Mirroring "Flips save states in the left-right direction across the stage center"; Auto Save States "Load save state when any fighter dies"; Save Dmg "Should save states retain CPU/player damage"; Dmg Range "Limits on random damage to apply to the CPU/player when loading a save state"; Enable Save States: upstream's text still names the pre-7.0 chords "Shield+Down Taunt / Shield+Up Taunt", which is stale; use the configured chords instead (open question 1); Save State Slot "Save and load states from different slots."; Randomize Slots "Slots to randomize when loading save state."

### 2.2 Button chords

- A chord passes on the frame when one of its buttons is held and every other button of the chord is newly pressed that frame (any button may be the held one). Chords read the raw controller state of player 1 after the game's own mapping, never Smash's mapped inputs. The GameCube layout maps "ZL" to the GCC L trigger (analogue or digital) and "ZR" to the GCC R trigger; "Pro R" is GCC Z; "Pro L" does not exist on GCC.
- Chords never pass while the plugin menu or the game's own pause menu is open.
- Each chord request is latched by the input hook and consumed once by the first consumer in the next fighter frame; it never fires twice for one press.
- **Save** (default ZL + D-pad Down) and **Load** (default ZL + D-pad Up) are live in Training only. In the fork's waiting-room practice they are live only while lobby practice owns both practice fighters (carried-over feature, see §6.9).
- The Load chord is read only on the callback of a fighter that is not CPU-operated and is not Nana.

### 2.3 Visible outputs

- On save: notification header "Save State", body "Saved Slot N" (N is the selected slot's display name), 120 frames; any earlier "Save State" notification is cleared first. Each saved fighter plays the effect `sys_deku_flash` on its `top` joint at size 0.25, following the joint, no joint scaling, concluded with the status.
- On load: both fighters die silently (no death explosion, no death rumble, no camera quake, no star KO) and reappear at the saved spots. The CPU plays the sound `se_system_position_reset` when it has been placed (only the CPU, so the sound is not doubled). No score effect, no screen darkening, no lingering slow-motion or zoom.
- At plugin start, two startup notifications tell the player the current Save and Load chords ("Save State" / "Load State", 120 frames each).

## 3. Inputs

- Menu settings listed in §2.1, read every frame from the live menu copy.
- Controller: player 1's raw button state for the two chords (§2.2).
- Game state per fighter, read on the fighter's per-frame callback: `StatusModule::status_kind`, `StatusModule::prev_status_kind`, `StatusModule::situation_kind`, `PostureModule::pos_x` / `pos_y` / `lr`, `DamageModule::damage(0)`, the fighter kind (`app::utility::get_kind`), the entry id (`WorkModule::get_int(FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID)`), `FighterInformation::is_operation_cpu` through the FighterManager singleton, `sv_information::stage_id()`, and the charge fields of SPEC-state-02.
- Global game state: the FighterManager singleton (symbol `_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E`), the ItemManager singleton (symbol `_ZN3lib9SingletonIN3app11ItemManagerEE9instance_E`), `app::smashball::is_training_mode` (symbol `_ZN3app9smashball16is_training_modeEv`).
- Files: the slot file (§7) read once, lazily, the first time the slots are touched.
- Random numbers: the game's `sv_math::rand` / `randf` with the `fighter` hash, for Random mirroring, random slots and random damage.

Definition used throughout: a fighter is **dead** when its status is `FIGHTER_STATUS_KIND_DEAD` or `FIGHTER_STATUS_KIND_STANDBY`, or, for a Pokémon Trainer Pokémon only, when its status is `WAIT` with previous status `NONE` while the trainer weapon's status is `WEAPON_PTRAINER_PTRAINER_STATUS_KIND_RESTART_CHANGE` (the one switching frame the plain test misses; SPEC-state-05 §2).

## 4. Outputs and native state touched

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| Fighter status | `StatusModule::change_status_force(DEAD, true)` | KillPlayer step for a fighter that is not already dead | fighter is a save-state participant (§6.9), not Nana |
| Fighter status | `StatusModule::change_status_request(WAIT / FALL, false)` | PosMove step, per saved situation (§5.3) | state PosMove / NanaPosMove |
| Fighter position | `PostureModule::init_pos(pos, true, false)` for a grounded saved situation; `PostureModule::set_pos(pos)` otherwise | every PosMove frame | see §5.3; the first flag resets ground contacts and collision history at the destination, the second stays false so linked objects are not translated (fork fix, folded) |
| Fighter facing | `PostureModule::set_lr(mirror × saved lr)` | every PosMove frame | — |
| Fighter velocity | `KineticModule::clear_speed_all` | every PosMove frame | — |
| Damage | `DamageModule::heal(−current, 0)` then `DamageModule::add_damage(target, 0)` | PosMove completion, per Save Dmg setting | — |
| Animation command sleep | `MotionAnimcmdModule::set_sleep(true)` after respawn; `set_sleep(false)` on every PosMove frame | §5.2, §5.3 | — |
| Sound | `SoundModule::stop_all_sound`, `SoundModule::pause_se_all(true/false)`, `SoundModule::play_se_no3d("se_system_position_reset", true, true)` | §5.2, §5.3 | — |
| Rumble | `ControlModule::stop_rumble(true)` after respawn, `stop_rumble(false)` on PosMove | §5.2, §5.3 | — |
| Camera | `SlowModule::clear_whole`, `CameraModule::zoom_out(0)`, `CameraModule::stop_quake(k)` for every quake kind `CAMERA_QUAKE_KIND_NONE..=CAMERA_QUAKE_KIND_MAX` | after respawn (§5.2) | — |
| Screen effects | `EffectModule::kill_kind("sys_bg_criticalhit", false, false)`, `EffectModule::remove_screen("bg_criticalhit", 0)` | after respawn | — |
| Articles | for article index 0..=0x25: if `ArticleModule::is_exist` and not in the keep-list (§6.4), `ArticleModule::remove_exist_object_id(Article::get_battle_object_id(ArticleModule::get_article(idx)))` | after respawn | keep-list by fighter kind |
| Items | for every active item of the ItemManager: `ItemManager::remove_item_from_id(Item::get_battle_object_id)` | after respawn | ItemManager singleton non-null |
| Jostle timer | `WorkModule::set_int(1, FIGHTER_INSTANCE_WORK_ID_INT_HIT_STOP_IGNORE_JOSTLE_FRAME)` when the current value is > 1 | inside the `dead_rebirth_wait_frame` parameter override (§4.1) | — |
| Shulk art damage flash | `WorkModule::set_int(0, FIGHTER_INSTANCE_WORK_ID_INT_SHULK_MONAD_ARTS_DAMAGE_FLASH_FRAME)`, `EffectModule::remove_common("monad_arts_damage_buster")`, `remove_common("monad_arts_damage_smash")` | same override | — |
| Save-state effect | `EffectModule::req_on_joint("sys_deku_flash", "top", 0, 0, 0.25, 0, 0, true, NO_JOINT_SCALE \| FOLLOW \| CONCLUDE_STATUS, 0, 0)` | Save step | — |
| Slot file | write the whole slot table (§7) | when both fighters have finished a Save | game thread only |

### 4.1 Hooks that make the load silent and instant

These hooks are the save-state core's own; they are what makes a load look like a teleport instead of a death. All are active only in Training (fork: or during an owned lobby-practice restore, §6.9).

| Hooked function | Site (13.0.5) | Behaviour |
| --- | --- | --- |
| `WorkModule::get_param_int` (symbol hook) | — | For parameter group `common`: `dead_rebirth_wait_frame` → 1 (and performs the jostle and Shulk cleanups above); `rebirth_move_frame` → 0; `rebirth_wait_frame` → 0; `rebirth_invincible_frame` → 0; `rebirth_invincible_add_frame` → 0; `rebirth_move_frame_trainer` → 0 only while a load is in progress. For group `param_mball`, `change_fly_frame` → 0 only while a load is in progress. Everything else: original value. Note these overrides apply to every death in Training, not only to loads (open question 3), **unless Respawn invincibility is On** (draft 9, N2; §2.1): then every override of this row applies only while a load is in progress, so a natural death uses the game's own respawn (its invincibility and, where its own path allows, its respawn platform). |
| `sv_fighter_util::set_dead_rumble` (symbol `_ZN3app15sv_fighter_util15set_dead_rumbleEP9lua_State`) | — | Returns 0 without calling the original: no death rumble in Training. |
| `CameraModule::req_quake` (symbol hook) | — | While killing (§5.1 states KillPlayer / WaitForAlive for either fighter), the requested quake kind is replaced by `CAMERA_QUAKE_KIND_NONE`. |
| `SoundModule::play_se` (symbol hook) | — | While killing, the sound hash is replaced by `se_silent` (kills the death explosion and misfoot sounds). |
| Fighter sound module `play_se` (vtable path) | `main+0x4cf6c0` | See SPEC-state-03 §4 (buff silence) and SPEC-state-05 §4 (Kirby, Sora, Pokémon Trainer silences). |
| `EffectModule::req` (symbol hook) | — | While a load is in progress and the fighter is not being buffed, the effect size is forced to 0 (hides throw explosions, Ice Climbers squall and similar). Before that test the Pokémon Trainer size rule of SPEC-state-05 §4 is applied. Upstream installs two separate replacement hooks on this one function; the fork merged them into one hook that does both, which is the required shape (one hook per function). |
| `EffectModule::req_follow` (symbol hook) | — | While a load is in progress, `sys_score_aura` is requested at size 0 (stops the score aura appearing on the CPU when loading during hitstop). |
| `EffectModule::req_on_joint` (symbol hook) | — | Pokémon Trainer size rule only (SPEC-state-05 §4). |
| Stage "can star KO" query | `main+0x2610a40` | Returns false in Training: no star KOs, so a death is immediately followed by the respawn. |

The per-fighter frame callback that drives §5 is the shared "once per frame per fighter" entry: upstream installs it by writing its own function pointer into the fighter vtable slot at `main+0x4f80568` (original on-frame at `main+0x614630`); the companion on-init slot is `main+0x4f80240` (original `main+0x60dfa0`). Those two patches are shared with the whole plugin and belong to the runtime spec, not to this one; this spec only requires that `save_states(fighter)` runs once per fighter per game frame after the game's own on-frame work.

## 5. Timing and frame rules

Each fighter (player, CPU, and Nana, who shares Popo's record) owns one record per slot; the player and CPU records of a slot are independent copies of the same shape. Each record carries a step marker with the values **Save, NoAction, KillPlayer, WaitForAlive, PosMove, ApplyBuff, NanaPosMove, WaitForPokemonSwitch, WaitForCopyAbility**. The marker is runtime-only: it is written to the slot file as part of the record but must be forced back to NoAction when the file is read (fork fix, folded: a persisted in-progress marker must never resume a load in a new process).

**Selected slot.** On every callback: if Randomize Slots has any slot selected, the selected slot is the last randomly drawn slot (`RANDOM_SLOT`, initially slot 1); otherwise it is Save State Slot. The Save chord always saves into Save State Slot, never into the random slot. Upstream tests "any slot selected" by drawing a weighted random choice and comparing with empty, which rolls the game's RNG every frame for every fighter; the required behaviour is to test emptiness without drawing (open question 4).

**Entry gate.** `save_states(fighter)` returns immediately when Enable Save States is Off. It then resolves the fighter's record: entry id 1 → CPU record, otherwise → player record.

### 5.1 Load request (any frame, marker NoAction or not)

A load is requested when either
- Auto Save States is On, the fighter's record marker is NoAction and the fighter is dead (§3 definition); or
- the Load chord passed (read only on a non-CPU-operated, non-Nana fighter's callback),

and the fighter is not Nana. Then:
1. If the record marker is NoAction: choose the slot (draw the weighted random slot and remember it as `RANDOM_SLOT` when Randomize Slots is non-empty; else the selected slot) and set **both** the player and CPU records of that slot to KillPlayer.
2. Set the mirror sign for this load: None → +1; Alternate → the negation of the previous sign; Random → +1 or −1 with equal probability. (The sign starts at +1 and is reset to +1 by every Save.)
3. Stop any input-recording playback.
4. Return (nothing else happens on this frame).

If the marker is not NoAction (a load is already in progress) upstream still performs steps 2–3, which can flip Alternate mirroring between the two fighters' placements; the required behaviour is to ignore a load request while either record of the selected slot is not NoAction (open question 5).

### 5.2 Killing and waiting

- **KillPlayer** (not Nana): apply the Pokémon Trainer pre-death rule (SPEC-state-05 §5.1). If the fighter is not dead, force status DEAD. Marker → WaitForAlive. Return.
- **WaitForAlive**: apply the Pokémon Trainer pre-death rule again. When the fighter is no longer dead (it has respawned) and is not Nana: run the respawn cleanup (`on_death` in upstream's naming: stop sounds, remove non-kept articles, remove every item in the match, sleep animcmd, pause SE, stop rumble, clear slow, zoom camera out, kill and remove the critical-hit screen effects, stop every quake kind), marker → PosMove. Fall through to §5.3 in the same frame.

While either record of the selected slot is KillPlayer or WaitForAlive the plugin is **killing**; while either record is anything but NoAction the plugin is **loading**. Both predicates are read by the hooks of §4.1, SPEC-state-03 and SPEC-state-05.

### 5.3 Placing (PosMove / NanaPosMove), repeated every frame until the status matches

1. If marker is NanaPosMove and the fighter is not Nana, or Nana is in STANDBY, return.
2. Stop all sound, wake animcmd (`set_sleep(false)`), unpause SE, stop rumble (false), clear all speed.
3. Compute the target: if mirror sign is −1, `x = −(saved_x − stage_offset(stage))`, else `x = saved_x`; `y = saved_y`; `z = 0`. Stage offsets: Smashville (`StageID::Animal_Village`) 1.195, Town and City (`Animal_City`) 1.448, Yoshi's Island (`Yoshi_Island`) −1.053, all others 0 (open question 2).
4. If the saved situation is CLIFF, raise `y` for fighters whose ledge grab needs it: +6 for Donkey Kong, King Dedede, King K. Rool; +4 for Samus, Dark Samus, Mewtwo, Terry; and additionally +2 for Bowser, Mythra, Snake.
5. Facing: `lr = mirror sign × saved lr`.
6. Write position (grounded: `init_pos(pos, true, false)`; else `set_pos`), write facing, then run the training reset (SPEC-state-06 §5.2: mash, SDI direction, in-game frame counters, combo, ledge delay, throw delay, shield tilt).
7. Status handshake by saved situation:
   - GROUND: if status is not WAIT, request WAIT and stay in PosMove; else the fighter is placed.
   - AIR: if status is not FALL, request FALL and stay; else placed.
   - CLIFF: if status is not FALL, CLIFF_CATCH_MOVE or CLIFF_CATCH, request FALL and stay; else placed.
   - any other situation: placed immediately.
8. When placed (marker → NoAction in this frame), in this order:
   a. Damage: player record uses Save Dmg (Player), CPU record uses Save Dmg (CPU). Default → leave the respawn damage (0%); Save State → set to the saved percent; Random Value → set to a uniform random value between Lower and Upper of the matching Dmg Range. "Set" means heal everything then add the target. Upstream passes the CPU's limits as (Upper, Lower) and the player's as (Lower, Upper); required behaviour is (Lower, Upper) for both (open question 6).
   b. Item (player record only, not Nana): the fork restores the exact held item snapshot when the record has one; otherwise nothing is granted (the *Character Item* menu fallback was dropped, N6; the first-boot import has already written snapshots for older saves, SPEC-state-04).
   c. Charges: if the record's saved fighter kind equals the current fighter kind, write the saved charges (SPEC-state-02). If the fighter is Kirby, marker → WaitForCopyAbility.
   d. Buffs: if the fighter is Hero, Cloud, Joker, Little Mac, Sephiroth, Wii Fit Trainer, Shulk, Min Min or Wario, marker → ApplyBuff (this overrides Kirby's WaitForCopyAbility only in theory; no fighter is both).
   e. Steve: if the record has a Steve state, write it (SPEC-state-05 §5.4; a non-Steve fighter ignores it).
   f. CPU record only: play `se_system_position_reset` (no 3D).
   g. Pokémon Trainer Pokémon: raise the trainer's `ENABLE_CHANGE_POKEMON` flag and marker → WaitForPokemonSwitch (SPEC-state-05 §5.1).
9. Ice Climbers hand-off: if the fighter is Popo and its previous status was REBIRTH, marker → NanaPosMove, so the next frames place Nana (same record, same target) with steps 1–8 (Nana skips the item; her kind never matches the saved Popo kind so charges are skipped). The fork skips this hand-off when its Solo Popo feature has removed Nana (carried over; interface only).
10. Recording / playback trigger: if Recording Trigger is "Save State Load", start the input-recording lockout-record; otherwise, if Save State Playback has a slot selected and the saved situation is not CLIFF, start playback of a weighted random selected slot (ledge playback is left to the ledge-option feature). Upstream performs this on every PosMove frame of both fighters and draws the random slot twice (once to test, once to use); the required behaviour is one trigger when both records of the slot have reached NoAction, with the slot drawn once (open question 7).

### 5.4 Waits after placement

- **ApplyBuff**: SPEC-state-03 §5 runs every frame until it reports done; then clear the fighter's buffing flag and marker → NoAction.
- **WaitForPokemonSwitch**: marker → NoAction once SPEC-state-05 §5.1 reports the switch finished.
- **WaitForCopyAbility**: marker → NoAction when Kirby's `COPY_ON_START` flag (work flag id `0x20000104`) is clear; SPEC-state-05 §5.2 also ends it from the copy hook.

### 5.5 Save (end of every callback)

1. If the Save chord passed: mirror sign → +1; set both records of Save State Slot to Save; clear and post the "Saved Slot N" notification. Because the chord is consumed by the first fighter that reads it, the second fighter picks the step up from its own record marker.
2. If the fighter's record marker is Save and the fighter is not Nana: marker → NoAction and capture `x`, `y`, `lr`, damage, situation kind, fighter kind, the charges of SPEC-state-02 and the Steve state of SPEC-state-05 (the fork additionally captures the held item snapshot, the stage id and the role's exact Mii configuration; carried over). Play the `sys_deku_flash` effect. When neither record of the slot is still Save, write the whole slot table to the file on this (game) thread.

### 5.6 Settings changed mid-sequence

- Enable Save States turned Off during a load: the sequence freezes where it is (markers persist, fighters may be dead or sleeping). Required behaviour: cancel all markers to NoAction when the toggle turns Off (the fork exposes `cancel_pending()` for this and for scene departure).
- Mirroring changed mid-load: the sign was fixed at the request; no effect until the next request.
- Save State Slot changed mid-load: `get_slot` is re-read every frame, so the markers of the newly selected slot are consulted while the old slot's markers stay set; the load stalls until those are cleared. Required behaviour: latch the slot at the request and use it until both records reach NoAction (open question 5 covers this with the in-progress guard).
- Randomize Slots cleared mid-load: same latch rule.

## 6. Edge cases and interactions

1. **Nana** never requests a load or save, never saves, never gets an item, and is placed only through the NanaPosMove hand-off. She shares Popo's record (same entry id).
2. **Save during hitstun, grab, bury, ledge, respawn**: the capture is value-only (position, facing, damage, situation); the status is not saved. Loading a state captured mid-grab places the fighter in WAIT/FALL at the grab position. A state saved while dead captures the blast-zone position; nothing prevents this (open question 8).
3. **Load while dead** (Auto Save States): the KillPlayer step sees the fighter already dead and proceeds straight to WaitForAlive.
4. **Articles kept through a load**: Mii Brawler / Swordfighter / Gunner hat articles, Rosalina's Luma, Steve's crafting table, Pyra's and Mythra's sword articles, Olimar's Pikmin, Duck Hunt's reticle. Every other article index 0..=0x25 of the fighter is removed; every item in the match is removed.
5. **Two slots loaded quickly / second Load press mid-load**: see §5.1 and open question 5.
6. **Load with a different fighter than saved** (fork hot swap, Pokémon switch, Kirby): position, facing and damage are restored onto the current fighter; charges only when the kind matches (§5.3 step 8c).
7. **Stage changed since the save**: upstream loads regardless; the fork records the stage id and its setup library refuses a restore on a different stage (carried over). The core only needs to expose the saved stage id field.
8. **CPU-as-P1 / CPU Control** (fork): "player record" and "CPU record" are chosen by entry id, never by who operates the fighter; the Load chord is read only from the human-operated, non-Nana fighter. The fork resolves the two roles through a practice-ownership lease instead of raw entry ids (`practice::role` → 0/1, `practice::acquire`) so that extra Training CPUs never touch a record; the core must take the role from an injectable resolver.
9. **Lobby practice** (fork): save states apply only to the two local practice fighters while lobby practice owns them; outside that the chords, autoload and all §4.1 suppressions are inactive (`effects_enabled() = native Training ∨ (practice enabled ∧ loading)`). The fork additionally offers a quick reset (SPEC-state-06 §6), a preview pair for clip editing and a setup-library bind that writes a slot (`SlotBind { slot, player, cpu, restore, name }`) and may start a load on that slot. The core must expose: `cancel_pending()`, `is_loading()`, `is_killing()`, `slot_pair(slot) → Option<(record, record)>`, `queue_bind(SlotBind)`, `request_macro_restore()`, `request_preview_restore()`, `request_quick_reset()`, and a per-role "state load started / completed" notification used by the Mii HUD. None of these are re-specified here.
10. **Game pause / menu open**: chords do not pass while either menu is open; a load in progress continues frame by frame when the game unpauses.
11. **Hot swap mid-load** (fork): the fork retires the practice lease and cancels every marker before the native entry is replaced; the core must not hold pointers across frames (always re-resolve module accessors by entry id).
12. **Missing or malformed slot file**: all ten records take the defaults of §7.3 and a warning is logged; nothing is shown to the player.
13. **Training with more than one CPU** (fork note): only entry ids 0 and 1 are save-state participants; other fighters must return at the entry gate.

## 7. Persistence

### 7.1 Files

Upstream wrote TOML to `sd:/ultimate/TrainingModpack/save_states.toml`. The fork's carried-over layer writes pretty JSON to `sd:/ultimate/TrainingModpack/save_states.json` (menu settings in `training_modpack_menu.conf` beside it; corrected 2026-11-02, `training_modpack.json` is the updater record) and ships `tools/port_save_states.py` to convert old TOML. This project's root is `sd:/ultimate/TrainingRooms/`; the file name `save_states.json` is the carried-over contract and the settings file is `settings.json` (SPEC-core-08; open question 9 on the root, decided).

The file is read once, lazily, on first use of the slots (upstream notes that reading it eagerly at plugin start crashed), and written whenever both fighters finish a Save, from the game thread.

### 7.2 Record contract (one `SavedState`, serde field names)

| Field | Type | Meaning |
| --- | --- | --- |
| `x`, `y` | f32 | saved position |
| `percent` | f32 | saved damage |
| `lr` | f32 | facing, +1 right / −1 left |
| `situation_kind` | i32 | the game's situation constant (GROUND / AIR / CLIFF / other) |
| `state` | enum string: `Save`, `NoAction`, `KillPlayer`, `WaitForAlive`, `PosMove`, `ApplyBuff`, `NanaPosMove`, `WaitForPokemonSwitch`, `WaitForCopyAbility` | step marker; forced to `NoAction` on read |
| `fighter_kind` | i32 | saved fighter kind, −1 when never saved |
| `charge` | object | SPEC-state-02 §7: `int_x`, `int_y`, `int_z` (i32), `float_x`, `float_y`, `float_z` (f32), `has_charge` (bool), `robin_charges` (object) — every field optional; the writer emits explicit `null` for an absent field, as the fork's real files do (byte compatibility; lead correction 2026-11-02), and the reader accepts both `null` and a missing key |
| `steve_state` | object or null | SPEC-state-05 §7 |
| `held_item` | object, optional (fork) | `{"state": "item", choice, variation, lifetime, lifetime_max, fuse, fuse_max}` or `{"state": "empty"}` / `{"state": "unsupported"}`; absent in older saves |
| `saved_stage` | i32, optional (fork) | stage id at capture |
| `mii` | object, optional (fork) | exact Mii configuration; its tags are u64 and may exceed i64::MAX, which is why the format is JSON |

The file is `{"player": [5 records], "cpu": [5 records]}`; the fork accepts shorter arrays when importing. Unknown keys are ignored; a record missing `held_item`, `saved_stage` or `mii` loads with those absent; a malformed `held_item` becomes `unsupported` rather than rejecting the file.

### 7.3 Defaults (never-saved record)

`x = 0, y = 0, percent = 0, lr = 1, situation_kind = 0, state = NoAction, fighter_kind = −1`, empty charge, Steve state at the game's starting values (SPEC-state-05 §7), no item, no stage, no Mii. Loading a never-saved slot therefore places both fighters at the stage origin facing right with 0% (the fork's lobby practice refuses such a load; in Training upstream allows it).

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | see docs/resource-costs.md (W5) | ≤ fork | `tools/build/nro_size.py` per-feature build |
| per-frame work | see docs/resource-costs.md (W5) | two record reads per fighter per frame when idle; no RNG draw when idle | diag counters |
| heap allocations in hooks | upstream allocates a `HashMap` for the stage offsets on every mirrored placement frame | 0 | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Chord passes on hold+press, not on simultaneous press; not while a menu is open | `state01_chord_hold_then_press` | `E-state-01-chord` | |
| Save captures x, y, lr, percent, situation, kind into Save State Slot only | `state01_save_captures_fields` | `E-state-01-save-ground` | |
| Load request sets both records KillPlayer and stops playback | `state01_load_request_marks_both` | `E-state-01-load-ground` | |
| Kill → WaitForAlive → PosMove → NoAction sequence, grounded, WAIT handshake | `state01_sequence_ground` | `E-state-01-load-ground` | |
| Airborne saved state ends in FALL | `state01_sequence_air` | `E-state-01-load-air` | |
| Ledge saved state ends in FALL/CLIFF_CATCH; per-fighter y adjustments | `state01_ledge_adjust_table` | `E-state-01-load-ledge-dk` | |
| Grounded restore under a soft platform lands on the saved ground (init_pos) | — | `E-state-01-under-platform` | |
| Mirroring None / Alternate (toggles each load) / Random | `state01_mirror_sign_rules` | `E-state-01-mirror-alternate` | |
| Mirrored x uses stage offset for Smashville, T&C, Yoshi's Island | `state01_stage_offset_table` | `E-state-01-mirror-smashville` | |
| Damage Default / Save State / Random within (Lower, Upper) per role | `state01_damage_modes` | `E-state-01-damage-random` | |
| Random slot drawn once per request, remembered until next request | `state01_random_slot_latch` | `E-state-01-random-slots` | |
| Autoload triggers on death when On, never when Off | `state01_autoload_on_death` | `E-state-01-autoload` | |
| Nana placed via NanaPosMove after Popo; never saves | `state01_nana_handoff` | `E-state-01-ice-climbers` | |
| Article keep-list and item wipe on respawn | `state01_article_keep_list` | `E-state-01-luma-kept` | |
| Silence and no quake while killing; effects size 0 while loading | `state01_suppression_predicates` | `E-state-01-silent-load` | |
| Respawn parameter overrides (`dead_rebirth_wait_frame` = 1, others 0) | `state01_param_overrides` | `E-state-01-instant-respawn` | |
| **N2** Respawn invincibility Off (default): a natural death respawns at once with no invincibility, as the fork (parity) | `state01_respawn_off_is_fork` | `E-state-01-respawn-off` (blast-zone death, P1 and CPU) | |
| **N2** Respawn invincibility On: the §4.1 overrides apply only while loading; a natural death of P1 or of the CPU uses the game's own respawn invincibility (the fighter is invincible for the game's frames) | `state01_respawn_on_overrides_only_while_loading`, `state01_respawn_on_p1_and_cpu` | `E-state-01-respawn-on-invincible` | |
| **N2** Respawn platform: with the toggle On the fighter respawns on the platform as in a match where the game's own respawn path gives it; if it cannot, the limit is recorded and goes to the operator before shipping | — | `E-state-01-respawn-on-platform` (platform visible, or the recorded limit) | |
| **N2** The row is greyed while Auto Save States is On; a load (chord or auto) still respawns instantly and silently with the toggle On; the stored value is kept | `state01_respawn_row_greyed_with_autoload`, `state01_load_unaffected_by_respawn_toggle` | `E-state-01-respawn-on-then-load` | |
| **N2** Setting persisted as `respawn_invincibility` (default Off, missing key loads Off) | `state01_respawn_setting_default_and_roundtrip` (with `core04_draft9_fields_default`) | — | |
| No star KO in Training | — | `E-state-01-no-star-ko` | |
| CPU plays `se_system_position_reset`, player does not | — | `E-state-01-reset-sfx` | |
| Save notification text and duration | `state01_notification_text` | `E-state-01-notification` | |
| File written after both saves; read lazily; marker forced NoAction | `state01_file_roundtrip_marker_reset` | `E-state-01-persist-restart` | |
| Missing / malformed file → defaults, warning | `state01_file_missing_defaults` | — | |
| Record JSON shape incl. optional fork fields | `state01_record_json_shape` | — | |
| Load request ignored while loading (open question 5) | `state01_ignore_request_mid_load` | `E-state-01-double-press` | |
| Enable Off mid-load cancels markers | `state01_disable_cancels` | `E-state-01-disable-mid-load` | |
| Recording trigger / save-state playback fire once at completion | `state01_playback_once` | `E-state-01-playback-trigger` | |

## 10. Open questions

1. The Enable Save States help text in upstream and the fork still says "Shield+Down Taunt / Shield+Up Taunt" (the pre-7.0 chords). Recommendation: render the configured chords (the startup notifications already do).
2. Stage centre offsets: the code mirrors `x` around half the offsets 1.195 (Smashville), 1.448 (Town and City) and −1.053 (Yoshi's Island), while the upstream README FAQ 12 says the asymmetry (Smashville left side 4 units longer, Town and City left side 2 units shorter) "is not currently accounted for"; the two statements disagree in sign for Smashville. Recommendation: measure the left and right ledge x on each of the three stages in Eden and derive the centre from those, then replace both the constants and the FAQ; also check Smashville's and Town and City's moving platform is excluded from the measurement.
3. The respawn parameter overrides (instant respawn, no invincibility) apply to every death in Training, not only to save-state loads. This is upstream's observable behaviour ("use this instead of the built-in reset") but it also changes natural deaths. Recommendation: keep upstream's behaviour for parity and record it in the user docs; revisit if a user asks for normal respawns. **Decided 2026-11-01 (B8, N2): not kept as is; the batch is accepted with a change:** natural deaths in Training become the toggle *Respawn invincibility*, default Off (= the upstream and fork behaviour recommended here), and On gives the game's own respawn invincibility and, where its own path allows, the respawn platform (§2.1).
4. `get_slot` draws a weighted random slot every frame per fighter merely to test whether Randomize Slots is empty, consuming the game's `fighter` RNG stream. Recommendation: test emptiness without drawing; draw only at the load request. This changes RNG consumption relative to upstream, which no user-visible behaviour depends on.
5. A Load chord or autoload while a load is in progress re-rolls the mirror sign and stops playback without restarting the sequence; with Alternate mirroring the two fighters can be placed with different signs. Recommendation: ignore load requests while either record of the selected slot is not NoAction, and latch the slot at the request.
6. CPU random damage passes the limits as (Upper, Lower). With the game's `randf(range)` a negative range may clamp or misbehave; the player side passes (Lower, Upper). Recommendation: (Lower, Upper) for both.
7. The recording/playback trigger runs on every PosMove frame of both fighters and draws the playback slot twice (test, then use), so the slot tested may differ from the slot played. Recommendation: trigger once when both records reach NoAction, drawing the slot once. The fork already moved its named-macro trigger to that point.
8. Saving while dead or during a blast-zone fly captures an off-stage position. Recommendation: accept the capture (upstream does) but have the fork's lobby validity check (finite values, lr ∈ {−1, +1}, situation ∈ {GROUND, AIR, CLIFF}) apply in Training too before a load is honoured.
9. File root: the template says `sd:/ultimate/TrainingRooms/…`; the carried-over JSON layer and the port tool name `sd:/ultimate/TrainingModpack/`. Recommendation for the lead: new root, same file names, and extend `port_save_states.py` to copy from the old root once. **Decided earlier (2026-11-01) by SPEC-core-08:** the root is `sd:/ultimate/TrainingRooms/`, with a one-time copy of `save_states.json` and `setups/` (the settings file is `settings.json`).
10. Upstream's two separate replacement hooks on `EffectModule::req` (one for Pokémon Trainer effect sizes, one for load suppression) are folded here into a single hook; confirm the hook framework forbids double replacement so this does not regress silently.
11. Upstream 077a8e43 wraps the training reset check in `if !is_training_mode()` (see SPEC-state-06 open question 1); it also gates the whole per-fighter frame body on `is_training_mode()`, which this spec assumes.
