# Recording

Recording is save states, macros, chains, combos, and sequences. A save state only warps. A macro is a recording of inputs, and a chain is a macro made of macros. Each role has one active macro, and that role's Play chord plays it. Combos and sequences are what you drill. They have no chord.

Two layers are easy to mix up, so this page keeps them apart.

What a press does in a match: Load warps the save and starts nothing. The runtime asks the recorder what a finished load should do, and that answer is nothing. Record (ZR + D-pad Down) captures the pop-up's role and, when you stop, writes a named macro. The role stays Player 1 until the pop-up sets it. Play on Player 1 (ZR + D-pad Left) and Play on CPU (ZR + D-pad Up) play that role's active macro, or stop it. The recording pop-up (ZR + D-pad Right) opens and closes over the match.

The machine is `src/model/playback.rs` and the library. The pop-up, the Recording page, and the file screens are `src/model/recording_menu.rs`. The pictures are full-screen captures, 1280 wide, under 10 seconds, one behaviour each, and [MEDIA.md](MEDIA.md) is the list.

Files live under `sd:/ultimate/TrainingRooms/`. A macro, a chain, a combo, a sequence, and a save state are separate files. Renaming either end of a connection keeps it, because the connection stores the file's id.

## Save states only warp

Save (ZL + D-pad Down) writes the one bound save. Load (ZL + D-pad Up) warps both fighters to that save and starts nothing. It does not start a macro or a chain. One save is bound at a time, for both roles, because a save restores both fighters.

Auto-load on death only warps to the bound save. If none is bound, it does nothing. Mirroring, damage on load, and buffs still apply to that warp.

A save has no role and no chain. The old checklist that played a recording because a state loaded is not part of this machine.

![Load warps to the bound save and leaves the CPU standing](media/rc5-save-warp.gif)

## Recording a macro, and its start save

Record is ZR + D-pad Down. Stopping the recording keeps the inputs as a named macro. The recording also writes a save state named with the macro, "`name` · start", and the macro stores that file's id. The Start save switch defaults to On.

On means the role's Play chord warps to that save once, before the first pass, then plays. Off plays from where the fighters are and keeps the connection, so turning it On again warps there the next time you play. A macro that never had a start save has no switch. Replays and migrated slots are like that: Play starts where the fighters already are.

If the connected start save was deleted, playback skips the warp and shows "No start save. This macro plays from where the fighters are." A macro that never had one does not show that sentence.

When you record, the new macro is the most recently used of its kind. Each kind keeps five of those, the most recently used first. A sixth use drops the least recent out of that five. It stays on the card.

![Record, then the new macro and its start save](media/rc5-record-start-save.gif)

## Set active, and the Play chords

Each role has one active macro. It is either a recording or a chain. Set active for Player 1 replaces Player 1's previous one. Set active for the CPU replaces the CPU's. The two roles do not share an active macro.

| Chord | Default | What it does |
| --- | --- | --- |
| Save | ZL + D-pad Down | Writes the bound save. |
| Load | ZL + D-pad Up | Warps to the bound save. Starts nothing. |
| Record | ZR + D-pad Down | Records the pop-up's role. Player 1 until that role is CPU. |
| Play on Player 1 | ZR + D-pad Left | Plays Player 1's active macro, or stops it if it is already playing. |
| Play on CPU | ZR + D-pad Up | Plays the CPU's active macro, or stops it. This chord was Input Playback. |
| Recording pop-up | ZR + D-pad Right | Opens the pop-up. The same chord closes it. |

Both Play chords may be on at once. They do not share a machine: Player 1 and the CPU can play at the same time. A chain does not use that. Its macros play one after another.

Stopping Record selects the new file as that role's active macro, so the next Play on that role plays it. A recording made for the CPU names the CPU's fighter. If that role has no active macro, its Play chord does nothing. With the Start save switch On, and the file still there, the chord warps to the start save once as it enters play, then plays the inputs. A later pass does not warp.

Mash interrupts playback, when that option is On, stops the macro that role is playing. A chain stops as that one macro.

The old per-slot playback chords are not added.

![Play on CPU warps once to the start save, then plays the active macro](media/rc5-play-chord.gif)

## The loop

The loop belongs to the role, not to a file. Player 1 and the CPU each keep their own count. The counts are infinite, 1, 2, 3, 5, and 10. A new index uses 1. An old Loop On becomes infinite, and Loop Off, or a missing loop row, becomes 1.

The Play chord warps to the start save once, before the first pass, when the switch is On. Every later pass plays on from where the fighters are. The loop never warps back. A recording and a chain follow the same rule. Infinite keeps going. A finite count plays the macro, or the whole chain, that many times.

![The second pass of a loop continues from where the fighters are](media/rc5-loop-no-warp.gif)

## Chains

A chain is a file in the macro list: an ordered list of recordings, numbered 1, 2, 3. It cannot contain a chain. The picker lists recordings only, so a chain is never a step of another chain.

A chain starts where its first macro starts, at that macro's start save. The chain has its own Start save switch, On by default. On warps there once when the Play chord starts the chain. Off plays from where the fighters are. The macros inside the chain do not use their own start saves, whatever those switches say. If macro 1 has no start save, the chain has none.

The role it is set active for is the role whose Play chord plays it. The macros play in order, from where the fighters are after that one warp. With a loop, the chain repeats as one unit: 1, 2, 3, then 1 again. No pass warps back.

An empty chain plays nothing. A chain whose recordings have all been deleted plays nothing. There is no extra sentence for either case. R in the numbered picker removes a number, including the last one, so the list may be empty. A adds the next number. A on a tile that already has a number does nothing. B is done. It is the Character Cycles picker, except that the last number can come off.

![The CPU's Play chord warps to macro 1's start save, then plays the chain in order](media/rc5-chain-play.gif)

## Combos

Save last combo, on today's Recording › Combos row, keeps the last combo the frame strip saw and makes it the drill. The file is the combo's links and the input events of each follow-up: presses, releases, the C-stick, and the stick flicks (fast-fall, dash, tap-jump). Make combo records the same events, because it plays into the same strip. Any fighter can practise it. The format is in [COMBOS.md](COMBOS.md).

A combo counts when its recorded input events are there, the same kind and the same facing-relative direction, in order, and within the on-time tolerance, and the move that comes out is the recorded move. TRUE is not required. A link that is not a true combo still counts. TRUE stays on the helper bar as information. Drift is the held stick between those events. It is shown and it is not graded.

A different move does not count, even when the events match. That attempt drops. An attempt also drops on a missing event, an event outside the tolerance, a different direction, or the exchange ending before the step. An older file that has only presses is graded on those presses, with left and right still one direction. The exchange ends 20 frames after the CPU can act again, or on a shielded hit. A gap of 20 frames still belongs to the exchange. A gap of 21 opens a new one, so the step drops. A single combo's best stays the most true links in one attempt. The streak follows the count above, so a non-true link can still extend the streak.

![A follow-up that is not a true combo still counts, and a different move drops](media/rc5-grade-not-true.gif)

Make combo, on a macro's or a chain's own screen, plays that file once on Player 1 from the file's own start save. A chain uses its first macro's start save. The pass is stripped to its links and saved as a new combo named Combo N, with Made from set to the source. That combo joins the five most recently used. The file on screen stays. The drill does not change. A pass with no hit and no follow-up writes nothing and shows "No combo yet: land a hit and a follow-up first."
If the connected start save was deleted, Make combo skips the warp, shows "No start save. This macro plays from where the fighters are.", and still plays. A switch that is Off, or a file that never had a start save, does not warp and does not show that sentence. A chain whose first macro is missing skips that step and plays the next one that is still there.
Play skips a deleted start save and shows the same sentence. A macro that never had one does not show it.

![Make combo plays the macro once and leaves the current drill in place](media/rc5-make-combo.gif)

Crop on a combo or a macro opens a handle screen. Up and Down switch In and Out. L and R move the handle that is focused, and they do not change the menu page. A saves a new file under the next unused Macro N or Combo N and returns to the original. B returns without writing. The original stays. A macro crop keeps that range of frames. The new file keeps the original's one start state, stamped at the clamped start frame. It does not store a pose for every frame. A combo crop keeps the links from the first handle through the last. The replay window is a separate row under Replays. It is not this Crop row.
A chain has no Crop row, because it has no frames of its own.

![Crop keeps a range of links as a new combo](media/rc5-crop-combo.gif)

There is no chord for a combo. Choosing Drill makes it the one drill.

## Sequences and the one drill

A sequence is an ordered list of combo files. It stores each combo's id. Rename a combo and the sequence shows the new name. Delete a combo and the sequence skips that step. An empty sequence drills nothing.

The helper drills one thing. Drilling a sequence replaces a drilled combo, and drilling a combo replaces a drilled sequence. A numbered row inside a sequence drills that combo alone, with that combo's own streak, and the sequence file stays.

The bar under the frame strip is the combo helper's bar with a sequence header and a row for the sequence's combos. A finished sequence adds one to the streak, and best is the highest streak. A drop sends the sequence back to combo 1 and the streak to 0. Best is kept. TRUE is shown and is not required, including between the combos of a sequence. The same grade as a single combo decides each step: the input events, the direction, the order, the tolerance, and the move. Drift is shown and is not graded.

The picker is the same numbered picker as a chain, including R removing the last number.

![Drill a sequence: combo 2 of 2, then a drop back at combo 1](media/rc5-sequence-drill.gif)

## The pop-up

The pop-up is one window over the match. While it is open it takes the menu's buttons. B closes it and leaves the bound save, the active macros, and the cursor as they were.

The title reads `Recording (Player 1)` or `Recording (CPU)`. L sets Player 1. R sets the CPU.

Player 1 has two tabs, Save states & Macros and Combos & Sequences. ZL and ZR switch those tabs. The CPU view has only Save states & Macros, and ZL and ZR do not change it.

Each tab is two stacked sections. Each section shows four rows under a header that stays put, and that section's window follows its own cursor. ZL + Up or ZL + Down moves the section the cursor is in by 12 rows. Left and Right move between the two sections.

The five most recently used of a kind are listed first, most recent first. Those rows sit in a green outline. A hairline is drawn under the fifth when that boundary is in the four-row window. The status line still says hairline=5 when the fifth row is scrolled off this window.

The connections of the row under the cursor are the menu's two help lines, headed CONNECTIONS. They are not a panel on the right. A chain that is the active macro of the role in view includes that role's loop on those lines. Other connection lines, such as a start save, a chain step, a combo made from the file, or the recorded fighter, are not limited to that role. The MACROS header shows the loop of the role in view.

The bound save, the active macro of the role in view, and the drilled combo or sequence are green rows. The name stays the ordinary light colour. The cursor on these screens is a white border and a white bar on the left. A row can be green and hold the cursor at the same time.

A on a save binds it to Save and Load. A on a macro or a chain sets it active for the role in view. A on a combo or a sequence drills it. X cycles the loop, and only while the cursor is in Macros, and that count is written to the recording index. Y does nothing on this window.

![The pop-up over the match, Player view, cursor on a save](media/rc5-popup.gif)

The orange floppy is the save state a macro starts at. It is grey when that macro's Start save switch is Off. The Save states header uses the same orange floppy. A chain row shows how many macros it contains. The layout has no chain-mark picture yet, so the count is the chain mark. With the cursor on a chain, those macros show 1, 2, 3 in front of the name. + New chain is the last row under Macros, and + New sequence is the last row under Sequences. A opens the numbered picker. B on the picker opens the name step. In an online waiting room that name step does not open the keyboard. The new file stays Chain N or Sequence N.

## The Recording page

The Recording page uses the same stacked sections and the same CONNECTIONS help lines. The left column is Save states & Macros, Combos & Sequences, Replays, and Options. There is no tab switch on the page, because that column lists the groups. ZR switches Player 1 and CPU. L and R change the menu page, except while a file screen, the crop screen, the numbered picker, or the name step is open: then L and R stay on Recording. The CPU view hides Combos & Sequences. A opens the file's own screen. On a save, Bind to Save / Load is that screen's first row. X cycles the loop only when the cursor is in Macros, and that count is written to the recording index. B closes the menu when no file screen is open. On a file screen, B goes back.

![The Recording page, Player view, saves above macros](media/rc5-recording-page.gif)

The old Recording and Saves rows leave this page. Their setting ids stay, so an old settings file still loads. What leaves the page is the slot number, Randomize slots, Record Slot, Trigger, the Duration dropdown, button slots, the save-state playback checklist, Source, and per-slot playback. The file cap stays at 36,000 frames. The capture stops taking frames there. A recording of that length is kept. One frame more is not saved. Replays keeps Ask before switching fighter, Edit placement, Replay window, and Replay capture. Options keeps Crop trailing neutral, Hitstun timing, Mash interrupts playback, the loop's list of counts, Settings override playback, Combo helper, On-time tolerance, and After an attempt. Respawn on platform stays under Fighters & Stages › Stages.

### A file's own screen

The breadcrumb reads `Recording (Player view)  >  Save states & Macros  >  name`, or Combos & Sequences for a combo or a sequence. The CPU view says `Recording (CPU view)`.

Set active is one row, for the role in view. Right sets On for that role. Left sets Off. On makes this file that role's active macro. Left on a row that is already Off does not clear the other role.

A macro that has a start save lists Start save, then Set active, Rename, Crop, Make combo, and Delete. Without a start save, Start save is absent. A chain that has a start save on its first macro lists Start save first, then Set active, Edit chain, Rename, Make combo, and Delete. A chain has no Crop. A combo lists Drill, then Made from when it was made from a macro whose name is still there, then Rename, Crop, and Delete. A save lists Bind to Save / Load, Duplicate, Rename, and Delete. A sequence lists Drill sequence, a numbered row for each combo, Edit combos, Rename, and Delete. A on a numbered combo drills that combo alone.

Rename opens the Switch keyboard. B, or an empty confirm, keeps the old name. A name the library stores may be at most 48 characters, and it may not be empty or contain a control character. An empty confirm on a new chain or sequence keeps Chain N or Sequence N. A recording stopped with no typed name is Macro N. Duplicate names the copy Save N. Save overwrites the bound save, and when nothing is bound it creates Save N. A combo saved with no typed name is Combo N. In an online waiting room the keyboard stays closed, and B on the name step keeps that N name. A local-wireless waiting room and Training open the keyboard.

The combo's help lines list each link as the two moves and TRUE or not, then that link's input events, then its drift when it has one. The helper still says link for those hits. The word sequence is the file of combos, and the word chain is only the compound macro.

![A macro's own screen: Start save On, and Set active for Player 1](media/rc5-macro-screen.gif)

## Fighter Mismatch

Fighter Mismatch is the prompt a replay clip or a combo can open before it switches a fighter. The title is Fighter Mismatch. The rows are Keep current fighter and Switch to recorded fighter. Nothing is hidden or refused because the fighter differs. A sequence may mix fighters.

Play holds until those rows are answered. The cursor starts on Keep. Keep plays on the fighter you have. Switch changes to the fighter the file was recorded on, then plays. B cancels and does not play. A macro asks when its recorded fighter differs. A chain asks only for its first macro, and the line is `screen=mismatch ask=macro1`. Later macros are not checked and do not switch. A combo drill asks when the drilled combo changes to one recorded on another fighter. The first time a saved drill is seen, it does not ask. Load does not ask.

![Play of a chain whose first macro was recorded on another fighter](media/rc5-fighter-mismatch.gif)

## Migration of the old slots

The first time this library is built, empty slots are skipped. A recording slot that holds inputs becomes a macro named `Slot N (YYYY-MM-DD)`, using the date the migration runs and the slot's number starting at 1. Those macros have no start save, so they have no Start save switch, and they play from where the fighters are. No chain is built from them.

A save index that holds a save becomes one file, both fighters, with the same `Slot N (date)` name. The slot the Save and Load chords were using becomes the bound save. An empty index is skipped.

`macros/selected.txt`, when it names a file that is there, becomes Player 1's active macro. The CPU has no active macro until you set one. The old "play a recording when a state loads" row is not turned into a chain.

![Play on CPU runs a migrated slot from where the fighters are, with no warp](media/rc5-migration.gif)

## Where the rules come from

The playback machine, the files, the grades, the chords, and the migration are `src/model/playback.rs`, `src/model/library/`, `src/model/combo.rs`, `src/model/drill.rs`, `src/model/chords.rs`, and `src/model/library/migrate.rs`. The pop-up, the Recording page, and the file screens are `src/model/recording_menu.rs` and `src/model/ui/frame.rs`. Record, Play, and the pop-up chord are applied in `src/model/rr_host.rs`.
