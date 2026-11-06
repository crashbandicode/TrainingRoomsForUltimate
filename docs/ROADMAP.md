# Roadmap

Deferred work after 0.2.0-rc.3. Folded from the former `NEXT-VERSION.md` (removed after v0.2.0-rc.1; see that tag)
on 2026-11-05; no item was dropped.

## Next batch

- **Combo helper** (decision COMBO-HELPER, option A; `SPEC-input-07`, branch `p23/combo-helper`): record a combo, save
  it with a name from the Switch keyboard, and have each attempt graded link by link in a bar under the frame strip.
  Built and host-tested; its Eden rows run in the next grouped session.
- **Mii editor**: the Mii editor batch, tested in the same grouped session.

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
| F9 helper promotion, fixed-seat ownership, hat/item cutover | Removed from the RC; M5q count behaviour kept. Partial count/body/HUD/reset evidence only. |
| Hurtboxes (HB1) | Phase 3 work; see the hitbox visualizer's current state in the README. |
| Ten hot fighters plus five separate hot hats | Proven capacity behaviour kept; no separate 15-body feature. |
| Saved/replay Mii bust provider (Q15) | No new provider or hook; the native saved-Mii selection stays. |
| Frame/status text backgrounds (UI22) | Reverted to the pre-UI22 state. The input log and its light-background contrast are gone in rc.3 (the frame strip has its own backdrop). |
| Further latency work (F6) | M5q cold-switch hitches kept; the zero-Unmapped and zero-allocation fixes stay. |
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

- Combo: switch P1 to the combo's recorded fighter, with the same Switch/Keep prompt as a named macro.
- Lobby Items (N11): now in rc.5 (`p23/lobby-items`, primary order 2026-11-05 18:58Z); see the N11 section below.
- Ten hot fighters plus five hot hats (five while one CPU; operator decision HOT-SLOTS-CEILING); preloading starts on
  entering Training; no hitch on the first switch to a fighter that is not preloaded yet (F6).
- Replay/saved-Mii busts (Q15): not built; the Mii rows stay text (operator 2026-11-05; the game makes a Mii picture
  only through its live selector).
- Backgrounds behind the plugin's notices and status texts (UI22).
- Timer-recording truncation: root cause and fix.
- Then F9 helper promotion, fixed seats and the hat/item cutover.

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
