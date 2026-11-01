# TrainingRooms menu — proposal for review

Proposal v2, 2026-11-01. Source of truth: [`MENU-DRAFT.md`](../MENU-DRAFT.md) (every item, option list and default, taken from the shipped C116.1 menu definition). Screens below are rendered mockups in the in-game overlay look (1280×720, `tools/render_menu_mockup.py`, files in [`menu-mockup/`](menu-mockup/)); they show the first group of each page selected. Nothing is built until a version is approved.

**What changes:** today's 9 tabs (103 visible rows) become **5 pages with 76 rows**. Every single-choice list becomes a dropdown (23), sets stay as checklists, ranges stay as sliders, and three repetitive tabs collapse into sub-screen editors (the 13 override rows, the 6 button chords, the 7 group switches). Setting ids do not change, so existing settings files load as they are.

**Already decided (operator, 2026-11-01):** MIT with the handle as holder; storage moves to `sd:/ultimate/TrainingRooms/` (one-time import of the old folder proposed); the CPU-pilot plugin owns "CPU plays P1" and has its own page, so this menu does not repeat it.

## Opening and navigating

| Action | Input |
| --- | --- |
| Open / close the menu | `B + D-pad Up` (always), or a shortcut from `button_config.ini` (shown read-only under Display & Controls) |
| Change page | `L` / `R` |
| Move | D-pad or stick; the left column is the page's groups, the right pane the selected group's items |
| Change a value | `A` on a toggle flips it; on a dropdown `◂` / `▸` cycle the value (question 2: or `A` opens the list); on a checklist `A` ticks the focused entry; `A` on an editor opens its sub-screen |
| Back | `B` (closes a sub-screen first) |
| Reset the page to defaults | `X` |
| Help | one line at the bottom for the focused item (question 10: or a two-line panel) |

## Control legend

| Shown as | Meaning | Today |
| --- | --- | --- |
| `[ON]` / `[OFF]` | toggle | toggle |
| `<  value  >` | dropdown: one choice; `◂`/`▸` cycle, or open the list | a "ToggleSingle" list where exactly one row is on |
| `n of N selected  >` | checklist: a set; random among the selected where the feature randomizes | a "ToggleMultiple" list |
| `|=====---|  0 - 150 %` | slider: a range | slider |
| `<  8  >` | number | single-choice list of numbers |
| `…  >` | editor / grid / list: opens a sub-screen | a submenu or grid |
| small `upstream` / `ours` tag | origin: a clean-roomed upstream feature, or our own code carried over | — |
| `advanced` on a group | collapsed by default on the console | — |

## Page 1 · CPU

Everything that changes what the CPU does, in the order a player thinks about a situation: reactions, defence, ledge and tech, shield, attacks and movement.

![CPU page](menu-mockup/01-cpu.png)

| Group | Items (control) |
| --- | --- |
| Reactions | Mash (checklist, 13 actions) · Followup (checklist, 13) · Mash Triggers (checklist, 17 situations) · **Situation overrides** (editor, replaces the 13-row Override tab) |
| Defence | Airdodge / DI / SDI Direction (checklists, 11 directions) · SDI Strength, Clatter Strength (dropdowns: None, Normal, Medium, High) · Dodge Staling (toggle) |
| Ledge & Tech | Ledge Options (checklist, 10 incl. Playback Slot 1–5) · Ledge Delay (checklist 0–300 by 10) · Tech / Mistech Options (checklists, 4 each) · Hide Tech Animations (toggle) |
| Shield | Shield (dropdown: None, Infinite, Hold, Constant) · Shield Tilt (dropdown, 11 directions) · OoS Offset (checklist 0–30 hits) · Reaction Time (checklist 0–30 frames) |
| Attacks & Movement | Attack Angle (checklist) · Throw (checklist: None, Forward, Back, Up, Down) · Throw / Pummel Delay (checklists 0–150 by 5) · Falling Aerials, Full Hop, Fast Fall (dropdowns: Yes, No, Random) · Aerial / Fast Fall Delay (checklists 0–30) · Crouch (toggle) |
| *(hidden unless the CPU-pilot plugin is absent)* | P1 CPU Control (toggle) · P1 CPU Level (dropdown 1–9) — the fallback; with the pilot present one info row says "P1 control: CPU pilot plugin (its own menu)" (question 11) |

### Sub-screen · Situation overrides

![Situation overrides](menu-mockup/06-situation-overrides.png)

Pick the situation in the dropdown (13: ledge getups, tech, clatter, tumble, hitstun, parry, shieldstun, footstool, landing, trump), tick the actions for it. The page row shows "n of 13 set".

## Page 2 · Saves

Positions and damage: the save-state rules and the named setups built on them.

![Saves page](menu-mockup/02-saves.png)

| Group | Items (control) |
| --- | --- |
| Save states | Save States (toggle) · Slot (dropdown 1–5) · Randomize Slots (checklist 1–5) · Mirroring (dropdown: None, Alternate, Random) · Auto-load on death (toggle) · Player / CPU damage on load (dropdowns: Default, Save State, Random) · Player / CPU damage range (sliders 0–150 %, greyed unless Random) · Buffs on load (checklist, 12) · Item on load (older saves) (dropdown, 17; question 6) |
| Setup library | Save setup to library… (editor: name entry) · Setup library (list; `A` binds + restores, `Y` binds) |

## Page 3 · Recording

Inputs the CPU plays back: numbered recordings, playback rules, and the named-macro and replay tools that feed the same slots.

![Recording page](menu-mockup/03-recording.png)

| Group | Items (control) |
| --- | --- |
| Recording | Record Slot (dropdown 1–5) · Trigger (dropdown: Button Combination, Save State Load) · Duration (dropdown 60–600 frames by 30) · Crop trailing neutral (toggle) |
| Playback | Button slots (checklist 1–5) · Hitstun timing (dropdown: As Hitstun Ends, As Hitstop Ends, As Hitstop Begins) · Save-state slots (checklist 1–5) · Mash interrupts playback (toggle) · Loop (toggle) |
| Named macros | Source (dropdown: Numbered Slots, Named Library) · Ask before switching fighter (toggle) · Named library (list) · Save slot to library… (editor) · Edit placement (editor) · Replay window (editor) · Replay capture (toggle, new in the menu; question 7) |

## Page 4 · Fighters & Stages

Who is on screen and where: swaps, cycles, hats, items, Miis, the stage rows, and (collapsed) the loading machinery that makes swaps fast.

![Fighters and Stages page](menu-mockup/04-fighters-stages.png)

| Group | Items (control) |
| --- | --- |
| Player 1 and CPU | Player Swapping (toggle) · Player Character, CPU Character (grid pickers) · Character Cycles (two-role ordered grid; `ZR` switches role; Solo Popo marks edited here) · Solo Popo (original Ice Climbers) (toggle) |
| Extras | Kirby Hats (two-role grid) · Give item (grid, 45 items) · Mii Brawler / Swordfighter / Gunner (lists: saved Miis, replay Miis, Add New…) |
| Stages | Lobby Practice (toggle) · Lobby Stages (ordered grid + All; `Y` cycles form) · Training Stage Rotation (toggle) · Stage Hazards (toggle) |
| Loading *(advanced)* | Cycle Preload (dropdown Off, 1–5) · Preload (dropdown: Fighters, Hats, Both) · Ready Fighters (number, default 8) · Ready Fighter Bodies (dropdown: Off, 2, 3) · Group Swap (toggle) · **Swap groups** (checklist of the 7 switches; entries grey with the reason when a dependency is off) · Preload Samples (toggle; question 8) |

### Sub-screen · Swap groups

![Swap groups](menu-mockup/08-swap-groups.png)

## Page 5 · Display & Controls

What is drawn and which buttons do what; nothing here changes what the CPU does.

![Display and Controls page](menu-mockup/05-display-controls.png)

| Group | Items (control) |
| --- | --- |
| On screen | HUD, Hitbox Visualization, Frame Advantage (toggles) · Input Display (dropdown: Off, Smash Inputs, Raw Inputs, Status Only) · Group inputs by status (toggle) · Input Delay (number 0–30 frames) |
| Controls | **Button chords** (editor, replaces the Button Config tab) · L+R+A Reset (toggle) · Menu shortcut 1 / 2 (info, from `button_config.ini`; question 9) · Permanent shortcut (info: `B + D-pad Up`) |

### Sub-screen · Button chords

![Button chords](menu-mockup/07-button-chords.png)

Six rows; defaults ZL+D-pad Down (save), ZL+D-pad Up (load), ZR+D-pad Down (record), ZR+D-pad Up (playback), ZL+D-pad Right (cycle player), ZL+D-pad Left (cycle CPU). Hold one button and press the others to re-bind; a one-button chord is allowed.

## On-screen indicators (outside the menu)

What the plugin shows while you play, unchanged in meaning from the fork unless noted:

| Indicator | Where | Meaning |
| --- | --- | --- |
| Frame advantage number | above the player | frames of advantage after a hit on shield / a hit (On screen › Frame Advantage) |
| Hitbox / hurtbox shapes | on the fighters | red attack, yellow hurtbox, grab and shield colours as the game's debug palette (On screen › Hitbox Visualization) |
| Input display | bottom corner | last inputs as Smash inputs or raw buttons, optionally grouped by status |
| Swap notice | centre | "Loading…" while a fighter swap or group build runs; it disappears when the swap is ready. A refused warm swap falls back to the normal reload without a notice (the log says why) |
| CPU stock icon | HUD | shows the CPU's current fighter **in the costume it wears** (fixed in C116.1; carried over) |
| Save-state flash | screen edge | brief flash on save / load |
| Practice-stage card | waiting room | the stage grid while Lobby Practice is on |
| Install line | `sd:/ultimate/TrainingRooms/practice-diagnostics.log` (not on screen) | one line at boot: version, sites checked, authoritative groups, anything skipped |

## Decisions for you

1. **Pages:** these five, or four (fold Display & Controls into Saves), or six (split Stages & Lobby out of Fighters & Stages)?
2. **Dropdown input:** `◂`/`▸` cycle in place (fast, easy to bump), or `A` must open the list?
3. **Random-among-a-set delays** (aerial / fast-fall / throw / pummel / ledge delay, OoS offset, reaction time, randomize slots, playback slots): keep as checklists (today's semantics) or a min–max "random in range" slider (simpler, changes the semantics)?
4. **Sub-screens** for Situation overrides, Button chords and Swap groups (as drawn), or rows on the page?
5. **First page on open:** the last page used (today) or always CPU?
6. **Item on load (older saves):** keep for pre-snapshot saves, or drop and migrate those saves once on import?
7. **Replay capture in the menu:** add the toggle (writes `replay_capture.ini`, mirrors the Deluxe page row) or keep it INI/Deluxe-only?
8. **Preload Samples:** shipped under Loading (advanced), or `diag` build only?
9. **Menu shortcuts:** read-only from the INI (as drawn) or editable here with the INI as the file of record?
10. **Help text:** one line as today, or a two-line panel?
11. **P1 CPU fallback when the pilot plugin is absent:** show the hidden group (as proposed), INI-only, or drop it?

Answered: license holder = handle; storage root = `sd:/ultimate/TrainingRooms/`.
