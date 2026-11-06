# Recording redesign — revision 2

Phase 1 screens. Nothing here is built. The menu pictures are redrawn on the real rc.1 Recording page (`eden-runs/tr-p23-docs-capture-03`, the same frame the combo-helper mockups used). The pop-up, the chain, and combo-from-macro sit on a real Training frame: `artifacts/phase23/overlay-investigation/frames/fix-tr-training.png` (Steve and Villager on Battlefield). Names and lengths are examples.

Green is the recommendation on the one question that is still open. Everything else in this file is decided, either by the operator or as the reading of those answers.

## Revision 2

- **The start save is a link. Decided.** Record writes a save named "<macro name> · start" and the macro stores that file's id. Sharing the macro means sharing both files. Renaming the save keeps the link, because the link is the id. Saving over that same file moves the remembered start. Deleting it leaves the macro playable from where the fighters are.
- **The delete notice is not approved.** The candidate sentence is "No start save. This macro plays from where the fighters are." It is a restriction string, so it waits for a yes. Until then, no new sentence is shown and the macro still plays.
- **Respawn on platform** leaves Saves. It is a row under Fighters & Stages › Stages. `p23/respawn-label` implements the move. The setting id stays `respawn_invincibility`.
- **The pin** is a tilted thumbtack: a flat head and a needle.
- **Where a save sits.** The pictures use a Save states section on both the Player tab and the CPU tab. A save is both fighters, so it is not a third role. The other option, a third top tab named Saves, is below.
- **Hot save states** stay open. The recommendation is still five, with the same pin and newest rules as macros.

## Already decided

- One Recording page, plus one pop-up. Record follows the pop-up's role: Player records you, CPU records the CPU.
- Hot memory is one minute resident per hot macro (3,600 frames, 28.8 KiB). Five hot macros are 144 KiB. A longer file still plays; the rest is read in 600-frame windows. The file cap stays 36,000 frames. No recording is refused for length.
- A new recording takes a hot slot even when all five are pinned. The oldest pin leaves the hot list and stays on the card.
- Loop counts are ∞, 1, 2, 3, 5, 10. X cycles them. The list is the Options row.
- Player 1 and the CPU may play at the same time.
- Save states are named files. You pick one in the pop-up or on this page and bind it. Save and Load use that file. There is no slot number and no extra slot shortcut.
- A save can be chained to a macro. Chain is on or off for that save. When it is on, loading the save starts that macro.
- The old "play a recording because a state loaded" checklist is this chain. It is not a separate toggle.
- Pins are a thumbtack. The active macro, the drilled combo, and the bound save are a green row. Hot items sit at the top inside an outline. Pinned items are first inside that outline.
- R moves between sections. The sections are Macros, Combos, and Save states. Combos is only on the Player role, because the one drilled combo is the player's.
- Names use the Switch's own keyboard, the one TrainingRooms and the fork already open. This document draws it as a labelled schematic.
- A recording's start is its own save file, "<macro name> · start", and the macro stores that file's id. **Decided.**
- Respawn on platform is under Fighters & Stages › Stages, not under save states. **Decided.** The move itself is `p23/respawn-label`.

## How to read a list

- The **green row** is the one that is in force: the macro that plays, the combo being drilled, or the save Save and Load use.
- The **blue-gray row** with the white bar is the cursor. It can sit on a different row from the green one.
- A **gold thumbtack**, tilted, with a flat head and a needle, is pinned.
- The **green outline** is the hot set: at most five, pins first, then the newest. Everything under the outline is the rest of the library, on the card.

## Pop-up

The pop-up is one panel over the match, so its size can be judged against the fighters. B closes it. It owns the buttons while it is open, the way the menu does.

L switches role (Player / CPU). R moves to the next section and wraps. A selects the row under the cursor: that macro becomes the active one for this role, that combo becomes the drill, or that save becomes the bound save. Y pins or unpins. X cycles the loop. ZL+Up and ZL+Down jump 12 rows, the same jump the menu uses, and they stop at the ends.

### Player, Macros

![Pop-up, Player, Macros](RECORDING-REDESIGN-popup-player.png)

Nair loop is green, so Play on Player 1 toggles that file. Dash grab is the cursor, and it is pinned, so the thumbtack is on a row that is not the green one. Replay 86f and Slot 2 are hot and unpinned. Macro 6 and Slot 5 are on the card, outside the outline.

The loop mark here is "on": green, with a dot under it. That is ∞.

### Player, Combos

![Pop-up, Player, Combos](RECORDING-REDESIGN-popup-combos.png)

Ryu BnB is pinned, so it is above the other hot combos. Nair loop is the drill (green). Combo 2 is hot. Combo 4 is only on the card.

A on a combo row makes it the drill. There is one drill. It is the player's, on either role tab's meaning: the CPU tab does not have this section.

### Player, Save states

![Pop-up, Player, Save states](RECORDING-REDESIGN-popup-saves.png)

Corner pressure is the bound save (green) and it is pinned. Its chain is on, so Load starts Nair loop. Platform is pinned, chain off, and it is the cursor. Slot 1 is hot. Under the outline, Nair loop · start and CPU throw · start are the files Record created. Slot 3 is the rest of the library.

A binds the cursor's save. The Save chord then writes that file. The Load chord restores it, and starts the chained macro only when that row's chain is on.

### CPU

![Pop-up, CPU, Macros](RECORDING-REDESIGN-popup-cpu.png)

CPU throw is the macro Play on CPU toggles. The loop mark is the count 1, in the middle of the icon. R from here goes to Save states and skips Combos. The save list is the same library as on the Player tab. A save is the match, both fighters, not a per-role slot.

Record on this tab records the CPU's fighter. The new file becomes the CPU's active macro and enters the hot list.

### Loop mark

![Loop mark](RECORDING-REDESIGN-loop.png)

X cycles ∞, 1, 2, 3, 5, 10. Off is the white loop with no dot. ∞ is the green loop with the dot underneath. A count is the same green loop with the number in the middle. The loop belongs to the role tab, not to the section and not to the file. Each role remembers its own count. **Decided.**

### Pop-up buttons

| Button | What it does |
| --- | --- |
| A | Make the cursor's row the active macro, the drill, or the bound save |
| Y | Pin or unpin that row |
| X | Cycle the loop |
| L | Player / CPU |
| R | Next section. Player: Macros, Combos, Save states. CPU: Macros, Save states |
| ZL+Up / ZL+Down | Move 12 rows, stop at the ends |
| B | Close |

On the Recording page, R is the same section button (Macros, Combos, Save states, Replays, Options). L/R no longer change the top-level page while that page is open. Up onto the tabs, then Left or Right, leaves Recording. The other pages keep L/R as the page change.

### Where the save list sits

![Where a save sits](RECORDING-REDESIGN-save-where.png)

The pop-up pictures use A. Save states are a section on both role tabs, and A on a row binds it. B, a third tab named Saves, would bind the same way. It is the other option, not the one these screens use, because that tab would read as a third role.

## Recording page

The left column is the sections. R moves down that column from anywhere on the page. The right side is that section's list or its settings. The footer names R.

### Macros

![Recording page, Macros](RECORDING-REDESIGN-page.png)

Same rules as the pop-up. A opens that file's actions.

| Button | What it does |
| --- | --- |
| A | Open the macro's actions |
| Y | Pin or unpin |
| R | Next section |
| ZL+Up / ZL+Down | Move 12 rows |
| B | Close the menu |

### Combos

![Recording page, Combos](RECORDING-REDESIGN-page-combos.png)

The outline is the hot five. Ryu BnB is pinned, so it is first. The green row is the drill.

### Save states

![Recording page, Save states](RECORDING-REDESIGN-page-saves.png)

The green row is the file Save and Load use. Chain is a column, so a chained save is visible without opening it. Under the outline, "Nair loop · start" and "CPU throw · start" are the files Record created. Each macro links to its own by id. They are not the bound save unless you bind one.

The Save chord overwrites the bound file. If nothing is bound yet, Save creates a file named Save N, binds it, and puts it in the hot list. Duplicate, on the file's own screen, writes a second file and leaves this one. Saving Corner pressure does not change Nair loop · start. Saving over Nair loop · start does, because that is the file the macro points at.

### Replays

![Recording page, Replays](RECORDING-REDESIGN-page-replays.png)

| Row | What it does |
| --- | --- |
| Replay capture | On: saving a replay also keeps the inputs. This is the row that exists today. |
| Replay window | The existing window. Its In/Out crop saves a named macro into this library. It does not grow a second Crop command. |

A replay export uses the snap the window already has: a mid-action In walks back to idle or fall, at most two seconds, and that neutral start is what the file stores.

### Options

![Recording page, Options](RECORDING-REDESIGN-page-options.png)

| Row | What it does |
| --- | --- |
| Crop trailing neutral | On: idle frames at the end of a recording are dropped. Off: the recording keeps them. |
| Hitstun timing | When playback starts after the CPU is hit. The choices stay the ones this row has today. |
| Mash interrupts playback | On: a mash stops a playing recording. |
| Loop counts | The list X cycles. **Decided:** ∞, 1, 2, 3, 5, 10. |
| Button chords | Record, the two Play chords, the pop-up, and Save and Load. |

## One file's actions

### A macro

![Macro actions](RECORDING-REDESIGN-macro-actions.png)

| Row | What it does |
| --- | --- |
| Start save | The file Record created, "Nair loop · start". The macro stores its id. Renaming that save keeps the link. |
| Set active for Player 1 | Play on Player 1 toggles this file |
| Set active for CPU | Play on CPU toggles this file |
| Rename | The Switch keyboard. Header "Name macro". Empty OK keeps the previous name. A new recording that was never renamed is Macro N. |
| Crop | In and Out, then a new file. This file stays. |
| Make combo | Play the file once on Player 1 and keep the links. This file stays. |
| Pin | Thumbtack. Pins sort first inside the hot outline. |
| Delete | Removes the file from the library |

### A combo

![Combo actions](RECORDING-REDESIGN-combo-actions.png)

| Row | What it does |
| --- | --- |
| Drill | This becomes the one drilled combo |
| Rename | Header "Name combo". Empty OK on a new combo is Combo N. Online, the keyboard stays closed and the name is Combo N. |
| Crop | First link and last link. A saves a new combo. This file stays. |
| Pin | Same hot rules as macros |
| Delete | Removes the file |

### A save state

![Save actions](RECORDING-REDESIGN-save-actions.png)

| Row | What it does |
| --- | --- |
| Bind to Save / Load | The green row. The chords use this file. |
| Chain | On or off, Left and Right. On: Load starts the chained macro. Off: Load only restores the save. |
| Chained macro | Which file Load starts. A picks from the macro library. The macro plays on the role stored in that file. |
| Duplicate | A new save file, a copy of this one. This file stays bound. |
| Rename | Header "Name save state". A new save that was never renamed is Save N. Online, the keyboard stays closed. |
| Pin | Same hot rules, if the hot-save recommendation below is accepted. |
| Delete | Removes the file. If it was bound, Save and Load do nothing until another is bound. |

## The keyboard

![Switch keyboard, schematic](RECORDING-REDESIGN-keyboard.png)

This is the system keyboard, drawn as a schematic because it is the Switch's own screen, not a TrainingRooms keyboard. Training passes the header, the current name, and a 48-character limit, and reads back what OK returns. B cancels and keeps the old name. The same screen names a macro, a combo, and a save state. The header is the only difference: "Name macro", "Name combo", "Name save state".

Online, in a waiting room, the keyboard is not opened. A new file keeps Macro N, Combo N, or Save N, and it can be renamed later in Training.

## Combo from macro

![Combo from macro](RECORDING-REDESIGN-combo-from-macro.png)

Make combo is a row on that macro. It is not a conversion of the file into links. CPU playback writes the CPU's controller, and the combo strip only records Player 1 hitting the CPU, so the pass plays the frames on Player 1.

1. Load the linked save, "Nair loop · start", when that file is still there. The playable start is the macro's own start, which a replay export has already snapped to idle or fall. If the file was deleted, skip the load and play from where the fighters are. The notice for that case is the candidate sentence in Revision 2, and it is not approved yet.
2. The menu closes. The file plays once on Player 1. The strip records the exchange that opens during the pass.
3. If you want a shorter combo, Crop then asks for the first and last link and saves a copy. The new combo becomes the drill and enters the hot list. The macro file is not written.

No link uses the sentence that already exists: "No combo yet: land a hit and a follow-up first."

## Crop

### A combo, by links

![Crop a combo](RECORDING-REDESIGN-crop.png)

The outline is the kept range, first link through last link. Link 4 is outside it. A saves a new combo. The original stays, so it is the undo. Link offsets are already relative to each link's own start, so a link range stays valid. This is not a frame crop.

### A macro, by frames

![Crop a macro](RECORDING-REDESIGN-crop-macro.png)

In and Out are the Replay window's handles. L and R set them, Left and Right move the selected one. A saves a new named file. The original stays. On this screen L and R are those handles, not the section button. B returns to the macro.

## Shortcuts

![Button chords](RECORDING-REDESIGN-shortcuts.png)

| Chord | Default | Change from today |
| --- | --- | --- |
| Save | ZL + Down | Same chord. It writes the bound save file. |
| Load | ZL + Up | Same chord. It loads that file, and starts the chained macro when chain is on. |
| Record | ZR + Down | Same chord. The pop-up's role says who is recorded. |
| Play on Player 1 | ZR + Left | Was unused. Toggles the player's active macro. |
| Play on CPU | ZR + Up | Was Input Playback. Toggles the CPU's active macro. |
| Recording pop-up | ZR + Right | Was unused. |
| Cycle Player | ZL + Right | Unchanged. |
| Cycle CPU | ZL + Left | Unchanged. |

Menu shortcut 1 stays Minus. Open menu stays B + Up. All eight rows can be edited here.

Both Play chords may be on at once. **Decided.**

## Chain, on a load

![Load starts the chained macro](RECORDING-REDESIGN-chain.png)

Load of Corner pressure, with chain on, restores the save and then plays Nair loop. Chain off restores the save and plays nothing. The chain is stored on the save, not on the macro, and each save has its own on/off.

This replaces the old checklist of "which slot plays after a load", and it replaces "start recording when a state loads". Loading does not start a recording.

## Migration

Today's five recording slots become named macros, "Slot 1 (2026-11-05)" through "Slot 5", unpinned, and they start as the hot five. Macros and combos that already have names stay as they are.

Today's five save slots are one slot index for both fighters (`save_states.json` keeps a player record and a CPU record at each index). Each index becomes one file, "Slot N (2026-11-05)", holding both fighters. They start hot and unpinned. The slot the Save and Load chords were using becomes the bound save.

The old Input Record and Input Playback chords are the new Record and Play on CPU chords. One notice, once: "Record, Play on Player 1, Play on CPU, and the recording pop-up replace Input Record and Input Playback."

## Memory

A frame is 8 bytes.

| What is resident | Size |
| --- | --- |
| One hot macro, one minute | 3,600 × 8 = 28,800 bytes |
| Five hot macros | 144,000 bytes |
| One file at the 36,000-frame cap | 288 KiB, not all resident |
| Five files if they were fully resident | 1,440 KiB. Not the plan. |
| Five hot combos | A combo is at most 16 links. A few kilobytes. |
| Five hot saves | Each file is the two fighters' position, damage, and charges, plus a Mii only when the save has one. Far under the macro budget. |

**Decided** for macros: one minute resident, stream the rest. Hot saves are the open question below. They are small either way.

## What leaves

- The slot number on the Saves page, and Randomize slots.
- Record Slot, Trigger, and the Duration dropdown. The file cap stays. Nothing is refused for being long.
- Button slots, and the save-state playback checklist.
- Source. The named library is the only list.
- Per-slot playback chords. They are not added.

The Saves page keeps the on/off switch for the chords, mirroring, auto-load on death, damage on load, buffs, and the setup library. Setups stay a separate library from save states.

![Respawn on platform, under Stages](RECORDING-REDESIGN-respawn.png)

Respawn on platform is not on that Saves page and not on Recording › Options. It is under Fighters & Stages › Stages. The setting id stays `respawn_invincibility`. `p23/respawn-label` implements the move. This document only records where the row goes.

An empty recording still uses the existing "Recording was empty". A deleted start save does not yet have an approved sentence. The candidate, waiting for a yes, is "No start save. This macro plays from where the fighters are."

Fighter mismatch keeps the Switch / Keep prompt, for macros and for combos.

Lobby practice matches Training. The "Combos work in Training" sentence stays gone.

## Still open

![Hot save states](RECORDING-REDESIGN-choices.png)

### Hot save states

**A, recommended.** Five hot saves, with the same rules as macros: thumbtack pins first, then the newest, inside the outline. Binding a save puts it in the hot list. A new save takes a hot slot even when all five are pinned. The auto-created "<macro name> · start" files stay under the outline unless you pin or bind them.

**B.** No hot list for saves. The pop-up shows the whole library, and only the bound save is marked.

## Pictures

Menu chrome: the rc.1 Recording page, `eden-runs/tr-p23-docs-capture-03/out/c15-q7/b-021632Z-877073-q7-row.png`.

Match: `artifacts/phase23/overlay-investigation/frames/fix-tr-training.png`.

The keyboard is a schematic of the system keyboard. It is not a capture of the Switch keyboard, and it is not a new TrainingRooms keyboard.
