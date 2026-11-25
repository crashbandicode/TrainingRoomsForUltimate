# Controls

Use and change the button chords for practice. They work in Training and in the waiting rooms.

<video src="../media/tut-controls-change-chord.mp4" autoplay muted loop playsinline controls></video>

*Button chords: Save State Save reads ZL + D-pad Down; X, then holding ZL + D-pad Left sets the new chord, and that chord saves. Eden, 2026-11-25, build `c51f4d81`.* [Open the clip](../media/tut-controls-change-chord.mp4)

A chord is "hold one button, then press the others". It fires once per press. The game doesn't see a chord's
buttons while you hold them, and chords don't fire while a menu is open.

## The chords

| Action | Default | Change it at |
| --- | --- | --- |
| Save State Save | ZL + D-pad Down | Button chords |
| Save State Load | ZL + D-pad Up | Button chords |
| Record | ZR + D-pad Down | Button chords |
| Play on CPU | ZR + D-pad Up | Button chords |
| Play on Player 1 | ZR + D-pad Left | Button chords |
| Recording pop-up | ZR + D-pad Right | Button chords |
| Cycle Player | ZL + D-pad Right | Button chords |
| Cycle CPU | ZL + D-pad Left | Button chords |
| Input display | R + D-pad Down | Button chords |
| Game speed | Hold R + D-pad Up, then Left / Right | Button chords |
| Open the menu | Minus | Menu shortcut 1 |
| Open the menu | Disabled | Menu shortcut 2 |

- Save and Load need Saves › Save States On (the default).
- Cycle Player needs Fighters & Stages › [Player Swapping](../menu/fighters-and-stages.md#player-swapping) On. It is
  Off at first. Cycle CPU always works.

## Changing a chord

1. Open **Display & Controls › Button chords** with A, choose a row and press A.
2. Press X, then let go of every button.
3. Hold the new chord and let go.
4. A keeps it, B cancels, Y empties it, which turns that action off.
5. Close the menu. The new chord works from then on.

Two rows may share a chord; both actions then happen. Y on the Button chords row puts all ten back to your saved
defaults.

## Opening the menu

- **B + D-pad Up** always opens the menu. It can't be changed or turned off.
- **Menu shortcut 1** (Minus) and **Menu shortcut 2** (disabled) are changed on Display & Controls with the same
  steps as a chord. A change works at once. A shortcut can't be B + D-pad Up or the other shortcut's chord.
- The two shortcuts are kept in `sd:/ultimate/TrainingRooms/button_config.ini`, which you can also edit while the
  game is closed, for example `shortcut_2=ZL + DPAD_UP`. Names: A, B, X, Y, L, R, ZL, ZR, DPAD_UP, DPAD_DOWN,
  DPAD_LEFT, DPAD_RIGHT, Plus, Minus, Left Stick Press, Right Stick Press; `disabled` turns a shortcut off.

## Fixed buttons

| Buttons | What they do |
| --- | --- |
| B + D-pad Up | Opens the menu. |
| L + R + A | Reset. In Training it is the game's own reset; in a waiting room it puts both practice fighters back. Turn it on or off at Display & Controls › [L+R+A Reset](../menu/display-and-controls.md#l-r-a-reset). |
| Hold B, click the left stick | Your Kirby takes the next hat. The left stick must be at rest. |
| Hold B, click the right stick | The CPU's Kirby takes the next hat. |
| Hold R3, D-pad Left / Right | Scrolls the input display back and forward. Let go of R3 to return to now. |

## GameCube controller

GameCube L is ZL, GameCube R is ZR, and GameCube Z is R. There is no Switch L and no Minus, so open the menu with
B + D-pad Up; a chord that uses L or Minus can't be pressed. In the menu, Z turns the page and L + Up / Down jumps
between sections.

## Online Deluxe

With Online Deluxe installed, some chords wait so its own buttons work:

- While its window shows a settings page that uses a D-pad button, a chord on that button waits. Of the defaults,
  only Play on Player 1 and the Recording pop-up are affected.
- While L is held, Input display and Game speed wait, unless your chord uses L itself.
- On a GameCube controller, holding both triggers holds back Input display, Game speed and chords with one trigger.

## When the CPU pilot plays Player 1

While an AI plays Player 1 (the CPU pilot), Play on Player 1 and, in a waiting room, L + R + A are refused with a
notice. Record works: the recording holds Player 1's inputs as the game receives them, the AI's included.
More: CPU › [Player 1 control](../menu/cpu.md#player-1-control).

More: [Display & Controls](../menu/display-and-controls.md), [Using the menu](using-the-menu.md).
