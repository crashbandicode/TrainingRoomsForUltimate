# Current release

<!-- Shipping a release: change the line below first; the steps are in docs/BUILD.md, "Release checklist". -->

Current release: rc.15

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/5c59ac60ffc033815119d657b463e73f2fadb636" autoplay muted loop playsinline controls></video>

*The menu over Training.* [Open the clip](media/tut-menu-tour.mp4)

| | |
| --- | --- |
| Last verified console install | `dc5302f1`, installed and [read back](EVIDENCE-rc15-installs.md#fixed-build) on 2026-11-27 |
| Repository main at this audit | `30f7f21a` (2026-11-27) |
| Shipping behavior (not a console receipt) | [Clip editor](guide/recording.md#crop), [Loading](menu/fighters-and-stages.md#loading) and [Tech practice](guide/tech-practice.md) describe the fixes. |
| Preview install cancelled | `cc71537c` was never installed: [preview notes](RELEASE-NOTES-rc15.md#preview-build) |
| Changes | [Release notes](RELEASE-NOTES-rc15.md) |
| Tests | [Install evidence](EVIDENCE-rc15-installs.md) |

## Which build is mine?

The menu footer shows `0.2.0-rc.15 · rc15`, then the build's commit.

| The tag ends with | Build | Installed |
| --- | --- | --- |
| `dc5302f1` | Fixed build | 2026-11-27 |
| `cc71537c` | Preview build | Never installed |
| `5df3d9dc` | Install 6 | 2026-11-26 |
| `b7a382e4` | Install 5 | 2026-11-25 |
| `a945b0c1` | Install 4 | 2026-11-25 |
| `e7667f26` | Install 3 | 2026-11-24 |
| `71158421` | Install 2 | 2026-11-24 |
| `152629cd` | Install 1 | 2026-11-23 |
| Other commit | Unlisted build | Check the release name; commit alone doesn't establish age. |

## What's new in rc.15

| Build | In short | More |
| --- | --- | --- |
| Fixed build | Fighter switching, return after scrubbing, menu help and Tech practice launches | [Notes](RELEASE-NOTES-rc15.md#fixed-build) |
| Preview | Scrubbing shows saved poses on matching fighters | [Notes](RELEASE-NOTES-rc15.md#preview-build) |
| Install 6 | Y on In, a free playhead, fighters shown while you scrub | [Notes](RELEASE-NOTES-rc15.md#install-6) |
| Install 5 | Record under an AI, Delete for captures, Change placement, Guard the Edge options | [Notes](RELEASE-NOTES-rc15.md#install-5) |
| Install 4 | Drills read moves alike for every fighter; the clip editor; Recordings; Trim | [Notes](RELEASE-NOTES-rc15.md#install-4) |

## Installing

See [Installing TrainingRooms](guide/getting-started.md#installing) for the required files and their destination.

## Known issues

These columns record the two earlier builds, not the behavior of the fixes linked above.

| Issue | On install 6 | In the preview build |
| --- | --- | --- |
| Scrub positions stop at 20 seconds on long takes | Open | Fixed |
| First X refused after a snap | Open; deselect the handle, then press X | Fixed |
| A fighter beside a wall is shown off its point | Open | Refusal host-tested; wall case not flown |
| Scrubbing shows places, not moves | By design | Poses when recorded; older captures show positions only |
| A save's Off stage fighter stays grounded | Open; switch On stage, then Off stage | Fixed |
| Guard the Edge rounds can diverge after release | Open; measured release coordinates matched | Unchanged |
| The recorded-fighter switch timed out in the preview | Not in this build | Affected preview only; the fix waits for the roster |
| Fighters slid apart after B from a posed frame | Not in this build | Seen in preview; [leaving the editor](guide/recording.md#crop) describes the fixed behavior |

Details: [release notes](RELEASE-NOTES-rc15.md#known-issues), [Install evidence](EVIDENCE-rc15-installs.md).

## Good to know

| Topic | Where |
| --- | --- |
| Test coverage: Eden runs and console reports, no full hardware test | [Install evidence](EVIDENCE-rc15-installs.md) |
| Waiting rooms have their own limits | [What is different, or refused](guide/waiting-rooms.md#what-is-different-or-refused) |
| A fighter outside Ready Fighters loads when you swap to it | [Fighter cycles](guide/fighter-cycles.md) |
| Limits kept from the Training Modpack | [Differences from the fork](DIFFERENCES-FROM-THE-FORK.md) |

## Earlier releases

| Release | Notes |
| --- | --- |
| rc.15, build by build | [rc.15 release notes](RELEASE-NOTES-rc15.md) |
| rc.5, rc.6, rc.8, rc.9 | [rc.5](RELEASE-NOTES-rc5.md), [rc.6](RELEASE-NOTES-rc6.md), [rc.8](RELEASE-NOTES-rc8.md), [rc.9](RELEASE-NOTES-rc9.md) |
| rc.12, rc.13 | [rc.12](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.12), [rc.13](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.13) |
| rc.7, rc.10, rc.11, rc.14 | No separate release notes |
