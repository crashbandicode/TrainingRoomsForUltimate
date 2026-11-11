# Media

Every GIF in [`media/`](media/) is actual Eden footage: whole screen, 1280×720, one clip per behaviour, under 10 s,
game-speed timing kept, trimmed at the ends (any cut inside a clip is listed with it). Frames were reviewed (start,
action, result) before a clip was kept. The raw takes stay outside git with their runs.

## Post-ship refresh: rc.7 as shipped (2026-11-11)

- **Why:** many earlier clips carry a line of text across the frame, "Key BTN - LS +0000,+0000 RS +0000,+0000 GC LT ---
  RT ---". It comes from another plugin, `libsmash_gamepad_overlay.nro`, which was in the capture sessions' plugin set.
  The rc.1–rc.4 clips also show the fork's old 15-row input log at the left edge.
  - The audit is `artifacts/phase23/feature-gifs/POSTSHIP-RETAKES.md`: 43 of the 68 GIFs carry the bar.
  - Every docs capture now leaves that plugin out.
- **Build:** rc.7 `722525a5` as shipped (release NRO `b6e60bb9…`). The clips use the same commit's diag NRO
  `cfec85c7…`, because the rows check its log lines; the diag build adds log lines only.
- **Runs:** one instance each, 2026-11-11:
  - on the Yoga: `docs-rd2-722525a5` (05:06–05:14Z), `docs-rd2b-722525a5` (05:27–05:34Z) and `docs-rd3b-722525a5`
    (05:18–05:23Z);
  - on the Surface: `docs-rd5-722525a5` (07:42–07:48Z).
- **Review and cuts:** each take's frames were read on its row's timeline. The cuts use `clips.py cutw` at 10 fps, as
  above.

| File | Run, row | Embedded in | Length, size | Raw windows kept | What the frames show |
| --- | --- | --- | --- | --- | --- |
| `media/rc7-menu-tour.gif` | `docs-rd2b-722525a5`, `fx-menu-tour-hitbox` | README (top), DIFFERENCES-FROM-THE-FORK.md | 7.5 s, 3.6 MB | 4.0–4.6, 7.6–8.6, 9.9–10.7, 11.4–12.2, 12.9–13.7, 14.5–15.5, 24.6–25.6, 28.5–30.0 s | Training with nothing drawn; the menu on CPU, then Saves, Recording, Fighters & Stages, Display & Controls (Hitbox Visualization [OFF]); [ON]; back in Training, yellow shapes cover Mario and Jigglypuff. |
| `media/rc7-hitbox.gif` | `docs-rd2b-722525a5`, `fx-menu-tour-hitbox` | README, Hitboxes and grab boxes | 8.8 s, 8.0 MB | 4.0–4.8, 22.3–25.0, 28.5–33.8 s | 0–0.8 s Training, nothing drawn; 0.8–2.1 s Display & Controls, Hitbox Visualization [OFF] focused, then [ON]; 3.5–8.8 s yellow shapes on both fighters, Player 1's forward smash, the CPU 6.3% → 24.8%. The row's own help text reads "Hurtboxes are not available in this RC". |
| `media/rc7-respawn-on.gif` | `docs-rd2b-722525a5`, `fx-respawn-on` | README, Save states and respawn | 9.6 s, 9.1 MB | 3.0–3.5, 17.9–19.2, 24.0–27.8, 34.0–38.0 s | 0–0.5 s Training; 0.5–1.8 s Fighters & Stages › Stages, Respawn on platform [OFF] focused, then [ON]; 1.8–5.6 s Player 1 runs off the left side and is KO'd; 5.6–9.6 s Player 1 on the revival platform at the top, coming down. The take has a second KO between 27.8 and 34.0 s (the run-off macro went on after the first respawn); the platform shown follows it. |
| `media/rc7-change-stage.gif` | `docs-rd2b-722525a5`, `fx-change-stage` | README, Stages | 5.4 s, 3.1 MB | 0.0–0.8, 8.8–10.0, 10.6–11.6, 12.4–13.6, 18.6–19.8 s | Training; the Change Stage grid, the cursor from Waiting Room to Battlefield [N]; Y gives Battlefield [Ω] ("Form: Omega"); A, and "Practice Stage · Stage requested"; Training on Battlefield Ω. |
| `media/rc7-kirby-hats.gif` | `docs-rd3b-722525a5`, `fx-p1-kirby-hats` | README, Fighters, cycles and Kirby hats | 8.1 s, 7.1 MB | 0.0–8.1 s (the whole take) | 0–1.1 s Player 1's plain Kirby vs Olimar; the hat chord; 4.4 s Kirby wears Falco's hat; the chord again; 7.4 s Sheik's hat. |
| `media/rc7-combo-helper.gif` | `docs-rd2-722525a5`, `fx-make-combo-drill` | README, Combos | 5.4 s, 6.1 MB | 42.8–44.0, 50.8–55.0 s | The pop-up's COMBOS with Combo 1 (made from Macro 1, jab → jab) drilled; back in Training the jabs; the bar under the frame strip: "Combo 1 · 1 Jab>Jab · TRUE −5 · on time". |

| `media/rc7-di-up.gif` | `docs-rd5-722525a5`, `fx-di-up` (Surface, 07:42–07:48Z) | README, CPU reactions | 9.0 s, 7.9 MB | 7.9–8.6, 9.3–12.3, 21.9–22.5, 23.7–24.9, 31.6–32.1, 32.8–35.8 s | 0–0.7 s Toon Link loaded at 80% (CPU damage on load Random Value 80–80, DI and SDI empty); 0.7–3.7 s Player 1's forward smash, the CPU launched on a shallow line, landing at the right; 3.7–5.5 s CPU › Defence › DI Direction, "none selected", then "Up x1"; 5.5–9.0 s the same load at 80% and forward smash, a steeper line, the CPU near the top right 1.5 s after the hit (mid-height with no DI). |
| `media/rc7-menu-shortcut.gif` | `docs-rd5-722525a5`, `fx-menu-shortcut` | README, Menu shortcuts; DIFFERENCES-FROM-THE-FORK.md | 6.2 s, 1.5 MB | 0.0–0.3, 5.0–5.6, 6.7–7.5, 8.0–9.4, 10.2–10.9, 13.8–14.6, 15.4–17.0 s | Training; Display & Controls › Menu shortcut 1 "Minus"; the chord editor; the capture reads "ZL", then "ZL + D-pad Up"; A, and the row reads "ZL + D-pad Up" (`button_config.ini written by the writer thread`); Training; the menu open again (`[Ui] menu opened`), from ZL + D-pad Up. |
| `media/rc7-respawn-off.gif` | `docs-rd5-722525a5`, `fx-respawn-off` | DIFFERENCES-FROM-THE-FORK.md | 7.5 s, 6.3 MB | 3.8–4.4, 9.75–10.8, 15.6–17.2, 18.2–22.4 s | Training; Fighters & Stages › Stages › Respawn on platform [ON], then [OFF]; Player 1 runs off the left side, KO; 0.5 s after the blast Player 1 is back at the top, falling, with no platform (the plugin's `dead_rebirth_wait_frame` returned 1). |

The takes' frames were read on each row's timeline; the machine checks are in each row's JSON (`di_state` Up alone,
`respawn_invincibility` Off, the shortcut written and the menu opened by it).

From `docs-rd1-722525a5` (the Yoga, alone, 2026-11-11 04:32–04:48Z; the rc.5 docs rows and the rc.6 sequence row on
the shipped build), reviewed take by take (batch I). The rows' own takes are 1280 wide at 15 fps and are kept at 15 fps.

| File | Row | Embedded in | Length, size | Raw windows kept | What the frames show |
| --- | --- | --- | --- | --- | --- |
| `media/rc7-frame-strip-lanes.gif` | `gif-frame-strip-lanes` | FRAME-STRIP.md | 3.8 s, 5.6 MB | 0–3.8 s | Player 1's lane gold, red, blue; the CPU's light grey then purple; runs print 11, 30, 23; ADVANTAGE −7 on hit. |
| `media/rc7-frame-strip-chips.gif` | `gif-frame-strip-chips` | FRAME-STRIP.md | 3.2 s, 4.7 MB | 0–3.2 s | The attack chip with its arrow; a leftover X chip from the setup is on the strip until 1.3 s, never beside the new chip. |
| `media/rc7-frame-strip-cstick.gif` | `gif-frame-strip-cstick` | FRAME-STRIP.md | 2.7 s, 4.1 MB | 0–2.7 s | A C-stick forward smash: the chip's right arrow; the stick row stays blank. |
| `media/rc7-frame-strip-stick-row.gif` | `gif-frame-strip-stick-row` | FRAME-STRIP.md | 2.7 s, 4.0 MB | 0–1.1, 4.4–6.0 s | Held directions and blanks under Player 1's lane. The take's ledge fall and KO (1.2–2.4 s) are cut, with the down-right run in that part. |
| `media/rc7-frame-strip-freeze-stick.gif` | `gif-frame-strip-freeze-stick` | FRAME-STRIP.md | 4.7 s, 7.0 MB | 0–4.7 s | The strip keeps scrolling through neutral, left, right with no button pressed; Mario runs off the left ledge at 3.2 s. |
| `media/rc7-chain-play.gif` | `gif-chain-play` | RECORDING.md, Chains | 3.6 s, 5.0 MB | 0–3.6 s | One warp; recording 1 (walk and jab), then recording 2 (jab) with no warp between. |
| `media/rc7-fighter-mismatch.gif` | `gif-fighter-mismatch` | RECORDING.md | 5.0 s, 4.1 MB | 0–5.0 s | The cursor starts on Keep; Mario plays the chain; no second prompt. |
| `media/rc7-loop-no-warp.gif` | `gif-loop-no-warp` | RECORDING.md | 3.8 s, 5.3 MB | 0–3.8 s | The first pass warps; the second continues from where the first ended, with no warp. |
| `media/rc7-sequence-drill.gif` | `seq-gif` | README, Combos; RECORDING.md; RELEASE-NOTES-rc6.md | 6.0 s, 8.9 MB | 0–6.0 s | SEQUENCE Combo pairs: Pair 1 lands (✓), Pair 2 starts, "L1 wrong move: Up tilt", DROP +16, back at combo 1; streak 4 → 0, best 9. |
| `media/rc7-record.gif` | `fx-record-list` (10 fps) | README, Recording and playback | 7.6 s, 3.5 MB | 11.6–13.0, 15.2–16.0, 17.9–18.7, 20.0–20.7, 22.4–23.7, 26.4–29.0 s | The pop-up with no saves and no macros (Player, then CPU); Record on Player 1 and two jabs; the pop-up lists "Macro 1" (Recorded on Mario) and "Macro 1 · start", with the connection. No playback. |

Kept from rc.5 but cut down to leave out the bar (primary 04:40Z, rule c: the retake from rd1 fell short, and the clip
is clean outside the bar). The crops keep every frame and its time; the trims keep the menu part only.
- Cropped to the top 436 rows (the bar is at y 443–466): `rc5-frame-strip-advantage-hit`, `-advantage-shield`,
  `-drop`, `-exchange`, `-freeze`, `-input`, `-last`, `-link` (FRAME-STRIP.md) and `rc5-play-chord` (RECORDING.md),
  each as `<name>-crop.gif`. The strip and readouts are whole; the fighters are cut at the knees or waist.
  - The rd1 retakes of those eight strip clips started with Player 1 out of jab range (the setup's turn step logged
    "no bodies found"), so no hit, no readout. The play-chord retake does not show the stop.
- Trimmed: `rc5-popup-trim.gif` (0.45–3.95 s, the pop-up only; the bar shows once it closes, so the close is not
  shown) and `rc5-recording-page-trim.gif` (from 0.5 s; the first 0.47 s in Training had the bar).
- Removed: `rc5-record-start-save.gif` (RECORDING.md). `rc7-record-macro.gif`, embedded right after it, shows the
  same record and start save without the bar.

With Online Deluxe 0.10.1 and the CPU pilot 0.2.1 (the card's own install, `switch-mtp-v0101-v021`) staged and the
gamepad overlay left out: `docs-rd6-722525a5` (Surface, 2026-11-11 09:28–09:33Z) and `docs-rd6b-722525a5`
(09:58–10:03Z).

| File | Run, row | Embedded in | Length, size | Raw windows kept | What the frames show |
| --- | --- | --- | --- | --- | --- |
| `media/rc7-replay-capture.gif` | `docs-rd6-722525a5`, `fx-replay-capture` | README, Menu shortcuts and replay capture; DIFFERENCES-FROM-THE-FORK.md | 5.4 s, 3.4 MB | 0.0–0.6, 10.3–11.9, 27.3–28.4, 29.3–31.4 s | Training (a leftover "Saved Slot 1" notice); Recording › Replays › Replay capture [OFF], then [ON]; Training, the Online Deluxe overlay at the top right (its network page); its plugins page, "Training Rooms › Replay capture On". The log has `[ReplayCapture] menu preference enabled=true result=Applied` and `replay_capture.ini written by the writer thread enabled=true` at the menu's close. (The row's own check looked before the close and failed; the take is complete.) |

`record-refused.gif` (2026-11-05, the bar and the old input log) is removed and **not retaken**: on this build Record
was not refused. In `docs-rd6b-722525a5` `fx-record-refused`:
- the overlay's CPU Pilot page turned P1 from Player to "CPU (L3)", and the HUD read "CPU Lv3";
- the pilot moved Player 1;
- TrainingRooms had logged `pilot_present=true` at boot;
- Record (ZR + D-pad Down) then showed "Input Recording · Lockout", then "Recording", not "P1 is controlled by an AI:
  recording is off".

The evidence clip is `artifacts/phase23/feature-gifs/evidence/rd6b-record-not-refused.gif` (640 wide, 7.4 s, 4.0 MB;
evidence clips stay outside this repository): Player 1's HUD reads MARIO; the overlay's CPU Pilot page goes from
"Player" to "CPU (L3)" and the HUD from MARIO to CPU Lv3; the pilot dashes Player 1; Record shows "Input Recording ·
Lockout", then "Recording".

The README, DIFFERENCES-FROM-THE-FORK.md and the rc.6 release notes carry it as a known issue: when the standalone CPU
pilot controls Player 1, Record isn't refused yet (fix planned). The fix is planned for rc.9.

Retired from the README (each carried the bar):
- `mii-editor.gif` and `mii-editor-list.gif` (2026-11-05): `rc7-mii-editor.gif` shows the editor, the new row and the
  pick;
- `menu-tour.gif`, `hitbox-overlay.gif`, `respawn-on.gif`, `change-stage.gif`, `cpu-kirby-hat.gif`,
  `combo-helper.gif`, `di-up.gif` and `di-none.gif` (one clip now), `menu-shortcut.gif`, and `respawn-off.gif`
  (DIFFERENCES-FROM-THE-FORK.md): replaced above.

## rc.7 docs clips (2026-11-10)

- **Build:** rc.7 `7d720c68` (`p23/rc7-final` at 21:06Z: F1 plus the Fighter Mismatch role fix), the diag NRO
  `a45ba951…`, which adds log lines only. rc.7's final head later moved to `b3678dfe`, which adds A's body-bank
  scheduler fix. None of these three clips depends on that code.
- **Run:** `docs-rc7b-7d720c68`, the Surface, one instance, 2026-11-10 21:38–22:38Z. 30 of 30 rows passed, after one
  retry of four rows that had failed on harness faults.
- **Also:** `docs-rc7c-b3678dfe`, the XPS, one instance, 2026-11-11 01:44–01:52Z, on rc.7's final head `b3678dfe`
  (diag NRO `9dfc784c…`), for `rc7-record-macro` and `rc7-cpu-level-notice`.
- **Review and cuts:**
  - Each take was reviewed against its row's own timeline (`take_timeline.py`; batch F in
    `artifacts/phase23/feature-gifs/takes/verdicts/`).
  - Each was cut to before, action and result (`clips.py cutw`, 10 fps): one palette of 128 colours from three frames,
    no dither, the game's frame times.
  - Of a frame shorter than 1/10 s and the one after it, the longer is shown. Before 2026-11-11 00:58Z the cut kept the
    first, and so dropped the 300 ms Switch frame from the first `rc7-mismatch-player1` cut.

| File | Run, row | Embedded in | Length, size | Raw windows kept | What the frames show |
| --- | --- | --- | --- | --- | --- |
| `media/rc7-mismatch-player1.gif` | `docs-rc7b-7d720c68`, `cov-combo-other-fighter` | RECORDING.md, Fighter Mismatch | 5.8 s, 3.9 MB | 48.6–51.2, 53.0–56.2 s | 0–0.8 s the pop-up's COMBOS, the cursor on Luigi jabs ("Recorded on Luigi"); 0.8–2.2 s Fighter Mismatch, the footer "Recorded Luigi · current Player 1 Mario", Keep focused; 2.2–2.6 s "Switch to recorded fighter" focused and chosen; 2.6–4.6 s the match, Player 1 still Mario (HUD MARIO); 4.7–5.8 s Luigi drops in as Player 1, "Player Character · Player fighter applied", HUD LUIGI. Player Swapping was Off (it doesn't block this switch). |
| `media/rc7-mii-editor.gif` | `docs-rc7b-7d720c68`, `fx-mii-editor-use` | README, Mii fighters | 7.3 s, 5.2 MB | 19.43–20.43, 34.92–36.42, 63.72–64.32, 67.12–68.62, 73.6–76.3 s | 0–1.0 s Smash's Mii Fighter Settings, specials 1/1/1/1; 1.0–2.4 s Neutral Special 2 Flashing Mach Punch and Side Special 2 Burning Dropkick, Save; 2.4–2.5 s the panel closes (Save); 2.5–3.1 s the Mii Brawler list with a fourth row, "4. 格斗 [2211]", the "> " on row 1; 3.1–3.8 s row 4 focused; 3.8–4.6 s "4. > 格斗 [2211]" (the marker leaves row 1); 4.6–7.3 s Training, Player 1 is Mii Brawler ("Player Mii configuration applied"), and B is the new neutral special's punch rush (strip: a multi-hit move). The row also checked that one new Mii file was written and nothing else changed. |
| `media/rc7-lobby-stages-rotation.gif` | `docs-rc7b-7d720c68`, `fx-lobby-stages-rotation` | README, Stages | 7.8 s, 6.3 MB | 12.32–14.2, 24.92–27.4, 43.52–44.4, 66.67–69.2 s | 0–1.9 s the Lobby Stages grid: "1. Waiting Room", "2. Big Battlefield"; 1.9–3.0 s Training Stage Rotation [ON], the menu closes; 3.0–4.4 s Training on the training stage (Mario vs Donkey Kong); 4.4–5.3 s the character select, READY TO FIGHT; 5.3–7.8 s Training opens on Big Battlefield (Mario vs Dark Pit). The plugin's log names the stage it applied (`[TrainingRotation] apply`, stage 2). |
| `media/rc7-record-macro.gif` | `docs-rc7c-b3678dfe`, `fx-record-list` | RECORDING.md, Recording a macro | 6.6 s, 3.4 MB | 11.7–12.9, 15.3–16.0, 18.1–18.7, 20.3–20.9, 22.5–23.9, 26.4–28.5 s | 0–1.2 s the pop-up on Player 1: SAVE STATES (0), MACROS (0); 1.2–1.9 s Record (the pop-up opens and closes on Player); 1.9–3.1 s two jabs; 3.1–4.4 s the stop (the pop-up's role to CPU, the chord again); 4.4–6.6 s the pop-up: SAVE STATES (1) "Macro 1 · start", MACROS (1) "Macro 1" (Recorded on Mario), and CONNECTIONS "start save of Macro 1". The row checked that `macro-1.json` and `macro-1-start.json` are new. |
| `media/rc7-cpu-level-notice.gif` | `docs-rc7c-b3678dfe`, `fx-lc-entry-notice` | README, CPU reactions | 9.7 s, 7.1 MB | 0.0–0.6, 4.2–5.4, 17.4–19.0, 21.1–22.9, 33.9–38.4 s | 0–0.6 s Training, Mario vs Hero; 0.6–1.8 s the pause menu (CPU Behavior Stop), Quit Training; 1.8–3.6 s the character select, the CPU card "Random, CPU Lv. 3"; 3.6–4.5 s READY TO FIGHT; 4.5–5.4 s loading; 5.4–9.5 s Training, Mario vs King K. Rool, the notice "CPU level · Lv 7 from TrainingRooms (Training menu: Lv 3)". CPU Behavior is Stop, so the CPU's play at Lv 7 isn't shown. |

## rc.6 clips

`rc6-sequence-drill.gif` comes from run `rc6-seq-diag1` on the Yoga, 2026-11-08, row `seq-gif`, 13:40:04–13:40:27Z,
with the diag NRO `50c8a5df` of `p23/rc6-seq-drill` `e6ac460`. The diag build adds log lines only; the screen draws the
same. The take is Eden's own recorder at 1280×720 and 15 fps, 6 s, 91 frames, 13.3 MB (raw take SHA-256
`71d818e4…372d`, kept in `artifacts/phase23/rc6-eden/seq-diag1/out/seq-gif/`). It was re-encoded with the playbook's
`tools/eden-solo/encode.py`: one shared palette of 96 colours, no dither, every second frame kept with the time of both,
so it still plays at game speed (46 frames, 6.9 MB).

| File | Re-run row | Host | Embedded in | Build | Length | Start, then the inputs | Must be visible | In `media/` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `media/rc6-sequence-drill.gif` | `seq-gif` in `artifacts/phase23/rc6-eden/rows-seq/rows.json` | yoga | RECORDING.md, Sequences and the one drill; RELEASE-NOTES-rc6.md | rc.6 | 6 s | A sequence of three one-link combos (Pair 1, 2, 3: jab, jab), file streak 4 and best 9, is the drill. Load to a save with Player 1 in jab range. Jab, jab (Pair 1); walk forward a little; then an up tilt where Pair 2's jab should be. | The bar's SEQUENCE header, combo 1 of 3, and the row Pair 1 → Pair 2 → Pair 3. Pair 1 turns green with the check and the header reads combo 2 of 3, "Pair 1 landed". Then Pair 2 turns red, the footer reads "Pair 2 dropped · L1 wrong move: Up tilt · restart at combo 1", the header is back on combo 1, and the streak goes from 4 to 0 with best 9. | yes |

## rc.6 docs clips (2026-11-10)

A media audit on 2026-11-10 asked of every GIF whether it shows the state before, the action, and the result in the
game or in a file, not only a menu. These clips replace or join the GIFs that showed only a menu
(`artifacts/phase23/feature-gifs/AUDIT-MEDIA.md`).

- **Build:** rc.6 `e6501ac6`, the diag NRO `c1b9f694…`, which adds log lines only, so the screen draws the same.
  Eden's own recorder took them at 1280×720.
- **Runs:**
  - `docs-boot1-e6501ac6`: the XPS, 2026-11-10, 14:19–15:59Z;
  - `docs-boot2-e6501ac6`: the Surface, 14:21–16:19Z;
  - worker B's `rc6-docs-online-e6501ac6`: the XPS, 13:21–13:47Z. Two instances, b with TrainingRooms and a
    vanilla, in a Quickplay room hosted in Eden.
- **Review and cuts:**
  - Each take was reviewed at the times in its row below (`clips.py times`).
  - Each was cut to the before, the action and the result (`clips.py cutw`, 10 fps): one palette of 128 colours from
    three frames, no dither, the game's frame times.
  - The windows kept from the raw take are listed with each clip. The raw takes stay with the runs in
    `artifacts/phase23/feature-gifs/runs/`.

| File | Run, row | Embedded in | Length, size | Raw windows kept | What the frames show |
| --- | --- | --- | --- | --- | --- |
| `media/rc6-crop-combo.gif` | boot 1, `fx-crop-combo` | RECORDING.md, Combos | 9.1 s, 7.3 MB | 25.0–27.6, 43.3–46.6, 57.2–58.2, 59.3–61.7 s | 0–2.6 s Combo 1 drilled, two links in the bar; 2.6–5.9 s Crop, Out 2 → 1; 5.9–6.9 s Combo 2 listed beside Combo 1; 6.9–9.1 s Combo 2 drilled, one link. The row also checked that Combo 1's file was unchanged. |
| `media/rc6-make-combo.gif` | boot 1, `fx-make-combo-drill` | RECORDING.md, Combos | 9.5 s, 5.4 MB | 32.4–33.4, 34.4–37.6, 42.6–44.0, 51.0–54.9 s | 0–1.0 s Make combo on Macro 1; 1.0–4.2 s the macro plays (jab, jab); 4.2–5.6 s COMBOS lists Combo 1, made from Macro 1, one link, drilled; 5.6–9.5 s Play: the warp, the jabs land, and the bar grades Combo 1 TRUE. |
| `media/rc6-macro-set-active.gif` | boot 1, `fx-set-active-play` | RECORDING.md, A file's own screen | 6.3 s, 5.5 MB | 33.0–35.9, 38.2–41.8 s | 0–1.3 s the macro's screen, Set active Off; 1.3–2.9 s On, Player 1; 2.9–6.3 s Play on Player 1: the fighters warp to its start save and the jab lands (CPU 0 → 2.7%). |
| `media/rc6-cycle-switch.gif` | boot 2, `fx-cycle-switch` | README, Fighters, cycles and Kirby hats; DIFFERENCES-FROM-THE-FORK.md | 8.6 s, 8.6 MB | 15.2–17.2, 21.6–23.0, 25.6–27.4, 28.6–30.0, 31.6–33.6 s | 0–2.0 s Character Cycles, red badges: Mario 0, Mii Brawler 1, Mii Swordfighter 2, Mii Gunner 3, Link 4; 2.0–3.4 s Training, Player 1 Mii Gunner; 3.4–5.2 s the chord: Link, "Player fighter applied"; 5.2–6.6 s Mario, "Original fighter restored"; 6.6–8.6 s Mii Brawler, "Player Mii configuration applied". |
| `media/rc6-replay-window.gif` | boot 1, `cov-replay-window` | README, Menu shortcuts and replay capture | 8.4 s, 6.1 MB | 24.1–25.4, 25.8–26.8, 28.2–30.2, 31.0–31.8, 32.9–35.0, 36.4–37.6 s | 0–1.3 s Replay Captures lists the capture (Mario vs Mii Gunner, 180 frames); 1.3–2.3 s the Replay window; 2.3–4.3 s Fighter Mismatch, Switch to recorded fighter; 4.3–5.1 s Training, CPU Mario; 5.1–7.2 s "Saved Mii applied", the CPU reloads; 7.2–8.4 s CPU Mii Gunner. The capture is the row's fixture, an rc.4 capture. |
| `media/rc6-waiting-partners.gif` | B, `qp-panel-change` | README, Waiting rooms | 6.6 s, 5.5 MB | B's clip, as kept in its run's `clips/` | 0.0–0.6 s one partner; 1.2 s the game's waiting-room menu (Training Partners 1, Stop); 3.7 s Right → 2, and a second partner drops in; 5.0–6.6 s both stand. |
| `media/rc6-waiting-reentry.gif` | B, `qp-reentry` | README, Waiting rooms | 9.9 s, 6.8 MB | B's clip, as kept in its run's `clips/` | 0.0–0.5 s loading; 0.6 s the next room fades in with the two remembered partners; 1.5–9.9 s both stand (Stop). |
| `media/rc6-save-last-combo.gif` | boot 1, `cov-save-last-combo` | RECORDING.md, Combos | 9.3 s, 5.8 MB | 16.8–20.3, 27.4–33.2 s | 0–3.5 s jab into jab, TRUE combo on the strip; 3.5–4.7 s Recording page, COMBOS (0), cursor on + Save last combo…; 4.7–6.8 s the name step (Eden's keyboard window is off frame); 6.8–8.3 s COMBOS (1): "Jab jab saved, Recorded on Mario"; 8.5–9.3 s the match, the helper bar on Jab jab saved. |
| `media/rc6-named-mii.gif` | boot 1, `cov-named-mii` | RECORDING.md, Fighter Mismatch | 7.6 s, 7.0 MB | 11.0–18.6 s | 0–0.8 s Player 1 and the CPU on Mario; 0.9–2.8 s Fighter Mismatch, "Recorded Mii Gunner · current CPU Mario", Switch to recorded fighter; 2.9–3.8 s the CPU reloads; 3.9–6.4 s "Saved Mii applied", the CPU is the recorded Mii Gunner; 6.4–7.6 s it stands. |
| `media/rc6-autoload.gif` | boot 1, `cov-autoload` | RECORDING.md, Save states only warp | 9.6 s, 5.6 MB | 12.2–16.3, 26.5–28.2, 31.8–35.6 s | 0–1.4 s Player 1 runs off the left ledge; 2.6 s the KO blast with Player 1 already back beside the CPU (the bound Left save), no revival; 4.1–5.8 s the menu, the Left save's screen, Bind to Save / Load On, then Off; 5.8–9.6 s Player 1 runs off again and comes down on the game's revival, no warp. |
| `media/rc6-both-play.gif` | boot 1, `cov-both-play` | RECORDING.md, Set active, and the Play chords | 6.0 s, 6.8 MB | 13.4–16.0, 19.0–22.4 s | 0–1.0 s both idle; 1.1–2.6 s both Play chords at once: Player 1 and the CPU jab together, both strip lanes run side by side; 2.6–4.0 s idle; 4.1–6.0 s one after the other: the CPU's lane trails Player 1's. |
| `media/rc6-mii-same.gif` | boot 2, `cr-mii-identical` | README, Mii fighters | 9.0 s, 3.6 MB | 15.0–19.1, 24.4–27.2, 39.3–41.4 s | 0–1.5 s Mii Brawler's Player list, three Miis; 1.5 s Add New… focused; 4.4–5.5 s Smash's Mii Fighter Settings on a copy of the class's Mii, Save; 7.4–9.0 s Training, "Mii Fighter – Same as 格斗, nothing new saved". The row checked every Mii file byte-identical. |
| `media/rc6-lobby-combos.gif` | pair, `p13-lobby-combos` (guest b, Local Wireless room hosted in Eden) | README, Waiting rooms | 3.9 s, 3.3 MB | 18.3–19.6, 34.8–36.0, 43.6–45.0 s | 0–1.3 s jab into jab in the waiting room; 1.3–2.5 s COMBOS (0), + Save last combo…; 2.5–3.3 s COMBOS (1): "Lobby jabs, Recorded on Sheik"; 3.3–3.9 s back in the room, the helper bar drills Lobby jabs. |

The review of every take of the three docs boots, per feature, with the frames that show each effect or what a
reshoot must add, is the Clip column of the feature ledger (`artifacts/phase23/FEATURE-LEDGER.md`). Its 640-wide
evidence clips stay outside git, in `artifacts/phase23/feature-gifs/evidence/`.

## rc.5 clips

23 of the rows below are in [`media/`](media/), and the player docs embed them. Three more were there until
2026-11-10, when clips that show the result replaced them (rc.6 docs clips above); they remain in the history up to
`da26d632`. They are Eden footage of
`p23/rc5-wire-fix` diag builds, and for the last three of `p23/rc6-hot-slots` diag builds, which add log lines only (the
screen draws the same):

- The frame-strip clips other than the C-stick one: run `rc5-docs-gifs-xps2` on the XPS, 2026-11-08 00:30–00:56Z, diag NRO `6bbf2ecd`. The drop
  clip and the input clip are the same take.
- `rc5-frame-strip-cstick.gif`, `rc5-macro-screen.gif` and `rc5-crop-combo.gif`: run `rc5-docs-gifs2-xps1`, 2026-11-08
  03:58–04:13Z, diag NRO `58a9f4de` (`614d9e8`). That C-stick take replaces an earlier one whose stick row showed the
  C-stick.
- `rc5-fighter-mismatch.gif`: run `rc5-mismatch-xps3`, 2026-11-08, diag NRO `e9ccf373`.
- `rc5-play-chord.gif`, `rc5-loop-no-warp.gif` and `rc5-chain-play.gif`: run `rc5-docs-gifs4-xps1`, 2026-11-08
  05:35–05:46Z, diag NRO `e5120f8a` (`cf1fafb`, with the played-frame queue). Player 1 starts in reach of the
  CPU's jab, so each clip shows the CPU's jabs landing.
- `rc5-popup.gif`, `rc5-record-start-save.gif`, `rc5-recording-page.gif` and `rc5-save-warp.gif`: Surface run
  `out-confirm-2116Z`, 2026-11-07.
- `rc5-migration.gif`: run `rc6-docs-gifs-xps2` on the XPS, 2026-11-08 16:45Z, diag NRO `d20ead7b` (`p23/rc6-hot-slots`
  `688c478`), booted on cov-rec's pre-library card. The row first checked the card the way cov-rec's #37 row does:
  two dated saves, Slot 2 bound, `oldjab` active, one migrate line. The take's first 0.9 s (a leftover notice) are
  trimmed. It was re-encoded with ffmpeg, one shared palette and no dither: 3.9 MB, 8.1 s.
  - RECORDING.md now embeds `rc5-migration-crop.gif` (2026-11-11): the same frames and times, cropped to the top 436
    rows to leave out the gamepad overlay plugin's input bar. The two lists and the notices are whole; the fighters
    are cut at the waist.
- `rc5-save-warp.gif` (Surface `out-confirm-2116Z`, above): RECORDING.md now embeds `rc5-save-warp-crop.gif`
  (2026-11-11), cropped the same way; the fighters are cut at the knees.
- `rc5-make-combo.gif`: run `rc6-docs-gifs-xps7` on the XPS, 2026-11-08 17:46Z, the same diag NRO, fixture `mk6` (the
  rc.5 `mk` fixture with its start save moved into reach, `make_mk6.py`). After the clip the row checked the card:
  `combo-2` written and hot, `combo-1` still the drill. The take is trimmed to 0.7–10.4 s and re-encoded the same way:
  6.1 MB, 9.7 s.
- `rc5-grade-not-true.gif`: run `rc6-docs-gifs-xps9` on the XPS, 2026-11-08 18:58Z, the same diag NRO, row
  `gif-gnt-f` (the plan's `gif-gnt-b`, renamed for the rerun). The strip logged the three links the row needs: Make combo
  `TRUE 0` (input +4), attempt 1 `+1 DROP` (input +5), attempt 2 up tilt into up smash (input +4). The take is trimmed
  at 0.8 s. It was re-encoded with one shared palette of 96 colours and no dither, keeping every second frame with the
  time of both: 9.9 MB, 7.1 s.
  - RECORDING.md now embeds `rc5-grade-not-true-crop.gif` (2026-11-11): the same frames and times, cropped to the top
    436 rows to leave out the gamepad overlay plugin's input bar (y 443–452). The frame strip and helper bar are
    whole; the fighters are cut at the knees in the last 2 s.

Each was taken at 1280×720 and 15 fps by the runner's `gif` step. Seven takes over 10 MB were re-encoded with one shared
palette of 96 colours and no dither: `rc5-popup.gif` and `rc5-save-warp.gif` keep every frame, and
`rc5-frame-strip-stick-row.gif`, `rc5-frame-strip-freeze-stick.gif`, `rc5-record-start-save.gif`,
`rc5-loop-no-warp.gif` and `rc5-chain-play.gif` keep every second
frame with the time of both, so they still play at game speed.

The other rows are not captured yet, and their doc sections have no picture. Capture each one in the Eden session that
boots the build named on the row. Do not keep a take where the "Must be visible" line is missing. Full screen,
1280×720, one behaviour, game speed, under 10 s. Trim only the ends.

Shared start, unless a row says otherwise:

- Training mode, Battlefield, both fighters at 0%. Player 1 Mario, CPU Mario.
- Display & Controls › Input Display = Frame strip. Hitbox Visualization = Off.
- CPU › Shield = None, and DI Direction and SDI Direction cleared, unless the row changes one of them.
- Menu closed before the timed part. The strip is at the top. Both fighters are standing, not in the intro demo.
- Chords at their defaults: Save ZL+Down, Load ZL+Up, Record ZR+Down, Play on CPU ZR+Up, Play on Player 1 ZR+Left,
  recording pop-up ZR+Right.

`strip` means the frame strip. `rr-11` means the stick row and the C-stick arrow. `rr-6` means the pop-up and the
Recording page. A row that says `rr-6` is not a capture of the old slot page. Re-run row and Host name the row in
`eden/recording-redesign/rows.json` that captures the file.

| File | Re-run row | Host | Embedded in | Build | Length | Start, then the inputs | Must be visible | In `media/` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `media/rc5-frame-strip-lanes.gif` | `media-rc5-frame-strip-lanes` | surface | FRAME-STRIP.md, Lanes and colours | strip | 6 s | Stand at centre. Dash (left stick right, past the walk threshold), then A for a forward smash into the CPU. Hold still until both can act. | P1 gold startup, red active, blue recovery. CPU light-grey hitlag, then purple hitstun. A run of 7 or more shows its length. Newest frame on the right. | yes |
| `media/rc5-frame-strip-chips.gif` | `media-rc5-frame-strip-chips` | surface | FRAME-STRIP.md, Chips | strip | 4 s | Left stick up-right (past both thresholds), then A once. Release the stick. | One green attack chip, an up-right arrow on it, and a white tick down to P1's lane. No second chip. | yes |
| `media/rc5-frame-strip-cstick.gif` | `media-rc5-frame-strip-cstick` | yoga | FRAME-STRIP.md, Chips | rr-11 | 4 s | Left stick neutral. C-stick right, one forward smash. Do not press A. | One attack chip with a right arrow. The left stick never left neutral. | yes |
| `media/rc5-frame-strip-stick-row.gif` | `media-rc5-frame-strip-stick-row` | yoga | FRAME-STRIP.md, Stick row | rr-11 | 6 s | Hold left stick left for about half a second, return to neutral for about a quarter second, then hold down-right, then alternate left and right (a dash dance) for about two seconds. No buttons. | A row under P1's lane. A direction only while the stick is held. Blank cells while neutral. The row scrolls with the lanes. | yes |
| `media/rc5-frame-strip-freeze.gif` | `media-rc5-frame-strip-freeze` | surface | FRAME-STRIP.md, Freeze | strip | 5 s | Jab the CPU with A, then both stand with the stick neutral and no buttons for a full second. Then A again. | The lanes stop moving while both stand, then move again on the second jab. | yes |
| `media/rc5-frame-strip-freeze-stick.gif` | `media-rc5-frame-strip-freeze-stick` | yoga | FRAME-STRIP.md, Freeze | rr-11 | 5 s | After a jab, both can act. Do not press a button. Move the left stick from neutral to left, then to right, for about two seconds. | The strip keeps scrolling while the stick changes direction. It does not freeze. | yes |
| `media/rc5-frame-strip-advantage-hit.gif` | `media-rc5-frame-strip-advantage-hit` | surface | FRAME-STRIP.md, ADVANTAGE | strip | 5 s | A, one jab on the CPU, then both stand until the readout settles. | ADVANTAGE is a signed number, not a dash, and the line under it reads `on hit`. Plus is green. Minus is red. | yes |
| `media/rc5-frame-strip-advantage-shield.gif` | `media-rc5-frame-strip-advantage-shield` | surface | FRAME-STRIP.md, ADVANTAGE | strip | 6 s | Open the menu (B+Up). On CPU, ZL+Down until Shield. Right on the Shield row until it reads Hold. B closes. Jab the CPU's shield with A. Stand until the readout settles. | ADVANTAGE reads `on shield`. LINK does not become TRUE combo or DROP for that shield hit. | yes |
| `media/rc5-frame-strip-link.gif` | `media-rc5-frame-strip-link` | surface | FRAME-STRIP.md, LINK | strip | 5 s | A, then A again as soon as the first jab can be cancelled, so the second jab hits before the CPU can act. | LINK shows a number of 0 or below and the words `TRUE combo`, in green. A green bar above P1's lane reads `window Nf`. | yes |
| `media/rc5-frame-strip-drop.gif` | `media-rc5-frame-strip-drop` | surface | FRAME-STRIP.md, LINK | strip | 6 s | Jab with A. Wait until the CPU can act, then jab again before one second has passed (inside 20 frames of the CPU being free). | LINK shows a positive number and the word `DROP`, in red. | yes |
| `media/rc5-frame-strip-input.gif` | `media-rc5-frame-strip-input` | surface | FRAME-STRIP.md, INPUT | strip | 6 s | Same late jab as the drop clip: wait a few frames after you can act, then A. | INPUT shows `Nf late` or `on time` or `buffered Nf`, and then either `N spare` or `Nf past`. | yes |
| `media/rc5-frame-strip-last.gif` | `media-rc5-frame-strip-last` | surface | FRAME-STRIP.md, LAST | strip | 7 s | Two jab follow-ups in a row (jab, jab, jab), then stand. | The LAST row shows up to four pills, newest first. A pill reads `TRUE` or `DROP` with its number, and the moves read like `Jab > Jab`. | yes |
| `media/rc5-frame-strip-exchange.gif` | `media-rc5-frame-strip-exchange` | surface | FRAME-STRIP.md, The 20-frame exchange | strip | 8 s | Jab with A. Stand still for a full second (more than 20 frames after the CPU can act). Jab again. | The second hit does not turn LINK into a large DROP of the first exchange. After both can act, ADVANTAGE is the new exchange, `on hit`. | yes |
| `media/rc5-save-warp.gif` | `media-rc5-save-warp` | yoga | RECORDING.md, Save states only warp | strip | 6 s | At centre, 0%, ZL+Down to save the current slot. Walk both fighters to the ledge. ZL+Up. | Both fighters return to the saved spot. No recording starts. The CPU does not play inputs. | yes |
| `media/rc5-record-start-save.gif` | `media-rc5-record-start-save` | yoga | RECORDING.md, Recording a macro | rr-6 | 8 s | Pop-up on Player. ZR+Down, walk forward and jab once, ZR+Down again to stop. Open the pop-up. | A new macro listed first, an orange floppy, and a start save named with the macro. | yes |
| `media/rc5-play-chord.gif` | `media-rc5-play-chord` | yoga | RECORDING.md, Set active | rr-6 | 8 s | Set that macro active for the CPU. Stand away from its start save. ZR+Up. | One warp to the start save, then the CPU plays the jab. A second press of ZR+Up stops it. | yes |
| `media/rc5-loop-no-warp.gif` | `media-rc5-loop-no-warp` | yoga | RECORDING.md, The loop | rr-6 | 8 s | Same macro, CPU loop set to 2 (X while the cursor is in Macros). ZR+Up and let both passes finish. | The first pass warps once. The second pass does not warp. It continues from where the first pass ended. | yes |
| `media/rc5-chain-play.gif` | `media-rc5-chain-play` | yoga | RECORDING.md, Chains | rr-6 | 9 s | + New chain. Number two recordings, the first of which has a start save. Name it. Set it active for the CPU. ZR+Up. | One warp to macro 1's start save, then recording 1, then recording 2, with no warp between them. | yes |
| `media/rc5-grade-not-true.gif` | `gif-gnt-b` after `gm-utilt-42` (`artifacts/phase23/rc6-combined/rows-docs-gifs`) | xps | RECORDING.md, Combos | strip | 7 s | A save with the CPU at 42% and Player 1 in reach, facing it. A macro from that save: up tilt, then up tilt 41 frames later, a link the strip calls `TRUE 0`. Make combo on it makes Combo 1; drill it. Combo helper On, On-time tolerance 1 frame. Attempt 1: Player 1's Play of the same inputs with the follow-up one frame later, so LINK reads `+1 DROP`. The CPU's Play warps both fighters back to the same save. Attempt 2: up tilt, then up smash with the same input timing (INPUT `+4`, as the reference). | Attempt 1 still counts (the bar does not require TRUE). Attempt 2 drops, and the bar says the move was wrong. | yes |
| `media/rc5-make-combo.gif` | `gif-make-combo` (`artifacts/phase23/rc6-combined/rows-docs-gifs`) | xps | RECORDING.md, Combos | rr-6 | 10 s | Drill some other combo first, so the bar is showing it. The macro's start save must have Player 1 in reach of the CPU (the `mk6` fixture: x −12; at x −30 every jab misses and Make combo writes nothing). On the macro's screen, Make combo. Then the Recording page, Down to Combos & Sequences. | That macro plays once on Player 1 from its own start save. A new combo named Combo N appears in the hot five, made from that macro. The drilled combo is still the drill when it finishes. | replaced by `rc6-make-combo.gif` (2026-11-10) |
| `media/rc5-crop-combo.gif` | `media-rc5-crop-combo` | yoga | RECORDING.md, Combos | rr-6 | 7 s | Open a combo of at least two links. Crop. Move a handle with L or R. A saves. | The handle screen is open. A writes a new combo. The original combo is still in the list. | replaced by `rc6-crop-combo.gif` (2026-11-10) |
| `media/rc5-popup.gif` | `media-rc5-popup` | yoga | RECORDING.md, The pop-up | rr-6 | 8 s | In the match, ZR+Right. Cursor on a save that is not the bound save. Down onto a macro. B. | The window over the match. The title reads Recording (Player 1). Two stacked sections, four rows under each header. The bound save's name is green. The cursor is the dark highlight bar. B closes it and the bound save is unchanged. | yes |
| `media/rc5-recording-page.gif` | `media-rc5-recording-page` | yoga | RECORDING.md, The Recording page | rr-6 | 8 s | Open the menu, go to Recording. ZR once. Then ZR back. | Player view shows saves and macros, and Combos & Sequences in the left column. CPU view hides Combos & Sequences. The bound save's name is green in both. | yes |
| `media/rc5-macro-screen.gif` | `media-rc5-macro-screen` | yoga | RECORDING.md, A file's own screen | rr-6 | 6 s | On the Recording page, A on a macro that has a start save. | Breadcrumb reads Recording (Player view), then Save states & Macros, then the name. Start save shows the orange floppy and On. Set active is one row for that role. | replaced by `rc6-macro-set-active.gif` (2026-11-10) |
| `media/rc5-fighter-mismatch.gif` | `media-rc5-fighter-mismatch` | yoga | RECORDING.md, Fighter Mismatch | rr-6 | 7 s | CPU is Mario. Active chain whose first macro was recorded on a different fighter (change P1 to Pikachu, record a jab, change P1 back to Mario, put that recording first in a chain, set the chain active for the CPU). ZR+Up. | The Fighter Mismatch rows open, cursor on Keep. Keep plays the chain on Mario. There is no second prompt when a later macro plays. | yes |
| `media/rc5-migration.gif` | `gif-migration` (`artifacts/phase23/rc6-combined/rows-docs-gifs`) | xps | RECORDING.md, Migration | first boot of a card from before the library | 8 s | Before boot, the card holds the old `save_states.json` with saves in indexes 1 and 2, the Save and Load chords on index 2, and `macros/selected.txt` naming `oldjab` (cov-rec's `boot/` card). Boot into Training. ZR+Right for the pop-up, Right to MACROS, B. ZR+Left. | SAVE STATES lists `Slot 2 (YYYY-MM-DD)` and `Slot 1 (YYYY-MM-DD)`, and Slot 2 is the bound save. MACROS lists `Old jab` as Player 1's active macro. Play runs it from where Player 1 stands; there is no warp. | yes |

rc.5 has no sequence-drill row because rc.5 did not drill sequences. rc.6 drills them, and the clip is under rc.6
clips above ([RECORDING.md](RECORDING.md), Sequences and the one drill). The migration clip shows what a card updated
from an earlier build carries over: its save indexes and its selected macro. Recording slots were never kept between
boots (`src/model/library/migrate.rs`), so no card reaches the library with a recording slot that becomes
`Slot N (date)`.

The grade-not-true row was measured before the take (runs `rc6-docs-gifs-xps8` and `-xps9`). Plugin macros played by
Player 1's Play chord put the follow-up an exact number of frames after the first press, from the save; Eden's own
inputs drifted by up to 6 frames between two presses. From the shared start (CPU at 0%), jab into jab reads `TRUE 0`
with the second press 21 frames after the first, but any A in jab 1's window is jab 2, so no different move can follow at
that timing. At 0%, up tilt into up tilt first links at a gap of 45 frames, `+15 DROP`, and down tilt into up tilt did
not link at gaps of 19–37 frames. At the CPU's 42%, up tilt into up tilt
reads `TRUE 0` at a gap of 41 frames (three plays) and `+1 DROP` at 42 (two plays), and up smash hits at those gaps.

The take ran on `688c478`, before rc.6's recording fixes, and two things in it differ from rc.6. On that build
*Save last combo* had no row on the Recording page (`Editor::ComboSave` was in no menu table), so the drill was made
with Make combo. A combo made from a macro there had links but no input events (`"events": []`): the strip read Player
1's poll before the macro's playback was written into it, so it never saw the macro's presses. Attempt 1 in the clip
therefore counts because its move matches; the on-time tolerance had no event to test. rc.6 has the
*+ Save last combo…* row (`src/model/recording_menu.rs:1584`), and the strip reads the played frame, so Make combo
stores the macro's events (`Sticks::after_recorder`, `src/model/input_delay.rs:76`; cov-rec row `cov-make-combo`,
every link has events).

The redesign mockups named `docs/decisions/RECORDING-REDESIGN-*.png` are not in this tree. They are not a substitute
for the rows above. The rc.3 clip `media/frame-strip.gif` remains the earlier take of the strip. The RC10 clip
`media/menu-recording.gif` remains the slot recording.

Clips in the sections below are already in the tree. They have no row in `eden/recording-redesign/rows.json`.

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
| `cycle-badges.gif` | Fighters & Stages › Character Cycles: the game's stock icons with the order badges (red for Player 1, blue for the CPU); ZR switches role. 9.5 s. Replaced on 2026-11-10 by `rc6-cycle-switch.gif`, which also shows the chord switching in that order; it remains in the history up to `da26d632`. |
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
