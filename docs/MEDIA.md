# Media

Every GIF in [`media/`](media/) is actual Eden footage: whole screen, 1280×720, one clip per behaviour, under 10 s,
game-speed timing kept, trimmed at the ends (any cut inside a clip is listed with it). Frames were reviewed (start,
action, result) before a clip was kept. The raw takes stay outside git with their runs.

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
- `rc5-make-combo.gif`: run `rc6-docs-gifs-xps7` on the XPS, 2026-11-08 17:46Z, the same diag NRO, fixture `mk6` (the
  rc.5 `mk` fixture with its start save moved into reach, `make_mk6.py`). After the clip the row checked the card:
  `combo-2` written and hot, `combo-1` still the drill. The take is trimmed to 0.7–10.4 s and re-encoded the same way:
  6.1 MB, 9.7 s.
- `rc5-grade-not-true.gif`: run `rc6-docs-gifs-xps9` on the XPS, 2026-11-08 18:58Z, the same diag NRO, row
  `gif-gnt-f` (the plan's `gif-gnt-b`, renamed for the rerun). The strip logged the three links the row needs: Make combo
  `TRUE 0` (input +4), attempt 1 `+1 DROP` (input +5), attempt 2 up tilt into up smash (input +4). The take is trimmed
  at 0.8 s. It was re-encoded with one shared palette of 96 colours and no dither, keeping every second frame with the
  time of both: 9.9 MB, 7.1 s.

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
