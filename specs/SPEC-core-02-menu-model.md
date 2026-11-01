# Menu model (tabs, submenus, toggles, sliders, navigation, reset) — behavioural specification

- **Spec id:** `SPEC-core-02`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), spec-writer role, 2026-11-01
- **Upstream reference read (spec writer only):** `training_mod_tui/src/lib.rs`, `training_mod_tui/src/containers/{mod,app,tab,submenu,toggle}.rs`, `training_mod_tui/src/structures/{stateful_list,stateful_slider,stateful_table}.rs`, `training_mod_consts/src/lib.rs` (the menu table and defaults), `training_mod_consts/src/options.rs` (option sets), `src/common/menu.rs` (input dispatch) at commit `97cbe9b9` (unchanged in `077a8e43`); fork changes at stage M5i `training_mod_tui/src/lib.rs` lines `3-10, 16-101`, `containers/app.rs` `5-6, 16, 52-67, 70-88, 98-109, 112-118, 120-364, …`, `containers/submenu.rs` `22, 42, 53, 64-67, 75, 88, 101, 116, 135-169, 176-181, 210`, `containers/tab.rs` `16-19`, `structures/*` (Default impls, `is_empty`), `training_mod_consts/src/lib.rs` `7-35, 40, 58-59, 120-122, 132-293, 299, 301, 303-307, 323, 384, 409-443, 445-457, …` (fork tabs and hidden rows), `options.rs` `42-45, 62-65, 74-76, 84-87, 227-228, 230-234, 313-353, 429-430, 432-436, 446, 1044-1052`, from `docs/provenance/provenance-files.md`.
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

The menu model is the pure, host-testable state machine behind the in-game settings menu: a ring of tabs, each a grid of submenus, each a grid of toggles or one two-handle slider, with a cursor, a page state (submenu list / toggle page / slider page / confirmation / closed), a serialized copy of the current settings and of the player's saved defaults, and the rules for every button. It knows nothing about panes, controllers or files; SPEC-core-03 feeds it button events and draws it, SPEC-core-04 stores what it serializes.

## 2. User-visible behaviour

### 2.1 Structure and pages

- The menu is a ring of **tabs**. Each tab shows up to 32 **submenus** on an 8-row by 4-column grid, filled row-major; empty cells are skipped by the cursor. Each submenu opens either a **toggle page** (the same 8×4 grid of options) or a **slider page** (two handles, Min and Max).
- Pages: `SUBMENU` (the tab's grid), `TOGGLE`, `SLIDER`, `CONFIRMATION` (a Yes/No question), `CLOSE` (the menu is about to close and commit). The menu always opens on `SUBMENU` with the previously selected tab and cursor positions (the model keeps its cursors between openings; the fork additionally re-opens large single-choice pickers on the chosen option, §6).
- A tab is always selected (index 0 after construction).

### 2.2 Submenu kinds

| Kind | Toggle `max` | Meaning of a toggle value |
| --- | --- | --- |
| Single choice (`ToggleSingle`) | 1 | exactly one option has value 1, the rest 0 (pressing A on an option clears all others first) |
| Multiple choice, unweighted (`ToggleMultiple`, no weights) | 1 | each option independently on (1) or off (0) |
| Multiple choice, weighted (`ToggleMultiple`, weights) | 5 (`TOGGLE_MAX`) | each option 0..5; a value is a random-selection weight for the consuming feature (SPEC-cpu-*), 0 = never |
| Slider | — | two integers `lower ≤ upper` within `[min, max]` |

A toggle press increments the value and wraps from `max` to 0. A single-choice submenu therefore toggles an option *off* when it is pressed again (value 1 → 0 with all others already 0), leaving the submenu with no selection; the consuming features treat "no selection" as the empty option (for example `Shield Tilt` ships with no selection). Weighted options show their weight as a number badge (SPEC-core-03).

### 2.3 Buttons (logical names; controller mapping in SPEC-core-03 §3)

| Button | `SUBMENU` | `TOGGLE` | `SLIDER` | `CONFIRMATION` |
| --- | --- | --- | --- | --- |
| A | open the selected submenu (toggle or slider page) | increment the selected toggle (single choice: clear others first) | select / deselect the hovered handle | if hovering **Yes**: apply the reset; then return to the page the question came from |
| B | close the menu (`CLOSE`) | back to `SUBMENU` | if a handle is selected: deselect it; else back to `SUBMENU` | return to the originating page without resetting |
| X | save the current settings as the player's **defaults** (all pages except `CONFIRMATION`/`CLOSE`; the saved snapshot is written with the menu on close) | same | same | — |
| Y | — | **clear** the selected toggle to 0, only in a weighted multiple-choice submenu (`max > 1`) | — | — |
| ZL / ZR | previous / next tab (ring) | — | — | — |
| L | — | — | — | — |
| R | ask "reset **all** settings to the defaults?" | ask "reset the **current** setting to the defaults?" | same as `TOGGLE` | — |
| Up / Down | move the submenu cursor a row (wrap; skip empty cells) | move the toggle cursor a row | selected handle: value ± 10 (fast step), clamped | — |
| Left / Right | move the submenu cursor a column (wrap; skip empty cells; moving left onto an empty cell slides further left until a filled cell) | move the toggle cursor a column | selected handle: value ∓ / ± 1 (slow step); no handle selected: switch hover between Min and Max | switch hover between No and Yes |
| Start | upstream defined "close" for Start but never dispatched it from the controller (see SPEC-core-03 §2.3) | | | |

Grid movement rules: `next_row`/`prev_row` wrap within the table's row count; `next_col`/`prev_col` wrap within the column count; after a move the cursor must land on a filled cell: moving down/up or right onto an empty cell repeats the move in the same direction; moving left onto an empty cell slides left ("carriage return") until a filled cell. A row with no filled cell is a model error (panic upstream) and cannot occur with ≤ 32 items filled row-major.

Hold repeat: a direction held on either analog stick repeats every 21 frames (the hold counter counts frames held, fires when it exceeds 20, and every accepted input resets all hold counters). The d-pad does not repeat. This rule is implemented in the input adapter (SPEC-core-03 §3) but belongs to the model's contract for tests.

### 2.4 Slider rules

- Fields: `state` ∈ {LowerHover, UpperHover, LowerSelected, UpperSelected, None}, `lower`, `upper`, `min`, `max`, `incr_slow`, `incr_fast`. Upstream constants: `min 0`, `max 150`, `slow 1`, `fast 10`; the only sliders are the two damage ranges, default `[0, 150]`.
- Increments saturate and clamp: raising `lower` never passes `upper`; raising `upper` never passes `max`; lowering `lower` never passes `min`; lowering `upper` never passes `lower`.
- A opens the slider page with `LowerHover`; A again selects the hovered handle (`*Selected`), A once more deselects; B deselects or leaves the page.

### 2.5 Reset and defaults

- **Defaults** are a second full settings snapshot (`defaults_menu`), initialised to the built-in factory values and replaced whenever the player presses X ("Save Defaults").
- **Reset all** (R on the submenu list, confirm Yes): every submenu takes its value array from the defaults snapshot.
- **Reset current** (R on a toggle or slider page, confirm Yes): only the open submenu takes its array from the defaults snapshot.
- Confirmation hover starts on **No**; Left/Right switch; A confirms the hovered answer; B cancels. The confirmation page remembers the page it came from and returns to it.
- A **factory reset** is deleting the settings file (SPEC-core-04).

### 2.6 Help text

The footer shows the selected submenu's help text on the submenu list and the question text on the confirmation page ("Are you sure you want to reset the current setting to the defaults?" / "Are you sure you want to reset ALL settings to the defaults?"). On the toggle and slider pages upstream leaves the footer unchanged; the fork adds a paging footer for long lists (§6). The clean room writes its own help sentences; the upstream meanings are listed in §2.7 for the writer.

### 2.7 Tabs, submenus, options and defaults (upstream `97cbe9b9`)

Ids are the stable serialization keys (SPEC-core-04). "W" = weighted multiple choice (max 5), "M" = unweighted multiple choice (max 1), "S" = single choice, "Sl" = slider. Option lists are in declaration order (the array order on disk). Defaults are the factory `BASE_MENU`.

**Option sets** (shared by several rows):

| Set | Options in order |
| --- | --- |
| OnOff | On, Off |
| BoolFlag | True, False |
| Direction (11) | Out, Up Out, Up, Up In, In, Down In, Down, Down Out, Neutral, Left, Right |
| Action (30) | Air Dodge, Jump, Shield, Spot Dodge, Roll Forwards, Roll Backwards, Neutral Air, Forward Air, Back Air, Up Air, Down Air, Neutral Special, Side Special, Up Special, Down Special, Forward Smash, Up Smash, Down Smash, Jab, Forward Tilt, Up Tilt, Down Tilt, Grab, Dash, Dash Attack, Playback Slot 1, Playback Slot 2, Playback Slot 3, Playback Slot 4, Playback Slot 5 |
| Delay (31) | 0, 1, 2, …, 30 (frames) |
| MedDelay (31) | 0, 5, 10, …, 150 (frames, step 5) |
| LongDelay (31) | 0, 10, 20, …, 300 (frames, step 10) |
| MashTrigger (17) | Hitstun, Shieldstun, Parry, Tumble, Landing, Ledge Trump, Footstool, Clatter, Ledge Option, Tech Option, Mistech Option, Grounded, Airborne, Distance: Close, Distance: Mid, Distance: Far, Always |
| AttackAngle | Neutral, Up, Down |
| ThrowOption | None, Forward Throw, Backward Throw, Up Throw, Down Throw |
| LedgeOption (10) | Neutral Getup, Roll, Jump, Getup Attack, Wait, Playback Slot 1–5 |
| TechFlags | No Tech, Roll Forwards, Roll Backwards, Tech In Place |
| MissTechFlags | Neutral Getup, Getup Attack, Roll Forwards, Roll Backwards |
| Shield | None, Infinite, Hold, Constant |
| SdiFrequency / ClatterFrequency | None, Normal, Medium, High |
| SaveStateMirroring | None, Alternate, Random |
| SaveDamage | Default, Save State, Random Value |
| DamagePercent (slider) | Lower, Upper |
| SaveStateSlot / RecordSlot / PlaybackSlot | Slot 1, Slot 2, Slot 3, Slot 4, Slot 5 |
| HitstunPlayback | As Hitstun Ends, As Hitstop Ends, As Hitstop Begins |
| RecordTrigger | Button Combination, Save State Load |
| RecordingDuration (19) | 60, 90, 120, …, 600 (frames, step 30) |
| CharacterItem (17) | None, Player 1st Var. … Player 8th Var., CPU 1st Var. … CPU 8th Var. |
| BuffOption (18) | Acceleratle, Oomph, Psyche Up, Bounce, Arsene, Deep Breathing, Limit, KO Punch, 1-Winged Angel, Jump, Speed, Shield, Buster, Smash, Power Dragon, Mini Waft, Half Waft, Full Waft |
| ButtonConfig (16) | A, B, X, Y, Pro L, Pro R; GCC Z, Pro ZL; GCC L, Pro ZR; GCC R, DPad Up, DPad Down, DPad Left, DPad Right, Plus, Minus, Left Stick Press, Right Stick Press |
| InputDisplay | None, Smash Inputs, Raw Inputs, Status Only |
| UpdatePolicy (dropped) | Stable, Beta, Disabled |

**Tab `mash` — "Mash Settings"**

| # | Title | id | Kind / set | Default | Meaning (for the help text) |
| --- | --- | --- | --- | --- | --- |
| 1 | Mash Toggles | `mash_state` | W Action | none | action performed as soon as possible |
| 2 | Followup Toggles | `follow_up` | W Action | none | action after a mash option |
| 3 | Mash Triggers | `mash_triggers` | M MashTrigger | Hitstun, Shieldstun, Tumble, Clatter | what causes the CPU to mash |
| 4 | Attack Angle | `attack_angle` | W AttackAngle | none | angle for angled attacks |
| 5 | Throw Options | `throw_state` | W ThrowOption | None = 1 | throw after a landed grab |
| 6 | Throw Delay | `throw_delay` | W MedDelay | none | frames to delay the throw |
| 7 | Pummel Delay | `pummel_delay` | W MedDelay | none | frames after a grab before pummelling |
| 8 | Falling Aerials | `falling_aerials` | W BoolFlag | False | aerials rising or falling |
| 9 | Full Hop | `full_hop` | W BoolFlag | True | full or short hop |
| 10 | Aerial Delay | `aerial_delay` | W Delay | none | delay before an aerial |
| 11 | Fast Fall | `fast_fall` | W BoolFlag | False | fast fall during a jump |
| 12 | Fast Fall Delay | `fast_fall_delay` | W Delay | none | frames before fast falling |
| 13 | OoS Offset | `oos_offset` | W Delay | none | shield hits before mashing out of shield |
| 14 | Reaction Time | `reaction_time` | W Delay | none | frames before mashing out of shield |

**Tab `override` — "Override Settings"** (13 rows, all W Action, default none): Ledge Neutral Getup `ledge_neutral_override`, Ledge Roll `ledge_roll_override`, Ledge Jump `ledge_jump_override`, Ledge Attack `ledge_attack_override`, Tech Action `tech_action_override`, Clatter `clatter_override`, Tumble `tumble_override`, Hitstun `hitstun_override`, Parry `parry_override`, Shieldstun `shieldstun_override`, Footstool `footstool_override`, Landing `landing_override`, Ledge Trump `trump_override`. Meaning: mash actions performed after that situation.

**Tab `defensive` — "Defensive Settings"**

| # | Title | id | Kind / set | Default |
| --- | --- | --- | --- | --- |
| 1 | Airdodge Direction | `air_dodge_dir` | W Direction | none |
| 2 | DI Direction | `di_state` | W Direction | none |
| 3 | SDI Direction | `sdi_state` | W Direction | none |
| 4 | SDI Strength | `sdi_strength` | S SdiFrequency | None |
| 5 | Clatter Strength | `clatter_strength` | S ClatterFrequency | None |
| 6 | Ledge Options | `ledge_state` | W LedgeOption | Neutral Getup, Roll, Jump, Getup Attack = 1 |
| 7 | Ledge Delay | `ledge_delay` | W LongDelay | none |
| 8 | Tech Options | `tech_state` | W TechFlags | all four = 1 |
| 9 | Mistech Options | `miss_tech_state` | W MissTechFlags | all four = 1 |
| 10 | Shield Toggles | `shield_state` | S Shield | None |
| 11 | Shield Tilt | `shield_tilt` | S Direction | no selection |
| 12 | Crouch | `crouch` | S OnOff | Off |
| 13 | Dodge Staling | `stale_dodges` | S OnOff | On |
| 14 | Hide Tech Animations | `tech_hide` | S OnOff | Off |

**Tab `input` — "Input Recording"**

| # | Title | id | Kind / set | Default |
| --- | --- | --- | --- | --- |
| 1 | Recording Slot | `recording_slot` | S RecordSlot | Slot 1 |
| 2 | Recording Trigger | `record_trigger` | S RecordTrigger | Button Combination |
| 3 | Recording Duration | `recording_duration` | S RecordingDuration | 150 |
| 4 | Recording Crop | `recording_crop` | S OnOff | On |
| 5 | Playback Button Slots | `playback_button_slots` | W PlaybackSlot | Slot 1 |
| 6 | Playback Hitstun Timing | `hitstun_playback` | S HitstunPlayback | As Hitstun Ends |
| 7 | Save State Playback | `save_state_playback` | W PlaybackSlot | none |
| 8 | Playback Mash Interrupt | `playback_mash` | S OnOff | On |
| 9 | Playback Loop | `playback_loop` | S OnOff | Off |

**Tab `button` — "Button Config"**

| # | Title | id | Kind / set | Default |
| --- | --- | --- | --- | --- |
| 1 | Menu Open Start Press | `menu_open_start_press` | S OnOff | On (fork: removed from the menu, kept as an ignored legacy key; SPEC-core-03 §6) |
| 2 | Save State Save | `save_state_save` | M ButtonConfig | Pro ZL; GCC L + DPad Down |
| 3 | Save State Load | `save_state_load` | M ButtonConfig | Pro ZL; GCC L + DPad Up |
| 4 | Input Record | `input_record` | M ButtonConfig | Pro ZR; GCC R + DPad Down |
| 5 | Input Playback | `input_playback` | M ButtonConfig | Pro ZR; GCC R + DPad Up |

Chord semantics ("hold any one button and press the others") are SPEC-input-* (button chords).

**Tab `save_state` — "Save States"**

| # | Title | id | Kind / set | Default |
| --- | --- | --- | --- | --- |
| 1 | Mirroring | `save_state_mirroring` | S SaveStateMirroring | None |
| 2 | Auto Save States | `save_state_autoload` | S OnOff | Off |
| 3 | Save Dmg (CPU) | `save_damage_cpu` | S SaveDamage | Default |
| 4 | Dmg Range (CPU) | `save_damage_limits_cpu` | Sl DamagePercent | [0, 150] |
| 5 | Save Dmg (Player) | `save_damage_player` | S SaveDamage | Default |
| 6 | Dmg Range (Player) | `save_damage_limits_player` | Sl DamagePercent | [0, 150] |
| 7 | Enable Save States | `save_state_enable` | S OnOff | On |
| 8 | Save State Slot | `save_state_slot` | S SaveStateSlot | Slot 1 |
| 9 | Randomize Slots | `randomize_slots` | W SaveStateSlot | none |
| 10 | Character Item | `character_item` | S CharacterItem | None |
| 11 | Buff Options | `buff_state` | M BuffOption | none |

**Tab `misc` — "Misc Settings"**

| # | Title | id | Kind / set | Default |
| --- | --- | --- | --- | --- |
| 1 | Frame Advantage | `frame_advantage` | S OnOff | Off |
| 2 | Hitbox Visualization | `hitbox_vis` | S OnOff | Off |
| 3 | Input Display | `input_display` | S InputDisplay | Smash Inputs |
| 4 | Input Display Status | `input_display_status` | S OnOff | Off |
| 5 | Input Delay | `input_delay` | S Delay | 0 |
| 6 | Stage Hazards | `stage_hazards` | S OnOff | Off |
| 7 | HUD | `hud` | S OnOff | On |
| 8 | Auto-Update | `update_policy` | S UpdatePolicy | Stable — **dropped** (SPEC-core-01 §6) |
| 9 | L+R+A Reset | `lra_reset` | S OnOff | On |

Upstream limits: a tab or submenu may hold at most 32 cells (8×4); the largest upstream option sets have 31 entries (Delay, MedDelay, LongDelay) and 30 (Action). The fork's ordered rosters and stage lists exceed 32 and need paging (§6).

## 3. Inputs

- Button events from the input adapter (SPEC-core-03): A, B, X, Y, L, R, ZL, ZR, Up, Down, Left, Right, Start — already mapped from the physical controller to these logical names.
- The two JSON snapshots at open (current settings and defaults, SPEC-core-04), as a map `id → [u8]`.
- Nothing from the game.

## 4. Outputs and native state touched

None. The model never touches game memory. Its outputs are:

| Output | Consumer |
| --- | --- |
| the page, cursors, hover and selection states | the renderer (SPEC-core-03) |
| `serialized_settings`: the current settings as a JSON object `id → [u8]` (sliders `[lower, upper]`), refreshed after every A/B press and after every bulk update | the commit on close (SPEC-core-04) |
| `serialized_default_settings`: the defaults snapshot, replaced on X | the commit on close |
| the combined close payload `{"menu": <settings>, "defaults_menu": <defaults>}` | SPEC-core-04 |
| "should show Clear Toggle hint" = page is `TOGGLE` and the submenu is multiple-choice and the selected toggle's `max > 1` | the renderer |

## 5. Timing and frame rules

- The model advances only on button events delivered by the input adapter, once per input frame while the menu is open. Rendering reads it on the UI thread; the two run under one lock (SPEC-core-03 §5).
- Serialization happens on every A and B press (upstream serialized the whole menu to a JSON string on each press); that is acceptable for a menu but must not run per frame.
- Bulk updates (`update_all_from_json`, `update_one_from_json`) accept a map `id → [u8]` and write each submenu's values by index; **the fork fix to fold in:** clear every toggle of the submenu to 0 before applying the array, so an array shorter than the current option list (an older file, or a catalog that grew) leaves the appended options off instead of keeping stale values. Slider arrays must have exactly two elements.
- A missing id in the map leaves that submenu unchanged; an unknown id in the map is ignored.

## 6. Edge cases and interactions

- **Single choice with no selection**: allowed (see §2.2); consumers treat it as the empty set.
- **Weighted toggles with all zeros**: the consuming feature does nothing (empty set).
- **Reset while on a slider page**: "reset current" replaces `[lower, upper]` from the defaults and keeps the hover state.
- **X on the confirmation page** is ignored (upstream let X through on every page; the fork too — recommendation: ignore on `CONFIRMATION`/`CLOSE`; Q1).
- **Tab ring with one tab**: previous/next return the same tab.
- **Menu open/closed**: SPEC-core-03 owns opening (`page = SUBMENU`) and closing (`page == CLOSE` → commit); the model only sets `CLOSE`.
- **Lobby practice / CPU-as-P1 / hot swap**: not the model's concern; the fork hides or re-titles rows per scene in the renderer.
- **Fork model hooks (fork's own code, carry over; the clean-room model must offer these extension points):**
  1. `SubMenuType::Ordered` — a toggle grid whose values are ranks: cell 0 is "All" (value 1 means "all, in native order", every other cell 0); A on a ranked cell removes it (keeping at least one ranked cell) and renumbers higher ranks down; A on an unranked cell gives it rank `count+1` and clears "All".
  2. **Paging**: a toggle table may have more than 8 rows; the renderer shows the 8-row page containing the selected row and the footer `"<title> - page i/n; Up/Down moves between pages.\n<first help line>"`; Up/Down cross page boundaries through the ordinary row wrap. Large single-choice pickers reopen with the cursor on the selected option.
  3. `hidden_submenus`: rows that are serialized, reset and bulk-updated like visible rows but never drawn or navigated (the fork stores the CPU cycle roster, CPU Kirby hats and two Solo Popo flags there because another screen edits them).
  4. `AppPage::ROSTER`: a full-screen portrait grid page with its own cursor (`RosterGrid`), a purpose (fighter cycles / Kirby hats / manual swap), a `roster_kinds` catalog mapping grid cells to fighter kinds, and role toggling; ZL/ZR/X/Y/R/A/B have page-specific meanings there (select all, switch role, cycle form, Solo Popo, remove, confirm, back).
  5. **Action rows**: submenus whose ids name an action (`named_macro_library`, `setup_library_save`, …) are intercepted by the runtime **before** `on_a` opens a page; they serialize as an ordinary OnOff array that nothing reads.
  6. **Pending intents**: the model records intents (stage change, manual character selection, Mii selection, Mii editor request) and the runtime takes them exactly once when `page == CLOSE` (`take_pending_*` return `None` on any other page). A "manual selection" locks all input for a short flash (`pending_manual` frames counted down by the draw callback), then forces `CLOSE`.
  7. `open_settings_for_player(json, original_kind)`: re-synchronise the whole model from the committed settings at every opening, cancel any mounted sub-catalogue, clear pending intents, and record the scene's original fighter kind (used to grey out cells).
  8. Per-row dynamic title/help (the stage row is "Lobby Stages" or "Practice Stages" depending on the scene), and X/L/R reused on that row for mode switching and paging.
  9. A sub-catalogue can be **mounted** into a tab at runtime (the Mii catalogue inserts three launcher rows after a given row if the tab has room: `len + 3 ≤ 32`) and unmounted on departure.
  10. `single_toggle_index(app, id)`: the index of the one selected option of a single-choice row, `None` for absent/non-single/empty/multiple.
- **Fork fixes to fold in (upstream defects):** the `update_from_vec` clearing rule (§5); `Tab::is_empty` and `Default` impls are cosmetic.

## 7. Persistence

None in the model. The close payload and its file are SPEC-core-04. The factory defaults are the constants in §2.7.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | not measured separately (part of the menu group) | see `docs/resource-costs.md` (W5) | `tools/build/nro_size.py` per-feature build |
| per-frame work | none while closed; one serialization (JSON string of ~90 arrays) per A/B press while open | same | diag counters |
| heap allocations in hooks | the model allocates (strings, vectors) on input events only | allocation only on input events, never per frame | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Grid navigation wraps and skips empty cells; carriage return on left | `core02_table_nav_wrap_skip`, `core02_table_carriage_return` | `core02-E1` cursor walk screenshots | `docs/EDEN-RESULTS.md` |
| Toggle increment wraps at max; single choice clears others; Y clears only weighted | `core02_toggle_single_exclusive`, `core02_toggle_wrap`, `core02_y_clears_weighted_only` | `core02-E2` | — |
| Slider clamps and steps (±1 slow, ±10 fast), hover/select states | `core02_slider_clamp_steps`, `core02_slider_states` | `core02-E3` | — |
| Page transitions for A/B on every page | `core02_page_machine` | `core02-E1` | — |
| X saves defaults; R asks; Yes resets all / current; No cancels | `core02_defaults_and_reset` | `core02-E4` reset flow | `docs/EDEN-RESULTS.md` |
| Serialization shape `id → [u8]`, sliders `[lower, upper]`; round trip through bulk update | `core02_serialize_roundtrip` | — | — |
| Bulk update clears before applying (short arrays leave appended options off) | `core02_update_from_shorter_array` | — | — |
| Factory table: every row of §2.7 with kind, option count and default | `core02_factory_table_matches_spec` (table-driven) | `core02-E5` every tab screenshot | `docs/EDEN-RESULTS.md` |
| Hold repeat every 21 frames on sticks, none on d-pad | `core02_hold_repeat_period` (adapter) | `core02-E6` | — |
| Clear-toggle hint rule | `core02_clear_hint_rule` | `core02-E2` | — |
| Fork hooks: Ordered ranks, paging, hidden rows, pending intents taken once at CLOSE | carried over with the fork's own tests | — | — |

## 10. Open questions

1. Should X (save defaults) be ignored on the confirmation page? Upstream accepts it anywhere. Recommendation: ignore on `CONFIRMATION` and `CLOSE`.
2. Upstream's help text for `Enable Save States` still says "Shield+Down Taunt / Shield+Up Taunt" although the default chords are ZL/ZR + D-pad; the clean room should derive the sentence from the configured chord or drop the chord from the sentence.
3. Should a single-choice row be allowed to end with no selection (upstream behaviour) or should A on the selected option be a no-op? Consumers already handle the empty set; recommendation: keep upstream behaviour for compatibility with saved files that contain all-zero single-choice arrays.
4. `Menu Open Start Press` and `Auto-Update` are removed rows in the fork (shortcuts moved to an INI; updater dropped). The clean room's factory table should omit both; their keys must still be accepted and ignored on load (SPEC-core-04).
5. The fork's `Characters` and `Character Extras` tabs and the `Macro Source`, `Fighter Prompt`, `Lobby Practice`, `Lobby Stages`, `Training Stage Rotation`, P1 CPU rows, cycle-preload rows, and the action rows are the fork's own; this spec only requires the hooks of §6 so they can be mounted unchanged. Confirm that no clean-room rewrite of those tabs is planned.
