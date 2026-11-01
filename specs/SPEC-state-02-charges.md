# Save states: special-move charges — behavioural specification

- **Spec id:** `SPEC-state-02`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/charge.rs`, `src/training/character_specific/pikmin.rs` (`get_current_pikmin`, `spawn_pikmin`, `follow`), `src/training/character_specific/kirby.rs` (`get_kirby_hat_charge`), `src/training/mod.rs` (`handle_article_get_int`), `README.md` (saved attribute list) at commit `97cbe9b9` (unchanged at `077a8e43`); fork changes at `…/stages/m5i/stage/src/training/charge.rs` lines 2, 10, 19-23, 26-29, 432-439, 441-444, 454-456, 459 and `…/stage/training_mod_consts/src/practice_setup.rs` (fork-only file) from `docs/provenance/provenance-files.md`
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

When a state is saved, the charge level of the fighter's chargeable special (Samus's shot, DK's punch, Sheik's needles, Robin's tomes and so on) is recorded with it; when the state is loaded onto the same fighter kind, the charge is written back so the player can practise from a fully or partially charged setup without recharging. Where the game shows a visual for a full charge (the generic `charge_max` flash, a hand or arm glow, DK's face), that visual is recreated so the restored charge looks right.

## 2. User-visible behaviour

- No menu item: charges are always saved and always restored when the fighter kind matches the saved kind. The README's "Charge level" list in SPEC-state-01 §2 is the user-facing promise.
- Visible outputs on load: the fighter's charge meter or article matches the saved level; at full charge the game's own full-charge effect is shown (table in §4); Robin's book and Levin Sword durabilities and Incineroar's Revenge are restored; Olimar's three Pikmin are respawned in the saved colour order; Sora's current spell is the saved one.

## 3. Inputs

- The saved record's `charge` object (SPEC-state-01 §7.2) and `fighter_kind`.
- The current fighter kind, the fighter's module accessor on the PosMove completion frame of SPEC-state-01 §5.3 step 8c.
- For Olimar: the Pikmin troops manager pointer (work int64 id `0x100000C0` on Olimar's module accessor) and each Pikmin's module accessor by battle object id.
- For Sora: the fighter's `BattleObject` resolved from `module_accessor.battle_object_id` through the game's battle-object lookup (`main+0x3ac560`, the function used by `try_get_battle_object`).

## 4. Outputs and native state touched

`int_x/int_y/int_z/float_x/float_y/float_z/has_charge` below are the record's charge fields; the meaning of each field depends on the fighter. "Set when present" means the write happens only when the record has that field (older saves may omit fields). `charge_max` means `EffectModule::req_common(fighter, "charge_max", 0.0)`. A "follow effect" is `EffectModule::req_follow(fighter, effect, joint, pos, rot, 1.0, false, attribute, 0, lifetime, 0, 0, false, false)`; `attribute` is 0 unless stated, `lifetime` is 0 unless stated (−1 = unlimited). Every write is guarded by "current fighter kind equals the saved kind" (SPEC-state-01 §5.3 8c) and by the fighter being a save-state participant.

| Fighter (kind constant) | Saved on Save (read) | Written on Load | Full-charge value and visual |
| --- | --- | --- | --- |
| Mario (`FIGHTER_KIND_MARIO`) | `int_x` ← `WorkModule::get_int(FIGHTER_MARIO_INSTANCE_WORK_ID_INT_SPECIAL_LW_CHARGE)` (F.L.U.D.D.) | set when present | 80 → `charge_max` |
| Donkey Kong (`DONKEY`) | `int_x` ← `FIGHTER_DONKEY_INSTANCE_WORK_ID_INT_SPECIAL_N_COUNT` | set when present | 110 → `FighterUtil::set_face_motion_by_priority(fighter, FIGHTER_FACIAL_SPECIAL, "special_n_max_face")` |
| Samus / Dark Samus (`SAMUS`, `SAMUSD`) | `int_x` ← `FIGHTER_SAMUS_INSTANCE_WORK_ID_INT_SPECIAL_N_COUNT` | set when present | 112 → `charge_max` + follow effect `samus_cshot_max` (Samus) or `samusd_cshot_max` (Dark Samus) on joint `armr`, pos (7.98004, −0.50584, −0.25092), rot (−91.2728, −1.7974, 176.373); handle stored with `WorkModule::set_int(handle, FIGHTER_SAMUS_INSTANCE_WORK_ID_INT_EFH_CHARGE_MAX)` |
| Kirby (`KIRBY`) | `has_charge` ← flag `FIGHTER_KIRBY_INSTANCE_WORK_ID_FLAG_COPY`; `int_z` ← `FIGHTER_KIRBY_INSTANCE_WORK_ID_INT_COPY_CHARA`; then the copied ability's own charge per SPEC-state-05 §4.2 | SPEC-state-05 §5.2 (copy ability is re-granted, then its charge written from the copy hook) | per copied ability |
| Sheik (`SHEIK`) | `int_x` ← `FIGHTER_SHEIK_INSTANCE_WORK_ID_INT_NEEDLE_COUNT` | set when present; then `ArticleModule::generate_article_enable(FIGHTER_SHEIK_GENERATE_ARTICLE_NEEDLEHAVE, false, −1)` and `ArticleModule::set_visibility(NEEDLEHAVE, "set_main", group, ArticleOperationTarget(0))` with group `group_main_default` for 6, `group_main_5` … `group_main_1` for 5 … 1, `group_main_0` otherwise | 6 → `charge_max` |
| Mewtwo (`MEWTWO`) | `int_x` ← `…_INT_SHADOWBALL_CHARGE_FRAME`; `int_y` ← `…_INT_PREV_SHADOWBALL_CHARGE_FRAME`; `has_charge` ← flag `…_FLAG_SHADOWBALL_HAD` (all `FIGHTER_MEWTWO_INSTANCE_WORK_ID_…`) | set each when present; flag via `WorkModule::set_flag` | `int_y` = 120 → `charge_max` + follow effect `mewtwo_shadowball_max_hand` on `handl` and on `handr`, pos (1.0, 0.5, 0), lifetime −1; handles → `…_INT_EF_ID_SHADOWBALL_MAX_L` / `_R` |
| Mr. Game & Watch (`GAMEWATCH`) | `float_x` ← `FIGHTER_GAMEWATCH_INSTANCE_WORK_ID_FLOAT_SPECIAL_LW_GAUGE`; `float_y` ← `…_FLOAT_SPECIAL_LW_ATTACK` | set each when present | gauge 3.0 → `charge_max`; any other gauge → `EffectModule::remove_common("charge_max")` (clears a flash left from a bucket catch) |
| Wario (`WARIO`) | `int_x` ← `FIGHTER_WARIO_INSTANCE_WORK_ID_INT_GASS_COUNT` | set when present | none (0 … 6000 frames; the buff of SPEC-state-03 may override) |
| Squirtle (`PZENIGAME`) | `int_x` ← `FIGHTER_PZENIGAME_INSTANCE_WORK_ID_INT_SPECIAL_N_CHARGE` | set when present | 45 → `charge_max` |
| Olimar (`PIKMIN`) | `int_x`, `int_y`, `int_z` ← the colour (`WEAPON_PIKMIN_PIKMIN_INSTANCE_WORK_ID_INT_VARIATION`) of the 1st, 2nd, 3rd Pikmin in line order, held Pikmin first (§5.1); absent when fewer than three exist | `ArticleModule::remove_exist(FIGHTER_PIKMIN_GENERATE_ARTICLE_PIKMIN, ARTICLE_OPE_TARGET_ALL)`; then while the active Pikmin count (`ArticleModule::get_active_num`) is 0 / 1 / 2 and the matching field is present, spawn one Pikmin of that colour (§5.1); finally order every following Pikmin into `WEAPON_PIKMIN_PIKMIN_STATUS_KIND_AIR_FOLLOW` | none |
| Lucario (`LUCARIO`) | `int_x` ← `…_INT_AURABALL_CHARGE_FRAME`; `int_y` ← `…_INT_PREV_AURABALL_CHARGE_FRAME`; `has_charge` ← flag `…_FLAG_AURABALL_HAD` (`FIGHTER_LUCARIO_INSTANCE_WORK_ID_…`) | set each when present | `int_y` = 90 → `charge_max` + follow effects `lucario_hadoudan_max_l` on `handl` and `lucario_hadoudan_max_r` on `handr`, pos 0, lifetime −1; handles → `…_INT_EF_ID_AURABALL_MAX_L` / `_R` |
| R.O.B. (`ROBOT`) | `float_x` ← `…_FLOAT_BEAM_ENERGY_VALUE` (laser); `float_y` ← `…_FLOAT_GYRO_CHARGE_VALUE`; `float_z` ← `…_FLOAT_BURNER_ENERGY_VALUE` (fuel) (`FIGHTER_ROBOT_INSTANCE_WORK_ID_…`) | set each when present | gyro 90.0 → `charge_max` |
| Wii Fit Trainer (`WIIFIT`) | `float_x` ← `FIGHTER_WIIFIT_INSTANCE_WORK_ID_FLOAT_SPECIAL_N_CHARGE_LEVEL_RATIO` | set when present | none (0 … 1) |
| Pac-Man (`PACMAN`) | `int_x` ← `…_INT_SPECIAL_N_CHARGE_RANK`; `has_charge` ← flag `…_FLAG_SPECIAL_N_PULL_THROW` (`FIGHTER_PACMAN_INSTANCE_WORK_ID_…`) | set rank when present; set the pull/throw flag to `has_charge` when present; if rank is 12 and `has_charge` is present, also set flag `…_FLAG_SPECIAL_N_MAX_HAVE_ITEM` true | 12 (Key) → `charge_max` |
| Robin (`REFLET`) | `robin_charges` ← `thunder_kind` (`…_INT_SPECIAL_N_THUNDER_KIND`), `yellow_book` (`…_INT_SPECIAL_N_CURRENT_POINT`), `red_book` (`…_FLOAT_SPECIAL_S_CURRENT_POINT`), `green_book` (`…_INT_SPECIAL_HI_CURRENT_POINT`), `purple_book` (`…_INT_SPECIAL_LW_CURRENT_POINT`), `levin_charge` (`…_INT_THUNDER_SWORD_CURRENT_POINT`), `levin_recharge` (`…_INT_THUNDER_SWORD_REVIVAL_COUNT`), `has_levin` (flag `…_FLAG_THUNDER_SWORD_ON`) (`FIGHTER_REFLET_INSTANCE_WORK_ID_…`) | when the object is present, write all eight back (ints, float, flag) | `thunder_kind` = 3 (Thoron) → `charge_max` + follow effect `reflet_thunder_max` on `handl`, pos (1, 2, 0), lifetime −1 |
| Incineroar (`GAOGAEN`) | `int_x` ← `…_INT_ATTACK_HIT_REVENGE_ATTACK_NO`; `float_x` ← `…_FLOAT_REVENGE_TOTAL_DAMAGE`; `float_y` ← `…_FLOAT_REVENGE_RATE`; `has_charge` ← flag `…_FLAG_IS_REVENGE` (`FIGHTER_GAOGAEN_INSTANCE_WORK_ID_…`) | set each when present; if `has_charge` is true: set the flag, set `…_INT_REVENGE_TIMER` to 3500 (a fresh full timer; the remaining time is deliberately not saved), and request the follow effect `gaogaen_revenge_aura` on each of five joints (hash40 values `0x90fa00879`, `0x9f5af351a`, `0x51ba4b748`, `0x5e1ab8a2b`, `0x8813bc317`), attribute 32768, lifetime −1 | — |
| Piranha Plant (`PACKUN`) | `int_x` ← `FIGHTER_PACKUN_INSTANCE_WORK_ID_INT_SPECIAL_S_COUNT` | set when present | 75 → `charge_max` + follow effect `packun_poison_max_smoke` on `hip`, attribute 32768, lifetime −1; handle → `…_INT_SPECIAL_S_CHARGE_MAX_EFFECT_HANDLE` |
| Hero (`BRAVE`) | `int_x` ← `FIGHTER_BRAVE_INSTANCE_WORK_ID_INT_SPECIAL_N_HOLD_FRAME` | always first `EffectModule::remove_common("charge_max")` and clear flag `…_FLAG_SPECIAL_N_MAX_EFFECT`; then set when present | none recreated (0 … 81); the game redraws its own effect from the hold frame |
| Banjo & Kazooie (`BUDDY`) | `int_x` ← `FIGHTER_BUDDY_INSTANCE_WORK_ID_INT_SPECIAL_S_REMAIN` (Wonderwing feathers) | set when present | none (0 … 5) |
| Mii Gunner (`MIIGUNNER`) | `int_x` ← `FIGHTER_MIIGUNNER_INSTANCE_WORK_ID_INT_GUNNER_CHARGE_COUNT` | set when present | 120 → `charge_max` + follow effect `miigunner_cshot_max` on `armr`, pos (6, 0, 0); handle → `…_INT_EFH_CHARGE_MAX` |
| Sora (`TRAIL`) | `int_x` ← `FIGHTER_TRAIL_INSTANCE_WORK_ID_INT_SPECIAL_N_MAGIC_KIND` | when present: write `(saved + 1) mod 3` to `…_INT_SPECIAL_N_MAGIC_KIND`, raise flag `FIGHTER_TRAIL_STATUS_SPECIAL_N1_FLAG_CHANGE_MAGIC` (the game's spell-change routine is a no-op without it), then call `FighterSpecializer_Trail::change_magic(fighter)` on the fighter's `BattleObject`; the routine advances one spell, landing on the saved one and redrawing the HUD | — |
| every other kind | nothing | nothing | — |

Related hook: upstream installs a replacement hook on `ArticleModule::get_int` that only calls the original. It has no behaviour and must not be reimplemented (open question 1).

## 5. Timing and frame rules

- Save: all reads happen in the Save step of SPEC-state-01 §5.5 on the fighter's frame callback; a single frame.
- Load: all writes happen in the PosMove completion frame (SPEC-state-01 §5.3 step 8c), after damage and item, before buffs and the Steve state, only if the saved kind equals the current kind. A single frame for every fighter except Olimar and Kirby.
- Kirby: the copy ability is requested on the completion frame; the copied charge is written later by the copy hook (SPEC-state-05 §5.2) while the record waits in WaitForCopyAbility.

### 5.1 Olimar line order and respawn

Reading order: first the held Pikmin (count from the troops manager's held count; their battle object ids come from Olimar's work ints `FIGHTER_PIKMIN_INSTANCE_WORK_INT_PIKMIN_HOLD_PIKMIN_OBJECT_ID_0/1/2`), then the following Pikmin (count and `BattleObject` pointers from the troops manager's follower array). The troops manager layout at 13.0.5: `+0x08` max count (always 3), `+0x10` current following count, `+0x18` follower pointer array, `+0x20..+0x38` three follower `BattleObject` pointers, `+0x80` held count, `+0x88` held pointer array, `+0x90..+0xa8` three held `BattleObject` pointers. Only ids that are valid and active (`sv_battle_object::is_active`) contribute a colour; others leave the field absent.

Spawning one Pikmin of colour `v`: write `prev(v)` to `FIGHTER_PIKMIN_INSTANCE_WORK_INT_PRE_PIKMIN_VARIATION` and `prev(prev(v))` to `…_INT_BEFORE_PRE_PIKMIN_VARIATION`, where `prev(v) = v − 1` for `v > 0` and `prev(0) = 4` (the game chooses the next colour from the two previous ones), then `ArticleModule::generate_article(fighter, 0, false, −1)`. The three spawns are gated on the active count being exactly 0, 1 and 2 in turn so that a failed spawn does not shift the order. After spawning, every following Pikmin (from the troops manager follower array) gets `StatusModule::change_status_request(AIR_FOLLOW, false)` so the line forms at once.

Upstream removes all Pikmin articles before spawning, so the three saved colours replace whatever Olimar had. Olimar's own Pikmin article is on the keep-list of SPEC-state-01 §6.4 so the game does not also remove them during the respawn cleanup.

## 6. Edge cases and interactions

1. Saved kind differs from the current kind (fork hot swap, Pokémon switch): nothing is written. The fork's hot swap documents "character-specific charge restores only for a matching kind".
2. Charge fields absent (older save, or a field the fighter does not use): each write is skipped independently.
3. Robin: `levin_recharge` is saved so that a state saved right after entering Training restores a sword that is still recharging; upstream notes the L+R+A reset does not refill Levin (SPEC-state-06 open question 2).
4. Incineroar: Revenge is restored with a full timer regardless of how much time remained when saved (deliberate).
5. Olimar with fewer than three Pikmin saved: only the present fields spawn; the rest of the line stays empty.
6. Kirby with no copy ability saved (`has_charge` false): nothing is written and the record still waits in WaitForCopyAbility until the flag test of SPEC-state-01 §5.4 ends it (the flag is already clear, so this is one frame).
7. Lobby practice / CPU-as-P1 (fork): no additional rule; the kind check applies per role.
8. Two fighters of the same kind (player and CPU both Samus): each record restores its own fighter.

## 7. Persistence

The `charge` object of SPEC-state-01 §7.2. Field names and types: `int_x`, `int_y`, `int_z`: i32; `float_x`, `float_y`, `float_z`: f32; `has_charge`: bool; `robin_charges`: object with `thunder_kind` i32, `yellow_book` i32, `red_book` f32, `green_book` i32, `purple_book` i32, `levin_charge` i32, `levin_recharge` i32, `has_levin` bool. Every field is optional and omitted when absent; an empty object `{}` is a valid "no charge". The fork keeps these as plain value types in a portable crate with a `validate()` that bounds Robin's counts; the setup-library file shares the same shape (carried over).

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | see docs/resource-costs.md (W5) | ≤ fork | `tools/build/nro_size.py` per-feature build |
| per-frame work | see docs/resource-costs.md (W5) | 0 when idle (save and load frames only) | diag counters |
| heap allocations in hooks | upstream allocates vectors in the Olimar read | 0 (fixed arrays of 3) | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Save reads the table's fields for each listed kind | `state02_read_table_per_kind` | `E-state-02-save-samus` | |
| Load writes only present fields; kind mismatch writes nothing | `state02_write_only_present_and_matching_kind` | `E-state-02-kind-mismatch` | |
| Full-charge visuals requested at exactly the listed values | `state02_full_charge_visuals` | `E-state-02-samus-max-glow` | |
| Sheik needle article visibility group per count | `state02_sheik_needle_groups` | `E-state-02-sheik-needles` | |
| G&W removes `charge_max` when the gauge is below 3 | `state02_gw_bucket_clear_flash` | `E-state-02-gw-bucket` | |
| Pac-Man key flag only at rank 12 with fruit held | `state02_pacman_key_flag` | `E-state-02-pacman-key` | |
| Robin writes all eight values; Thoron effect at kind 3 | `state02_robin_tomes` | `E-state-02-robin-thoron` | |
| Incineroar: full timer and five aura effects when Revenge saved | `state02_incineroar_revenge` | `E-state-02-incineroar` | |
| Hero clears the max effect before writing | `state02_hero_clear_then_set` | `E-state-02-hero-frizz` | |
| Sora writes (saved+1) mod 3, raises the flag, calls change_magic | `state02_sora_spell_rotation` | `E-state-02-sora-spell` | |
| Olimar: held-first order, prev-colour arithmetic, gated spawns, AIR_FOLLOW | `state02_pikmin_order_and_spawn` | `E-state-02-pikmin-order` | |
| Kirby fields read (copy flag, copied kind, copied charge) | `state02_kirby_read_fields` | `E-state-02-kirby-hat-saved` | |
| No hook on `ArticleModule::get_int` | `state02_no_article_get_int_hook` | — | |

## 10. Open questions

1. Upstream's `ArticleModule::get_int` replacement is a pure pass-through (a leftover). Recommendation: do not reimplement it; one fewer hooked site.
2. Robin's and Incineroar's `charge_max`/aura effects are created with the "unlimited lifetime" argument and no handle is kept for Robin or Incineroar; the game removes them when the move is used. Confirm in Eden that loading twice in a row does not stack a second aura (upstream does not remove the previous one; the respawn cleanup kills fighter effects, so it should not stack).
3. Wario's saved waft count and the Wario buff (SPEC-state-03) both write `GASS_COUNT` on the same frame sequence (charge first, then buff); the buff wins when a waft level is selected. Confirm this ordering is wanted.
