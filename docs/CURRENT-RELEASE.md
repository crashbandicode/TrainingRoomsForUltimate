# Current release

<!-- Whoever ships a release: change the `Current release:` line below first, then the README's top and the Status section
     of docs/TECHNICAL.md, and put the same release in Cargo.toml's version (0.2.0-rc.N, with Cargo.lock's own entry) in the
     release's name commit. The docs checks (tests/docs_gates.rs) fail until they all name the same release. The steps are
     in docs/BUILD.md, "Release checklist". -->

Current release: rc.15

Status: **Internal candidate, untested on hardware.** The feature pages carry their individual Eden evidence;
the final integrated gate and Eden smoke are pending. This page says which release is current. The
[README](../README.md) and the [technical notes](TECHNICAL.md#status) follow it.

## Which build is mine?

Open the menu (B + D-pad Up) and read the end of the key hints at the foot of the page. rc.15 reads
`0.2.0-rc.15 · rc15` and then the commit it was built from. Builds before rc.14 start with `0.2.0-rc.6` whatever their
release, so for them read the second part (`rc13`, `rc12`, ...).
More: [Getting started › Which build you have](guide/getting-started.md#which-build-you-have).

## What's new in rc.15

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

- **Untested on hardware.** See the status above.
- **Fighter loading.** A fighter that doesn't fit in Ready Fighters loads when you swap to it, so that swap takes a
  moment ([Fighter cycles](guide/fighter-cycles.md#what-the-loading-rows-do)).
- **Limits kept from the Training Modpack** are listed in
  [Differences from the fork](DIFFERENCES-FROM-THE-FORK.md#kept-from-m5q).
- **Waiting rooms** have their own limits and refusals:
  [What is different, or refused](guide/waiting-rooms.md#what-is-different-or-refused).

Something else? [Troubleshooting](guide/troubleshooting.md) says what to check and what to send.

## Earlier releases

- Release notes in this folder: [rc.5](RELEASE-NOTES-rc5.md), [rc.6](RELEASE-NOTES-rc6.md),
  [rc.8](RELEASE-NOTES-rc8.md) and [rc.9](RELEASE-NOTES-rc9.md).
- On their release pages: [rc.12](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.12),
  [rc.13](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.13)
  and [rc.14](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.14).
- rc.7, rc.10 and rc.11 have no notes: see [Status](TECHNICAL.md#status).
