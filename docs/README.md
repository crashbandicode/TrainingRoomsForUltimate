# TrainingRooms documentation

Start with the [README](../README.md) for installing and the buttons. Each page of the in-game menu has its own doc
below, with every option on it; the guides explain a topic across pages.

## Start here

| Page | Covers |
| --- | --- |
| [Getting started](guide/getting-started.md) | What you need, installing, the first start, opening the menu, a first look around, and how to read the build tag. |
| [Current release](CURRENT-RELEASE.md) | Which release is current, what's new in it, how to install it and its known issues. |
| [Troubleshooting](guide/troubleshooting.md) | What you may see, why, and what to do: the plugin or the menu not loading, greyed rows, the notices that say no, slow fighter loading, waiting rooms, settings, and where the logs are. |

## The menu, page by page

Open the menu with B + D-pad Up (or Minus). L and R turn the pages.

| Page | What's on it |
| --- | --- |
| [CPU](menu/cpu.md) | How the CPU reacts: mash and followups, DI and SDI, ledge and tech options, shield, attacks and movement, letting a CPU play as Player 1, the CPU's level. |
| [Saves](menu/saves.md) | Save states: mirroring, auto-load on death, damage on load, buffs; the setup library. |
| [Recording](menu/recording.md) | Your saves, macros, combos and sequences; replays; playback and combo helper options; the Recording pop-up. |
| [Fighters & Stages](menu/fighters-and-stages.md) | Fighters for you and the CPU, character cycles, Kirby hats, items, Mii fighters, stages, waiting-room practice, loading options. |
| [Display & Controls](menu/display-and-controls.md) | The HUD, hitboxes, frame advantage, the input display, input delay; button chords, reset, menu shortcuts. |

## Guides

| Guide | Covers |
| --- | --- |
| [Using the menu](guide/using-the-menu.md) | Opening it, moving around, each kind of option, saving your defaults, resetting. |
| [Controls](guide/controls.md) | Every button chord, its default and how to change it; GameCube controllers; Online Deluxe and the CPU pilot. |
| [CPU behaviour](guide/cpu-behaviour.md) | How mash, followups, triggers and overrides combine; the direction grids; delays. |
| [Save states](guide/save-states.md) | Saving and loading, what a state keeps, damage, mirroring, buffs, the setup library. |
| [Recording](guide/recording.md) | Recording and playback, the pop-up, named macros and chains, loop, placement, replays. |
| [Combos](guide/combos.md) | The combo helper, saving and drilling combos, sequences. |
| [Frame strip](guide/frame-strip.md) | Reading the input display: lanes, frame advantage, input timing. |
| [Game speed](guide/game-speed.md) | The speed pop-up and the stopwatch, in Training and the waiting rooms. |
| [Fighter cycles](guide/fighter-cycles.md) | Cycling fighters, the Character Cycles editor, Solo Popo, Kirby hats, loading. |
| [Stages](guide/stages.md) | Change Stage, Lobby Stages and their forms, stage rotation, hazards. |
| [Waiting rooms](guide/waiting-rooms.md) | Practising in online and local waiting rooms: what works there and what doesn't. |
| [Mii fighters](guide/mii.md) | Picking a saved Mii, making a new one, replay Miis. |

## Reference

- [Technical notes](TECHNICAL.md): how it's installed and built, how each feature works inside,
  [the hidden Developer page](TECHNICAL.md#the-developer-page), the build status.
- [Feature ledger](FEATURE-LEDGER.md): every feature with the test runs and clips that show it working: rc.11, and
  a section for rc.13 (Eden-tested, untested on hardware); rc.12 has no rows in it.
- [Media](MEDIA.md): where each GIF was recorded.
- [Restrictions](RESTRICTIONS.md): every message that refuses something, and the approval behind it.
- [Building](BUILD.md) and [differences from the Training Modpack](DIFFERENCES-FROM-THE-FORK.md).
- Release notes: [rc.5](RELEASE-NOTES-rc5.md), [rc.6](RELEASE-NOTES-rc6.md), [rc.8](RELEASE-NOTES-rc8.md),
  [rc.9](RELEASE-NOTES-rc9.md) in this folder; rc.12's, rc.13's and rc.14's are on their
  [release pages](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases). rc.7, rc.10 and rc.11 have
  none; see [Status](TECHNICAL.md#status).

## Project records

Kept for the history of decisions and measurements; they describe the build of their date, not necessarily this one.

- [Decisions](DECISIONS.md) and [decisions/](decisions/), [design/](design/), [the menu redesign](menu-redesign.md),
  [the menu mockup](menu-mockup/), the [roadmap](ROADMAP.md).
- [Mii developer notes](MII.md).
- Measurements: [code reduction (rc.10)](CODE-REDUCTION-rc10.md), [coverage (rc.9)](COVERAGE-rc9.md),
  [size plan](SIZE-PLAN.md), [test cuts (rc.9)](TEST-CUT-LEDGER-rc9.md), [resource costs](resource-costs.md),
  [asset reproduction](ASSET-REPRODUCTION-R15.md).
- Provenance: [provenance/](provenance/), [test-provenance/](test-provenance/).
