# Current release

Current release: rc.15

## TrainingRooms 0.2.0-rc.15 (hardware-tested)

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
3. Copy `libtraining_rooms.nro` from the [rc.15 download](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.15) into that folder, replacing the previous copy.
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

## Which build is mine?

The TrainingRooms menu footer shows the release and build.

## Earlier releases

[rc.13 release](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.13) · [release notes](RELEASE-NOTES.md#release-notes-rc13)

## Release notes

[Release notes](RELEASE-NOTES.md#release-notes-rc15)
