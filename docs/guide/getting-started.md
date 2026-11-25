# Getting started

Install TrainingRooms and start practising. It works in Training and in the waiting rooms of online and local play.
Which release is current and what is new: [Current release](../CURRENT-RELEASE.md).

<video src="../media/tut-menu-tour.mp4" autoplay muted loop playsinline controls></video>

*B + D-pad Up opens the menu, R turns through the five pages, B closes it and practice goes on. Eden, 2026-11-25, build `c51f4d81`.* [Open the clip](../media/tut-menu-tour.mp4)

## Installing

1. You need a Switch with Super Smash Bros. Ultimate **13.0.5** and Skyline set up for the game. On any other game
   version the plugin installs nothing.
2. Copy `libtraining_rooms.nro` and `libnro_hook.nro` into
   `atmosphere/contents/01006A800016E000/romfs/skyline/plugins/` on the SD card. The hook enables the CPU's DI,
   tech and out-of-shield options. Its usual companion, `libparam_hook.nro`, is optional for TrainingRooms.
3. Remove `libtraining_modpack.nro` from that folder if it is there. The two can't run together.
4. Start the game and open Training.

To update, copy the new `libtraining_rooms.nro` over the old one. Your settings are not in that file; they are in
`sd:/ultimate/TrainingRooms/`.

## First practice

1. On a new install, the first practice shows six short reminders: **Welcome**, then **Open Menu**, **Save State**,
   **Load State**, **Input Record** and **Input Playback**, each with its buttons. Later starts leave them out.
2. If the Training Modpack's folder `sd:/ultimate/TrainingModpack/` is on the card, its settings, save states and
   recordings are copied into `sd:/ultimate/TrainingRooms/` once. The originals are not changed.
3. Hold **B** and press **D-pad Up** to open the menu. **Minus** opens it too, until you change Menu shortcut 1.
4. **L** and **R** turn the pages. **B** closes the menu, and your changes apply.

Try these three first:

| Try | Buttons | More |
| --- | --- | --- |
| Save where you stand, then come back to it | ZL + D-pad Down saves, ZL + D-pad Up loads | [Save states](save-states.md) |
| See the boxes that hit and hurt | Open Display & Controls › Hitbox Visualization with A. Tick Hitboxes and Hurtboxes with A, back out with B, then attack. | [Hitbox Visualization](../menu/display-and-controls.md#hitbox-visualization) |
| Slow the game down | Hold R + D-pad Up, then Left or Right | [Game speed](game-speed.md) |

## The pages

| Page | What it is for |
| --- | --- |
| [CPU](../menu/cpu.md) | How the CPU behaves: mash, DI, tech, shield, its level. |
| [Saves](../menu/saves.md) | Save and restore practice positions. |
| [Recording](../menu/recording.md) | Record, play back and drill sequences. |
| [Fighters & Stages](../menu/fighters-and-stages.md) | Fighters, stages and the practice games. |
| [Display & Controls](../menu/display-and-controls.md) | What you see, and the buttons. |

## Which build you have

The key hints at the bottom of every menu page end with the build tag, for example `0.2.0-rc.15 · rc15 1a2b3c4d`.

| Part | What it is |
| --- | --- |
| `0.2.0-rc.15` | The version number inside the plugin. |
| `rc15` | The short name of the same release. |
| `1a2b3c4d` | The commit the build was made from, so yours will differ. `-dirty` after it means uncommitted changes; `unknown` means the commit wasn't known. |

A build before rc.14 reads `0.2.0-rc.6` in the first part whatever its release, so read the second part there.

More: [Using the menu](using-the-menu.md), [Controls](controls.md), [Waiting rooms](waiting-rooms.md),
[Troubleshooting](troubleshooting.md), [all the docs](../README.md).
