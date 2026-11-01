# Save states: fighter-specific restore — behavioural specification

- **Spec id:** `SPEC-state-05`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/character_specific/mod.rs`, `kirby.rs`, `pikmin.rs`, `ptrainer.rs`, `steve.rs`, `src/training/save_states.rs` (`ARTICLE_ALLOWLIST`, `on_ptrainer_death`, Kirby and trainer waits), `src/training/charge.rs` (Kirby branch), `src/training/mod.rs` (`handle_change_motion`, `handle_fighter_play_se`, `handle_fighter_effect`, `handle_fighter_joint_effect`), `src/common/mod.rs` (`is_dead`, `is_ptrainer`), `src/common/offsets.rs` (`COPY_SETUP`, `KIRBY_OPFF`, `ACTIVATE_AUTONOMY`, `POKEMON_DECIDE`) at commit `97cbe9b9` (unchanged at `077a8e43`); fork changes at `…/stages/m5i/stage/src/training/character_specific/kirby.rs` lines 2, 5, 17, 19, 22, 37, 606-607; `pikmin.rs` 2-3, 34, 36, 51-52; `ptrainer.rs` 1, 5, 143, 145, 153-168, 173-174; `steve.rs` 4-6; `mod.rs` 67-76; `…/stage/src/training/charge.rs` 432-439, 441-444, 454-456, 459; `…/stage/src/training/save_states.rs` 520-555, 1104 from `docs/provenance/provenance-files.md`
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

Some fighters carry state that the generic save and load of SPEC-state-01 would destroy or mishandle: Pokémon Trainer must come back as the saved Pokémon without the switch animation; Kirby must get his copied ability back with its charge; Olimar's Pikmin must keep their order and not wander off; Steve must keep his materials and tool durabilities; Luma, Duck Hunt's reticle, the Mii hats, Pyra's and Mythra's swords and the Pikmin must survive the respawn cleanup; and Sora's spell-change jingle must not play on every load. This spec collects those rules.

## 2. User-visible behaviour

No menu items of its own. Visible results on load:

- Pokémon Trainer: the saved Pokémon appears directly at the saved spot; no Poké Ball throw, no flying plate, no switch sound; the trainer stands on the ground behind.
- Kirby: wears the saved hat again with the saved copied charge, provided the current opponent is the fighter the hat came from (or the same Pokémon Trainer / Aegis family); the copy sound does not play.
- Olimar: his three Pikmin come back in the saved colour order and follow him immediately; in Training generally, idle Pikmin never go autonomous and lose their order.
- Steve: material counts and the HUD material bar, tool materials and durabilities match the save.
- Luma, the reticle, Mii hats, Aegis swords, Pikmin: still present after the load.
- Sora: the saved spell is current and the spell-switch sound is silent outside his neutral special.

## 3. Inputs

- The record's `fighter_kind`, `charge` and `steve_state` (SPEC-state-01 §7.2).
- Pokémon Trainer: the Pokémon's parent trainer object (`LinkModule::get_parent_object_id(pokemon, FIGHTER_POKEMON_LINK_NO_PTRAINER)` → `sv_battle_object::module_accessor`), the trainer's current Pokémon (`LinkModule::get_node_object_id(trainer, WEAPON_PTRAINER_PTRAINER_LINK_NO_POKEMON)`), the trainer's Poké Ball article (`ArticleModule::is_exist / get_article(trainer, WEAPON_PTRAINER_PTRAINER_GENERATE_ARTICLE_MBALL)` → `Article::get_battle_object_id`), trainer status and situation, a 5-frame in-game counter.
- Kirby: flags `FIGHTER_KIRBY_INSTANCE_WORK_ID_FLAG_COPY` and `COPY_ON_START` (work flag id `0x20000104`), int `…_INT_COPY_CHARA`, the copy module pointer (work int64 id `0x10000106`) whose copied fighter kind is the i32 at `+0x17398`, the opponent's kind and entry id.
- Olimar: the troops manager (SPEC-state-02 §5.1), each Pikmin's status.
- Steve: work ints `FIGHTER_PICKEL_INSTANCE_WORK_ID_INT_MATERIAL_NUM_GRADE_1 / _WOOD / _STONE / _IRON / _GOLD / _RED_STONE / _DIAMOND` and the tool record buffer pointer `…_INT_EXTEND_BUFFER` (work int64).

## 4. Outputs and native state touched

### 4.1 Hooks and writes

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| Trainer may switch | `WorkModule::off_flag(trainer, WEAPON_PTRAINER_PTRAINER_INSTANCE_WORK_ID_FLAG_ENABLE_CHANGE_POKEMON)` before death; `on_flag` after placement | §5.1 | fighter is a Pokémon (`PZENIGAME`, `PFUSHIGISOU`, `PLIZARDON`) |
| Trainer animation speed | `MotionModule::set_rate(trainer, 1000.0)` (pre-death and on the `restart` motion while loading); `set_rate(trainer, 1.0)` once the trainer is in `WEAPON_PTRAINER_PTRAINER_STATUS_KIND_RUN` | §5.1 | — |
| Poké Ball | `MotionModule::set_rate(mball, 1000.0)`; `ArticleModule::set_visibility_whole(trainer, MBALL, false, ARTICLE_OPE_TARGET_ALL)` | pre-death, when the article exists | — |
| Trainer situation/status after switch | `StatusModule::set_situation_kind(trainer, GROUND, true)`; `StatusModule::change_status_force(trainer, RUN, false)` | §5.1 when the trainer is airborne after the switch | — |
| Chosen Pokémon on switch | inline hook at `main+0x34cec44` (inside the game's Pokémon-decide routine): while loading, register `w8` (the chosen Pokémon index) is replaced per §5.1 step 5 | every call | Training, loading, a saved Pokémon kind |
| Trainer respawn timing | `WorkModule::get_param_int` overrides `rebirth_move_frame_trainer` → 0 and `param_mball`/`change_fly_frame` → 0 while loading (SPEC-state-01 §4.1) | every call | — |
| Trainer `restart` motion | replacement hook on `MotionModule::change_motion` (shared with the tech feature): after the original, if the object's kind is `WEAPON_KIND_PTRAINER_PTRAINER`, the motion is `restart` and a load is in progress → `set_rate(1000.0)` | every call | Training |
| Pokémon switch effects | in the `EffectModule::req` and `EffectModule::req_on_joint` hooks: for objects of kind `PZENIGAME`, `PFUSHIGISOU`, `PLIZARDON`, `WEAPON_KIND_PTRAINER_PTRAINER`, `WEAPON_KIND_PTRAINER_MBALL` or −1: `sys_flying_plate` → size 0 always in Training; `ptrainer_change_light` and hash40 `0x10e3fac8d9` → size 0 while loading | every call | Training |
| Pokémon switch sounds | in the fighter `play_se` hook (`main+0x4cf6c0`): `se_ptrainer_change_appear`, `se_ptrainer_ball_open`, `se_ptrainer_ball_swing` → `se_silent` while loading | every call | Training |
| Kirby copy ability | game copy-setup routine at `main+0xba0e80` `(kirby, opponent_entry_id, copied_kind, true, false)` | load completion | §5.2 conditions; **fork fix folded**: the second argument is the opponent's real entry id (`WorkModule::get_int(opponent, FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID)`), not the literal 1 |
| Kirby copied charge | replacement hook on Kirby's per-frame copy routine at `main+0xb971d0` `(param, kirby_fighter*) → u64`: §5.2 step 4 | every call | Training, loading |
| Kirby copy sounds | fighter `play_se` hook: hashes `0x1453dd86e4` and `0x14bdd3e7c8` → `se_silent` unless Kirby's status is `FIGHTER_KIRBY_STATUS_KIND_SPECIAL_N_DRINK` | every call | Training (always, not only while loading) |
| Pikmin autonomy | replacement hook on the Pikmin "activate autonomy" routine at `main+0x34b6cd0` `(weapon*, weapon_work_module*)`: owner accessor = `*(work_module + 0x8)`; call the original only when the Pikmin's status is in `WEAPON_PIKMIN_PIKMIN_STATUS_KIND_ATTACK_AIR ..= WEAPON_PIKMIN_PIKMIN_STATUS_KIND_ATTACK_HI4_LANDING` (thrown or attacking); otherwise return without calling | every call | Training (fork: or an owned restore) |
| Pikmin order | SPEC-state-02 §5.1 (remove, respawn in order, follow) | load completion | Olimar |
| Steve materials | `WorkModule::set_int` of the seven material ints | load completion | fighter is Steve (`PICKEL`) |
| Steve tools | tool records at `buffer + 0xC × i` (i = 0 sword, 1 axe, 2 pick, 3 shovel): byte `+0` material kind (the game's `FighterPickelMaterialKind`, stored as one `char`), f32 `+4` durability; the buffer address is re-read from `…_INT_EXTEND_BUFFER` on every load, never cached | load completion | buffer pointer non-null |
| Steve HUD bar | `FighterSpecializer_Pickel::sub_material_num(steve, i, 0)` for i = 0…6 (subtracting zero refreshes the bar) | after the writes | — |
| Article keep-list | SPEC-state-01 §5.2 respawn cleanup skips: (`MIIFIGHTER`, `FIGHTER_MIIFIGHTER_GENERATE_ARTICLE_HAT`), (`MIISWORDSMAN`, `…MIISWORDSMAN_…_HAT`), (`MIIGUNNER`, `…MIIGUNNER_…_HAT`), (`ROSETTA`, `FIGHTER_ROSETTA_GENERATE_ARTICLE_TICO`), (`PICKEL`, `FIGHTER_PICKEL_GENERATE_ARTICLE_TABLE`), (`ELIGHT`, `FIGHTER_ELIGHT_GENERATE_ARTICLE_ESWORD`), (`EFLAME`, `FIGHTER_EFLAME_GENERATE_ARTICLE_ESWORD`), (`PIKMIN`, `FIGHTER_PIKMIN_GENERATE_ARTICLE_PIKMIN`), (`DUCKHUNT`, `FIGHTER_DUCKHUNT_GENERATE_ARTICLE_RETICLE`) | respawn cleanup | kind matches |
| Sora spell sound | fighter `play_se` hook: hash `0x156fdf29ba` → `se_silent` when the object's status is below `FIGHTER_TRAIL_STATUS_KIND_SPECIAL_N1` (Sora's and Kirby's neutral-special statuses are all at or above it) | every call | Training (always) |
| Shulk art damage flash | cleared inside the `dead_rebirth_wait_frame` override (SPEC-state-01 §4.1) | every death | — |

### 4.2 Kirby copied-ability charge table

Read on Save (after `FLAG_COPY` and `COPY_CHARA`), written from the copy hook on load. `charge_max` and follow effects as defined in SPEC-state-02 §4; joints and effects for Kirby are hash40 values the spec writer could not name and are given as constants.

| Copied fighter | Saved | Written on load |
| --- | --- | --- |
| Samus / Dark Samus | `int_x` ← `FIGHTER_SAMUS_INSTANCE_WORK_ID_INT_SPECIAL_N_COUNT` (read on Kirby) | set; 112 → `charge_max` + follow `samus_cshot_max` / `samusd_cshot_max` on `handr`, pos 0, rot 0; handle → `…_INT_EFH_CHARGE_MAX` |
| Sheik | `int_x` ← `…SHEIK…_INT_NEEDLE_COUNT` | set; 6 → `charge_max` (no needle article on Kirby) |
| Mewtwo | `int_x`, `int_y`, `has_charge` as SPEC-state-02 | set; `int_y` = 120 → `charge_max` + follow effect `0x1ac6d446d8` on joints `0x5e008fd84` (left) and `0x51a07c0e7` (right), lifetime −1; handles → `…_INT_EF_ID_SHADOWBALL_MAX_L/_R`; flag |
| Squirtle | `int_x` ← `…PZENIGAME…_INT_SPECIAL_N_CHARGE` | set; 45 → `charge_max` |
| Olimar | `int_x` ← `FIGHTER_PIKMIN_INSTANCE_WORK_INT_PRE_PIKMIN_VARIATION`, `int_y` ← `…_INT_BEFORE_PRE_PIKMIN_VARIATION` | set both (Kirby's next-Pikmin colour state) |
| Lucario | `int_x`, `int_y`, `has_charge` as SPEC-state-02 | set; `int_y` = 90 → `charge_max` + follow effects `0x164bf96ca1` (left joint `0x5e008fd84`) and `0x16b1f651c2` (right joint `0x51a07c0e7`), lifetime −1; handles → `…_INT_EF_ID_AURABALL_MAX_L/_R`; flag |
| R.O.B. | `float_x` ← `…ROBOT…_FLOAT_BEAM_ENERGY_VALUE` | set |
| Wii Fit Trainer | `float_x` ← `…WIIFIT…_FLOAT_SPECIAL_N_CHARGE_LEVEL_RATIO` | set |
| Pac-Man | `int_x`, `has_charge` as SPEC-state-02 | as SPEC-state-02 (rank, flag, key flag at 12) |
| Robin | `int_x` ← `…REFLET…_INT_SPECIAL_N_THUNDER_KIND` | set; 3 → `charge_max` + follow effect `0x12db3e4172` on joint `0x5eb263e0d`, lifetime −1 |
| Hero | `int_x` ← `…BRAVE…_INT_SPECIAL_N_HOLD_FRAME` | remove `charge_max`, clear `…_FLAG_SPECIAL_N_MAX_EFFECT`, set |
| Sora | `int_x` ← `…TRAIL…_INT_SPECIAL_N_MAGIC_KIND` | as SPEC-state-02 (rotate, flag, `change_magic`) |
| any other | nothing | nothing |

## 5. Timing and frame rules

### 5.1 Pokémon Trainer

1. **Pre-death rule** (KillPlayer and every WaitForAlive frame, SPEC-state-01 §5.2): for a Pokémon fighter, clear the trainer's `ENABLE_CHANGE_POKEMON`, set the trainer's motion rate to 1000, and if the Poké Ball article exists set its rate to 1000 and hide it. This stops the trainer from reacting to the death and skips its animations.
2. The game's respawn of a Pokémon goes through the trainer's Poké Ball throw. The parameter overrides (`rebirth_move_frame_trainer`, `change_fly_frame` → 0) and the `restart` motion rate make it instantaneous; the effect and sound hooks hide it.
3. **Decide hook** (`main+0x34cec44`, inline): the game chooses which Pokémon comes out. While loading, the record for the Pokémon's role (player or CPU) gives the saved kind; `value = saved_kind − FIGHTER_KIND_PZENIGAME`. If `value > 2` (the record was not saved as a Pokémon) leave the register alone. Otherwise (fork fix, folded): `x9` points at the trainer's Pokémon descriptor; read `count` (u32 at `+0x14`) and three kinds (u32 at `+0x18`, `+0x1c`, `+0x20`); the three kinds are a rotation of (Squirtle, Ivysaur, Charizard) starting at the trainer's chosen starting Pokémon; only when `count == 3`, the pointer is non-null and 8-byte aligned, and the kinds are exactly such a rotation, write `w8 = index of saved_kind within those three`. Upstream wrote `w8 = value`, which is right only when the trainer starts with Squirtle. The descriptor fields are the fork's own observation (open question 1).
4. **After placement** (SPEC-state-01 §5.3 step 8g): raise `ENABLE_CHANGE_POKEMON`; marker → WaitForPokemonSwitch.
5. **WaitForPokemonSwitch** each frame: if the trainer is in `RUN`, restore its motion rate to 1.0. Wait 5 in-game frames on the trainer's own delay counter (the switch flash is late). Then, if the trainer's situation is AIR, force it to GROUND and force status `RUN` (the trainer otherwise locks up in the air). Report finished → marker NoAction.
6. Dead test: a Pokémon in `WAIT` whose previous status is `NONE` while its trainer is in `WEAPON_PTRAINER_PTRAINER_STATUS_KIND_RESTART_CHANGE` counts as dead (SPEC-state-01 §3) so that the one undetectable switching frame does not end the wait early; the trainer-status part of the test distinguishes this from the frame after an L+R+A reset.

### 5.2 Kirby

1. Save: `has_charge` ← has a copy ability; `int_z` ← copied fighter kind; then the copied charge per §4.2.
2. Load completion, only if the current fighter is Kirby and the saved kind is Kirby (SPEC-state-01 §5.3 step 8c): if `has_charge` is present: opponent = the other of player/CPU. Accept when the opponent's kind equals `int_z`, or both are in {Squirtle, Ivysaur, Charizard, −1} (an opponent whose kind reads −1 counts as a Pokémon when its trainer parent link is a valid active object), or both are in {Pyra (`EFLAME`), Mythra (`ELIGHT`)}. When accepted call the copy-setup routine with the opponent's entry id and `int_z`. Marker → WaitForCopyAbility in every case.
3. The game then runs its copy start on a later frame and sets `COPY_ON_START`.
4. Copy hook (`main+0xb971d0`) on each Kirby frame while loading: remember whether `COPY_ON_START` was set **before** calling the original (the original clears it); call the original; if it was set: read the copied kind from the copy module and write the copied charge (§4.2) from the role's record; end WaitForCopyAbility (marker → NoAction).
5. SPEC-state-01 §5.4 also ends WaitForCopyAbility whenever `COPY_ON_START` is clear, which covers "no hat saved", "opponent did not match" and "copy already finished".
6. The fork's hat engine (managed sources) replaces step 2 with an asynchronous hat restore that does not need the opponent in the match and services itself from the same copy hook (`kirby_hats::restore(kirby, kind)`, `kirby_hats::on_frame(kirby)`): interface only.

### 5.3 Olimar

- Every frame in Training the autonomy hook keeps non-thrown Pikmin from activating autonomy (order preservation).
- On load completion the line is rebuilt (SPEC-state-02 §5.1). The Pikmin article is on the keep-list so the respawn cleanup leaves it; upstream still removes and respawns all Pikmin on load to enforce the order.

### 5.4 Steve

- Save: read the seven material counts and the four tool records (material kind byte and durability float) from the tool buffer.
- Load completion (SPEC-state-01 §5.3 step 8e), only for Steve: write the seven counts, the four material bytes, the four durabilities, then refresh the bar with seven zero-subtractions. The tool buffer address must be read fresh each time: a stored address from another Training session crashes.
- Non-Steve fighters ignore a present Steve state; the never-saved default is Steve's starting kit (§7).

### 5.5 Kept articles, Sora, Shulk

Keep-list and the Sora and Shulk hooks are single-site rules listed in §4.1; they have no sequence of their own.

### 5.6 Character-specific status equivalences (consumer: the CPU mash feature)

A lookup used by the mash/combo features to decide whether a fighter is "in" an expected status when its real status is fighter-specific: Bowser `SPECIAL_HI` → `FIGHTER_KOOPA_STATUS_KIND_SPECIAL_HI_G`, `_HI_A`; Sora `SPECIAL_N` → `FIGHTER_TRAIL_STATUS_KIND_SPECIAL_N1`, `_N1_SHOOT`, `_N1_END`, `_N2`, `_N3`; Sora `ATTACK_AIR` → `FIGHTER_TRAIL_STATUS_KIND_ATTACK_AIR_N`, `_ATTACK_AIR_F`; King K. Rool `SPECIAL_S` → `FIGHTER_KROOL_STATUS_KIND_SPECIAL_S_THROW`, `_CATCH`, `_FAILURE`, `_GET`. Fork fix, folded: Kirby `SPECIAL_N` → any status in `[FIGHTER_KIRBY_STATUS_KIND_MIIFIGHTER_SPECIAL_N, FIGHTER_KIRBY_STATUS_KIND_NUM)` (every copied neutral-special status, including charge, cancel and air phases). This lives with the fighter-specific code upstream; the mash spec should cite it.

## 6. Edge cases and interactions

1. Pokémon Trainer saved as Ivysaur, loaded while Charizard is out: the decide hook makes the respawn bring out Ivysaur. Saved as a non-Pokémon (player swapped fighters): the register is left alone and the current Pokémon returns.
2. Both fighters are Pokémon Trainer: each Pokémon resolves its own trainer by parent link; both waits run independently with the same 5-frame counter (open question 2).
3. Kirby vs Pokémon Trainer: the family rule lets a Squirtle hat return while Charizard is out (the copy routine is given the saved kind, so the hat is Squirtle's).
4. Kirby is the CPU: the opponent is the player, entry id 0 (the fork fix); upstream passed entry 1 and the hat failed or copied the wrong fighter.
5. Kirby loaded as a different kind (hot swap): nothing happens; WaitForCopyAbility ends on the flag test.
6. Olimar with Pikmin held during the save: held Pikmin are saved first in the order (SPEC-state-02 §5.1), so the restored line starts with them.
7. Steve's crafting table is kept across the load; the table's position is not saved (it stays where it was).
8. Duck Hunt's reticle, Luma, Mii hats, Aegis swords: kept; their internal state (Luma's position, reticle position) is whatever the respawn produces.
9. Lobby practice / CPU-as-P1 (fork): all role lookups use the practice resolver; `get_state_pokemon`, `get_charge_state`, `end_copy_ability` return "no result" instead of panicking when a role cannot be resolved (fork fix, folded).
10. Leaving Training during WaitForPokemonSwitch or WaitForCopyAbility: markers are cancelled with the session (SPEC-state-01 §6.9).

## 7. Persistence

- Steve: `steve_state` object or `null` in the record: `mat_g1`, `mat_wood`, `mat_stone`, `mat_iron`, `mat_gold`, `mat_redstone`, `mat_diamond` (i32); `sword_mat`, `axe_mat`, `pick_mat`, `shovel_mat` (one-code-point strings, the material kind as a `char`, written in JSON as e.g. `"\u0001"`); `sword_durability`, `axe_durability`, `pick_durability`, `shovel_durability` (f32). Default (never saved): g1 36, wood 18, stone 0, iron 3, gold 0, redstone 2, diamond 0; all four tool materials `1`; sword durability 25.0; axe, pick, shovel durability 70.0. The fork bounds material kinds to 0…15 on import (carried over).
- Kirby: in the `charge` object (`has_charge`, `int_z`, plus the copied fields).
- Pokémon Trainer: `fighter_kind` of the record.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | see docs/resource-costs.md (W5) | ≤ fork | `tools/build/nro_size.py` per-feature build |
| per-frame work | see docs/resource-costs.md (W5) | the autonomy, copy, change-motion, effect and sound hooks each add one or two reads per call; zero otherwise | diag counters |
| heap allocations in hooks | 0 | 0 | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Trainer pre-death rule: flag off, rates 1000, ball hidden | `state05_pt_predeath` | `E-state-05-pt-silent-switch` | |
| Decide hook: rotation mapping, descriptor validation, non-Pokémon record untouched | `state05_pt_decide_mapping` | `E-state-05-pt-ivysaur-start-charizard` | |
| Wait: 5 frames, RUN rate 1.0, airborne trainer grounded | `state05_pt_wait_sequence` | `E-state-05-pt-trainer-grounded` | |
| Pokémon dead test special case | `state05_pt_dead_test` | — | |
| Switch effects and sounds silenced; flying plate always hidden | `state05_pt_suppressions` | `E-state-05-pt-no-plate` | |
| Kirby acceptance rule (equal, PT family incl. −1, Aegis family) | `state05_kirby_accept_rule` | `E-state-05-kirby-pt-family` | |
| Copy-setup called with the opponent's entry id | `state05_kirby_opponent_entry` | `E-state-05-kirby-cpu-hat` | |
| Copy hook: flag read before original, charge written, wait ended | `state05_kirby_copy_hook` | `E-state-05-kirby-samus-charge` | |
| Kirby copied charge table | `state05_kirby_charge_table` | `E-state-05-kirby-mewtwo-max` | |
| Kirby and Sora sound silences by status | `state05_sfx_status_rules` | `E-state-05-kirby-sora-sfx` | |
| Pikmin autonomy only in thrown statuses | `state05_pikmin_autonomy_gate` | `E-state-05-pikmin-order-kept` | |
| Steve read/write of counts and tool records; bar refresh; fresh buffer address | `state05_steve_roundtrip` | `E-state-05-steve-materials` | |
| Keep-list exact pairs | `state05_keep_list` | `E-state-05-luma-reticle-kept` | |
| Kirby status interval in the equivalence table | `state05_status_equivalence` | — | |

## 10. Open questions

1. The Pokémon descriptor layout at the decide site (`x9` → `+0x14` count, `+0x18..+0x24` kinds) is the fork's observation on 13.0.5 and is not in upstream. Recommendation: re-verify against the owned binary before the site table is frozen, and keep the "leave the register alone unless everything matches" rule.
2. The trainer switch delay counter is one shared counter; two Pokémon Trainers in a load share it. Recommendation: one counter per role.
3. `sys_flying_plate` is hidden for the Pokémon Trainer family on every spawn in Training, not only during loads (upstream: "we never want the flying plate"). Recommendation: keep for parity; note it in the user docs.
4. The Kirby and Sora sound silences apply to every such sound in Training, not only during loads (a real copy outside `SPECIAL_N_DRINK`, a real spell change outside neutral special). Recommendation: gate both on "loading" and verify in Eden that no load-time sound slips through; if one does, keep upstream's unconditional rule and document it.
5. Copy-module offset `+0x17398` for the copied fighter kind is an upstream guess ("maybe, no clue" on the vtable). Recommendation: verify on the owned binary.
6. Upstream respawns all Pikmin on every load even when the current line already matches; this costs three article spawns. Recommendation: keep (simplest correct behaviour).
