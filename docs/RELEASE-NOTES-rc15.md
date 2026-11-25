# rc.15: the builds of November 2026

Notes for players. rc.15 is one release name for a line of builds. Each build's menu footer reads
`0.2.0-rc.15 · rc15` and then the commit it was made from, so the commit tells the builds apart
([Which build you have](guide/getting-started.md#which-build-you-have)).

Four of these builds were installed on the project's test console one after another, and are called install 1 to 4.
Times are UTC.

| Build | Installed | Footer ends with | In short |
| --- | --- | --- | --- |
| [Next build](#next-build-not-installed-yet) | not yet | | Record with an AI on Player 1, named grab and throw keys, B stops a preview, Delete for replay captures, named move marks, playback that starts without reading the library |
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

## Next build (not installed yet)

The build after install 4. Its name is expected to be rc.16. The first part below is merged; the second is still in
work. None of it has been seen in a game: each merge passed the host tests and builds only.

**How this build is tested:** one short Eden smoke boot on the finished build (each new item started once, no crash
and the expected log lines), then the operator's full test on the console.

### Merged

#### Record works while an AI plays Player 1 (merged 2026-11-25)

- With the CPU pilot plugin, or TrainingRooms' own Player 1 control, playing Player 1, Record now starts. The take
  holds Player 1's inputs as the game receives them, the AI's included.
- Before, Record was refused. Play on Player 1 and L + R + A are still refused while an AI plays Player 1.
- More: [Controls › When the CPU pilot plays Player 1](guide/controls.md#when-the-cpu-pilot-plays-player-1).

#### Grabs, throws and edge attacks are named in combo files (merged 2026-11-25)

- A combo file now writes `grab`, `throw`, `ledge_attack`, `getup_attack` and `trip_attack` by name in a link's
  `from` and `to`.
- **Not backward compatible:** install 4 and earlier skip a file that has one of these five there. Files without them
  are not affected, and this build still reads older files.
- More: [Combo files › The format](guide/combos.md#the-format).

#### B stops a preview, and the pop-up shows its chords (merged 2026-11-25)

- While any preview plays, the key line reads `B stop preview`. B or X stops it and drops the rest of its frames.
- After the last frame the result stays for about a second and the key line reads `B back`.
- The Recording pop-up's first help line shows the Record, Play on Player 1 and Play on CPU chords as you have set
  them.
- More: [Recording › Replay window](menu/recording.md#replay-window) and
  [The Recording pop-up](menu/recording.md#the-recording-pop-up).

#### Delete for replay captures (merged 2026-11-25)

- A capture's screen under RECORDINGS has a **Delete** row, and Y deletes the capture under the cursor in the Replay
  window. It is removed at once, without asking, as a macro is.
- Macros, combos and drills already made from it stay. The game's own replay in the Vault is untouched.
- More: [Recording › Recordings](menu/recording.md#recordings).

#### The clip editor's marks name the form of a move (merged 2026-11-25)

- A move mark on the editor's timeline keeps which form of the move it was: the first, second or third jab, the
  rapid jab or its finisher, which aerial, grab or throw, and a side tilt or smash aimed up or down.
- With Move names On, a mark reads as the combo and drill lists do, for example Jab 1, Jab 2 or Forward air. A take
  saved before this has no forms: its jab reads Jab.
- A take, and each fighter of a replay capture, stores the fighter it was recorded on, so its specials keep that
  fighter's own names whoever Player 1 is.
- More: [Recording › Replay window](menu/recording.md#replay-window) and [Move names](menu/recording.md#move-names).

#### Playback starts without reading the library (merged 2026-11-25)

- Play no longer reads the whole recording library before it starts. It reads the index, the file each side plays
  and its start save, and nothing while a macro plays. How long a start now takes has not been measured in a game.
- **Playback delay** is a new option under Recording › Options, None by default (10 to 120 frames). It waits after
  the fighters are placed and before the first input, once per Play. It applies to Play on Player 1, Play on CPU, a
  capture's Play back and Make combo's pass; not to the editor's previews or the CPU's Playback Slots.
- **Known limit:** if Play on Player 1 starts in the middle of one of Player 1's input polls, that one poll can pass
  what you hold instead of nothing. It is one frame at most, at the start.
- More: [Playback delay](menu/recording.md#playback-delay).

### In work

Planned for this build and not merged yet. Each moves up to Merged when it lands, and what it does may still change.

- **In work: Guard the Edge and Juggler start on the Training stage.** Both use the floor you stand on to find its
  ledges. Guard the Edge uses a ledge with open air beside it. Juggler keeps its above-floor start with the CPU over
  Player 1; an open gap is also allowed, but not a spot under the stage or inside solid ground. A floor with no
  open ledge shows a message.
- **In work: two Guard the Edge options.** One starts the CPU under the stage. The other starts the CPU with its
  midair jumps already used. Both are Off by default, which is how the drill works today.
- **In work: Eggs-ercise's Off stage zone stays out of solid ground.** Spots that are not off stage are dropped, and
  the zone stops at the next solid.
- **In work: placement's Off stage switch.** A fighter switched to Off stage whose spot is not off stage goes beside
  the nearest ledge that has open air. The crosshair then moves freely.
- **In work: placement on the crosshair.** Changing where a save state puts the fighters uses the crosshair, on the
  save state itself.

## Install 4

Installed 2026-11-25 (07:26). Build `a945b0c1`.

### Drills and combos

- **One rule for every fighter.** A step is one action: the input that starts it and what the game does with it. Its
  release, how long the button was held, a special's directions and its travel belong to that action. There is no
  per-fighter list any more. [How a combo is graded](guide/recording.md#how-a-combo-is-graded)
- **Special directions.** A special records a direction when the fighter starts to move a way that is not the
  special's own (up to two per special). They stay in the special's step instead of becoming a step of their own.
- **Held buttons.** A release is graded on how long the button was held, and only when the game reacted to it. A
  quick tap is not graded on its release.
- **Angled tilts and smashes** use the angle the game chose, not the stick position.
- **Steered specials.** A drill recorded while steering a special must be steered the same way.
  [Match angle and deflection](menu/recording.md#match-angle-and-deflection) (Off by default) also compares the
  stick's angle and deflection.
- **Ledge, get-up and trip attacks** are moves of their own.
- **Steps show their moves** instead of Step N, in the list, the bar and the editor.
- **Move names.** [Move names](menu/recording.md#move-names) (Off by default) shows common move names, and the game's
  own names for specials and Final Smashes.
- **L + R + A.** The reset chord by itself leaves no drill result and starts no load, however long it is held.
- **Result lines are drawn whole.** A reason line is no longer cut off at 24 characters.

### Recording

- **The clip editor is a strip over the game.** Crop and preview while both fighters stay visible. The timeline marks
  where each move starts and ends, Y snaps a handle to the nearest one, and a ZR tap swaps the preview between the CPU
  and Player 1. [Replay window](menu/recording.md#replay-window)
- **Recordings.** RECORDINGS is the first row of MACROS on the Recording page. It lists the latest and previous Vault
  captures and your saved captures; from one you can play it back or make a macro, a combo or a drill.
  [Recordings](menu/recording.md#recordings)
- **Trim.** Record drops idle frames at the Start, the End, Both or neither (the older On becomes End). Each macro and
  drill also has its own Trim for playback, which changes nothing in the file.
  [Trim trailing neutral](menu/recording.md#trim-trailing-neutral)
- **Playback indicator.** [Show playback indicator](menu/recording.md#show-playback-indicator) shows PLAY P1 or
  PLAY CPU while a recording plays.
- **The pop-up.** ZR switches between Player and CPU; ZL, L and R switch its tabs.
  [The Recording pop-up](menu/recording.md#the-recording-pop-up)

### The input display

- **Scroll back.** Hold R3 and press D-pad Left or Right to look through the last 600 displayed columns. That's about
  ten seconds while the strip advances continuously; a quiet stretch can stay on screen much longer. Let go of R3
  to return to the present. [Frame strip](guide/frame-strip.md)

### The menu

- **Sliders.** Up and Down choose Min or Max and change it; holding speeds up. Damage ranges move in 0.5% steps.
  [Changing a row](guide/using-the-menu.md#changing-a-row)

### Waiting rooms

- **Quick Play.** Picking a different fighter at the character select starts the waiting-room practice fresh.

### Tested

- **Installed:** yes, and read back.
- **Tested in Eden** (2026-11-25, Training): the drill grading rows for Pikachu, Fox, Palutena, Mario and Kirby; Trim;
  the playback indicator for Player 1; the pop-up's ZR switch; scroll-back; the clip editor; a capture played from
  RECORDINGS; Move names in the drill list. These ran on an earlier head, `a9131331`; two fixes separate it from the
  installed build. The reset chord, with Mario's and Kirby's drills as controls, ran on the installed build.
- **Not covered by the final Eden session:** the waiting-room editor, the Quick Play practice pick, new pick and
  rematch, the airborne steered special, the deflection half of Match angle and deflection, ledge attacks, the Trim
  setting's migration and the CPU's playback indicator. The Quick Play match was a font check for Online Deluxe.
- **The operator reported** (2026-11-25, on this build): the move names Jab 1 and Jab 2 are fine; and playback
  took a second or more to start. A change for that wait is merged for the next build; it has not been timed in a
  game. They also asked for placement changes and a Delete for replay captures; the Delete is merged for the next
  build, and placement is in work.

## Install 3

Installed 2026-11-24 (03:07). Build `e7667f26`.

- **Guard the Edge in a waiting room.** Turning Lobby Practice Off while a drill runs now gives the room's partner back.
  In install 2 the drill's borrowed fighter stayed.
- **The menu and a macro on Player 1.** While the menu is open the CPU stands still. A macro queued or playing on
  Player 1 now lifts that hold, so it plays as recorded.
  [Opening and closing](guide/using-the-menu.md#opening-and-closing)

### Tested

- **Installed:** yes, and read back. It went on the card without an Eden run of that build.
- **Tested in Eden afterwards** (2026-11-24): in a waiting room, the Guard the Edge fix above and the practice
  partner's Stop passed. In Training, a run of the input changes passed for the frame strip's direction marks and for
  six fighters' drills, and found two defects that install 4 fixes: four fighters' special directions were saved as a
  step of their own, and nothing was graded after a Training reset.
- **The operator reported** (2026-11-24, on this build):
  - In Quick Play, after a match the loser declined the rematch, picked a different fighter and came back to the
    room: the earlier fighter was put back as both the CPU and Player 1. Install 4 changes this (a different fighter
    picked at the character select starts the practice fresh). That fix has not been run in Eden.
  - The recording crop and preview showed a black screen, without the fighters and the timeline. In install 4 the clip
    editor is a strip over the game; its layout passed in Eden on `a9131331`.

## Install 2

Installed 2026-11-24 (00:43). Build `71158421`.

### Recording

- **Chain and sequence pickers.** B always backs out and saves nothing; X is done.
  [Combos & Sequences](menu/recording.md#combos-sequences)
- **Drill preview.** On a drill's row, X plays the take you recorded it from. A drill made another way reads
  "Re-record to preview". [Input drills](guide/recording.md#input-drills)
- **Playback Slot on Mash.** A Playback Slot starts once the fighter can act.
  [The Playback Slot actions](guide/cpu-behaviour.md#the-playback-slot-actions)
- **Special directions in drills and combos.** Each part of a directional special became a step. Install 4 replaces
  this with one rule for every fighter.
- **A start under a platform.** A fighter saved in the air under a platform is placed there.

### Practice games

- **Guard the Edge and Juggler setup.** The CPU waits at its picked start while you set up, and B puts it back. The
  Player 1 crosshair no longer leaves a floor, and the start point stops a little inside each end of the floor.
  [Guard the Edge and Juggler](menu/fighters-and-stages.md#guard-the-edge-and-juggler)
- **Waiting rooms.** After a drill the room's partner gets its Training Partner Behavior back.

### The menu and the display

- **The open menu holds the CPU.** The CPU stands still while the menu is open, and Stop takes effect behind the menu.
  [Opening and closing](guide/using-the-menu.md#opening-and-closing)
- **The frame strip** marks the direction on a move.
- **A fighter pop-up** no longer leaves its box on screen after it closes.

### Tested

- **Installed:** yes, and read back.
- **Tested in Eden** (2026-11-23 and 24): the combined verdict was partial. The rows that ran passed for list
  wrapping, the recording indicators, Record and Play on the CPU, the Playback Slot timing, the frame strip's marks
  and the Guard the Edge start points. One defect was confirmed and is fixed in install 3 (Lobby Practice Off during a
  drill). The runs were stopped for the install, so the remaining rows did not run.
- **The operator reported:** nothing is on record for this build. It was on the card for about two and a half hours.

## Install 1

Installed 2026-11-23 (20:04). Build `152629cd`. It fixes defects the operator found on the console that day, in the
build before it.

- **Lists wrap.** Up and Down wrap at the ends of a list. On the Recording page each list wraps by itself; Left and
  Right switch lists. [Moving around](guide/using-the-menu.md#moving-around)
- **You can see that you are recording.** A REC mark shows while a take is open, tinted for the side being recorded.
- **Recording the CPU.** The recorded left and right no longer flip with the way the fighter faces.
- **1 Frame names its button.** The game-speed label shows which button steps a frame.
  [Game speed](guide/game-speed.md)
- **Crop's preview plays through Recording's playback.** It hides the page, plays to the end and works for a
  Player 1 macro. [Crop](guide/recording.md#crop)
- **Guard the Edge.** It waits for a KO-ed Player 1 to return before it starts, and a KO counts once Player 1 is
  back.

### Tested

- **Installed:** yes, and read back.
- **Tested in Eden** (2026-11-23): in Training, 16 of 27 rows passed, 5 passed in part and 2 failed (the Guard the
  Edge start point, reworked in install 2); 1 failed in its setup, 2 did not run and 1 was left to a hand check. In
  the waiting room, Record and Play on the CPU and Crop's save passed; the preview's timing rows did not run there.
  The review cleared it as a limited release on that basis.
- **The operator reported:** nothing is on record for this build. The defects it fixes were reported from the
  console on 2026-11-23, on the build before it.

## Before install 1

The first rc.15 build (2026-11-15) added the clip editor, input drills, article hurtboxes and the colour legend
([Current release](CURRENT-RELEASE.md)). Between that build and install 1, more went into rc.15 without notes of its
own. By theme, with the page that describes each:

- **Practice games.** Eggs-ercise, Break the Targets, Tech practice, Mash out, Parry practice, Guard the Edge and
  Juggler, each with its own screen and a Start row. [Practice games](menu/fighters-and-stages.md#practice-games)
- **Waiting rooms.** Lobby Practice is On from the first start.
  [Lobby Practice is On by default](guide/waiting-rooms.md#lobby-practice-is-on-by-default)
- **Recording.** The pop-up's Player or CPU side stays until you change it, and Place fighters before Record sets
  both fighters up first. [Place fighters before Record](menu/recording.md#place-fighters-before-record)
- **The menu.** A held Up or Down repeats and speeds up, and L and R turn the page from inside a game's screen, a
  fighter grid or an editor. [Moving around](guide/using-the-menu.md#moving-around)

The build on the card just before install 1 (`17060a5e`, installed 2026-11-23 at 11:40) was played by the operator
that day. The operator reported:

- lists did not wrap, nothing on screen showed that a recording was running, and a CPU recording's left and right
  shuddered. Install 1 changes all three;
- Crop's preview kept the menu up and stopped early, cropping a Player 1 macro did not work, and the 1 Frame label
  left out its button. Install 1 changes these too;
- starting a new chain and going back before saving opened the keyboard, froze the menu for a moment and left both
  fighters frozen after the menu closed. Install 2 changes the chain picker: B backs out and saves nothing;
- in Guard the Edge, picking the CPU in the fighter picker and backing out left a black box over Hitbox Visualization. Install 2
  puts back the part of the menu the pop-up had borrowed;
- the Guard the Edge case GJ1-2 worked on the build they were playing. That build counted the KO immediately;
  waiting for Player 1 to return was new in install 1.

Each report with its time and the build it was made on: [Install evidence](EVIDENCE-rc15-installs.md#the-operator-s-reports-from-the-console).

Earlier releases: [rc.9](RELEASE-NOTES-rc9.md), and rc.12 to rc.14 on their
[release pages](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases).
