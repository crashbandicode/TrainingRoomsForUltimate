# Current release

<!-- Whoever ships a release: change the `Current release:` line below first, then the README's top and the Status section
     of docs/TECHNICAL.md, and put the same release in Cargo.toml's version (0.2.0-rc.N, with Cargo.lock's own entry) in the
     release's name commit. The docs checks (tests/docs_gates.rs) fail until they all name the same release. The steps are
     in docs/BUILD.md, "Release checklist". -->

Current release: rc.14

Status: **Eden-tested and untested on hardware.** rc.14 was run in the Eden emulator (2026-11-15); no one has run it
on a Switch yet. This page says which release is current. The [README](../README.md) and the
[technical notes](TECHNICAL.md#status) follow it.

Its release page, with the notes and the plugin files:
[v0.2.0-rc.14](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.14).

## Which build is mine?

Open the menu (B + D-pad Up) and read the end of the key hints at the foot of the page. rc.14 reads
`0.2.0-rc.14 · rc14` and then the commit it was built from. Builds before rc.14 start with `0.2.0-rc.6` whatever their
release, so for them read the second part (`rc13`, `rc12`, ...).
More: [Getting started › Which build you have](guide/getting-started.md#which-build-you-have).

## What's new in rc.14

- **Practice games.** Fighters & Stages › Practice games: Eggs-ercise, where targets pop up for you to break, and Break
  the Targets, where you place up to 10 targets yourself and break them against the clock. Layouts are saved as files.
  [Practice games](menu/fighters-and-stages.md#practice-games)
- **Hide CPU.** The CPU page can take the CPU out of the fight without touching its settings.
  [Hide CPU](menu/cpu.md#hide-cpu)
- **Hitbox Visualization is one checklist.** Hitboxes, Grab boxes, Hurtboxes, Frame data colours, Launch angles and
  Windboxes, in any mix. [Hitbox Visualization](menu/display-and-controls.md#hitbox-visualization)
- **Sweet and sour spots.** Frame data colours show strong and weak hitboxes, Spot labels name each hit Sweet, Sour or
  Late on the frame strip, and a combo can match the spot it was saved with.
  [Frame data colours](menu/display-and-controls.md#frame-data-colours),
  [Spot labels](menu/display-and-controls.md#spot-labels),
  [Match sweet/sour spot](menu/recording.md#match-sweet-sour-spot)
- **Setup library.** Saves › Save setup here... keeps where both fighters stand now as a named setup, and each setup has
  a screen of its own to restore, rename or delete it. [Setup library](menu/saves.md#setup-library)
- **Fighter cycles.** A cycle with more fighters than Ready Fighters leaves room for keeps the ones that fit loaded and
  loads the rest when you swap to them.
  [What the Loading rows do](guide/fighter-cycles.md#what-the-loading-rows-do)

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
- On their release pages: [rc.12](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.12)
  and [rc.13](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.13).
- rc.7, rc.10 and rc.11 have no notes: see [Status](TECHNICAL.md#status).
