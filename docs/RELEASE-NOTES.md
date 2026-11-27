# Release notes

<a id="release-notes-rc15"></a>

## rc.15 · November 15–27, 2026

For Training players editing recordings, grading drills and practising against the CPU.

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
2. Put `libnro_hook.nro` there if missing. Remove `libtraining_modpack.nro` if present.
3. Copy the downloaded `libtraining_rooms.nro` there. Replace the previous copy to update.
4. Keep your settings and recordings in `sd:/ultimate/TrainingRooms/`.

To uninstall, remove only `libtraining_rooms.nro`. Keep saved data and shared hooks.


<a id="release-notes-rc15--fixed-build"></a>

### What changed

| Change | Read more |
| --- | --- |
| Switch recorded fighters after loading finishes | [Guide](guide/recording.md) |
| Return fighters to their spots after scrubbing a pose | [Guide](guide/recording.md) |
| Read the CPU-first setup order in Guard the Edge | [Guide](guide/guard-the-edge.md) |
| Count Tech practice attempts with factory launch settings | [Guide](guide/tech-practice.md) |

### Tested on

| Feature | Hardware | Eden |
| --- | --- | --- |
| Switch recorded fighters after loading finishes | no | no |
| Return fighters to their spots after scrubbing a pose | no | yes |
| Read the CPU-first setup order in Guard the Edge | no | yes |
| Count Tech practice attempts with factory launch settings | no | yes |

“No” means a passing run is unconfirmed for this release.


### Earlier changes in rc.15

| Date | Change | Guide |
| --- | --- | --- |
| <a id="release-notes-rc15--preview-build"></a>Nov 27 | Scrub recorded poses on matching fighters. | [Guide](guide/recording.md) |
| <a id="release-notes-rc15--install-6"></a>Nov 26 | Move the playhead freely or snap In to a move. | [Guide](guide/recording.md) |
| <a id="release-notes-rc15--install-5"></a><a id="release-notes-rc15--record-works-while-an-ai-plays-player-1"></a>Nov 25 | Record while an AI controls Player 1. | [Guide](guide/recording.md) |
| <a id="release-notes-rc15--combo-reader-through-a945b0c1"></a>Nov 25 | Save named grabs and throws. Earlier readers cannot open those keys. | [Guide](guide/combos.md) |
| <a id="release-notes-rc15--install-4"></a>Nov 25 | Trim recordings and edit clips over the game. | [Guide](guide/recording.md) |
| <a id="release-notes-rc15--the-input-display"></a>Nov 25 | Scroll back through the input display. | [Guide](guide/frame-strip.md) |
| <a id="release-notes-rc15--install-3"></a>Nov 24 | Keep the practice partner stopped in the waiting room. | [Guide](guide/waiting-rooms.md) |
| <a id="release-notes-rc15--install-2"></a>Nov 24 | Back out of a chain without saving. | [Guide](guide/combos.md) |
| <a id="release-notes-rc15--install-1"></a>Nov 23 | See which fighter is recording. | [Guide](guide/recording.md) |
| Nov 15 | Edit clips and practise input drills. | [Guide](guide/recording.md) |

### Earlier rc.15 tests

| Feature | Hardware | Eden |
| --- | --- | --- |
| Scrub recorded poses on matching fighters | no | yes |
| Move the playhead or snap In to a move | no | yes |
| Record while an AI plays Player 1 | no | yes |
| Save named grabs and throws | no | no |
| Trim recordings and edit clips over the game | no | yes |
| Scroll back through inputs | no | yes |
| Stop the waiting-room practice partner | no | yes |
| Back out of a new chain | no | no |
| Show the active recording role | no | yes |
| Edit clips and practise input drills | no | yes |

### Known limits

- Older captures without poses show positions only. [Practice restrictions](RESTRICTIONS.md)

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
| Keep D-pad chords separate from Online Deluxe menu navigation | no | yes |
| Suppress TrainingRooms chords when both GameCube triggers are held | no | no |
| Refuse Play on Player 1 and Quick Reset while CPU Pilot controls Player 1 | no | yes |

“No” means a passing run is unconfirmed for this release.

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

Use rc.6. There is no public rc.5 download.

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

- Use rc.6 to drill a sequence or save the last combo.
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
