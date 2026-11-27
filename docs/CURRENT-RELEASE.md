# Current release

<!-- Shipping a release: change the line below first; the steps are in docs/BUILD.md, "Release checklist". -->

Current release: rc.15

| | |
| --- | --- |
| On the test console | Install 6, built from `5df3d9dc`, installed 2026-11-26 and read back |
| Staged, not installed | A preview build from `cc71537c`: [what it changes](RELEASE-NOTES-rc15.md#preview-build) |
| What each build changed | [rc.15 release notes](RELEASE-NOTES-rc15.md) |
| What was tested, and how | [Install evidence](EVIDENCE-rc15-installs.md) |

## Which build is mine?

Open the menu with B + D-pad Up and read the end of the key hints: `0.2.0-rc.15 · rc15`, then the commit.

| The tag ends with | Build | Installed |
| --- | --- | --- |
| `cc71537c` | Preview build | Not installed; staged 2026-11-27 |
| `5df3d9dc` | Install 6 | 2026-11-26 |
| `b7a382e4` | Install 5 | 2026-11-25 |
| `a945b0c1` | Install 4 | 2026-11-25 |
| `e7667f26` | Install 3 | 2026-11-24 |
| `71158421` | Install 2 | 2026-11-24 |
| `152629cd` | Install 1 | 2026-11-23 |
| Anything else | An older release | [Which build you have](guide/getting-started.md#which-build-you-have) |

## What's new in rc.15

| Build | In short | More |
| --- | --- | --- |
| Preview | Scrubbing shows each fighter's recorded move; fixes to install 6 | [Notes](RELEASE-NOTES-rc15.md#preview-build) |
| Install 6 | Y on In, a free playhead, fighters shown while you scrub | [Notes](RELEASE-NOTES-rc15.md#install-6) |
| Install 5 | Record under an AI, Delete for captures, Change placement, Guard the Edge options | [Notes](RELEASE-NOTES-rc15.md#install-5) |
| Install 4 | Drills read moves alike for every fighter; the clip editor; Recordings; Trim | [Notes](RELEASE-NOTES-rc15.md#install-4) |

## Installing

Copy `libtraining_rooms.nro` and `libnro_hook.nro` into the Skyline plugins folder: [Getting started](guide/getting-started.md).

## Known issues

| Issue | On install 6 | In the preview build |
| --- | --- | --- |
| Scrub positions stop at 20 seconds on long takes | Open | Fixed |
| First X refused after a snap | Open; deselect the handle, then press X | Fixed |
| A fighter beside a wall is shown off its point | Open | That point is refused |
| Scrubbing shows places, not moves | By design | Shows the recorded move |
| A save's Off stage fighter stays grounded | Open; switch On stage, then Off stage | Fixed |
| Guard the Edge's start differs between rounds | Open | Unchanged |
| The recorded-fighter switch times out in the editor | Not in this build | Open; fix in work |
| Fighters slide apart after B from a posed frame | Not in this build | Open; fix in work |

What happens in each, and the runs behind them: [release notes](RELEASE-NOTES-rc15.md#known-issues) and
[Install evidence](EVIDENCE-rc15-installs.md).

## Good to know

| Topic | Where |
| --- | --- |
| Test coverage: Eden runs and console reports, no full hardware test | [Install evidence](EVIDENCE-rc15-installs.md) |
| Waiting rooms have their own limits | [What is different, or refused](guide/waiting-rooms.md#what-is-different-or-refused) |
| A fighter outside Ready Fighters loads when you swap to it | [Fighter cycles](guide/fighter-cycles.md) |
| Limits kept from the Training Modpack | [Differences from the fork](DIFFERENCES-FROM-THE-FORK.md) |
| Something else | [Troubleshooting](guide/troubleshooting.md) |

## Earlier releases

| Release | Notes |
| --- | --- |
| rc.15, build by build | [rc.15 release notes](RELEASE-NOTES-rc15.md) |
| rc.5, rc.6, rc.8, rc.9 | [rc.5](RELEASE-NOTES-rc5.md), [rc.6](RELEASE-NOTES-rc6.md), [rc.8](RELEASE-NOTES-rc8.md), [rc.9](RELEASE-NOTES-rc9.md) |
| rc.12, rc.13, rc.14 | [rc.12](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.12), [rc.13](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.13), [rc.14](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.14) |
| rc.7, rc.10, rc.11 | No notes: see [Status](TECHNICAL.md#status) |
