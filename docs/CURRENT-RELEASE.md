# Current release

Current release: rc.15

TrainingRooms 0.2.0-rc.15 is the November 27, 2026 build for console testing. The [public download](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases) is rc.13.

## What you need

| Thing | Upstream | Our fork | What the fork changes |
| --- | --- | --- | --- |
| Switch firmware | [latest](https://en-americas-support.nintendo.com/app/answers/detail/a_id/22525) | — | — |
| Atmosphère | [latest](https://github.com/Atmosphere-NX/Atmosphere/releases/latest) | [Atmosphère fork](https://git.syntaxbyexample.com/intpa/atmosphere-ro-cleanup-p1) | Unmaps plugin memory when loading fails. |
| Super Smash Bros. Ultimate | 13.0.5 | — | — |
| Skyline | [latest](https://github.com/skyline-dev/skyline/releases/latest) | none | — |
| ARCropolis | [latest](https://github.com/Raytwo/ARCropolis/releases/latest) | [ARCropolis fork](https://git.syntaxbyexample.com/intpa/ARCropolis) | Rebuilds damaged mod caches and refreshes lookup tables after mod changes. |

## Install

1. Close Smash. Open `sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/`.
2. Put `libnro_hook.nro` there if missing. For the public rc.13 download, also install `libparam_hook.nro` if missing. Remove `libtraining_modpack.nro` if present.
3. Copy the `libtraining_rooms.nro` for your chosen version there. Replace the previous copy to update.
4. Keep your settings and recordings in `sd:/ultimate/TrainingRooms/`.
5. For the public download, keep `manifest.json` and `SHA256SUMS.txt` on your computer for verification.

To uninstall, remove only `libtraining_rooms.nro`. Keep saved data and shared hooks.

## What changed

| Change | Read more |
| --- | --- |
| Switch recorded fighters after loading finishes | [Guide](guide/recording.md) |
| Return fighters to their spots after scrubbing a pose | [Guide](guide/recording.md) |
| Explain loading and extra fighter groups in the menu | [Loading](menu/fighters-and-stages.md#loading) |
| Clarify what L+R+A does in Training | [Controls](guide/controls.md) |
| Read the CPU-first setup order in Guard the Edge | [Guide](guide/guard-the-edge.md) |
| Count Tech practice attempts with factory launch settings | [Guide](guide/tech-practice.md) |

## Tested on

| Feature | Hardware | Eden |
| --- | --- | --- |
| Switch recorded fighters after loading finishes | no | no |
| Returning fighters to their spots after scrubbing | no | no |
| Showing a recorded pose while scrubbing | no | yes |
| Loading and Group Swap help | no | no |
| L+R+A help | no | no |
| Read the CPU-first setup order in Guard the Edge | no | yes |
| Count Tech practice attempts with factory launch settings | no | yes |

“No” means a passing run is unconfirmed for this release.

## Which build is mine?

Read the version in the TrainingRooms menu footer.

## Known issues

- When scrubbing a recording without pose data, you see only the fighters’ recorded positions.
- [Waiting-room and practice restrictions](RESTRICTIONS.md)

## Earlier releases

[Release notes](RELEASE-NOTES.md#release-notes-rc15)
