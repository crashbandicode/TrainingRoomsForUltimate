# Fighter cycles

Switch fighters without leaving practice. A chord steps Player 1 or the CPU through a list you choose; Kirby can also
step through a list of copy hats.

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/6d08f7457ab8d11ca30c8d7ed1140dd4611397e9" autoplay muted loop playsinline controls></video>

*The Cycle Player chord, ZL + D-pad Right, swaps Mario for Donkey Kong where he stands. Eden, install 6's diagnostic build `5df3d9dc`, 2026-11-26.* [Open the clip](../media/tut-fighter-cycle.mp4)

## The cycle chords

| Chord | Default | What it does |
| --- | --- | --- |
| Cycle Player | ZL + D-pad Right | Loads Player 1's next fighter; needs Player Swapping On. |
| Cycle CPU | ZL + D-pad Left | Loads the CPU's next fighter. |

They work in Training and in a waiting room with [Lobby Practice](../menu/fighters-and-stages.md#lobby-practice) On;
there, Cycle CPU changes the first training partner. Each role's starting fighter is **0**. Each press loads the next
number in that role's list; after the last, it returns to 0. With more than one CPU in Training, the extra CPUs take
the next fighters of the CPU list. To change the buttons, see [Controls](controls.md#changing-a-chord).

A notice explains when a switch cannot happen, for example while another fighter swap or a save-state load is running.

## Player Swapping

Fighters & Stages › [Player Swapping](../menu/fighters-and-stages.md#player-swapping) starts Off. Cycle Player does
nothing and shows no notice until you turn it On. A pick in Player Character still loads when the menu closes; the CPU
chord and CPU Character work either way.

![Player Swapping Off, then On: the same chord replaces Mario with Donkey Kong only when On](../media/rc13-player-swapping-gate.gif)
*The same Cycle Player chord with Player Swapping Off, then On. Eden, rc.13 (untested on hardware).*

## The Character Cycles editor

Open Fighters & Stages › [Character Cycles](../menu/fighters-and-stages.md#character-cycles) with A. Red numbers are
Player 1's order and blue numbers the CPU's; the help line says which list you are editing.

| Button | In the grid |
| --- | --- |
| ZR | Switch Player / CPU list. |
| A on a fighter | Add it at the end; from All, start a new list with that fighter. |
| A on All | Take every fighter in character-select order. |
| R | Remove a fighter; the last one stays. |
| X on Pokémon Trainer or Pyra / Mythra | Show the next form; A adds that form. |
| Y on Ice Climbers | Mark or unmark Solo Popo for that list. |
| B | Go back; the order applies when the menu closes. |

Both lists start as All. Y on the Character Cycles row, outside the grid, restores both to your saved defaults. A Mii
class is numbered only while that role has a Mii of the class it can load; see [Mii fighters](mii.md).

## Solo Popo

Ice Climbers can be the pair or Popo alone. The three controls start Off:

| Which Ice Climbers? | Control |
| --- | --- |
| Player 1's starting fighter, including a later pick of it in Player Character | [Solo Popo (original Ice Climbers)](../menu/fighters-and-stages.md#solo-popo-original-ice-climbers) |
| Player 1's cycle, or a Player Character pick after starting as another fighter | Y on Ice Climbers in the Player cycle list |
| The CPU's pick and cycle | Y on Ice Climbers in the CPU cycle list |

If Player 1's original and cycle controls differ, Ice Climbers can appear twice in the cycle: as 0 one way and as a
number the other.

## Kirby hats

Hold **B** with the left stick at rest, then click the **left stick** for your Kirby's next hat, or the **right
stick** for the CPU's. This chord is fixed. Kirby takes the hat when standing or falling, not during a move; if Kirby
lost its hat, the next click gives that hat back before moving on.

<!-- CLIP: tut-kirby-hat.mp4 | Kirby as Player 1: hold B, click the left stick → the next hat, twice -->
*Clip coming.*

Choose the order in Fighters & Stages › [Kirby Hats](../menu/fighters-and-stages.md#kirby-hats). ZR switches the Player
and CPU lists; A adds a hat, All takes everyone, R removes one, X previews a shared slot's form, and B goes back. Each
list starts with Mario only. With Training's CPU Behavior set to Control, the controller driving the CPU uses B and
either stick click for the CPU's hat.

## What the Loading rows do

TrainingRooms can load cycle fighters and Kirby hats ahead of time so a switch need not load them from scratch.
Fighters & Stages › [Loading](../menu/fighters-and-stages.md#loading) controls how many are kept ready and which
groups are loaded. If a fighter is not ready, a swap waits for it to load. A Fighters notice shows loading progress
or why loading is waiting. The defaults suit most players; see the menu page for each row and its memory trade-off.

More: [Fighters & Stages](../menu/fighters-and-stages.md), [Controls](controls.md).
