# MENU-DRAFT — the clean-room menu, for the operator's green light

Draft 8, 2026-11-01, project lead (drafts 2 to 7 the same day; draft 3 is the
operator's redraft: the fork's grid, item and stage visuals where they fit, a
direction grid for DI / SDI / Airdodge / Shield Tilt; draft 4 follows the
operator's review of v3: the Kirby Hats, Solo Popo and character-items chips
are a neutral placeholder — their final icons are open and will be designed
once the functionality is settled — and the portrait and stage grids show the
real fighter icons and stage thumbnails, cropped from captures of the fork's
own grids for the review only; draft 5 follows the operator's review of v4:
the item grid uses the fork's own 38 item icons, and the stage grid draws
each thumbnail from the stage's own game UI texture at run time, so a stage
mod's picture shows — see *Grid pictures* below). Supersedes `docs/menu-redesign.md`
(draft 1, which keeps the ASCII wireframe and the control definitions). Nothing
here is built; the menu is implemented only after the operator approves this
page. Rendered screens: `docs/MENU-PROPOSAL.md` (v9; the screens carry no
origin tags, the Origin column in the tables below is the internal record).

**Draft 6 (the operator's review of v6):** the DI and SDI direction chips show
the game's own DI / SDI direction visual, read from the game at run time like
the stage thumbnails; see *Direction chips* below.

**Draft 7 (new evidence, no design change):** the visuals are identified. DI
is the game's system effect `sys_vector` (a blue streak with white sparkle
lines, turned to the launch angle); SDI is a different one, `sys_hitstop_spark`
(a white spark on the head on each SDI input frame). The DI chips on the
rendered screens are now real crops of the DI streak from a capture; the SDI
chips stay a placeholder until an SDI capture exists. The operator's Final
Destination mod replaces all four stage UI slots, and the fork's baked stage
grid does not follow it (*Grid pictures*). Evidence: `specs/SPEC-core-07`
§4.5–4.6.

**Draft 8 (new evidence, no design change):** the SDI spark is captured (a
white spiky burst with a ring outline on the CPU's head, pointing along the
SDI vector; W8), and the DI streak is confirmed to turn with the launch. The
rendered DI and SDI chips are now renders made from the game's own effect
file (W9), one still per effect turned per direction, with crops of the
captures beside them on the chip sheet as the check; screen 17 is the SDI
grid. New question 18: in the fork a changed SDI Direction applies only at
the next Training reset, and the menu does not say so. Evidence:
`specs/SPEC-core-07` §4.6, `specs/SPEC-cpu-02` question 9.

Sources for §1: the fork's menu definition as shipped in C116.1
(`training_mod_consts/src/lib.rs`, `options.rs`, `cycle_preload_config.rs`,
`lobby_stages.rs`, `training_mod_tui/src/mii_menu.rs`, `docs/menu-shortcuts.md`,
`docs/named-macros.md`, all in stage M5i) and upstream `97cbe9b` for the
upstream rows' defaults. Origin: **upstream** = Training Modpack origin
(clean-room reimplementation), **ours** = written for the fork (carried over).

Control vocabulary used in both tables:

| Control | Meaning |
| --- | --- |
| `toggle` | two values (On/Off, Yes/No) |
| `single` | today's `ToggleSingle`: one value from a list, cycled in place |
| `multi` | today's `ToggleMultiple`: a set; where the game picks at random among the selected values that is said in the note |
| `slider` | today's slider (two handles for a range) |
| `ordered` | today's ordered-list control (numbered order) |
| `grid` | the fork's portrait / hat / item / stage grids |
| `list` | the fork's scrolling lists (Mii lists, libraries, replay window) |
| `action` | a row that opens a sub-screen or performs an operation |
| `dropdown` | **new**: one value from a list; ◂/▸ cycles in place, A opens the popup list |
| `number` | **new**: one numeric value with ◂/▸ step and a popup for direct pick |
| `multi-select` | **new**: a set, edited in a popup checklist (A toggles, ZL all/none) |
| `editor` | **new name** for `action`: opens a sub-screen (grid, list, chord editor, override editor) |
| `info` | read-only row |
| `direction grid` | **new** (draft 3): a sub-screen of 3×3 chips — In · Neutral · Out across, Up · – · Down down, relative to the opponent — plus two absolute chips Left / Right; each chip draws its direction (DI and SDI: the game's own DI / SDI visual, read at run time, draft 6; Airdodge and Shield Tilt: the clean room's own arrow; *Direction chips* below); A adds one weight (the random pick is weighted, as today's counts), Y clears, ZL all / none; `(single)` = exactly one chip (Shield Tilt). The page row previews the lit chips as nine dots (DI: the selected chips themselves, draft 7; SDI: the same, draft 8) |
| `portrait grid` | the fork's 13×7 roster grid (`training_mod_tui/src/roster_grid.rs`): the stock fighter icons (whether they follow the stage rule below is question 16), red P1 / blue CPU order badges, ZR role switch, S marks, X form preview, the All cell |
| `item grid` | the fork's paged 4×8 item grid (`training_mod_consts/src/character_items.rs`, 38 items) with **the fork's own 38 item icons** (`scripts/item-icons/assets/icon-00..37.png`, 64×64, made from the game's own item files; draft 5) |
| `stage grid` | the fork's paged 4×4 stage tiles, All first, rotation badges, Y form tag (`training_mod_tui/src/stage_menu.rs`); **each thumbnail is the stage's own game UI texture, read at run time** (draft 5; *Grid pictures* below) |
| `chip` | an icon at the row's right edge (Kirby Hats, Solo Popo, character items); **draft 4: a neutral placeholder** (dashed rounded square, short label); the final icons are open and will be designed once the functionality is settled |

**Grid pictures (draft 5; answers question 13 for stages and items).**

- **Stage grid: no baked thumbnails.** When the grid opens a page, the clean
  room reads each visible stage's own UI texture from the game's file system
  at run time and draws it in the tile. That is the same file a stage mod
  replaces, so a mod that changes a stage's icon (for example a Final
  Destination replacement on the operator's card) shows in the grid. The
  repository and the layout archive carry no stage picture. The fork, by
  contrast, bakes 115 stock thumbnails into its `layout.arc`
  (`scripts/stage-grid/assets/stage-NNN.png`, decoded from the stock
  `stage_1` textures), so a stage mod never shows there. File per stage:
  `ui/replace/stage/stage_1/stage_1_<name_id>.bntx`, or `ui/replace_patch/…`
  for the 12 patch stages, with `name_id` from the game's `ui_stage_db.prc`
  (Final Destination: `stage_1_end.bntx`). The read must go through the
  mod-aware view (ARCropolis's mod file system or the game's own loader),
  never through ARCropolis's vanilla `arc:/` view. If no read succeeds, the
  tile falls back to a text tile, never to a baked picture. Contract,
  candidate load routes and unknowns: `specs/SPEC-core-07-assets-inventory.md`
  §4.5. Texture slot (draft 7): the operator's Final Destination mod
  (`Cosmic Destination`) replaces all four slots `stage_1` … `stage_4` at the
  stock sizes, and the game's own stage select shows its picture; the fork's
  grid shows the same baked tile with and without the mod.
- **Item grid:** the fork's own 38 item icons, as the fork ships them.
- **Portrait grid:** unchanged for now; whether fighter portraits follow the
  stage rule is question 16.

**Direction chips (draft 6; draft 7 adds the identified visuals).**

- **DI and SDI: the game's own visual, read at run time.** Each DI Direction
  and SDI Direction chip shows the visual the game itself draws for a DI / SDI
  direction, taken from the game's files when the grid opens, through the
  same mod-aware read as the stage thumbnails (`specs/SPEC-core-07` §4.5), so
  an effect mod shows. Nothing is drawn after it, committed or baked. The rule
  and the remaining unknowns: `specs/SPEC-core-07` §4.6.
- **Which asset (draft 7).** DI: the system effect `sys_vector` (emitter set
  `P_CmnVector` in `effect/system/common/ef_common.eff`), a blue streak with
  white sparkle lines, coloured by its emitter parameters. SDI: a different
  effect, `sys_hitstop_spark` (`P_CmnHitStopSpark`, same file), white. The
  fork and upstream request neither; the game draws them itself. Upstream's
  `di_state.svg` / `sdi_state.svg` are black glyphs and stay unused. The white
  crescent seen in the captures is the attacker's own slash (also in the
  no-DI baseline), not part of the DI visual.
- **Airdodge Direction and Shield Tilt keep the clean room's own arrow chips.**
  The game's visual is its response to DI / SDI during hitlag and launch.
  Putting it on air-dodge or shield chips would suggest a launch influence
  those options do not have. Layout, weights, Neutral and the absolute
  Left / Right are the same in all four grids.
- **Neutral** (stick at rest, no influence) keeps the ring; **Left / Right**
  use the same picture as In / Out, marked `abs`.
- **One picture rotated (draft 7).** The game draws one effect and turns it to
  the DI-corrected launch angle (SDI: to the SDI vector), so each chip is one
  picture turned to its angle of `SPEC-cpu-02` §2. The rendered DI and SDI
  chips do the same with one render per effect made from the game's effect
  file (draft 8); crops of the captured streak and spark sit beside them on
  the chip sheet as the check.
- **Fallback:** if the asset cannot be read, the chip shows the clean room's
  own arrow, never a baked copy of the game's picture.

## 1. Inventory: the menu as the fork ships it (C116.1)

9 tabs (pages), 103 visible rows plus 4 hidden submenus and 3 settings that live
outside the menu.

| page | group | item | control | options/range | default | origin | note |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Mash Settings | — | Mash Toggles | multi | Airdodge, Jump, Shield, Spotdodge, Roll In, Roll Out, Aerial, Jab, Tilts, Smash, Grab, Dash, Dash Attack | none | upstream | random pick among selected |
| Mash Settings | — | Followup Toggles | multi | same 13 actions | none | upstream | random pick |
| Mash Settings | — | Mash Triggers | multi | Hitstun, Shieldstun, Parry, Tumble, Landing, Ledge Trump, Footstool, Clatter, Ledge Option, Tech Option, Mistech Option, Grounded, Airborne, Distance: Close, Distance: Mid, Distance: Far, Always | upstream default set | upstream | |
| Mash Settings | — | Attack Angle | multi | Neutral, Up, Down | none | upstream | random pick |
| Mash Settings | — | Throw Options | multi | None, Forward, Back, Up, Down | none | upstream | random pick |
| Mash Settings | — | Throw Delay | multi | 0–150 frames by 5 | none | upstream | random pick |
| Mash Settings | — | Pummel Delay | multi | 0–150 by 5 | none | upstream | random pick |
| Mash Settings | — | Falling Aerials | multi | Yes, No | No | upstream | random pick |
| Mash Settings | — | Full Hop | multi | Yes, No | Yes | upstream | random pick |
| Mash Settings | — | Aerial Delay | multi | 0–30 frames | none | upstream | random pick |
| Mash Settings | — | Fast Fall | multi | Yes, No | No | upstream | random pick |
| Mash Settings | — | Fast Fall Delay | multi | 0–30 | none | upstream | random pick |
| Mash Settings | — | OoS Offset | multi | 0–30 hits | none | upstream | random pick |
| Mash Settings | — | Reaction Time | multi | 0–30 frames | none | upstream | random pick |
| Override Settings | — | Ledge Neutral Getup | multi | 13 actions | none | upstream | one row per situation |
| Override Settings | — | Ledge Roll | multi | 13 actions | none | upstream | |
| Override Settings | — | Ledge Jump | multi | 13 actions | none | upstream | |
| Override Settings | — | Ledge Attack | multi | 13 actions | none | upstream | |
| Override Settings | — | Tech Action | multi | 13 actions | none | upstream | |
| Override Settings | — | Clatter | multi | 13 actions | none | upstream | |
| Override Settings | — | Tumble | multi | 13 actions | none | upstream | |
| Override Settings | — | Hitstun | multi | 13 actions | none | upstream | |
| Override Settings | — | Parry | multi | 13 actions | none | upstream | |
| Override Settings | — | Shieldstun | multi | 13 actions | none | upstream | |
| Override Settings | — | Footstool | multi | 13 actions | none | upstream | |
| Override Settings | — | Landing | multi | 13 actions | none | upstream | |
| Override Settings | — | Ledge Trump | multi | 13 actions | none | upstream | |
| Defensive Settings | — | Airdodge Direction | multi | Neutral, Out, Up-Out, Up, Up-In, In, Down-In, Down, Down-Out, Left, Right | none | upstream | random pick |
| Defensive Settings | — | DI Direction | multi | 11 directions | none | upstream | random pick |
| Defensive Settings | — | SDI Direction | multi | 11 directions | none | upstream | random pick |
| Defensive Settings | — | SDI Strength | single | None, Normal, Medium, High | None | upstream | |
| Defensive Settings | — | Clatter Strength | single | None, Normal, Medium, High | None | upstream | |
| Defensive Settings | — | Ledge Options | multi | Neutral Getup, Roll, Jump, Attack, Wait, Playback Slot 1–5 | upstream default set | upstream | random pick |
| Defensive Settings | — | Ledge Delay | multi | 0–300 frames by 10 | none | upstream | random pick |
| Defensive Settings | — | Tech Options | multi | Miss Tech, Tech In Place, Tech Roll In, Tech Roll Out | all | upstream | random pick |
| Defensive Settings | — | Mistech Options | multi | Neutral Getup, Getup Attack, Roll In, Roll Out | all | upstream | random pick |
| Defensive Settings | — | Shield Toggles | single | None, Infinite, Hold, Constant | None | upstream | |
| Defensive Settings | — | Shield Tilt | single | 11 directions | Neutral | upstream | |
| Defensive Settings | — | Crouch | single | On, Off | Off | upstream | two values shown as single |
| Defensive Settings | — | Dodge Staling | single | On, Off | On | upstream | |
| Defensive Settings | — | Hide Tech Animations | single | On, Off | Off | upstream | |
| Input Recording | — | Recording Slot | single | Slot 1–5 | Slot 1 | upstream | |
| Input Recording | — | Recording Trigger | single | Button Combination, Save State Load | Button Combination | upstream | |
| Input Recording | — | Recording Duration | single | 60–600 frames by 30 | 150 | upstream | |
| Input Recording | — | Recording Crop | single | On, Off | On | upstream | |
| Input Recording | — | Playback Button Slots | multi | Slot 1–5 | Slot 1 | upstream | random pick |
| Input Recording | — | Playback Hitstun Timing | single | As Hitstun Ends, As Hitstop Ends, As Hitstop Begins | As Hitstun Ends | upstream | |
| Input Recording | — | Save State Playback | multi | Slot 1–5 | none | upstream | random pick |
| Input Recording | — | Playback Mash Interrupt | single | On, Off | On | upstream | |
| Input Recording | — | Playback Loop | single | On, Off | Off | upstream | |
| Input Recording | — | Macro Source | single | Numbered Slots, Named Library | Numbered Slots | ours | |
| Input Recording | — | Fighter Prompt | single | On, Off | On | ours | ask before a replay/named recording switches the CPU fighter |
| Input Recording | — | Named Library | action (list) | named recordings on the SD card | — | ours | A loads, Y copies into the current slot |
| Input Recording | — | Save Slot to Library | action (keyboard) | name entry | — | ours | |
| Input Recording | — | Edit Placement | action (editor) | nudge loaded macro | — | ours | |
| Input Recording | — | Replay Window | action (list + editor) | saved match captures → fighter → input window | — | ours | |
| Button Config | — | Save State Save | multi (chord) | 16 buttons | ZL + D-pad Down | upstream | hold one, press the others |
| Button Config | — | Save State Load | multi (chord) | 16 buttons | ZL + D-pad Up | upstream | |
| Button Config | — | Input Record | multi (chord) | 16 buttons | ZR + D-pad Down | upstream | |
| Button Config | — | Input Playback | multi (chord) | 16 buttons | ZR + D-pad Up | upstream | |
| Button Config | — | Cycle CPU Character | multi (chord) | 16 buttons | ZL + D-pad Left | ours | |
| Button Config | — | Cycle Player Character | multi (chord) | 16 buttons | ZL + D-pad Right | ours | |
| Save States | — | Mirroring | single | None, Alternate, Random | None | upstream | |
| Save States | — | Auto Save States | single | On, Off | Off | upstream | load on any death |
| Save States | — | Save Dmg (CPU) | single | Default, Save State, Random | Default | upstream | |
| Save States | — | Dmg Range (CPU) | slider | 0–150 %, Lower/Upper | 0–150 | upstream | |
| Save States | — | Save Dmg (Player) | single | Default, Save State, Random | Default | upstream | |
| Save States | — | Dmg Range (Player) | slider | 0–150 %, Lower/Upper | 0–150 | upstream | |
| Save States | — | Enable Save States | single | On, Off | On | upstream | help text still says Shield+Taunt (stale) |
| Save States | — | Save State Slot | single | 1–5 | 1 | upstream | |
| Save States | — | Randomize Slots | multi | 1–5 | none | upstream | |
| Save States | — | Character Item | single | None, Player 1st–8th Var., CPU 1st–8th Var. | None | upstream | fork: only for older saves without an item snapshot |
| Save States | — | Buff Options | multi | Accelerate, Oomph, Psyche Up, Bounce, Arsene, Deep Breathing, Limit, K.O. Punch, Wing, Monado arts, Power Dragon, Waft | none | upstream | |
| Save States | — | Save Setup to Library | action (keyboard) | name entry | — | ours | |
| Save States | — | Setup Library | action (list) | saved setups | — | ours | A binds+restores, Y binds |
| Misc Settings | — | Frame Advantage | single | On, Off | Off | upstream | |
| Misc Settings | — | Hitbox Visualization | single | On, Off | Off | upstream | |
| Misc Settings | — | Input Display | single | None, Smash Inputs, Raw Inputs, Status Only | Smash Inputs | upstream | |
| Misc Settings | — | Input Display Status | single | On, Off | Off | upstream | |
| Misc Settings | — | Input Delay | single | 0–30 frames | 0 | upstream | README says 0–10; code has 0–30 |
| Misc Settings | — | P1 CPU Control | single | On, Off | Off | ours | CPU AI drives P1 after the menu closes |
| Misc Settings | — | P1 CPU Level | single | 1–9 | 3 | ours | |
| Misc Settings | — | Stage Hazards | single | On, Off | Off | upstream | |
| Misc Settings | — | HUD | single | On, Off | On | upstream | |
| Misc Settings | — | L+R+A Reset | single | On, Off | On | upstream | |
| Misc Settings | — | Lobby Practice | single | On, Off | Off | ours | waiting-room practice |
| Misc Settings | — | Lobby Stages | ordered (grid) | legal stages in order + "All (listed order)" | fork default order | ours | stage grid; Y cycles Normal/Omega/Battlefield form |
| Misc Settings | — | Training Stage Rotation | single | On, Off | Off | ours | |
| Characters | — | Player Swapping | single | On, Off | Off | ours | enables P1 changes |
| Characters | — | Player Character | grid | all fighters (portraits) | Mr. Game & Watch | ours | applies at once; X previews forms |
| Characters | — | CPU Character | grid | all fighters | Pikachu | ours | |
| Characters | — | Character Cycles | grid (ordered, two roles) | fighters; red P1 order, blue CPU order, ZR switches role, Y Solo Popo on Ice Climbers | P1: G&W, Palutena, Pikachu; CPU: Pikachu, Kirby, Bowser, Bayonetta | ours | |
| Character Extras | — | Solo Popo | single | On, Off | Off | ours | P1's original Ice Climbers |
| Character Extras | — | Kirby Hats | grid (ordered, two roles) | hat orders per role | fork default list | ours | |
| Character Extras | — | Character Item (give) | grid | 38 items (`ITEM_NAMES`), turnip faces included | none | ours | gives P1 an item now; draft 2 said 45 (corrected) |
| Character Extras | — | Mii Brawler | list | saved Miis, replay Miis, Add New | role's saved pick | ours | |
| Character Extras | — | Mii Swordfighter | list | same | same | ours | |
| Character Extras | — | Mii Gunner | list | same | same | ours | |
| Character Extras | — | Cycle Preload | single | Off, 1–5 (built-in max) | built-in max | ours | |
| Character Extras | — | Preload Sources | single | Fighters, Hats, Both | Both | ours | |
| Character Extras | — | Preload Samples | single | On, Off | Off | ours | sparse timing/memory samples |
| Character Extras | — | Ready Fighter Bodies | single | Off, 2, 3 | native max (3) | ours | names from `cycle_preload_config.rs` test `ALL_NAMES` |
| Character Extras | — | Group Swap | single | On, Off | On | ours | master switch for the group rows |
| Character Extras | — | Ready Fighters | single | total across roles | 8 | ours | |
| Character Extras | — | Ice Climbers Pair Bank | single | On, Off | On | ours | needs 3 bodies |
| Character Extras | — | Original Fighter Bank | single | On, Off | On | ours | |
| Character Extras | — | Solo Popo Groups | single | On, Off | On | ours | needs Group Swap |
| Character Extras | — | Ice Climbers Pair Groups | single | On, Off | On | ours | needs Pair Bank + Group Swap |
| Character Extras | — | PT/Aegis Groups | single | On, Off | On | ours | |
| Character Extras | — | Ice Climbers Pair Group | single | On, Off | On | ours | needs Pair Bank + Group Swap |
| Character Extras | — | Mii Groups | single | On, Off | On | ours | |
| (hidden) | — | CPU Cycle Characters | ordered | fighters | Pikachu, Kirby, Bowser, Bayonetta | ours | edited through the Character Cycles grid |
| (hidden) | — | CPU Kirby Hats | ordered | hats | fork default list | ours | edited through the Kirby Hats grid |
| (hidden) | — | Player Cycle Solo Popo | toggle | On, Off | Off | ours | red S on the grid |
| (hidden) | — | CPU Solo Popo | toggle | On, Off | Off | ours | blue S on the grid |
| (outside the menu) | — | Menu shortcuts | INI | `button_config.ini`: `shortcut_1`, `shortcut_2`; permanent B + D-pad Up | Minus / disabled | ours | restart to apply |
| (outside the menu) | — | Replay capture | INI + Deluxe page | `replay_capture.ini` `enabled=on/off`; a toggle on the Deluxe Plugins page when Deluxe is present | on | ours | |
| (outside the menu) | — | Cycle preload INI | INI | `cycle_preload.ini` caches the preload/group choices for the next boot | as saved | ours | written once by the plugin |

## 2. Proposed layout: five pages, grouped, dropdowns for single choices

Page order and L/R tab titles: **CPU · Saves · Recording · Fighters & Stages · Display & Controls**.
Rows are listed in the order they appear; a `group` is a section header on the
page (collapsed by default when marked *(advanced)*). Setting ids do not
change, so existing `training_modpack.json` files load as they are.

| page | group | item | control | options/range | default | origin | note |
| --- | --- | --- | --- | --- | --- | --- | --- |
| CPU | Reactions | Mash | multi-select | 13 actions | none | upstream | random among selected (as today) |
| CPU | Reactions | Followup | multi-select | 13 actions | none | upstream | |
| CPU | Reactions | Mash Triggers | multi-select | 17 situations | upstream default set | upstream | |
| CPU | Reactions | Situation overrides | editor | sub-screen: dropdown of 13 situations + multi-select of 13 actions; row shows "n of 13 set" | none | upstream | replaces the 13-row Override tab |
| CPU | Defence | Airdodge Direction | direction grid | 9 relative chips + Left / Right, weighted | none | upstream | draft 3: chips instead of an 11-row checklist |
| CPU | Defence | DI Direction | direction grid | same | none | upstream | draft 3; draft 6: chips show the game's own DI visual, read at run time (draft 7: `sys_vector`) |
| CPU | Defence | SDI Direction | direction grid | same | none | upstream | draft 3; draft 6: chips show the game's own SDI visual, read at run time (draft 7: `sys_hitstop_spark`; draft 8: captured, screen 17); when a change applies: question 18 |
| CPU | Defence | SDI Strength | dropdown | None, Normal, Medium, High | None | upstream | was single |
| CPU | Defence | Clatter Strength | dropdown | None, Normal, Medium, High | None | upstream | was single |
| CPU | Defence | Dodge Staling | toggle | On, Off | On | upstream | |
| CPU | Ledge & Tech | Ledge Options | multi-select | Neutral Getup, Roll, Jump, Attack, Wait, Playback Slot 1–5 | upstream default set | upstream | |
| CPU | Ledge & Tech | Ledge Delay | multi-select | 0–300 by 10 | none | upstream | see open question 3 |
| CPU | Ledge & Tech | Tech Options | multi-select | 4 | all | upstream | |
| CPU | Ledge & Tech | Mistech Options | multi-select | 4 | all | upstream | |
| CPU | Ledge & Tech | Hide Tech Animations | toggle | On, Off | Off | upstream | |
| CPU | Shield | Shield | dropdown | None, Infinite, Hold, Constant | None | upstream | was "Shield Toggles", single |
| CPU | Shield | Shield Tilt | direction grid (single) | one of the 11 chips | Neutral | upstream | was single; draft 2 had a dropdown |
| CPU | Shield | OoS Offset | multi-select | 0–30 hits | none | upstream | |
| CPU | Shield | Reaction Time | multi-select | 0–30 frames | none | upstream | |
| CPU | Attacks & Movement | Attack Angle | multi-select | Neutral, Up, Down | none | upstream | |
| CPU | Attacks & Movement | Throw | multi-select | None, Forward, Back, Up, Down | none | upstream | |
| CPU | Attacks & Movement | Throw Delay | multi-select | 0–150 by 5 | none | upstream | |
| CPU | Attacks & Movement | Pummel Delay | multi-select | 0–150 by 5 | none | upstream | |
| CPU | Attacks & Movement | Falling Aerials | dropdown | Yes, No, Random | No | upstream | was a Yes/No multi; Random = both selected |
| CPU | Attacks & Movement | Aerial Delay | multi-select | 0–30 | none | upstream | |
| CPU | Attacks & Movement | Full Hop | dropdown | Yes, No, Random | Yes | upstream | same mapping |
| CPU | Attacks & Movement | Fast Fall | dropdown | Yes, No, Random | No | upstream | same mapping |
| CPU | Attacks & Movement | Fast Fall Delay | multi-select | 0–30 | none | upstream | |
| CPU | Attacks & Movement | Crouch | toggle | On, Off | Off | upstream | |
| CPU | *(fallback; shown only when the standalone CPU pilot plugin is absent)* | P1 CPU Control | toggle | On, Off | Off | ours | operator 2026-11-01: the standalone pilot owns P1 and has its own menu page; the clean room keeps only the handoff (install line `standalone_pilot_present`, pilot owns P1). Proposal: hide this group when the pilot is present and show one `info` row "P1 control: CPU pilot plugin (its own menu)" instead; see open question 11 |
| CPU | *(fallback)* | P1 CPU Level | dropdown | 1–9 | 3 | ours | same visibility rule; setting ids unchanged so existing files stay compatible |
| Saves | Save states | Save States | toggle | On, Off | On | upstream | was "Enable Save States" |
| Saves | Save states | Slot | dropdown | 1–5 | 1 | upstream | was single |
| Saves | Save states | Randomize Slots | multi-select | 1–5 | none | upstream | |
| Saves | Save states | Mirroring | dropdown | None, Alternate, Random | None | upstream | was single |
| Saves | Save states | Auto-load on death | toggle | On, Off | Off | upstream | was "Auto Save States" |
| Saves | Save states | Player damage on load | dropdown | Default, Save State, Random | Default | upstream | was single |
| Saves | Save states | Player damage range | slider | 0–150 % | 0–150 | upstream | greyed unless Random |
| Saves | Save states | CPU damage on load | dropdown | Default, Save State, Random | Default | upstream | |
| Saves | Save states | CPU damage range | slider | 0–150 % | 0–150 | upstream | greyed unless Random |
| Saves | Save states | Buffs on load | multi-select | 12 buffs | none | upstream | |
| Saves | Save states | Item on load (older saves) | dropdown | None, Player 1st–8th Var., CPU 1st–8th Var. | None | upstream | was single; see open question 6 |
| Saves | Setup library | Save setup to library… | editor | name entry for the selected slot | — | ours | |
| Saves | Setup library | Setup library | editor (list) | saved setups; A binds+restores, Y binds | — | ours | |
| Recording | Recording | Record Slot | dropdown | 1–5 | 1 | upstream | was single |
| Recording | Recording | Trigger | dropdown | Button Combination, Save State Load | Button Combination | upstream | was single |
| Recording | Recording | Duration | dropdown | 60–600 frames by 30 | 150 | upstream | was single |
| Recording | Recording | Crop trailing neutral | toggle | On, Off | On | upstream | was "Recording Crop" |
| Recording | Playback | Button slots | multi-select | 1–5 | 1 | upstream | random among selected |
| Recording | Playback | Hitstun timing | dropdown | As Hitstun Ends, As Hitstop Ends, As Hitstop Begins | As Hitstun Ends | upstream | was single |
| Recording | Playback | Save-state slots | multi-select | 1–5 | none | upstream | |
| Recording | Playback | Mash interrupts playback | toggle | On, Off | On | upstream | |
| Recording | Playback | Loop | toggle | On, Off | Off | upstream | |
| Recording | Named macros | Source | dropdown | Numbered Slots, Named Library | Numbered Slots | ours | was single |
| Recording | Named macros | Ask before switching fighter | toggle | On, Off | On | ours | was "Fighter Prompt" |
| Recording | Named macros | Named library | editor (list) | | — | ours | |
| Recording | Named macros | Save slot to library… | editor | | — | ours | |
| Recording | Named macros | Edit placement | editor | | — | ours | |
| Recording | Named macros | Replay window | editor | | — | ours | |
| Recording | Named macros | Replay capture | toggle | On, Off | On | ours | **new in menu**: writes `replay_capture.ini`; Deluxe page toggle stays in sync; see open question 7 |
| Fighters & Stages | Player 1 and CPU | Player Swapping | toggle | On, Off | Off | ours | |
| Fighters & Stages | Player 1 and CPU | Player Character | editor (portrait grid) | all fighters | Mr. Game & Watch | ours | row shows the chosen portrait |
| Fighters & Stages | Player 1 and CPU | CPU Character | editor (portrait grid) | all fighters | Pikachu | ours | row shows the chosen portrait |
| Fighters & Stages | Player 1 and CPU | Character Cycles | editor (portrait grid, two roles) | ordered P1 (red) and CPU (blue) picks | as today | ours | row previews both orders with badges; hidden CPU order and Solo Popo marks edited here |
| Fighters & Stages | Player 1 and CPU | Solo Popo (original Ice Climbers) | toggle + chip | On, Off | Off | ours | chip: placeholder (final icon open) |
| Fighters & Stages | Extras | Kirby Hats | editor (portrait grid, two roles) + chip | hat order per role | as today | ours | chip: placeholder (final icon open) |
| Fighters & Stages | Extras | Give item | editor (item grid) + chip | 38 items, 4×8 per page | none | ours | tiles: the fork's own item icons; chip: placeholder (final icon open) |
| Fighters & Stages | Extras | Mii Brawler | dropdown (list) | saved Miis, replay Miis, Add New… | role's saved pick | ours | ZR switches role inside the list |
| Fighters & Stages | Extras | Mii Swordfighter | dropdown (list) | same | same | ours | |
| Fighters & Stages | Extras | Mii Gunner | dropdown (list) | same | same | ours | |
| Fighters & Stages | Stages | Lobby Practice | toggle | On, Off | Off | ours | from Misc |
| Fighters & Stages | Stages | Lobby Stages | editor (stage grid) | ordered legal stages + All, 4×4 per page; Y cycles form | as today | ours | from Misc; row previews the first ordered stages; thumbnails read at run time from each stage's game UI texture (stage mods show) |
| Fighters & Stages | Stages | Training Stage Rotation | toggle | On, Off | Off | ours | from Misc |
| Fighters & Stages | Stages | Stage Hazards | toggle | On, Off | Off | upstream | from Misc; grouped with the stage rows (mockup v1's placement, adopted) |
| Fighters & Stages | Loading *(advanced)* | Cycle Preload | dropdown | Off, 1–5 | built-in max | ours | was single |
| Fighters & Stages | Loading *(advanced)* | Preload | dropdown | Fighters, Hats, Both | Both | ours | was "Preload Sources" |
| Fighters & Stages | Loading *(advanced)* | Ready Fighters | number | total across roles | 8 | ours | was single |
| Fighters & Stages | Loading *(advanced)* | Ready Fighter Bodies | dropdown | Off, 2, 3 | native max (3) | ours | was single |
| Fighters & Stages | Loading *(advanced)* | Group Swap | toggle | On, Off | On | ours | master switch |
| Fighters & Stages | Loading *(advanced)* | Swap groups | multi-select | Ice Climbers pair bank, Original fighter bank, Solo Popo groups, Ice Climbers pair groups, PT/Aegis groups, Ice Climbers pair group, Mii groups | all on | ours | replaces 7 toggle rows; entries grey with the reason when a dependency is off |
| Fighters & Stages | Loading *(advanced)* | Preload Samples | toggle | On, Off | Off | ours | diagnostics; see open question 8 |
| Display & Controls | On screen | HUD | toggle | On, Off | On | upstream | from Misc |
| Display & Controls | On screen | Hitbox Visualization | toggle | On, Off | Off | upstream | from Misc |
| Display & Controls | On screen | Frame Advantage | toggle | On, Off | Off | upstream | from Misc |
| Display & Controls | On screen | Input Display | dropdown | Off, Smash Inputs, Raw Inputs, Status Only | Smash Inputs | upstream | was single ("None" renamed Off) |
| Display & Controls | On screen | Group inputs by status | toggle | On, Off | Off | upstream | was "Input Display Status" |
| Display & Controls | On screen | Input Delay | number | 0–30 frames | 0 | upstream | was single |
| Display & Controls | Controls | Button chords | editor | sub-screen with 6 chord rows: Save State Save, Save State Load, Input Record, Input Playback, Cycle Player, Cycle CPU | as today | upstream (4) / ours (2) | replaces the Button Config tab |
| Display & Controls | Controls | L+R+A Reset | toggle | On, Off | On | upstream | from Misc; grouped with the controls (mockup v1's placement, adopted) |
| Display & Controls | Controls | Menu shortcut 1 | info | from `button_config.ini` | Minus | ours | read-only; INI stays the editor |
| Display & Controls | Controls | Menu shortcut 2 | info | from `button_config.ini` | disabled | ours | |
| Display & Controls | Controls | Permanent shortcut | info | B + D-pad Up | — | ours | |

Counts: pages 9 → 5; visible rows 103 → 76 with the standalone pilot installed
(13 override rows → 1 editor, 6 chord rows → 1 editor, 7 group toggles → 1
multi-select, 2 P1 CPU rows → 1 info row, +1 Replay capture, +3 info rows), 77
when the pilot is absent and the two fallback rows show; dropdowns 0 → 22 (23
with the fallback; Shield Tilt is a direction grid in draft 3, not a dropdown);
direction grids 4; sub-screens 11 → 18 (14 editors + 4 direction grids).

## 3. Rationale per page, and what is dropped or merged

| page | one-line rationale |
| --- | --- |
| CPU | Everything that changes what the CPU does, in the order a player thinks about a situation: how it reacts, how it defends, ledge and tech, shield, attacks and movement. Driving P1 with the CPU belongs to the standalone CPU pilot plugin and its own page; the clean room only hands P1 over to it. |
| Saves | Positions and damage: the save-state rules and the named setups built on them. |
| Recording | Inputs the CPU plays back: numbered recordings, playback rules, and the fork's named-macro and replay tools that feed the same slots. |
| Fighters & Stages | Who is on screen and where: swaps, cycles, hats, items, Miis, the stage rows (lobby practice, stage list, rotation, hazards), and (collapsed) the loading machinery that makes swaps fast. |
| Display & Controls | What is drawn and which buttons do what (chords, the L+R+A reset chord, the menu shortcuts); nothing here changes what the CPU does. |

Dropped or merged:

| item today | proposal | why |
| --- | --- | --- |
| Override Settings tab (13 rows) | merged into one *Situation overrides* editor (CPU page) | identical rows differing only in the situation; the editor picks the situation from a dropdown |
| Button Config tab (6 rows) | merged into one *Button chords* editor (Display & Controls) | rarely changed; one sub-screen keeps the chord-entry rule in one place |
| 7 swap-group toggles | merged into one *Swap groups* multi-select | all independent On/Off with the same help shape; dependencies shown by greying |
| Falling Aerials, Full Hop, Fast Fall (Yes/No multi) | *Yes / No / Random* dropdowns | the only set semantics a Yes/No multi can express is "random", so name it |
| Menu Open Start Press (retired field) | dropped from the menu; the field stays in the JSON for old files | the INI replaced it in the fork |
| Auto-Update (upstream) | dropped (already gone in the fork) | no network code in the clean room |
| "Character Item" under Save States | kept but renamed *Item on load (older saves)*; proposed to drop once the fork's item snapshot is the only path (open question 6) | duplicates *Give item* for new saves |
| Misc Settings tab | dissolved into Saves (reset, hazards), Fighters & Stages (lobby, stages), Display & Controls (HUD, hitboxes, frame advantage, input display, input delay), CPU (P1 control) | the grab-bag page was the main source of "where is it" |
| Characters + Character Extras tabs | merged into *Fighters & Stages* with the loading rows collapsed | extras are either per-fighter cosmetics or advanced loading knobs |
| Stale help text ("Shield+Down Taunt") | replaced by the configured chord, read live | wrong today |
| P1 CPU Control, P1 CPU Level (Misc) | removed from the normal layout; kept as a hidden fallback group on the CPU page that appears only when `libssbu_cpu_pilot` is not on the card; with the pilot present, one info row points at the pilot's page | operator 2026-11-01: the standalone pilot works online, is on the card and has its own menu page; the clean room must not duplicate it. The handoff (install line `standalone_pilot_present`, pilot owns P1) is carried over unchanged. |

## 4. Open questions for the operator

Decided 2026-11-01 (not menu questions; recorded here because the review doc
carries this page): the MIT copyright holder is the handle
(`Copyright (c) 2026 intpa`); the storage root is `sd:/ultimate/TrainingRooms/`
with a one-time first-boot import of the fork's settings, save states, Mii
presets and macro library, originals untouched (`specs/SPEC-core-08`). The
persistence behind every row above therefore lives under the new root.
Question 13 is answered (draft 5); questions 1–12 and 14–18 remain open.

1. **Five pages as laid out**, or the variants in `docs/menu-redesign.md` (four pages by merging *Display & Controls* into *Saves*; six by splitting *Stages & Lobby* out of *Fighters & Stages*)?
2. **Dropdown input**: ◂/▸ cycles the value in place without opening the list (fast, but easier to change by accident). Keep, or require A to open?
3. **Random-among-a-set delays** (Aerial/Fast Fall/Throw/Pummel/Ledge Delay, OoS Offset, Reaction Time, Randomize Slots, Playback slots): keep as multi-select checklists (exactly today's semantics), or change the delays to a *min–max range* slider ("random in range") which is simpler to read but changes the semantics?
4. **Sub-screens**: *Situation overrides*, *Button chords* and *Swap groups* as sub-screens/popups (as drawn), or as rows on the page?
5. **First page on open**: the last page used (today's behaviour) or always *CPU*?
6. **Item on load (older saves)**: keep for compatibility with saves made before the fork's item snapshot, or drop it and migrate those saves (the fork's JSON import could set the snapshot from this field once)?
7. **Replay capture in the menu**: add the toggle (it writes `replay_capture.ini` and mirrors the Deluxe Plugins page row), or keep it INI/Deluxe-only as today?
8. **Preload Samples** (writes sparse timing/memory samples to the diagnostics file): keep as a shipped setting under *Loading (advanced)*, or move it to the `diag` build only?
9. **Menu shortcuts**: shown read-only from the INI (as drawn), or editable in the menu with the INI as the file of record?
10. **Help text position and length**: one line at the bottom as today, or a two-line panel so the longer fork help texts (group rows) do not need shortening?
11. **P1 CPU fallback** (the standalone pilot now owns P1): when the pilot is absent, show the two fallback rows on the CPU page (as proposed), keep the fallback INI-only, or drop the TM-side control altogether and require the pilot?
12. **Direction grid weights** (draft 3): show the weight digit on a selected chip (upstream's counts, today's semantics, as drawn) or plain on/off chips with a uniform random pick?
13. **Grid picture source** — *answered 2026-11-01 (draft 5).* Background (draft 4): the fork commits its grid pictures — `scripts/roster-grid/assets/` 89 stock icons (`sources.json`: SmashWiki copies), `scripts/stage-grid/assets/` 115 thumbnails decoded from the game's `stage_1` textures, `scripts/item-icons/assets/` 38 icons rendered from the game's item files — and builds them into its patched `layout.arc`. **Answer:** stage thumbnails are not carried over or baked at all; the stage grid reads each stage's own game UI texture at run time, through the mod-aware file view, so stage mods show (*Grid pictures* in the vocabulary section; `specs/SPEC-core-07` §4.5). The item grid uses the fork's own 38 item icons. Fighter portraits: question 16. The review crops and copies in `docs/menu-mockup/captures/` stay mockup-only.
14. **Grid paging** (draft 3): items 4×8 per page (two pages of 38) and stages 4×4 (eight pages of All + 117) as the fork does, or denser pages so the items fit on one screen?
15. **Mii rows** (draft 3): lists as drawn, with the fork's exact bust thumbnail beside each saved Mii where one exists, or text only? (Saved Miis have no stock portrait, so a portrait grid does not apply.)
16. **Fighter portraits and mods** (draft 5, open): should the portrait grid follow the stage rule — read each fighter's own UI portrait texture from the game at run time, so character and costume mods show — or keep a fixed set of stock icons, as the fork does with its 89 committed icons? Not decided.
17. **DI / SDI visual** (draft 6, open): (a) confirm that only the DI and SDI chips show the game's own visual and that Airdodge Direction and Shield Tilt keep the clean room's arrows (as proposed; unchanged in draft 7); (b) *status, draft 7:* the visuals are identified (DI `sys_vector`, SDI `sys_hitstop_spark`, two different effects) and the DI streak is captured; an SDI capture is still to be scheduled (`specs/SPEC-core-07` §4.6). *Status, draft 8:* the SDI spark is captured (Up and In) and the DI turn with the launch is confirmed; the screens draw renders made from the game's effect file, the captures are the check. (a) is still open.
18. **When a CPU setting change takes effect** (draft 8, open): in the fork, a CPU SDI Direction changed in the menu takes effect only at the next Training reset (`sdi::roll_direction()` ← `reset::on_reset()`), and the menu does not say so; other per-reset rolls (shield tilt) may do the same. Recommendation: apply the change immediately (re-roll when the row changes), or at least show "applies on next reset" on the row. Not decided (`specs/SPEC-cpu-02` question 9).

## 5. Reconciliation with the coordinator's mockup v1

v1 = the `SPEC` array in the coordinator's interactive page
`trainingrooms-menu-mockup.html` (session scratchpad; since deleted — the
operator wants PNG/SVG renderings of the in-game look, which the coordinator
renders from this draft). This draft is the design authority; the differences
below are what the renderings must get right, each with its reason. Origin
labels (`up`/`ours`) in v1 agree with `PROVENANCE.md` for every row it has.
v1 also kept a *CPU as player 1* group; per the operator it is now the hidden
fallback described in §3.

### 5.1 Agreed and adopted from v1

- Five pages; dropdowns for single choices; chip sets for sets; sliders for the
  damage ranges; *Situation overrides* as one situation dropdown plus an action
  chip set; the swap-group switches as one chip set.
- Stage Hazards grouped with the stage rows and L+R+A Reset grouped with the
  controls (both moved in this draft, §2).

### 5.2 Grouping differences

| topic | v1 | this draft | reason |
| --- | --- | --- | --- |
| Pages | CPU · Fighters · States & Recording · Display & Stage · Controls | CPU · Saves · Recording · Fighters & Stages · Display & Controls | v1's *States & Recording* is the longest page (29 rows) while *Controls* has 8 rows of which 6 collapse into one chord editor, leaving a near-empty page; splitting saves from recording (15/16 rows) and folding controls into the display page evens the pages out. Operator choice (open question 1). |
| Stage rows | on *Display & Stage* | on *Fighters & Stages* | Lobby Practice gates the swaps, hats, items and Miis in the waiting room, and Lobby Stages / Training Stage Rotation decide what the swaps happen on; players set them together. |
| Cycle rosters, Kirby hats | two rows each (Player, CPU) | one two-role grid each (ZR switches role; the CPU order is a hidden submenu edited through the grid) | that is the control the fork shipped and tested; two separate editors would duplicate the grid. Either is acceptable. |
| Solo Popo | two toggles | one toggle (original Ice Climbers) + the red/blue S marks on the cycle grid | the fork has three independent settings; two of them are per-slot marks on the grid, not rows. |
| Loading rows | open on the page | collapsed *(advanced)* section | ten rows most players never touch. |

### 5.3 Rows missing from v1

Mii Brawler / Mii Swordfighter / Mii Gunner (saved-Mii lists with *Add New…*),
*Give item* (the Character Extras item grid), *Lobby Stages* (ordered stage grid
with the *All* entry and the Y form cycle), *Replay capture* (today INI/Deluxe
page; proposed as a menu toggle, open question 7), the second menu shortcut and
the permanent B + D-pad Up, *Preload* value *Hats*, the *Ice Climbers Pair Group*
switch (distinct from *Ice Climbers Pair Groups*, see below).

### 5.4 Option lists and control kinds in v1 that do not match the shipped definition

Shipped values are from `training_mod_consts/src/options.rs`,
`cycle_preload_config.rs` and `lib.rs` in stage M5i (§1 above).

| v1 row | v1 options / kind | shipped |
| --- | --- | --- |
| Mash actions, Follow-up actions, override actions | 11 / 5 / 8 items incl. "Up B", "Side B", "Neutral B" | the same 13 for all three: Airdodge, Jump, Shield, Spotdodge, Roll In, Roll Out, Aerial, Jab, Tilts, Smash Attacks, Grab, Dash, Dash Attack |
| Mash triggers | 15 incl. one "Distance" | 17: …, Distance: Close, Distance: Mid, Distance: Far, Always |
| Throw | 4 | 5 (None, Forward, Back, Up, Down) |
| Throw delay, Pummel delay | 0, 10, 20, 30, 60 | 0–150 by 5 (31 values) |
| Aerial delay, Fast fall delay, Reaction time | 0, 1, 2, 3, 5, 10 | 0–30 (31 values) |
| OoS offset | frames 0–10 | 0–30 **hits** |
| Airdodge / DI / SDI direction | 5–6 incl. "Random", "Diagonals" | 11: Neutral, Out, Up-Out, Up, Up-In, In, Down-In, Down, Down-Out, Left, Right; "random" is several selected |
| Ledge options | 5 | 10: Neutral Getup, Roll, Jump, Attack, Wait, Playback Slot 1–5 |
| Ledge delay | 0, 10, 20, 30, Random | 0–300 by 10 |
| Shield tilt | None, Up, Down, Left, Right, Diagonals | the 11 directions; default Neutral |
| Falling aerials, Full hop, Fast fall | Yes/No chips | this draft: dropdown Yes / No / Random (Random = both selected today) |
| Save damage (player, CPU) | Default, Saved, Random range | Default, Save State, Random |
| Character item | 5 shown | None, Player 1st–8th Var., CPU 1st–8th Var. (17) |
| Buffs | "Monad", "Wafting", "Breathing" | Monado arts, Waft, Deep Breathing (take the exact labels from `options.rs` for v2) |
| Recording trigger | Command, Save state load | Button Combination, Save State Load |
| Recording crop | None / Trim start / Trim end / Both | On / Off (removes trailing neutral frames) |
| Playback hitstun timing | Instant, After hitstun, After walk | As Hitstun Ends, As Hitstop Ends, As Hitstop Begins |
| Macro source | Slots, Named library | Numbered Slots, Named Library |
| Named library, Setup library, Replay window | dropdowns with sample names ("Ledge trap 1", "Last 3") | lists / sub-screens of files and captures; the sample names are invented and must not appear in v2 |
| Save setup to library, Save slot to library, Edit placement | toggles | actions (name entry; placement editor) |
| Input display | None, Smash, Raw | + Status Only |
| Input delay | 0–10 | 0–30 |
| Training stage rotation | Off / Legal stages / Custom list | On / Off (it uses the Lobby Stages order) |
| Preload sources | Both, Player, CPU | Fighters, Hats, Both |
| Preload samples | dropdown Off/On | toggle |
| Cycle preload | toggle | Off, 1–5 (default: the built-in maximum) |
| Ready fighters | 4–12 | the numeric set in `cycle_preload_config.rs`, default 8 (confirm the exact set before v2) |
| Ready fighter bodies | 2, 3 | Off, 2, 3 |
| Group kinds | 4 chips (PT/Aegis, Ice Climbers pair, Solo Popo, Mii) + 2 toggles | 7 switches: Ice Climbers Pair Bank, Original Fighter Bank, Solo Popo Groups, Ice Climbers Pair Groups (a pair window may own groups), PT/Aegis Groups, Ice Climbers Pair Group (the pair is itself a group), Mii Groups; this draft puts all 7 in one chip set |
| Shortcuts (6 chords) | dropdowns of preset chords ("Grab + D-pad Down", "ZR + D-pad Left") | 16-button chord editors (hold one, press the others); defaults ZL + D-pad Down, ZL + D-pad Up, ZR + D-pad Down, ZR + D-pad Up, ZL + D-pad Left, ZL + D-pad Right |
| Menu shortcut | Minus / Plus / Disabled | two INI slots, each any `+` chord of the 16 names (`disabled` allowed), plus the permanent B + D-pad Up; this draft shows them read-only (open question 9) |
| Row count | "112 submenus" | 103 visible rows + 4 hidden submenus in the shipped feature set (§1); the difference is rows behind cargo features not in the C116.1 build |

### 5.5 Resource cost per upstream item (for the mockup's info panel)

From `docs/resource-costs.md` Table B and `PROVENANCE.md` §B; "not measured"
means the fork never built the feature in isolation.

| upstream rows | memory | CPU | confidence |
| --- | --- | --- | --- |
| Mash, Followup, Mash Triggers, overrides, Attack Angle, Throw, delays, Full Hop, Fast Fall | lock-protected scalars + a small action queue | 4 per-call hooks + 1 per-frame call per fighter | not measured |
| Airdodge / DI / SDI, Clatter, Dodge Staling, Ledge, Tech, Hide Tech, Shield, Shield Tilt, Crouch | lock-protected scalars | 7 on-event + 6 per-call hooks + 7 per-frame calls per fighter | not measured |
| Frame Advantage | small heap vector | 1 per-frame call + 1 per-call hook | not measured |
| Save States (all rows) | ≈1.6 KB heap for 10 slots (derived); upstream's `toml` persistence crate 110,592 B of NRO, removed by the fork's JSON | 1 per-frame call + 8 per-call hooks | partly measured |
| Buffs, Character Item, fighter-specific restore | — | 5 / 16 / 3 hooks, on load | not measured |
| L+R+A Reset | — | 1 inline hook | not measured |
| Stage Hazards | 2 cached addresses | 2 boot-time text scans + 1 inline hook | not measured |
| Input Recording rows | 24,000 B static table (derived) | per-frame input hook + 1 CPU hook | not measured |
| Input Display rows | 15-entry ring + ≈2,300 status strings; the fork's renderer fix +12,288 B image | 212→1 pane lookups per unchanged frame, 27.0 ms→0.216 ms per 120 rapid frames (host, measured) | measured (host) |
| Input Delay | ≤ 30 × 8 B queue | one push per P1 poll | derived |
| Button chords | small hash map | per-frame in the input hook | not measured |
| HUD (layout archive) | inflated archive 4,213,268 B on the game heap for the process lifetime upstream; 5–9 s per load in the fork | `handle_draw` on every layout draw | measured (log) |
| Hitbox Visualization | 360 B static tables | 1 per-frame call + 3 hooks | derived |
| Menu (all pages) | 3 static settings copies + the menu tree on the heap; settings file 22,589 B | per-frame condition + per-player input hook + draw | not measured |
