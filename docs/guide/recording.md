# Recording

Recording lets you capture what you do, play it back on yourself or the CPU, and drill combos. This guide explains how
the pieces fit together. The [Recording page](../menu/recording.md) lists every row and button.

![The five kinds of file: save state, macro, chain, combo and sequence, and what plays or grades them](../diagrams/recording-pieces.png)

| File | What it is | You make it with |
| --- | --- | --- |
| Save state | Where both fighters stand. It only warps. | The Save chord, and Record (as "Macro N · start"). |
| Macro | A recording of inputs. | Record (ZR + D-pad Down). |
| Chain | A macro made of macros, played in order. | + New chain. |
| Combo | The links of a combo you drill. | + Save last combo…, or Make combo on a macro. |
| Sequence | Combos drilled one after another. | + New sequence. |

- Player 1 and the CPU each have one **active macro**, a recording or a chain. That side's Play chord plays it.
- The combo helper grades one **drill** at a time, a combo or a sequence. Combos have no chord.
- Each file is saved on its own under `sd:/ultimate/TrainingRooms/`. Renaming a file keeps its connections.

## Save states only warp

Save (ZL + D-pad Down) writes the one bound save. Load (ZL + D-pad Up) warps both fighters to it and starts nothing: no
macro, no chain. One save is bound at a time, for both sides, because a save holds both fighters. Auto-load on death
only warps to the bound save too, and with none bound it does nothing. Mirroring, damage on load and buffs still apply
to that warp. The full story is in [Save states](save-states.md).

![Load warps to the bound save and leaves the CPU standing](../media/rc7-save-warp.gif)

*Player 1 walks away, Load puts both fighters back, and the CPU just stands: nothing plays.*

![Auto-load on death: with a bound save Player 1 is back on stage at once; unbound, the game's revival](../media/rc6-autoload.gif)

*Player 1 is KO'd and is back at the bound save at once. With Bind to Save / Load turned Off on that save, the next KO
gives the game's revival.*

## Recording a macro, and its start save

Press Record (ZR + D-pad Down) to start and again to stop. The inputs are kept as a named macro (Macro 1, Macro 2, …;
rename it on its own screen). The Record presses themselves are not in the recording.

![Record writes the inputs as a macro and the spot as its start save; the macro becomes the active macro of the side that recorded](../diagrams/recording-record-flow.png)

- **Who records:** the side the pop-up is set to (Player 1 until you pick CPU there). A recording made for the CPU
  names the CPU's fighter. Stopping makes the new macro that side's active macro, so you can Play at once, and the
  most recently used of its kind (the [hot five](../menu/recording.md#save-states--macros)).
- **The start save:** Record also writes a save state, "Macro 1 · start", with both fighters where they stood. The
  macro remembers that save, and the fighter's facing and speed at that moment.
- **Start save switch** (on the macro's own screen, On by default): On warps both fighters to the start save once,
  then plays. Off plays from where the fighters stand; the connection stays, so On again warps next time.
- **Idle frames at the end** are dropped when Recording › Options ›
  [Crop trailing neutral](../menu/recording.md#crop-trailing-neutral) is On (the default). A take that is idle
  throughout saves nothing ("Recording was empty").
- **Length:** the capture stops taking frames at 36,000 (10 minutes). A recording that long is kept, and one frame more
  is not saved.
- **No start save:** macros made from replays, and ones carried over from the old numbered slots, have none and no
  switch: Play starts where the fighters already are. If a start save was deleted, Play skips the warp and shows "No
  start save. This macro plays from where the fighters are."

![An empty library, Record on Player 1, two jabs, the stop: Macro 1 and its start save](../media/rc7-record-macro.gif)

*The pop-up lists nothing; Record, two jabs, stop; it then lists Macro 1 (recorded on Mario) and "Macro 1 · start".*

<!-- CAPTURE-REQUEST rec-start-save-onoff: a side-by-side GIF of the Start save switch: the same macro played with it On (warps to the start save first) and Off (plays from where the fighters stand). See artifacts/docs4/CAPTURE-REQUESTS.md. -->

## Set active, and the Play chords

A macro, or a chain, plays on the side it is active for. Set active for Player 1 replaces Player 1's old one, and Set
active for the CPU replaces the CPU's: the two sides don't share one.

| Chord | Default | What it does |
| --- | --- | --- |
| Record | ZR + D-pad Down | Starts or stops recording the side the pop-up is set to. |
| Play on Player 1 | ZR + D-pad Left | Plays Player 1's active macro, or stops it if it is playing. |
| Play on CPU | ZR + D-pad Up | Plays the CPU's active macro, or stops it. This chord was Input Playback. |
| Recording pop-up | ZR + D-pad Right | Opens the pop-up. B closes it (the chord does nothing while it is open). |

Save and Load (ZL + D-pad Down and Up) are in [Save states](save-states.md). All chords are changed in Display &
Controls › Button chords ([Controls](controls.md)).

- **Play** gives the side one recorded frame per game frame, in order, and warps to the start save once when its switch
  is On. A macro recorded on one side plays on either. With no active macro, the chord does nothing.
- **Both Play chords can run at once:** Player 1 and the CPU play together. A chain's own macros play one after
  another.
- **Your chord buttons don't reach the game** while you hold them, so pressing Record or Play never makes Player 1
  shield.
- **Mash interrupts playback** (Recording › Options), when On, stops the CPU's macro the moment the CPU would mash,
  library macros and chains included. Player 1's macro keeps playing. See
  [Mash interrupts playback](../menu/recording.md#mash-interrupts-playback).

![Play on CPU warps once to the start save and plays the active macro; the same chord again stops it](../media/rc7-play-chord.gif)

*Play on CPU: one warp to the start save, then the macro; the same chord again stops it.*

![Both Play chords at once: Player 1 and the CPU play together, then one after the other](../media/rc6-both-play.gif)

*Both Play chords pressed together start the two jabs at once, with both lanes running side by side. Pressed one after
the other, the CPU's lane trails Player 1's.*

## The loop

The loop is how many times the Play chord plays the active macro. It belongs to the side, not to a file: Player 1 and
the CPU each keep their own count, from ∞, 1, 2, 3, 5 and 10, and a new library starts at 1. Set it with Recording ›
Options › [Loop](../menu/recording.md#loop), or with X in MACROS.

![Play warps once, before the first pass; later passes of a loop, and of a chain, play on from where the fighters are](../diagrams/recording-passes.png)

- **Only the first pass warps** to the start save (when its switch is On). The loop never warps back.
- **A chain repeats as a whole:** 1, 2, 3, then 1 again. ∞ keeps going until you press the Play chord again.
- **Reset page to defaults** (Recording › Options) puts both counts back to 1.

![The second pass of a loop continues from where the fighters are](../media/rc7-loop-no-warp.gif)

## Chains

A chain is a macro made of macros: an ordered list of your recordings, numbered 1, 2, 3. It can't hold another chain,
because the picker lists recordings only.

- **Make one:** + New chain, the last row under MACROS, opens the numbered picker
  ([how it works](../menu/recording.md#a-files-own-screen)). It is the Character Cycles picker, except that R can take
  off the last number, so a chain can end up empty. A on a tile that already has a number does nothing.
- **Where it starts:** at its first macro's start save. The chain has its own Start save switch, On by default. On
  warps there once when the Play chord starts the chain. Off plays from where the fighters are. The macros inside don't
  use their own start saves, whatever their switches say. If macro 1 has no start save, the chain has none.
- **How it plays:** the macros in order, from where the fighters are after that one warp. With a loop, the whole chain
  repeats and nothing warps back.
- **Nothing to play:** an empty chain, or one whose recordings were all deleted, plays nothing, with no extra message.

![The CPU's Play chord warps to macro 1's start save, then plays the chain in order](../media/rc7-chain-play.gif)

<!-- CAPTURE-REQUEST rec-chain-picker: the numbered picker for + New chain (numbers 1, 2, 3 on recordings, R removing one, then the name step). A GIF; see artifacts/docs4/CAPTURE-REQUESTS.md. -->

## Combos

### Getting a combo

- **+ Save last combo…** is the first row of COMBOS, on the Recording page and in the pop-up's Combos & Sequences tab.
  It keeps the last combo the frame strip saw and makes it the drill. A opens the keyboard for a name. An empty name,
  or a room where no keyboard can open, gives it the next Combo N. With no link on the strip yet it says so and saves
  nothing.
- **Make combo,** on a macro's or a chain's own screen, plays that file once on Player 1 from the file's own start
  save (a chain uses its first macro's) and keeps what it did as a new combo, Combo N, with Made from set to the
  source.
  - The file on screen stays, the drill doesn't change, and the new combo joins the five most recently used.
  - A pass with no hit and no follow-up saves nothing and says "No combo yet: land a hit and a follow-up first."
  - If the start save was deleted, Make combo skips the warp, shows "No start save. This macro plays from where the
    fighters are." and still plays. A switch that is Off, or a file that never had a start save, doesn't warp and shows
    nothing. A chain whose first macro is missing skips that step and plays the next one still there.
- **What a combo file holds:** its links, and the input events of each follow-up: presses, releases, the C-stick and
  stick flicks (fast-fall, dash, tap-jump). Make combo records the same events, because it plays into the same strip.
  Any fighter can practise a combo. The format is in [Combo files](combos.md).

![Save last combo: jab into jab, then COMBOS lists it and the helper bar drills it](../media/rc6-save-last-combo.gif)

*Jab into jab is a TRUE combo on the strip. COMBOS goes from (0) to (1) and lists "Jab jab saved, recorded on Mario".
The name is typed on Eden's keyboard, a window that isn't in the frame.*

![Make combo plays the macro and lists Combo 1, made from it; drilled and played again, the bar grades it TRUE](../media/rc6-make-combo.gif)

### How a combo is graded

With a combo or sequence as the drill, the combo helper's bar under the frame strip grades each attempt link by link.
A link **counts** when the recorded input events come, of the same kind and the same facing-relative direction, in
order and within the [On-time tolerance](../menu/recording.md#on-time-tolerance), and the move that comes out is the
recorded one.

![A link is graded on its inputs and its move; the examples show an on-time and a 1-frame-late press that count, a press 3 frames late and a missing press that drop, and what else drops or does not](../diagrams/recording-grading.png)

- **TRUE is not required.** A link that is not a true combo still counts. TRUE stays on the bar as information. Drift,
  the stick you hold between the events, is shown and never graded.
- **A follow-up's events end at its hit,** so a button you still hold when the hit lands is not graded for its release.
- **An older file with only presses** is graded on those presses, with left and right as one direction.
- **The exchange** ends 20 frames after the CPU can act again, or on a shielded hit. A gap of 20 frames still belongs
  to it. A gap of 21 opens a new one, so the step drops.
- **Best and streak:** a single combo's best is the most true links in one attempt. The streak counts the links that
  counted, so a link that is not true can still extend it.

![A drilled TRUE 0 link: one frame later reads +1 DROP and still counts; a different move drops](../media/rc5-grade-not-true-crop.gif)

*Cropped to the top of the screen.*

### Crop

Crop on a combo or a macro opens two handles, In and Out ([keys](../menu/recording.md#a-files-own-screen)).

- The cut is saved as a new file under the next unused Macro N or Combo N, and you go back to the original, which stays.
- A macro crop keeps that range of frames. The new macro keeps the original's one start save, stamped at the crop's
  start frame; no pose is stored for every frame.
- A combo crop keeps the links from the first handle through the last.
- A chain has no Crop row, because it has no frames of its own. The Replay window is a separate row under Replays.

![Crop on a two-link combo, Out 2 to 1, saves Combo 2; drilled, the bar shows its one link](../media/rc6-crop-combo.gif)

![Crop on the macro Walk and jab: In moves from 0 to 8, Out from 180 to 170, A saves a new macro](../media/rc13-crop-macro.gif)<br>
*Crop on a 180-frame macro. R moves In from 0 to 8, Down picks Out, L moves it from 180 to 170, A saves the cut as a new file: the count goes from 13 to 14 and the original is still listed. Each press is named in a box. Eden, rc.13 (untested on hardware).*

## Sequences and the one drill

A sequence is an ordered list of combos. It stores each combo's id, so renaming a combo shows the new name in the
sequence, and deleting a combo makes the sequence skip that step. An empty sequence drills nothing, and a sequence of
any length drills every combo in it.

- **One drill at a time.** Drilling a sequence replaces a drilled combo, and drilling a combo replaces a drilled
  sequence. There is no chord: use Drill on the file's own screen, or A on it in the pop-up.
- **A numbered row inside a sequence** drills that combo alone, with that combo's own streak. The sequence file stays.
- **Editing:** the picker is the one a chain uses, including R removing the last number. Edit combos, Rename and Delete
  are written to the card at once.

![Drill a sequence: combo 1, then combo 2, then a drop back at combo 1](../media/rc7-sequence-drill.gif)

### The sequence bar

Drilling a sequence adds a header and a row of its combos to the helper's bar.

- **Header:** SEQUENCE, the sequence's name, "combo k of n", the current combo's name, its links and the tolerance;
  the streak and the best on the right.
- **Row:** the sequence's combos in order, joined by arrows: done is green with a check, the current one is white,
  later ones are grey.
- **Under the row:** the current combo's own link cells. The footer names the combo that just landed and the current
  one.

![The sequence bar mid-drill: the strip above, then SEQUENCE, Jab pressure, combo 2 of 3, streak 0 and best 2; combo 1 green with a check, combo 2 white, combo 3 grey, the current combo's link cell and the footer](../media/rc13-sequence-bar.png)<br>
*The sequence bar mid-drill: combo 1 done (green, with a check), combo 2 current (white), combo 3 later (grey), with the streak and the best. Eden, rc.13 (untested on hardware).*

### Passing and dropping

- **Combo 2 follows inside the same exchange as combo 1.** The link from combo 1's last hit to combo 2's first hit
  joins them: it is shown and not graded, and the move it lands must be combo 2's first move. The CPU may act for up
  to 20 frames between the combos, and TRUE is not required there either.
- **Each step** is graded like a single combo: the events, the direction, the order, the tolerance and the move.
- **A finished sequence** adds one to the streak, and the best is the highest streak.
- **A drop** sends the sequence back to combo 1 and the streak to 0 (the best is kept). A step drops on any fault a
  single combo drops on, on a different first move after the joining link, or when the exchange ends before the next
  combo. A save-state load or a scene change in the middle of a sequence is a drop too.
- **After a drop** the dropped combo is red and the footer names the fault, until combo 1 starts again in a new
  exchange.
- **Saved with the file:** the streak and the best are written to the sequence file, so CONNECTIONS shows the same
  numbers as the bar, after the menu closes and after you leave Training.

## The pop-up

The pop-up is one window over the match, so you can work with your files without leaving it. Its layout and keys are
on the [Recording page's pop-up section](../menu/recording.md#the-recording-pop-up).

- While it is open it takes your buttons. B closes it and leaves the bound save, the active macros and the cursor as
  they were. That B never reaches your fighter, tapped or held; your buttons stay with the pop-up until you let go.
- L Player and CPU R choose the side you are working on. That side is the one Record records and Edit placement edits.
- A acts at once: no file screens. Renaming, cropping and deleting are on the page.

![The pop-up over the match, Player view, cursor on a save](../media/rc5-popup-trim.gif)

*Cropped to the pop-up.*

### Reading the lists

| You see | It means |
| --- | --- |
| A green row | The bound save, the active macro of the side in view, or the drilled combo or sequence. A row can be green and hold the cursor too. |
| A green outline, a thin line under it | The hot five: the five files of a kind you used last, newest first. |
| An orange floppy on a macro | The save it starts at; grey when that macro's Start save switch is Off. The Start save row of its own screen shows the same floppy, and the SAVE STATES header too. |
| A number on a chain's row | How many macros it holds. With the cursor on a chain, they show 1, 2, 3 before their names. |
| A red number badge (pop-up) | A row's place in the chain or sequence under the cursor. The page writes the number before the name. |
| A square before a header (pop-up) | The kind: blue MACROS, purple COMBOS, pink SEQUENCES. Each header shows how many files it lists. |
| A white border and a bar on the left | The cursor. |

CONNECTIONS, the right-hand column in the pop-up and two help lines on the page, shows what the row under the cursor is
joined to. Each file sits in a box labelled SAVE STATE, MACRO, COMBO or SEQUENCE, under a heading:

| Heading | Shows |
| --- | --- |
| start save of | The macros that start from this save. |
| starts from (warps there once, first) | A macro's start save. |
| in chains | The chains a macro is in. |
| combos made from it, made from | A macro's combos, and the macro a combo came from. |
| in sequences, combos | The sequences a combo is in, and a sequence's combos. |

Plain lines follow: a combo's links and fighters, whether a save is bound, and a macro's recorded fighter and length.
A row's connections are not limited to the side in view.

## A file's own screen

On the Recording page, A on a file opens its own screen. The rows each kind of file has are in
[the menu reference](../menu/recording.md#a-files-own-screen). The rules behind them:

- **Set active** is one row, for the side in view: Right sets On and Left sets Off. Left on a row that is already Off
  doesn't clear the other side.
- **Names:** a name may be at most 48 characters, and it may not be empty or hold a control character.
  - Rename opens the Switch keyboard. B, or an empty confirm, keeps the old name.
  - A new chain or sequence with an empty name is Chain N or Sequence N. A recording stopped with no typed name is
    Macro N, a duplicated save is Save N, and a combo saved with no typed name is Combo N.
  - In an online waiting room the keyboard stays closed, and B on the name step keeps that N name. A local-wireless
    waiting room and Training open the keyboard.
- **The breadcrumb** reads "Recording (Player view) > Save states & Macros > name", or Combos & Sequences for a combo or
  a sequence. The CPU view says "Recording (CPU view)".
- **A combo's help lines** list each link as the two moves and TRUE or not, then its input events, then its drift when
  it has one. The word sequence means the file of combos, and the word chain is only the compound macro.

![A macro's own screen: Set active On for Player 1, then Play on Player 1 warps to its start save and plays it](../media/rc6-macro-set-active.gif)

![A macro's own screen: Start save, Set active, Playback slot, Rename, Crop, Make combo, Delete](../media/rc13-file-screen-macro.png)<br>
*A macro (Walk and jab, with a start save): Start save reads "Jab start · On" beside the orange floppy. Eden, rc.13 (untested on hardware).*

![A chain's own screen: Start save, Set active, Edit chain, Rename, Make combo, Delete](../media/rc13-file-screen-chain.png)<br>
*A chain (Chain 1, P1 jab then Walk and jab): its Start save is the first macro's. Eden, rc.13 (untested on hardware).*

![A combo's own screen: Drill, Made from, Rename, Crop, Delete](../media/rc13-file-screen-combo.png)<br>
*A combo (Jab jab, made from the macro P1 jab). Eden, rc.13 (untested on hardware).*

![A save state's own screen: Bind to Save / Load, Duplicate, Rename, Delete](../media/rc13-file-screen-save.png)<br>
*A save state (Jab start, bound: Bind to Save / Load reads On). Eden, rc.13 (untested on hardware).*

## Replays: Edit placement and the Replay window

**Edit placement** moves where a macro starts. Play warps the fighters to the macro's start save, and the offset moves
the side that plays it.

![Play warps both fighters to the start save; with an offset only the side that plays moves, and x- x+ and X- X+ set it](../diagrams/recording-placement.png)

- **Which macro:** the active macro of the side the pop-up is set to (its lit tab, not the page's ZR view), the file
  that side's Play chord plays. For a chain it is macro 1, whose start save the chain uses.
- **The anchor** is where that macro's start save puts the side. The offset is added on the next Play, to the playing
  side only. The keys, Write, Keep, New and Back are in the
  [menu reference](../menu/recording.md#edit-placement).
- **Momentum:** "Keep recorded speed", the default, gives the side the speed it had when Record started, on its first
  input after the warp. "Start at rest" starts it from standstill. Like the offset, it applies with the warp to the
  start save.

**The Replay window** lists your saved match captures. A opens the clip editor on one: the fighter, the crop window,
the playhead and the timeline. On a capture's Mii track it reads the recorded Mii, asks Fighter Mismatch, and Switch
loads that exact Mii on the CPU. Its keys are in the [menu reference](../menu/recording.md#replay-window).

<!-- CAPTURE-REQUEST rec-replay-editor: the Replay window's clip editor, a still with the fighter, the crop window, the playhead and the timeline visible (rc.13). See artifacts/docs4/CAPTURE-REQUESTS.md. -->

## Fighter Mismatch

Fighter Mismatch is the question a macro, a chain, a replay clip or a combo drill can ask before it switches a fighter.

![Play, a replay clip or a combo drill meets a different fighter: the question appears only when the recorded fighter differs and Ask before switching fighter is On](../diagrams/recording-mismatch.png)

- **The prompt:** its first line names the recorded fighter and the side that plays it ("current Player 1" or "current
  CPU"). Keep current fighter plays on the fighter you have. Switch to recorded fighter changes to the one the file
  was recorded on, then plays. B cancels and doesn't play. The cursor starts on Keep.
- **Play waits** until it is answered. With the menu closed, the prompt is drawn over the match, the frame strip is
  hidden, and Player 1's presses answer it instead of reaching the match: the D-pad moves, A chooses, B cancels.
- **Ask before switching fighter Off** (Recording › Replays): nothing asks, and the fighter you have stays, as Keep
  would leave it.
- **Nothing is refused** because the fighter differs. A sequence may mix fighters.

| It asks | When |
| --- | --- |
| A macro | Its recorded fighter differs. For a CPU macro recorded on an exact Mii, Switch loads that Mii: its look, specials, colour, voice, outfit and hat. |
| A chain | Only for its first macro. Later macros aren't checked and don't switch. |
| A combo drill | The drilled combo changes to one recorded on another fighter. The first time a saved drill is seen, it doesn't ask. |
| A replay clip | Opening a capture in the Replay window, when the CPU isn't the recorded fighter. |
| Load | Never. |

A Mii from a replay carries no ID and no name of its own, so it is installed as the saved Mii with the same look, or
else with a made-up ID and the name "New Mii".

![Play of a chain whose first macro was recorded on another fighter](../media/rc7-fighter-mismatch.gif)

![A CPU macro recorded on a Mii: Switch loads that exact Mii](../media/rc6-named-mii.gif)

*A CPU macro recorded on a Mii Gunner, played with the CPU on Mario. Switch to recorded fighter reloads the CPU, and
"Saved Mii applied" brings in the recorded Mii Gunner (red armour, spiked helmet).*

![A combo recorded on Luigi, drilled with Player 1 on Mario: Switch makes Player 1 Luigi](../media/rc7-mismatch-player1.gif)

*With Player Swapping Off, Fighter Mismatch reads "Recorded Luigi · current Player 1 Mario"; Switch to recorded
fighter, and Player 1 becomes Luigi ("Player fighter applied").*

## Migration of the old slots

The first time this library is built:

- **Recording slots** that hold inputs become macros named `Slot N (YYYY-MM-DD)`, with the date the migration runs.
  Empty slots are skipped. These macros have no start save, so they have no switch and play from where the fighters
  are. No chain is built from them. Recording slots were never kept between boots, so on a card updated from an earlier
  build they are empty and no slot becomes a macro.
- **Save indexes** that hold a save become one file, both fighters, with the same `Slot N (date)` name. The slot the
  Save and Load chords were using becomes the bound save. An empty index is skipped.
- **`macros/selected.txt`**, when it names a file that is there, becomes Player 1's active macro. The CPU has none
  until you set one. The old "play a recording when a state loads" row is not turned into a chain.
- **The loop:** an old Loop On becomes ∞, and Loop Off, or a missing loop row, becomes 1.
- **The page:** the old Recording and Saves rows are gone from the Recording page: the slot number, Randomize slots,
  Record Slot, Trigger, the Duration dropdown, button slots, the save-state playback checklist, Source and per-slot
  playback. Their settings ids stay, so an old settings file still loads. Respawn on platform stays under Fighters &
  Stages › Stages.

![The first boot of a card from before the library: two dated saves with Slot 2 bound, and Old jab active](../media/rc5-migration-crop.gif)

*Cropped to the top of the screen.*
