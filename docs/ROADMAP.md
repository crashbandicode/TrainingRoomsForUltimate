# Roadmap

Deferred work after 0.2.0-rc.3. Folded from the former `NEXT-VERSION.md` (removed after v0.2.0-rc.1; see that tag)
on 2026-11-05; no item was dropped.

## Done: 0.2.0-rc.6 (release date: TODO-FINAL-PASS)

rc.5 was withdrawn before it reached players. rc.6 ships only when every feature since rc.3 has passed in Eden on the
rc.6 build; the final pass is TODO-FINAL-PASS. The player notes are [RELEASE-NOTES-rc6.md](RELEASE-NOTES-rc6.md).

| Item | Where | State |
| --- | --- | --- |
| Sequence drilling: the sequence bar, the joining link, streak and best in the file | `p23/rc6-seq-drill` | Eden `rc6-seq-diag1` 18/18, `rc6-seq-rel1` 5/5 |
| *+ Save last combo…* at the top of COMBOS (decision SAVE-LAST-COMBO) | `p23/rc6-cov-rec` | Eden `rc6-covrec-diag10` 12/12; a local-wireless room `covrest-pair-y1` |
| The r8 recording pop-up: role and page tabs, counts, CONNECTIONS column, lines, badges, footer | `p23/rc6-cov-rec` | Eden `rc6-covrec-diag10`, `-rel7` 3/3 |
| Recording fixes: the bound save drives Save, Load and Auto-load; CONNECTIONS; Make combo events; mash; setup restore; Edit placement; Edit and Delete written | `p23/rc6-cov-rec`, `p23/rc6-seq-drill` | Eden `rc6-covrec-diag3` 32/35 then `-diag10`; `rc6-seq-diag1` |
| Hot fighters at Training entry (start-up windows, hats at entry), the group-receipt and back-to-the-original fixes | `p23/rc6-hot-slots` | Eden `rc6-hot-xps3` 21/21; console F6 on the test card |
| Group speed (decision GROUP-SPEED): warm exit from a Mii, async Mii build, groups built in the background | `p23/rc6-hot-slots` | Eden `rc6-hot-xps6` 5/5, `rc6-hot-xps8`, `rc6-hot-xps13` 13/14 (the 14th is the fallback allocation, fixed in integration). TODO-PROOF: Mii, Ice Climbers and Pokémon Trainer/Pyra-Mythra groups built in the background |
| A cycle change makes things hot (decision CYCLE-CHANGE-HOT) | `p23/rc6-hot-slots` | Eden `rc6-hot-xps13`. TODO-PROOF: the reload pause (23 frames, target 4) |
| No. of CPUs seats: the first raise, a Mii seat's configuration, the proof of a promoted seat, the portrait race, the seats after a cycle change | `p23/rc6-cov-rest` | Eden `covrest-10`, `-14`, `covrest-yoga-06` |
| The "Same as ‹name›, nothing new saved" notice (decision MII-SAME-NOTICE) | `p23/rc6-cov-rest` | Eden `covrest-07` |
| The exact Mii on a recorded macro (#54), the Replay window's Mii track (#68), the notice plates (#64) | `p23/rc6-cov-rest` | Eden `covrest-yoga-04`, `covrest-07`; `rc6-int-smoke2` for the plates |
| No heap in the CPU-as-P1 fallback's hooks | `p23/rc6-integration` `6d6994e` | Eden `rc6-int-smoke2` 11/11 |
| rc.5 features first proven in Eden: P2 L+R+A and Mii swaps, respawn, intro once, the online flag, the Lobby Items online smoke, the timer capture through sudden death, the Mii editor's interrupt and resume, combos in a local-wireless room | the lane branches | Eden `covrest-16`, `-04`, `-13`/`-15`, `covrest-pair-02`/`-03`, `rc6-int-vs3`, `covrest-pair-y3`/`-y1`. TODO-PROOF: Lobby Items li-01..03 (no item spawns in Eden's host-only room) |

The once-seen game-thread stop of `covrest-yoga-05` (Eden only, a diag reload path) is noted as open and unreproduced.

## Done: 0.2.0-rc.3 (2026-11-05)

Four additions on top of rc.2 (candidate `p23/rc3-candidate` `3c8f5bc`; see
[Differences from the fork](DIFFERENCES-FROM-THE-FORK.md#added-in-rc3)):

- **Frame strip** (decision INPUT-VIZ, option A): the input display is a strip of the last 60 frames of Player 1 and
  the CPU, with how each follow-up linked and the frame advantage. It replaces the 15-row input log.
- **Change Stage**: a row under Fighters & Stages › Stages that goes to the picked stage when the menu closes, in
  Training and in waiting-room practice. rc.2's Direct stage pick, which did nothing, works again.
- **Menu navigation**: ZL + Up/Down jumps sections; the menu reopens where it was closed; ZR no longer resets a page
  (decision SETTINGS-PERSIST), and each page ends with a confirmed *Reset page to defaults*.
- **Own hitbox discs** (decision HB-OWN-ORBS, option 1): every fighter sphere is drawn as the plugin's own disc and
  is never refused. Projectiles keep the game's effect spheres. Known limit: the discs trail the camera by one to two
  frames while it pans.

## Done: 0.2.0-rc.2 (2026-11-05)

The three operator decisions of 2026-11-05 that rc.1 did not carry (code at `p23/rc2-pause-helper` `9b28ca0`; see
[Differences from the fork](DIFFERENCES-FROM-THE-FORK.md#added-in-rc2)):

- **Pause while a helper fighter loads** (decision PAUSE-HELPER, option a): when the pause opens while a preloaded
  helper entry has no actor yet, the entry is retired through the game's single-slot retire, creation waits until the
  pause exits, and the helper is created again. In rc.1 this case made the game write through a null pointer on Eden.
- **Solo Popo marks** (decision SOLO-S-PLACEMENT): the marks sit on the icon's lower corners, never over the name.
- **Kirby hat requests end** (decision HAT-REQUEST-END, option A): a request ages every game frame and always ends,
  and a role that stops being Kirby ends its request.

## Deferred (operator decision 2026-11-03)

The RC is M5q parity plus the new UI and clean MIT provenance; this list is not an RC gate.

| Deferred work | State at rc.1 |
| --- | --- |
| F9 helper promotion, fixed-seat ownership, hat/item cutover | Done in rc.5 (`p23/f9`): fixed CPU seats, slow mode on live entries, hat and item cutover, and the option-1 same-close raise from 1 to 3. |
| Hurtboxes (HB1) | Phase 3 work; see the hitbox visualizer's current state in the README. |
| Ten hot fighters plus five separate hot hats | Done in rc.5 (`p23/hot-slots`): ten hot fighters and five hot hats, hats off the fighter budget. No separate 15-body feature. |
| Saved/replay Mii bust provider (Q15) | No new provider or hook; the native saved-Mii selection stays. |
| Frame/status text backgrounds (UI22) | Done in rc.5: a dark plate behind each notice text (header, short body, long body). The input-log plates stay reverted. The frame strip keeps its own backdrop. |
| Further latency work (F6) | rc.6, console test of 2026-11-08 (release `1e6c6a0e`): Player 1's first switch 53 ms and hats 7–23 ms, the first included, with no hitch. The slow group switches it found became the rc.6 group-speed work (decision GROUP-SPEED). |
| Hitbox sphere replacement | Done for fighter spheres in rc.3 (own discs, never refused). Projectiles keep the game's effect spheres. |
| Remaining feature/edge-case inventory | Kept for later planning. |

**Timer-recording truncation: fixed 2026-11-05** (`p23/timer-recording`; `docs/DECISIONS.md` DEFERRED-LIST item 7).
This reverses the earlier note that called it an observed limitation with no established M5q equivalence.

- **The bug:** a timer match that went to sudden death saved its live replay capture cut at the end of regular time,
  marked truncated (Eden p4c-03: native counter 4449, both tracks 3600 frames).
- **The cause:**
  - Fighters are sampled only during READY…GO play. From TIME! to the sudden-death GO, the native recorder keeps
    counting with no sample.
  - The capture worker sealed every track at the first gap.
- **It is inherited:** M5q's `live_capture.rs` gap rule and `live_replay.rs` `is_ready_go` gate are the same code.
- **The fix:**
  - A gap during which every fighter stopped together is now a `CaptureBreak`, which keeps the true counters. The
    capture continues through sudden death, complete.
  - The `.smd` timeline marks it with a global `Gap` event, and the editor cache keeps the break.
  - The clip editor refuses a selection that crosses it.
- **Still sealed:** a gap while another fighter played, a backwards counter, a lone track, an invalid sample, and more
  than 16 breaks. (The input-log contrast went with the input log in rc.3.)

The named-macro library and preview extensions (F8 design C) are done in rc.5 (`p23/replay-window`: Setup library,
Edit placement, Replay window, and a named macro recorded on another fighter with Switch/Keep). The state-destructor
alternative was dropped by the operator.

## Next (operator 2026-11-05, DEFERRED-LIST)

In this order (`docs/DECISIONS.md` DEFERRED-LIST on `review/menu-proposal` `6d19902`):

- Combo: switch P1 to the combo's recorded fighter, with the same Switch/Keep prompt as a named macro. Done in rc.5 (`p23/combo-switch`).
- Lobby Items (N11): now in rc.5 (`p23/lobby-items`, primary order 2026-11-05 18:58Z); see the N11 section below.
- Ten hot fighters plus five hot hats (five while one CPU; operator decision HOT-SLOTS-CEILING): done in rc.5
  (`p23/hot-slots`). Preloading starts on entering Training. Its Eden rows first ran in rc.6 (`rc6-hot-xps3`), which
  fixed the hats at entry and the group exchanges from a start-up window; the first switch on the console is in the
  F6 row above.
- Replay/saved-Mii busts (Q15): not built; the Mii rows stay text (operator 2026-11-05; the game makes a Mii picture
  only through its live selector).
- Backgrounds behind the plugin's notices and status texts (UI22): done in rc.5. A dark plate sits behind each notice text. The frame strip keeps its own backdrop.
- Timer-recording truncation: fixed in rc.5 (`p23/timer-recording`). See the section above.
- F9 helper promotion, fixed seats and the hat/item cutover: done in rc.5 (`p23/f9`).

## Phase 2 N11: Lobby Items (operator order)

TrainingRooms only. A *Lobby Items* toggle on Fighters & Stages › Stages, after Stage Hazards, default On (native
behaviour), saved like every setting. Off stops waiting-room practice item spawning at the native scene rule and
removes an existing item once through the native removal; no per-frame sweep. Training, character select and online
matches are untouched. Acceptance: host tests for policy, scene, persistence and import; 64-byte guards; zero hook
allocations; one Eden batch (Off no-spawn window, existing-item removal, On spawn return, zero Unmapped, both consoles
`desync_seen` 0 in an online smoke with the row Off).

**Implemented 2026-11-05 (`p23/lobby-items`), in 0.2.0-rc.5; its Eden rows run in the rc.5 grouped session.**

- **Spawning:** one inline hook on the item spawner's own switch test (`main+0x15d9a80`, the `cbz` after
  `ldrb w8,[c,#0x30]`). In a qualified waiting scene with the row Off, the native takes its own off path; elsewhere the
  hook leaves everything alone. Nothing is written to the game, so nothing needs restoring.
- **Existing items:** removed once per Off edge through `ItemManager::remove_item_from_id`, from Player 1's frame, and
  only after the practice ItemGate allows it.
- **Code:** the policy is in `src/model/lobby_items.rs`, the runtime in `src/runtime/lobby/lobby_items.rs`, and the
  seam rows in `rows_lobby_items.rs` (`tools/gen_rows_lobby_items.py`).
- **Design and static facts:** the P-N11 section of the lane report.
