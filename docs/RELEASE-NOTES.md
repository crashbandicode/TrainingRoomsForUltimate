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
2. Keep `libnro_hook.nro` there. Remove `libtraining_modpack.nro` if present.
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

For players using this November test build. [Requirements and installation](#what-you-need).

### What changed

| Change | Read more |
| --- | --- |
| Keep TrainingRooms chords separate from the Online Deluxe menu | [Guide](guide/controls.md) |
| Refuse Quick Reset while an AI controls Player 1 | [Guide](guide/controls.md) |

### Tested on

| Feature | Hardware | Eden |
| --- | --- | --- |
| Keep TrainingRooms chords separate from the Online Deluxe menu | no | yes |
| Refuse Quick Reset while an AI controls Player 1 | no | yes |

“No” means a passing run is unconfirmed for this release.

### Known limits

- [Practice restrictions](RESTRICTIONS.md)

<a id="release-notes-rc8"></a>

## rc.8 · November 2026

For players using this November test build. [Requirements and installation](#what-you-need).

<a id="release-notes-rc8--a-rare-freeze-as-the-menu-closes"></a>

### What changed

| Change | Read more |
| --- | --- |
| Toggle the input display with R and D-pad Down | [Guide](guide/frame-strip.md) |
| Change game speed with R and D-pad Up | [Guide](guide/game-speed.md) |
| Close the menu without freezing | [Guide](guide/using-the-menu.md) |

### Tested on

| Feature | Hardware | Eden |
| --- | --- | --- |
| Toggle the input display with R and D-pad Down | no | yes |
| Change game speed with R and D-pad Up | no | yes |
| Close the menu without freezing | no | yes |

“No” means a passing run is unconfirmed for this release.

### Known limits

- [Practice restrictions](RESTRICTIONS.md)

<a id="release-notes-rc6"></a>

## rc.6 · November 2026

For players using this November test build. [Requirements and installation](#what-you-need).

<a id="release-notes-rc6--fighters-load-hot"></a>

### What changed

| Change | Read more |
| --- | --- |
| Practise a sequence and save the last combo | [Guide](guide/combos.md) |
| Switch between ready fighters | [Guide](guide/fighter-cycles.md) |
| Keep waiting-room practice partners | [Guide](guide/waiting-rooms.md) |

### Tested on

| Feature | Hardware | Eden |
| --- | --- | --- |
| Practise a sequence and save the last combo | no | yes |
| Switch between ready fighters | no | yes |
| Keep waiting-room practice partners | no | yes |

“No” means a passing run is unconfirmed for this release.

### Known limits

- [Practice restrictions](RESTRICTIONS.md)

<a id="release-notes-rc5"></a>

## rc.5 · November 2026

This candidate was withdrawn before player testing. Its features shipped in rc.6.

### What changed

| Change | Read more |
| --- | --- |
| Read combo timing from the frame strip | [Guide](guide/frame-strip.md) |
| Record and replay inputs using chords | [Guide](guide/recording.md) |

### Tested on

| Feature | Hardware | Eden |
| --- | --- | --- |
| Read combo timing from the frame strip | no | no |
| Record and replay inputs using chords | no | no |

“No” means a passing run is unconfirmed for this release.

### Known limits

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
