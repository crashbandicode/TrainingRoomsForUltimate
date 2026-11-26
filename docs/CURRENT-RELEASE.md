# Current release

<!-- Whoever ships a release: change the `Current release:` line below first, then the README's top and the Status section
     of docs/TECHNICAL.md, and put the same release in Cargo.toml's version (0.2.0-rc.N, with Cargo.lock's own entry) in the
     release's name commit. The docs checks (tests/docs_gates.rs) fail until they all name the same release. The steps are
     in docs/BUILD.md, "Release checklist". -->

Current release: rc.15

Status: **rc.15, whose newest build is install 5** (commit `b7a382e4`, installed on the project's test console on
2026-11-25).

- **Installed:** the file on the card was read back and matches the build.
- **Tested in Eden, in part:** a short smoke boot on this exact build ran with no crash. Guard the Edge and Juggler
  opened their set-up on the Training stage, and Guard the Edge also from the island and on Battlefield. Eggs-ercise
  had its off-stage zone, the recording picker's Off stage switch moved a fighter, a take played and the clip editor
  previewed. A second boot opened the menu, and Guard the Edge's set-up, in a one-player waiting room. Neither
  drill was played through, no placement was saved, no second player joined a room, and most of the build's other
  new items were not touched.
- **On the console:** three reports so far. On 2026-11-25 the operator reported that the clip editor kept refusing
  with "In is mid-action" after a snap to the move start. On 2026-11-26 they reported that a fighter placed off stage
  on a save stays on the ground once it has landed, and that Guard the Edge's starting spot varies between rounds.
  Known issues below has each. They are reports of what they saw, not tests of the build; the reports on the earlier
  builds are in the evidence page.

What ran, on which build, what did not, and every report from the console: [Install evidence](EVIDENCE-rc15-installs.md).
What each build changed:
[rc.15 release notes](RELEASE-NOTES-rc15.md). This page says which release is current. The [README](../README.md) and
the [technical notes](TECHNICAL.md#status) follow it.

## Which build is mine?

Open the menu (B + D-pad Up) and read the end of the key hints at the foot of the page. rc.15 reads
`0.2.0-rc.15 · rc15` and then the commit it was built from. That commit tells the rc.15 builds apart:

| The tag ends with | Build | Installed |
| --- | --- | --- |
| `b7a382e4` | Install 5 | 2026-11-25 |
| `a945b0c1` | Install 4 | 2026-11-25 |
| `e7667f26` | Install 3 | 2026-11-24 |
| `71158421` | Install 2 | 2026-11-24 |
| `152629cd` | Install 1 | 2026-11-23 |

Builds before rc.14 start with `0.2.0-rc.6` whatever their release, so for them read the second part (`rc13`,
`rc12`, ...).
More: [Getting started › Which build you have](guide/getting-started.md#which-build-you-have).

## What's new in rc.15

Install 5 (2026-11-25), in short. The full list, and installs 1 to 4, are in the
[rc.15 release notes](RELEASE-NOTES-rc15.md#install-5).

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

- **"In is mid-action" after a snap in a match capture.** Plain Y snaps In to a mark. If that frame is still
  mid-action, preview and save are refused. With In selected, hold ZL or ZR (Z on a GameCube pad) and press Y to snap to a
  move start. Release the modifier, then press X to preview or A to save. This refusal does not apply to Crop on
  saved files. Reported on install 5 and reproduced in a host test; install 4 uses the same snap code.
  A fix is merged (`dbded2d1`) for the build after install 5; it is not on the card, and it has not run in a game.
- **Placement on a save, reported on install 5:** a fighter switched to Off stage floats, but once it is moved into
  the ground it lands and then stays on the ground as the crosshair moves. Switch On stage and Off stage again to put
  it right. Not fixed; no cause is established.
- **Guard the Edge, reported on install 5:** the fighter sometimes starts slightly off the chosen spot, lower on one
  round and higher on the next. Not fixed; no cause is established.
  [The operator's reports](EVIDENCE-rc15-installs.md#the-operator-s-reports-from-the-console)
- **Install 5 test coverage:** three reports from console play are on record, not a full hardware verdict. Eden covered
  parts of Training in the first boot and a one-player Local Wireless waiting room in the second.
  [Install evidence](EVIDENCE-rc15-installs.md#install-5-b7a382e4)
- **Playback was slow to start on install 4:** a second or more (the operator's report). Install 5 changes how Play
  starts; the start has not been timed in a game.
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
