# Assets inventory (upstream artwork to replace; fork asset generators) — behavioural specification

- **Spec id:** `SPEC-core-07`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required (**artwork must be redrawn or replaced, never copied**)
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), spec-writer role, 2026-11-01
- **Upstream reference read (spec writer only):** `src/static/img/*.svg` (42 files; metadata and a few small files read in full, the rest by metadata search), `src/static/layout.arc` (member names and `TrMod*` pane names extracted by string search of the binary; no archive was unpacked), `src/training/ui/{menu,mod,display,input_log}.rs` (pane names referenced), `photos/*` (README images, by name only) at commit `97cbe9b9` (unchanged in `077a8e43`); fork: `src/static/layout.arc` (size only), `src/static/waiting_ui.patch`, `scripts/make_waiting_patch.py`, `scripts/waiting_patch_manifest.json`, `scripts/roster-grid/README.md`, `scripts/stage-grid/README.md`, `scripts/item-icons/assets/sources.json` (head), from `docs/provenance/provenance-files.md`.
- **Implementers must not open the upstream paths above.** Everything an implementer and a designer need is in this file and in SPEC-core-03/05.

## 1. Purpose (one paragraph)

This spec lists every piece of upstream artwork and layout data the fork inherited, says where each is used, and states that none of it may be carried into the clean room: the upstream repository publishes no license, one icon is Font Awesome Pro (commercial license), one is an Adobe Illustrator export with a Japanese layer name that is probably a game texture, and the layout archive embeds the game's own stock files. A designer redraws the icons from the semantic list below; the layout parts are rebuilt from the player's own copy of the game's stock archive with the fork's builder, which already derives all of its own portrait, stage and item images from the game's files at build time.

## 2. User-visible behaviour

- Each submenu button on the menu's submenu page shows an icon that illustrates the setting (SPEC-core-03 §2.2); toggle pages show a check mark or a weight digit `1`–`5`.
- The notification panel, the input-log rows and the slider panel are plugin panes with no artwork of their own beyond backgrounds and text (they clone stock game materials).
- Replacing artwork changes nothing functionally; the pane names and the mapping from submenu id to icon are the contract (SPEC-core-03 §4.2).

## 3. Inputs

None at runtime beyond the archive (SPEC-core-05). At build time: the player's own stock `info_training` layout archive (and, for the fork's waiting room, the stock `info_waiting` archive), the redrawn icons, the fork's generated portraits/stage/item images.

## 4. Outputs and native state touched

None beyond SPEC-core-05 (the injected archive). This spec is an inventory.

### 4.1 Upstream SVG icons (`src/static/img/`, 42 files, all to be redrawn)

Every file except the three marked is an Inkscape-authored drawing (`sodipodi`/`inkscape` namespaces; 39 files), with an empty `dc:title` where metadata exists; no license statements, no `<text>` labels. The spec writer did not render them; "what it should convey" is the setting's meaning, which the redraw must illustrate. The "used by" column is the pane name inside each button's `Icon` container (SPEC-core-03 §4.2), which equals the submenu id (one truncated to 24 characters).

| File | Used by (pane / submenu id) | What the new icon should convey | Origin notes |
| --- | --- | --- | --- |
| `aerial_delay.svg` | `aerial_delay` | delay before an aerial attack (airborne fighter + clock) | Inkscape, has a grid |
| `air_dodge_dir.svg` | `air_dodge_dir` | air-dodge direction (arrows around a dodging figure) | Inkscape |
| `attack_angle.svg` | `attack_angle` | angled attack (up/neutral/down angle marks) | Inkscape |
| `buff_state.svg` | `buff_state` | a buff/power-up applied on load | Inkscape |
| `character_item.svg` | `character_item` | a character's item given on load | Inkscape (12 primitives) |
| `check.svg` | `check` (toggle pages) | a check mark | **Font Awesome Pro 6.1.2, commercial license (comment in file: "Font Awesome Pro 6.1.2 by @fontawesome … License - https://fontawesome.com/license (Commercial License) Copyright 2022 Fonticons, Inc."); filled `#47d147` with a black stroke. Must be replaced with an original mark or a Free-licensed glyph with attribution** |
| `clatter_strength.svg` | `clatter_strength` | mash-out-of-grab strength (a round emblem with an inner segment) | Inkscape |
| `crouch.svg` | `crouch` | a crouching figure | Inkscape (11 primitives) |
| `di_state.svg` | `di_state` | directional influence (launch arrow bent by stick direction) | Inkscape (8 primitives) |
| `falling_aerials.svg` | `falling_aerials` | aerial while falling (figure descending with an attack) | Inkscape, grid |
| `fast_fall.svg` | `fast_fall` | fast fall (down arrow with speed lines) | Inkscape |
| `fast_fall_delay.svg` | `fast_fall_delay` | fast fall with a delay (down arrow + clock) | Inkscape |
| `follow_up.svg` | `follow_up` | an action after the first (two linked actions) | Inkscape |
| `frame_advantage.svg` | `frame_advantage` | frame advantage readout (plus/minus frames) | Inkscape |
| `full_hop.svg` | `full_hop` | full hop vs short hop (two jump heights) | Inkscape |
| `hitbox_vis.svg` | `hitbox_vis` | hitbox spheres on a fighter | Inkscape |
| `input_delay.svg` | `input_delay` | delayed input (controller + clock) | Inkscape |
| `ledge_delay.svg` | `ledge_delay` | ledge option delay (ledge + clock) | Inkscape |
| `ledge_state.svg` | `ledge_state` | ledge options (figure on a ledge) | Inkscape |
| `m_retnormal.svg` | not referenced by name in code; probably rasterized into the combined texture as a "return/back" arrow | a return arrow | **Adobe Illustrator 18.1.1 export, layer `レイヤー_2`, white arrow on 80×80: by name (`m_ret…`) and tooling this is most likely a game UI texture; must not be used** |
| `mash_state.svg` | `mash_state` | mashing (a rotated button being pressed with a stroke) | Inkscape (rect + line) |
| `mash_triggers.svg` | `mash_triggers` | what triggers a mash (a trigger/lightning + figure) | Inkscape |
| `miss_tech_state.svg` | `miss_tech_state` | missed-tech options (figure on the ground) | Inkscape (14 primitives) |
| `oos_offset.svg` | `oos_offset` | out-of-shield after N hits (shield with hit counters) | Inkscape |
| `pummel_delay.svg` | `pummel_delay` | pummel delay (grab + clock) | Inkscape |
| `quick_menu.svg` | not referenced by name in code; likely the menu/title badge | three chevrons `>>>` (80×80, black 2.1 px strokes) | Inkscape; original drawing but upstream-owned: redraw |
| `reaction_time.svg` | `reaction_time` | reaction time (eye/clock) | Inkscape |
| `save_damage_cpu.svg` | `save_damage_cpu` (fork reuses it for `fighter_prompt`) | CPU damage kept on load (CPU chip + percent) | Inkscape |
| `save_damage_limits_cpu.svg` | `save_damage_limits_cpu` | CPU random damage range (percent range bar) | Inkscape |
| `save_damage_limits_player.svg` | `save_damage_limits_playe` (24-char pane name) | player random damage range | Inkscape |
| `save_damage_player.svg` | `save_damage_player` | player damage kept on load | Inkscape |
| `save_state_autoload.svg` | `save_state_autoload` | auto-load on death | Inkscape (9 primitives) |
| `save_state_enable.svg` | `save_state_enable` | save states on/off (floppy/bookmark) | Inkscape |
| `save_state_mirroring.svg` | `save_state_mirroring` | mirrored position (figure and its reflection) | Inkscape |
| `sdi_state.svg` | `sdi_state` | SDI direction | Inkscape |
| `sdi_strength.svg` | `sdi_strength` | SDI strength (stick with intensity marks) | Inkscape |
| `shield_state.svg` | `shield_state` | shield behaviour (shield bubble) | Inkscape |
| `shield_tilt.svg` | `shield_tilt` | shield tilt direction | Inkscape |
| `stage_hazards.svg` | `stage_hazards` | stage hazards on/off | Inkscape |
| `tech_state.svg` | `tech_state` | tech options (figure hitting the ground and rolling) | Inkscape (12 primitives) |
| `throw_delay.svg` | `throw_delay` | throw delay (grab + clock) | Inkscape (11 primitives) |
| `throw_state.svg` | `throw_state` | throw direction (figure thrown with arrows) | Inkscape (18 primitives) |

Submenus without an SVG of their own upstream (their icon panes may be absent or shared in the archive): `input_display`, `input_display_status`, `hud`, `lra_reset`, `stale_dodges`, `tech_hide`, `update_policy`, `menu_open_start_press`, `recording_slot`, `record_trigger`, `recording_duration`, `recording_crop`, `playback_button_slots`, `hitstun_playback`, `save_state_playback`, `playback_mash`, `playback_loop`, `save_state_slot`, `randomize_slots`, `save_state_save`, `save_state_load`, `input_record`, `input_playback`. Upstream's renderer hides an icon it cannot find, so these rows show no icon. The clean room draws icons for every row (Q1).

### 4.2 Upstream `layout.arc` (4,213,268 B; the game's stock `info_training` archive plus additions)

Members (by name; the stock ones belong to the game and come from the player's own copy at build time):

- **Plugin layout files (to be recreated):** `blyt/training_mod_menu_button.bflyt`, `blyt/training_mod_menu_button_row.bflyt`, `blyt/training_mod_menu_slider.bflyt`, `blyt/training_mod_input_icon.bflyt`, `blyt/training_mod_input_log.bflyt`.
- **Stock layout files (game-owned, in the stock archive):** `blyt/info_training.bflyt` (modified by upstream to add Parts panes and plugin containers), `blyt/com_btn_arrow_00.bflyt`, `blyt/com_btn_keyhelp_00.bflyt`, `blyt/com_csr_lct0_01.bflyt`, `blyt/com_csr_lct1_01.bflyt`, `blyt/com_scr_btn_00_knob.bflyt`, `blyt/com_scr_lct_00_bg.bflyt`, `blyt/com_system_anim.bflyt`, `blyt/info_training_btn0_00_item.bflyt`, `blyt/info_training_btn0_01_itembtn.bflyt`, `blyt/info_training_btn0_02_others.bflyt`, `blyt/info_training_btn0_03_itemsw.bflyt`, `blyt/info_training_btn0_04_item_other.bflyt`, `blyt/info_training_btn1_00_finish.bflyt`.
- **Stock animations (game-owned):** 157 `anim/*.bflan` files (`com_btn_*`, `com_csr_*`, `com_scr_*`, `com_system_anim_*`, `info_training_*`), among them `anim/info_training_in_menu.bflan`, whose movement of `L_staying_help` from −80 to 0 the plugin reads to detect the pause menu (SPEC-core-03).
- **Texture atlas:** `timg/__Combined.bntx` — the game's textures plus the rasterized plugin icons, digits and check mark. Any texture the plugin adds must be new and the stock ones must be byte-identical to the player's copy (the fork's builder checks original texture metadata, pixels, mip offset arrays and 4 KiB pool alignment).

Plugin pane names present in the archive (the runtime contract; see SPEC-core-03 §4.2 and SPEC-input-* for the input log and notifications): `TrModMenu`, `TrModMenuButtonRow0`–`TrModMenuButtonRow7` (each with `Button0`–`Button3` parts containing `TitleTxt`, `TitleBg`, `Icon` + stacked icon pictures named by submenu id, `check`, `1`–`5`, `ValueTxt`), `TrModSlider` (`Background`, `Header`, `MinButton`, `MaxButton`), `TrModTitle` (`set_txt_00`, `set_txt_01`), `TrModDisp0` (+ `TrModDisp0Header`, `TrModDisp0Txt`, `TrModDIsp0Base` — note the upstream capitalisation typo), `TrModInputLog` and `TrModInputLog0`–`TrModInputLog14` (15 rows), `TrModSelectors`, plus key-help parts `LeftTab`, `CurrentTab`, `RightTab`, `SaveDefaults`, `ResetDefaults`, `ClearToggle` (each with `set_txt_icon`, `set_txt_help`) and `FooterTxt`. Native panes the plugin reads or hides: `L_staying_help`, `status_R`. Pane names are limited to 24 characters.

Design notes recorded by upstream and the fork for whoever rebuilds the parts: a new BFLYT per component plus a Parts pane in `info_training.bflyt` whose part name is the file stem; panes copied from stock parts must be renamed so the stock animations stop driving them; portrait/picture panes should clone an existing plain material (the fork clones `pic_icn_speed`: identity texture transforms, UV source 0, shader variant `0x19`) and never the screen-projected stripe material of `FooterBackground`; text fonts/materials clone `TrModDisp0Txt`; the native canvas is 1920×1080 mapped to 1280×720.

### 4.3 Upstream README images (`photos/`, 11 files)

Screenshots and GIFs of the upstream menu and features (`mash_settings.png`, `defensive_settings.png`, `save_states.png`, `misc_settings.png`, `mash_triggers.png`, `damage_range.png`, `character_item.gif`, `mash_trigger_tumble.gif`, `FLy0xiw.gif`, `ezgif-2-299a3e9b46.gif`, one Switch capture JPG). Upstream-owned; not carried. The clean room records its own GIFs (playbook GIFS.md).

### 4.4 Fork asset generators (fork's own; carry over as tools, outputs derived from the player's game files)

| Generator | Output | Source of pixels | Notes |
| --- | --- | --- | --- |
| `scripts/roster-grid/` (Rust builder `main.rs`, `menu_pictures.rs`, `stage_panel.rs`, `mips.rs`, `validation.rs`; `reproduce.py`; `make_all_icon.py`; `grid.json`) | the fork's `layout.arc` (7,820,940 B) with a `roster_grid` panel (89 `roster_icon_XX` portraits, 87 badge pairs `roster_p_NN`/`roster_c_NN`, cursor, background, title/name/help/role text) appended under `TrModMenu` in `info_training.bflyt` and `trmod_waiting.bflyt`; plus the stage panel | 89 PNG fighter portraits decoded from the game's own character-select textures (`assets/fighter-XX.png`, digests in `assets/sources.json`); `all-smash-ball.png` | uses the external `nx-layout-toolbox` library; verifies three baseline archive digests and every PNG digest; proves reversal to the original layout byte-for-byte |
| `scripts/stage-grid/` (`decode_owned.py`, `catalog.json`, `assets/sources.json`) | `stage_grid` panel: 16 tiles, cursor, labels, ranks, 115 hidden `stage_icon_NNN` pictures (108×90, aspect-fit into 340×130), `stage_all_icon` | 115 PNGs decoded from the game's `stage_1` textures; names from the game's `ui_stage_db.prc` and `msg_name+us_en.msbt` | the Waiting Room has no image and uses a text tile |
| `scripts/item-icons/` (`assets/icon-00..37.png`, `sources.json`) | 38 item icons (64×64 RGBA) as `item_icon_NN` pictures for the character-item picker | native `item_0` UI icons where they exist, otherwise renders of the item's own model and textures from the game archive (fixed orthographic camera), recorded per icon with arc paths and digests | raw captures and pre-fit renders kept outside the repo |
| `scripts/make_waiting_patch.py` + `scripts/waiting_patch_manifest.json` | `src/static/waiting_ui.patch` (92,398 B, format `TMW1`) | copy ranges from the stock `info_waiting` archive and the fork's Training archive plus 60,435 literal bytes; no complete game archive is embedded | manifest records sha256 of stock, donor, target and patch |

These derive from the game's own data; the clean room keeps them (and their READMEs' pane contracts) as-is. They are not upstream artwork.

## 5. Timing and frame rules

Build time only.

## 6. Edge cases and interactions

- **License:** the upstream repository has no license file; `check.svg` carries a Font Awesome Pro commercial license notice; `m_retnormal.svg` is very likely a game texture. None of §4.1–4.3 may be copied, traced or vectorised-from-raster into the clean room. Redraws must be original drawings from the semantic column.
- **Game-owned bytes:** the stock `info_training` layout files, animations and textures inside the archive are the game's; the clean-room build takes them from the player's own installed copy (the fork's `reproduce.py` verifies the baseline digests and refuses other inputs) and the repository ships only the plugin's own parts, icons and the build recipe — Q2 (what exactly is committed: the full archive as today, or a recipe).
- **Pane-name contract:** renaming icon panes (SPEC-core-03 Q6) changes the archive and the renderer together; keep one table in the model crate.
- **Font glyphs:** the controller glyphs (SPEC-core-03 §4.3) are the game's font, not artwork to redraw.

## 7. Persistence

None.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| Archive on disk | upstream 4,213,268 B; fork 7,820,940 B (+ 92,398 B waiting recipe) | ≤ fork, with icons as small indexed textures (W5) | file size; `nro_size.py` (`.rodata`, deflated) |
| Icon textures | 42 icons rasterized into the atlas (sizes unknown; the SVGs are 80×80 or Font Awesome 448×512 viewBoxes) | one atlas page for all menu icons; sizes chosen by the designer with the 96 px button in mind | builder report |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| No upstream artwork bytes in the repository (SVG, PNG, archive members) | `core07_no_upstream_asset_digests` (compare digests against the upstream list kept outside the repo) | — | `PROVENANCE.md` |
| Every submenu id has an icon pane and a texture in the built archive | `core07_icon_coverage` (builder report) | `core07-E1` submenu pages screenshot | `docs/EDEN-RESULTS.md` |
| Stock members byte-identical to the player's archive; reversal proof | builder validation (fork tool) | — | builder report |
| Pane-name contract (SPEC-core-03 §4.2) satisfied by the archive | `core07_pane_contract` (parse the built BFLYT names) | `core07-E1` | — |
| Fork generators reproduce from recorded digests | fork `reproduce.py` checks | — | `scripts/*/assets/sources.json` |

## 10. Open questions

1. **Icons for every row.** Upstream left about 23 rows without an icon. Recommendation: the designer draws a full set (fewer icons per set if the clean-room menu regroups rows; `docs/menu-redesign.md`).
2. **What is committed.** Shipping the built archive commits game-owned bytes (as upstream and the fork do). Recommendation: commit the plugin's own parts, icons and the recipe; build the archive from the player's stock files in CI/locally, like the fork's waiting-room recipe already does.
3. **`quick_menu.svg` and `m_retnormal.svg` usage** could not be confirmed from code (no pane named after them); the archive's texture names were not extracted. Resolve when the archive is rebuilt from scratch (they will simply not exist).
4. **Check mark and digits.** Replace the Font Awesome check with an original glyph; the digits `1`–`5` are text rendered as pictures upstream — the clean room could use the game font in a text pane instead of pictures (fewer textures).
