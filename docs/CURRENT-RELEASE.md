# Current release

Current release: rc.16

## TrainingRooms 0.2.0-rc.16 (Eden-tested)

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

## Which build is mine?

The TrainingRooms menu footer shows the release and build.

## Earlier releases

[rc.15 release](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.15) · [release notes](RELEASE-NOTES.md#release-notes-rc15)

[rc.13 release](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.13) · [release notes](RELEASE-NOTES.md#release-notes-rc13)

## Release notes

[Release notes](RELEASE-NOTES.md#release-notes-rc16)
