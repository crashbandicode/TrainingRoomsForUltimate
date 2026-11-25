# Current release

<!-- Whoever ships a release: change the `Current release:` line below first, then the README's top and the Status section
     of docs/TECHNICAL.md, and put the same release in Cargo.toml's version (0.2.0-rc.N, with Cargo.lock's own entry) in the
     release's name commit. The docs checks (tests/docs_gates.rs) fail until they all name the same release. The steps are
     in docs/BUILD.md, "Release checklist". -->

Current release: rc.15

Status: **rc.15, whose newest build is install 4** (commit `a945b0c1`, installed on the project's test console on
2026-11-25).

- **Installed:** the file on the card was read back and matches the build.
- **Tested in Eden, in part:** the final session checked drill grading, Trim, the clip editor and scroll-back in
  Training on `a9131331`. The reset-chord and Load-after retests passed on `a945b0c1`. The Quick Play match tested
  Online Deluxe's fonts, not TrainingRooms' waiting-room or Quick Play changes.
- **No recorded hardware test:** no report from play on the console is on record for install 4.

What ran, on which build, and what did not: [Install evidence](EVIDENCE-rc15-installs.md). What each build changed:
[rc.15 release notes](RELEASE-NOTES-rc15.md). This page says which release is current. The [README](../README.md) and
the [technical notes](TECHNICAL.md#status) follow it.

## Which build is mine?

Open the menu (B + D-pad Up) and read the end of the key hints at the foot of the page. rc.15 reads
`0.2.0-rc.15 · rc15` and then the commit it was built from. That commit tells the rc.15 builds apart:

| The tag ends with | Build | Installed |
| --- | --- | --- |
| `a945b0c1` | Install 4 | 2026-11-25 |
| `e7667f26` | Install 3 | 2026-11-24 |
| `71158421` | Install 2 | 2026-11-24 |
| `152629cd` | Install 1 | 2026-11-23 |

Builds before rc.14 start with `0.2.0-rc.6` whatever their release, so for them read the second part (`rc13`,
`rc12`, ...).
More: [Getting started › Which build you have](guide/getting-started.md#which-build-you-have).

## What's new in rc.15

Install 4 (2026-11-25), in short. The full list, and installs 1 to 3, are in the
[rc.15 release notes](RELEASE-NOTES-rc15.md).

- **Drills and combos read moves the same way for every fighter.** A step is one action: its release, hold, special
  directions and travel belong to it. [How a combo is graded](guide/recording.md#how-a-combo-is-graded)
- **The clip editor is a strip over the game,** with move marks, a snap and a preview on the CPU or Player 1.
  [Replay window](menu/recording.md#replay-window)
- **Recordings.** Play a Vault or saved capture back, or make a macro, combo or drill from it.
  [Recordings](menu/recording.md#recordings)
- **Trim** for Record and for each macro and drill, and a playback indicator.
  [Trim trailing neutral](menu/recording.md#trim-trailing-neutral)
- **Scroll the input display back** with R3 and D-pad Left or Right. [Frame strip](guide/frame-strip.md)
- **Move names** and **Match angle and deflection**, both Off by default.
  [Move names](menu/recording.md#move-names)

From the first rc.15 build (2026-11-15):

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

- **No recorded hardware test of install 4.** See the status above.
- **Not covered by the final Eden session:** the editor in the waiting room; the Quick Play practice pick, new pick
  and rematch; the airborne entry of a steered special; the deflection half of Match angle and deflection; ledge
  attacks; the Trim setting's migration; and the CPU's playback indicator.
  [Install evidence](EVIDENCE-rc15-installs.md#install-4-a945b0c1)
- **L + R + A pressed one button at a time.** A button pressed before the chord and still held when it completes
  belongs to the reset, and its drill result is taken back, so a result can show for a moment first.
  [Input drills](guide/recording.md#input-drills)
- **Fighter loading.** A fighter that doesn't fit in Ready Fighters loads when you swap to it, so that swap takes a
  moment ([Fighter cycles](guide/fighter-cycles.md#what-the-loading-rows-do)).
- **Limits kept from the Training Modpack** are listed in
  [Differences from the fork](DIFFERENCES-FROM-THE-FORK.md#kept-from-m5q).
- **Waiting rooms** have their own limits and refusals:
  [What is different, or refused](guide/waiting-rooms.md#what-is-different-or-refused).

Something else? [Troubleshooting](guide/troubleshooting.md) says what to check and what to send.

## Earlier releases

- The builds of rc.15, install by install: [rc.15 release notes](RELEASE-NOTES-rc15.md).
- Release notes in this folder: [rc.5](RELEASE-NOTES-rc5.md), [rc.6](RELEASE-NOTES-rc6.md),
  [rc.8](RELEASE-NOTES-rc8.md) and [rc.9](RELEASE-NOTES-rc9.md).
- On their release pages: [rc.12](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.12),
  [rc.13](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.13)
  and [rc.14](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.14).
- rc.7, rc.10 and rc.11 have no notes: see [Status](TECHNICAL.md#status).
