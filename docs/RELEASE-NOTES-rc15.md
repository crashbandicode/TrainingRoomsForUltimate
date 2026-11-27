# rc.15: the builds of November 2026

Notes for players. rc.15 is one release name for a line of builds. Each build's menu footer reads
`0.2.0-rc.15 · rc15` and then the commit it was made from, so the commit tells the builds apart
([Which build you have](guide/getting-started.md#which-build-you-have)).

Six of these builds were installed on the project's test console one after another, and are called install 1 to 6.
The preview build is staged for the card; install 6 is still the newest installed build.
Times are UTC.

| Build | Installed | Footer ends with | In short |
| --- | --- | --- | --- |
| [Preview build](#preview-build) | Not installed; staged 2026-11-27 | `cc71537c` | Scrubbing shows each fighter's recorded move; fixes to install 6's scrubbing and to a save's placement |
| [Install 6](#install-6) | 2026-11-26 | `5df3d9dc` | The clip editor: Y on In makes a capture previewable, a free playhead, and the fighters shown while you scrub |
| [Install 5](#install-5) | 2026-11-25 | `b7a382e4` | Record with an AI on Player 1, named grab and throw keys, B stops a preview, Delete for replay captures, named move marks, playback that starts without reading the library, Guard the Edge and Juggler use the floor you stand on, the eggs' off-stage zone, a save's placement changed with the crosshair |
| [Install 4](#install-4) | 2026-11-25 | `a945b0c1` | Drills and combos read moves the same way for every fighter; the clip editor over the game; Recordings; Trim; input scroll-back |
| [Install 3](#install-3) | 2026-11-24 | `e7667f26` | Two fixes to install 2 |
| [Install 2](#install-2) | 2026-11-24 | `71158421` | Fixes to Recording, the practice games and the menu |
| [Install 1](#install-1) | 2026-11-23 | `152629cd` | Fixes for defects found on the console that day |

What "tested" means in these notes:

- **Installed:** the file on the card was read back and matched the build. That shows the build is there, not how it plays.
- **Tested in Eden:** a named run in the Eden emulator gave a verdict for that item, on the build named beside it.
- **The operator reported:** the project's operator played that build on the console and said what they saw. These
  are reports, not a list of passes, and only reports that are on record are written.

The runs, verdicts and file hashes behind every line are in [Install evidence](EVIDENCE-rc15-installs.md).

## Preview build

An in-between build for the operator to play while two defects are fixed. It is **staged for the card and not
installed**, and it is not the next release.

| Preview | Details |
| --- | --- |
| Built from | `cc71537c` (the footer ends with it) |
| Release file | `78db86f5…`, 5,419,008 bytes |
| Replaces | Install 6, when it is installed |

### Scrubbing shows the recorded moves

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

### Fixes to install 6

| Install 6 | This build |
| --- | --- |
| A long take kept only its first 1,200 frames of scrub positions | It keeps them all. A 1,610-frame take had 1,610 |
| The first X after a snap could be refused | X plays after a held frame |
| A fighter beside a wall was shown off its point | The editor refuses that point instead: "Can't stand there on this stage." |
| At a refused point the snap hint could hide that line | The refusal line shows |
| A save's Off stage fighter stayed grounded once it touched the ground | It floats again |

Guard the Edge's uneven start is not changed by this build.

### Known issues

| Issue | What happens | Way round | Status |
| --- | --- | --- | --- |
| Fighter switch times out | After ZL + L the editor waits, then reads "The fighter did not load; the editor keeps the current one". The open editor blocks the roster from applying the switch. After 600 worker passes it gives up without withdrawing the request. In Eden, Player 1 changed fighters only after the editor closed; the CPU side was not tested | None established | Not fixed in this preview |
| Fighters slide apart after B | Both fighters return to their places, but their push boxes can keep the width of the scrubbed pose for a few ticks. The boxes still overlap, so the game pushes the fighters apart. A quick B left each about 0.6 units off in four of six tries | None established | Not fixed in this preview |

Both are to be fixed before the next release.

### Tested

| Check | Preview build |
| --- | --- |
| Installed | No. The file is staged with its approval record |
| Host | Tests and all builds passed on `cc71537c`. Refusing a fighter pushed by a wall, and captures keeping their animation, are host-tested only |
| Eden, on the release file, first boot | No crash. Passed: a long take, snapped X, the refusal at a point inside the stage, a save's fighter floating again, the settings check, and the menu in a waiting room as host. Scrub poses showed for both fighters |
| Eden, on the release file, second boot | No crash. Snapped X, scrub poses and the settings check passed again. The fighter choice failed as above |
| Eden, on the diagnostic build | Three chosen frames were shown exactly, and X and B let the fighters go. The slide was found there |
| Not flown | A capture saved and read back in a game; a fighter pushed by a wall; a macro gaining its moves from one Play; the CPU side of the fighter choice; a second player in a waiting room |
| On the console | Nothing yet |

The runs behind these: [Install evidence](EVIDENCE-rc15-installs.md#preview-build-cc71537c).

## Install 6

| Install 6 | Details |
| --- | --- |
| Installed | 2026-11-26 (12:22), and read back |
| Build | `5df3d9dc`. Still an rc.15 build: the footer reads `0.2.0-rc.15 · rc15 5df3d9dc` |
| Holds | install 5, plus three changes to the clip editor and one fix to the third, all merged on 2026-11-26 |

All three changes are in the clip editor of the [Replay window](menu/recording.md#replay-window).

### Y on In makes a capture previewable

*No clip: it is held until the known issue with X after a snap is fixed.*

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

### A free playhead in the clip editor

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

### The fighters are shown while you scrub

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

### Tested

| Check | Install 6 |
| --- | --- |
| Installed | Yes, and read back. TrainingRooms was the only one of the 21 active plugins replaced. A rollback copy of install 5 was also saved on the card |
| Host | Every merge passed host tests and builds |
| Eden, on the build before the fix (`eb39d6bc`), not on this exact build | Play started no later than on install 5; a take was recorded and held at In with no preview; an old macro got its positions from one Play; Y on In made a capture previewable; and the playhead browsed a take. The same session found the defect that the fix addresses |
| Eden, on install 6's diagnostic build (same commit) | Passed so far: record and hold, release on X and on close, the old macro, a crop, delete, a take on the CPU, the playhead rows, the snap rows up to their preview, and the menu's stack check. Found: the three issues listed under [Known issues](CURRENT-RELEASE.md#known-issues) as found in testing. No smoke boot of the installed release file, and no waiting-room pair, has flown |
| On the console | Scrubbing moved the fighters but did not show the moves. The operator requested recorded poses and a choice of recorded or current fighter for each role; neither is in install 6. |

The runs and what they did not cover: [Install evidence](EVIDENCE-rc15-installs.md#install-6-5df3d9dc).

## Install 5

| Install 5 | Details |
| --- | --- |
| Installed | 2026-11-25 (23:06), and read back |
| Build | `b7a382e4`. Still an rc.15 build: the footer reads `0.2.0-rc.15 · rc15 b7a382e4` |
| Holds | the ten changes below, all merged on 2026-11-25 |

| Change | In short |
| --- | --- |
| [Record with an AI on Player 1](#record-works-while-an-ai-plays-player-1) | Record starts instead of being refused |
| [Named grabs, throws and edge attacks](#grabs-throws-and-edge-attacks-are-named-in-combo-files) | Five more moves are written by name in combo files |
| [B stops a preview](#b-stops-a-preview-and-the-pop-up-shows-its-chords) | And the Recording pop-up shows your chords |
| [Delete for replay captures](#delete-for-replay-captures) | A Delete row, and Y in the Replay window's capture list |
| [Named move marks](#the-clip-editor-s-marks-name-the-form-of-a-move) | Jab 1, Jab 2, Forward air on the editor's timeline |
| [Playback start](#playback-starts-without-reading-the-library) | Play no longer reads the whole library; a Playback delay option |
| [Guard the Edge and Juggler](#guard-the-edge-and-juggler-start-from-the-floor-you-stand-on) | They start from the floor you stand on |
| [Two Guard the Edge options](#two-guard-the-edge-options) | Under the stage; CPU midair jumps |
| [Eggs-ercise](#eggs-ercise-s-off-stage-zone-follows-the-stage) | The off-stage zone follows the stage |
| [Change placement](#a-save-s-placement-is-changed-with-the-crosshair) | Move where a save puts the fighters, with the crosshair |

### Record works while an AI plays Player 1

*No clip: Record under an AI is not filmed.*

An AI here is the CPU pilot plugin, or TrainingRooms' own Player 1 control.

| While an AI plays Player 1 | Install 5 | Before |
| --- | --- | --- |
| Record | Starts. The take holds Player 1's inputs as the game receives them, the AI's included | Refused |
| Play on Player 1 | Refused | Refused |
| L + R + A in a waiting room (Quick Reset) | Refused | Refused |

More: [Controls › When the CPU pilot plays Player 1](guide/controls.md#when-the-cpu-pilot-plays-player-1).

### Grabs, throws and edge attacks are named in combo files

| Combo files | Install 5 |
| --- | --- |
| Written by name | `grab`, `throw`, `ledge_attack`, `getup_attack` and `trip_attack`, in a link's `from` and `to` |
| **Not backward compatible** | Install 4 and earlier skip a file that has one of these five there |
| Not affected | Files without them. This build still reads older files |

More: [Combo files › The format](guide/combos.md#the-format).

### B stops a preview, and the pop-up shows its chords

*No clip: B stopping a preview is not filmed.*

| When | The key line reads | What happens |
| --- | --- | --- |
| While any preview plays | `B stop preview` | B or X stops it and drops the rest of its frames |
| After the last frame, for about a second | `B back` | The result stays on screen |

The Recording pop-up's first help line shows the Record, Play on Player 1 and Play on CPU chords as you have set
them.

More: [Recording › Replay window](menu/recording.md#replay-window) and
[The Recording pop-up](menu/recording.md#the-recording-pop-up).

### Delete for replay captures

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

### The clip editor's marks name the form of a move

| Move marks | Install 5 |
| --- | --- |
| A mark keeps the form of its move | The first, second or third jab, the rapid jab or its finisher, which aerial, grab or throw, and a side tilt or smash aimed up or down |
| With Move names On | A mark reads as the combo and drill lists do, for example Jab 1, Jab 2 or Forward air |
| A take saved before this | Has no forms: its jab reads Jab |
| Specials | A take, and each fighter of a replay capture, stores the fighter it was recorded on, so its specials keep that fighter's own names whoever Player 1 is |

More: [Recording › Replay window](menu/recording.md#replay-window) and [Move names](menu/recording.md#move-names).

### Playback starts without reading the library

| Play's start | Install 5 |
| --- | --- |
| What it reads | The index, the file each side plays and its start save. Not the whole recording library, and nothing while a macro plays |
| Time from pressing Play to the first input | In one Eden run on 2026-11-26: 9 frames for the 3-second macros; 10 for the newly recorded 30-second take and 11 for the old one; 27 for the single start of the old ten-minute macro. Not timed on console. [Timing comparison](EVIDENCE-rc15-installs.md#eden-on-the-build-before-the-fix-eb39d6bc) |
| Known limit | If Play on Player 1 starts in the middle of one of Player 1's input polls, that one poll can pass what you hold instead of nothing. It is one frame at most, at the start |

| Playback delay | A new option under Recording › Options |
| --- | --- |
| Values | None by default; 10 to 120 frames |
| When it waits | After the fighters are placed and before the first input, once per Play |
| Applies to | Play on Player 1, Play on CPU, a capture's Play back and Make combo's pass |
| Does not apply to | The editor's previews, or the CPU's Playback Slots |

More: [Playback delay](menu/recording.md#playback-delay).

### Guard the Edge and Juggler start from the floor you stand on

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
([Install evidence](EVIDENCE-rc15-installs.md#install-5-b7a382e4)).

More: [Fighters & Stages › Guard the Edge and Juggler](menu/fighters-and-stages.md#guard-the-edge-and-juggler).

### Two Guard the Edge options

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

### Eggs-ercise's off-stage zone follows the stage

*No clip: the zone on another stage is not filmed.*

| The off-stage zone | Install 5 |
| --- | --- |
| Where a zone lies | Beside a ledge of the stage's main floor that has open air next to it, so a stage can have two zones, one or none |
| Its outer edge | Stops where that open air ends |
| On the Training stage | The one zone is the gap between the long floor and the island |
| Where a target appears | Only in the air off stage: never inside another part of the stage, and never above a floor or a platform |
| Known limits | A platform that moves after the stage is read is not followed, and a piece of the stage narrower than 10 units can be missed |

More: [Fighters & Stages › Eggs-ercise](menu/fighters-and-stages.md#eggs-ercise).

### A save's placement is changed with the crosshair

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

### Tested

| Check | Install 5 |
| --- | --- |
| Installed | Yes, and read back. TrainingRooms was the only plugin replaced; the other 20 are byte-identical. A rollback copy of install 4 was also saved on the card |
| Eden, on this exact build (2026-11-25) | One short smoke boot, with no crash. Nine of its ten checks passed |
| Seen in that boot | Guard the Edge and Juggler opened their set-up on the Training stage. Guard the Edge was also checked from the island and on Battlefield. Eggs-ercise had one off-stage zone in the Training stage's gap. The recording picker's Off stage switch moved Player 1 beside the ledge. A take played on Player 1. The clip editor opened and previewed |
| A short check, not a play test | Neither drill was played through, no placement was saved or loaded, and most of the other new items were not touched. [Install evidence](EVIDENCE-rc15-installs.md#the-eden-smoke-boot-of-the-finished-build) |
| The waiting room | Not run in that boot, because the test's own route failed. A second boot the same evening reached a Local Wireless waiting room with one player: TrainingRooms' menu opened there, and Guard the Edge opened its set-up. No second player joined, and nothing was played. [Install evidence](EVIDENCE-rc15-installs.md#the-waiting-room-boot) |
| The operator reported, 2026-11-25 | In the clip editor, the refusal that In is mid-action kept showing after a snap to the move start, so the clip could not be previewed or saved. A host test reproduces it: in a match capture, plain Y can leave In on a mid-action frame. The way round and the fix, which is in install 6 (`dbded2d1`) and not in install 5: [Known issues](CURRENT-RELEASE.md#known-issues) |
| The operator reported, 2026-11-26 | A fighter placed off stage on a save stays on the ground once it has landed; and Guard the Edge's start varies between rounds (the CPU is let go at the same point, and the game's AI then moves differently). Neither is fixed in install 6: [Known issues](CURRENT-RELEASE.md#known-issues) |

## Install 4

| Install 4 | Details |
| --- | --- |
| Installed | 2026-11-25 (07:26) |
| Build | `a945b0c1` |

The clips below were recorded in Eden on the installed build, not on the console. Where a clip is a close-up, the effect sits on a
small panel that cannot be read at full-screen size. The yellow BRIDGE LIVE banner in some frames is the test
tool's, not TrainingRooms'.

### Drills and combos

**Drill steps are listed by move**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/cd0b8b07aa26c3e00a692faedd589d16126e5410" autoplay muted loop playsinline controls></video><br>
*DRILLS, with a three-step drill selected: its help line reads S1 jab, S2 side tilt, S3 up b. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-drill-step-names.mp4)

**Move names Off and On**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/edd9787940d24d0eb09d0a71f1ea68de4f58267b" autoplay muted loop playsinline controls></video><br>
*With Move names Off the drill reads jab, side tilt, up b. With it On it reads Jab 1, Forward tilt, Final Cutter. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-move-names-toggle.mp4)

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

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/cbc8cd75b8b22c99acd1e03fc910fb5efdf3ca5f" autoplay muted loop playsinline controls></video><br>
*Place fighters starts Kirby lying down, and his get-up attack is recorded. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-edge-getup-record.mp4)

**A get-up attack, played back**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/016231d178e57592cfe067807f46bbf627cc48f1" autoplay muted loop playsinline controls></video><br>
*Playing the take puts Kirby down again and repeats the get-up attack. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-edge-getup-play.mp4)

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

### Recording

**The clip editor over the game**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/99f4cf63a17f837c9077eecda13bb9a267650d8b" autoplay muted loop playsinline controls></video><br>
*Crop opens the editor as a strip under the fighters. The End handle moves in, and Y snaps it to the end of the jab. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-replay-editor-timeline.mp4)

**Preview on Player 1, then on the CPU**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/7029a8b5915102970c6af108e1d3909f5e614645" autoplay muted loop playsinline controls></video><br>
*X previews the kept part on Player 1. A ZR tap switches to the CPU and X previews it there. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-replay-editor-preview.mp4)

**Recordings**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/f5b8891b2eb0285ad5178ed34f668f765a5d28ce" autoplay muted loop playsinline controls></video><br>
*RECORDINGS lists a Quick Play capture. A opens its screen: Inputs from, Plays on, Play back and the three Make rows. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-vault-recordings-menu.mp4)

**Playing a capture back**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/a743aae1c8bfe6267175ce653776bd89eccd3da0" autoplay muted loop playsinline controls></video><br>
*Play back runs the captured King K. Rool on the CPU, with the timeline of its moves under the game. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-vault-playback.mp4)

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

### The input display

**Scrolling the input display back**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/16db06e0ff5b07c584ded3173e52cfade51d4dfa" autoplay muted loop playsinline controls></video><br>
*Close-up of the input display. With R3 held, D-pad Left scrolls back (the label reads -0.2 s, then -5.0 s), and the display then returns to now. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-scrollback-zoom.mp4)

| Change | What it does | More |
| --- | --- | --- |
| Scroll back | Hold R3 and press D-pad Left or Right to look through the last 600 displayed columns. That's about ten seconds while the strip advances continuously; a quiet stretch can stay on screen much longer. Let go of R3 to return to the present. | [Frame strip](guide/frame-strip.md) |

### The menu

**Damage range sliders**

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/04dbf0b9d7e33489a3bd051e0ffb1d49efaffc74" autoplay muted loop playsinline controls></video><br>
*CPU damage range: Min goes 0, 0.5, 3, 8% and Max 150, 148.5, 144%, by taps and a hold. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](media/i4-cpu-damage-range.mp4)

| Change | What it does | More |
| --- | --- | --- |
| Sliders | Up or Down chooses Min or Max. Press A to edit it, then tap Up or Down to change the value, or hold to move faster. A or B finishes the edit. Damage ranges move in 0.5% steps. | [Changing a row](guide/using-the-menu.md#changing-a-row) |

### Waiting rooms

*No clip: install 4's waiting-room changes are not filmed.*

| Change | What it does | More |
| --- | --- | --- |
| Quick Play | Picking a different fighter at the character select starts the waiting-room practice fresh. |  |

### Tested

| Check | Install 4 |
| --- | --- |
| Installed | Yes, and read back. |
| Tested in Eden (2026-11-25, Training) | The drill grading rows for Pikachu, Fox, Palutena, Mario and Kirby; Trim; the playback indicator for Player 1; the pop-up's ZR switch; scroll-back; the clip editor; a capture played from RECORDINGS; Move names in the drill list. These ran on an earlier head, `a9131331`; two fixes separate it from the installed build. The reset chord, with Mario's and Kirby's drills as controls, ran on the installed build. |
| Not covered by the final Eden session | The waiting-room editor, the Quick Play practice pick, new pick and rematch, the airborne steered special, the deflection half of Match angle and deflection, ledge attacks, the Trim setting's migration and the CPU's playback indicator. The Quick Play match was a font check for Online Deluxe. |
| The operator reported (2026-11-25, on this build) | The move names Jab 1 and Jab 2 are fine; and playback took a second or more to start. Install 5 changes how Play starts; timed later in Eden, not on the console, it started a 3-second macro 9 frames after the press and a ten-minute one 27 frames after. They also asked for placement changes and a Delete for replay captures; both are in install 5. |

## Install 3

| Install 3 | Details |
| --- | --- |
| Installed | 2026-11-24 (03:07) |
| Build | `e7667f26` |

| Fix | What it does | More |
| --- | --- | --- |
| Guard the Edge in a waiting room | Turning Lobby Practice Off while a drill runs now gives the room's partner back. In install 2 the drill's borrowed fighter stayed. |  |
| The menu and a macro on Player 1 | While the menu is open the CPU stands still. A macro queued or playing on Player 1 now lifts that hold, so it plays as recorded. | [Opening and closing](guide/using-the-menu.md#opening-and-closing) |

### Tested

| Check | Install 3 |
| --- | --- |
| Installed | Yes, and read back. It went on the card without an Eden run of that build. |
| Tested in Eden afterwards (2026-11-24) | In a waiting room, the Guard the Edge fix above and the practice partner's Stop passed. In Training, a run of the input changes passed for the frame strip's direction marks and for six fighters' drills, and found two defects that install 4 fixes: four fighters' special directions were saved as a step of their own, and nothing was graded after a Training reset. |
| The operator reported (2026-11-24, on this build), in Quick Play | In Quick Play, after a match the loser declined the rematch, picked a different fighter and came back to the room: the earlier fighter was put back as both the CPU and Player 1. Install 4 changes this (a different fighter picked at the character select starts the practice fresh). That fix has not been run in Eden. |
| The operator reported (2026-11-24, on this build), in the clip editor | The recording crop and preview showed a black screen, without the fighters and the timeline. In install 4 the clip editor is a strip over the game; its layout passed in Eden on `a9131331`. |

## Install 2

| Install 2 | Details |
| --- | --- |
| Installed | 2026-11-24 (00:43) |
| Build | `71158421` |

### Recording

| Change | What it does | More |
| --- | --- | --- |
| Chain and sequence pickers | B always backs out and saves nothing; X is done. | [Combos & Sequences](menu/recording.md#combos-sequences) |
| Drill preview | On a drill's row, X plays the take you recorded it from. A drill made another way reads "Re-record to preview". | [Input drills](guide/recording.md#input-drills) |
| Playback Slot on Mash | A Playback Slot starts once the fighter can act. | [The Playback Slot actions](guide/cpu-behaviour.md#the-playback-slot-actions) |
| Special directions in drills and combos | Each part of a directional special became a step. Install 4 replaces this with one rule for every fighter. |  |
| A start under a platform | A fighter saved in the air under a platform is placed there. |  |

### Practice games

| Change | What it does | More |
| --- | --- | --- |
| Guard the Edge and Juggler setup | The CPU waits at its picked start while you set up, and B puts it back. The Player 1 crosshair no longer leaves a floor, and the start point stops a little inside each end of the floor. | [Guard the Edge and Juggler](menu/fighters-and-stages.md#guard-the-edge-and-juggler) |
| Waiting rooms | After a drill the room's partner gets its Training Partner Behavior back. |  |

### The menu and the display

| Change | What it does | More |
| --- | --- | --- |
| The open menu holds the CPU | The CPU stands still while the menu is open, and Stop takes effect behind the menu. | [Opening and closing](guide/using-the-menu.md#opening-and-closing) |
| The frame strip | Marks the direction on a move. |  |
| A fighter pop-up | No longer leaves its box on screen after it closes. |  |

### Tested

| Check | Install 2 |
| --- | --- |
| Installed | Yes, and read back. |
| Tested in Eden (2026-11-23 and 24) | The combined verdict was partial. The rows that ran passed for list wrapping, the recording indicators, Record and Play on the CPU, the Playback Slot timing, the frame strip's marks and the Guard the Edge start points. One defect was confirmed and is fixed in install 3 (Lobby Practice Off during a drill). The runs were stopped for the install, so the remaining rows did not run. |
| The operator reported | Nothing is on record for this build. It was on the card for about two and a half hours. |

## Install 1

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

### Tested

| Check | Install 1 |
| --- | --- |
| Installed | Yes, and read back. |
| Tested in Eden (2026-11-23) | In Training, 16 of 27 rows passed, 5 passed in part and 2 failed (the Guard the Edge start point, reworked in install 2); 1 failed in its setup, 2 did not run and 1 was left to a hand check. In the waiting room, Record and Play on the CPU and Crop's save passed; the preview's timing rows did not run there. The review cleared it as a limited release on that basis. |
| The operator reported | Nothing is on record for this build. The defects it fixes were reported from the console on 2026-11-23, on the build before it. |

## Before install 1

The first rc.15 build (2026-11-15) added the clip editor, input drills, article hurtboxes and the colour legend
([Current release](CURRENT-RELEASE.md)). Between that build and install 1, more went into rc.15 without notes of its
own. By theme, with the page that describes each:

| Theme | What went in | More |
| --- | --- | --- |
| Practice games | Eggs-ercise, Break the Targets, Tech practice, Mash out, Parry practice, Guard the Edge and Juggler, each with its own screen and a Start row. | [Practice games](menu/fighters-and-stages.md#practice-games) |
| Waiting rooms | Lobby Practice is On from the first start. | [Lobby Practice is On by default](guide/waiting-rooms.md#lobby-practice-is-on-by-default) |
| Recording | The pop-up's Player or CPU side stays until you change it, and Place fighters before Record sets both fighters up first. | [Place fighters before Record](menu/recording.md#place-fighters-before-record) |
| The menu | A held Up or Down repeats and speeds up, and L and R turn the page from inside a game's screen, a fighter grid or an editor. | [Moving around](guide/using-the-menu.md#moving-around) |

The build on the card just before install 1 (`17060a5e`, installed 2026-11-23 at 11:40) was played by the operator
that day. What they reported, and what came of it:

| The operator reported | Since then |
| --- | --- |
| Lists did not wrap, nothing on screen showed that a recording was running, and a CPU recording's left and right shuddered. | Install 1 changes all three |
| Crop's preview kept the menu up and stopped early, cropping a Player 1 macro did not work, and the 1 Frame label left out its button. | Install 1 changes these too |
| Starting a new chain and going back before saving opened the keyboard, froze the menu for a moment and left both fighters frozen after the menu closed. | Install 2 changes the chain picker: B backs out and saves nothing |
| In Guard the Edge, picking the CPU in the fighter picker and backing out left a black box over Hitbox Visualization. | Install 2 puts back the part of the menu the pop-up had borrowed |
| The Guard the Edge case GJ1-2 worked on the build they were playing. | That build counted the KO immediately; waiting for Player 1 to return was new in install 1 |

Each report with its time and the build it was made on: [Install evidence](EVIDENCE-rc15-installs.md#the-operator-s-reports-from-the-console).

Earlier releases: [rc.9](RELEASE-NOTES-rc9.md), and rc.12 to rc.14 on their
[release pages](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases).
