# Assets inventory (upstream artwork to replace; fork asset generators) — behavioural specification

- **Spec id:** `SPEC-core-07`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required (**artwork must be redrawn or replaced, never copied**)
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), spec-writer role, 2026-11-01
- **Upstream reference read (spec writer only):** `src/static/img/*.svg` (42 files; metadata and a few small files read in full, the rest by metadata search), `src/static/layout.arc` (member names and `TrMod*` pane names extracted by string search of the binary; no archive was unpacked), `src/training/ui/{menu,mod,display,input_log}.rs` (pane names referenced), `photos/*` (README images, by name only) at commit `97cbe9b9` (unchanged in `077a8e43`); fork: `src/static/layout.arc` (size only), `src/static/waiting_ui.patch`, `scripts/make_waiting_patch.py`, `scripts/waiting_patch_manifest.json`, `scripts/roster-grid/README.md`, `scripts/stage-grid/README.md`, `scripts/item-icons/assets/sources.json` (head), from `docs/provenance/provenance-files.md`.
- **Revision 2026-11-01 (menu proposal v5; Claude agent, Opus 5.5, menu-draft worker):** §4.5 added. The stage grid reads its thumbnails at run time from the game's own stage UI textures (MENU-DRAFT.md question 13, answered). Fork sources read for it: `src/stage_asset_probe.rs`, `src/training/ui/stage_grid.rs`, `scripts/stage-grid/{README.md,decode_owned.py,catalog.json,assets/sources.json}`. Also read: the fork campaign's ARCropolis 4.1.1 API audit `artifacts/enhancements/arc-stage-assets/REPORT.md` and the probe log `artifacts/enhancements/plan4-native-08/practice-diagnostics.log`, both under `/home/intpa/smash-training-lobby/`. All are fork-owned; no upstream file was opened.
- **Revision 2026-11-01 (menu proposal v8; Claude agent, Opus 5.5, menu-draft worker):** §4.5 U5 answered and §4.6 rewritten from two evidence sets of the fork campaign, cited in place and not copied: `C1162 = /home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/c1162`, `C1162/di-sdi-trace/RESULT.md` (static trace of game 13.0.5; its `re/` files are game-derived and stay outside the repository) and `C1162/captures-ally/RESULT.md` (Eden captures on the Ally host, fork M5l). FACT / INFERENCE below follow the trace's own marking.
- **Implementers must not open the upstream paths above.** Everything an implementer and a designer need is in this file and in SPEC-core-03/05.

## 1. Purpose (one paragraph)

This spec lists every piece of upstream artwork and layout data the fork inherited, says where each is used, and states that none of it may be carried into the clean room: the upstream repository publishes no license, one icon is Font Awesome Pro (commercial license), one is an Adobe Illustrator export with a Japanese layer name that is probably a game texture, and the layout archive embeds the game's own stock files. A designer redraws the icons from the semantic list below; the layout parts are rebuilt from the player's own copy of the game's stock archive with the fork's builder, which already derives all of its own portrait, stage and item images from the game's files at build time.

## 2. User-visible behaviour

- Each submenu button on the menu's submenu page shows an icon that illustrates the setting (SPEC-core-03 §2.2); toggle pages show a check mark or a weight digit `1`–`5`.
- The notification panel, the input-log rows and the slider panel are plugin panes with no artwork of their own beyond backgrounds and text (they clone stock game materials).
- Replacing artwork changes nothing functionally; the pane names and the mapping from submenu id to icon are the contract (SPEC-core-03 §4.2).

## 3. Inputs

At run time: the archive (SPEC-core-05), the game's per-stage UI textures for the stage grid (§4.5) and the DI / SDI effect data in `ef_common.eff` for the DI / SDI chips (§4.6), both read through the mod-aware file view. At build time: the player's own stock `info_training` layout archive (and, for the fork's waiting room, the stock `info_waiting` archive), the redrawn icons, the fork's generated portraits/stage/item images.

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
| `scripts/stage-grid/` (`decode_owned.py`, `catalog.json`, `assets/sources.json`) | `stage_grid` panel: 16 tiles, cursor, labels, ranks, 115 hidden `stage_icon_NNN` pictures (108×90, aspect-fit into 340×130), `stage_all_icon` | 115 PNGs decoded from the game's `stage_1` textures; names from the game's `ui_stage_db.prc` and `msg_name+us_en.msbt` | the Waiting Room has no image and uses a text tile. **Clean room (2026-11-01): the tile, label, rank, cursor and pane contract carry over; the 115 baked `stage_icon_NNN` pictures and `decode_owned.py` do not. Thumbnails are read at run time (§4.5).** `catalog.json` (ids, `name_id`, patch flag, names; no pixels) carries over as the stage list |
| `scripts/item-icons/` (`assets/icon-00..37.png`, `sources.json`) | 38 item icons (64×64 RGBA) as `item_icon_NN` pictures for the character-item picker | native `item_0` UI icons where they exist, otherwise renders of the item's own model and textures from the game archive (fixed orthographic camera), recorded per icon with arc paths and digests | raw captures and pre-fit renders kept outside the repo |
| `scripts/make_waiting_patch.py` + `scripts/waiting_patch_manifest.json` | `src/static/waiting_ui.patch` (92,398 B, format `TMW1`) | copy ranges from the stock `info_waiting` archive and the fork's Training archive plus 60,435 literal bytes; no complete game archive is embedded | manifest records sha256 of stock, donor, target and patch |

These derive from the game's own data; the clean room keeps them (and their READMEs' pane contracts) as-is, except the baked stage pictures (§4.5). They are not upstream artwork. Fighter portraits: **decided 2026-11-01 (menu question 16): they follow §4.5**: each fighter's own UI portrait is read at run time through the mod-aware view, with a text tile as the fallback, so character and costume mods show; nothing is baked or committed. This needs one more development probe: the same unknowns as the stage thumbnails (U1–U4) plus which texture is the right costume's (open question 6).

### 4.5 Stage-grid thumbnails at run time (MENU-DRAFT.md draft 5, question 13)

**Rule.** Every stage tile draws the stage's own UI texture, read from the game's file system when the page is shown. That is the same file a stage mod replaces through ARCropolis, so a mod that changes a stage's icon (e.g. a Final Destination replacement) shows in the grid. No stage picture is committed to the repository or baked into the layout archive. The *All* tile keeps the stock Smash Ball texture already in the stock archive (the fork reuses `roster_5e`).

**What the fork does today (for contrast).** `scripts/stage-grid/decode_owned.py` decoded the stock `ui/{replace,replace_patch}/stage/stage_1/stage_1_<name_id>.bntx` files once. The fork's development probe `src/stage_asset_probe.rs` (cargo feature `stage-asset-probe`, off in release builds) lists exactly these 115 `stage_1` paths. Its README does not name the capture route. The decoded PNGs were baked into `layout.arc` as 115 hidden `stage_icon_NNN` panes (108×90 each), and `src/training/ui/stage_grid.rs` only toggles their visibility and position. The probe reads through ARCropolis's `arc:/` mount. Its `ArcFuse` / `VanillaArc` deliberately serve the vanilla archive. Either way the baked set is fixed at build time, so a stage mod never shows in the fork's grid.

**Which file, per stage.**

| Field | Value | Source |
| --- | --- | --- |
| Path | `ui/replace/stage/stage_1/stage_1_<name_id lower-case>.bntx`; `ui/replace_patch/…` when the stage is a patch stage (12 of 115) | fork `decode_owned.py` (`source_path`), `catalog.json` (`is_patch`) |
| `name_id` | from the game's `ui/param/database/ui_stage_db.prc` (`can_select = true`, sorted by `disp_order`); differs from the native package name for some stages | fork `scripts/stage-grid/README.md`, `catalog.json` |
| Examples | Battlefield `stage_1_battlefield.bntx`; Final Destination `stage_1_end.bntx`; Small Battlefield `replace_patch/…/stage_1_battlefields.bntx`; Hyrule Castle `stage_1_zelda_hyrule.bntx`; Kongo Falls `stage_1_dk_waterfall.bntx` | `catalog.json`, fork `assets/sources.json` |
| Stock size | `stage_1`: one texture, 108×90, 20,584 B per file (Battlefield, Final Destination); `stage_2` and `stage_3`: 135,272 B per file for the same two stages | fork `sources.json`; probe log (`plan4-native-08`) |
| Hash | ARCropolis / the game address files by `hash40` of the archive-relative path (no `arc:/` prefix), e.g. `stage_1_battlefield` `0x31c16bfb09`, `stage_1_end` `0x294cd46fe9` | ARCropolis audit, hash table |

The stage list (native id → `name_id`, patch flag, display name) is data, not pictures. It carries over from the fork's `catalog.json`.

**How it is resolved so mod replacements apply.** The read has to see ARCropolis's replacement view. The audit (source-established for ARCropolis 4.1.1, API 1.9; not yet proven at run time on this profile) names three candidates:

1. **`arcrop_load_file(hash40, buffer, length, &out_size)`** after `arcrop_get_decompressed_size`. It loads through `GlobalFilesystem::load_into` and `ModFs::read_into`, so a mod's file wins. Precondition: the export skips ARCropolis's global lock because it expects to run inside an API callback that already holds it. Calling it from an arbitrary thread is not an established contract (unknown U1).
2. **The `mods:/` mount.** ARCropolis mounts it after mod processing and then raises `ModFilesystemMounted` (event 1). Whether `mods:/<path>` serves only mod files or the merged view is not established (U2). If it serves only mod files: try `mods:/<path>` first, then fall back to the stock file.
3. **The game's own resource loader**, which ARCropolis already redirects. This is the path the game's own stage-select screen uses, so it sees exactly what the player sees. The 13.0.5 entry point and its thread and lock rules are not identified yet (U3).

Never `arc:/` for these thumbnails: it is the vanilla view. Resolve the ARCropolis exports optionally (`skyline::nn::ro::LookupSymbol`, require API major 1, minor ≥ 9), register the event callback in the plugin's early `main` (events are not replayed for late subscribers), and read nothing before `ModFilesystemMounted`. Without ARCropolis no ARCropolis mod can be active. The tile then shows the stock texture through route 3, or the text tile.

**Drawing.** (Draft 9 paging, decided 2026-11-02: the stage grid shows 5 × 8 = 40 tiles per page when docked and 4 × 4 = 16 when handheld, SPEC-core-03 §2.5; the layout therefore needs up to 40 picture panes, and the figures "16" below read "up to 40 docked, 16 handheld"; the read cost for 40 files is part of U7.) The layout carries 16 picture panes, one per tile, with no stage texture. On page open the plugin reads at most 16 files and decodes or binds each BNTX to its tile's pane, aspect-fit into the fork's 340×130 picture box (native 1920×1080 canvas). It releases them when the page or grid closes. How a run-time BNTX is bound to an `nn::ui2d` picture pane (texture-info creation, GPU memory pool, alignment) is unknown U4. A CPU decode to RGBA plus an upload is the fallback design. A mod's texture may differ in size or format from stock: accept any size up to a cap (proposal: 1 MiB per file, the probe's cap), aspect-fit it, and on a missing file, a refused read, an unexpected magic (`BNTX\0\0\0\0`), an oversize file or a decode failure draw the text tile, as the Waiting Room does. Never substitute a baked picture.

**Unknowns (to resolve before the stage grid is implemented).**

- U1–U4 above.
- U5 **Texture slot — answered for the first data point (2026-11-01).** The operator's Final Destination mod (`Cosmic Destination`, 59 files) replaces all four slots at the stock sizes: `ui/replace/stage/stage_1/stage_1_end.bntx` (20,584 B, sha256 `1b7e291e…`) and `stage_2` / `stage_3` / `stage_4` (135,272 B each, different hashes). With the mod, the game's own stage select shows the mod's picture in both the small tile (`stage_1`) and the large preview (`stage_2` / `stage_3`). The fork's grid FD tile is byte-identical with and without the mod (GIF sha256 `49ee6ad1…` in both): its baked pictures do not follow mods. Reading `stage_1` at run time would show this mod's picture. Evidence: `C1162/captures-ally/RESULT.md` §1, §3; file list and hashes `card-read-receipt.json`, `card-mods-SHA256SUMS.txt`; comparison sheets `stage-icons/cmp-sss-fd-stock-vs-mod.png`, `stage-icons/cmp-modpack-grid-stock-vs-mod.png`. Still open: mods that ship only `stage_2` (then read `stage_2` and scale it down, or try `stage_1` and then `stage_2`). U1–U4 are untouched by this capture.
- U6 **Forms.** Whether Omega and Battlefield forms have their own UI textures; the fork shows the stage's one picture for every form.
- U7 **Timing and cost.** The time to read and decode 16 files on page open on the console, and whether a one-page cache is needed (16 × 20,584 B ≈ 330 KB stock; mods may be larger).
- U8 **Added stages.** Stage-expansion mods that add entries to `ui_stage_db.prc` are out of scope; the list stays the fork's 115 (+ Waiting Room).

### 4.6 DI / SDI direction chips from the game's own visual at run time (MENU-DRAFT.md drafts 6–7, question 17)

**Rule.** The chips of the DI Direction and SDI Direction grids show the visual the game itself draws for a DI / SDI direction, read from the player's game files when the grid opens, through the mod-aware route of §4.5 (U1–U3), so an effect mod shows. Nothing is drawn after it, committed to the repository or baked into the layout archive. If the read fails, the chip shows the clean room's own arrow, never a stored copy of the game's picture. Airdodge Direction and Shield Tilt keep the clean room's arrows (the game's visual belongs to hitlag and launch; MENU-DRAFT.md *Direction chips*). Neutral keeps the ring; Left / Right reuse the In / Out picture with an `abs` mark.

**What is established (2026-11-01).** Static trace of game 13.0.5 (`C1162/di-sdi-trace/RESULT.md`, §1–§5), Eden captures (`C1162/captures-ally/RESULT.md` §2; SDI and DI rotation: `C1162/captures-ally-sdi/RESULT.md` §0–§3) and renders from the effect file (`C1162/di-sdi-extract/RESULT.md`). Evidence is cited, not copied.

| # | Finding | Mark | Source |
| --- | --- | --- | --- |
| V1 | **DI** = system effect `sys_vector` (hash40 `0x0aff1ac5b6`), an emitter set in a particle file, not a UI texture: `effect/system/common/ef_common.eff` (arc hash40 `0x222d35b012`, 33,453,584 B), EFFN `SYS_VECTOR` → emitter set **`P_CmnVector`** (5 emitters). Requested by `lua2cpp_common`'s `FighterStatusDamage__requestVectorAdjustEffect`, called from the damage-fly statuses | FACT | trace §1–§2 |
| V1 | **SDI** = a different effect: `sys_hitstop_spark` (`0x11c6e92484`) → **`P_CmnHitStopSpark`** (2 emitters, same file), requested on joint `head` on each SDI input frame (`…check_hit_stop_delay_flick`) | FACT | trace §1, §3.2 |
| V1 | `sys_direction` / `sys_direction2` are fighter-specific (requested only from two fighter specializer areas), not DI / SDI | FACT | trace §3.4 |
| V2 | One effect, rotated about Z: DI to the **DI-corrected launch angle** atan2(vy, vx), then following it at ≤ 5.5° per frame; SDI to the **SDI vector**. Not a different shape per direction. So a chip is one picture turned to each angle of SPEC-cpu-02 §2 (that the stick angle stands in for the launch angle is INFERENCE) | FACT (rotation); INFERENCE (chip mapping) | trace §3.1–§3.2, §5 |
| V2 | DI spawns only when the launch has speed ≥ 2.0 and DI turns it by ≥ 1° (`vector_adjust_effect_speed` / `_angle`); no Training option or modpack flag gates it | FACT | trace §3.1 (`fighter/common/param/effect.prc`) |
| V3 | Colour is emitter data, not texture data: the textures are single-channel BC4/BC5 masks (plus one BC3 flare); no `set_rgb` / `set_color` call in the DI / SDI code. DI = white core plus blue (colour B ≈ (0, 0.08–0.46, 1)); SDI = all white. The values are FACT; which emitter field means what (+0x9a0 / +0x9b0 constants, +0x3c0 colour keys, +0x440 alpha keys) is INFERENCE | FACT / INFERENCE | trace §3.3 |
| — | Seen in Eden: runs A (DI Up) and C (DI Out) show a blue streak with white sparkle lines at about +5 … +15 frames after contact; the no-DI baseline does not. The white crescent in every run, baseline included, is the attacker's slash, not DI. Run B (SDI Up, High) showed nothing SDI-specific; W8 found why (next rows) | observation | captures §2; frames `C1162/captures-ally/di-sdi/runA/frames/`, `…/runC/frames/` |
| — | **SDI seen in Eden (W8, 2026-11-01):** a **white, jagged, spiky burst of thin streaks with a ring-shaped outline, drawn on the CPU's head and pointing along the SDI vector**. Up = a tall vertical crown of spikes above the head; In (CPU right of the player) = the same shape lying horizontally, spikes pointing left. Entirely white, no tint; spawned on each SDI input and lingering a few frames. SDI Up: from about +4, clear +6 … +17 (offset 0 = first frame of the hit flash); SDI In: +8 … +17; the no-SDI control (Strength None, same hit) shows none. With SDI In the CPU also ends the hitlag clearly displaced toward the player. Best frames `sdi-up/frames/sdiup_f+12`, `f+14`, `sdi-in/frames/sdiin_f+12`, `f+14`; close-up sheet `compare/cmp-sdi-spark-closeup.png` | observation (answers V7) | `C1162/captures-ally-sdi/RESULT.md` §1 |
| — | **Why W5's run B was empty:** the fork rolls the CPU's SDI direction only at a Training reset (`sdi::roll_direction()` ← `reset::on_reset()`), not when the menu changes; run B changed SDI after the match started, so the direction stayed Neutral. A player-visible cost; the clean-room rule is SPEC-cpu-02 open question 9 | FACT (fork source) | `C1162/captures-ally-sdi/RESULT.md` §0 |
| — | **DI rotation confirmed in motion (W8):** with DI Left / Right on a near-vertical up-smash launch, the streak points about straight up (Left) and about 20° right of vertical (Right), against about 35–45° up-right for W5's forward smash; the later launch trails agree. One picture turned to the **DI-corrected launch vector**, not to the stick direction; the DI-induced turn is small (a few to about 20°) | observation (confirms V2 for DI) | `C1162/captures-ally-sdi/RESULT.md` §2; `compare/cmp-di-rotation-vs-w5.png` |
| — | **Renders from the effect file (W9):** the emitters' own textures, meshes and colours, composited as one still (particle life t = 0.3), match both captures in character (SDI: white spiky arch at the head plus a thin line along the vector; DI: blue band with white dashes along the launch). Field meanings are inference and one SDI emitter is placed by calibration | FACT (values) / INFERENCE (fields) | `C1162/di-sdi-extract/RESULT.md` §3–§6 |
| — | Neither the fork nor upstream draws, requests or hides either effect; DI writes the stick floats that the game's `correctDamageVectorCommon` reads, so the fork's DI drives both the launch and this visual | FACT | fork M5i `src/training/directional_influence.rs`, `sdi.rs`; trace §3.1 |

**What the clean room does at run time to draw a chip.**

1. After ARCropolis's `ModFilesystemMounted` (§4.5), read the player's own `effect/system/common/ef_common.eff` through the mod-aware view (never `arc:/`), so an effect mod's version shows.
2. Find the emitter set (`P_CmnVector` for DI, `P_CmnHitStopSpark` for SDI) through the EFFN name table, then its emitters' texture ids and colour fields; locate those textures in the file's BNTX.
3. Draw one still per visual: the emitters' greyscale masks tinted with their emitter colours (DI: the streak masks tinted blue / white plus the flare; SDI: the spark and impact ring, white), composited once.
4. Turn that one still to each chip's direction angle (SPEC-cpu-02 §2, with the In / Out mirroring the menu displays). Never bake the result; release it when the grid closes.
5. Any failure (file, name, texture, colour or decode) → the clean room's arrow.

A chip is a faithful still made from the game's textures and colours; it does not reproduce the particle animation (scrolling speckles, colour and alpha over time).

**Streaks (N10, decided 2026-11-01).** The SDI chips draw the spark's white streaks longer than the first renders, closer to the captured spark: **twice as long (×2.0, confirmed 2026-11-02)**; the DI chips are unchanged. This is a drawing-size rule for the still of step 3 (the review screens 01, 16 and 17 show it); the data source is unchanged.

**Unknowns (to resolve before the chips are implemented).**

- V4 **Read and parse at run time.** The read route and pane binding are §4.5 U1–U4. New here: parsing the `.eff` at run time — EFFN table, emitter set chain, the emitter texture-id and colour fields (layout inferred, no format spec), the GTNT / BNTX texture index — and decoding BC4 / BC5 / BC3. Byte offsets found in the 13.0.5 file are in the trace §5; a modded file may differ, so the parser looks names up rather than using fixed offsets.
- V5 **Cost.** The file is 33,453,584 B: too large to read per grid open. Use ranged reads (DI textures 29,696 B at mip 0, SDI 53,248 B, plus about 16 KB of headers each) or a resident copy (V6); one still per visual, shared by all chips.
- V6 **Resident copy.** Whether the game keeps `ef_common.eff`'s textures resident on the GPU, so the menu could bind them without a file read.
- V7 **SDI capture — answered 2026-11-01 (W8).** The spark is captured for SDI Up and SDI In, absent in the no-SDI control (table above). Not captured: a third direction, a numeric position track.

**Review mockup only.** Since proposal v9, `docs/menu-mockup/` draws the DI and SDI chips from W9's game-derived renders (one Up still per effect, byte copies, turned per direction as the game turns the effect), and the chip sheet shows crops of the captures beside them as the visual check (DI: run A frame +13; SDI: W8 `sdiup_f+12` and `sdiin_f+12`). Sources, sha256 and crop boxes are in `docs/menu-mockup/index.md`. All of it is review material and never ships; no `.eff`, texture, mesh or parameter file is in the repository.


## 5. Timing and frame rules

Build time, except the stage-grid thumbnails (§4.5): no read before ARCropolis's `ModFilesystemMounted` event; at most 16 reads when a stage page opens, off the frame hook if U1–U3 allow; nothing kept after the grid closes (unless U7 calls for a one-page cache).

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
| Stage thumbnails | 115 baked 108×90 pictures in the fork's `layout.arc` (share of its 7,820,940 B not measured) | none in the archive; ≤ 16 files read per open page (stock `stage_1` 20,584 B each, ≈ 330 KB), released on close | builder report (archive without `stage_icon_NNN`); read time and heap use logged in `diag` (U7) |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| No upstream artwork bytes in the repository (SVG, PNG, archive members) | `core07_no_upstream_asset_digests` (compare digests against the upstream list kept outside the repo) | — | `PROVENANCE.md` |
| Every submenu id has an icon pane and a texture in the built archive | `core07_icon_coverage` (builder report) | `core07-E1` submenu pages screenshot | `docs/EDEN-RESULTS.md` |
| Stock members byte-identical to the player's archive; reversal proof | builder validation (fork tool) | — | builder report |
| Pane-name contract (SPEC-core-03 §4.2) satisfied by the archive | `core07_pane_contract` (parse the built BFLYT names) | `core07-E1` | — |
| Fork generators reproduce from recorded digests | fork `reproduce.py` checks | — | `scripts/*/assets/sources.json` |
| Stage grid draws the run-time texture, mods apply, no baked stage picture (§4.5) | `core07_no_stage_pictures` (no `stage_icon_NNN` texture in the built archive; no stage PNG in the repo); `core07_stage_paths` (every catalog row maps to its `replace` / `replace_patch` path) | `core07-E2`: the stage grid on a profile with a stage mod that replaces a stage's UI texture (e.g. the Final Destination mod) shows the mod's picture on that tile and stock pictures elsewhere; `core07-E3`: the same without the mod shows stock; `core07-E4`: a missing or refused file shows the text tile | `docs/EDEN-RESULTS.md` |
| **N5** Stage grid page reads: at most 40 stage files when docked (5 × 8) and 16 when handheld (4 × 4), released when the page closes | `core07_stage_page_read_budget` | `core07-E10` diag read count per page in both layouts | `docs/EDEN-RESULTS.md` |
| DI / SDI chips draw the game's own visual at run time, nothing baked, arrow fallback (§4.6) | `core07_no_di_sdi_pictures` (no DI / SDI picture in the repo or the built archive) | `core07-E5`: the DI and SDI grids show the game's visual in each direction; `core07-E6`: an unreadable asset shows the clean room's arrows | `docs/EDEN-RESULTS.md` |
| **N10** SDI chips draw the spark's white streaks ×2.0 longer than the first renders (closer to the capture); DI chips unchanged | `core07_sdi_chip_streak_length` (the chip still's streak length is above the first-render value and within a bound of the captured spark's, measured on the host render of the stock effect file kept outside the repo) | `core07-E7`: the SDI grid beside a capture crop (`sdiup_f+12`); the chip's streaks are visibly longer than the draft-8 chip | `docs/EDEN-RESULTS.md` |
| **Q16** (decided) Fighter portraits are read at run time, text-tile fallback, nothing baked or committed. **Amended 2026-11-03 (operator, RC10):** the roster grids draw the game's own *stock icon* instead of the `chara_1` portrait (`picture=1`); still read at run time through the mod-aware loader, nothing baked or committed. **Amended again 2026-11-04 (operator):** the stock-icon set is the native key builder's `picture=2` (`chara_2`, the 64×64 flat head icon — the art the fork's menu showed; `picture=0` is the 128×128 `chara_0` portrait crop, not the stock icon) | `core07_no_portrait_pictures` (no portrait PNG in the repo or the built archive; the fork's 89 stock icons are not carried over) | `core07-E8`: the portrait grid shows a character / costume mod's stock icon; `core07-E9`: an unreadable file shows a text tile | `docs/EDEN-RESULTS.md` |

## 10. Open questions

1. **Icons for every row.** Upstream left about 23 rows without an icon. Recommendation: the designer draws a full set (fewer icons per set if the clean-room menu regroups rows; `docs/menu-redesign.md`).
2. **What is committed.** Shipping the built archive commits game-owned bytes (as upstream and the fork do). Recommendation: commit the plugin's own parts, icons and the recipe; build the archive from the player's stock files in CI/locally, like the fork's waiting-room recipe already does.
3. **`quick_menu.svg` and `m_retnormal.svg` usage** could not be confirmed from code (no pane named after them); the archive's texture names were not extracted. Resolve when the archive is rebuilt from scratch (they will simply not exist).
4. **Check mark and digits.** Replace the Font Awesome check with an original glyph; the digits `1`–`5` are text rendered as pictures upstream — the clean room could use the game font in a text pane instead of pictures (fewer textures).
5. **Stage thumbnails at run time (§4.5).** Decided: read at run time, never baked. Open: U1–U8 (load route and its thread/lock rules, `mods:/` semantics, the game-loader entry point, binding a run-time BNTX to a picture pane, the texture slot mods replace, forms, cost, added stages). U5 is answered for the operator's Final Destination mod (all four slots replaced, §4.5). Recommendation: resolve U1–U3 with one bounded development probe that reads one stock and one modded stage through each candidate route and logs size and sha256.
6. **Fighter portraits** (MENU-DRAFT.md question 16): whether the portrait grid follows the same rule so character and costume mods show. **Decided 2026-11-01: as recommended;** read at run time, text-tile fallback (§4.4). Open: the portrait file per fighter and costume, and the §4.5 unknowns for it, to be resolved by one more development probe; the fork's 89 committed stock icons (`scripts/roster-grid/assets/`) are then not carried over. **Amended 2026-11-03 (operator, RC10):** the roster grids show the game's own stock icon instead of the `chara_1` portrait, still read at run time and never baked; the fork's icon files stay uncarried. **Amended again 2026-11-04 (operator):** the stock-icon set is key-builder `picture=2` (`chara_2`, the 64×64 flat head icon the fork's menu showed — `picture=0` is the `chara_0` portrait crop, not the stock icon).
7. **DI / SDI chips (§4.6).** Decided: the game's own visual, read at run time, never baked; DI and SDI only. **Question 17 decided 2026-11-01: as recommended (DI and SDI only), with the SDI streaks drawn longer (N10, §4.6).** Answered: V1 (DI `sys_vector` / `P_CmnVector`, SDI `sys_hitstop_spark` / `P_CmnHitStopSpark`, two visuals), V2 (one effect rotated; for DI confirmed in motion by W8), V3 (emitter colours over greyscale masks), V7 (SDI captured, W8). Open: V4–V6. Recommendation: V4–V6 together with §4.5 U1–U4 in one bounded development probe; W9's compositing rule (`C1162/di-sdi-extract/RESULT.md` §4) is the starting point for step 3.
