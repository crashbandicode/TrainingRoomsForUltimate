# Menu rendering on the native UI and menu input — behavioural specification

- **Spec id:** `SPEC-core-03`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), spec-writer role, 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/ui/menu.rs`, `src/training/ui/mod.rs` (draw entry, `set_icon_text`, `fade_out`), `src/common/menu.rs` (input handler, open/close), `src/common/button_config.rs` (open chord, start-press logic), `src/common/input.rs` (controller structures), `src/training/frame_counter.rs` (the close counter), `src/training/mod.rs` (the input-mapping hook order) at commit `97cbe9b9` (unchanged in `077a8e43`); fork changes at stage M5i `src/training/ui/menu.rs` lines `6, 66-142, 163-240, 270-278, 324-327, 329-347, 350, 356, 367, 384-398, 401-407, 414, …`, `src/common/menu.rs` `11, 15, 18-21, 23-33, 35-60, 71-79, 82-83, 95-97, 100-101, 107-115, 118-127, 130-134, …`, `src/common/button_config.rs` `3, 12-71, 226-227, 230, 234, 240-241, 253-254, 257-287, 292-296, 300-304, 307, 309-346, …`, plus the fork-only `training_mod_consts/src/menu_shortcuts.rs` and `docs/menu-shortcuts.md`, from `docs/provenance/provenance-files.md`.
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

The settings menu is drawn by the game's own 2D layout engine: the plugin adds panes to the Training HUD layout (SPEC-core-05) and, once per frame, writes titles, colours, visibility and glyphs into those panes to show the model of SPEC-core-02. While the menu is open the plugin reads Player 1's controller before the game does, turns the presses into model events, and hands the game an empty input so nothing happens in the match; for a short time after closing it also swallows inputs so the closing press does not become an attack. The menu opens on a chord, hides itself behind the game's own pause menu, and commits its settings when it closes.

## 2. User-visible behaviour

### 2.1 Opening

- **Permanent chord:** hold B and press D-pad Up, or hold D-pad Up and press B (the generic "hold any one button of the chord, press the others this frame" rule). Always available in Training; cannot be configured away. Upstream README: "Hold DPAD UP and press B while in training mode."
- **Upstream `Menu Open Start Press` (default On):** pressing **Minus** opens the menu (unless the game's pause menu is open). Holding **Plus (Start)** for 10 or more frames opens the menu; while Plus is held the game does not see it; releasing Plus after 1–9 held frames re-injects a Plus press so the game's pause menu opens normally. After release the plugin waits 2 frames before trusting a new Plus. The fork **replaced** this with configurable shortcuts read from an INI at boot (§6, fork's own, carry over); Plus then always goes straight to the game's pause menu. Q1 asks which contract the clean room ships.
- The chord is only accepted when neither the game's pause menu nor this menu (nor, in the fork, a named-macro panel or a replay reload) owns input.
- Opening: the model goes to its submenu page, the menu becomes active, and a redraw is requested. The fork also re-synchronises the model from the committed settings on every open (SPEC-core-02 §6 item 7).

### 2.2 While open

- The menu panel appears over the Training HUD; the native right-hand status panel (`status_R`) and the plugin's notification panel are hidden; the game's HUD elements are hidden too (SPEC-core-05 HUD rule includes "menu open").
- Title bar text: "Modpack Menu" (the clean room uses its own name). Three tab labels: previous tab (with its key glyph), **current tab in yellow**, next tab (with its key glyph). Key hints: `Save Defaults` with the X glyph; `Reset All` (submenu page) or `Reset Current` (toggle/slider page) with the R glyph (Pro) or Z glyph (GameCube), hidden on the confirmation page; `Clear Toggle` with the Y glyph only when the model says so (SPEC-core-02 §4).
- Grid: up to 8 rows × 4 buttons. Each button shows a title; the **selected** button has a blue background and white text with shadow and outline; the others a near-black background and grey-blue text.
  - On the submenu page each button also shows the submenu's **icon** (SPEC-core-07) and no check or number.
  - On a toggle page of a single-choice or unweighted submenu each button shows a **check** picture when its value is 1 (the whole icon area hidden when 0).
  - On a weighted toggle page each button shows the digit picture `1`..`5` for its weight (nothing for 0).
- Footer: the selected submenu's help text (submenu page) or the confirmation question (confirmation page).
- Slider page: a separate panel with a header (the submenu title), a **Min** button and a **Max** button, each showing its value; the hovered handle is highlighted blue, the selected (being edited) handle orange.
- Confirmation page: only the middle row is visible with two buttons, **No** (left) and **Yes** (right); the hovered one is highlighted.
- Controller glyphs follow the connected controller type: GameCube controllers show L/R for tab switching and Z for reset; everything else shows ZL/ZR and R.
- Hold repeat on the sticks (every 21 frames) as SPEC-core-02 §2.3.

### 2.3 Closing

- B on the submenu page closes. (Upstream also defined Start as "close" in the model but never wired it; the fork wires nothing either. Q2.)
- On close: the panel fades out over 15 frames (logarithmic alpha curve), the settings are committed and written (SPEC-core-04), features see the new values from the next frame, and for those 15 frames **all** Player 1 inputs are discarded (the game sees no buttons, no sticks) so the closing B press does not become a special attack and a Plus press does not reopen the pause menu (upstream changelog: "Menu exit inputs (B, Start) no longer have effects in-game").
- If every controller is unplugged while the menu is open (no style set on controller ids 0–7 and 0x20), the menu closes without committing.

### 2.4 Game pause menu meanwhile

- The game's pause menu is detected from its own layout: the help footer pane `L_staying_help` of `info_training` rests at `pos_y = -80` and animates to 0 over 10 frames when the pause menu opens (`info_training_in_menu` animation); `pos_y != -80` means "pause menu active". While it is active the plugin's panel is hidden even if the model says open, and all plugin chords are refused.
- Upstream, with `Menu Open Start Press` on: pressing Plus while the pause menu is open closes it (the plugin passes the press through) and never opens the plugin menu.

## 3. Inputs

- The game's per-player final input mapping function (`main+0x1750dd0`, SPEC-core-01 §4.2): arguments `(controller mappings*, player index, out mapped inputs*, controller holder*, flag)`. The plugin runs the original first, then reads/mutates Player 1 (`player index == 0`) only. The controller holder contains a pointer to the game's controller record at `+0x10`.
- Controller record fields used (game structure, 13.0.5; offsets from the record start): `current_buttons` (+0x8, 32-bit button bitfield), `previous_buttons` (+0xc), left stick x/y (+0x10/+0x14), triggers, `just_down` (+0x50), `just_release` (+0x54), `style` (+0x60: Handheld 1, Dual Joy-Con 2, Left Joy-Con 3, Right Joy-Con 4, Pro Controller 5, Debug Pad 6, GameCube 7). The button bitfield bit order: dpad up, dpad right, dpad down, dpad left, x, a, b, y, l, r, zl, zr, left SL, left SR, right SL, right SR, stick L, stick R, plus, minus, left-stick up/right/down/left, right-stick up/right/down/left, real digital L, real digital R.
- The mapped output record: `buttons` (game action bitflags), left/right stick x/y as `i8`. "Empty" is all zero.
- Controller type to logical-button mapping (also used by the chord features, SPEC-input-*): A/B/X/Y direct; `L`: Pro `l`, GameCube none; `R`: Pro `r`, GameCube `zr`; `ZL`: Pro `zl` or either SL, GameCube `l` or `real_digital_l`; `ZR`: Pro `zr` or either SR, GameCube `r` or `real_digital_r`; d-pad, plus, minus, stick clicks direct.
- Directions for the model: d-pad bits, left-stick direction bits and right-stick direction bits all count as Left/Right/Up/Down (press edge for d-pad and sticks; hold repeat for sticks only).
- `GetNpadStyleSet(id)` for ids 0..7 and 0x20 (all-controllers-unplugged check).
- The model (SPEC-core-02) and these flags: menu active, "redraw requested", "pause menu active", the Player 1 controller style (kept for the renderer), the close frame counter.
- The layout panes of `info_training` listed in §4.

## 4. Outputs and native state touched

### 4.1 Hooks

| Native state | How it is reached | When | Guard |
| --- | --- | --- | --- |
| Final input mapping | replace hook at `main+0x1750dd0` (entry; the function starts with a frame setup, 64-byte guard in SPEC-core-01) | every frame, every player | 64-byte guard; HOOK-SHARING chain if another plugin owns the entry |
| Player 1 mapped output | the `out` record written by the game, overwritten with the empty record while the menu is open and for 15 frames after closing | per frame | — |
| Player 1 controller record | `current_buttons`, `previous_buttons`, `just_down`, `just_release` zeroed for 15 frames after closing (so downstream plugin features see nothing either); upstream start-press logic cleared only the Plus bit while Plus was held and re-set it on a short release | per frame | — |
| UI tick | SPEC-core-05 (per-layout anchors) | per `info_training` calculate | — |

The order of sub-handlers inside the input hook is load-bearing: (1) developer-config reload (dev only), (2) **menu input** (may empty the output and the controller record), (3) chord requests (reads the possibly emptied record, so no chord fires while the menu is open or closing; also the Plus-hold logic), (4) input delay (SPEC-input-*), (5) input log (reads the delayed state), (6) input recording/playback (mutates the output last). Nothing runs outside Training mode (the hook returns right after the original).

### 4.2 Panes written (all inside the `info_training` layout; parts come from the injected archive, SPEC-core-05/07)

| Pane | Type | Written |
| --- | --- | --- |
| `TrModMenu` | container | visible = menu active and pause menu not active; alpha/global alpha from the fade |
| `TrModMenuButtonRow0` … `TrModMenuButtonRow7` | parts (`training_mod_menu_button_row`) | visible per page |
| `Button0` … `Button3` inside each row | parts (`training_mod_menu_button`) | visible per cell |
| `TitleTxt` in a button | text box | title; colour white (255,255,255) + shadow + outline when selected, else (178,199,211); no shadow/outline |
| `TitleBg` in a button | picture (material) | white/black material colours: selected (0,28,118,255)/(0,22,112,0); unselected (8,13,17,255)/(5,10,14,0); slider handle being edited (255,166,7,255)/(240,154,7,0) |
| `Icon` in a button | container | visible on the submenu page and when a toggle value is non-zero |
| `<submenu id>` pictures stacked inside `Icon` (one per upstream submenu id; `save_damage_limits_playe` for `save_damage_limits_player` because pane names are at most 24 characters) | picture | exactly the selected submenu's icon visible on the submenu page; all hidden elsewhere |
| `check` in a button | picture | visible on single/unweighted toggle pages |
| `1` … `5` in a button | pictures | the weight digit visible on weighted toggle pages |
| `ValueTxt` in a button | text box | hidden on the grid (used by the slider buttons) |
| `TrModSlider` | parts (`training_mod_menu_slider`) | visible only on the slider page |
| `Background`, `Header`, `MinButton`, `MaxButton` (+ `TitleTxt`, `TitleBg`, `ValueTxt`, `Icon` inside each button) in the slider | picture / text / parts | header = submenu title; Min/Max titles; values; hover/selected colours as above; icons hidden |
| `TrModTitle` → `set_txt_00`, `set_txt_01` | text boxes | the menu title |
| `LeftTab`, `CurrentTab`, `RightTab`, `SaveDefaults`, `ResetDefaults`, `ClearToggle` → `set_txt_icon`, `set_txt_help` | key-help parts (clones of the game's `com_btn_keyhelp_00`) | icon = one private-use glyph (§4.3), help text; `CurrentTab` text in yellow (255,255,0) with default material colours |
| `FooterTxt` | text box | help / question |
| `status_R` (native) | container | hidden while the menu is open (fork: also while a modal panel is open); upstream panicked if missing, the fork tolerates its absence — fold in |
| `L_staying_help` (native) | read only | `pos_y != -80` → pause menu active |
| `TrModDisp0` (notifications) | container | hidden while the menu is open (SPEC-input-* notifications draw) |

Pane write API (library bindings, not upstream code): find a pane by name recursively; set visible; set alpha and global alpha; read/write position; text box: set text string, write UTF-16 glyph codes directly into the text buffer and set its length (for key glyphs), set colour, set default material colours, enable/disable shadow and outline; picture: material white/black colours.

### 4.3 Key glyphs (UTF-16 private-use code points in the game's font)

| Logical key | Pro Controller / Joy-Con | GameCube |
| --- | --- | --- |
| L | `0xE0E4` | `0xE204` |
| R | `0xE0E5` | `0xE205` |
| X | `0xE0E2` | `0xE206` |
| Y | `0xE0E3` | `0xE207` |
| ZL | `0xE0E6` | — (GameCube L is `0xE204`) |
| ZR | `0xE0E7` | — (GameCube R is `0xE205`) |
| Z | — | `0xE208` |
| A | `0xE0E0` | `0xE0E0` |
| D-pad Up / Down / Left / Right | `0xE079` / `0xE07A` / `0xE07B` / `0xE07C` | `0xE209` / `0xE20A` / `0xE20B` / `0xE20C` |
| Plus | `0xE0EF` | `0xE20D` |
| Minus | `0xE0F0` | none |
| Left / Right stick press | `0xE104` / `0xE105` | none |

Menu hints: tab left/right = ZL/ZR (GameCube L/R); save defaults = X; reset = R (GameCube Z); clear = Y. (B has no distinct glyph upstream, "TODO".)

## 5. Timing and frame rules

- **Input** is processed in the final-input-mapping hook, once per frame per player, on the game thread, after the game mapped the raw controller.
- **Drawing** happens in the UI tick for the `info_training` layout (SPEC-core-05): once per frame on the UI job thread (and once more when the layout is registered). The tick does, every frame: pause-menu detection, panel visibility, fade, `status_R` visibility, slider panel hidden; and, **only when a redraw was requested** by the input handler (or by open), the full rebuild: title, hide every row/button/value, key hints, then the page renderer. Requesting a redraw only on input keeps the per-frame pane traffic small (upstream design; required).
- The close counter is a "real frame" counter that ticks in the `info_training` UI tick (not in the fighter frame), so it also advances while the game is paused or fighters are gone. The input hook discards Player 1 inputs while the counter is between 0 and 15 **and counting** (fork fix: upstream required `count > 0`, which let the very first frame after closing through before the first UI tick advanced the counter; fold in), and stops/resets it at 15.
- The fade uses `remaining = 15 − count` frames: alpha = `255 / log10(16) × log10(remaining + 1)` for `0 < remaining < 15`, 255 at ≥ 15, 0 at 0 (fork: saturating subtraction, fold in).
- A setting changes effect **after** the menu closes (commit on close), never while open.
- Lock discipline: input and draw both lock the model; neither may call into the game while holding it beyond pane writes. The fork's rule "atomics only while the model lock is held" and its named-macro/replay-capture exceptions are carry-over.

## 6. Edge cases and interactions

- **Pause menu open while our menu is open**: our panel hides; chords refused; input still captured by us (model keeps state). Q3: should the pause menu be refused while our menu is open instead? Upstream's Plus handling (with start-press on) suppressed Plus while held, so the pause menu could not open while our menu was open; with the fork's INI design Plus passes through unless a configured chord claims it.
- **All controllers unplugged**: menu closes without commit; nothing is written.
- **Controller type changes while open**: glyphs follow the Player 1 style read every input frame.
- **L+R+A reset while open**: the reset is a game feature on P1's buttons; with inputs emptied it cannot fire while the menu is open.
- **Lobby practice (fork)**: the same panel is hosted in the waiting-room layout `info_waiting` under a `TrModPort` container; the pause-menu detection there uses the native `option_menu_setting` pane (`pos_x != -540` → busy) or the game's popup-manager busy state; title "Lobby Practice"; some rows show "Available in Training mode only." in the footer. Carry over as the fork's own.
- **CPU-as-P1 / hot swap (fork)**: when a swap is in progress `is_training_mode()` reports false and the menu cannot open; the model's pending intents are taken at close.
- **Second player**: only Player 1 is read and blocked; other controllers keep working (a CPU-controlling second controller still moves the CPU while the menu is open). Q4.
- **Another plugin owning the input-mapping entry**: HOOK-SHARING chain; the Deluxe family does not hook it (fork survey); the fork's hook order inside the function is internal.
- **Icon stacking**: upstream hid stacked icons by walking every submenu id of every tab for every button (cloning the whole model per render); the fork hides every child of `Icon` by walking the pane's child list. Fold in the child-list walk (no per-render clone).
- **Fork's INI shortcuts (carry over, fork's own):** `sd:/…/button_config.ini`, section `[menu]`, keys `shortcut_1` (default `Minus`) and `shortcut_2` (default `disabled`); names case-insensitive, chords joined by `+` (A, B, X, Y, L, R, ZL, ZR, DPad_Up/Down/Left/Right, Plus, Minus, LStick, RStick and the display labels); `disabled`/`none` disable a slot; malformed or duplicate keys fall back per slot; file ≤ 8 KiB; created with the defaults when absent; read once at boot. A shortcut passes when every button of the chord is held and at least one was pressed this frame (edge-triggered, no repeat while held). An explicit `Plus` chord consumes the pause press. The permanent B + D-pad Up chord always works.
- **Fork extras in the renderer (carry over):** roster grid and stage grid overlays, item-icon flash animation (24 draw callbacks, one tile, colour blend), named-macro panel rendering, Mii catalogue view with greyed rows, GameCube L/R paging on the stage page.

## 7. Persistence

None here; the commit on close is SPEC-core-04; the shortcuts INI is described above. The menu's cursor positions are not persisted.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | not measured separately | see `docs/resource-costs.md` (W5) | `tools/build/nro_size.py` per-feature build |
| per-frame work (menu closed) | one pane visibility write and one fade computation per `info_training` tick; one bitfield read per input frame | same | diag counters |
| per-frame work (menu open, no input) | visibility/fade only (no rebuild) | same | diag counters |
| per-input rebuild | up to 32 buttons × ~8 pane lookups by name (recursive string search) | bounded; cache pane pointers for the lifetime of one layout registration (Q5) | diag timing |
| heap allocations in hooks | upstream allocated per input (format strings for pane names, model clone per render) | 0 per frame; allocation only on input events | counting allocator |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Permanent chord opens (hold either, press the other) | `core03_open_chord_rule` | `core03-E1` | `docs/EDEN-RESULTS.md` |
| Chord refused while pause menu / our menu / modal owns input | `core03_chord_gates` | `core03-E2` | — |
| Menu open empties P1 output and reads presses; other players untouched | `core03_input_capture_p1_only` | `core03-E3` P2 controller still moves CPU | — |
| Close: 15-frame input discard incl. the first frame; fade curve | `core03_close_window_15_frames`, `core03_fade_curve` | `core03-E4` B on close does not attack | `docs/EDEN-RESULTS.md` |
| All controllers unplugged closes without commit | `core03_unplug_closes` | `core03-E5` | — |
| Pause-menu detection from `L_staying_help`; panel hidden; chords refused | `core03_vanilla_menu_detect` (pos_y → flag) | `core03-E6` | — |
| Pane write table §4.2 (which panes, which values per page) | `core03_render_plan_per_page` (model → pane write list) | `core03-E7` screenshots of every page | `docs/EDEN-RESULTS.md` |
| Glyph table §4.3 per controller style | `core03_glyph_table` | `core03-E8` GameCube vs Pro | — |
| Redraw only on input; visibility every tick | `core03_redraw_flag` | `core03-E9` diag counter steady with menu open | — |
| Sub-handler order inside the input hook | `core03_input_hook_order` (sequence model) | `core03-E10` | — |
| Upstream start-press variant or fork INI variant (Q1) | `core03_start_press_state_machine` / fork tests | `core03-E11` | — |

## 10. Open questions

1. **Open-chord contract.** Upstream: B + D-pad Up always; optional Minus press and 10-frame Plus hold with Plus re-injection. Fork: B + D-pad Up always, INI shortcuts (default Minus), Plus never implicit. Recommendation: ship the fork contract (fewer input side effects, host-tested parser) and do not implement the Plus-hold state machine.
2. **Start to close the menu.** Defined in the model but never dispatched upstream. Recommendation: do not wire it (Plus must keep meaning "pause" for the game).
3. **Pause menu versus our menu.** Hide (upstream) or refuse the pause menu while ours is open. Recommendation: refuse Plus while our menu is open (suppress the bit as the fork does for modal panels), so the two menus never overlap.
4. **Second controller while the menu is open.** Upstream blocks only Player 1. Lobby practice and CPU control make a second controller meaningful. Recommendation: keep upstream behaviour; document it.
5. **Pane lookups by name per rebuild.** Upstream searches the pane tree by string for every write. Recommendation: resolve the pane set once per layout registration (the registration anchor tells us when the layout is new) and store raw pointers until the next registration or scene change; measure first.
6. **Name limit.** Pane names are 24 characters; the icon for `save_damage_limits_player` was truncated. The clean-room archive should use short icon pane names (e.g. `ic_<nn>`) mapped from submenu ids in code.
