# Media

Every GIF in [`media/`](media/) is actual Eden footage: whole screen, 1280×720, one clip per behaviour, under 10 s,
game-speed timing kept, trimmed at the ends (any cut inside a clip is listed with it). Frames were reviewed (start,
action, result) before a clip was kept. The raw takes stay outside git with their runs.

## rc.5 captures still needed

The player docs already point at these paths. None of these files exist yet. Capture each one in the Eden session
that boots the build named on the row. Do not keep a take where the "Must be visible" line is missing. Full screen,
1280×720, one behaviour, game speed, under 10 s. Trim only the ends.

Shared start, unless a row says otherwise:

- Training mode, Battlefield, both fighters at 0%. Player 1 Mario, CPU Mario.
- Display & Controls › Input Display = Frame strip. Hitbox Visualization = Off.
- CPU › Shield = None, and DI Direction and SDI Direction cleared, unless the row changes one of them.
- Menu closed before the timed part. The strip is at the top. Both fighters are standing, not in the intro demo.
- Chords at their defaults: Save ZL+Down, Load ZL+Up, Record ZR+Down, Play on CPU ZR+Up, Play on Player 1 ZR+Left,
  recording pop-up ZR+Right.

`strip` means the frame strip already on this branch. `rr-11` means the stick-row build. `rr-6` means the pop-up and
Recording page build. A row that says `rr-6` is not a capture of the old slot page.

| File | Embedded in | Build | Length | Start, then the inputs | Must be visible |
| --- | --- | --- | --- | --- | --- |
| `media/rc5-frame-strip-lanes.gif` | FRAME-STRIP.md, Lanes and colours; README, Frame strip | strip | 6 s | Stand at centre. Dash (left stick right, past the walk threshold), then A for a forward smash into the CPU. Hold still until both can act. | P1 gold startup, red active, blue recovery. CPU light-grey hitlag, then purple hitstun. A run of 7 or more shows its length. Newest frame on the right. |
| `media/rc5-frame-strip-chips.gif` | FRAME-STRIP.md, Chips | strip | 4 s | Left stick up-right (past both thresholds), then A once. Release the stick. | One green attack chip, an up-right arrow on it, and a white tick down to P1's lane. No second chip. |
| `media/rc5-frame-strip-cstick.gif` | FRAME-STRIP.md, Chips | rr-11 | 4 s | Left stick neutral. C-stick right, one forward smash. Do not press A. | One attack chip with a right arrow. The left stick never left neutral. |
| `media/rc5-frame-strip-stick-row.gif` | FRAME-STRIP.md, Stick row | rr-11 | 6 s | Hold left stick left for about half a second, return to neutral for about a quarter second, then hold down-right, then alternate left and right (a dash dance) for about two seconds. No buttons. | A row under P1's lane. A direction only while the stick is held. Blank cells while neutral. The row scrolls with the lanes. |
| `media/rc5-frame-strip-freeze.gif` | FRAME-STRIP.md, Freeze | strip | 5 s | Jab the CPU with A, then both stand with the stick neutral and no buttons for a full second. Then A again. | The lanes stop moving while both stand, then move again on the second jab. |
| `media/rc5-frame-strip-freeze-stick.gif` | FRAME-STRIP.md, Freeze | rr-11 | 5 s | After a jab, both can act. Do not press a button. Move the left stick from neutral to left, then to right, for about two seconds. | The strip keeps scrolling while the stick changes direction. It does not freeze. |
| `media/rc5-frame-strip-advantage-hit.gif` | FRAME-STRIP.md, ADVANTAGE | strip | 5 s | A, one jab on the CPU, then both stand until the readout settles. | ADVANTAGE is a signed number, not a dash, and the line under it reads `on hit`. Plus is green. Minus is red. |
| `media/rc5-frame-strip-advantage-shield.gif` | FRAME-STRIP.md, ADVANTAGE | strip | 6 s | Open the menu (B+Up). On CPU, ZL+Down until Shield. Right on the Shield row until it reads Hold. B closes. Jab the CPU's shield with A. Stand until the readout settles. | ADVANTAGE reads `on shield`. LINK does not become TRUE combo or DROP for that shield hit. |
| `media/rc5-frame-strip-link.gif` | FRAME-STRIP.md, LINK | strip | 5 s | A, then A again as soon as the first jab can be cancelled, so the second jab hits before the CPU can act. | LINK shows a number of 0 or below and the words `TRUE combo`, in green. A green bar above P1's lane reads `window Nf`. |
| `media/rc5-frame-strip-drop.gif` | FRAME-STRIP.md, LINK | strip | 6 s | Jab with A. Wait until the CPU can act, then jab again before one second has passed (inside 20 frames of the CPU being free). | LINK shows a positive number and the word `DROP`, in red. |
| `media/rc5-frame-strip-input.gif` | FRAME-STRIP.md, INPUT | strip | 6 s | Same late jab as the drop clip: wait a few frames after you can act, then A. | INPUT shows `Nf late` or `on time` or `buffered Nf`, and then either `N spare` or `Nf past`. |
| `media/rc5-frame-strip-last.gif` | FRAME-STRIP.md, LAST | strip | 7 s | Two jab follow-ups in a row (jab, jab, jab), then stand. | The LAST row shows up to four pills, newest first. A pill reads `TRUE` or `DROP` with its number, and the moves read like `Jab > Jab`. |
| `media/rc5-frame-strip-exchange.gif` | FRAME-STRIP.md, The 20-frame exchange | strip | 8 s | Jab with A. Stand still for a full second (more than 20 frames after the CPU can act). Jab again. | The second hit does not turn LINK into a large DROP of the first exchange. After both can act, ADVANTAGE is the new exchange, `on hit`. |
| `media/rc5-save-warp.gif` | RECORDING.md, Save states only warp | strip | 6 s | At centre, 0%, ZL+Down to save the current slot. Walk both fighters to the ledge. ZL+Up. | Both fighters return to the saved spot. No recording starts. The CPU does not play inputs. |
| `media/rc5-record-start-save.gif` | RECORDING.md, Recording a macro | rr-6 | 8 s | Pop-up on Player. ZR+Down, walk forward and jab once, ZR+Down again to stop. Open the pop-up. | A new macro at the top of the hot five, an orange floppy, and a start save named with the macro. |
| `media/rc5-play-chord.gif` | RECORDING.md, Set active; README, Recording | rr-6 | 8 s | Set that macro active for the CPU. Stand away from its start save. ZR+Up. | One warp to the start save, then the CPU plays the jab. A second press of ZR+Up stops it. |
| `media/rc5-loop-no-warp.gif` | RECORDING.md, The loop | rr-6 | 8 s | Same macro, CPU loop set to 2 (X while the cursor is in Macros). ZR+Up and let both passes finish. | The first pass warps once. The second pass does not warp. It continues from where the first pass ended. |
| `media/rc5-chain-play.gif` | RECORDING.md, Chains | rr-6 | 9 s | + New chain. Number two recordings, the first of which has a start save. Name it. Set it active for the CPU. ZR+Up. | One warp to macro 1's start save, then recording 1, then recording 2, with no warp between them. |
| `media/rc5-grade-not-true.gif` | RECORDING.md, Combos | strip | 9 s | Save last combo on a jab into jab that the strip called a true combo. Load the same save. Attempt 1: the same two jabs, late enough that LINK says DROP, but within the tolerance and still a jab. Attempt 2: the follow-up is a forward tilt instead of a jab. Combo helper On. | Attempt 1 still counts (the bar does not require TRUE). Attempt 2 drops, and the bar says the move was wrong. |
| `media/rc5-make-combo.gif` | RECORDING.md, Combos | rr-6 | 8 s | Drill some other combo first, so the bar is showing it. On the macro's screen, Make combo. | The macro plays once on Player 1 from its start save. A new combo appears. The drilled combo is still the drill when it finishes. |
| `media/rc5-crop-combo.gif` | RECORDING.md, Combos | rr-6 | 7 s | Open a combo of at least two links. Crop. L and R set the range to drop the last link. A saves. | A new combo of the kept links. The original combo is still in the list. |
| `media/rc5-sequence-drill.gif` | RECORDING.md, Sequences | rr-6 | 9 s | A sequence of two short combos, drilled, streak 0. Land combo 1 cleanly, then fail combo 2 with a different move. | The bar names the sequence and moves from combo 1 to combo 2. After the drop it is back on combo 1, streak 0. |
| `media/rc5-popup.gif` | RECORDING.md, The pop-up | rr-6 | 8 s | In the match, ZR+Right. Cursor on a save that is not the bound save. Down onto a macro. B. | The window over the match. Player \| CPU at the top. Two stacked sections. Green fill on the bound save. White cursor. B closes it and the bound save is unchanged. |
| `media/rc5-recording-page.gif` | RECORDING.md, The Recording page | rr-6 | 8 s | Open the menu, go to Recording. ZR once. Then ZR back. | Player view shows saves and macros, and Combos & Sequences in the left column. CPU view hides Combos & Sequences. The bound save is green in both. |
| `media/rc5-macro-screen.gif` | RECORDING.md, A file's own screen | rr-6 | 6 s | On the Recording page, A on a macro that has a start save. | Breadcrumb with the role and the name. Start save with the floppy, switch On. Set active is one row for that role. |
| `media/rc5-fighter-mismatch.gif` | RECORDING.md, Fighter Mismatch | rr-6, plus the chain prompt | 7 s | CPU is Mario. Active chain whose first macro was recorded on a different fighter (change P1 to Pikachu, record a jab, change P1 back to Mario, put that recording first in a chain, set the chain active for the CPU). ZR+Up. Choose Keep. | One Fighter Mismatch prompt, title `Fighter Mismatch`, rows Keep current fighter and Switch to recorded fighter. It names the first macro's fighter. No second prompt when the later macro plays. |
| `media/rc5-migration.gif` | RECORDING.md, Migration | first boot of a card that still has the old slots | 8 s | Before boot, leave a non-empty recording in slot 1 and leave the other recording slots empty. Boot. Set nothing else active. ZR+Up if Player 1's active macro is that migrated slot; otherwise open the list and play `Slot 1 (YYYY-MM-DD)`. | The name uses `Slot 1` and the date. Play does not warp. Empty slots did not become files. |

The redesign mockups in `docs/decisions/RECORDING-REDESIGN-*.png` are not on this branch. They are not a substitute
for the rows above. The rc.3 clip `media/frame-strip.gif` remains the earlier take of the strip. The RC10 clip
`media/menu-recording.gif` remains the slot recording.

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
| `respawn-off.gif` | Off: the same KO, and P1 drops back in at once. This 2026-11-05 clip changed the row on the Saves page; the row is now on Fighters & Stages › Stages. 5.6 s, step 2. |
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
