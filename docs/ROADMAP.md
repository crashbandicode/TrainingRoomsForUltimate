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
| Setup Library / SaveSetup | Screen entries and transactions disabled; saved files left intact. |
| Edit placement; switching the CPU to a recorded ordinary fighter (replay window, named macros) | Closed (`engine::macro_ui::workflows::RELEASE`). The replay window and exact-Mii named macros ship again since 2026-11-05 (operator). |
| Saved/replay Mii bust provider (Q15) | No new provider or hook; the native saved-Mii selection stays. |
| Frame/status text backgrounds (UI22) | Reverted to the pre-UI22 state. The input log and its light-background contrast are gone in rc.3 (the frame strip has its own backdrop). |
| Named-macro library/preview extensions (F8 design C) | Exposed operations narrowed; basic capture and synchronization kept. |
| State-destructor alternative | Never integrated. |
| Further latency work (F6) | M5q cold-switch hitches kept; the zero-Unmapped and zero-allocation fixes stay. |
| Hitbox sphere replacement | Done for fighter spheres in rc.3 (own discs, never refused). Projectiles keep the game's effect spheres. |
| Remaining feature/edge-case inventory | Kept for later planning. |

Timer-recording truncation is an observed TrainingRooms limitation with no established M5q equivalence; it is not
called an inherited bug. (The input-log contrast went with the input log in rc.3.)

## Phase 2 N11: Lobby Items (operator order)

TrainingRooms only. A *Lobby Items* toggle on Fighters & Stages › Stages, after Stage Hazards, default On (native
behaviour), saved like every setting. Off stops waiting-room practice item spawning at the native scene rule and
removes an existing item once through the native removal; no per-frame sweep. Training, character select and online
matches are untouched. Acceptance: host tests for policy, scene, persistence and import; 64-byte guards; zero hook
allocations; one Eden batch (Off no-spawn window, existing-item removal, On spawn return, zero Unmapped, both consoles
`desync_seen` 0 in an online smoke with the row Off).
