# Save states: character items on load — behavioural specification

- **Spec id:** `SPEC-state-04`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/character_specific/items.rs`, `src/training/save_states.rs` (item call on load), `training_mod_consts/src/options.rs` (`CharacterItem`), `training_mod_consts/src/lib.rs` (Character Item row), `README.md` (item table) at commit `97cbe9b9` (unchanged at `077a8e43`); fork changes at `…/stages/m5i/stage/src/training/character_specific/items.rs` lines 480-488, 492-504, 536-553, `…/stage/src/training/save_states.rs` lines 95-96, 410, 1047-1053, `…/stage/training_mod_consts/src/lib.rs` lines 1033-1039, fork-only files `src/character_items.rs`, `src/turnip_identity.rs`, `training_mod_consts/src/character_items.rs`, `docs/character-items.md` from `docs/provenance/provenance-files.md`
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

When a save state finishes loading, the player's fighter can be handed one of the items that a fighter in the match normally produces (a turnip with a chosen face, Link's Remote Bomb, a Pac-Man fruit, R.O.B.'s Gyro and so on) so that item setups can be practised without pulling the item first. The item belongs to the player's own fighter or to the CPU's fighter, chosen by index in a fixed per-fighter list. The fork extended this into exact held-item snapshots and a stand-alone item menu; those are carried over and only their contact surface with the load sequence is stated here.

## 2. User-visible behaviour

- Menu item (Save States tab): **Character Item**, setting id `character_item`, single-choice, default None. Values: None; Player 1st Var. … Player 8th Var.; CPU 1st Var. … CPU 8th Var. Upstream help: "The item to give to the player's fighter when loading a save state". Fork help (carried over, since the fork made this a fallback): "Item for older saves without an item snapshot. New saves restore the item held when saved; use Character Extras to give an item."
- Live in Training only (fork: and owned lobby-practice restores). Only the **player's** fighter ever receives the item, and never Nana.
- Visible output: on load completion the player's fighter holds the item (or, for Diddy's banana, the banana is produced and handed over). If the chosen fighter has fewer variations than the index, nothing appears. No notification.

### 2.1 Item table (variation index per fighter, in this order)

| Fighter | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Banjo & Kazooie | Grenade Egg (`ITEM_KIND_BUDDYBOMB`) | | | | | | | |
| Bowser Jr. | Mechakoopa (`ITEM_KIND_MECHAKOOPA`) | | | | | | | |
| Daisy | Smile Turnip (article `FIGHTER_DAISY_GENERATE_ARTICLE_DAIKON`, variation `ITEM_VARIATION_DAISYDAIKON_1`) | Winky Turnip (`…DAIKON_6`) | Dot-Eyes Turnip (`…DAIKON_7`) | Stitch-Face Turnip (`…DAIKON_8`) | Mr. Saturn (`ITEM_KIND_DOSEISAN`) | Bob-omb (`ITEM_KIND_BOMBHEI`, `ITEM_VARIATION_BOMBHEI_NORMAL`) | | |
| Diddy Kong | Banana Peel (article `FIGHTER_DIDDY_GENERATE_ARTICLE_ITEM_BANANA`) | Peanut (`ITEM_KIND_DIDDYPEANUTS`) | | | | | | |
| King K. Rool | Crownerang (`ITEM_KIND_KROOLCROWN`) | | | | | | | |
| Link | Remote Bomb (`ITEM_KIND_LINKBOMB`, special path §5.2) | Arrow (`ITEM_KIND_LINKARROW`) | | | | | | |
| Mega Man | Metal Blade (`ITEM_KIND_METALBLADE`) | | | | | | | |
| Pac-Man | Cherry (`ITEM_KIND_PACMANCHERRY`) | Strawberry (`…STRAWBERRY`) | Orange (`…ORANGE`) | Apple (`…APPLE`) | Melon (`…MELON`) | Galaxian (`ITEM_KIND_PACMANBOSS`) | Bell (`…BELL`) | Key (`…KEY`) |
| Peach | Smile Turnip (article `FIGHTER_PEACH_GENERATE_ARTICLE_DAIKON`, `ITEM_VARIATION_PEACHDAIKON_1`) | Winky (`…DAIKON_6`) | Dot-Eyes (`…DAIKON_7`) | Stitch-Face (`…DAIKON_8`) | Mr. Saturn | Bob-omb (normal) | | |
| Richter | Holy Water (`ITEM_KIND_RICHTERHOLYWATER`) | | | | | | | |
| R.O.B. | Gyro (`ITEM_KIND_ROBOTGYRO`, `ITEM_VARIATION_ROBOTGYRO_1P`) | Gyro `_2P` | Gyro `_3P` | Gyro `_4P` | Gyro `_5P` | Gyro `_6P` | Gyro `_7P` | Gyro `_8P` |
| Robin | Book (`ITEM_KIND_BOOK`) | Levin Sword (`ITEM_KIND_THUNDERSWORD`) | | | | | | |
| Sheik | Burst Grenade (`ITEM_KIND_EXPLOSIONBOMB`) | | | | | | | |
| Simon | Holy Water (`ITEM_KIND_SIMONHOLYWATER`) | | | | | | | |
| Snake | Grenade (`ITEM_KIND_SNAKEGRENADE`) | | | | | | | |
| Toon Link | Bomb (`ITEM_KIND_TOONLINKBOMB`) | | | | | | | |
| Villager | Wood Chip (`ITEM_KIND_WOOD`) | | | | | | | |
| Young Link | Bomb (`ITEM_KIND_YOUNGLINKBOMB`) | | | | | | | |

Upstream leaves Snake's cardboard box and Wario's bike out deliberately. Items with no variation use variation 0.

## 3. Inputs

- `character_item` (read on the load completion frame).
- Player and CPU module accessors (by entry id 0 and 1) and their fighter kinds.
- The ItemManager singleton for the hand-over path (§5.2).
- Fork: the player record's `held_item` snapshot (SPEC-state-01 §7.2) and the fork's item engine.

## 4. Outputs and native state touched

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| Player holds an ordinary item | `ItemModule::have_item(player, ItemKind(kind), variation, 0, false, false)` | §5.2 path A | player record, not Nana, load completion |
| Link's Remote Bomb created and linked for detonation | `ArticleModule::generate_article_have_item(generator, FIGHTER_LINK_GENERATE_ARTICLE_LINKBOMB, FIGHTER_HAVE_ITEM_WORK_MAIN, "invalid")` on the generator (the fighter whose kind is Link) | §5.2 path B | — |
| Hand-over from generator to player | `ItemModule::drop_item(cpu, 0.0, 0.0, 0)`; `item = ItemManager::get_active_item(manager, 0)`; `ItemModule::have_item_instance(player, item, 0, false, false, false, false)` | paths B (when the player is not Link) and C | ItemManager non-null |
| Diddy's banana produced | `ArticleModule::generate_article_have_item(generator, FIGHTER_DIDDY_GENERATE_ARTICLE_ITEM_BANANA, FIGHTER_HAVE_ITEM_WORK_MAIN, "invalid")`; `WorkModule::on_flag(generator, FIGHTER_DIDDY_STATUS_SPECIAL_LW_FLAG_ITEM_THROW)`; `ArticleModule::shoot(generator, BANANA, ARTICLE_OPE_TARGET_ALL, false)` | path C, banana | — |
| Turnip produced | `ArticleModule::generate_article(generator, DAIKON article, false, 0)` with the face override of §5.3 active | path C, turnips | — |
| Turnip face probabilities | sixteen replacement hooks on the game's face-probability getters `app::peachdaikon::PEACH_PEACHDAIKON_DAIKON_n_PROBE()` and `app::daisydaikon::DAISY_DAISYDAIKON_DAIKON_n_PROBE()`, n = 1…8 (mangled `_ZN3app11peachdaikon31PEACH_PEACHDAIKON_DAIKON_n_PROBEv`, `_ZN3app11daisydaikon31DAISY_DAISYDAIKON_DAIKON_n_PROBEv`): in Training, while a face override `n` is set, face `n` returns 58.0 and every other face returns 0.0; otherwise the original value | every call | Training |
| CPU mash queue | `mash::clear_queue()` after any item grant, so a queued mash action does not throw the new item at once | end of `apply_item` | — |

## 5. Timing and frame rules

### 5.1 When

On the PosMove completion frame of SPEC-state-01 §5.3 step 8b, for the player record only, not Nana. Fork order (folded as the required order): if the record carries a held-item snapshot, restore that snapshot through the fork's item engine and skip the menu item; otherwise, if Character Item is not None, apply the menu item. Single frame.

### 5.2 Resolving and granting the menu item

1. `idx` = position of the setting: Player 1st … 8th → 1…8 (variation `idx − 1` of the **player's** fighter kind); CPU 1st … 8th → 9…16 (variation `idx − 9` of the **CPU's** fighter kind).
2. Take the `variation`-th entry of §2.1 for that kind. If there is none, do nothing except clear the mash queue.
3. **Generator** = the player when the entry's fighter kind equals the player's kind, else the CPU (the fighter that can produce the item).
4. Grant:
   - **Path A (ordinary item kind, not Link's bomb)**: `have_item` directly on the player with the entry's kind and variation. The generator is not used; the game can create these kinds for any fighter.
   - **Path B (Link's Remote Bomb)**: generate the bomb article in the generator's hand (so detonation stays linked). If the player is not Link, drop from the CPU and hand active item 0 to the player.
   - **Path C (article kinds: turnips and the banana)**: set the face override (§5.3) from the variation (`…DAIKON_8` → 8, `_7` → 7, `_6` → 6, `_1` → 1, anything else → none); produce the article with the generator (banana: generate-have-item, raise the throw flag, shoot; turnip: generate article); drop from the CPU; hand active item 0 to the player; clear the face override.
5. Clear the CPU's mash queue.

The hand-over reads "active item 0" of the ItemManager and relies on the respawn cleanup having removed every other item in the match (SPEC-state-01 §5.2), so the just-produced item is the only one (open question 1).

### 5.3 Turnip face override

A single global optional face number. While set, the sixteen probability hooks force the game's turnip face roll: 58.0 for the chosen face, 0.0 for the others (the game normalises over the eight faces, so the chosen face is certain). Faces: 1 smile (normal), 6 winky, 7 dot-eyes, 8 stitch-face; faces 2–5 exist in the game but are not selectable. The override is set immediately before the article is generated and cleared immediately after the hand-over, within one frame; the hooks are otherwise pass-through. The fork's item engine reuses the same override (interface: a `set_face(Option<u8>)` the fork can call around its own grant).

## 6. Edge cases and interactions

1. Neither fighter is the item's owner kind (for example "CPU 1st Var." when the CPU is Mario): Mario has no entry, nothing is granted.
2. Player and CPU are the same kind: the generator is the player.
3. The player's fighter cannot hold items in its current status (it is in WAIT/FALL at completion, so this does not arise in practice).
4. Nana: skipped. Popo may receive the item.
5. Lobby practice (fork): items that need their fighter's data are produced by a hidden source fighter the fork loads; path C's "drop from CPU" and "active item 0" assumptions are replaced by the fork's engine. The core only calls `restore(snapshot)` / `apply_item(menu)` at the hook point.
6. Hot swap mid-load (fork): the fork cancels pending item work on retirement; nothing to add here.
7. CPU-as-P1 (fork): "player" is entry id 0 regardless of operator.
8. The turnip face override is global: if two turnips were produced in one frame they would share the face; the sequence never does this.
9. Items in Training with items turned on in the game's own item switch: the respawn cleanup removes those too (SPEC-state-01 §5.2); the granted item appears afterwards.

## 7. Persistence

Menu setting `character_item` (carried over in `training_modpack.json`). The fork's held-item snapshot is part of the save-state record (SPEC-state-01 §7.2 `held_item`): `{"state": "item", "choice": u8 (index into the fork's 38-item list), "variation": i32, "lifetime": i32, "lifetime_max": i32, "fuse": i32, "fuse_max": i32}` or `{"state": "empty"}` or `{"state": "unsupported"}`; absent in older saves, in which case the menu item is the fallback. The fork writes the item's remaining lifetime and fuse back only when the item's maxima match the saved maxima (carried over).

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | see docs/resource-costs.md (W5) | ≤ fork; the 45-entry table is static data | `tools/build/nro_size.py` per-feature build |
| per-frame work | see docs/resource-costs.md (W5) | 0 outside the grant frame; sixteen probability hooks each add one optional read per call | diag counters |
| heap allocations in hooks | 0 | 0 | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Index → (role kind, variation) mapping for all 17 values | `state04_setting_to_variation` | — | |
| Table lookup returns the n-th entry per fighter; out of range → none | `state04_table_lookup` | `E-state-04-out-of-range-noop` | |
| Path A: direct have_item with kind and variation | `state04_path_a_direct` | `E-state-04-metal-blade` | |
| Path B: Link bomb via generator; hand-over when player is not Link | `state04_path_b_link_bomb` | `E-state-04-remote-bomb-cpu-link` | |
| Path C: banana flag+shoot, turnip generate, face override set and cleared | `state04_path_c_article` | `E-state-04-stitch-face` | |
| Face hooks: 58.0 / 0.0 while overridden, pass-through otherwise | `state04_face_probability_hooks` | `E-state-04-face-roll` | |
| Mash queue cleared after grant | `state04_clears_mash_queue` | `E-state-04-no-instant-throw` | |
| Player only, never Nana, never CPU | `state04_player_only` | `E-state-04-ice-climbers` | |
| Snapshot present → fork restore, menu skipped; absent → menu item | `state04_snapshot_precedence` | `E-state-04-snapshot-vs-menu` | |

## 10. Open questions

1. The hand-over takes "active item 0" of the ItemManager. It is correct only because the load wiped every item two frames earlier; a stage item spawned in between would be handed over instead. Recommendation: hand over the item whose battle object id the generation returned, or fall back to the newest active item of the expected kind (the fork's engine already searches by kind).
2. Path A gives CPU-owned kinds (Snake's grenade to a non-Snake player) with a plain `have_item`; some of these kinds behave oddly without their owner's data in the match. Upstream accepts this. Recommendation: keep, document per item in the user guide which ones need the owner present.
3. The menu's variation list is positional ("Player 3rd Var.") and does not name the item; the fork replaced it with a named, iconed menu (carried over). Recommendation for the menu redesign: keep the positional setting only as the legacy fallback the fork made it.
4. The sixteen symbol hooks cost sixteen guarded sites for one feature. Recommendation: confirm with the lead that the turnip faces justify them; otherwise restrict to the four selectable faces (eight hooks) and leave faces 2–5 at their game probability.
