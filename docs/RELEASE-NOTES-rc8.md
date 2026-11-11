# rc.8 (draft)

Notes for players. rc.8 is the internal name; the release's version name is set when it ships. Until then the menu's
footer shows this build as `0.2.0-rc.6 · rc8` and the commit it was built from (below). This draft lists what the rc.8
head `3f9d61dd` contains for players, plus the CPU pilot plugin's 0.2.3, which ships beside it. How to use recording is
[RECORDING.md](RECORDING.md), the frame strip is [FRAME-STRIP.md](FRAME-STRIP.md), and the pictures are the list in
[MEDIA.md](MEDIA.md).

## New

### Input display chord: R + D-pad Down

![R + D-pad Down hides the frame strip, then shows it again](media/rc8-input-display-chord.gif)

- Turns the input display (the frame strip) on and off without opening the menu, in Training and in the waiting-room
  practice.
- A notice says "Input display" and On or Off.
- It changes the same setting as Display & Controls › Input Display: the menu shows the same value afterwards, and the
  choice is saved.

### Game speed chord: hold R + D-pad Up

![The Game speed pop-up steps to 1/2, then 1/4; the jabs slow down; the pause menu shows Speed 1/4](media/rc8-game-speed-chord.gif)

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

![Back to x1: the stopwatch reads x1.5, then goes away](media/rc8-speed-hud-fixed.gif)
  - 1 Frame stays in the pause menu. While Training is on 1 Frame, the pop-up shows "1 Frame", and the first step goes
    from x1.
- **In the waiting-room practice:** 1/4, 1/2, 2/3, x1 and x1.5, local only.
  - Only your practice changes speed. The room's timers and the other player stay at normal speed, at 2/3 and x1.5 too.
  - Each practice session starts at x1.
- **Never in a match:** the chord never changes a Quickplay or online match, and a match found at any practice speed
  starts at normal speed.
- **With the standalone CPU pilot** driving Player 1, the chord works the same way. In the waiting-room practice the
  pilot still gets one input per game update at every speed, x1.5 included, so its inputs are never doubled.
  <!-- R14 (worker B's r14bc-eca14e4b pair): keep this line only if R14 passes on the final head. -->

### Both chords

- They're two new rows of Display & Controls › Button chords: "Input display" and "Game speed".
- They can be changed or cleared like the other chords.
- R is Z on a GameCube controller.

### The build in the menu footer

- The key hints at the foot of every menu page end with the build tag: the version, then `rc8` and the commit the NRO
  was built from, for example `0.2.0-rc.6 · rc8 3f9d61dd`.
- A build from uncommitted changes adds `-dirty`.

## Fixed

### Record is refused while an AI drives Player 1

- While an AI drives Player 1, for example the standalone CPU pilot, Record is refused, from the Record chord as well as
  from the pop-up. The notice reads "P1 is controlled by an AI: recording is off", and nothing is recorded.
- A recording that is already running can always be stopped.
- This replaces rc.7's known issue, where the Record chord still started a recording while the pilot drove Player 1.

## With the CPU pilot plugin 0.2.3

The CPU pilot is a separate plugin, `libssbu_cpu_pilot.nro`. Its 0.2.3 ships beside this build:

- The pilot can drive Player 1 in Quickplay's waiting-room practice too, armed from its CPU Pilot page as in Training.
  The practice shows no `CPU Lv` tag.
- When a match is found, the pilot keeps Player 1 in the match, through its online mode as before.
- TrainingRooms' tools work beside it in the practice: Record is refused while it drives, and the Game speed chord
  works at every practice speed.
- Local Wireless waiting rooms, matches and other online modes keep 0.2.2's behaviour.
