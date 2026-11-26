# Current release

<!-- Whoever ships a release: change the `Current release:` line below first, then the README's top and the Status section
     of docs/TECHNICAL.md, and put the same release in Cargo.toml's version (0.2.0-rc.N, with Cargo.lock's own entry) in the
     release's name commit. The docs checks (tests/docs_gates.rs) fail until they all name the same release. The steps are
     in docs/BUILD.md, "Release checklist". -->

Current release: rc.15

Status: **rc.15, whose newest build is install 6** (commit `5df3d9dc`, installed on the project's test console on
2026-11-26).

- **Installed:** the file on the card was read back and matches the build.
- **Eden:** install 6 is in testing. At installation, two Training boots were running on its diagnostic build;
  the installed release file had no smoke verdict. The build before its last fix (`eb39d6bc`) passed its Play-start comparison with install 5, its record,
  old-macro, snap and browse checks, and part of the diagnostic checks. The Play-start result and the rename result
  carry over: rename passed by typing in Eden's keyboard window, not by pad. The rest of the build is install 5,
  which had a short smoke boot and a one-player waiting-room boot.
- **On the console:** no report on install 6 yet. Three reports on install 5 are on record. The clip editor's
  "In is mid-action" refusal after a snap (2026-11-25) has its fix in install 6. A fighter placed off stage on a
  save that stays on the ground once it has landed, and Guard the Edge's starting spot varying between rounds
  (both 2026-11-26), are not fixed. Known issues below has each. They are reports of what the operator saw, not
  tests of a build; the reports on the earlier builds are in the evidence page.

What ran, on which build, what did not, and every report from the console: [Install evidence](EVIDENCE-rc15-installs.md).
What each build changed:
[rc.15 release notes](RELEASE-NOTES-rc15.md). This page says which release is current. The [README](../README.md) and
the [technical notes](TECHNICAL.md#status) follow it.

## Which build is mine?

Open the menu (B + D-pad Up) and read the end of the key hints at the foot of the page. rc.15 reads
`0.2.0-rc.15 · rc15` and then the commit it was built from. That commit tells the rc.15 builds apart:

| The tag ends with | Build | Installed |
| --- | --- | --- |
| `5df3d9dc` | Install 6 | 2026-11-26 |
| `b7a382e4` | Install 5 | 2026-11-25 |
| `a945b0c1` | Install 4 | 2026-11-25 |
| `e7667f26` | Install 3 | 2026-11-24 |
| `71158421` | Install 2 | 2026-11-24 |
| `152629cd` | Install 1 | 2026-11-23 |

Builds before rc.14 start with `0.2.0-rc.6` whatever their release, so for them read the second part (`rc13`,
`rc12`, ...).
More: [Getting started › Which build you have](guide/getting-started.md#which-build-you-have).

## What's new in rc.15

Install 6 (2026-11-26), in short. The full list, and installs 1 to 5, are in the
[rc.15 release notes](RELEASE-NOTES-rc15.md#install-6).

- **Y on In makes a match capture previewable** in the Replay window.
- **A free playhead in the clip editor:** Left and Right browse the take, Y goes to the nearest move, L and R cut
  there.
- **The fighters are shown while you scrub,** from positions kept in a file beside each macro. An old macro gets
  them when you play it once.
  [Replay window](menu/recording.md#replay-window)

From install 5 (2026-11-25):

- **Record works while an AI plays Player 1.**
  [When the CPU pilot plays Player 1](guide/controls.md#when-the-cpu-pilot-plays-player-1)
- **Delete for replay captures.** [Recordings](menu/recording.md#recordings)
- **Play starts without reading the whole library,** and has a Playback delay option.
  [Playback delay](menu/recording.md#playback-delay)
- **Change placement:** move where a save state puts the fighters, with the crosshair, on the save itself.
  [Change placement](menu/recording.md#change-placement)
- **Guard the Edge and Juggler start from the floor you stand on,** and Guard the Edge has two new options.
  [Guard the Edge and Juggler](menu/fighters-and-stages.md#guard-the-edge-and-juggler)
- **Eggs-ercise's off-stage zone follows the stage.** [Eggs-ercise](menu/fighters-and-stages.md#eggs-ercise)
- **B stops a preview,** the pop-up shows its chords, the editor's marks name the form of a move, and grabs, throws
  and edge attacks are named in combo files. [rc.15 release notes](RELEASE-NOTES-rc15.md#install-5)

From install 4 (2026-11-25):

- **Drills and combos read moves the same way for every fighter.** A step is one action: its release, hold, special
  directions and travel belong to it. [How a combo is graded](guide/recording.md#how-a-combo-is-graded)
- **The clip editor is a strip over the game,** with move marks, a snap and a preview on the CPU or Player 1.
  [Replay window](menu/recording.md#replay-window)
- **Recordings.** Play a Vault or saved capture back, or make a macro, combo or drill from it.
  [Recordings](menu/recording.md#recordings)
- **Trim** for Record and for each macro and drill, and a playback indicator.
  [Trim trailing neutral](menu/recording.md#trim-trailing-neutral)
- **Scroll the input display back** with R3 and D-pad Left or Right. [Frame strip](guide/frame-strip.md)
- **Move names** and **Match angle and deflection**, both Off by default.
  [Move names](menu/recording.md#move-names)

From the first rc.15 build (2026-11-15):

- **One clip editor.** Crop a macro, combo or drill with the Replay window's editor. Save the kept part as a new file;
  macros can preview on the CPU, while combos and drills crop whole links or steps.
  [Recording](menu/recording.md)
- **Input drills.** Record a drill directly, or make one from a macro with no hits. Practise and grade its input steps,
  with up to 16 steps per drill. [Recording guide](guide/recording.md)
- **Article hurtboxes.** A separate checklist option draws supported fighter articles and items, including Pikmin, Cypher and Gyro.
  [Hitbox Visualization](menu/display-and-controls.md#hitbox-visualization)
- **Colour legend.** Hitbox Visualization's checklist shows the colours for its current normal or Frame data scheme.
  [Hitbox Visualization](menu/display-and-controls.md#hitbox-visualization)

## Installing

Copy `libtraining_rooms.nro`, `libnro_hook.nro` and `libparam_hook.nro` into
`atmosphere/contents/01006A800016E000/romfs/skyline/plugins/` on the SD card, and remove `libtraining_modpack.nro` if it
is there. The steps, and what to expect on the first start: [Getting started](guide/getting-started.md).

## Known issues

- **Fixed in install 6: "In is mid-action" after a snap in a match capture.** On install 5 (install 4 uses the same snap code), plain Y
  snaps In to a mark, and if that frame is still mid-action, preview and save are refused. The way round there:
  with In selected, hold ZL or ZR (Z on a GameCube pad) and press Y, release the modifier, then press X to preview
  or A to save. In install 6, Y resolves the move itself (`dbded2d1`). In Eden on the build before install 6's last
  fix (`eb39d6bc`), Y moved In to the attack's start, and X then previewed. No report from the console on it yet.
- **Placement on a save, reported on install 5:** a fighter switched to Off stage floats, but once it is moved into
  the ground it lands and then stays on the ground as the crosshair moves. Switch On stage and Off stage again to put
  it right. Neither install 5 nor install 6 fixes it. A fix is planned for the following build; there is no
  confirmed in-game fix yet.
- **Guard the Edge, reported on install 5:** the fighter sometimes starts slightly off the chosen spot, lower on one
  round and higher on the next. Not fixed in install 5 or in install 6; a fix is planned for the following build.
  No cause is established.
  [The operator's reports](EVIDENCE-rc15-installs.md#the-operator-s-reports-from-the-console)
- **Test coverage:** install 6 has no report from the console yet, and its own Eden checks are in testing.
  [Install evidence](EVIDENCE-rc15-installs.md#install-6-5df3d9dc)
  Install 5 has three reports from console play, not a full hardware verdict. Eden covered parts of Training in its
  first boot and a one-player Local Wireless waiting room in its second.
  [Install evidence](EVIDENCE-rc15-installs.md#install-5-b7a382e4)
- **Playback was slow to start on install 4:** a second or more (the operator's report). Install 5 changes how Play
  starts. Timed in Eden, not on the console, install 5 started a 3-second macro 9 frames after the press, a
  30-second one 10 or 11, and a ten-minute one 27.
  [Install evidence](EVIDENCE-rc15-installs.md#install-6-5df3d9dc)
- **Not covered by install 4's final Eden session, nor by install 5's smoke boot:** the editor in the waiting room; the Quick Play practice pick, new pick
  and rematch; the airborne entry of a steered special; the deflection half of Match angle and deflection; ledge
  attacks; the Trim setting's migration; and the CPU's playback indicator.
  [Install evidence](EVIDENCE-rc15-installs.md#install-4-a945b0c1)
- **L + R + A pressed one button at a time.** A button pressed before the chord and still held when it completes
  belongs to the reset, and its drill result is taken back, so a result can show for a moment first.
  [Input drills](guide/recording.md#input-drills)
- **Fighter loading.** A fighter that doesn't fit in Ready Fighters loads when you swap to it, so that swap takes a
  moment ([Fighter cycles](guide/fighter-cycles.md#what-the-loading-rows-do)).
- **Limits kept from the Training Modpack** are listed in
  [Differences from the fork](DIFFERENCES-FROM-THE-FORK.md#kept-from-m5q).
- **Waiting rooms** have their own limits and refusals:
  [What is different, or refused](guide/waiting-rooms.md#what-is-different-or-refused).

Something else? [Troubleshooting](guide/troubleshooting.md) says what to check and what to send.

## Earlier releases

- The builds of rc.15, install by install: [rc.15 release notes](RELEASE-NOTES-rc15.md).
- Release notes in this folder: [rc.5](RELEASE-NOTES-rc5.md), [rc.6](RELEASE-NOTES-rc6.md),
  [rc.8](RELEASE-NOTES-rc8.md) and [rc.9](RELEASE-NOTES-rc9.md).
- On their release pages: [rc.12](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.12),
  [rc.13](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.13)
  and [rc.14](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.14).
- rc.7, rc.10 and rc.11 have no notes: see [Status](TECHNICAL.md#status).
