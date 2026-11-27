# Recording

Press R twice from the CPU page for your recordings, your combos and their playback options ([Recording guide](../guide/recording.md)).

## The page and its pop-up

![The same saves and macros in both views, with RECORDINGS on the page only](../diagrams/recording-page-popup.png)<br>
*The page also opens match recordings.*

Use your [Recording pop-up chord](display-and-controls.md#button-chords) to open the library over practice.

| Key | On the page | In the pop-up |
| --- | --- | --- |
| <a id="the-recording-pop-up"></a>A | Opens the file's own screen. | Acts at once: binds a save, sets a macro active, picks the drill. |
| X | In MACROS, cycles Loop. On an input drill with a take, previews it. | Cycles Loop in MACROS, opens placement on a save, or previews a drill's take. |
| ZR | Switches between the Player view and the CPU view. | Switches the side that Record records. |
| B | Backs out of a file or sublist before closing the menu. | Backs out of DRILLS first; closing the pop-up doesn't reach your fighter. |

## Save states & Macros

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/35e8edb3e156f65dc2b9cc87b450a10d83e2f8d9" autoplay muted loop playsinline controls></video>

*Record tints Player 1 red.* [Open the clip](../media/tut-record-macro.mp4)

A on a save state, macro or chain opens its own screen ([page and pop-up controls](../guide/recording.md#a-file-s-own-screen)).

| Setting | Values (default in bold) | What it does |
| --- | --- | --- |
| <a id="recordings"></a>RECORDINGS > | Vault and saved captures | Play one back, or make a macro, a combo or a drill from it. |
| <a id="a-file-s-own-screen"></a>Bind to Save / Load | On, Off | Save writes here; Load uses it unless a [named setup](saves.md#setup-library) is selected. |
| Start save | On, Off | On: warps both fighters to the macro's start save once, before it plays. |
| Momentum | **Keep recorded speed**, Start at rest | The speed the playing side has on its first input. |
| Offset | Only on macros with an old offset | A clears the offset from the start position. |
| Set active | On, Off | On: this view's Play chord plays this macro. |
| <a id="playback-slot"></a>Playback slot | None, Slot 1 to 5 | When CPU picks this slot, it plays its macro (600 frames maximum). <video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/b2aab5ee61a4097dd38ab7fdc1d1ef28df4fd864" autoplay muted loop playsinline controls></video> [Open the clip](../media/rc14-playback-slot.mp4) |
| Trim | Start, End, Both, Off | Skips neutral frames at those sides in playback. The file is unchanged. |
| Crop | Opens the clip editor | Saves a shorter copy; the original stays ([Crop](../guide/recording.md#crop)). |
| Make combo | Plays once on Player 1 | [What the take can save](../guide/recording.md#practise-a-combo-or-input-drill). |
| + New chain, Edit chain | Opens the numbered picker | [Put macros in playback order](../guide/recording.md#chains). |
| Duplicate, Rename, Delete | | Delete removes the file at once, without asking. <video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/c03fa6ac1f34b176b612ebf960b84b8f4ded74b2" autoplay muted loop playsinline controls></video> [Open the clip](../media/rc13-save-duplicate-delete.mp4) |

## Combos & Sequences

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/b635893c4df02daaae5535f19b5b3f5c2346245b" autoplay muted loop playsinline controls></video>

*The recorded drill appears in the list.* [Open the clip](../media/tut-input-drill-record.mp4)

<a id="combos"></a>

Player view only: the combos, input drills and sequences that the Combo helper grades ([Combos](../guide/recording.md#combos)).

| Setting | Values | What it does |
| --- | --- | --- |
| + Save last combo… | | Keeps the last combo the frame strip saw and makes it the drill. |
| <a id="input-drills"></a>DRILLS > + Record drill… | | Arms a Player 1 take that needs no hit. Record starts and stops it. |
| + New sequence | Opens the numbered picker | A sequence is combos drilled one after another. |
| Drill, Drill sequence | A selects | Makes this the file the helper grades, replacing the previous choice. |
| Crop | Opens the clip editor | Whole links for a combo, whole steps for a drill. <video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/b01d3788d21072c6f728edd38e2ba35aa3a89f94" autoplay muted loop playsinline controls></video> [Open the clip](../media/rc15-input-drill.mp4) |
| X on an input drill | A recorded take is required | Previews the inputs on Player 1. B or X stops it. |

## Replays

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/23b4e00d007427260ccff3ee35d14c0f976f2e50" autoplay muted loop playsinline controls></video>

*Scrubbing shows each fighter's move.* [Open the clip](../media/tut-clip-scrub-moves.mp4)

[Edit a match capture](../guide/recording.md#crop) or [move a start save's fighters](../guide/recording.md#change-placement-and-the-replay-window).

| Setting | Values (default in bold) | What it does |
| --- | --- | --- |
| <a id="ask-before-switching-fighter"></a>Ask before switching fighter | **On**, Off | On: a file recorded on another fighter asks first. Off: your fighter stays. |
| <a id="change-placement"></a>Change placement | Opens the placement picker | Places both fighters of the active macro's start save. |
| <a id="replay-window"></a>Replay window | Opens the capture list | Choose a capture, then select a fighter and input range in the editor. [Clip](../media/tut-clip-snap.mp4) |
| <a id="replay-capture"></a>Replay capture | On, Off | On: saving a replay also captures its inputs. The choice is saved automatically. |
| <a id="fighter-toggle"></a>ZL+L, ZL+R in the clip editor | Player 1, CPU | Switch recorded/original fighter; cancel only while the footer says **B cancels**. [Player 1](../media/tut-toggle-p1.mp4), [CPU](../media/tut-toggle-cpu.mp4) |
| <a id="moves-while-you-scrub-preview-build"></a>Scrubbing in the clip editor | Positions | The opening In frame lines up with the playback target's current spot. |
| Scrubbing, pose | Matching fighter and stored pose | Shows the recorded pose; otherwise the editor shows position only. [Clip](../media/tut-clip-scrub-home.mp4) |

![The clip editor's strip with twelve numbered labels](../diagrams/clip-editor-legend.svg)<br>
*The clip editor's strip, labelled.* [Open the full-size legend](../diagrams/clip-editor-legend.svg)

## Options

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/290448eb4f0df99be8dd578410f0ed32594febce" autoplay muted loop playsinline controls></video>
*Loop at 3 plays the macro three times.* [Open the clip](../media/rc12-loop.mp4)

How a macro plays back, and how the Combo helper grades.

| Setting | Values (default in bold) | What it does |
| --- | --- | --- |
| <a id="trim-trailing-neutral"></a>Trim trailing neutral | Start, **End**, Both, Off | Removes idle frames from those sides of a new take. |
| <a id="place-fighters-before-record"></a>Place fighters before Record | On, **Off** | On: Record first opens the placement picker. |
| <a id="hitstun-timing"></a>Hitstun timing | **As Hitstun Ends**, As Hitstop Ends, As Hitstop Begins | When playback starts after the CPU is hit. <video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/90c372d1da1d4a97f6403f94bbcfc985678aceec" autoplay muted loop playsinline controls></video> [Open the clip](../media/rc12-hitstun-timing.mp4) |
| <a id="mash-interrupts-playback"></a>Mash interrupts playback | **On**, Off | On: Mash stops CPU playback, except during [Parry practice](../guide/parry-practice.md). <video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/2af5cb699f793423755a53b4f52cef141cc6d29c" autoplay muted loop playsinline controls></video> [Open the clip](../media/rc11-mash-interrupts.mp4) |
| <a id="loop"></a>Loop | ∞, **1**, 2, 3, 5, 10 | How many times the active macro plays. X cycles it in MACROS. |
| <a id="settings-override-playback"></a>Settings override playback | On, **Off** | Off: a playing recording wins over the DI, SDI, air-dodge, tech and throw settings. |
| <a id="shared-start-save"></a>Shared start save | **Prompt**, Always change, Always copy | Applies when at least one macro starts from the save you're editing. |
| <a id="combo-helper"></a>Combo helper | **On**, Off | The card shows the result; cells under the strip grade individual links. |
| <a id="match-sweet-sour-spot"></a>Match sweet/sour spot | On, **Off** | On: a link counts only if it lands the saved spot. |
| <a id="match-angle-and-deflection"></a>Match angle and deflection | On, **Off** | On: an aimed special must match the saved stick angle and deflection. |
| <a id="move-names"></a>Move names | On, **Off** | On: moves show their names. Off: the short names. |
| <a id="on-time-tolerance"></a>On-time tolerance | Exact, **1 frame**, 2 frames | How far from the recorded press still counts as on time. |
| <a id="after-an-attempt"></a>After an attempt | **Keep playing**, Load save state | Keep playing, or load the save state to retry from the same spot. |
| <a id="show-playback-indicator"></a>Show playback indicator | **On**, Off | On: shows which role is playing a recording. |
| <a id="playback-delay"></a>Playback delay | **None**, 10, 20, 30, 45, 60, 90, 120 frames | Frames to wait after the fighters are placed, before the first input. |

## Reset page to defaults

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/86dc476dd4a6065467ff4b1fba402925b0af2ea1" autoplay muted loop playsinline controls></video>
*A opens the reset question; A again confirms.* [Open the clip](../media/rc13-reset-page-recording.mp4)

| Setting | Values | What it does |
| --- | --- | --- |
| Reset page to defaults | A, then confirm | Restores saved settings and resets both Loop counts to 1; Replay capture stays unchanged. |

## Good to know

![The Recording page, Player view: the hot five outlined, the bound save and the active macro green, orange floppies, CONNECTIONS filled](../media/rc13-recording-page.png)<br>
*Green marks the bound save and the active macro.*

| If | Then |
| --- | --- |
| You press X or Y in the main lists | Neither changes defaults. The [capture list](../guide/recording.md#crop) has its own delete key. |
| A file was recorded on another fighter | Play asks first, unless Ask before switching fighter is Off. |
| You reset this page | Your files, the bound save, the active macros and the drill stay. |
| The take has no pose for this frame | Switching fighters cannot show a pose the take did not store. |
