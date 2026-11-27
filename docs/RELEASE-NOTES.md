# Release notes

<a id="release-notes-rc15"></a>

<a id="release-notes-rc15--rc-15-the-builds-of-november-2026"></a>

## rc.15: the builds of November 2026

| Build | Installed | Footer ends with | In short |
| --- | --- | --- | --- |
| [Fixed build](#release-notes-rc15--fixed-build) | 2026-11-27 | `dc5302f1` | Clip editor, menu help and Tech practice fixes |

These notes cover the November builds of the untagged rc.15 candidate. Each build's menu footer reads
`0.2.0-rc.15 · rc15` and then the commit it was made from, so the commit tells the builds apart
([Which build you have](guide/getting-started.md#which-build-you-have)).

The first six installs are numbered. The preview was skipped; the fixed build replaced install 6. Times are UTC.

| Build | Installed | Footer ends with | In short |
| --- | --- | --- | --- |
| [Fixed build](#release-notes-rc15--fixed-build) | 2026-11-27 | `dc5302f1` | Clip editor, menu help and Tech practice fixes |
| [Preview build](#release-notes-rc15--preview-build) | Install cancelled 2026-11-27 | `cc71537c` | Scrubbing shows recorded poses on matching fighters; fixes to scrubbing and save placement |
| [Install 6](#release-notes-rc15--install-6) | 2026-11-26 | `5df3d9dc` | The clip editor: Y on In makes a capture previewable, a free playhead, and the fighters shown while you scrub |
| [Install 5](#release-notes-rc15--install-5) | 2026-11-25 | `b7a382e4` | Record with an AI on Player 1, named grab and throw keys, B stops a preview, Delete for replay captures, named move marks, playback that starts without reading the library, Guard the Edge and Juggler use the floor you stand on, the eggs' off-stage zone, a save's placement changed with the crosshair |
| [Install 4](#release-notes-rc15--install-4) | 2026-11-25 | `a945b0c1` | Drills and combos read moves the same way for every fighter; the clip editor over the game; Recordings; Trim; input scroll-back |
| [Install 3](#release-notes-rc15--install-3) | 2026-11-24 | `e7667f26` | Two fixes to install 2 |
| [Install 2](#release-notes-rc15--install-2) | 2026-11-24 | `71158421` | Fixes to Recording, the practice games and the menu |
| [Install 1](#release-notes-rc15--install-1) | 2026-11-23 | `152629cd` | Fixes for defects found on the console that day |

What "tested" means in these notes:

- **Installed:** the file on the card was read back and matched the build. That shows the build is there, not how it plays.
- **Tested in Eden:** a named run in the Eden emulator gave a verdict for that item, on the build named beside it.
- **The operator reported:** the project's operator played that build on the console and said what they saw. These
  are reports, not a list of passes, and only reports that are on record are written.

The runs, verdicts and file hashes behind every line are in [Install evidence](EVIDENCE.md#evidence-rc15-installs).

<a id="release-notes-rc15--fixed-build"></a>

### Fixed build

| Area | What changed |
| --- | --- |
| [Clip editor](guide/recording.md#crop) | Switching fighters waits for loading; the footer says when B can still cancel. |
| Leaving a posed frame | Both fighters return to their original spots without sliding. |
| Menu help | Clarifies Loading, Group Swap, L+R+A and Guard the Edge's CPU-first setup. |
| [Tech practice](guide/tech-practice.md) | Factory launches reach a surface, including walls, so attempts can be graded and counted. |

[Card readback and build-specific tests](EVIDENCE.md#evidence-rc15-installs--fixed-build).

<a id="release-notes-rc15--preview-build"></a>

### Preview build

| Preview | Details |
| --- | --- |
| Built from | `cc71537c` (the footer ends with it) |
| Release file | `78db86f5…`, 5,419,008 bytes |
| At cancellation | Install 6 was still on the card. |

This preview was tested in Eden. Its planned console install was cancelled; it is not the next release.


<a id="release-notes-rc15--scrubbing-shows-the-recorded-moves"></a>

#### Scrubbing shows the recorded moves

On install 6 a scrubbed fighter was moved to its recorded place and kept the pose it had. On this build it also
shows the recorded move when that frame has a pose and the fighter matches the take.

| What | How it works |
| --- | --- |
| A macro recorded on this build | Record keeps each frame's animation with its position |
| A match capture or a Vault recording made on this build | Keeps each frame's animation too |
| A macro without recorded moves | Still loads and plays. The editor reads "Play it once to see the moves while scrubbing." |
| An older capture | Still loads. It shows positions only |
| Fighter choice | Each side shows the fighter on stage or the take's recorded fighter. ZL + L requests the switch for Player 1; ZL + R requests it for the CPU. The Player 1 switch did not finish in the editor; the CPU switch was not tested |
| Not shown | Projectiles, items and damage, as before |

<a id="release-notes-rc15--fixes-to-install-6"></a>

#### Fixes to install 6

| Install 6 | This build |
| --- | --- |
| A long take kept only its first 1,200 frames of scrub positions | It keeps them all. A 1,610-frame take had 1,610 |
| The first X after a snap could be refused | X plays after a held frame |
| A fighter beside a wall was shown off its point | The editor refuses that point instead: "Can't stand there on this stage." |
| At a refused point the snap hint could hide that line | The refusal line shows |
| A save's Off stage fighter stayed grounded once it touched the ground | It floats again |

Guard the Edge's uneven start is not changed by this build.

<a id="release-notes-rc15--known-issues"></a>

#### Known issues

| Issue | What happens | Way round | Status |
| --- | --- | --- | --- |
| Fighter switch times out | After ZL + L the editor waits, then reads "The fighter did not load; the editor keeps the current one". The open editor blocks the roster from applying the switch. After 600 worker passes it gives up without withdrawing the request. In Eden, Player 1 changed fighters only after the editor closed; the CPU side was not tested | None established | Not fixed in this preview |
| Fighters slide apart after B | Both fighters return to their places, but their push boxes can keep the width of the scrubbed pose for a few ticks. The boxes still overlap, so the game pushes the fighters apart. A quick B left each about 0.6 units off in four of six tries | None established | Not fixed in this preview |

These results describe the cancelled preview, not the [clip editor in the fixed build](guide/recording.md#crop).

<a id="release-notes-rc15--tested"></a>

#### Tested

| Check | Preview build |
| --- | --- |
| Installed | No. The staged copy was removed when the install was cancelled |
| Host | Tests and all builds passed on `cc71537c`. Refusing a fighter pushed by a wall, and captures keeping their animation, are host-tested only |
| Eden, on the release file, first boot | No crash. Passed: a long take, snapped X, the refusal at a point inside the stage, a save's fighter floating again, the settings check, and the menu in a waiting room as host. Scrub poses showed for both fighters |
| Eden, on the release file, second boot | No crash. Snapped X, scrub poses and the settings check passed again. The fighter choice failed as above |
| Eden, on the diagnostic build | Three chosen frames were shown exactly, and X and B let the fighters go. The slide was found there |
| Not flown | A capture saved and read back in a game; a fighter pushed by a wall; a macro gaining its moves from one Play; the CPU side of the fighter choice; a second player in a waiting room |
| On the console | The preview install was cancelled. |

The runs behind these: [Install evidence](EVIDENCE.md#evidence-rc15-installs--preview-build-cc71537c).

<a id="release-notes-rc15--install-6"></a>

### Install 6

| Install 6 | Details |
| --- | --- |
| Installed | 2026-11-26 (12:22), and read back |
| Build | `5df3d9dc`. Still an rc.15 build: the footer reads `0.2.0-rc.15 · rc15 5df3d9dc` |
| Holds | install 5, plus three changes to the clip editor and one fix to the third, all merged on 2026-11-26 |

All three changes are in the clip editor of the [Replay window](menu/recording.md#replay-window).

<a id="release-notes-rc15--y-on-in-makes-a-capture-previewable"></a>

#### Y on In makes a capture previewable

No install 6 clip shows a successful match-capture preview after this snap. The [later preview-build clip](media/tut-clip-snap.mp4) shows Y snapping In and X playing the capture.

On a match capture, plain Y could leave In in the middle of an action, and X and A were then refused. The operator
found this on install 5, where the way round is ZL or ZR + Y: [Known issues](CURRENT-RELEASE.md#known-issues).

| With In selected | What Y does in install 6 |
| --- | --- |
| The mark is in the middle of an action | Snaps In to the mark and also takes the move's short setup, as ZL or ZR + Y does, so X and A work from it |
| No move can start at that frame | The strip says why |
| Y a second time | Keeps the same move |

| Checked | Result |
| --- | --- |
| Host tests | All 1,176 crop ranges of a 48-frame capture: every preview that worked on install 5 kept its starting state and inputs. The check behind "In is mid-action" is unchanged |
| Eden, on `eb39d6bc` | With In one frame into an attack, X played nothing and the refusal showed; Y moved In to the attack's start; X then previewed and A saved a macro |
| Eden, on install 6's diagnostic build | A second Y kept the same move, and the fast snap landed. The preview after a snap is not shown there: no preview in the Replay window fed a frame in that boot |
| Known issue in install 6 | With In still selected after Y, X can be refused with "Let the CPU settle to idle and leave the ledge before previewing". Press L to deselect In, then X: [Known issues](CURRENT-RELEASE.md#known-issues) |

<a id="release-notes-rc15--a-free-playhead-in-the-clip-editor"></a>

#### A free playhead in the clip editor

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/4a1682c8e3a70a876ed2764fc4e6629bbd6cfcdc" autoplay muted loop playsinline controls></video>

*With neither handle selected, Right moves the playhead. Y takes it to the start of the nearest move, at 66. L sets In there; R later keeps frame 100 as the last frame (Out 101). Eden diagnostic build `5df3d9dc`, 2026-11-26.* [Open the clip](media/tut-clip-playhead.mp4)

With no handle selected, on a macro or a capture:

| Key | What it does |
| --- | --- |
| Left, Right | Move the playhead over the whole take, inside or outside the kept part. In and Out do not change |
| Y | Moves the playhead to the nearest move mark. An unmarked take boundary is not a snap target |
| L, once the playhead has moved | Makes its frame the first kept frame |
| R, once the playhead has moved | Makes its frame the last kept frame (Out is one frame later) |

- Neither cut can cross the other handle: the editor says which to set first and changes nothing.
- An In set this way is like one you nudged: if it is mid-action on a capture, Y resolves the move as above.
- Combos and drills are unchanged.

| Checked | Result |
| --- | --- |
| Eden, on `eb39d6bc` | With nothing selected the playhead moved to two other frames while In and Out stayed where they were, and the strip read "Playhead moved. L sets the start here, R the end." |
| Eden, install 6 diagnostic build | In the macro demonstration, Y snapped from 64 to a mark at 66. L set In to 66; R later kept frame 100 (Out 101). This is footage, not a verdict for a test row. |
| Eden test rows, same build | The playhead's browse, cut, no-move and snap rows passed |
| No host test | Holding a direction on a pad |

<a id="release-notes-rc15--the-fighters-are-shown-while-you-scrub"></a>

#### The fighters are shown while you scrub

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/867d9322ecba30bf1c344ca71e5a3b394461e594" autoplay muted loop playsinline controls></video>

*Crop on Macro 3: L and R select the handles, and the fighters follow the selected frame without playing a preview. Eden diagnostic build `5df3d9dc`, 2026-11-26.* [Open the clip](media/tut-clip-scrub.mp4)

With In or Out selected, or once the playhead has moved, both fighters are shown where the take had them on that
frame, without playing a preview. Both use the target's position when the hold begins, so their recorded spacing
is kept. This works on a macro and on a match capture.

| The macro | Where its positions come from |
| --- | --- |
| Recorded on this build | Record saves them in a second file beside the macro |
| A crop, or a capture saved as a macro | It carries its positions |
| Without positions | Play it by itself from its first input, with Play or with X from the start of the take, and they are saved; you do not record it again. After X in the editor they show at once |

| The editor reads | When |
| --- | --- |
| "Play it once to see positions while scrubbing." | The macro has no positions |
| "Positions end here." | The selected frame is beyond the saved positions. This can follow an early stop during generation or the long-recording defect below. |
| "Can't stand there on this stage." | The editor detected a point inside solid ground or a wall, outside the camera limits, or past the safe blast-line bounds. A held fighter returns to its hold origin; the other fighter is judged separately. Install 6 judges the point only, not whether the fighter's body fits beside a wall: see below. |

| Rule | What it means |
| --- | --- |
| The macro's own file | Stays the same: older builds can still read new macros, older macros still load and play on this build, and Play reads what it read before. The macro does not need positions to play |
| Positions made by a playback | Are that playback's, not the original recording's |
| A valid positions file saved by Record | Is never replaced by a playback. A damaged or mismatched file counts as missing, so an eligible playback can replace it |
| No positions file is made | For a chain step, when the playing character is a different fighter kind from the one recorded, or when Trim skips the first input |
| Where the three lines show | A snapped move's temporary setup gets the notice first. Otherwise a clean In shows the position message; a mid-action In shows the snap hint instead. This can hide "Can't stand there on this stage." Fixed in the next build. |
| Fighter pose | Frozen or neutral; this is not a replay of the animation |
| Not shown | Projectiles, items and damage. Combos and drills have no position display |

**Long recordings:** two takes kept all 1,553 and 3,253 input frames but only the first 1,200 frames of scrub
positions. Both were recorded with the menu closed. This is not a length limit on playback.
[Known issues](CURRENT-RELEASE.md#known-issues)

**Fixed before the card build** (merged 2026-11-26): at first only the camera's limits were tested, so a point inside
the ground counted as a place to stand. An Eden session on the build before this fix found it on a match capture:
both fighters stood beside their recorded points while the editor said they were held.

| Checked | Result |
| --- | --- |
| Holds on `eb39d6bc` | Both fighters followed In and Out without a preview; deselecting released them |
| Old macro and crop on `eb39d6bc` | The no-positions line appeared. One Play generated positions; X showed them immediately in the editor. A crop kept them. |
| Play start on `eb39d6bc` | Each tested macro started on install 5's frame or one earlier, including the first Plays of the old 30-second and ten-minute macros |
| Timing coverage | One Eden host, one boot per build. The playback path is unchanged; install 6 and the console were not timed here. |
| Eden, on install 6's diagnostic build | Record with no preview, In and Out following the handles, release on deselect, the old-macro line, a crop keeping its positions, and the file sizes passed. Deleting a macro removed both its files. The menu thread used 49,904 B of stack, under its 57,648 B bound |
| A point where a fighter cannot stand, on that build | On a match capture the fighter whose point was inside the floor was not moved. The strip showed the hint to snap, not "Can't stand there on this stage.", because In was mid-action there |
| Known issue found there | A point just left of the end of the floor and below it was free, but the fighter's body did not fit beside the wall: the game pushed it about 5 units from its recorded point, and the editor still treated it as held. [Known issues](CURRENT-RELEASE.md#known-issues) |
| Release, on that build | In Crop on a recorded macro, X played the preview and let the fighters go after it. B closed the editor with both fighters back at their homes. This is not the match-capture preview. |
| Not established yet | Scrub behavior in a waiting room, or through a fighter swap or scene change. A fighter walked off stage gave no verdict yet |
| The installed release file | No smoke boot yet; the rows above ran the diagnostic build of the same commit |
| Host | No host test runs the Switch code that holds the fighters |

<a id="release-notes-rc15--tested-1"></a>

#### Tested

| Check | Install 6 |
| --- | --- |
| Installed | Yes, and read back. TrainingRooms was the only one of the 21 active plugins replaced. A rollback copy of install 5 was also saved on the card |
| Host | Every merge passed host tests and builds |
| Eden, on the build before the fix (`eb39d6bc`), not on this exact build | Play started no later than on install 5; a take was recorded and held at In with no preview; an old macro got its positions from one Play; Y on In made a capture previewable; and the playhead browsed a take. The same session found the defect that the fix addresses |
| Eden, on install 6's diagnostic build (same commit) | Passed in this run: record and hold, release on X and on close, the old macro, a crop, delete, a take on the CPU, the playhead rows, the snap rows up to their preview, and the menu's stack check. Found: the three issues listed under [Known issues](CURRENT-RELEASE.md#known-issues) as found in testing. These results cover neither a smoke boot of the installed release file nor a waiting-room pair |
| On the console | Scrubbing moved the fighters but did not show the moves. The operator requested recorded poses and a choice of recorded or current fighter for each role; neither is in install 6. |

The runs and what they did not cover: [Install evidence](EVIDENCE.md#evidence-rc15-installs--install-6-5df3d9dc).

<a id="release-notes-rc15--install-5"></a>

### Install 5

| Install 5 | Details |
| --- | --- |
| Installed | 2026-11-25 (23:06), and read back |
| Build | `b7a382e4`. Still an rc.15 build: the footer reads `0.2.0-rc.15 · rc15 b7a382e4` |
| Holds | the ten changes below, all merged on 2026-11-25 |

| Change | In short |
| --- | --- |
| [Record with an AI on Player 1](#release-notes-rc15--record-works-while-an-ai-plays-player-1) | Record starts instead of being refused |
| [Named grabs, throws and edge attacks](#release-notes-rc15--grabs-throws-and-edge-attacks-are-named-in-combo-files) | Five more moves are written by name in combo files |
| [B stops a preview](#release-notes-rc15--b-stops-a-preview-and-the-pop-up-shows-its-chords) | And the Recording pop-up shows your chords |
| [Delete for replay captures](#release-notes-rc15--delete-for-replay-captures) | A Delete row, and Y in the Replay window's capture list |
| [Named move marks](#release-notes-rc15--the-clip-editor-s-marks-name-the-form-of-a-move) | Jab 1, Jab 2, Forward air on the editor's timeline |
| [Playback start](#release-notes-rc15--playback-starts-without-reading-the-library) | Play no longer reads the whole library; a Playback delay option |
| [Guard the Edge and Juggler](#release-notes-rc15--guard-the-edge-and-juggler-start-from-the-floor-you-stand-on) | They start from the floor you stand on |
| [Two Guard the Edge options](#release-notes-rc15--two-guard-the-edge-options) | Under the stage; CPU midair jumps |
| [Eggs-ercise](#release-notes-rc15--eggs-ercise-s-off-stage-zone-follows-the-stage) | The off-stage zone follows the stage |
| [Change placement](#release-notes-rc15--a-save-s-placement-is-changed-with-the-crosshair) | Move where a save puts the fighters, with the crosshair |

<a id="release-notes-rc15--record-works-while-an-ai-plays-player-1"></a>

#### Record works while an AI plays Player 1

*No clip: Record under an AI is not filmed.*

An AI here is the CPU pilot plugin, or TrainingRooms' own Player 1 control.

| While an AI plays Player 1 | Install 5 | Before |
| --- | --- | --- |
| Record | Starts. The take holds Player 1's inputs as the game receives them, the AI's included | Refused |
| Play on Player 1 | Refused | Refused |
| L + R + A in a waiting room (Quick Reset) | Refused | Refused |

More: [Controls › When the CPU pilot plays Player 1](guide/controls.md#when-the-cpu-pilot-plays-player-1).

<a id="release-notes-rc15--grabs-throws-and-edge-attacks-are-named-in-combo-files"></a>

#### Grabs, throws and edge attacks are named in combo files

<a id="release-notes-rc15--combo-reader-through-a945b0c1"></a>

| Combo files | Install 5 |
| --- | --- |
| Written by name | `grab`, `throw`, `ledge_attack`, `getup_attack` and `trip_attack`, in a link's `from` and `to` |
| **Not backward compatible** | Install 4 and earlier skip a file that has one of these five there |
| Not affected | Files without them. This build still reads older files |

More: [Combo files › The format](guide/combos.md#the-format).

<a id="release-notes-rc15--b-stops-a-preview-and-the-pop-up-shows-its-chords"></a>

#### B stops a preview, and the pop-up shows its chords

*No clip: B stopping a preview is not filmed.*

| When | The key line reads | What happens |
| --- | --- | --- |
| While any preview plays | `B stop preview` | B or X stops it and drops the rest of its frames |
| After the last frame, for about a second | `B back` | The result stays on screen |

The Recording pop-up's first help line shows the Record, Play on Player 1 and Play on CPU chords as you have set
them.

More: [Recording › Replay window](menu/recording.md#replay-window) and
[The Recording pop-up](menu/recording.md#the-recording-pop-up).

<a id="release-notes-rc15--delete-for-replay-captures"></a>

#### Delete for replay captures

*No clip: deleting a capture is not filmed.*

| Where | How |
| --- | --- |
| A capture's screen under RECORDINGS | The **Delete** row |
| The Replay window's capture list, before opening the editor | Y deletes the highlighted capture |

| Delete | What it does |
| --- | --- |
| Asks first | No. The capture is removed at once, as a macro is |
| Keeps | Macros, combos and drills already made from it. The game's own replay in the Vault is untouched |

More: [Recording › Recordings](menu/recording.md#recordings).

<a id="release-notes-rc15--the-clip-editor-s-marks-name-the-form-of-a-move"></a>

#### The clip editor's marks name the form of a move

| Move marks | Install 5 |
| --- | --- |
| A mark keeps the form of its move | The first, second or third jab, the rapid jab or its finisher, which aerial, grab or throw, and a side tilt or smash aimed up or down |
| With Move names On | A mark reads as the combo and drill lists do, for example Jab 1, Jab 2 or Forward air |
| A take saved before this | Has no forms: its jab reads Jab |
| Specials | A take, and each fighter of a replay capture, stores the fighter it was recorded on, so its specials keep that fighter's own names whoever Player 1 is |

More: [Recording › Replay window](menu/recording.md#replay-window) and [Move names](menu/recording.md#move-names).

<a id="release-notes-rc15--playback-starts-without-reading-the-library"></a>

#### Playback starts without reading the library

| Play's start | Install 5 |
| --- | --- |
| What it reads | The index, the file each side plays and its start save. Not the whole recording library, and nothing while a macro plays |
| Time from pressing Play to the first input | In one Eden run on 2026-11-26: 9 frames for the 3-second macros; 10 for the newly recorded 30-second take and 11 for the old one; 27 for the single start of the old ten-minute macro. Not timed on console. [Timing comparison](EVIDENCE.md#evidence-rc15-installs--eden-on-the-build-before-the-fix-eb39d6bc) |
| Known limit | If Play on Player 1 starts in the middle of one of Player 1's input polls, that one poll can pass what you hold instead of nothing. It is one frame at most, at the start |

| Playback delay | A new option under Recording › Options |
| --- | --- |
| Values | None by default; 10 to 120 frames |
| When it waits | After the fighters are placed and before the first input, once per Play |
| Applies to | Play on Player 1, Play on CPU, a capture's Play back and Make combo's pass |
| Does not apply to | The editor's previews, or the CPU's Playback Slots |

More: [Playback delay](menu/recording.md#playback-delay).

<a id="release-notes-rc15--guard-the-edge-and-juggler-start-from-the-floor-you-stand-on"></a>

#### Guard the Edge and Juggler start from the floor you stand on

*No clip of the choice of floor. Each drill's set-up is filmed in its guide:
[Guard the Edge](guide/guard-the-edge.md) and [Juggler](guide/juggler.md).*

| Rule | What it means |
| --- | --- |
| Where a drill starts | From the floor you are standing on, not from the middle of the stage. On a stage with more than one piece of ground, such as the Training stage, stand on the piece you want and choose Start |
| Guard the Edge's sides | Each end of that floor that has open air beside it. A floor with one such end has one side |
| Juggler | Needs no ledge: its CPU spot is over the stage or in open air beside it |
| A start you set up | Is used again only on the floor it was made on |

| When | The message |
| --- | --- |
| Start cannot begin the drill; the row returns to Start | "The stage could not be read", "Stand on the stage to start this drill" or "This floor has no ledge to guard" |
| Juggler's set-up refused the CPU spot | "Pick a point over the stage or beside it". Stay on the CPU page, move the crosshair and press **A** again |

Before this change, the Training stage check left the menu showing Running even though neither drill began
([Install evidence](EVIDENCE.md#evidence-rc15-installs--install-5-b7a382e4)).

More: [Fighters & Stages › Guard the Edge and Juggler](menu/fighters-and-stages.md#guard-the-edge-and-juggler).

<a id="release-notes-rc15--two-guard-the-edge-options"></a>

#### Two Guard the Edge options

*No clip: the two options are not filmed.*

| Option | Default | The other setting |
| --- | --- | --- |
| **Under the stage** | Off | On: the CPU's start can also be a point under the stage |
| **CPU midair jumps** | Kept | Used: the CPU starts every attempt with its midair jumps already spent |

| Known limit | |
| --- | --- |
| A point just under the stage | Can put the fighter's upper body inside the stage |
| Perfect mode | Its route from under the stage is not established |
| The fighter's own moves | Whether they give the jumps back is not known |

More: [Fighters & Stages › Guard the Edge and Juggler](menu/fighters-and-stages.md#guard-the-edge-and-juggler).

<a id="release-notes-rc15--eggs-ercise-s-off-stage-zone-follows-the-stage"></a>

#### Eggs-ercise's off-stage zone follows the stage

*No clip: the zone on another stage is not filmed.*

| The off-stage zone | Install 5 |
| --- | --- |
| Where a zone lies | Beside a ledge of the stage's main floor that has open air next to it, so a stage can have two zones, one or none |
| Its outer edge | Stops where that open air ends |
| On the Training stage | The one zone is the gap between the long floor and the island |
| Where a target appears | Only in the air off stage: never inside another part of the stage, and never above a floor or a platform |
| Known limits | A platform that moves after the stage is read is not followed, and a piece of the stage narrower than 10 units can be missed |

More: [Fighters & Stages › Eggs-ercise](menu/fighters-and-stages.md#eggs-ercise).

<a id="release-notes-rc15--a-save-s-placement-is-changed-with-the-crosshair"></a>

#### A save's placement is changed with the crosshair

*No clip: Change placement is not filmed.*

**Change placement** moves where a save state puts the fighters. The menu closes and a picker opens on the live
game: move the fighters with the crosshair.

| The picker | |
| --- | --- |
| Pages | Where, Spot, State, Percent and Preview |
| To save | A on Preview, or + from any page |
| Opens from | The Recording page; a save's, a macro's or a chain's own screen; X on a save in the pop-up. For a macro it changes the macro's start save |

If at least one macro starts from the save, the picker normally asks whether to change it or make a copy.
**Shared start save** can answer ahead: Always change or Always copy instead of Prompt.

| A copy made from | Which macro links change |
| --- | --- |
| A macro | That macro's start-save link goes to the copy |
| A chain | Only its first macro gets that link |
| The save itself | None |

| A fighter switched to Off stage, with the spot | Does this |
| --- | --- |
| In open air or under the stage | Stays where it is |
| Over the stage or inside it | Goes to the open air beside the nearest ledge, and the crosshair then moves freely |

The picker preserves each macro's inputs, Momentum and offset. A macro with an offset from an earlier build
keeps it until you clear it with the Offset row on the macro's own screen.

More: [Recording › Change placement](menu/recording.md#change-placement) and
[the guide](guide/recording.md#change-placement-and-the-replay-window).

<a id="release-notes-rc15--tested-2"></a>

#### Tested

| Check | Install 5 |
| --- | --- |
| Installed | Yes, and read back. TrainingRooms was the only plugin replaced; the other 20 are byte-identical. A rollback copy of install 4 was also saved on the card |
| Eden, on this exact build (2026-11-25) | One short smoke boot, with no crash. Nine of its ten checks passed |
| Seen in that boot | Guard the Edge and Juggler opened their set-up on the Training stage. Guard the Edge was also checked from the island and on Battlefield. Eggs-ercise had one off-stage zone in the Training stage's gap. The recording picker's Off stage switch moved Player 1 beside the ledge. A take played on Player 1. The clip editor opened and previewed |
| A short check, not a play test | Neither drill was played through, no placement was saved or loaded, and most of the other new items were not touched. [Install evidence](EVIDENCE.md#evidence-rc15-installs--the-eden-smoke-boot-of-the-finished-build) |
| The waiting room | Not run in that boot, because the test's own route failed. A second boot the same evening reached a Local Wireless waiting room with one player: TrainingRooms' menu opened there, and Guard the Edge opened its set-up. No second player joined, and nothing was played. [Install evidence](EVIDENCE.md#evidence-rc15-installs--the-waiting-room-boot) |
| The operator reported, 2026-11-25 | In the clip editor, the refusal that In is mid-action kept showing after a snap to the move start, so the clip could not be previewed or saved. A host test reproduces it: in a match capture, plain Y can leave In on a mid-action frame. The way round and the fix, which is in install 6 (`dbded2d1`) and not in install 5: [Known issues](CURRENT-RELEASE.md#known-issues) |
| The operator reported, 2026-11-26 | A fighter placed off stage on a save stays on the ground once it has landed; and Guard the Edge's start varies between rounds (the CPU is let go at the same point, and the game's AI then moves differently). Neither is fixed in install 6: [Known issues](CURRENT-RELEASE.md#known-issues) |

<a id="release-notes-rc15--install-4"></a>

### Install 4

| Install 4 | Details |
| --- | --- |
| Installed | 2026-11-25 (07:26) |
| Build | `a945b0c1` |

The clips below were recorded in Eden on the installed build, not on the console. Where a clip is a close-up, the effect sits on a
small panel that cannot be read at full-screen size. Ten takes showed the test tool's yellow banner. They are not
embedded; each is described where it stood.

<a id="release-notes-rc15--drills-and-combos"></a>

#### Drills and combos

**Drill steps are listed by move**

The selected drill lists S1 jab, S2 side tilt and S3 up b.

**Move names Off and On**

Move names changes the labels from jab, side tilt and up b to Jab 1, Forward tilt and Final Cutter.

**Match angle and deflection, Off**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/4857608ce263f1c0ce535dea1b78cfe447b81cc1" autoplay muted loop playsinline controls></video><br>
*Close-up of the drill panel. A warp recorded at 135° is cast at 147° with the option Off: the attempt is green. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-action-off-147-zoom.mp4)

**Match angle and deflection, On, another angle**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/f8fe3ec94aa9ed3aa18a6313f3a1c581f3b5a261" autoplay muted loop playsinline controls></video><br>
*Close-up of the drill panel. The same cast at 147° with the option On: the attempt is red. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-action-on-147-zoom.mp4)

**Match angle and deflection, On, the recorded angle**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/4b1d65d8fab1b83e5d0199467a063377dc1463de" autoplay muted loop playsinline controls></video><br>
*Close-up of the drill panel. A cast at the recorded 135° with the option On: the attempt is green. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-action-on-135-zoom.mp4)

**A steered special, steered**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/3601efee83d20e7401cbe74b217770e30cd3fc1d" autoplay muted loop playsinline controls></video><br>
*Close-up of the drill panel. A drill recorded with an up special steered up-left: the steered cast is green. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-steer-steered-zoom.mp4)

**A steered special, not steered**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/6e2be13321f0d19ec828e933ef905fdf6b4a2a96" autoplay muted loop playsinline controls></video><br>
*Close-up of the drill panel. The same drill with the up special cast straight: the attempt is red. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-steer-straight-zoom.mp4)

**A get-up attack, recorded**

Place fighters starts Kirby lying down; Record captures his get-up attack.

**A get-up attack, played back**

Playing the take puts Kirby down again and repeats the get-up attack.

**L + R + A by itself**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/ff5c2315ca6efcb51bbb8fea5d3205962e7c7295" autoplay muted loop playsinline controls></video><br>
*Close-up of the drill panel. After one green attempt, L + R + A alone, short and long, adds no square: the streak stays at 1. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-reset-chord-alone-zoom.mp4)

**L + R + A after a failed attempt**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/ffcc3cfe7871855d69a43591dbbcfef92944d0ef" autoplay muted loop playsinline controls></video><br>
*Close-up of the drill panel. A failed attempt adds a red square, and the reset after it leaves that square in place. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-reset-after-fail-zoom.mp4)

**A ledge attack:** not filmed. The trip attack is filmed like the get-up attack and kept with the test records.

| Change | What it does | More |
| --- | --- | --- |
| One rule for every fighter | A step is one action: the input that starts it and what the game does with it. Its release, how long the button was held, a special's directions and its travel belong to that action. There is no per-fighter list any more. | [How a combo is graded](guide/recording.md#how-a-combo-is-graded) |
| Special directions | A special records a direction when the fighter starts to move a way that is not the special's own (up to two per special). They stay in the special's step instead of becoming a step of their own. |  |
| Held buttons | A release is graded on how long the button was held, and only when the game reacted to it. A quick tap is not graded on its release. |  |
| Angled tilts and smashes | Use the angle the game chose, not the stick position. |  |
| Steered specials | A drill recorded while steering a special must be steered the same way. [Match angle and deflection](menu/recording.md#match-angle-and-deflection) (Off by default) also compares the stick's angle and deflection. |  |
| Ledge, get-up and trip attacks | Are moves of their own. |  |
| Steps show their moves | Instead of Step N, in the list, the bar and the editor. |  |
| Move names | [Move names](menu/recording.md#move-names) (Off by default) shows common move names, and the game's own names for specials and Final Smashes. |  |
| L + R + A | The reset chord by itself leaves no drill result and starts no load, however long it is held. |  |
| Result lines are drawn whole | A reason line is no longer cut off at 24 characters. |  |

<a id="release-notes-rc15--recording"></a>

#### Recording

**The clip editor over the game**

Crop opens a strip below the fighters; Y snaps the End handle to the end of the jab.

**Preview on Player 1, then on the CPU**

X previews the cut on Player 1. After a ZR tap, X previews it on the CPU.

**Recordings**

Opening a Quick Play capture shows Inputs from, Plays on, Play back and the three Make rows.

**Playing a capture back**

Play back runs King K. Rool on the CPU, with the capture's timeline below.

**Trim: Off, Start, End and Both**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/e9f977ffad992d303fca7844745d5bf6847d48d8" autoplay muted loop playsinline controls></video><br>
*The same inputs were recorded four times, once per Trim setting. On playback, Start and Both skip the idle lead-in; Off and End keep it. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-trim-modes.mp4)

**The playback indicator**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/e8416edabcabf441325f0d62e064fb3f6bf22e20" autoplay muted loop playsinline controls></video><br>
*Close-up of the top left of the screen: the PLAY CPU mark shows while the CPU plays a take and goes when it ends. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-playback-indicator-zoom.mp4)

| Change | What it does | More |
| --- | --- | --- |
| The clip editor is a strip over the game | Crop and preview while both fighters stay visible. The timeline marks where each move starts and ends, Y snaps a handle to the nearest one, and a ZR tap swaps the preview between the CPU and Player 1. | [Replay window](menu/recording.md#replay-window) |
| Recordings | RECORDINGS is the first row of MACROS on the Recording page. It lists the latest and previous Vault captures and your saved captures; from one you can play it back or make a macro, a combo or a drill. | [Recordings](menu/recording.md#recordings) |
| Trim | Record drops idle frames at the Start, the End, Both or neither (the older On becomes End). Each macro and drill also has its own Trim for playback, which changes nothing in the file. | [Trim trailing neutral](menu/recording.md#trim-trailing-neutral) |
| Playback indicator | [Show playback indicator](menu/recording.md#show-playback-indicator) shows PLAY P1 or PLAY CPU while a recording plays. |  |
| The pop-up | ZR switches between Player and CPU; ZL, L and R switch its tabs. | [The Recording pop-up](menu/recording.md#the-recording-pop-up) |

<a id="release-notes-rc15--the-input-display"></a>

#### The input display

**Scrolling the input display back**

R3 and D-pad Left scroll back to -0.2 s, then -5.0 s, before the strip returns to live.

| Change | What it does | More |
| --- | --- | --- |
| Scroll back | Hold R3 and press D-pad Left or Right to look through the last 600 displayed columns. That's about ten seconds while the strip advances continuously; a quiet stretch can stay on screen much longer. Let go of R3 to return to the present. | [Frame strip](guide/frame-strip.md) |

<a id="release-notes-rc15--the-menu"></a>

#### The menu

**Damage range sliders**

Taps and a hold move Min from 0 through 0.5 and 3 to 8%, and Max from 150 through 148.5 to 144%.

| Change | What it does | More |
| --- | --- | --- |
| Sliders | Up or Down chooses Min or Max. Press A to edit it, then tap Up or Down to change the value, or hold to move faster. A or B finishes the edit. Damage ranges move in 0.5% steps. | [Changing a row](guide/using-the-menu.md#changing-a-row) |

<a id="release-notes-rc15--waiting-rooms"></a>

#### Waiting rooms

*No clip: install 4's waiting-room changes are not filmed.*

| Change | What it does | More |
| --- | --- | --- |
| Quick Play | Picking a different fighter at the character select starts the waiting-room practice fresh. |  |

<a id="release-notes-rc15--tested-3"></a>

#### Tested

| Check | Install 4 |
| --- | --- |
| Installed | Yes, and read back. |
| Tested in Eden (2026-11-25, Training) | The drill grading rows for Pikachu, Fox, Palutena, Mario and Kirby; Trim; the playback indicator for Player 1; the pop-up's ZR switch; scroll-back; the clip editor; a capture played from RECORDINGS; Move names in the drill list. These ran on an earlier head, `a9131331`; two fixes separate it from the installed build. The reset chord, with Mario's and Kirby's drills as controls, ran on the installed build. |
| Not covered by the final Eden session | The waiting-room editor, the Quick Play practice pick, new pick and rematch, the airborne steered special, the deflection half of Match angle and deflection, ledge attacks, the Trim setting's migration and the CPU's playback indicator. The Quick Play match was a font check for Online Deluxe. |
| The operator reported (2026-11-25, on this build) | The move names Jab 1 and Jab 2 are fine; and playback took a second or more to start. Install 5 changes how Play starts; timed later in Eden, not on the console, it started a 3-second macro 9 frames after the press and a ten-minute one 27 frames after. They also asked for placement changes and a Delete for replay captures; both are in install 5. |

<a id="release-notes-rc15--install-3"></a>

### Install 3

| Install 3 | Details |
| --- | --- |
| Installed | 2026-11-24 (03:07) |
| Build | `e7667f26` |

| Fix | What it does | More |
| --- | --- | --- |
| Guard the Edge in a waiting room | Turning Lobby Practice Off while a drill runs now gives the room's partner back. In install 2 the drill's borrowed fighter stayed. |  |
| The menu and a macro on Player 1 | While the menu is open the CPU stands still. A macro queued or playing on Player 1 now lifts that hold, so it plays as recorded. | [Opening and closing](guide/using-the-menu.md#opening-and-closing) |

<a id="release-notes-rc15--tested-4"></a>

#### Tested

| Check | Install 3 |
| --- | --- |
| Installed | Yes, and read back. It went on the card without an Eden run of that build. |
| Tested in Eden afterwards (2026-11-24) | In a waiting room, the Guard the Edge fix above and the practice partner's Stop passed. In Training, a run of the input changes passed for the frame strip's direction marks and for six fighters' drills, and found two defects that install 4 fixes: four fighters' special directions were saved as a step of their own, and nothing was graded after a Training reset. |
| The operator reported (2026-11-24, on this build), in Quick Play | In Quick Play, after a match the loser declined the rematch, picked a different fighter and came back to the room: the earlier fighter was put back as both the CPU and Player 1. Install 4 changes this (a different fighter picked at the character select starts the practice fresh). That fix has not been run in Eden. |
| The operator reported (2026-11-24, on this build), in the clip editor | The recording crop and preview showed a black screen, without the fighters and the timeline. In install 4 the clip editor is a strip over the game; its layout passed in Eden on `a9131331`. |

<a id="release-notes-rc15--install-2"></a>

### Install 2

| Install 2 | Details |
| --- | --- |
| Installed | 2026-11-24 (00:43) |
| Build | `71158421` |

<a id="release-notes-rc15--recording-1"></a>

#### Recording

| Change | What it does | More |
| --- | --- | --- |
| Chain and sequence pickers | B always backs out and saves nothing; X is done. | [Combos & Sequences](menu/recording.md#combos-sequences) |
| Drill preview | On a drill's row, X plays the take you recorded it from. A drill made another way reads "Re-record to preview". | [Input drills](guide/recording.md#input-drills) |
| Playback Slot on Mash | A Playback Slot starts once the fighter can act. | [The Playback Slot actions](guide/cpu-behaviour.md#the-playback-slot-actions) |
| Special directions in drills and combos | Each part of a directional special became a step. Install 4 replaces this with one rule for every fighter. |  |
| A start under a platform | A fighter saved in the air under a platform is placed there. |  |

<a id="release-notes-rc15--practice-games"></a>

#### Practice games

| Change | What it does | More |
| --- | --- | --- |
| Guard the Edge and Juggler setup | The CPU waits at its picked start while you set up, and B puts it back. The Player 1 crosshair no longer leaves a floor, and the start point stops a little inside each end of the floor. | [Guard the Edge and Juggler](menu/fighters-and-stages.md#guard-the-edge-and-juggler) |
| Waiting rooms | After a drill the room's partner gets its Training Partner Behavior back. |  |

<a id="release-notes-rc15--the-menu-and-the-display"></a>

#### The menu and the display

| Change | What it does | More |
| --- | --- | --- |
| The open menu holds the CPU | The CPU stands still while the menu is open, and Stop takes effect behind the menu. | [Opening and closing](guide/using-the-menu.md#opening-and-closing) |
| The frame strip | Marks the direction on a move. |  |
| A fighter pop-up | No longer leaves its box on screen after it closes. |  |

<a id="release-notes-rc15--tested-5"></a>

#### Tested

| Check | Install 2 |
| --- | --- |
| Installed | Yes, and read back. |
| Tested in Eden (2026-11-23 and 24) | The combined verdict was partial. The rows that ran passed for list wrapping, the recording indicators, Record and Play on the CPU, the Playback Slot timing, the frame strip's marks and the Guard the Edge start points. One defect was confirmed and is fixed in install 3 (Lobby Practice Off during a drill). The runs were stopped for the install, so the remaining rows did not run. |
| The operator reported | Nothing is on record for this build. It was on the card for about two and a half hours. |

<a id="release-notes-rc15--install-1"></a>

### Install 1

| Install 1 | Details |
| --- | --- |
| Installed | 2026-11-23 (20:04) |
| Build | `152629cd` |
| Holds | Fixes for defects the operator found on the console that day, in the build before it |

| Fix | What it does | More |
| --- | --- | --- |
| Lists wrap | Up and Down wrap at the ends of a list. On the Recording page each list wraps by itself; Left and Right switch lists. | [Moving around](guide/using-the-menu.md#moving-around) |
| You can see that you are recording | A REC mark shows while a take is open, tinted for the side being recorded. |  |
| Recording the CPU | The recorded left and right no longer flip with the way the fighter faces. |  |
| 1 Frame names its button | The game-speed label shows which button steps a frame. | [Game speed](guide/game-speed.md) |
| Crop's preview plays through Recording's playback | It hides the page, plays to the end and works for a Player 1 macro. | [Crop](guide/recording.md#crop) |
| Guard the Edge | It waits for a KO-ed Player 1 to return before it starts, and a KO counts once Player 1 is back. |  |

<a id="release-notes-rc15--tested-6"></a>

#### Tested

| Check | Install 1 |
| --- | --- |
| Installed | Yes, and read back. |
| Tested in Eden (2026-11-23) | In Training, 16 of 27 rows passed, 5 passed in part and 2 failed (the Guard the Edge start point, reworked in install 2); 1 failed in its setup, 2 did not run and 1 was left to a hand check. In the waiting room, Record and Play on the CPU and Crop's save passed; the preview's timing rows did not run there. The review cleared it as a limited release on that basis. |
| The operator reported | Nothing is on record for this build. The defects it fixes were reported from the console on 2026-11-23, on the build before it. |

<a id="release-notes-rc15--before-install-1"></a>

### Before install 1

| Theme | What went in | More |
| --- | --- | --- |
| Practice games | Eggs-ercise, Break the Targets, Tech practice, Mash out, Parry practice, Guard the Edge and Juggler, each with its own screen and a Start row. | [Practice games](menu/fighters-and-stages.md#practice-games) |
| Waiting rooms | Lobby Practice is On from the first start. | [Lobby Practice is On by default](guide/waiting-rooms.md#lobby-practice-is-on-by-default) |
| Recording | The pop-up's Player or CPU side stays until you change it, and Place fighters before Record sets both fighters up first. | [Place fighters before Record](menu/recording.md#place-fighters-before-record) |
| The menu | A held Up or Down repeats and speeds up, and L and R turn the page from inside a game's screen, a fighter grid or an editor. | [Moving around](guide/using-the-menu.md#moving-around) |

The first rc.15 build (2026-11-15) added the clip editor, input drills, article hurtboxes and the colour legend
([Current release](CURRENT-RELEASE.md)). Between that build and install 1, more went into rc.15 without notes of its
own. By theme, with the page that describes each:


The build on the card just before install 1 (`17060a5e`, installed 2026-11-23 at 11:40) was played by the operator
that day. What they reported, and what came of it:

| The operator reported | Since then |
| --- | --- |
| Lists did not wrap, nothing on screen showed that a recording was running, and a CPU recording's left and right shuddered. | Install 1 changes all three |
| Crop's preview kept the menu up and stopped early, cropping a Player 1 macro did not work, and the 1 Frame label left out its button. | Install 1 changes these too |
| Starting a new chain and going back before saving opened the keyboard, froze the menu for a moment and left both fighters frozen after the menu closed. | Install 2 changes the chain picker: B backs out and saves nothing |
| In Guard the Edge, picking the CPU in the fighter picker and backing out left a black box over Hitbox Visualization. | Install 2 puts back the part of the menu the pop-up had borrowed |
| The Guard the Edge case GJ1-2 worked on the build they were playing. | That build counted the KO immediately; waiting for Player 1 to return was new in install 1 |

Each report with its time and the build it was made on: [Install evidence](EVIDENCE.md#evidence-rc15-installs--the-operator-s-reports-from-the-console).

See [Earlier releases](CURRENT-RELEASE.md#earlier-releases) for the notes and published release pages.

<a id="release-notes-rc9"></a>

<a id="release-notes-rc9--rc-9"></a>

## rc.9

| Fix | Recorded result |
| --- | --- |
| Deluxe window shows a page with rows | TrainingRooms D-pad chords are suppressed |

- The rc.9 menu footer read `0.2.0-rc.6 · rc9` followed by the build's commit. rc.9 fixes the places where a TrainingRooms shortcut and another plugin's shortcut on the same buttons both fired, and it ships beside new builds of three of those plugins.
- The fix IDs (C3, C4, …) are the numbers of the conflict survey the fixes came from.
- What rc.8 changed is [RELEASE-NOTES-rc8.md](#release-notes-rc8).

<a id="release-notes-rc9--fixed"></a>

### Fixed

| Fix | Recorded result |
| --- | --- |
| Deluxe window shows a page with rows | TrainingRooms D-pad chords are suppressed |

<a id="release-notes-rc9--the-d-pad-chords-and-online-deluxe-s-window-c4"></a>

#### The D-pad chords and Online Deluxe's window (C4)

- With Online Deluxe 0.10.5, TrainingRooms' chords that use the D-pad don't fire while Deluxe's window shows a page
  with rows: Settings, Window Options, Plugins, or a plugin's own page such as CPU Pilot.
  - Those chords are save and load state, Record, Play on CPU, Play on Player 1, the Recording pop-up, the CPU and
    player cycles, the input display and the Game speed hold.
  - On those pages Deluxe uses D-pad Up/Down and ZR + D-pad Left/Right itself.
- **Before**, one press did both.
  - On Deluxe's CPU Pilot page, ZR + D-pad Right handed Player 1 to the CPU while TrainingRooms opened the Recording
    pop-up.
  - On Settings, ZR + Left/Right changed Deluxe's input buffer or rendering profile while Play on Player 1 started.
- **Not affected:**
  - Deluxe's Performance view has no rows, so the chords work while it's on screen.
  - TrainingRooms' menu shortcuts (Minus, and B + D-pad Up) always work.
- To use the D-pad chords again, hide Deluxe's window: L + R + D-pad Down until it closes.
- With an older Online Deluxe, or none, nothing changes.

<a id="release-notes-rc9--gamecube-controller-both-triggers-are-online-deluxe-s-l-r-c3"></a>

#### GameCube controller: both triggers are Online Deluxe's L + R (C3)

- On a GameCube controller, Online Deluxe's L + R chords are the two triggers, and TrainingRooms reads the triggers as
  ZL and ZR. So:
  - opening Deluxe's window (both triggers + D-pad Down) also saved a state and started a recording;
  - both triggers + D-pad Up also loaded a state and played the recording on the CPU;
  - both triggers + Left/Right also cycled the fighters.
- **Now:** while both triggers are held, no ZL or ZR chord fires, nor the Input display and Game speed chords, and the
  speed hold takes no step.
- **Unchanged:** one trigger works as before (left trigger + D-pad Down still saves a state), and Pro Controllers and
  Joy-Con are unchanged.
- This is checked by the host tests. It hasn't been tried on a GameCube controller.

<a id="release-notes-rc9--play-on-player-1-and-quick-reset-are-refused-while-an-ai-drives-player-1-c14"></a>

#### Play on Player 1 and Quick Reset are refused while an AI drives Player 1 (C14)

- As with Record in rc.8, Play on Player 1 and Quick Reset are refused while an AI drives Player 1, for example the
  standalone CPU pilot.
- The notices read "P1 is controlled by an AI: Play on Player 1 is off" and "P1 is controlled by an AI: Quick Reset is
  off".
- While Quick Reset is refused, Player 1's input is left to the pilot.
- Before, both wrote Player 1's input at the same time as the pilot.

<a id="release-notes-rc9--online-deluxe-s-l-r-d-pad-left-right-c10"></a>

#### Online Deluxe's L + R + D-pad Left / Right (C10)

- Deluxe's L + R + D-pad Left/Right fires no TrainingRooms chord on any controller and doesn't step the Game speed
  hold. It picks which online opponent Deluxe's window reports.
- It was checked, and nothing changed for it besides C3.

<a id="release-notes-rc9--good-to-know"></a>

### Good to know

| Quickplay action | Recorded result |
| --- | --- |
| Leave Battle | Close the TrainingRooms menu first |

<a id="release-notes-rc9--leaving-a-quickplay-room-with-the-trainingrooms-menu-open-c13"></a>

#### Leaving a Quickplay room with the TrainingRooms menu open (C13)

- The QuickPlay cancel fix plugin keeps the game's Pause › Leave Battle available for a few seconds after you join a
  Quickplay room.
- While the TrainingRooms menu is open, Plus doesn't reach the game. Close the menu first, then use Pause › Leave
  Battle.

<a id="release-notes-rc9--the-build-in-the-menu-footer"></a>

#### The build in the menu footer

- The rc.9 build tag at the foot of each menu page named the candidate, for example `0.2.0-rc.6 · rc9 01950601`.

<a id="release-notes-rc9--the-plugins-that-ship-beside-it"></a>

### The plugins that ship beside it

| Companion plugin | Recorded result |
| --- | --- |
| Online Deluxe 0.10.5 | Reports its window and page state |

<a id="release-notes-rc9--online-deluxe-0-10-5"></a>

#### Online Deluxe 0.10.5

- **Its window state, for other plugins (C4):** Deluxe now tells other plugins whether its window is drawn, whether
  the page has rows, and whether the Debug view is open. TrainingRooms rc.9 and the CPU pilot 0.2.4 use it.
- **Row selection (C5):** D-pad Up/Down no longer move Deluxe's row selection while ZL, ZR, R or L is held.
  - Those presses are TrainingRooms' chords, or a trigger on a GameCube controller.
  - A moved selection was the row that the next ZR + Left/Right then changed.
- **The Debug view is off by default (C6, C7).** Set `debug_view=on` in Deluxe's `config.ini` to get it.
  - **Off:** L + R + Plus and L + R + Minus do nothing in Deluxe. L + R + Minus no longer also opens Deluxe's Debug
    view beside TrainingRooms' menu, and L + R + Plus only pauses.
  - **On:** a Debug chord pressed with A held is ignored, so the replay viewer's quit (L + R + A + Plus) no longer
    opens it.
- **The waiting-room practice (C11):** it never lets ZL + D-pad change Deluxe's native character-select panel. This was
  checked in Deluxe's code and is now pinned by a test; nothing changes in play.

<a id="release-notes-rc9--cpu-pilot-0-2-4"></a>

#### CPU pilot 0.2.4

- While Online Deluxe's window is on screen, L + R + X no longer steps the CPU level in battle, because the press
  belongs to the window (C8).
- On the CPU Pilot page's P1 row, Deluxe's own request is the one step, so a press steps the level once instead of
  twice.
- With the window hidden, L + R + X steps as before.

<a id="release-notes-rc9--imgui_smash-1-0-2"></a>

#### imgui_smash 1.0.2

- Holding one stick click and pressing the other (L3 + R3) no longer turns on ImGui's controller navigation (C15).
  - With it on, the D-pad and face buttons also drove Online Deluxe's window while the game still got every press.
  - TrainingRooms' Kirby hat shortcut (B + a stick click) could turn it on by accident.
- To bring the toggle back, set `gamepad_nav_toggle = on` in `sd:/ultimate/imgui_smash/config.ini`.

<a id="release-notes-rc9--for-developers"></a>

### For developers

| Host suite | Recorded result |
| --- | --- |
| Test count | 41,918 before the cut, 865 after the table-driven replacements |

- **The host tests:** the suite went from 41,918 tests to 865, under the 1,000 limit.
  - The cut first took it to 942. Every cut and the reason for it is in [TEST-CUT-LEDGER-rc9.md](TEST-PROVENANCE.md#test-cut-ledger-rc9).
  - The cut tests then came back as table-driven tests.
- **Coverage:** line coverage is 91.5% with those 865 tests (90.8% before the cut, 34.2% just after it). How it was
  measured, and what keeps it below 95%, is in [COVERAGE-rc9.md](TEST-PROVENANCE.md#coverage-rc9).
- **Where the rules live:**
  - The chord rules are `model::chords::passes_row` (C3, C4) and `model::game_speed::Hold` (the speed hold).
  - Deluxe's export is read by `model::deluxe_overlay` and `runtime::deluxe_overlay`: looked up once, then again after
    a miss at most once a second. A missing export means no window.
- **The C14 refusals** use the same AI check as Record's: `recording::p1_driven_by_ai`, which also counts the game's
  own CPU flag on Player 1.
- **The footer's build name** comes from `build.rs` (`BUILD_NAME`). `build.rs` now reads `CARGO_MANIFEST_DIR` at run
  time, so a target directory shared between checkouts tags each build with its own commit.

<a id="release-notes-rc8"></a>

<a id="release-notes-rc8--rc-8"></a>

## rc.8

| Control | Recorded result |
| --- | --- |
| R + D-pad Down | Toggles the input display |

These notes cover rc.8 candidate `3f9d61dd` and CPU Pilot 0.2.3. The menu footer identified that build as
`0.2.0-rc.6 · rc8` followed by its commit. How to use recording is
[RECORDING.md](guide/recording.md), the frame strip is [FRAME-STRIP.md](guide/frame-strip.md), and the pictures are the list in
[MEDIA.md](MEDIA.md).

<a id="release-notes-rc8--new"></a>

### New

| Control | Recorded result |
| --- | --- |
| R + D-pad Down | Toggles the input display |

<a id="release-notes-rc8--input-display-chord-r-d-pad-down"></a>

#### Input display chord: R + D-pad Down

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/8bc5c548137a4ab83ddf533d683ccf7ebb2d7ae6" autoplay muted loop playsinline controls></video>

*R + D-pad Down hides the frame strip, then shows it again* [Open the clip](media/rc8-input-display-chord.mp4)

- Turns the input display (the frame strip) on and off without opening the menu, in Training and in the waiting-room
  practice.
- A notice says "Input display" and On or Off.
- It changes the same setting as Display & Controls › Input Display: the menu shows the same value afterwards, and the
  choice is saved.

<a id="release-notes-rc8--game-speed-chord-hold-r-d-pad-up"></a>

#### Game speed chord: hold R + D-pad Up

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/0f02cbc3879c9295bbfa358cbfa8a7d907eeaac3" autoplay muted loop playsinline controls></video>

*The Game speed pop-up steps to 1/2, then 1/4; the jabs slow down; the pause menu shows Speed 1/4* [Open the clip](media/rc8-game-speed-chord.mp4)

- Hold the chord and a small "Game speed" pop-up shows the current speed between two D-pad arrows.
- While you hold it, D-pad Left and Right step one speed at a time, slowest on the left. The steps stop at the slowest
  and fastest speeds instead of wrapping around.
- Let go and the pop-up closes. The speed stays.
- While you hold the chord, no other chord fires, and the chord's buttons and D-pad Left / Right don't reach the game.
  The rest of Player 1's controller still does, so you can keep moving.
- **In Training:** 1/4, 1/2, 2/3, x1 and x1.5, Training's own speeds.
  - The speed changes at once, with the menu closed: the chord sets the same Speed setting as the game's pause menu.
  - The pause menu's Other Settings › Speed and the HUD's stopwatch show the speed you chose, and a speed chosen in the
    pause menu is where the pop-up starts.
  - The stopwatch appears as soon as you step away from x1 and goes away when you step back to x1, as it does when you
    set the speed in the pause menu.

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/b8908852fc9c93984c0a88a9076d2a394f260fa8" autoplay muted loop playsinline controls></video>

*Hold R + D-pad Up, Right: the stopwatch reads x1.5; Left back to x1: it goes away* [Open the clip](media/rc8-speed-hud-fixed.mp4)
  - 1 Frame stays in the pause menu. While Training is on 1 Frame, the pop-up shows "1 Frame", and the first step goes
    from x1.
- **In the waiting-room practice:** 1/4, 1/2, 2/3, x1 and x1.5, local only.
  - Only your practice changes speed. The room's timers and the other player stay at normal speed, at 2/3 and x1.5 too.
  - Each practice session starts at x1.
- **Never in a match:** the chord never changes a Quickplay or online match, and a match found at any practice speed
  starts at normal speed.
- **With the standalone CPU pilot** driving Player 1, the chord works the same way. In the waiting-room practice the
  pilot still gets one input per game update at every speed, x1.5 included, so its inputs are never doubled.

<a id="release-notes-rc8--both-chords"></a>

#### Both chords

- They're two new rows of Display & Controls › Button chords: "Input display" and "Game speed".
- They can be changed or cleared like the other chords.
- R is Z on a GameCube controller.
- R + D-pad binds don't fire while L is held, so Online Deluxe's L + R + D-pad overlay chords no longer toggle the input
  display or open the speed pop-up.

<a id="release-notes-rc8--the-build-in-the-menu-footer"></a>

#### The build in the menu footer

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/bdce4e2f0ba4681509a24cf720e1f9d5948dd818" autoplay muted loop playsinline controls></video>

*Two menu pages: the key hints end with 0.2.0-rc.6 · rc8 3f9d61dd* [Open the clip](media/rc8-footer-tag.mp4)

- The rc.8 key hints at the foot of every menu page ended with the build tag: the version, then `rc8` and the commit the NRO
  was built from, for example `0.2.0-rc.6 · rc8 3f9d61dd`.
- A build from uncommitted changes adds `-dirty`.

<a id="release-notes-rc8--fixed"></a>

### Fixed

| Recording condition | Recorded result |
| --- | --- |
| AI drives Player 1 | Record is refused |

<a id="release-notes-rc8--record-is-refused-while-an-ai-drives-player-1"></a>

#### Record is refused while an AI drives Player 1

- While an AI drives Player 1, for example the standalone CPU pilot, Record is refused, from the Record chord as well as
  from the pop-up. The notice reads "P1 is controlled by an AI: recording is off", and nothing is recorded.
- A recording that is already running can always be stopped.
- This replaces rc.7's known issue, where the Record chord still started a recording while the pilot drove Player 1.

<a id="release-notes-rc8--a-rare-freeze-as-the-menu-closes"></a>

#### A rare freeze as the menu closes

- Closing the TrainingRooms menu could, rarely, freeze the game, in Training or in the waiting-room practice. It was
  seen once, on Eden, in the waiting-room practice right after *Add New…* in a Mii class list. When the menu closes,
  the game no longer waits for the menu's own thread, which is how it could get stuck.
- The freeze could not be made to happen again, so this fix comes from the code, not from a caught freeze. If the game
  still freezes as the menu closes, please report it.

<a id="release-notes-rc8--with-the-cpu-pilot-plugin-0-2-3"></a>

### With the CPU pilot plugin 0.2.3

| Companion plugin | Recorded result |
| --- | --- |
| CPU pilot 0.2.3 | Can drive Player 1 in Quickplay waiting-room practice |

The CPU pilot is a separate plugin, `libssbu_cpu_pilot.nro`. Its 0.2.3 ships beside this build:

- The pilot can drive Player 1 in Quickplay's waiting-room practice too, armed from its CPU Pilot page as in Training.
  The practice shows no `CPU Lv` tag.
- When a match is found, the pilot keeps Player 1 in the match, through its online mode as before.
- TrainingRooms' tools work beside it in the practice: Record is refused while it drives, and the Game speed chord
  works at every practice speed.
- Local Wireless waiting rooms, matches and other online modes keep 0.2.2's behaviour.

<a id="release-notes-rc6"></a>

<a id="release-notes-rc6--0-2-0-rc-6"></a>

## 0.2.0-rc.6

| Addition | Recorded result |
| --- | --- |
| Sequence drilling | Practise an ordered list of combos |

Notes for players. 0.2.0-rc.5 was withdrawn before it ran on players' consoles, so rc.6 followed
`0.2.0-rc.3.1`. Everything [RELEASE-NOTES-rc5.md](#release-notes-rc5) lists is in it, with the changes below. How to
use recording is [RECORDING.md](guide/recording.md), the Mii pages are [MII.md](TECHNICAL.md#mii), and the pictures are the list in
[MEDIA.md](MEDIA.md).

The switch and reset times below are the ranges reported for rc.6. The [feature ledger](FEATURE-LEDGER.md) names the candidate runs. Their original timing receipts were unavailable for the November 27 audit.

<a id="release-notes-rc6--new"></a>

### New

| Addition | Recorded result |
| --- | --- |
| Sequence drilling | Practise an ordered list of combos |

<a id="release-notes-rc6--sequence-drilling"></a>

#### Sequence drilling

rc.5 let you build and edit sequences, but not drill them. rc.6 drills them.

- A sequence is an ordered list of combos. Drill sequence, or A on a sequence in the pop-up, makes it the one drill.
  Drilling a sequence replaces a drilled combo, and drilling a combo replaces a drilled sequence.
- The bar under the frame strip is the combo helper's bar with a sequence header and a row for the sequence's combos:
  SEQUENCE, the name, combo k of n, the current combo, its links and the tolerance, and the streak and best. Done
  combos are green with a check, the current one is white, and the later ones are grey.
- Combo 2 follows combo 1 inside the same exchange. The link that joins them is shown on the strip and is not graded,
  but the move it lands must be combo 2's first move. The CPU may act for up to 20 frames between the combos. Each
  combo is graded the same way as a single drilled combo: the input events, the direction, the order, the tolerance
  and the move. TRUE and drift are shown, not graded.
- A finished sequence adds one to the streak, and best is the highest streak. A drop sends the sequence back to combo
  1 and the streak to 0, and best is kept. A different move after the joining link, the exchange ending before the
  next combo, and a save-state load or scene change in the middle of a sequence are drops too.
- The streak and best are saved in the sequence file. The menu shows the same numbers as the bar, and they stay after
  you leave Training.
- A numbered row inside a sequence drills that combo alone, with that combo's own streak. The sequence file stays.
- A deleted combo is skipped. An empty sequence drills nothing. Every combo of a sequence is drilled, however many it
  has.

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/42a8345eed3afe5d5842008cd1ed4fb229d18c7f" autoplay muted loop playsinline controls></video>

*Shown on rc.7 `722525a5` as shipped (diag NRO `cfec85c7`, log lines only), 2026-11-11; the rc.6 clip carried another plugin's input bar.* [Open the clip](media/rc7-sequence-drill.mp4)

The sequence bar is taller than the combo bar, so notices sit 56 layout units (about 37 pixels at 1280×720) lower
than in rc.5, under either bar.

<a id="release-notes-rc6--save-last-combo"></a>

#### Save last combo

- *+ Save last combo…* is the first row of COMBOS, on the Recording page and in the pop-up's Combos & Sequences tab.
- It saves the last combo the frame strip saw, with its input events, and makes it the drill.
- A opens the keyboard for its name.
- An empty name, or an online waiting room where the keyboard does not open, gives it the next Combo N name.
- A local-wireless waiting room opens the keyboard.
- With no link on the strip yet it says so and writes nothing. rc.5 had lost this row.

<a id="release-notes-rc6--the-recording-pop-up-as-drawn"></a>

#### The recording pop-up as drawn

The pop-up now matches the agreed drawing:

- the title Recording with the role tabs (L Player, CPU R) and the page tabs as outlined boxes;
- a count beside each section header, on a dark band with the kind's mark: the orange floppy for SAVE STATES, and a
  blue, purple or pink square for MACROS, COMBOS and SEQUENCES;
- a dim line under each file: a save's fighters, "Recorded on" and the fighter, or a chain's or a sequence's steps;
- a red number badge for a row's place in the chain or sequence under the cursor;
- CONNECTIONS as their own column on the right: the row under the cursor with its kind's pill, the joined files in
  boxes, then plain lines;
- a footer that says what A does there ("A bind", "A set active   X loop", "A drill");
- everything inside the black panel, including the loop counts and the highlight.

A drilled combo, like a bound save or an active macro, moves to the top of its list.

<a id="release-notes-rc6--fighters-load-hot"></a>

#### Fighters load hot

- **At Training entry.** Each role's start-up window of fighters is ready when Training starts. The Kirby hats of your
  cycles are loaded at entry too, as many as memory allows; rc.5 loaded the first hat only when you asked for it. On
  the console a hat change took 7–23 ms, the first one included.
- **The rest in the background.** After Training loads, the other groups of your cycles are built in the background
  while you play. The fighters on screen stay. Once the notice reads Ready or All loaded, switching into a group took
  about 50–70 ms in Eden, for Player 1 and for the CPU.
- **Mii groups.** A Mii group is built on the game's worker instead of the game thread. In Eden the game thread's longest pause during such a build was 34 ms (183 ms for the first Mii build after boot); the old build paused it at every Mii build, 141 ms on the console.
  - Leaving a Mii for the fighters it replaced takes about 45–115 ms; in rc.5 it was a full reload of 4–6 seconds, and the next visit to the Mii built it again.
  - Going back to the Mii is a warm switch too, about 50 ms.
  - Mii groups are built in the background too, so the first switch into a Mii is warm (47–72 ms in Eden, for Player 1
    and for the CPU), and leaving a Mii is warm to any built group (47–80 ms).
- Ice Climbers, Pokémon Trainer and Pyra/Mythra groups are built in the background too, also while a Mii is on screen
  and in a session that starts on one of them. Switching into them then took 64–82 ms in Eden. The background build
  now waits only while Kirby is on screen, and for a second Mii group while a Mii is on screen (see Known issues).
- Going back to an Ice Climbers (normal pair), Pokémon Trainer or Pyra/Mythra original while its group is on is a warm
  switch now; it used to reload. Going back to a Solo Popo or Mii original still reloads.
- The notice counts what is loading: "Loading 4/10", then "Ready 10/10" or "All loaded".
- Fighters past the ready budget still load when you switch to them, as before.

<a id="release-notes-rc6--a-cycle-change-takes-effect-at-once"></a>

#### A cycle change takes effect at once

- Changing Player 1's or the CPU's Character Cycles in Training takes effect when the menu closes.
- The new fighters are built in the background right away, the same way.
- The notice goes from Loading to Ready (in Eden, 14 s after a Player 1 change and 4 s after a CPU change).
- After a change, the first switches into the new fighters took 47–70 ms in Eden, also after a second change made while the first was still loading.

- **Fighters that left the cycle** stay loaded until you next open Training's pause menu, which frees them. Until then,
  a fighter loaded with them that the new cycle still has stays warm to switch to.
- **If the new fighters need their room,** the old ones are freed at once instead: only as many as needed, and first those the new cycle no longer has.
  - The game pauses for a moment while they go (0.04–0.27 s in Eden).
  - Their memory is handed back as soon as the game has unloaded them (about 0.04 s in Eden mid-play, unseen at a pause), so the new fighters start loading right after (about 1 s after the free for the Ice Climbers in Eden).
  - Fighters the new cycle still uses are freed only if there still isn't room after that.
  - You never need to open the pause menu for a change to load.
- Changing the cycle many times in a row doesn't build up memory: ten changes in Eden ended where they started.
- The CPU seats that No. of CPUs raises (below) follow the new CPU cycle too.
- While the new fighters build in the background, the game keeps running: its longest pause was 4 frames (about
  67 ms) in Eden, at the start of a Mii build, and 1–3 frames for the others.
- A switch you press while the new fighters build waits for that build to end and then goes through. It is never
  refused (in Eden it went through 0.12 s after the build finished).
- If the fighter on screen leaves the cycle, it stays until you switch away, and that switch goes to a fighter of the
  new cycle.

<a id="release-notes-rc6--no-of-cpus-the-seats-are-the-next-fighters"></a>

#### No. of CPUs: the seats are the next fighters

Raising No. of CPUs from 1 to 3 brings in the CPU cycle's next two fighters as CPUs 2 and 3, already on the first
raise. A Mii in those seats is the exact saved Mii, outfit and hat included. With three CPUs up, and after a raise
and a drop, the Mii classes stay in both cycles and load.

The two seats count in the Fighters notice. A seat that waits for memory shows as "… waiting for memory" there, and
it loads once memory is free (in Eden, "Loaded 1/5 · swap both to load 3 more (1 waiting for memory)", then
"Loaded 2/5 …").

<a id="release-notes-rc6--mii-editor-same-as-name-nothing-new-saved"></a>

#### Mii editor: "Same as ‹name›, nothing new saved"

A Save in the Mii editor that matches a Mii you already saved still saves nothing new and uses that Mii, as before.
It now says so: "Same as ‹saved name›, nothing new saved". A changed name alone counts as the same Mii.

<a id="release-notes-rc6--the-waiting-room-remembers-its-partners"></a>

#### The waiting room remembers its partners

In a local-wireless room or Quickplay's waiting area, the game's own menu sets Training partners (how many CPUs) and the
CPU action. When you change them, they're saved with the mod's settings. The next time you enter a waiting room, they're
set before the CPUs appear. With nothing saved, the game's defaults stand (Automatic, Attack).

<a id="release-notes-rc6--cpu-level"></a>

#### CPU Level

CPU › Level › **CPU Level** sets the CPUs' level: Off (the default) or Lv 1–9. It's saved with the other settings. Off
leaves the game's own level everywhere.

- **In Training,** Lv N replaces Training's own CPU level, the one the CPU (Lv. n) behavior plays at. Training's own
  level is the one set on the character select, and the pause menu keeps showing it.
  - If the two differ when Training starts, a small notice says so once, for example "CPU level: Lv 7 from
    TrainingRooms (Training menu: Lv 3)".
  - Changing CPU Level during the session takes effect at once, with no second notice. The next entry into Training
    checks again.
- **In the waiting room's practice** (local-wireless rooms and Quickplay's waiting area), the training partners play at
  Lv N, with no notice.
- **In a match,** nothing changes.

<a id="release-notes-rc6--fixed"></a>

### Fixed

| Fix | Recorded result |
| --- | --- |
| Save, Load and Auto-load | Follow the save bound in the pop-up |

- **Save, Load and Auto-load use the bound save.** In rc.5 they used the old save-state slot file, whatever the pop-up
  showed as bound. Save writes the bound save, and Load and Auto-load on death warp to it. With nothing bound,
  Auto-load does nothing.
- **CONNECTIONS.** A combo's connections list its links with their moves and input events. In rc.5 they were empty. A
  shared combo file with any allowed name now stays the drill.
- **Make combo** in rc.6 stored only the exchange its own playback made. With no hit it wrote nothing, where rc.5 saved
  the previous exchange (`covrec-nohit1`, `e39479df`). For the newer no-hit behavior, see [input drills](guide/recording.md#input-drills).
- **Mash interrupts playback** stops the CPU's macro, including a macro from the library. The CPU's mash does not stop
  Player 1's macro. In rc.5 the mash never stopped a library macro.
- **A setup restore** only moves the fighters. It writes no save file, and the next Load goes back to the bound save.
- **Edit placement** edits the active macro of the role in view, or a chain's first macro. Write over stores the
  offset in that file, Keep holds it until the game closes, and New saves a copy. Play warps to the start save with
  the offset. In rc.5 the row refused with "Load a macro and restore a Training state first".
- **Momentum** on Edit placement works. With "Keep recorded speed" Play starts the playing role with the speed it had
  when Record started; with "Start at rest" it starts from standstill. Play used to ignore the key.
- **A start save is where the recording started.** It used to hold the fighters where the recording stopped
  (`rr_host::service_at` at `9438039`), so a moving macro warped to its end.
- **Y in the Setup library** binds a setup without restoring it, and Load then goes to the setup until another save is
  bound or the bound save changes. Neither A nor Y writes a save file.
- **Edit combos, Edit chain and Delete** are written to the card. In rc.5 only a save state's Delete was.
- **The exact Mii.** Switch on a CPU macro recorded on an exact Mii loads that Mii: its look, specials, colour, voice,
  outfit and hat. rc.5 loaded the class's default Mii. A Mii from a replay has no ID and no name of its own; it is
  loaded as your saved Mii with the same look, or else under the name "New Mii".
- **Replay window.** The clip editor is drawn, a capture's Mii track no longer stays on "Reading recorded Mii...",
  its Fighter Mismatch question is shown, and Switch loads the exact Mii.
- **Notice plates.** Each dark plate is drawn under the notice texts, so a body's plate no longer covers the lower
  half of the header.
- **Crashes and hangs:** going back to the original fighter from a group of fighters could make the game call through a null pointer, and
    the game then hung on Quit Training;
  - a Mii in a No. of CPUs seat was loaded without its Mii and read null memory every frame;
  - after a scene change, a group switch could bring in the wrong fighter and leave the CPU cycle stuck on "Wait for
    the character swap to finish";
  - with heavy fighters in the cycles and little memory left, a switch could use more memory than there was, and the
    game crashed (seen in Eden). A switch now brings along only the next fighters that fit in memory. The others load
    when you switch to them;
  - the menu and the game could wait on each other for ever while a macro was being named, freezing the game. This
    was found by checking the code, not in a test run, and it is fixed.
- **No heap use in the game's hooks** when the CPU-as-Player 1 fallback runs, that is, without the standalone pilot
  plugin. rc.5 allocated there.
- **P1 CPU Control in an online match.** Without the standalone pilot plugin, P1 CPU Control now plays Player 1 in an
  online match. Before, it worked only in Training: in a Quickplay match Player 1 stood still, and the game dropped the
  console for inactivity. The [feature ledger, row 77](FEATURE-LEDGER.md) records both branch and candidate Quickplay runs.
- **Lobby Items Off in Quickplay's waiting area.** Off now also stops the waiting room's timed item drops. In rc.5 they
  kept coming with the row Off, one every 10–13 s. Items already out are still removed once when you turn it Off, and
  On brings items back. In local-wireless rooms the game spawns no items at all.

<a id="release-notes-rc6--known-issues"></a>

### Known issues

| Observed issue | Recorded result |
| --- | --- |
| Menu-close freeze | Four occurrences in Eden |

- **The game can freeze when the menu closes (rare).** In Eden it happened four times: once in Training after a diagnostic CPU-cycle reload with Cycle Preload Off, and three times in Quickplay's waiting area while paging through the menu (in the two longest loops, after 36 and 38 clean closes in a row).
  - Other loops of the same steps closed the menu cleanly.
  - While the game was frozen, none of this mod's code was running; the game's own code was busy.
  - The rc.6 investigation did not establish the cause.
  - See the [later menu-close fix](#release-notes-rc8--a-rare-freeze-as-the-menu-closes); the macro-naming freeze was separate.
- **Waiting for memory.** When memory is short, a group or a CPU seat can stay "waiting for memory". The Fighters
  notice says so, for example "Loaded 9/10 (1 waiting for memory)". That fighter still loads when you switch to it,
  the normal (slower) way.
- **Kirby.** While Player 1 or the CPU is Kirby, groups that aren't built yet wait; built groups stay ready. A session
  that starts on Kirby builds its other groups after the first switch away from Kirby. Leaving a Kirby that holds a
  copy ability reloads, as in rc.3.1.
- **Two Mii groups for one player.** The game keeps one Mii per player. With two Mii groups in one player's cycle, the
  second is built in the background only after that player leaves the first Mii.
- **A Training reset can pause for a moment.** Now and then a Training reset pauses the game for 0.14–0.23 s (seen in
  Eden). The pause is in the game's own reset, not in this mod's loading.
- **Record under a pilot:** rc.6 did not refuse it. [Waiting-room controls](guide/waiting-rooms.md#what-is-different-or-refused) describes the current behavior.

<a id="release-notes-rc6--corrections-to-the-rc-5-notes"></a>

### Corrections to the rc.5 notes

| Correction | Recorded result |
| --- | --- |
| Save last combo | The row was absent in rc.5 |

- "Save last combo still saves the combo you landed": rc.5 had no row for it. rc.6 has it, above.
- "A named macro can carry the exact Mii it was recorded on": rc.5 loaded the class default. rc.6 loads the exact Mii,
  above.
- rc.6 added sequence drilling.

<a id="release-notes-rc5"></a>

<a id="release-notes-rc5--0-2-0-rc-5"></a>

## 0.2.0-rc.5

| Display | Recorded result |
| --- | --- |
| Input history | Last second for Player 1 and the CPU |

0.2.0-rc.5 was withdrawn before players ran it. Its contents were included in 0.2.0-rc.6, with the changes and corrections in
[RELEASE-NOTES-rc6.md](#release-notes-rc6).

Notes for players. rc.5 followed `0.2.0-rc.3.1`. There is no separate `0.2.0-rc.4`. How to read the frame strip is [FRAME-STRIP.md](guide/frame-strip.md). How to use recording is [RECORDING.md](guide/recording.md). The pictures for those pages are the list in [MEDIA.md](MEDIA.md).

<a id="release-notes-rc5--frame-strip"></a>

### Frame strip

| Display | Recorded result |
| --- | --- |
| Input history | Last second for Player 1 and the CPU |

The input display is the frame strip: the last second of Player 1 and the CPU, a chip for each new button, and readouts for advantage, the link, and the timing of the press.

ADVANTAGE is the CPU's first actionable frame minus yours after the last hit. Plus means you act first. The line says on hit or on shield. Actionable includes cancel windows, so an aerial can read more plus than a frame-data listing that waits out the landing lag.

A follow-up more than 20 frames after the CPU can act is a new exchange, not a dropped link.

- Under Player 1's lane, a stick row shows the held direction on each frame.
- A C-stick attack, the attack the game marks as coming from the C-stick, takes its chip arrow from the C-stick when that stick is outside the dead zone.
- Any other press, including a C-stick that stays neutral, keeps the left-stick arrow.
- The stick row reads the physical left stick either way, so a C-stick attack with the left stick neutral leaves the row blank.

Holding one direction long enough can freeze the strip. A new binned direction, including a move out of neutral, starts it scrolling again.

<a id="release-notes-rc5--recording"></a>

### Recording

| Action | Recorded result |
| --- | --- |
| Load | Warps to a save without starting a recording |

- A save state only warps. Load does not start a recording. rc.5 used the old save-state slot for Save, Load and Auto-load on death, regardless of the binding shown in the pop-up. rc.6 made all three follow that binding.
- Record follows the pop-up's role. It captures Player 1 until that role is CPU. Stopping writes a named macro and that macro's start save.
- Each role has one active macro, a recording or a chain. Play on Player 1 and Play on CPU play that role's macro, or stop it if it is already playing. A macro plays back frame for frame, on either role.
- The chords' buttons do not reach the game while held, so Record and Play do not make Player 1 shield, and a recording does not contain the Record chord.
- The loop never warps back. Later passes continue from where the fighters are.
- A chain starts at its first macro's start save, plays its recordings in order, and cannot contain a chain.
- rc.5 omitted Save last combo. [rc.6 restored the row](#release-notes-rc6--save-last-combo). A drilled combo counts without having to be a true combo. A different move drops.
- Make combo plays the file under the cursor once on Player 1, from that file's own start save, and saves the links as a new combo named Combo N. In rc.5, a playback with no hit saved the previous exchange.
- Crop opens a handle screen and saves a new file. The original stays. Both rows are on the file's screen; what they do is in [RECORDING.md](guide/recording.md).
- A sequence is an ordered list of combos. rc.5 could build and edit sequences. Its helper drilled one combo. [rc.6 added sequence drilling](#release-notes-rc6--sequence-drilling).
- Play on a macro or a chain whose first macro was recorded on another fighter asks Fighter Mismatch first: Keep, Switch, or B to cancel. The frame strip is hidden while it is up.
- The pop-up and the Recording page are how you bind a save, set a macro active, and drill.

<a id="release-notes-rc5--chords"></a>

### Chords

| Chord | Default | Change |
| --- | --- | --- |
| Save | ZL + D-pad Down | Writes the old save-state slot. |
| Load | ZL + D-pad Up | Warps to the old save-state slot. Starts no recording. |
| Record | ZR + D-pad Down | Same chord. The pop-up's role says who is recorded. |
| Play on CPU | ZR + D-pad Up | Was Input Playback. Plays the CPU's active macro. |
| Play on Player 1 | ZR + D-pad Left | Plays Player 1's active macro. |
| Recording pop-up | ZR + D-pad Right | Opens the pop-up over the match. Press it again to close. |
| Cycle CPU | ZL + D-pad Left | Unchanged. |
| Cycle Player | ZL + D-pad Right | Unchanged. |

Open menu stays B + D-pad Up. Menu shortcut 1 stays Minus. Both Play chords may be held at once. An old settings file that does not have the two new chords still loads, and those chords appear at the defaults above.

The first time Training opens on a new install, the six reminders are still Welcome, Open Menu, Save State, Load State, Input Record, and Input Playback. The first Record or Play also posts, once: "Record, Play on Player 1, Play on CPU, and the recording pop-up replace Input Record and Input Playback."

<a id="release-notes-rc5--migration"></a>

### Migration

| Migration input | Recorded result |
| --- | --- |
| Empty slots and indexes | Skipped |

On the first build of the new library:

- Empty recording slots and empty save indexes are skipped.
- A used recording slot becomes a macro named `Slot N (YYYY-MM-DD)`, with the date the migration runs. It has no start save, so Play runs it from where the fighters are. Recording slots were never kept between boots, so a card updated from an earlier build has none to migrate.
- A used save index becomes one save file of both fighters, with the same kind of name. The slot Save and Load were using becomes the bound save.
- The named library's selected file becomes Player 1's active macro.
- Nothing is turned into a chain. Sequences and chains start empty.
- Loop On becomes infinite. Loop Off, or no loop row, becomes 1. Player 1 and the CPU then keep their own counts.

<a id="release-notes-rc5--what-leaves"></a>

### What leaves

| Playback trigger | Recorded result |
| --- | --- |
| Macro with a start save | The role’s Play chord |

The playback machine no longer starts a recording because a save loaded. That job is the macro's start save, started by the role's Play chord.

The slot number, Randomize slots, Record Slot, Trigger, the Duration dropdown, button slots, the save-state playback checklist, Source, and per-slot playback leave the Recording page. Their setting ids stay in the file, so an older settings file still opens. Per-slot playback chords are not added.

The file cap stays at 36,000 frames. The capture stops taking frames there.

The Saves page keeps the chord switch, mirroring, auto-load on death, damage on load, buffs, and the setup library. Setups stay a separate library from save states. Respawn on platform stays under Fighters & Stages › Stages.

Pins are gone. The five most recently used of each kind are listed first. Y does nothing on the pop-up or the Recording page.

<a id="release-notes-rc5--since-0-2-0-rc-3-1"></a>

### Since 0.2.0-rc.3.1

| Addition | Recorded result |
| --- | --- |
| Combo helper | Shown under the frame strip |

Recording and the stick row are above. The same build also adds:

- The combo helper under the frame strip, and combo files you can copy or share.
- Any fighter can practise one.
- Grading uses the recorded presses, releases, C-stick, and stick flicks.
- Drift is shown and is not graded.
- A link counts without being a true combo.
- Switch or Keep can change Player 1 to the combo's recorded fighter, and Player Swapping Off does not block that Switch.
- Combos open in lobby practice.
- In an online waiting room a saved combo is named Combo N.
- A saved combo's footer shows the recorded fighter.
- Notices sit under the frame strip.
- Add New on a Mii class opens Smash's own Mii editor. Saving adds one Mii. Leaving without saving adds none.
- Lobby Items, on Fighters & Stages › Stages. In rc.5, Off removed existing items. Timed drops continued in the Quickplay waiting area until rc.6.
- The replay window, edit placement, and the setup library. A timer match's replay capture continues through sudden death instead of stopping at the end of regular time.
- The six intro reminders show once on a new install. Each notice has a dark plate. Respawn on platform is on the Stages page. The Player cycle numbers are the list that cycle loads, and the CPU cycle is one list.
- Ready Fighters defaults to 10. Ready Hats are counted apart from those fighters. Raising the count promotes a parked CPU body. A Training count closes when that scene closes. A greyed Mii class is not in the CPU cycle.
- Switching to the Mii from a named macro loaded the class default in rc.5. rc.6 loaded the recorded Mii, including its appearance and specials.
- Waiting-room hitboxes resolve when the waiting room is the first practice of the boot.
