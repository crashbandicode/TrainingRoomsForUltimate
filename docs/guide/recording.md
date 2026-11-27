# Record, play and drill

## Make a take and play it back

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/35e8edb3e156f65dc2b9cc87b450a10d83e2f8d9" autoplay muted loop playsinline controls></video>

*Record a forward smash, then stop the take.* [Open the clip](../media/tut-record-macro.mp4)

| Goal | What to do |
|---|---|
| <a id='recording-a-macro-and-its-start-save'></a>Record | Open the Recording pop-up, choose Player or CPU with ZR, then close with B. |
| Make the take | Press Record, perform the moves, press Record again. [Find your chords](../menu/display-and-controls.md#button-chords). |
| <a id='set-active-and-the-play-chords'></a>Play | Select the macro for the intended role; press its Play chord. Press again to stop. |
| <a id='reading-the-lists'></a>Read the list | Green marks active files, white the cursor, a floppy the linked start save. |
| <a id='save-states-only-warp'></a>Start elsewhere | Set **Start save** Off to play from here; [Load](save-states.md) alone never plays inputs. |
| <a id='change-placement-and-the-replay-window'></a>Change the starting positions | Open **Change placement** on the save; choose spots, states and percents, then confirm Preview. |
| Edit a shared save | Choose copy for a separate start or change the save for all linked macros. |
| <a id='chains'></a>Play several takes | Choose **+ New chain**; A numbers macros, R removes, X finishes, B cancels. |
| <a id='the-loop'></a>Repeat | Set [Loop](../menu/recording.md#loop) for the role; only the first pass warps. |
| <a id='fighter-mismatch'></a>Fighter Mismatch | Keep your fighter, switch to the recorded fighter, or cancel with B. |
| <a id='the-pop-up'></a><a id='a-file-s-own-screen'></a>Manage a file | A in the pop-up acts immediately; A on the Recording page opens the file. |

## Practise a combo or input drill

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/b635893c4df02daaae5535f19b5b3f5c2346245b" autoplay muted loop playsinline controls></video>

*Kirby's input drill appears in the library.* [Open the clip](../media/tut-input-drill-record.mp4)

| Goal | What to do |
|---|---|
| <a id='practise-a-combo-or-cut-a-take'></a><a id='combos'></a><a id='getting-a-combo'></a>Drill hits you just landed | Choose **Combos & Sequences → + Save last combo…**, name it and try again. |
| Drill an existing file | In the Player view, select the combo and choose **Drill**. |
| Turn a take into a drill | **Make combo** replays it; linked hits make a combo, no contact an input drill. |
| <a id='how-a-combo-is-graded'></a>Read the grade | Match the move and timed inputs; the [strip's TRUE verdict](frame-strip.md#link) isn't required. |
| <a id='input-drills'></a>Practise movement without a hit | Choose **DRILLS → + Record drill…**; use Record to start and stop your inputs. |
| Preview an input drill | X on its row plays its available take on Player 1; B stops it. |
| <a id='sequences-and-the-one-drill'></a>Chain hit combos into a drill | **+ New sequence** numbers hit combos; a drop takes you back to the first. |
| Share a combo | [Copy its JSON file](combos.md) without sharing the rest of your library. |

## Crop

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/4a1682c8e3a70a876ed2764fc4e6629bbd6cfcdc" autoplay muted loop playsinline controls></video>

*Browse a take before setting its crop handles.* [Open the clip](../media/tut-clip-playhead.mp4)

| Goal | What to do |
|---|---|
| Open the editor | Choose **Crop** on a file, or select a capture from **Replay window**. |
| Delete a match capture | Press Y on the highlighted capture **in the list**. No confirmation; exported clips stay. |
| Choose whose inputs to keep | On a two-sided capture, D-pad Up/Down picks the recorded fighter. |
| Change the playback target | Tap ZR alone to switch between Player 1 and CPU; this stops a running preview. |
| Browse a macro or capture | With neither handle selected, Left/Right moves the playhead; Y seeks the nearest move. |
| Cut at the playhead | After browsing, L makes this the first kept frame; R makes it the last. |
| Adjust a cut | L selects In; R selects Out. Left/Right moves it; the same shoulder deselects it. |
| Scrub faster | Hold Left/Right to accelerate; add ZL or ZR to start fast (GameCube: Z). |
| Snap selected handles | Y snaps In toward move starts or zero; Out includes move ends and take end. |
| If no move end is known | Out uses the next start or the end of the take. |
| Include a captured move's setup | Hold ZL or ZR (GameCube: Z), press Y, then release the modifier before previewing. |
| Compare two Vault captures | From Latest, ZR + X swaps captures if Previous exists; this resets the cut. |
| Preview without saving | X previews a macro or capture; X or B stops playback or its tail. |
| Leave without saving | B closes the idle editor. |
| Show a take's moves on its fighter | In the editor, ZL+L switches Player 1 and ZL+R the CPU to the take's fighter. |
| X refuses a mid-action capture | Select In and press Y; [if X still refuses](../CURRENT-RELEASE.md#known-issues), deselect In with L. |
| Keep the result | A names a new file; the original stays. Combos crop by link, drills by step. |
