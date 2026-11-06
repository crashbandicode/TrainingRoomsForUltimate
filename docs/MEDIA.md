# Media

Every GIF in [`media/`](media/) is actual Eden footage: whole screen, 1280×720, one clip per behaviour, under 10 s,
game-speed timing kept, trimmed at the ends (any cut inside a clip is listed with it). Frames were reviewed (start,
action, result) before a clip was kept. The raw takes stay outside git with their runs.

## 0.2.0-rc.4 clips (2026-11-05)

Run `tr-p23-rc4-docs-2` on the Yoga, 2026-11-05 19:32–20:05Z: the RELEASE NRO of `p23/rc4-candidate` `ff69e53`
(`031f1eaecbd3d0d0284616baf10b8e8c955623b8821ce42976266c1dd10b2d9d`), with the CPU pilot, Online Deluxe and tile-cursor
plugins installed; Eden muted. P1 Mario and CPU Mario, Hitbox Visualization Off, Input Display Frame strip. Each clip is
one continuous take from Eden's own recorder at 1280, re-encoded with the playbook's `tools/eden-solo/encode.py` (one
shared palette, no dither, fps 0: Eden's own frame times). Facts in the captions come from guest tracepoints and the
profile's files, not from a diag build.

| File | What it shows |
| --- | --- |
| `combo-helper.gif` | The **combo helper** bar under the frame strip, on Training's stage. The target is *Combo 1*: P1's up tilt into up tilt, saved with *Save last combo*, a **true** link (TRUE −4) at the CPU's 42% (every save-state load sets it). Two attempts from the same save state: the same presses, graded **on time** ("clean, 1 of 1"); then the follow-up 3 frames sooner, **Attack 3f early**, still true ("1 of 1 true"). Offsets from `tr_combo_link_v1`: 0 and −3. 1280×720, every second frame kept with its time (7.5 fps), 9.4 s, 12.9 MB: the camera follows the fighters, so the whole frame moves and compresses less than the playbook's 3–5 MB note. |
| `mii-editor.gif` | The **Mii editor**: in Fighters & Stages › Mii Brawler (*1. 格斗 [1111]*, *2. 格斗 [2111]*, *Add New…*), A on *Add New…* opens the game's own Mii Fighter Settings, seeded from the role's Player choice; Special Moves › Neutral Special › *3 Exploding Side Kick*, OK, Save. The game gave the new Mii id 13349, and exactly one file, `mii_13349.bin`, was added; every other Mii file stayed byte-identical (tracepoints on the editor's decision and release; the profile's save folder). 1280×720, 9.3 s, 3.9 MB. |
| `mii-editor-list.gif` | Mii Brawler's list after a save earlier in the run (id 13345): Down to the new Mii, *2. 格斗 [2111]*, and A takes it as the Player choice (the marker moves to it; it applies when P1 next becomes a Mii Brawler). 1280×720, 4.5 s, 41 KB. |

After the editor saves, Training comes back with the boot's character-select fighters, not the menu's (reported to the Mii
lane).

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

### Change Stage

Run `tr-p23-rc3-fu2-2` on the Surface, 2026-11-05 ~17:30Z: the RELEASE NRO of `p23/rc3-candidate` `f0eda16` (with
the rc.3 review fixes; `e73d5af842c04430f73ab09b8b3dd74371f01adec194ade76e568b7ac357d2de`), the same plugins and fighters
as above. Re-encoded the same way (step 2, 96 colours), each frame then given 160 ms so the clip plays in 9.3 s for the
9.5 s taken.

| File | What it shows |
| --- | --- |
| `change-stage.gif` | Fighters & Stages › Change Stage: the stage grid with Battlefield picked; Y gives its Ω form and A closes the menu ("Stage requested" on the old stage); Training reloads on Battlefield Ω (the game's stage id went from `0x15b` to `0x1`, read by a guest tracepoint). 9.3 s, step 2. |

`hitbox-overlay.gif` below shows rc.1's effect spheres; rc.3 draws fighter spheres as discs.

## 0.2.0-rc.1 clips (2026-11-05)

Run `tr-p23-docs-capture-03` on the Surface, 2026-11-05Z: one boot of the released 0.2.0-rc.1 NRO
(`706c993ff885273206eee656cce1313ec8690b29176b3ebf66ae38ec28c64a46`), with the CPU pilot, Online Deluxe and tile-cursor
plugins installed. A boot settings file set Hitbox Visualization On, Respawn on platform On, DI Up, SDI Up/High, and
Kirby first in both cycle orders; the rest was changed in the menu on camera. P1 is Steve and the CPU Richter until
the cycle chords switch both to Kirby. Clips marked *step 2* keep every second frame (each shown twice as long) to
stay under 10 MB.

| File | What it shows |
| --- | --- |
| `menu-tour.gif` | B + D-pad Up opens the menu on CPU; R steps Saves, Recording, Fighters & Stages, Display & Controls; A turns Hitbox Visualization Off. 9.5 s. |
| `hitbox-overlay.gif` | Hitbox Visualization On: the yellow hurtbox shapes on both fighters; P1's forward smash lands (CPU to 20.5%). 6.0 s. |
| `di-up.gif`, `di-none.gif` | The same two forward smashes from the same damage (36% and 51%) with CPU DI Up and SDI Up/High, then with both reset: the DI Up launches go higher. 9.5 s each, step 2. |
| `di-sdi-chips.gif` | CPU page: the DI Direction grid (the game's DI line on each chip, Up ×1 picked) and the SDI Direction grid (the game's SDI streaks). 8.0 s. |
| `respawn-on.gif` | Respawn on platform On: P1 walks off, is KO'd and respawns on the game's platform at the top. 6.9 s, step 2. |
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
