# 0.2.0-rc.5

Notes for players. This is the build after `0.2.0-rc.3.1`. There is no separate `0.2.0-rc.4`. How to read the frame strip is [FRAME-STRIP.md](FRAME-STRIP.md). How to use recording is [RECORDING.md](RECORDING.md). The pictures for those pages are the list in [MEDIA.md](MEDIA.md).

## Frame strip

The input display is the frame strip: the last second of Player 1 and the CPU, a chip for each new button, and readouts for advantage, the link, and the timing of the press.

ADVANTAGE is the CPU's first actionable frame minus yours after the last hit. Plus means you act first. The line says on hit or on shield. Actionable includes cancel windows, so an aerial can read more plus than a frame-data listing that waits out the landing lag.

A follow-up more than 20 frames after the CPU can act is a new exchange, not a dropped link.

Under Player 1's lane, a stick row shows the held direction on each frame. A C-stick attack, the attack the game marks as coming from the C-stick, takes its chip arrow from the C-stick when that stick is outside the dead zone. Any other press, including a C-stick that stays neutral, keeps the left-stick arrow. The stick row uses the left stick either way.

Holding one direction long enough can freeze the strip. A new binned direction, including a move out of neutral, starts it scrolling again.

## Recording

- A save state only warps. Load does not start a recording. Auto-load on death only warps to the bound save, and it does nothing when none is bound.
- Recording captures Player 1. Stopping writes a named macro and that macro's start save. The pop-up can show a CPU role, and the capture model can store a CPU recording, but the recorder is not given the pop-up's role.
- Each role has one active macro, a recording or a chain. Play on Player 1 and Play on CPU play that role's macro, or stop it if it is already playing.
- The loop never warps back. Later passes continue from where the fighters are.
- A chain starts at its first macro's start save, plays its recordings in order, and cannot contain a chain.
- Save last combo still saves the combo you landed. A drilled combo counts without having to be a true combo. A different move drops.
- Make combo does not save a combo. It plays one pass of Player 1's active macro. Crop does not open a handle screen. Both rows are on the file's screen; what they do is in [RECORDING.md](RECORDING.md).
- A sequence is an ordered list of combos, and the helper drills one thing at a time.
- The pop-up and the Recording page are how you bind a save, set a macro active, and drill.

## Chords

| Chord | Default | Change |
| --- | --- | --- |
| Save | ZL + D-pad Down | Writes the bound save. |
| Load | ZL + D-pad Up | Warps to the bound save. Starts nothing. |
| Record | ZR + D-pad Down | Same chord. Captures Player 1. |
| Play on CPU | ZR + D-pad Up | Was Input Playback. Plays the CPU's active macro. |
| Play on Player 1 | ZR + D-pad Left | Plays Player 1's active macro. |
| Recording pop-up | ZR + D-pad Right | Opens the pop-up over the match. Press it again to close. |
| Cycle CPU | ZL + D-pad Left | Unchanged. |
| Cycle Player | ZL + D-pad Right | Unchanged. |

Open menu stays B + D-pad Up. Menu shortcut 1 stays Minus. Both Play chords may be held at once. An old settings file that does not have the two new chords still loads, and those chords appear at the defaults above.

The first time Training opens on a new install, the six reminders are still Welcome, Open Menu, Save State, Load State, Input Record, and Input Playback. The first Record or Play also posts, once: "Record, Play on Player 1, Play on CPU, and the recording pop-up replace Input Record and Input Playback."

## Migration

On the first build of the new library:

- Empty recording slots and empty save indexes are skipped.
- A used recording slot becomes a macro named `Slot N (YYYY-MM-DD)`, with the date the migration runs. It has no start save, so Play runs it from where the fighters are.
- A used save index becomes one save file of both fighters, with the same kind of name. The slot Save and Load were using becomes the bound save.
- The named library's selected file becomes Player 1's active macro.
- Nothing is turned into a chain. Sequences and chains start empty.
- Loop On becomes infinite. Loop Off, or no loop row, becomes 1. Player 1 and the CPU then keep their own counts.

## What leaves

The playback machine no longer starts a recording because a save loaded. That job is the macro's start save, started by the role's Play chord.

The slot number, Randomize slots, Record Slot, Trigger, the Duration dropdown, button slots, the save-state playback checklist, Source, and per-slot playback leave the Recording page. Their setting ids stay in the file, so an older settings file still opens. Per-slot playback chords are not added.

The file cap stays at 36,000 frames. The capture stops taking frames there.

The Saves page keeps the chord switch, mirroring, auto-load on death, damage on load, buffs, and the setup library. Setups stay a separate library from save states. Respawn on platform stays under Fighters & Stages › Stages.

Pins are gone. The five most recently used of each kind are listed first. Y does nothing on the pop-up or the Recording page.

## Since 0.2.0-rc.3.1

Recording and the stick row are above. The same build also adds:

- The combo helper under the frame strip, and combo files you can copy or share. Any fighter can practise one. Grading uses the recorded presses, releases, C-stick, and stick flicks. Drift is shown and is not graded. A link counts without being a true combo. Switch or Keep can change Player 1 to the combo's recorded fighter, and Player Swapping Off does not block that Switch. Combos open in lobby practice. In an online waiting room a saved combo is named Combo N. A saved combo's footer shows the recorded fighter. Notices sit under the frame strip.
- Add New on a Mii class opens Smash's own Mii editor. Saving adds one Mii. Leaving without saving adds none.
- Lobby Items, on Fighters & Stages › Stages. Off stops item spawning in the waiting room.
- The replay window, edit placement, and the setup library. A timer match's replay capture continues through sudden death instead of stopping at the end of regular time.
- The six intro reminders show once on a new install. Each notice has a dark plate. Respawn on platform is on the Stages page. The Player cycle numbers are the list that cycle loads, and the CPU cycle is one list.
- Ready Fighters defaults to 10. Ready Hats are counted apart from those fighters. Raising the count promotes a parked CPU body. A Training count closes when that scene closes. A greyed Mii class is not in the CPU cycle.
- A named macro can carry the exact Mii it was recorded on.
- Waiting-room hitboxes resolve when the waiting room is the first practice of the boot.
