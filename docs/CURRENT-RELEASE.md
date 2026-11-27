# Current release

<!-- Whoever ships a release: change the `Current release:` line below first, then the README's top and the Status section
     of docs/TECHNICAL.md, and put the same release in Cargo.toml's version (0.2.0-rc.N, with Cargo.lock's own entry) in the
     release's name commit. The docs checks (tests/docs_gates.rs) fail until they all name the same release. The steps are
     in docs/BUILD.md, "Release checklist". -->

Current release: rc.15

Status: **rc.15, whose newest build is install 6** (commit `5df3d9dc`, installed on the project's test console on
2026-11-26).

A **preview build** (`cc71537c`) is staged for the card and not installed. It adds recorded moves to scrubbing and
fixes several of install 6's issues; two defects found in it are being fixed.
[Preview build](RELEASE-NOTES-rc15.md#preview-build)

| Check | Install 6 |
| --- | --- |
| Installed | The file on the card was read back and matches the build |
| Eden, on install 6's diagnostic build | Passed so far: record and hold, release, the old macro, a crop, delete, a take on the CPU, the playhead rows, the snap rows up to their preview, and the menu's stack check. Found: the three issues in the second table under [Known issues](#known-issues) |
| Eden, on the installed release file | No smoke boot yet, and no waiting-room pair |
| Eden, on `eb39d6bc` (before the last fix) | Record, old macros, snap, browse and part of the diagnostic checks passed |
| Results carried over | Play-start comparison with install 5; rename through Eden's keyboard window, not by pad |
| The rest of the build | Is install 5, which had a short smoke boot and a one-player waiting-room boot |
| On the console | On install 6, the operator saw the fighters change position while scrubbing but not show the moves. See [Install evidence](EVIDENCE-rc15-installs.md#install-6-5df3d9dc); the three reports from install 5 are under [Known issues](#known-issues). |

What ran, on which build, what did not, and every report from the console: [Install evidence](EVIDENCE-rc15-installs.md).
What each build changed:
[rc.15 release notes](RELEASE-NOTES-rc15.md). This page says which release is current. The [README](../README.md) and
the [technical notes](TECHNICAL.md#status) follow it.

## Which build is mine?

Open the menu (B + D-pad Up) and read the end of the key hints at the foot of the page. rc.15 reads
`0.2.0-rc.15 · rc15` and then the commit it was built from. That commit tells the rc.15 builds apart:

| The tag ends with | Build | Installed |
| --- | --- | --- |
| `cc71537c` | Preview build | Not installed; staged 2026-11-27 |
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

| Issue on install 5 | What happens | Workaround | Install 6 |
| --- | --- | --- | --- |
| Match capture: "In is mid-action" after Y | Plain Y can leave In mid-action, blocking preview and save. Install 4 has the same snap code. | With In selected, hold ZL or ZR (Z on GameCube) and press Y. **Release the modifier**, then press X to preview or A to save. | **Fixed:** Y resolves the move (`dbded2d1`). [Snap check on the earlier build](RELEASE-NOTES-rc15.md#y-on-in-makes-a-capture-previewable) |
| Placement on a save | The Off stage fighter floats until it touches ground. It then stays grounded as the crosshair moves. | Switch On stage, then Off stage again. | Open; fix planned for the following build. |
| Guard the Edge | The release point was the same each round. At the in-view start the CPU sometimes air-dodged toward the stage and sometimes fell first. At the other three tested starts its motion or inputs first differed 6 to 9 frames after release. | None. | Open on install 6; unchanged in the preview. |

The reports behind these: [The operator's reports](EVIDENCE-rc15-installs.md#the-operator-s-reports-from-the-console).

| Found in testing install 6 | What happens | Workaround | Status |
| --- | --- | --- | --- |
| Scrub positions can stop at 20 seconds | Two long recordings kept all inputs but only 1,200 frames of positions. The rest still plays; the editor cannot place the fighters there. | None in install 6: Play does not extend a valid positions file saved by Record. | Open; cause read from the code. Fixed in the next build. |
| First X refused after a snap | Seen on a match capture. Before taking a temporary checkpoint, X needs the CPU grounded and idle. The scrub hold can prevent that. | Press L again for In (R for Out) to deselect, let the CPU settle, then press X. | Open; cause read from the code. A macro using its own start save bypasses this checkpoint. |
| A fighter beside a wall is shown off its point | While scrubbing, a recorded point close beside a wall can be free while the fighter's body does not fit there. The game pushes the fighter out, about 5 units in the case seen, and the editor still treats it as held at its point. Seen on a match capture, just left of the end of the Training stage's floor and below it. | None. | Open; accepted as a defect. Fixed in the next build. |
| Scrubbing shows where the fighters were, not their moves | The fighters are moved in space and keep the pose they had. Reported by the operator on install 6. | None. | By design in install 6; recorded poses are in design for the following build. |

The runs behind these: [Install evidence](EVIDENCE-rc15-installs.md#eden-on-install-6-itself-5df3d9dc-merge-17).

| In the preview build (`cc71537c`, not installed) | State |
| --- | --- |
| Scrub positions can stop at 20 seconds | Fixed: a 1,610-frame take kept all its positions |
| First X refused after a snap | Fixed: X plays after a held frame |
| A fighter beside a wall is shown off its point | The editor refuses that point instead; host-tested only |
| Scrubbing shows where the fighters were, not their moves | A matching fighter shows the recorded move on frames that have a pose |
| Placement on a save | Fixed: the Off stage fighter floats again |
| Guard the Edge | Not changed |

| Found in testing the preview build | What happens | Workaround | Status |
| --- | --- | --- | --- |
| Fighter switch times out | After ZL + L the editor waits, then reads "The fighter did not load; the editor keeps the current one". The open editor blocks the roster. After 600 worker passes it stops waiting but leaves the switch pending. In Eden, ZL + L changed Player 1 only after the editor closed. The CPU side was not tested. | None established. | Not fixed in this preview. |
| Fighters slide apart after B | The fighters are back at their places, but their push boxes can keep the scrubbed pose's width for a few ticks. The overlap pushes them apart. | None established. | Not fixed in this preview. |

The runs behind these: [Install evidence](EVIDENCE-rc15-installs.md#preview-build-cc71537c).

Other limits:

- **Test coverage:** the install 6 scrub report is an operator observation, not a full console test. Its Eden checks are in progress.
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
