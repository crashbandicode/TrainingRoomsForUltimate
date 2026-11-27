# Menu redesign: first proposal

The 2026-11-01 wireframe and controls below record the first proposal, before
implementation; [the later inventory](../MENU-DRAFT.md) superseded it. For the
menu in the game, use the [menu pages](README.md).

## Fork menu at the 2026-11-01 inventory

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

The inventory counted four control types (single toggle, multi toggle, ordered list, slider) plus portrait, hat, Mii and stage pickers, named and setup libraries, the clip editor and replay window.

The review identified too many pages, settings scattered (display items
in Misc, stages in Misc, chords on their own tab), 13 near-identical override
rows, 10 preload rows most players never touch, and single-choice settings shown
as toggle rows that take a whole row's worth of cycling.

## Five-page proposal with sections and dropdowns

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
| **Dropdown** (new) | `[ Value ▾ ]` | ◂/▸ cycle without opening; A opens a popup list, ▴/▾ move, A picks, B cancels | every single-choice setting (fork `ToggleSingle` rows with more than two values) |
| **Toggle** | `[ On ]` / `[ Off ]` | A or ◂/▸ flips | two-value settings |
| **Checklist** | `[ a, b, c ]` (summary) | A opens checklist or toggles item. B closes. ZL selects all/none. | multi-select settings (fork `ToggleMultiple`) |
| **Slider** | `[ 0 ━━●━━ 150 ]` | ◂/▸ move the handle; A edits the other end | ranges (damage) |
| **Editor ▸** | `[ summary ]  ▸` | A opens a sub-screen; B returns | overrides, chords, grids, libraries, Mii lists |
| **Section header** | `▸ Name` | collapsed by default when marked *(advanced)*; A on the header expands | grouping inside a page |

The draft retained these controls from the fork: Y resets the focused row to its default, ZR resets the
page, X saves the current values as the defaults, B closes the menu, the help
text of the focused row shows at the bottom, and the menu opens with Minus or
the permanent B + D-pad Up.

| Navigation changes recorded on 2026-11-05 | Decision |
| --- | --- |
| Reset (`p23/menu-nav`) | Move page reset off ZR to avoid accidental resets near roster role switching. |
| Reset page to defaults | End each page with confirmation. A reset, B cancelled, and shortcuts stayed. |
| Logging | Record the page and changed rows. |
| Reopening | Restore the player's previous menu position. |
| ZL + Up/Down (review MN-1/MN-2) | Jump one section on a page or one window in popup lists. |
| Other screens | Direction grids, sliders, roster/stage grids and panels kept separate ZL and direction presses. |
| Tap detection | Releasing ZL alongside another press did not count as a tap. |

### Page 1 — CPU (what the CPU does)

| Section | Row | Control | Values / default | Fork baseline |
| --- | --- | --- | --- | --- |
| Reactions | Mash | checklist | 13 actions / none | Mash → Mash Toggles |
| | Followup | checklist | 13 actions / none | Mash → Followup Toggles |
| | Mash Triggers | checklist | 16 situations / Hitstun, Shieldstun, … (upstream default) | Mash → Mash Triggers |
| | Situation overrides | editor ▸ | Choose among 13 situations, tick desired actions, see count configured. | Override tab (13 rows) |
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

The proposal considered checklists for random Yes/No values. Falling Aerials, Full Hop and Fast Fall now use the Yes / No / Random dropdown.

### Page 2 — Saves (positions, damage, setups)

| Section | Row | Control | Values / default | Fork baseline |
| --- | --- | --- | --- | --- |
| Save states | Save States | toggle | On | Save States → Enable |
| | Slot | dropdown | 1–5 / 1 | Save States |
| | Randomize Slots | checklist | 1–5 / none | Save States |
| | Mirroring | dropdown | None, Alternate, Random / None | Save States |
| | Auto-load on death | toggle | Off | Save States → Auto Save States |
| | Player damage on load | dropdown | Default, Save State, Random / Default | Save States → Save Dmg (Player) |
| | Player damage range | slider | 0–255 % | Save States |
| | CPU damage on load | dropdown | Default, Save State, Random / Default | Save States |
| | CPU damage range | slider | 0–150 % | Save States |
| | Buffs on load | checklist | 9+ / none | Save States → Buff Options |
| | Item on load (older saves) | dropdown | None, Player #1–8, CPU #1–8 / None | Save States → Character Item |
| Setup library | Save setup to library… | editor ▸ | names the selected slot, writes a portable file | Save States (fork) |
| | Setup library | editor ▸ | list; A binds and restores, Y binds only | Save States (fork) |
| Training room | L+R+A Reset | toggle | On | Misc |
| | Stage Hazards | toggle | Off | Misc |

### Page 3 — Recording (inputs the CPU plays back)

| Section | Row | Control | Values / default | Fork baseline |
| --- | --- | --- | --- | --- |
| Recording | Record Slot | dropdown | 1–5 / 1 | Input Recording → Recording Slot |
| | Trigger | dropdown | Button, Save State load / Button | Input Recording |
| | Duration | dropdown | 60–600 frames / 150 | Input Recording |
| | Crop trailing neutral | toggle | On | Input Recording → Recording Crop |
| | Place fighters before Record | toggle | Off | Record placement wizard |
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

| Section | Row | Control | Values / default | Fork baseline |
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
| | Ready Fighters | dropdown | total across roles / 10 | Character Extras |
| | Ready Fighter Bodies | dropdown | Off, 1–3 / native max | Character Extras |
| | Group Swap | toggle | On | Character Extras |
| | Swap groups | checklist | Seven independent switches, all on in the proposal. See list below. | Character Extras (7 rows → 1) |
| | Preload Samples | toggle | Off | Character Extras |

| Proposed switches | Entries |
| --- | --- |
| Banks | Ice Climbers pair bank, Original fighter bank |
| Groups | Solo Popo groups, Ice Climbers pair groups, PT/Aegis groups |
| Other groups | Ice Climbers pair group, Mii groups |
| Help | Independent On/Off switches shared a help format inside one popup. |
| Dependencies | Grey unavailable entries and explain the Group Swap or three-body requirement, as for portraits. |

### Page 5 — Display & Controls

| Section | Row | Control | Values / default | Fork baseline |
| --- | --- | --- | --- | --- |
| On screen | HUD | toggle | On | Misc |
| | Hitbox Visualization | toggle | Off | Misc |
| | Frame Advantage | toggle | Off | Misc |
| | Input Display | dropdown | Off, Smash, Raw, Status only / Smash (upstream default) | Misc |
| | Group inputs by status | toggle | Off | Misc → Input Display Status |
| | Input Delay | dropdown | 0–10 frames / 0 | Misc |
| Button chords | Chords | editor ▸ | Save/Load State, Input Record/Playback, Cycle Player/CPU. Hold one button and press the others. | Button Config (6 rows → 1) |
| | Menu shortcuts | read-only rows | the INI's `shortcut_1` / `shortcut_2` and the permanent B + D-pad Up | `button_config.ini` (fork) |

## Page count and row count

| | Fork baseline | Proposal |
| --- | ---: | ---: |
| Pages | 9 | 5 |
| Rows reported in the first proposal | 103 (tab counts above sum to 105) | 73 total. Merge overrides 13→1, chords 6→1, groups 7→1. Add two read-only shortcuts. |
| Single-choice rows shown as dropdowns | 0 | 24 |

## Alternatives considered on 2026-11-01

| Alternative | Trade-off discussed |
| --- | --- |
| Four pages | Merge Display & Controls into Saves as Room & Display. The resulting page grows longer. |
| Six pages | Split Fighters & Stages into Fighters and Stages & Lobby as lobby practice grows. |
| Loading editor | Put advanced Loading rows behind one editor instead of a collapsed section. |
| Yes/No values | Keep three checklists for random selection or use Yes / No / Random dropdowns. |

## Decisions after the first proposal

| Question | Alternatives reviewed | Decision |
| --- | --- | --- |
| Grouping | Four, five or six regular pages | Five regular pages plus a hidden Developer page (November 1). |
| Dropdown input | Cycle directly or require A to avoid accidental changes | Left/Right cycle directly. A opens the list (November 1). |
| Overrides, Chords and Swap groups | Separate page rows or one entry each | One editor or popup per group (November 1). |
| Menu shortcuts | Read-only INI display or in-menu editing | Two editable shortcuts write the INI. The permanent chord stays fixed (November 1). |
| Initial page | Last page used or always CPU | Last used. CPU on first open (November 2). |

The [later inventory](../MENU-DRAFT.md) records the 2026 decisions.

## Implementation assumptions in the first proposal

| Area | 2026-11-01 proposal |
| --- | --- |
| Menu model (SPEC-core-02) | Add Dropdown, Section and a sub-screen stack. |
| Editors | Reuse the fork's grids, lists and panels as editor rows. |
| Settings JSON | Preserve keys so the fork's `training_modpack.json` files load unchanged. |
| Swap groups | Keep seven settings fields behind one checklist. |
| Layout archive (SPEC-core-05) | Add dropdown and section-header panes with original artwork (SPEC-core-07). |
