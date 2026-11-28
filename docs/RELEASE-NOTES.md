# Release notes

<a id="release-notes-rc16"></a>

## rc.16 · November 28, 2026

TrainingRooms 0.2.0-rc.16 is the November 28, 2026 fix build on rc.15 for players practising against the CPU and editing recordings. It needs one plugin fewer. It has not been tested on a Switch yet.

### What you need

| Thing | Upstream | Our fork | What the fork changes |
| --- | --- | --- | --- |
| Switch firmware | [latest](https://en-americas-support.nintendo.com/app/answers/detail/a_id/22525) | — | — |
| Atmosphère | [latest](https://github.com/Atmosphere-NX/Atmosphere/releases/latest) | [Atmosphère fork](https://git.syntaxbyexample.com/intpa/atmosphere-ro-cleanup-p1) | Unmaps plugin memory when loading fails. |
| Super Smash Bros. Ultimate | 13.0.5 | — | — |
| Skyline | [latest](https://github.com/skyline-dev/skyline/releases/latest) | none | — |
| ARCropolis | [latest](https://github.com/Raytwo/ARCropolis/releases/latest) | [ARCropolis fork](https://git.syntaxbyexample.com/intpa/ARCropolis) | Rebuilds damaged mod caches and refreshes lookup tables after mod changes. |

libnro_hook and libparam_hook are not needed.

### Install

1. Close Smash. Open `sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/`.
2. Copy `libtraining_rooms.nro` from the rc.16 download into that folder, replacing the previous copy.
3. Keep `manifest.json` and `SHA256SUMS.txt` on your computer to verify the download; keep settings and recordings in `sd:/ultimate/TrainingRooms/`.
4. Remove `libtraining_modpack.nro` (the old Training Modpack) if present. Never install both.
5. Optional: delete `libnro_hook.nro`. If you use FX Control, the tile cursor or dispatcher-backoff, first update them to [FX Control 0.3.1](https://git.syntaxbyexample.com/intpa/ssbu-fx-control/releases/tag/v0.3.1), [tile cursor 0.1.0+frame-repeat.2](https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/releases/tag/v0.1.0+frame-repeat.2) and [dispatcher-backoff 0.1.2](https://git.syntaxbyexample.com/intpa/ssbu-dispatcher-backoff/releases/tag/v0.1.2), which no longer ask for it.

TrainingRooms no longer uses `libnro_hook.nro`, and never used `libparam_hook.nro`. Keep either file only if another mod of yours needs it.

To update, replace only `libtraining_rooms.nro`. To uninstall, remove only that file and keep saved data.

### What's new since rc.15

| Change | Read more |
| --- | --- |
| Run without `libnro_hook.nro` | [Install](#install) |
| Refuse a Tech practice Set or Random start that reaches no surface | [Guide](guide/tech-practice.md) |
| Draw every hitbox overlay box without waiting on the plugin | [Menu](menu/display-and-controls.md#hitbox-visualization) |
| Hold less memory while a live match is recorded | [Guide](guide/recording.md) |
| Practise Ledge Trap against CPU ledge options | [Menu](menu/practice-games.md#ledge-trap) |
| Record detected cut files for troubleshooting | [Guide](guide/troubleshooting.md#where-the-logs-are-and-what-to-send) |

### Tested on

| Feature | Hardware (Switch) | Eden |
| --- | --- | --- |
| Run without `libnro_hook.nro` | no | yes |
| Refuse a Tech practice Set or Random start that reaches no surface | no | yes |
| Draw every hitbox overlay box without waiting on the plugin | no | yes |
| Hold less memory while a live match is recorded | no | yes |
| Ledge Trap practice | no | yes |
| Cut-file diagnostic boot | no | yes |

“No” means a passing run is unconfirmed for this release.

### Known limits

- No successful tech was produced in the Eden runs of Tech practice with a Set start; launches and grading ran.
- When scrubbing a recording without pose data, you see only the fighters’ recorded positions. [Practice restrictions](RESTRICTIONS.md).

The TrainingRooms menu footer shows the release and build.

<a id="release-notes-rc15"></a>

## rc.15 · November 27, 2026

[Download TrainingRooms rc.15](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.15).

TrainingRooms 0.2.0-rc.15 is the November 27, 2026 hardware-tested release for players editing recordings, practising against the CPU, and using practice tools.

### What you need

| Thing | Upstream | Our fork | What the fork changes |
| --- | --- | --- | --- |
| Switch firmware | [latest](https://en-americas-support.nintendo.com/app/answers/detail/a_id/22525) | — | — |
| Atmosphère | [latest](https://github.com/Atmosphere-NX/Atmosphere/releases/latest) | [Atmosphère fork](https://git.syntaxbyexample.com/intpa/atmosphere-ro-cleanup-p1) | Unmaps plugin memory when loading fails. |
| Super Smash Bros. Ultimate | 13.0.5 | — | — |
| Skyline | [latest](https://github.com/skyline-dev/skyline/releases/latest) | none | — |
| ARCropolis | [latest](https://github.com/Raytwo/ARCropolis/releases/latest) | [ARCropolis fork](https://git.syntaxbyexample.com/intpa/ARCropolis) | Rebuilds damaged mod caches and refreshes lookup tables after mod changes. |

### Install

1. Close Smash. Open `sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/`.
2. Put libnro_hook.nro there if missing.
3. Copy `libtraining_rooms.nro` from this download into that folder, replacing the previous copy.
4. Keep `manifest.json` and `SHA256SUMS.txt` on your computer to verify the download; keep settings and recordings in `sd:/ultimate/TrainingRooms/`.
5. Remove `libtraining_modpack.nro` (the old Training Modpack) if present. Never install both.

libparam_hook is not needed (an older page said it was; you can keep or remove it).

To update, replace only `libtraining_rooms.nro`. To uninstall, remove only that file and keep shared hooks and saved data.

### What's new since rc.13

| Change | Read more |
| --- | --- |
| <a id="release-notes-rc15--install-5"></a><a id="release-notes-rc15--record-works-while-an-ai-plays-player-1"></a>Record while an AI controls Player 1 | [Guide](guide/recording.md) |
| Switch recorded fighters after loading finishes | [Guide](guide/recording.md) |
| Return fighters to their spots after scrubbing a pose | [Guide](guide/recording.md) |
| Scrub recorded poses on matching fighters | [Guide](guide/recording.md) |
| Move the playhead freely or snap In to a move | [Guide](guide/recording.md) |
| <a id="release-notes-rc15--install-4"></a>Trim recordings and edit clips over the game | [Guide](guide/recording.md) |
| <a id="release-notes-rc15--combo-reader-through-a945b0c1"></a>Save named grabs and throws | [Guide](guide/combos.md) |
| <a id="release-notes-rc15--the-input-display"></a>Scroll back through the input display | [Guide](guide/frame-strip.md) |
| Stop the practice partner in waiting rooms | [Guide](guide/waiting-rooms.md) |
| Back out of a chain without saving | [Guide](guide/combos.md) |
| Show the active recording role | [Guide](guide/recording.md) |
| Edit clips and practise input drills | [Guide](guide/recording.md) |
| Explain loading and extra fighter groups | [Guide](menu/fighters-and-stages.md#loading) |
| Clarify what L+R+A does in Training | [Guide](guide/controls.md) |
| Read the CPU-first setup order in Guard the Edge | [Guide](guide/guard-the-edge.md) |
| Count Tech practice attempts with factory launch settings | [Guide](guide/tech-practice.md) |

<a id="release-notes-rc15--fixed-build"></a>

### Tested on

| Feature | Hardware (Switch) | Eden |
| --- | --- | --- |
| Record while an AI controls Player 1 | yes | yes |
| Switch recorded fighters after loading finishes | yes | no |
| Return fighters to their spots after scrubbing a pose | yes | no |
| Scrub recorded poses on matching fighters | yes | yes |
| Move the playhead freely or snap In to a move | yes | yes |
| Trim recordings and edit clips over the game | yes | yes |
| Save named grabs and throws | yes | no |
| Scroll back through the input display | yes | yes |
| Stop the practice partner in waiting rooms | yes | yes |
| Back out of a chain without saving | yes | no |
| Show the active recording role | yes | yes |
| Edit clips and practise input drills | yes | yes |
| Explain loading and extra fighter groups | yes | no |
| Clarify what L+R+A does in Training | yes | no |
| Read the CPU-first setup order in Guard the Edge | yes | yes |
| Count Tech practice attempts with factory launch settings | yes | yes |

### Known limits

- Some hitbox-overlay hooks can wait on a plugin lock. The docs audit found this; it was not seen in play, and a fix is in progress for the next build.
- When scrubbing a recording without pose data, you see only the fighters’ recorded positions. [Practice restrictions](RESTRICTIONS.md).


<a id="release-notes-rc13"></a>

## rc.13 · November 14, 2026

[Download, installation steps, changes and testing for v0.2.0-rc.13](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.13).

<a id="release-notes-rc12"></a>

## rc.12 · November 13, 2026

[Download, installation steps, changes and testing for v0.2.0-rc.12](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.12).

<a id="release-notes-rc9"></a>

## rc.9 · November 2026

For players sharing controller shortcuts with Online Deluxe 0.10.5 and CPU Pilot 0.2.4.

### What you need

| Thing | Upstream | Our fork | What the fork changes |
| --- | --- | --- | --- |
| Switch firmware | [latest](https://en-americas-support.nintendo.com/app/answers/detail/a_id/22525) | — | — |
| Atmosphère | [latest](https://github.com/Atmosphere-NX/Atmosphere/releases/latest) | [Atmosphère fork](https://git.syntaxbyexample.com/intpa/atmosphere-ro-cleanup-p1) | Unmaps plugin memory when loading fails. |
| Super Smash Bros. Ultimate | 13.0.5 | — | — |
| Skyline | [latest](https://github.com/skyline-dev/skyline/releases/latest) | none | — |
| ARCropolis | [latest](https://github.com/Raytwo/ARCropolis/releases/latest) | [ARCropolis fork](https://git.syntaxbyexample.com/intpa/ARCropolis) | Rebuilds damaged mod caches and refreshes lookup tables after mod changes. |

### Install

1. Close Smash. Open `sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/`.
2. Keep the shared hook plugins in that folder. Remove `libtraining_modpack.nro` if present.
3. Copy your release's `libtraining_rooms.nro` there. Replace the previous copy to update.
4. Keep your settings and recordings in `sd:/ultimate/TrainingRooms/`.

To uninstall, remove only `libtraining_rooms.nro`. Keep saved data and shared hooks.

### What changed

| Change | Read more |
| --- | --- |
| Keep D-pad chords separate from Online Deluxe menu navigation | [Guide](guide/controls.md) |
| Suppress TrainingRooms chords when both GameCube triggers are held | [Guide](guide/controls.md) |
| Refuse Play on Player 1 and Quick Reset while CPU Pilot controls Player 1 | [Guide](guide/recording.md) |

### Tested on

| Feature | Hardware | Eden |
| --- | --- | --- |
| D-pad chord separation | no | no |
| Suppress TrainingRooms chords when both GameCube triggers are held | no | no |
| CPU Pilot shortcut refusals | no | no |

A passing run on rc.9 is unconfirmed. Eden verified D-pad separation on rc.10 and rc.11, and CPU Pilot shortcut refusals on rc.10.

### Known limits

- Close the TrainingRooms menu before choosing Leave Battle in Quickplay.
- [Practice restrictions](RESTRICTIONS.md)

<a id="release-notes-rc8"></a>

## rc.8 · November 2026

For players changing input display and game speed during practice.

### What you need

| Thing | Upstream | Our fork | What the fork changes |
| --- | --- | --- | --- |
| Switch firmware | [latest](https://en-americas-support.nintendo.com/app/answers/detail/a_id/22525) | — | — |
| Atmosphère | [latest](https://github.com/Atmosphere-NX/Atmosphere/releases/latest) | [Atmosphère fork](https://git.syntaxbyexample.com/intpa/atmosphere-ro-cleanup-p1) | Unmaps plugin memory when loading fails. |
| Super Smash Bros. Ultimate | 13.0.5 | — | — |
| Skyline | [latest](https://github.com/skyline-dev/skyline/releases/latest) | none | — |
| ARCropolis | [latest](https://github.com/Raytwo/ARCropolis/releases/latest) | [ARCropolis fork](https://git.syntaxbyexample.com/intpa/ARCropolis) | Rebuilds damaged mod caches and refreshes lookup tables after mod changes. |

### Install

1. Close Smash. Open `sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/`.
2. Keep the shared hook plugins in that folder. Remove `libtraining_modpack.nro` if present.
3. Copy your release's `libtraining_rooms.nro` there. Replace the previous copy to update.
4. Keep your settings and recordings in `sd:/ultimate/TrainingRooms/`.

To uninstall, remove only `libtraining_rooms.nro`. Keep saved data and shared hooks.

<a id="release-notes-rc8--a-rare-freeze-as-the-menu-closes"></a>

### What changed

| Change | Read more |
| --- | --- |
| Toggle the input display with R and D-pad Down | [Guide](guide/frame-strip.md) |
| Hold R and D-pad Up, then press Left or Right to change speed | [Guide](guide/game-speed.md) |
| Show the current speed when it differs from normal | [Guide](guide/game-speed.md) |
| Show the build name in the menu footer | [Guide](guide/using-the-menu.md) |
| Refuse Record while CPU Pilot controls Player 1 | [Guide](guide/recording.md) |
| Close the menu without freezing | [Guide](guide/using-the-menu.md) |

### Tested on

| Feature | Hardware | Eden |
| --- | --- | --- |
| Toggle the input display with R and D-pad Down | no | yes |
| Hold R and D-pad Up, then press Left or Right to change speed | no | yes |
| Show the current speed when it differs from normal | no | yes |
| Show the build name in the menu footer | no | yes |
| Refuse Record while CPU Pilot controls Player 1 | no | yes |
| Close the menu without freezing | no | no |

“No” means a passing run is unconfirmed for this release.

### Known limits

- Recording is unavailable while CPU Pilot drives Player 1.
- [Practice restrictions](RESTRICTIONS.md)

<a id="release-notes-rc6"></a>

## rc.6 · November 2026

For players drilling combos and cycling through practice partners.

### What you need

| Thing | Upstream | Our fork | What the fork changes |
| --- | --- | --- | --- |
| Switch firmware | [latest](https://en-americas-support.nintendo.com/app/answers/detail/a_id/22525) | — | — |
| Atmosphère | [latest](https://github.com/Atmosphere-NX/Atmosphere/releases/latest) | [Atmosphère fork](https://git.syntaxbyexample.com/intpa/atmosphere-ro-cleanup-p1) | Unmaps plugin memory when loading fails. |
| Super Smash Bros. Ultimate | 13.0.5 | — | — |
| Skyline | [latest](https://github.com/skyline-dev/skyline/releases/latest) | none | — |
| ARCropolis | [latest](https://github.com/Raytwo/ARCropolis/releases/latest) | [ARCropolis fork](https://git.syntaxbyexample.com/intpa/ARCropolis) | Rebuilds damaged mod caches and refreshes lookup tables after mod changes. |

### Install

1. Close Smash. Open `sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/`.
2. Keep the shared hook plugins in that folder. Remove `libtraining_modpack.nro` if present.
3. Copy your release's `libtraining_rooms.nro` there. Replace the previous copy to update.
4. Keep your settings and recordings in `sd:/ultimate/TrainingRooms/`.

To uninstall, remove only `libtraining_rooms.nro`. Keep saved data and shared hooks.

<a id="release-notes-rc6--fighters-load-hot"></a>

### What changed

| Change | Read more |
| --- | --- |
| Practise an ordered sequence of combos | [Guide](guide/combos.md) |
| Save the combo you just landed | [Guide](guide/combos.md) |
| Open the recording pop-up over the match | [Guide](guide/recording.md) |
| Switch faster between ready fighters | [Guide](guide/fighter-cycles.md) |
| Apply cycle order changes immediately | [Guide](guide/fighter-cycles.md) |
| Select extra CPUs from the cycle list | [Guide](guide/fighter-cycles.md) |
| Tell whether a Mii is identical to an existing entry | [Guide](guide/mii.md) |
| Keep waiting-room practice partners between visits | [Guide](guide/waiting-rooms.md) |
| Set the practice CPU's level | [Guide](menu/cpu.md) |

### Tested on

| Feature | Hardware | Eden |
| --- | --- | --- |
| Practise an ordered sequence of combos | no | yes |
| Save the combo you just landed | no | yes |
| Open the recording pop-up over the match | no | yes |
| Switch faster between ready fighters | no | yes |
| Apply cycle order changes immediately | no | yes |
| Select extra CPUs from the cycle list | no | yes |
| Tell whether a Mii is identical to an existing entry | no | yes |
| Keep waiting-room practice partners between visits | no | yes |
| Set the practice CPU's level | no | yes |

“No” means a passing run is unconfirmed for this release.

### Known limits

- Cold fighter switches can take longer than switches between ready fighters.
- [Practice restrictions](RESTRICTIONS.md)

<a id="release-notes-rc5"></a>

## rc.5 · November 2026

Withdrawn before console play. Its recording tools shipped in rc.6.

### Install

There is no public rc.5 download. For a playable build, follow the [current release installation steps](CURRENT-RELEASE.md#install).

### What changed

| Change | Read more |
| --- | --- |
| See both fighters' inputs and combo timing on the frame strip | [Guide](guide/frame-strip.md) |
| Record named macros and play them on either fighter | [Guide](guide/recording.md) |
| Chain recordings and edit combo sequences | [Guide](guide/combos.md) |
| Bind a save to the recording pop-up | [Guide](guide/save-states.md) |
| Migrate old save slots into named files | [Guide](guide/save-states.md) |

### Tested on

| Feature | Hardware | Eden |
| --- | --- | --- |
| See both fighters' inputs and combo timing on the frame strip | no | no |
| Record named macros and play them on either fighter | no | no |
| Chain recordings and edit combo sequences | no | no |
| Bind a save to the recording pop-up | no | no |
| Migrate old save slots into named files | no | no |

“No” means a passing run is unconfirmed for this release.

### Known limits

- Sequence drills and saving the last combo arrived in [rc.6](#release-notes-rc6).
- [Practice restrictions](RESTRICTIONS.md)

<a id="release-notes-rc3.1"></a>

## rc.3.1 · November 6, 2026

[Download, installation steps, changes and testing for v0.2.0-rc.3.1](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.3.1).

<a id="release-notes-rc3"></a>

## rc.3 · November 5, 2026

[Download, installation steps, changes and testing for v0.2.0-rc.3](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.3).

<a id="release-notes-rc2"></a>

## rc.2 · November 5, 2026

[Download, installation steps, changes and testing for v0.2.0-rc.2](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.2).

<a id="release-notes-rc1"></a>

## rc.1 · November 5, 2026

[Download, installation steps, changes and testing for v0.2.0-rc.1](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.1).

<a id="release-notes-rc0"></a>

## 0.1.0 · November 4, 2026

[Download, installation steps, changes and testing for v0.1.0](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.1.0).
