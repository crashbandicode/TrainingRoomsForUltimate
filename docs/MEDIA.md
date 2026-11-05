# Media

Every GIF in [`media/`](media/) is actual Eden footage with game-speed timing kept. Up to rc.3: whole screen, 1280×720,
one continuous take trimmed only at the ends, under 10 s. From rc.4: 640 wide, at most 15 fps, the action only (cuts
and crops are listed with each clip), under 10 MB. Frames were reviewed (start, action, result) before a clip was kept. The
raw takes stay outside git with their runs.

## 0.2.0-rc.4 clips (2026-11-05)

Run `tr-p23-rc4-docs-1` on the Yoga, 2026-11-05 18:22–18:51Z: the RELEASE NRO of `p23/rc4-candidate` `ff69e53`
(`031f1eaecbd3d0d0284616baf10b8e8c955623b8821ce42976266c1dd10b2d9d`), with the CPU pilot, Online Deluxe and tile-cursor
plugins installed; Eden muted. P1 Mario and CPU Mario on Final Destination, Hitbox Visualization On, Input Display Frame
strip. These clips follow the newer clip rule: 640 wide, at most 15 fps, the action only, under 10 MB, reviewed frame by
frame. They come from Eden's own recorder, cut and given one shared palette (no dither) after review. Facts in the
captions come from guest tracepoints and the profile's files, not from a diag build.

| File | What it shows |
| --- | --- |
| `combo-helper.gif` | The **combo helper** bar under the frame strip (the top of the screen, cropped). The target is *Combo 2*: P1's own three up tilts, saved with *Save last combo* from a save state. Earlier in the run the CPU pilot drove P1 and its combo was saved as *Combo 1*, but the pilot seldom repeated that one-link combo, so the attempts are P1's. Each attempt starts from the same save state. First the reference presses again: link 1 is graded **on time** (1 frame early, inside the 1-frame tolerance). Then the late version: link 1 **2f early**, link 2 **14f late**. Then the early one: link 1 **5f early**. The verdicts read DROP because the reference itself doesn't link at this damage (saved as DROP +5 / +2); the helper grades the press timing against the reference. Offsets from `tr_combo_link_v1`: −1, −2 and +14, −5. 640×190, every second frame (7.5 fps), 18.0 s, 7.7 MB. |
| `mii-editor.gif` | The **Mii editor**. Fighters & Stages › Mii Brawler opens the Player list (*1. 格斗 [1111]*, *Add New…*), and A on *Add New…* opens the game's own Mii Fighter Settings, seeded from the saved Mii. Neutral Special › *2 Flashing Mach Punch*, OK, Save. Training comes back. The list again shows the new Mii, *2. 格斗 [2111]*, and A takes it as the Player choice (it applies when P1 next becomes a Mii Brawler). The game gave the new Mii id 13345, and exactly one file, `mii_13345.bin`, was added; every other Mii file stayed byte-identical (tracepoints on the editor's decision and release; the profile's save folder). Cut: the load before the editor and the loading screen after Save. Training returns with the boot's character-select fighters, not the menu's (seen here). 640×360, 15 fps, 27.0 s, 3.3 MB. |

Not in these clips:
- **Exit without saving:** not shown; the rc.4 session `tr-p23-rc4-2` row mii-03 proved that it writes nothing.
- **An interrupted editor:** that needs a Quickplay or Local Wireless match starting while you edit, which Training
  can't show.

## 0.2.0-rc.3 clips (2026-11-05)

### Frame strip and own hitbox discs

Run `tr-p23-rc3-1` on the Surface, 2026-11-05 15:10–16:01Z: the RELEASE NRO of `p23/rc3-candidate` `3c8f5bc`
(`bf01353ea59e8e06d38975ed13aa686fd25e0c0617d5a66517152fcc292b8527`), with the CPU pilot, Online Deluxe and tile-cursor
plugins installed. P1 Ridley and CPU Ridley (picked in Fighters & Stages), Hitbox Visualization On, Input Display Frame
strip. Eden's raw takes (17–22 MB) were re-encoded with the playbook's `tools/eden-solo/encode.py` (one shared palette,
96 colours, no dither); *step 2* keeps every second frame, so the clip plays about 10–20% slower than the game.

| File | What it shows |
| --- | --- |
| `frame-strip.gif` | The frame strip (Display & Controls › Input Display): P1 jabs into the CPU and follows up; the strip shows the inputs, P1's and the CPU's frames, the link verdicts (`TRUE -5`, `TRUE -6`, `Jab > Jab` in the LAST row) and the advantage. 9.7 s, step 2. |
| `own-orbs-two-ridleys.gif` | Hitbox Visualization's own discs on every fighter sphere (55 per Ridley a frame, measured by a guest tracepoint): P1 forward-smashes, walks and forward-smashes again. 8.5 s, step 2. |

### Menu navigation

Run `tr-p23-menu-nav-02` on the Yoga, 2026-11-05 ~14:09Z: the `p23/menu-nav` diag build at `7ab1c50` (NRO
`1273c29f60503f76f02c1610cf291f2a0997db1b686ba1062c93204ab5812a1d`; the diag build adds log lines only, the menu draws
the same), with the CPU pilot, Online Deluxe and tile-cursor plugins installed.

| File | What it shows |
| --- | --- |
| `menu-section-jump.gif` | CPU page from Reactions: ZL + Down jumps Defence, Ledge & Tech, Shield, Attacks & Movement and Player 1 control (where *Reset page to defaults* closes the page), then ZL + Up goes back two sections. The footer lists ZL+Up/Down section and no ZR page reset. 9.0 s. |

Since rc.3, ZR on a page no longer resets it (the `menu-defaults.gif` clip below shows the old key): each page
ends with *Reset page to defaults*, which asks for confirmation.

### Not captured yet

| Feature | Clip | Where it is captured |
| --- | --- | --- |
| Change Stage | `change-stage.gif` | the rc.3 follow-up boot (the combined session's take ended before the new stage was in) |

`hitbox-overlay.gif` below shows rc.1's effect spheres; rc.3 draws fighter spheres as discs.

## 0.2.0-rc.1 clips (2026-11-05)

Run `tr-p23-docs-capture-03` on the Surface, 2026-11-05Z: one boot of the released 0.2.0-rc.1 NRO
(`706c993ff885273206eee656cce1313ec8690b29176b3ebf66ae38ec28c64a46`), with the CPU pilot, Online Deluxe and tile-cursor
plugins installed. A boot settings file set Hitbox Visualization On, Respawn invincibility On, DI Up, SDI Up/High, and
Kirby first in both cycle orders; the rest was changed in the menu on camera. P1 is Steve and the CPU Richter until
the cycle chords switch both to Kirby. Clips marked *step 2* keep every second frame (each shown twice as long) to
stay under 10 MB.

| File | What it shows |
| --- | --- |
| `menu-tour.gif` | B + D-pad Up opens the menu on CPU; R steps Saves, Recording, Fighters & Stages, Display & Controls; A turns Hitbox Visualization Off. 9.5 s. |
| `hitbox-overlay.gif` | Hitbox Visualization On: the yellow hurtbox shapes on both fighters; P1's forward smash lands (CPU to 20.5%). 6.0 s. |
| `di-up.gif`, `di-none.gif` | The same two forward smashes from the same damage (36% and 51%) with CPU DI Up and SDI Up/High, then with both reset: the DI Up launches go higher. 9.5 s each, step 2. |
| `di-sdi-chips.gif` | CPU page: the DI Direction grid (the game's DI line on each chip, Up ×1 picked) and the SDI Direction grid (the game's SDI streaks). 8.0 s. |
| `respawn-on.gif` | Respawn invincibility On: P1 walks off, is KO'd and respawns on the game's platform at the top. 6.9 s, step 2. |
| `respawn-off.gif` | Off (changed on the Saves page): the same KO, and P1 drops back in at once. 5.6 s, step 2. |
| `cpu-kirby-hat.gif` | P1's Special + R3 gives the CPU Kirby a hat (Mario's), with the "CPU Kirby Hats" notice. 4.2 s. |
| `p1-kirby-hat.gif` | P1's own Special + L3 hat chord, with the "Kirby Hats" notice. 4.1 s. |
| `cycle-badges.gif` | Fighters & Stages › Character Cycles: the game's stock icons with the order badges (red for Player 1, blue for the CPU); ZR switches role. 9.5 s. |
| `replay-capture.gif` | Recording page: A turns *Replay capture* On; the menu closes; L + R + Down opens the Online Deluxe overlay, whose Player Info page shows *Training Rooms › Replay capture: On*. 9.5 s. |
| `menu-shortcut.gif` | Display & Controls › Menu shortcut 1: X captures ZL + D-pad Up in place of Minus, A confirms, the menu closes, and the new chord opens it at once. 9.5 s. |
| `record-refused.gif` | With the CPU pilot driving P1 (armed on the Deluxe overlay), ZR + D-pad Down shows "P1 is controlled by an AI: recording is off". 4.7 s. |
| `waiting-menu.gif` | A Local Wireless room hosted in Eden, Solo waiting practice: B + D-pad Up opens the menu on its last page (Display & Controls); R steps to CPU and Saves. 8.0 s. |

Not captured in this session: a visible grab box (the 10%-speed take shows a forward smash into the CPU's shield, not a
grab), the air dodge Out / shield tilt In clips (at 0% the forward smash barely lifts the CPU, so neither shows),
and a multi-hit SDI clip.

## Menu clips from RC10 (2026-11-04)

Recorded with Eden-GIF on RC10 release boots on the Surface (runs `glm-gifs-04`, `glm-gifs-05`, `glm-mods-10`,
`glm-zr-03`), ARCropolis `92171b2f…` pinned with the profile's Cosmic Destination stage mod live.

| File | What it shows |
| --- | --- |
| `menu-tabs.gif` | B + Up opens the menu, R steps the five pages, B closes it. 7.9 s, NRO `50d95130`. |
| `menu-cpu.gif` | CPU page: DI Direction's grid, Right + A picks Out ×1. 9.8 s, NRO `50d95130`. |
| `menu-fighters.gif` | Character Cycles with the game's stock icons; ZR switches role three times and the icons stay on screen. 10.0 s, re-encoded to 640×360 at the time; release NRO `73e5ad24`. |
| `menu-stages.gif` | Lobby Stages grid: thumbnails loaded through the game's mod-aware loader (the Final Destination tile shows the Cosmic Destination mod); A adds Battlefield (rank 2). 9.7 s, NRO `50d95130`. |
| `menu-recording.gif` | ZR + Down records P1, ZR + Up plays it back under the "Input Recording — Playback" notice. 9.9 s, ≈3.3 fps. NRO `50d95130`. |
| `menu-defaults.gif` | The X / Y / ZR page keys on Display & Controls (values already at their defaults, so none moves). 9.8 s, NRO `50d95130`. |

The character grids draw the stock-icon set `chara_2` at the default costume, loaded at run time through the game's
file-resource service (operator decision 2026-11-03, amended 2026-11-04). A ZR role switch keeps every texture the new
page still uses (`model::ui_texture::carry`), so no icon leaves the screen.

The earlier waiting-menu clip (`waiting-menu-r16.gif`, development build `758cee5`, 2026-11-03) is replaced by the
rc.1 clip above; it and its provenance note remain in the history up to v0.2.0-rc.1.
