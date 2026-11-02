# HUD notifications and damage display — behavioural specification

- **Spec id:** `SPEC-input-05`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude (Fable 5.1), spec-writer for area `input`, 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/ui/notifications.rs`, `src/training/ui/display.rs`, `src/training/ui/damage.rs`, `src/training/ui/mod.rs` (HUD dispatch, HUD toggle), `src/common/mod.rs` (pause-menu struct, damage digits, ready-go), the stale-moves inline hook in `src/training/mod.rs`, all at commit `97cbe9b9` (unchanged at `077a8e43`). Fork changes at `…/stages/m5i/stage/src/training/ui/display.rs` line 28, `…/stage/src/training/ui/native_menu.rs` (whole file), `…/stage/src/training/mod.rs` lines 640–651 (stale hook rewrite), `…/stage/src/training/ui/mod.rs` lines 138–230 (tick), `…/stage/src/training/mod.rs` line 101 (session clear). `ui/notifications.rs` and `ui/damage.rs` are upstream-identical in the fork.
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

Two small HUD helpers. Notifications: a queue of short messages (a header and a body, in a colour, for a number of frames) shown one at a time in the modpack's own HUD slot, used by features such as input recording, save states, named macros and the lobby quick reset; hidden when the game's own Combo Display option is off. Damage display: while a save state kills and respawns the fighters, the game's percentage digits are forced visible and correct at once and the death explosion graphics are hidden, so the HUD never shows a blank or exploding percent during a reset.

## 2. User-visible behaviour

### 2.1 Menu items

None of their own. Related: the game's Training menu **Combo Display** option (off → no notifications), and the modpack's **HUD** toggle (`hud`, Misc tab, default On; off hides the game's `info_playercursor`, `info_playercursor_item`, `info_melee`, `info_radar_a`, `info_radar_b` layouts while a match is running — HUD spec).

### 2.2 Notifications

- One notification visible at a time, in the HUD part `TrModDisp0`: header text box `TrModDisp0Header`, body text box `TrModDisp0Txt` with the body in the notification's colour.
- A notification lasts its length in UI frames; when it ends the next queued one appears.
- Hidden while the mod menu is open (the queue keeps ticking only when visible — see §5).
- When the game's Combo Display option is off, the queue is emptied every HUD tick, so nothing shows.
- Users in this area (SPEC-input-01 §2.3): "Input Recording" / "Lockout", "Standby", "Recording" (60 frames, coloured) and "Playback" (black, length = playback length). Other features post their own (save-state spec, named macros, lobby reset "Quick Reset" / "Wait for both practice fighters to finish loading", 120 frames).

### 2.3 Damage display

No new element. During a save-state kill/respawn in Training the percentage digits of both fighters stay visible and show the current damage, and the death explosion and fire graphics of the percent display do not play.

## 3. Inputs

- Notification API calls from other features: post (header, body, length, colour — default colour black 0,0,0,255), clear by header, clear all.
- Mod-menu-open flag.
- The game's Combo Display option: a byte at offset 0xB6C in the game's Training pause-menu object (`PauseMenu`; also holds the stale-moves toggle at 0xB60).
- Damage: the fighter manager's "ready go" flag (`FighterManager::is_ready_go`), training mode, the layout name of the HUD layout being ticked (`info_melee`), each fighter's damage (`DamageModule::damage(accessor, 0)`) split into hundreds, tens, ones and tenths (zeros when the accessor is missing — this happens when background matchmaking unloads the fighters).

## 4. Outputs and native state touched

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| Combo Display value (read) | Upstream: an inline hook one instruction after the game writes the stale-moves toggle (`main+0x13e88c4`, register x22 = PauseMenu pointer) **retained the PauseMenu pointer** and read `+0xb6c` from it on every HUD tick. Fork fix (fold in): never retain the pointer; copy the byte the game itself just loaded at two inline sites following its `ldrb` of `PauseMenu+0xb6c` — `main+0x13e549c` (register w20) and `main+0x13e54d8` (register w8) — verified by the four words at `main+0x13e5498` (`396db114 6b14013f 540002c0 34000234`) and `main+0x13e54d4` (`396db108 3908c268 320003e8 3908be68`); values other than 0/1 are ignored; the copy resets to "enabled" on scene change; if the words differ, nothing is hooked and notifications stay visible. | at the game's own menu update | both pins match (fail closed to enabled) |
| Stale-moves toggle (adjacent, owned by another spec but shares the site) | inline at `main+0x13e88c4`: write byte 1 at `PauseMenu+0xb60` (fork: byte, after checking x22 ≠ 0 and native Training) | on menu construction | needle for `main+0x13e88c0`/`+0x13e88c4` pair resolves (the two sites sit inside each other's 64-byte windows; STALE_MENU hooks first) |
| Notification panes `TrModDisp0` (visibility), `TrModDisp0Header` and `TrModDisp0Txt` (text; body: default material colours then the colour) | modpack panes inside `info_training` (lobby practice: inside `TrModPort` of `info_waiting`) | HUD tick | panes exist (own layout archive) |
| Percent display panes under the `p1` and `p2` parts of `info_melee`: `set_dmg_num_3`, `dig_3`, `dig_3_anim`, `set_dmg_num_2`, `dig_2`, `dig_2_anim`, `set_dmg_num_1`, `dig_1`, `dig_1_anim`, `set_dmg_num_p`, `dig_dec`, `dig_dec_anim_00`, `set_dmg_num_dec`, `dig_dec_anim_01`, `dig_0_anim`, `set_dmg_p` — `pos_y`, `alpha`, `global_alpha`; `set_fxui_dead1/2/3`, `set_fxui_fire` — `alpha`, `global_alpha` ← 0 | game-owned panes found by name from the layout root (missing panes are skipped) | HUD tick of `info_melee` in Training while ready-go | names found; Training; ready-go |
| The `p1`/`p2` part layout's animation transform list: for material animations named `set_dmg_num_3 / _2 / _1 / _dec`, the transform's current frame ← the digit (hundreds / tens / ones / tenths) | walk the part's `anim_trans_list` (circular list; stop when a node points to itself or back to the head); each node's transform points at a resource animation block with `anim_cont_count` contents at `anim_cont_offsets_offset`; a content has a name and a type (1 = material) | only when a digit pane's alpha was not 255 (at most once per fighter per tick) | list well-formed |
| HUD entry point | the Revision 29 UI anchors (SPEC-input-02 §4): per layout per frame, after animate, before calculate | | five-word pins |

Rules: notifications never allocate in the HUD tick beyond the queue itself (open question 2); the damage helper never writes outside the `p1`/`p2` subtrees.

## 5. Timing and frame rules

### 5.1 Notifications

Each HUD tick of the training (or waiting-room host) layout:

1. If Combo Display is off: clear the queue (this also hides the pane in step 3).
2. Take the first queued notification.
3. `TrModDisp0` visible = a notification exists ∧ the mod menu is closed.
4. If none: stop. Otherwise decrement its length by one. On its first draw: write header and body, reset the body's material colours to default, set the body colour. If length ≤ 1 after the decrement: remove it; the next tick shows the following one.

Posting appends; clearing by header removes every queued notification with that header (used by the recorder to replace its phase message each frame); clear-all empties the queue (fork: also on session clear). Lengths are UI frames, so a 60-frame notification lasts one second at 60 fps, independent of game pause (the HUD keeps ticking under the vanilla pause menu; open question 3).

Length 0 is invalid (the decrement would underflow); treat a posted length < 1 as 1 (open question 1).

### 5.2 Damage display

Each HUD tick of `info_melee` while in Training and ready-go, for each of `p1` (Player entry) and `p2` (CPU entry — in lobby practice the CPU entry is 4 via the same resolver): compute the digits; for each pane name in the list of §4: skip the hundreds panes (names containing `3`) when hundreds = 0, and the tens panes (names containing `2`) when hundreds = tens = 0; set `pos_y` to 0 (`set_dmg_p`), −4 (`set_dmg_num_p`), −16 (`dig_dec`) or 0 (all others); if `alpha` or `global_alpha` ≠ 255 set both to 255 and, once per fighter per tick, rewrite the animation frames to the digits. Then set the four death-effect panes' `alpha` and `global_alpha` to 0.

This runs every tick, not only during a save-state load (open question 4).

### 5.3 Setting changes

Combo Display turned off while a notification shows → it disappears on the next tick and the queue is emptied. HUD toggle off hides the game HUD layouts; the modpack's own `TrModDisp0` lives in `info_training`, which the HUD toggle does not hide (open question 5).

## 6. Edge cases and interactions

1. **Retained PauseMenu pointer (fork fix, fold in).** Upstream dereferenced a pointer captured when the Training menu was constructed on every HUD tick; after a scene change the object is gone (use-after-free; the fork's console crash analysis attributes a class of freezes to retained native pointers). Never retain the pointer; copy the value at the game's own load sites, or read the option through a documented accessor if one is found.
2. **Recorder phase messages** re-post every frame (SPEC-input-01 §8); the queue therefore has one "Input Recording" entry during a phase and the first-draw text write repeats each tick. Fold-in target: post once per phase change (observable result unchanged).
3. **Lobby practice:** notifications draw inside `TrModPort`; the Combo Display option does not exist in the waiting room — the fork gates the clear on native Training mode, so notifications always show in the lobby.
4. **Pokémon Trainer / hot swap:** damage digits are read from the current body each tick; during a swap the resolver returns nothing and the digits read 0 — the helper then writes zeros into the animation frames (upstream behaviour; open question 4).
5. **Ice Climbers:** `p2` resolves to Popo; Nana has no percent display.
6. **Mod menu open:** `TrModDisp0` hidden; the queue still ticks (the HUD layout is still ticked), so a short notification can expire unseen. Open question 3.
7. **Save-state load:** the save-state spec posts its own notifications and relies on the damage helper for the HUD during kill/respawn; the hitbox visualiser and HUD toggle are separate.
8. **Background matchmaking unloading the fighters:** digits read as zeros; the helper must tolerate a missing accessor without writing anything else.

## 7. Persistence

None.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | queue (growable vector of {two strings, u32, colour, bool}); code not isolated — see `docs/resource-costs.md` (W5) | fixed-capacity queue (e.g. 8 entries) with static-string headers | `tools/build/nro_size.py` per-feature build |
| per-frame work | notifications: 1–3 pane lookups per tick; damage: up to 2 × (1 part lookup + 20 pane lookups) per `info_melee` tick plus an animation-list walk when alpha was altered | notifications ≤ 3 lookups; damage: lookups only while a save-state kill/respawn is in progress (open question 4) | diag counters |
| heap allocations in hooks | notifications: 2 string allocations + 1 push per post (per frame during recorder phases); damage: 0–2 (layout-name string conversions upstream) | 0 in steady state | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Queue order, one visible at a time, length countdown, removal at ≤ 1, length < 1 treated as 1 | `notify::queue_order_and_lengths` | IN05-E01 (two notifications in sequence) | `evidence/input/IN05-E01.md` |
| Clear by header removes all with that header; clear-all; session clear | `notify::clear_by_header_and_all` | IN05-E02 | |
| Hidden under the mod menu; emptied when Combo Display is off | `notify::hidden_and_combo_display_off` | IN05-E03 (toggle Combo Display in the game menu) | |
| Combo Display value copied at the game's load sites; no retained pointer; fail-closed to enabled on pin mismatch | `notify::combo_display_pins` (word table) | IN05-E04 (scene change then notification) | |
| Text and colour written on first draw only | `notify::first_draw_writes` | IN05-E01 | |
| Damage digits computed from damage (e.g. 123.4 → 1, 2, 3, 4; 7.0 → 0, 0, 7, 0) | `damage::digits` | — | |
| Pane rules: skipped leading zeros, pos_y values, alpha forcing, animation frame rewrite once per fighter, death effects hidden | `damage::pane_rules_fixture` | IN05-E05 (save-state load; percent never blank or exploding) | |
| Missing accessor → zeros, no other writes | `damage::missing_accessor_zero` | IN05-E06 (hot swap) | |
| Lobby practice: notifications visible inside the host part | — | IN05-E07 | |
| Zero heap allocations in the HUD tick in steady state | `notify::tick_allocates_nothing` | — | install line |

## 10. Open questions

1. **Length 0 underflow.** Upstream decrements an unsigned length before checking completion; a 0-length post (e.g. "Playback" for an empty slot) underflows. Recommendation: clamp to ≥ 1 at post time and check completion before decrementing.
2. **String headers.** Upstream headers and bodies are heap strings posted per frame by the recorder. Recommendation: static-string headers, bodies either static or small fixed buffers; post once per phase change.
3. **Clock.** Notifications tick per HUD draw, so they expire during the vanilla pause and while hidden under the mod menu. Recommendation: tick only when visible (parity change, user-visible only as "notifications wait for the menu to close"). **Decided 2026-11-01 (B8 batch): accepted.**
4. **Damage helper runs every frame.** Its purpose is the save-state kill/respawn window, but upstream runs it on every `info_melee` tick in Training (about 40 pane lookups per tick) and writes zero digits while a fighter is unloaded. Recommendation: run it only while a save-state load (or lobby quick reset) is in progress, and skip writes when the accessor is missing. Needs the save-state spec writer's agreement on the "load in progress" signal.
5. **HUD toggle and `TrModDisp0`.** The modpack's notifications remain visible with HUD off. Recommendation: keep (the toggle is about the game's HUD); confirm with the menu redesign. **Decided 2026-11-01 (B8 batch, kept line): kept as the fork does it** (with HUD Off the plugin's notices stay visible).
6. **Site sharing.** The stale-moves site pair (`main+0x13e88c0`/`+0x13e88c4`) and the two Combo Display observer sites are inline hooks in game menu code that other plugins may also hook; the playbook's hook-sharing rules apply if the card scan shows another owner.
