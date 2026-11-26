# Using the menu

Open the menu, move around it and change a row. It works in Training and in the waiting rooms. What each row does is
on its page's own doc: [CPU](../menu/cpu.md), [Saves](../menu/saves.md), [Recording](../menu/recording.md),
[Fighters & Stages](../menu/fighters-and-stages.md), [Display & Controls](../menu/display-and-controls.md).

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/b4aebb02d593833384f793c0b2801120ccd5c543" autoplay muted loop playsinline controls></video>

*The CPU stands. On the CPU page, D-pad Right flips Crouch On; B closes the menu and the CPU crouches. Eden, 2026-11-25, build `c51f4d81`.* [Open the clip](../media/tut-menu-change-option.mp4)

## Opening and closing

1. Hold **B** and press **D-pad Up**. This always works and can't be changed. **Minus** opens the menu too, unless you
   changed Menu shortcut 1 ([Controls](controls.md#opening-the-menu)).
2. Change what you want.
3. Press **B** to close it. Inside a list or an editor, B goes back one step first. Your changes apply when the menu
   closes.

- The menu opens in Training, and in a waiting room while Fighters & Stages ›
  [Lobby Practice](../menu/fighters-and-stages.md#lobby-practice) is On. It doesn't open over the game's pause menu.
- While it is open, your buttons go to the menu. Player 1 is not frozen. The CPU normally stands still, but not
  during a preview, save-state load, recording or macro playback. A macro queued for Player 1 also lifts the hold.
  A CPU fighter change waits until the menu closes.
- If the menu is closed for you (a match starts, or you leave Training), the changes from that visit are not kept.
- It reopens where you left it: the same page and row, and a list you left open.

## Moving around

| Buttons | What they do |
| --- | --- |
| L / R | Previous or next page. The pages wrap around. |
| Up / Down | One row. Hold to repeat; it speeds up. |
| ZL + Up / Down | The next or previous section. In a list, a screenful. |
| A | Opens the row, or flips a toggle. |
| Left / Right | Change the row's value. |

Two help lines describe the row you are on, and the footer lists the keys. A greyed row can't be changed until the
row it depends on changes. The Loading section on Fighters & Stages is collapsed; A on its header opens it.

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/941ad671c142b24ff631b02815e01f8006700c56" autoplay muted loop playsinline controls></video>

*Holding Down runs through the CPU page, faster the longer it is held, and Up runs back. On the Recording page, ZL + Down jumps from one list to the next. Eden, install 6's diagnostic build `5df3d9dc`, 2026-11-26.* [Open the clip](../media/tut-menu-list-scroll.mp4)

## Changing a row

| Control | Buttons |
| --- | --- |
| Toggle | A, Left or Right flips it. |
| List or number | Left / Right step the value. A opens the list: Up / Down choose, A sets, B goes back. |
| Checklist | A opens it. A ticks or unticks, ZL ticks all or none, B goes back. |
| Delay range | A opens it. Tick single values, or use the Min and Max handles; only one of the two holds a value. Y clears the one in use. More: [Delay rows](cpu-behaviour.md#delay-rows). |
| Slider (damage and Tech ranges) | A opens on Min. Up / Down choose Min or Max, A edits it: a tap moves one step, a hold speeds up. A or B puts the handle down. |
| Direction grid | The D-pad moves, each A adds weight to a direction, Y clears it, ZL selects all or none. More: [Direction chips](cpu-behaviour.md#direction-chips). |
| Editor (a summary and `>`) | A opens its own screen; follow its footer. |

## Your defaults

- **X** saves every current setting, on every page, as your defaults.
- **Y** puts the row you are on back to your saved default.
- **Reset page to defaults**, the last line of each page, asks first: A resets that page, B cancels.

The Recording page's lists use X and Y for their own actions; see [Recording](../menu/recording.md).

## Practice games

Under **Fighters & Stages › Practice games**, A opens a game's screen. Its first row is Start; while the game runs it
reads Stop. A on Start closes the menu and starts the game. A changed setting of a running game applies at its next
Start. More: [Practice games](../menu/practice-games.md).

More: [Controls](controls.md), [Getting started](getting-started.md).
