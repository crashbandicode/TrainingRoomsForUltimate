# TrainingRooms for Ultimate

TrainingRooms adds practice tools to Super Smash Bros. Ultimate's Training mode and to the waiting rooms of online and
local play: hitboxes, save states, recordings, combo drills, game speed and more, all from one menu.

**Current release: [rc.15](docs/CURRENT-RELEASE.md).** Its newest build, install 6 (`5df3d9dc`, 2026-11-26), is
installed and read back. Its Eden checks are in progress. What's new, how to install it and what is known not to work
are on that page; each build's changes are in the [release notes](docs/RELEASE-NOTES-rc15.md). New here? Start with [Getting started](docs/guide/getting-started.md). Something not
working? See [Troubleshooting](docs/guide/troubleshooting.md).

![The menu's five pages, then Hitbox Visualization's checklist with Hitboxes, Grab boxes and Hurtboxes ticked, both fighters' hurtbox pills and a neutral air's red hitbox](docs/media/rc14-menu-tour.gif)<br>
*The menu's five pages, then Hitbox Visualization: A ticks Hitboxes, Grab boxes and Hurtboxes, and both fighters are drawn as hurtbox pills (yellow in this clip; grey in the current colours). Eden, rc.14 (untested on hardware).*

[Stage tech practice](docs/menu/fighters-and-stages.md#stage-tech-practice): pick a spot and practise techs with launch settings, grades and sessions.

Every option is documented in the [docs](docs/README.md), one page per menu page.

## Install

Tested on: Super Smash Bros. Ultimate 13.0.5 · Switch firmware 23.5.0 · Atmosphère 1.12.2

1. Set up Skyline for Smash Ultimate 13.0.5 on your Switch.
2. Copy `libtraining_rooms.nro` and `libnro_hook.nro` into
   `atmosphere/contents/01006A800016E000/romfs/skyline/plugins/` on the SD card.
   The hook enables CPU DI, tech and out-of-shield options; its usual companion `libparam_hook.nro` is optional for TrainingRooms.
3. If `libtraining_modpack.nro` is there, remove it: the two can't run together.
4. Start the game and open Training. Settings from the Training Modpack are copied over the first time.
5. Press B + D-pad Up (or Minus) to open the menu.

## Controls

Use chords in Training and waiting-room practice. Open **Display & Controls › Button chords**, press A on a chord,
then X. Hold and release the new chord; A keeps it.

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/3a68fe9e37b78333c5a0ca217f7349b12124fe89" autoplay muted loop playsinline controls></video>

*Button chords: Save State Save reads ZL + D-pad Down; X, then holding ZL + D-pad Left sets the new chord, and that chord saves. Eden, 2026-11-25, build `c51f4d81`.* [Open the clip](docs/media/tut-controls-change-chord.mp4)

The clip's new chord is also Cycle CPU by default; change or clear Cycle CPU if you want it to do only one thing.

| Action | Default | Change it at |
| --- | --- | --- |
| Save State Save | ZL + D-pad Down | Button chords |
| Save State Load | ZL + D-pad Up | Button chords |
| Record | ZR + D-pad Down | Button chords |
| Play on CPU | ZR + D-pad Up | Button chords |
| Play on Player 1 | ZR + D-pad Left | Button chords |
| Recording pop-up | ZR + D-pad Right | Button chords |
| Cycle Player | ZL + D-pad Right | Button chords |
| Cycle CPU | ZL + D-pad Left | Button chords |
| Input display | R + D-pad Down | Button chords |
| Game speed | Hold R + D-pad Up, then Left / Right | Button chords |
| Open the menu | Minus | Menu shortcut 1 |
| Open the menu | Disabled | Menu shortcut 2 |

B + D-pad Up opens the menu and cannot be changed. L + R + A can reset, and holding B and clicking the left / right stick
cycles your / the CPU's Kirby hat (left stick at rest); these buttons are fixed. Cycle Player needs **Player Swapping**
On. See [Controls](docs/guide/controls.md) for GameCube and Online Deluxe.

### Game speed

Hold R + D-pad Up, then tap Left for slower or Right for faster. Release R to keep the speed. See [Game speed](docs/guide/game-speed.md), including 1 Frame.

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/d4df993d500e229bdcaee85defab1e2f3d5c2d48" autoplay muted loop playsinline controls></video>

*A forward smash at normal speed. Hold R + D-pad Up and tap Left: the label steps to 1/4, and the same forward smash plays slowly. The label steps back to normal speed at the end. Eden, the diagnostic file of preview build `cc71537c`, 2026-11-27.* [Open the clip](docs/media/tut-game-speed.mp4)

## The menu

Open with B + D-pad Up. L / R turn pages, Up / Down choose a row, A opens or changes it, and B goes back or closes the
menu. Changes apply when it closes; see [Using the menu](docs/guide/using-the-menu.md).

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/5c59ac60ffc033815119d657b463e73f2fadb636" autoplay muted loop playsinline controls></video>

*B + D-pad Up opens the menu, R turns through the five pages, B closes it and practice goes on. Eden, 2026-11-25, build `c51f4d81`.* [Open the clip](docs/media/tut-menu-tour.mp4)

### CPU

Choose how the [CPU responds](docs/guide/cpu-behaviour.md): open **CPU › Mash** with A and tick an action, then choose
when it fires in **Mash Triggers**.

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/67d92f30104dd21a740499e4b780336754029284" autoplay muted loop playsinline controls></video>

*With CPU › Mash set to an attack, Mario jabs Link and Link hits back. Link ends at 2.7%, Mario at 3.7%. Eden, install 6's diagnostic build `5df3d9dc`, 2026-11-26.* [Open the clip](docs/media/tut-cpu-mash.mp4)

### Saves

With Save States On, save both fighters' positions with ZL + D-pad Down; ZL + D-pad Up loads them again.
[Save states](docs/guide/save-states.md) also covers damage, mirroring and buffs on load.

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/2f0394ea36fb533c09e157925c016b4e369ab2a6" autoplay muted loop playsinline controls></video>

*ZL + D-pad Down saves beside the CPU; two jabs take it to 4.9%; ZL + D-pad Up puts both fighters back at 0%. Eden, 2026-11-25, build `c51f4d81`.* [Open the clip](docs/media/tut-save-load.mp4)

### Recording

Record a macro: select Player 1 or CPU in the Recording pop-up, press ZR + D-pad Down, do the moves, then press it
again to stop.

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/35e8edb3e156f65dc2b9cc87b450a10d83e2f8d9" autoplay muted loop playsinline controls></video>

*ZR + D-pad Down starts Record. Mario is tinted red as his forward smash sends Link to 18.5%. The tint clears when recording stops. Eden, install 6's diagnostic build `5df3d9dc`, 2026-11-26.* [Open the clip](docs/media/tut-record-macro.mp4)

To drill a combo, land it, then choose **Recording › Combos & Sequences › + Save last combo…**. The link cells grade
your next attempts.

<!-- CLIP: tut-combo-save-drill.mp4 | Kirby lands jab into side tilt → Recording › Combos & Sequences › + Save last combo… → the next try is graded on the link cells -->
*Clip coming.*

Cut a macro, combo or drill into a new file with [Crop](docs/guide/recording.md#crop).

Practise movement without a hit with [Input drills](docs/guide/recording.md#input-drills).

More: [Recording](docs/guide/recording.md) and [playback options](docs/menu/recording.md).

### Fighters & Stages

Switch fighters without leaving practice: choose the order in **Character Cycles**, then press ZL + D-pad Left to
cycle the CPU. See [Fighter cycles](docs/guide/fighter-cycles.md).

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/6d08f7457ab8d11ca30c8d7ed1140dd4611397e9" autoplay muted loop playsinline controls></video>

*The Cycle Player chord, ZL + D-pad Right, swaps Mario for Donkey Kong where he stands. Eden, install 6's diagnostic build `5df3d9dc`, 2026-11-26.* [Open the clip](docs/media/tut-fighter-cycle.mp4)

To change stage, open **Change Stage** with A, choose a stage and form, then press A. See [Stages](docs/guide/stages.md).

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/e0de6dbeba09dd60aa5dc2ec3c5fbde73470d79a" autoplay muted loop playsinline controls></video>

*Fighters & Stages › Change Stage: the grid, Y for the Ω form, A on Battlefield. The stage is swapped and both fighters stand on Battlefield's Ω form. Eden, the diagnostic file of preview build `cc71537c`, 2026-11-27.* [Open the clip](docs/media/tut-stage-change.mp4)

Choose and start a game under [Practice games](docs/menu/practice-games.md).

### Display & Controls

See attacks and hurtboxes: press A on [Hitbox Visualization](docs/menu/display-and-controls.md#hitbox-visualization),
then A to tick Hitboxes, Grab boxes and Hurtboxes. The Colour legend appears beside the checklist.

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/b5760c58e2de36d18ee1fc677043ffb7c7376dce" autoplay muted loop playsinline controls></video>

*With Hitboxes, Grab boxes and Hurtboxes ticked, both fighters show their hurtboxes as see-through pills. Look for the pink hitbox on Mario's jab and the purple box when he grabs. Eden, install 6's diagnostic build `5df3d9dc`, 2026-11-26.* [Open the clip](docs/media/tut-hitbox-visualization.mp4)

See hittable objects with [Article hurtboxes](docs/menu/display-and-controls.md#hitbox-visualization).

## Waiting rooms

Practise while waiting for a local wireless or Quickplay match. With **Fighters & Stages › Lobby Practice** On,
open the room menu with B + D-pad Up. See [Waiting rooms](docs/guide/waiting-rooms.md).

<!-- CLIP: tut-lobby-practice-menu.mp4 | A Local Wireless waiting room (one member): B + D-pad Up opens the room menu, Hurtboxes ticked, both practice fighters drawn as pills -->
*Clip coming.*

## License

MIT License: see [LICENSE](LICENSE).

This project was developed with the assistance of AI. It is an unofficial fan project, not affiliated with or
endorsed by Nintendo.

## More

[The docs](docs/README.md): every menu page and guide. The current release is [rc.15](docs/CURRENT-RELEASE.md),
**installed, with its own Eden checks in progress** (see [Status](docs/TECHNICAL.md#status)): what's new, how
to install it and its known issues.
The rc.15 builds are in [their release notes](docs/RELEASE-NOTES-rc15.md). Release notes for rc.12, rc.13 and rc.14
are on their
[release pages](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases); the ones in the docs folder
(rc.5, rc.6, rc.8 and rc.9) are listed on [the docs page](docs/README.md#reference). [Technical notes](docs/TECHNICAL.md).

## Build

Build the plugin with cargo-skyline; the toolchains and the rest of the gates are in [Building TrainingRooms](docs/BUILD.md).

```powershell
cargo skyline build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --no-default-features
```
