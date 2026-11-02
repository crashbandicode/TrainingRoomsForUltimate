# Button chords — behavioural specification

- **Spec id:** `SPEC-input-04`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude (Fable 5.1), spec-writer for area `input`, 2026-11-01
- **Upstream reference read (spec writer only):** `src/common/button_config.rs`, `src/common/input.rs` (controller structures), `src/common/menu.rs` (menu-open consumer and close grace), `src/training/ui/menu.rs` (vanilla-menu detection), the `ButtonConfig` option and the Button Config tab in `training_mod_consts/src/{options,lib}.rs`, consumers in `src/training/{save_states,input_record,mod}.rs`, all at commit `97cbe9b9` (unchanged at `077a8e43`). Fork changes at `…/stages/m5i/stage/src/common/button_config.rs` lines 3, 12–71, 226–227, 230, 234, 240–241, 253–254, 257–287, 292–296, 300–304, 307, 309–346, 362–413, 415–484, `…/stage/training_mod_consts/src/menu_shortcuts.rs` (whole file), `…/stage/training_mod_consts/src/lib.rs` lines 120–123, 384, 913, 942–955, `…/stage/src/training/ui/menu.rs` lines 623–639, plus `docs/menu-shortcuts.md`.
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

The player triggers save-state save and load, input recording and playback, and the mod menu with button chords on P1's controller, configurable from the menu. A chord is "hold any one button and press the others". Buttons are logical (A, B, X, Y, L, R, ZL, ZR, D-pad, Plus, Minus, stick clicks) and are mapped to the physical controller by its style, so a GameCube controller's Z and triggers behave sensibly. Chords are ignored while any menu owns the controller. The fork's INI-configured menu shortcuts and character-cycle chords sit on the same mechanism and are carried over; §9 states their interface.

## 2. User-visible behaviour

### 2.1 Menu items (Button Config tab, id `button`)

| Item | Setting id | Type | Values | Default |
| --- | --- | --- | --- | --- |
| Menu Open Start Press (upstream only; removed by the fork, §6.1) | `menu_open_start_press` | On/Off | | On upstream; Off in the fork, kept only as a saved field |
| Save State Save | `save_state_save` | multi choice (unweighted) | the 16 logical buttons | ZL + D-pad Down |
| Save State Load | `save_state_load` | multi choice | same | ZL + D-pad Up |
| Input Record | `input_record` | multi choice | same | ZR + D-pad Down |
| Input Playback | `input_playback` | multi choice | same | ZR + D-pad Up |
| Cycle CPU Character (fork, carry over) | `cpu_cycle` | multi choice | same | ZL + D-pad Left |
| Cycle Player Character (fork, carry over) | `player_cycle` | multi choice | same | ZL + D-pad Right |

**Menu shortcut 1 / 2 (draft 9, N7, decided 2026-11-01; menu question 9, option B).** Two more rows on the same page (Display & Controls › Controls), `editor` rows that open this spec's chord editor (the entry rule of §2.2: hold one button, press the others). They edit the INI shortcuts of §2.3 / §7: slot 1 (`shortcut_1`, default Minus) and slot 2 (`shortcut_2`, default disabled); an empty entry or `disabled` disables a slot. **A change applies at once** (the new chord is live as soon as the editor confirms; no restart, unlike the fork) and **the menu writes `button_config.ini`, the file of record** (§7). The permanent `B + D-pad Up` is shown as a fixed `info` row and cannot be edited or disabled. The two rows are not chord settings in the menu file: their state lives only in the INI.

Help text for every chord row: "Hold any one button and press the others to trigger". Upstream's Menu Open Start Press help: "Hold start or press minus to open the mod menu. To open the original menu, press start. The default menu open option is always available as Hold DPad Up + Press B."

Logical buttons and their menu labels: A, B, X, Y, L ("Pro L"), R ("Pro R; GCC Z"), ZL ("Pro ZL; GCC L"), ZR ("Pro ZR; GCC R"), DPad Up, DPad Down, DPad Left, DPad Right, Plus, Minus, Left Stick Press, Right Stick Press. The menu shows them with the game font's controller glyphs (§3.4).

### 2.2 Chord rule ("hold one, press the others")

A configured set S of logical buttons passes on a poll when there exists a button H in S such that H is currently down and every other button of S was pressed on this poll (its "just pressed" bit is set). Consequences: for a two-button chord, either button may be the held one; pressing both on the same poll passes (each is both down and just pressed); a one-button set passes on **every poll while it is held** (level-triggered — open question 1); an empty set never passes; extra unrelated buttons do not disqualify.

### 2.3 Menu-open chord

- Upstream: **B + D-pad Up** always (same rule as above). With Menu Open Start Press On: pressing **Minus** opens the mod menu (unless the vanilla pause menu is open); holding **Plus** for 10 or more polls opens the mod menu; holding Plus for 1–9 polls and releasing sends the game a synthesised Plus press (vanilla pause), provided the mod menu is closed and no menu-close grace is running; while Plus is held its bits are removed from the controller so the game does not pause early; a 2-poll release debounce.
- Fork (carry over, interface in §9; draft 9: the menu edits the INI, N7): the permanent **B + D-pad Up**, plus two INI shortcuts (default `shortcut_1=Minus`, `shortcut_2=disabled`) with edge semantics (all buttons of the chord down and at least one just pressed); Plus goes straight to the game's pause (no long-press opener, no synthesised press); an explicitly configured Plus shortcut consumes its pause press. The upstream long-Plus opener was removed because it delayed and synthesised the pause press (root-cause decision recorded in `docs/menu-shortcuts.md`); fold in the fork behaviour (§6.1).

### 2.4 Where chords are live

Training mode, and in lobby practice while the practice session is active (fork). All chords are suppressed while the vanilla pause menu is open or the mod menu is open (fork: also while the named-macro panel or the replay editor owns input). Save/Load additionally require Enable Save States On (save-state spec); Record requires Recording Trigger to contain Button Combination (SPEC-input-01).

## 3. Inputs

- The four (fork: six) chord settings and, upstream, Menu Open Start Press; fork: the parsed INI shortcuts.
- P1's game `Controller` (through the pointer at +0x10 of the fourth argument of the mapping function, SPEC-input-01 §4): `current_buttons` (+0x8), `previous_buttons` (+0xC), `just_down` (+0x88), `just_release` (+0x8C), `style` (+0x9C).
- Vanilla-menu-open and mod-menu-open flags; menu-close grace counter (a "real-time" frame counter started when the mod menu closes; 15 UI frames).

### 3.1 The game `Controller` (0xD0 bytes; offsets from its start)

| Offset | Field | Type |
| --- | --- | --- |
| 0x0 | vtable | u64 |
| 0x8 | current_buttons | 32-bit button set (§3.2) |
| 0xC | previous_buttons | 32-bit button set |
| 0x10 / 0x14 | left_stick_x / y | f32 (±1.0) |
| 0x18 | left_trigger | f32 |
| 0x20 / 0x24 | right_stick_x / y | f32 |
| 0x28 | right_trigger | f32 |
| 0x30 | gyro | 4 × f32 |
| 0x40, 0x58, 0x70 | button / lstick / rstick autorepeat info | 0x18 bytes each |
| 0x88 | just_down | 32-bit button set |
| 0x8C | just_release | 32-bit button set |
| 0x90 / 0x94 / 0x98 | autorepeat keys / threshold / initial-press threshold | u32 |
| 0x9C | style | u32 (§3.3) |
| 0xA0 | controller_id | u32 |
| 0xA4–0xB0 | four controller colours | u32 each |
| 0xB4 | led_pattern | u8 |
| 0xB5–0xB7 | autorepeat initial-press flags | bool each |
| 0xB8 | is_valid_controller | bool |
| 0xBB–0xC0 | is_connected, left/right connected, is_wired, left/right wired | bool each |
| 0xC4 | npad_number | u32 |

### 3.2 Button set bit order (bit 0 = least significant)

0 dpad_up, 1 dpad_right, 2 dpad_down, 3 dpad_left, 4 x, 5 a, 6 b, 7 y, 8 l, 9 r, 10 zl, 11 zr, 12 left_sl, 13 left_sr, 14 right_sl, 15 right_sr, 16 stick_l, 17 stick_r, 18 plus, 19 minus, 20 l_up, 21 l_right, 22 l_down, 23 l_left, 24 r_up, 25 r_right, 26 r_down, 27 r_left, 28 real_digital_l, 29 real_digital_r, 30–31 unused.

### 3.3 Controller styles and the logical-button mapping

Styles: 1 Handheld, 2 Dual Joy-Con, 3 Left Joy-Con, 4 Right Joy-Con, 5 Pro Controller, 6 debug pad, 7 GameCube.

| Logical | GameCube (style 7) | Every other style |
| --- | --- | --- |
| A, B, X, Y | the bit | the bit |
| L | never | l |
| R | zr (the GameCube Z arrives as zr) | r |
| ZL | l or real_digital_l | zl or left_sl or right_sl |
| ZR | r or real_digital_r | zr or left_sr or right_sr |
| D-pad ×4, Plus, Minus | the bit | the bit |
| Left / Right Stick Press | stick_l / stick_r | same |

The same mapping is applied to `current_buttons` (held) and `just_down` (pressed). Minus and L have no GameCube mapping, so chords containing them cannot pass on a GameCube controller (the permanent B + D-pad Up menu chord remains available).

### 3.4 Font glyphs for the menu's button display (private-use code points of the game font)

| Logical | Pro / Joy-Con | GameCube |
| --- | --- | --- |
| A | E0E0 | E0E0 |
| B | E0E0 (upstream TODO: no distinct glyph found) | E0E0 |
| X | E0E2 | E206 |
| Y | E0E3 | E207 |
| L | E0E4 | none |
| R | E0E5 | E205 |
| ZL | E0E6 | E204 |
| ZR | E0E7 | E208 |
| D-pad Up / Down / Left / Right | E079 / E07A / E07B / E07C | E209 / E20A / E20B / E20C |
| Plus | E0EF | E20D |
| Minus | E0F0 | none |
| Left / Right Stick Press | E104 / E105 | none |

## 4. Outputs and native state touched

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| P1 `Controller` Plus bits in `current_buttons`, `previous_buttons`, `just_down`, `just_release` | inside the final-input-mapping hook at `main+0x1750dd0` (SPEC-input-01 §4), chord stage; upstream: cleared while Plus is held under the long-press opener, set (current + just_down) when a short Plus is released to synthesise the vanilla pause; fork: cleared only while a modal panel owns input or when an explicit Plus menu shortcut was accepted this poll | P1 poll | hook at the documented site; `player_idx == 0`; Training/practice |
| Pending-request flags (plugin state), one per chord | set when the chord passes and the flag is clear; cleared when a consumer asks | P1 poll / consumer | — |
| Mod-menu-open flag (plugin state) | set by the menu consumer when the OpenMenu request is consumed in the per-fighter post-frame | post-frame | — |

Reads only: the `Controller` fields of §3.1; the vanilla-menu flag, derived by the HUD tick from the Training layout's help footer pane `L_staying_help` (pos_y ≠ −80 means the vanilla pause menu is opening or open; in the waiting room, fork: the `option_menu_setting` pane's pos_x ≠ −540 or a native popup busy).

## 5. Timing and frame rules

1. **Detection** happens once per P1 poll, before input delay (so chords are never delayed) and after the mod-menu input stage (which empties the mapped output while the menu is open but leaves the raw controller intact). For every chord whose request flag is clear, evaluate §2.2 against the raw controller and set the flag when it passes. Upstream also ORs the long-Plus/Minus opener result into the OpenMenu request.
2. **Consumption** is pull-based: a consumer calls "did chord X pass?" which returns the flag and clears it. Consumers and their cadence: OpenMenu — the per-fighter post-frame dispatch (upstream: for every fighter body; the first body wins); SaveState / LoadState — the save-state per-fighter step for the non-CPU, non-Nana body; InputRecord / InputPlayback — the recorder's post-frame step on the CPU body (SPEC-input-01 §5.2). A request raised while its consumer is not running stays pending until consumed or cleared (fork: session clear resets all flags; a "has pending request" query lets the loader know whether P1 is mid-request).
3. **Suppression:** detection returns false while the vanilla menu or the mod menu is open (fork: or the named panel / replay loading). Requests already pending are not cleared by suppression (upstream); fork clears them on session clear and when the waiting-room native UI becomes busy.
4. **Mod-menu close grace:** after the mod menu closes, the menu stage empties P1's mapped output for 15 UI frames; chords detected in that window still raise requests (upstream). Open question 2.
5. **Edge rule for the fork's cycle chords and INI shortcuts:** at least one button of the chord must be just pressed this poll; holding does not repeat. Not applied to the four legacy chords (parity), see open question 1.

## 6. Edge cases and interactions

1. **Plus handling (fork decision, fold in).** Upstream's long-press opener held the pause press back for up to 10 polls and re-synthesised it on release, which delayed and sometimes duplicated the vanilla pause. The fork removed the opener: Plus reaches the game untouched unless a modal panel owns input or an INI shortcut explicitly includes Plus. `menu_open_start_press` stays in the saved menu file for old configurations (default Off) and has no effect.
2. **GameCube controller:** Z arrives as the zr bit and is the logical R; L/R triggers (analogue or digital click) are logical ZL/ZR; logical L and Minus never pass. Default chords therefore work on a GameCube pad as Z + D-pad (ZR rows) and L + D-pad (ZL rows).
3. **Both record and playback chords in one poll:** both requests are raised; the recorder consumes playback first and drops the record request (SPEC-input-01 §2.2).
4. **Chord while a save-state load is in progress:** record/playback are consumed and ignored (SPEC-input-01 §6.4); save/load are handled by the save-state spec.
5. **Ice Climbers as P1:** Nana's body never consumes chords (save-state and recorder consumers skip Nana).
6. **CPU-as-P1 / native CPU Behavior = Control (fork):** the operator controller of the CPU entry (`player_idx == 1`) may trigger the CPU-cycle chord through a separate evaluation on that controller; it never enters P1's request set. Legacy chords are P1 only.
7. **Lobby practice:** chords are live while the practice session is active and the native waiting UI is not busy; the lobby L+R+A quick reset is a separate raw check (l or real_digital_l, r or real_digital_r, and a) that empties P1's output while held.
8. **Hot swap:** session clear resets all pending requests.
9. **Controller disconnect:** the mod menu closes itself when no controller is connected (menu spec); chord detection on a disconnected controller sees no bits.
10. **Poll multiplicity:** with the flag model, repeated polls in one frame cannot double-fire a chord; the one-button level trigger refires every poll regardless.

## 7. Persistence

- The chord settings are part of the menu file (menu spec).
- Fork INI (carry over): `sd:/ultimate/TrainingModpack/button_config.ini` in the fork; in the clean room `sd:/ultimate/TrainingRooms/button_config.ini` (decided, SPEC-core-08; imported from the fork once), created at startup with the default text (a comment line, `[menu]`, `shortcut_1=Minus`, `shortcut_2=disabled`) only when absent. **Draft 9 (N7): it is the file of record and the menu rewrites it** when the player confirms a shortcut change in *Menu shortcut 1 / 2* (the fork never rewrote it): the two values are written as the same `+`-joined names the parser reads (`disabled` for an empty slot), through the plugin's writer (a temporary name, then a rename, as the other small files), and the in-memory shortcuts are replaced at the same moment so the change applies at once; the rewrite happens at no other time (not at boot). Read at startup; the whole file ≤ 8 KiB; missing, unreadable, oversized or invalid-UTF-8 → defaults; `#` and `;` start comments; unknown sections/keys ignored; a malformed value (unknown or repeated button, empty, empty `+` component) or a duplicate key falls back to that slot's default independently (an invalid slot 1 restores Minus; an invalid slot 2 stays disabled); `disabled` or `none` disables a slot; button names are case-insensitive and accept the field names (`DPAD_UP`), the menu labels (`DPad Up`, `Pro ZL`, `GCC L`) and `Left Stick Press`/`Right Stick Press`.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | request flags (a small map) + INI shortcuts (2 × 16 flags); code not isolated — see `docs/resource-costs.md` (W5) | ≤ 64 B data | `tools/build/nro_size.py` per-feature build |
| per-frame work | per P1 poll: up to 6 chords × up to 16 buttons × 2 mappings | same | diag counters |
| heap allocations in hooks | upstream allocated a vector of the chord's buttons per chord per poll (4+ allocations per poll); fork iterates a fixed 16-button table (0) | 0 | counting allocator, install line |

## 9. Interface of the carried-over parts

**INI shortcuts (`training_mod_consts::menu_shortcuts`, host-tested):** `MenuShortcuts::parse(&str) -> MenuShortcuts` (rules of §7); draft 9 adds `MenuShortcuts::to_ini_text(&self, original: &str) -> String` (the writer of N7: the two shortcut lines replaced, see open question 6 for what else is kept) and a setter that replaces one slot and re-validates; `passes(held: Fn(Button) -> bool, pressed: Fn(Button) -> bool) -> bool` evaluating the permanent B + D-pad Up chord and the two slots with the edge rule (every button of a chord held, at least one pressed); `description() -> String` ("Minus or B + DPad Up" style, shown in the menu); constants: the permanent chord, the default INI text, the default shortcuts, and the 16-button table in bit order. The runtime passes the two mapping closures built from §3.3 on `current_buttons` and `just_down`.

**Chord core API used by the carried-over layers:** `combo_passes(chord) -> bool` (consume); `has_pending_request() -> bool` (observe without consuming; the character loader uses it); `clear_session_state()`; `consume_cpu_cycle_request()`, `consume_player_cycle_request()`; `operator_cpu_cycle_passes(controller) -> bool` (evaluate the CPU-cycle chord on another controller without touching P1's flags); `button_mapping(logical, style, bits) -> bool` (§3.3; the mod menu's navigation uses it on `just_down`); `name_to_font_glyph(logical, style) -> Option<u16>` (§3.4); `suppress_pause(controller, owns: bool)` (clears the four Plus bits when `owns`).

**Blocking owners the core must consult:** vanilla menu, mod menu, named-macro panel visible, replay capture loading.

## 10. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Menu items and defaults (§2.1) | `chords::menu_defaults` | IN04-E01 | `evidence/input/IN04-E01.md` |
| Hold-one-press-others rule, either button held, simultaneous press, extra buttons ignored, empty set never (§2.2) | `chords::hold_press_rule` | IN04-E02 | |
| One-button set is level-triggered (parity) or edge (decision, open question 1) | `chords::single_button_semantics` | IN04-E02 | |
| Logical → physical mapping per style, GameCube table (§3.3) | `chords::button_mapping_by_style` | IN04-E03 (GameCube adapter) | |
| Glyph table (§3.4) | `chords::glyph_table` | IN04-E01 screenshot | |
| Requests latch until consumed; cleared on session clear | `chords::request_latch_and_clear` | IN04-E04 | |
| Suppressed under vanilla and mod menus and modal panels | `chords::suppressed_under_menus` | IN04-E05 | |
| Plus passes straight to pause; explicit Plus shortcut consumes it; modal owners clear Plus bits (§6.1) | `chords::plus_passthrough_and_suppression` | IN04-E06 | |
| Permanent B + D-pad Up chord; INI defaults, parsing, fallbacks (§7) | `menu_shortcuts::*` (fork tests, carried over) | IN04-E07 (missing file created; edited X shortcut after restart) | |
| Chords not delayed by Input Delay | SPEC-input-03 `delay::ordering_with_chords_display_recorder` | IN03-E05 | |
| Zero heap allocations in the hook | `chords::detection_allocates_nothing` | — | install line |
| **N7** *Menu shortcut 1 / 2* rows open the chord editor; the permanent chord row is read-only and cannot be edited or disabled | `chords::shortcut_rows_open_editor`, `chords::permanent_chord_fixed` | IN04-E08 | `evidence/input/IN04-E08.md` |
| **N7** A confirmed shortcut change applies at once (the new chord opens the menu with no restart; the old chord no longer does) | `chords::shortcut_change_applies_immediately` | IN04-E08 | |
| **N7** The menu writes `button_config.ini` on a confirmed change only: the written text parses back to the same slots; `disabled` for an empty slot; temporary file then rename; no rewrite at boot or on any other event | `menu_shortcuts::to_ini_text_roundtrip`, `chords::ini_written_only_on_confirmed_change` | IN04-E08 (read the file after a change; restart; the shortcut survives) | |
| **N7** After a menu edit the file is still read by the carried-over parser: invalid hand edits fall back per slot as before | `menu_shortcuts::*` (fork tests, unchanged) | IN04-E07 | |

## 11. Open questions

1. **One-button chords are level-triggered.** A single configured button fires every poll while held (for playback: restarts the playback every frame; for save: saves every frame). Recommendation: require at least one just-pressed button in every chord (the fork already does this for its cycle chords and INI shortcuts); accept the parity change. **Decided 2026-11-01 (B8 batch): accepted.**
2. **Requests raised during the 15-frame close grace** (and, upstream, while suppressed earlier) are consumed later. Recommendation: do not raise requests while any input owner is active or the grace is running.
3. **GameCube: logical L and Minus unmapped.** Recommendation: keep (hardware limitation); the menu should grey those rows when the detected style is GameCube.
4. **Menu Open Start Press.** The fork removed the behaviour but kept the saved field. Recommendation for the menu redesign: drop the row; keep reading the field for old files only. **Decided earlier (2026-11-01; `docs/DECISIONS.md` *Already answered*): dropped; old keys read and ignored.**
5. **Default chord collisions.** ZL/ZR + D-pad defaults collide with taunts on controllers where D-pad is taunt; upstream accepted this. Recommendation: unchanged; mention in the README.
6. **Editing the shortcuts from the menu (N7): details the operator did not state.** The decision is that the shortcuts are editable in the menu, the INI stays the file of record, a change applies at once and the permanent chord stays fixed. Not decided, with recommendations: (a) what the rewrite keeps from the existing file: recommendation, keep comments and unknown lines and replace only the two `shortcut_` lines (otherwise a hand-edited file loses its notes); (b) whether the editor refuses a chord equal to the permanent `B + D-pad Up`, to the other shortcut slot, or to one of the six chord rows: recommendation, refuse the permanent chord and a duplicate slot with a short notice, and allow overlap with the other chords; (c) what the editor shows when the file cannot be written (read-only card or full): recommendation, keep the change in memory for the session, show one notice, log it.
