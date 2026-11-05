# Menu redesign: mockup and grouping table (draft for the operator's approval)

Draft 1, 2026-11-01, by the project lead. **Superseded by `../MENU-DRAFT.md`
(draft 2: today's full inventory, the per-row options/defaults/origin, and the
reconciliation with the coordinator's interactive mockup v1)**; this page keeps
the wireframe and the control definitions. Nothing here is built. The operator
approves or changes the menu before it is implemented (plan phase 2).

## What the fork's menu is today

Nine tabs, 103 rows, four control types (single toggle, multi toggle, ordered
list, slider) plus the fork's own screens (portrait grid, hat grid, Mii list,
named library, setup library, clip editor, replay window, stage menu):

| Tab | Rows | Of which single-choice | Notes |
| --- | ---: | ---: | --- |
| Mash Settings | 14 | 0 | all multi-select (random pick among selected) |
| Override Settings | 13 | 0 | one row per situation, each a multi-select of the same 13 actions |
| Defensive Settings | 14 | 5 | |
| Input Recording | 15 | 9 | 6 of the rows are the fork's named-macro rows |
| Button Config | 6 | 0 | chords (the fork retired Menu Open Start Press) |
| Save States | 13 | 6 | 2 fork rows (setup library) |
| Misc Settings | 12 | 12 | mixed bag: display, input delay, hazards, HUD, reset, lobby practice, stages, P1 CPU |
| Characters | 4 | 1 | fork |
| Character Extras | 14 | 14 | fork: Solo Popo, hats, item, 10 preload/group rows |

Problems the operator named: too many pages, settings scattered (display items
in Misc, stages in Misc, chords on their own tab), 13 near-identical override
rows, 10 preload rows most players never touch, and single-choice settings shown
as toggle rows that take a whole row's worth of cycling.

## Proposal: five pages, sections inside, dropdowns for single choices

```
 ┌──────────────────────────────────────────────────────────────────────────────┐
 │  CPU      SAVES      RECORDING      FIGHTERS & STAGES      DISPLAY & CONTROLS │  L / R: page
 ├──────────────────────────────────────────────────────────────────────────────┤
 │ ▸ Reactions                                                                  │  section header
 │   Mash                      [ Airdodge, Jump, Shield … ]      checklist   ⋯  │  A: open
 │   Followup                  [ none ]                          checklist      │
 │   Mash Triggers             [ Hitstun, Shieldstun, … ]        checklist      │
 │   Situation overrides       [ 2 of 13 set ]                   editor ▸       │  A: subscreen
 │ ▸ Defence                                                                    │
 │   Airdodge Direction        [ Neutral, Up ]                   checklist      │
 │   DI Direction              [ none ]                          checklist      │
 │   SDI Direction             [ none ]                          checklist      │
 │   SDI Strength              [ Normal          ▾ ]             dropdown       │  ◂ ▸ cycles, A opens list
 │   Clatter Strength          [ None            ▾ ]             dropdown       │
 │   Dodge Staling             [ On  ]                           toggle         │
 │ ▸ Ledge & Tech                                                               │
 │   …                                                                          │
 ├──────────────────────────────────────────────────────────────────────────────┤
 │  Help: Relative strength of the smash directional influence inputs.          │  help text of the row
 │  Y reset row · ZR reset page · X save as defaults · B close                  │
 └──────────────────────────────────────────────────────────────────────────────┘
```

Controls:

| Control | Looks like | Input | Used for |
| --- | --- | --- | --- |
| **Dropdown** (new) | `[ Value ▾ ]` | ◂/▸ cycle without opening; A opens a popup list, ▴/▾ move, A picks, B cancels | every single-choice setting (today's `ToggleSingle` rows with more than two values) |
| **Toggle** | `[ On ]` / `[ Off ]` | A or ◂/▸ flips | two-value settings |
| **Checklist** | `[ a, b, c ]` (summary) | A opens a popup list with check marks, A toggles, B closes; ZL selects all / none | multi-select settings (today's `ToggleMultiple`) |
| **Slider** | `[ 0 ━━●━━ 150 ]` | ◂/▸ move the handle; A edits the other end | ranges (damage) |
| **Editor ▸** | `[ summary ]  ▸` | A opens a sub-screen; B returns | overrides, chords, grids, libraries, Mii lists |
| **Section header** | `▸ Name` | collapsed by default when marked *(advanced)*; A on the header expands | grouping inside a page |

Rules kept from today: Y resets the focused row to its default, ZR resets the
page, X saves the current values as the defaults, B closes the menu, the help
text of the focused row shows at the bottom, and the menu opens with Minus or
the permanent B + D-pad Up.

Changed 2026-11-05 (operator, `p23/menu-nav`): ZR no longer resets the page. ZR is the role switch inside the
roster grids and was easy to hit by accident on a page. Each page now ends with a **Reset page to defaults** line,
which asks for confirmation (A reset, B cancel) and logs the page and the rows it changed. ZL + Up/Down jumps
between the sections of a page, and the menu reopens where it was closed.

Changed 2026-11-05 (review MN-1/MN-2): ZL + Up/Down jumps only where a jump means something, on a page (a section)
and in the popup lists (a window). On the other screens (the direction grids, the sliders, the roster and stage grids,
the panels) ZL acts on its press as before and Up/Down move as usual. Letting go of ZL on the same frame as another
press is not a ZL tap. The reset's confirmation says the menu shortcuts stay (they are not settings of the page).

### Page 1 — CPU (what the CPU does)

| Section | Row | Control | Values / default | Today |
| --- | --- | --- | --- | --- |
| Reactions | Mash | checklist | 13 actions / none | Mash → Mash Toggles |
| | Followup | checklist | 13 actions / none | Mash → Followup Toggles |
| | Mash Triggers | checklist | 16 situations / Hitstun, Shieldstun, … (upstream default) | Mash → Mash Triggers |
| | Situation overrides | editor ▸ | sub-screen: a dropdown picks the situation (13), a checklist sets its actions; summary shows how many are set | Override tab (13 rows) |
| Defence | Airdodge Direction | checklist | 11 directions / none | Defensive |
| | DI Direction | checklist | 11 / none | Defensive |
| | SDI Direction | checklist | 11 / none | Defensive |
| | SDI Strength | dropdown | None, Normal, Medium, High / None | Defensive |
| | Clatter Strength | dropdown | None, Normal, Medium, High / None | Defensive |
| | Dodge Staling | toggle | On | Defensive |
| Ledge & Tech | Ledge Options | checklist | 4 / default | Defensive |
| | Ledge Delay | checklist | 0–300 by 10 / none | Defensive |
| | Tech Options | checklist | 4 / all | Defensive |
| | Mistech Options | checklist | 4 / all | Defensive |
| | Hide Tech Animations | toggle | Off | Defensive |
| Shield | Shield | dropdown | None, Infinite, Hold, Constant / None | Defensive → Shield Toggles |
| | Shield Tilt | dropdown | 11 directions / Neutral | Defensive |
| | OoS Offset | checklist | 0–30 hits / none | Mash |
| | Reaction Time | checklist | 0–30 frames / none | Mash |
| Attacks & Movement | Attack Angle | checklist | Neutral, Up, Down / none | Mash |
| | Throw | checklist | 5 / none | Mash → Throw Options |
| | Throw Delay | checklist | 0–150 by 5 / none | Mash |
| | Pummel Delay | checklist | 0–150 by 5 / none | Mash |
| | Falling Aerials | checklist | Yes, No / No | Mash |
| | Aerial Delay | checklist | 0–30 / none | Mash |
| | Full Hop | checklist | Yes, No / Yes | Mash |
| | Fast Fall | checklist | Yes, No / No | Mash |
| | Fast Fall Delay | checklist | 0–30 / none | Mash |
| | Crouch | toggle | Off | Defensive |
| CPU plays Player 1 | P1 CPU Control | toggle | Off | Misc |
| | P1 CPU Level | dropdown | 1–9 / 3 | Misc |

Open point: the upstream `BoolFlag` rows (Falling Aerials, Full Hop, Fast Fall)
are multi-select on purpose (random Yes/No). The draft keeps them as
checklists; the alternative is a three-value dropdown *Yes / No / Random*.

### Page 2 — Saves (positions, damage, setups)

| Section | Row | Control | Values / default | Today |
| --- | --- | --- | --- | --- |
| Save states | Save States | toggle | On | Save States → Enable |
| | Slot | dropdown | 1–5 / 1 | Save States |
| | Randomize Slots | checklist | 1–5 / none | Save States |
| | Mirroring | dropdown | None, Alternate, Random / None | Save States |
| | Auto-load on death | toggle | Off | Save States → Auto Save States |
| | Player damage on load | dropdown | Default, Save State, Random / Default | Save States → Save Dmg (Player) |
| | Player damage range | slider | 0–150 % | Save States |
| | CPU damage on load | dropdown | Default, Save State, Random / Default | Save States |
| | CPU damage range | slider | 0–150 % | Save States |
| | Buffs on load | checklist | 9+ / none | Save States → Buff Options |
| | Item on load (older saves) | dropdown | None, Player #1–8, CPU #1–8 / None | Save States → Character Item |
| Setup library | Save setup to library… | editor ▸ | names the selected slot, writes a portable file | Save States (fork) |
| | Setup library | editor ▸ | list; A binds and restores, Y binds only | Save States (fork) |
| Training room | L+R+A Reset | toggle | On | Misc |
| | Stage Hazards | toggle | Off | Misc |

### Page 3 — Recording (inputs the CPU plays back)

| Section | Row | Control | Values / default | Today |
| --- | --- | --- | --- | --- |
| Recording | Record Slot | dropdown | 1–5 / 1 | Input Recording → Recording Slot |
| | Trigger | dropdown | Button, Save State load / Button | Input Recording |
| | Duration | dropdown | 60–600 frames / 150 | Input Recording |
| | Crop trailing neutral | toggle | On | Input Recording → Recording Crop |
| Playback | Button slots | checklist | 1–5 / 1 | Input Recording → Playback Button Slots |
| | Hitstun timing | dropdown | (upstream values) / Hitstun | Input Recording → Playback Hitstun Timing |
| | Save-state slots | checklist | 1–5 / none | Input Recording → Save State Playback |
| | Mash interrupts playback | toggle | On | Input Recording → Playback Mash Interrupt |
| | Loop | toggle | Off | Input Recording |
| Named macros | Source | dropdown | Numbered slots, Named library / Numbered | Input Recording → Macro Source (fork) |
| | Ask before switching fighter | toggle | On | → Fighter Prompt (fork) |
| | Named library | editor ▸ | | fork |
| | Save slot to library… | editor ▸ | | fork |
| | Edit placement | editor ▸ | | fork |
| | Replay window | editor ▸ | | fork |

### Page 4 — Fighters & Stages

| Section | Row | Control | Values / default | Today |
| --- | --- | --- | --- | --- |
| Player 1 and CPU | Player Swapping | toggle | Off | Characters |
| | Player Character | editor ▸ (portrait grid) | | Characters |
| | CPU Character | editor ▸ (portrait grid) | | Characters |
| | Character Cycles | editor ▸ (grid, red/blue orders, ZR switches role, Y Solo Popo) | | Characters |
| | Solo Popo (original Ice Climbers) | toggle | Off | Character Extras |
| Extras | Kirby Hats | editor ▸ (hat grid, both roles) | | Character Extras |
| | Give item | editor ▸ (item grid) | | Character Extras → Character Item choice |
| | Mii Brawler / Mii Swordfighter / Mii Gunner | dropdown each (saved Miis, replay Miis, *Add New…*) | role chosen with ZR inside the list | Character Extras (fork Mii lists) |
| Stages | Lobby Practice | toggle | Off | Misc |
| | Lobby Stages | editor ▸ (ordered stage grid; Y cycles Normal/Omega/Battlefield) | | Misc → Lobby Stages |
| | Training Stage Rotation | toggle | Off | Misc |
| Loading *(advanced, collapsed)* | Cycle Preload | dropdown | Off, 1–5 (built-in max) / max | Character Extras |
| | Preload | dropdown | Fighters, Hats, Both / Both | Character Extras → Preload Sources |
| | Ready Fighters | dropdown | total across roles / 8 | Character Extras |
| | Ready Fighter Bodies | dropdown | Off, 1–3 / native max | Character Extras |
| | Group Swap | toggle | On | Character Extras |
| | Swap groups | checklist | Ice Climbers pair bank, Original fighter bank, Solo Popo groups, Ice Climbers pair groups, PT/Aegis groups, Ice Climbers pair group, Mii groups / all on | Character Extras (7 rows → 1) |
| | Preload Samples | toggle | Off | Character Extras |

The seven group rows become one checklist because every one of them is an
independent On/Off with the same help shape; the per-entry help text moves
into the popup. Dependencies (needs Group Swap; needs 3 bodies) are shown by
greying the entry and saying why in the help line, as the fork does for
unavailable portraits.

### Page 5 — Display & Controls

| Section | Row | Control | Values / default | Today |
| --- | --- | --- | --- | --- |
| On screen | HUD | toggle | On | Misc |
| | Hitbox Visualization | toggle | Off | Misc |
| | Frame Advantage | toggle | Off | Misc |
| | Input Display | dropdown | Off, Smash, Raw, Status only / Smash (upstream default) | Misc |
| | Group inputs by status | toggle | Off | Misc → Input Display Status |
| | Input Delay | dropdown | 0–10 frames / 0 | Misc |
| Button chords | Chords | editor ▸ | sub-screen listing Save State Save, Save State Load, Input Record, Input Playback, Cycle Player, Cycle CPU; each row a chord editor (hold one, press the others) | Button Config (6 rows → 1) |
| | Menu shortcuts | read-only rows | the INI's `shortcut_1` / `shortcut_2` and the permanent B + D-pad Up | `button_config.ini` (fork) |

## Page count and row count

| | Today | Proposal |
| --- | ---: | ---: |
| Pages | 9 | 5 |
| Rows on pages | 103 | 73 (13 overrides → 1, 6 chords → 1, 7 group toggles → 1, 2 read-only shortcut rows added) |
| Single-choice rows shown as dropdowns | 0 | 24 |

## Variants to decide

1. **Four pages** instead of five: merge *Display & Controls* into *Saves* (as
   "Room & Display") — fewer pages, but the page gets long.
2. **Six pages**: split *Fighters & Stages* into *Fighters* and *Stages & Lobby*
   when lobby practice grows.
3. The *Loading (advanced)* section could live under a single "Advanced…" editor
   row instead of a collapsed section.
4. Keep the three Yes/No `BoolFlag` rows as checklists (random) or make them
   *Yes / No / Random* dropdowns.

## Open questions for the operator

1. Approve the five-page grouping, or pick a variant above.
2. Dropdown input: ◂/▸ cycling without opening the list is quick but makes
   accidental changes easier; keep it, or require A to open?
3. Should *Situation overrides*, *Chords* and *Swap groups* be sub-screens (as
   drawn) or stay as rows on the page?
4. Menu shortcuts: show them read-only (as drawn) or make them editable in the
   menu too (the INI stays the file of record either way)?
5. Which page opens first: the last page used (today) or always CPU?

## What this changes for the implementation

- The clean-room menu model (SPEC-core-02) needs the Dropdown and Section
  controls and a sub-screen stack; the fork's grids, lists and panels plug in
  as *editor* rows unchanged.
- Settings keys do not change: every row maps to an existing field in the
  settings JSON, so the fork's `training_modpack.json` files load as they are.
  The seven group toggles stay seven fields behind one checklist.
- The layout archive (SPEC-core-05) needs new pane sets for the dropdown popup
  and the section header; the artwork is ours (SPEC-core-07).
