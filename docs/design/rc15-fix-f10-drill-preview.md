# F10 design: preview an input drill through the Recording player

The [Recording menu](../menu/recording.md) describes the shipped drill preview.

## 1. Before F10 (observations on F7’s head)

| Fact | Where |
| --- | --- |
| Input drills stored events with directions and offsets, without input frames. | `model/combo.rs` (`InputCapture`), `model/combo_record.rs` (`kind: input_drill`) |
| Event types were press, release, C-stick and three flicks. | `model/combo.rs` |
| `+ Record drill…` saved P1 events as Drill N and discarded captured frames. | `model/rr_host.rs`, `service_at`, record edge 2 |
| Make combo kept the source macro id in `made_from`. | `rr_runtime::save_made_combo` |
| The player's Play queue read a card macro’s frame range for one role. | `rr_host::request_preview(source, role, start, end)` → `Playback::play_range` |
| Stop dropped remaining frames. The player exposed whether preview still ran. | `stop_preview()`, `preview_running()` (F7) |
| P1 playback filled the mapped record after the menu neutralized real input. | `runtime/features/input_delay.rs`, `recorder::on_p1_poll` |
| Make combo already played P1 under the menu. | `recorder::on_p1_poll` |
| X cycled the MACROS loop and was free on combo and drill rows. | `recording_menu.rs`, `handle_page`, `press_popup_outcome` |
| Crop used X to start, X/B to stop, and ZR to stop and switch targets. | `replay_capture.rs`, `on_input` |
| The editor stayed open for the 60-frame tail after playback. | `clip_panel.rs`, `PreviewTail`, `hides_page` |

So a drill has nothing the player could play unless its take's frames are kept where the player reads, and the player reads macro files.

## 2. Shipped design

| Stage | Data |
| --- | --- |
| Record | Store the event drill and its owned frame take. |
| Select | Resolve the nonempty take on the focused drill. |
| Preview | Play the trimmed frame range once on Player 1. |

### 2.1 A recorded drill owns its take

When `+ Record drill…` stops and the drill is saved, the take's frames are saved as a macro file the drill owns, and the drill file names it in a new field, `take`.

- **Hidden:** a take is named `Drill take` followed by a 16-digit key from its frames and save time.
- Its id comes from its name, so a drill copied from another card names no take here, says `Re-record to preview`, and deletes nothing.
- Only a macro with such a name that a drill names counts as a take; the menu removes it from its catalog.
- **Deleted with the drill:** deleting the drill deletes its take unless another drill file still names it. A file the library skips is not a drill and keeps no take. Whether the take goes is settled before the drill file is touched; when the folder cannot be read, nothing is deleted.
- **The frames** are the ones an ordinary Record of Player 1 would have saved, with the recording trim applied (Start, End, Both or Off). No start save is written; the take is not an active macro and is not marked as used. The drill, the grading target, the Record role and `made_from` (empty for a direct drill) behave as before.
- An overfull or empty take saves neither file, as before. A crop of a drill has other inputs, so the cropped drill has no take.

### 2.2 The preview

- **Which rows:** a drill row (DRILLS, page and pop-up) that has its own take with at least one frame. Nothing else is ever played: `made_from` is not used, because a crop of a made drill keeps `made_from` and would play the uncropped source, which is not the drill's inputs.
- **A drill without a take** (recorded before this build, a crop, or one Make combo made) shows `Re-record to preview` as the first line of its CONNECTIONS text, and X does nothing on it. A made drill's source macro can still be played as a macro.
- **Button:** X, the Crop and Replay preview's button. X in MACROS still cycles the loop; context decides.
- **What plays:** `rr_host::request_drill_preview(take id, frames, trim)` queues one preview on Player 1 through the Play queue, using the drill’s trim setting to choose the frame range, from where the fighters stand (a take has no start save). The shared player supplies the input frames.
- **While it plays:** the menu (or the pop-up) stays open and keeps Player 1's pad, and is not drawn. The strip's key line reads `B stop preview`.
- Only X and B act: they stop the preview, and the page returns once the player has dropped the rest of the take (the same worker pass in practice), so no frame of the take is handed to Player 1 behind a shown page.
- After the last frame the result stays on screen for about a second and the key line reads `B back`; B or X returns to the page or pop-up at once, or it returns after the tail.
- **Not started** (nothing happens) while the shared preview gate says no: practice not ready, a load in flight, a take open, a Play queued or running on Player 1 (it is not replaced), or another preview running.
- **Hint:** the footer of a previewable drill row names it. Page: `A open   X preview   ZR role   L/R page   B close`. Pop-up: `A drill   X preview   Left/Right section   ZL/L/R tab   ZR role   ZL+Up/Down page   B close`.
- **Bridge:** one line added to `tr.menu/1`, `recording.preview` = `drill` while the preview hides the menu, else empty.
- **Training and the waiting room:** the same code; nothing is scene-specific.
- **One writer:** `state::PREVIEW_HIDES` is set and cleared by the menu worker only. A lost renderer drops the menu's ownership; the worker's next pass sees neither the menu nor the pop-up open, stops the player and clears the flag.

### 2.3 Implementation map

| Piece | File | Owner note |
| --- | --- | --- |
| the drill file's `take` field | `model/combo_record.rs` | Recording UI |
| `save_drill_take`, `save_drill_with_take`, `delete_combo`, the catalog's `take` | `model/rr_runtime.rs` | no player code |
| `Effect::PreviewDrill { source, frames, trim }`, both faces, hints and take filtering | `model/recording_menu.rs` | Recording UI |
| `DrillPreview` (only X and B act; the 60-frame tail, reusing `PreviewTail`) | `model/macro_ui/drill_preview.rs` (new) | Recording UI |
| start, hide, press routing, return; the bridge line | `runtime/ui/worker.rs`, `runtime/ui/state.rs`, `runtime/ui/tick.rs`, `model/bridge_state.rs` | Recording UI |
| Save/delete the owned take, dispatch its effect and guard preview | `model/rr_host.rs`: `save_input`, `keep_drill_take`, `delete_file`, `apply_menu` and the preview guards | Playback integration: calls the existing player |
| the shared gate becomes `pub(crate)` | `runtime/macros/replay_capture.rs` `capture_ready` | Playback integration: one word |

The original F10 patch left these files and paths alone: `playback.rs`, `recording.rs`, and in `rr_host.rs` everything but the places named.

## 3. Compatibility and playback limits

| Drill source | List-row preview |
| --- | --- |
| Direct recording with owned take | Available. |
| Older event-only drill, cropped drill or Make combo drill | Re-record to preview. |

- **Old recorded drills** have no frames anywhere. Building inputs from their events would not be the recorded inputs (the stick between events was never stored), so they say `Re-record to preview`.
- **A drill Make combo made** has no preview in this build. Its source macro holds its inputs, but the file cannot tell a made drill from a crop of one, and keeping a take at Make combo's save is inside the player's code (`finish_make`), which this item does not edit.
- **A take left without its drill** (the drill file removed by hand on the card) shows up under MACROS with its keyed take name; nothing cleans it up.
- **A drill save that fails** before the drill file is written deletes the take it just wrote. Once the drill file is written both stay, also when the index update then fails; the drill is then not made the active drill.
- **After X or B** the page returns when the player reports no run left.
- A preview plays from the files read at its start, so a card that stops reading while it plays does not end it, and a Stop drops its queued frames without reading the card (changed 2026-11-25 with the held playback files).
- If the card's recordings cannot be read when a preview starts, or when the mod writes the library while one plays, the player ends the preview's run and drops its queued frames as after a Stop, and nothing resumes when the card reads again.
- One limit stays in the player's code: a frame the pad hook is taking at the instant the queue is dropped is still handed over, within the game frame of the stop.
- **A drill's own screen and the Crop editor** get no preview key: the key is on the list rows.
- **Preview strip:** the menu keeps the pad and hides its library. The strip stays visible with the stop or back hint.

## 4. Tests (host)

| Test surface | What the assertions cover |
| --- | --- |
| Library | Ownership, shared takes, deletion and failed writes. |
| Player | Role, queued frames, stop and tail. |
| Interface | Both faces, hints, hidden state and input ownership. |

- `a_direct_drill_keeps_its_take_as_a_macro_without_a_save_or_role_change` (`rr_host`, replaces `direct_drill_shares_record_without_macro_or_role_change`): one keyed take macro with the take's frames, the drill's `take` is its id and its `made_from` stays empty, no save state, no active macro, the Record role unchanged. Red on F7's head.
- `deleting_a_drill_deletes_its_take` (`rr_host`): the Delete effect on that drill leaves no macro file.
- `a_failed_drill_save_deletes_its_take` (`rr_runtime`): a failed drill save removes the take it just wrote.
- `an_index_failure_after_the_drill_file_keeps_the_pair` (`rr_runtime`): a later index failure leaves the drill and its take intact.
- `a_drill_file_from_another_card_neither_plays_nor_deletes_the_local_take` (`rr_runtime`): a copied drill cannot name or delete a different local take.
- `a_take_two_drill_files_name_survives_the_first_delete` (`rr_runtime`): a shared take remains until its last drill is deleted.
- `a_skipped_combo_file_does_not_keep_a_deleted_drills_take` (`rr_runtime`): a skipped file does not keep a take.
- `a_combo_folder_that_cannot_be_read_refuses_the_delete_and_keeps_the_pair` (`rr_runtime`): an unreadable folder leaves both files intact.
- `x_during_a_play_on_player_one_starts_no_drill_preview` (`rr_host`): a queued or running Play keeps its inputs and is not replaced.
- `a_takes_name_is_the_prefix_and_sixteen_hex_digits` (`combo_record`): names require the prefix and exactly 16 lowercase hex digits.
- `a_recorded_drills_take_previews_on_player_one_through_the_play_queue` (`rr_host`): `request_preview` of the take feeds Player 1's queue exactly the take's frames, and nothing to the CPU's.
- `a_stop_while_the_index_is_unreadable_retires_the_preview_run` (`rr_host`): with the index unreadable a held preview plays on past its queued frames without a read; a Stop drops the queued frames without a read; a library write by the mod makes the next pass read the index, and that failure drops the queued frames and does not resume the run.
- `a_drills_take_is_kept_by_a_rename_and_dropped_by_a_crop` (`combo_record`): the `take` field through the file, a rename and a crop.
- `x_previews_a_drill_that_has_its_take_on_both_faces` (`recording_menu`): both faces hide takes from macro lists and pickers. A valid take supplies its id and frame count; the test checks both hints. Missing takes and drills with only `made_from` show `Re-record to preview` and produce no effect. X in MACROS still cycles the loop.
- `a_cropped_made_drill_has_no_preview` (`recording_menu` or where the crop path's test lives): a crop of a drill made from a macro keeps `made_from`, shows `Re-record to preview` and X does nothing.
- `only_x_and_b_stop_a_drill_preview_and_the_page_returns_after_the_tail` (`drill_preview`).
- `after_x_or_b_the_page_waits_for_the_player_to_stop_and_skips_the_tail` (`drill_preview`).
- `the_snapshot_says_a_drill_preview_hides_the_menu` (`bridge_state`).
- `a_drill_preview_hides_the_open_menu_and_keeps_its_pad` (source test over `runtime/ui/worker.rs`, `tick.rs`, `state.rs`): the effect goes through `rr_host::request_drill_preview` behind the shared gate; `visible` falls with `PREVIEW_HIDES`; a press while hidden reaches nothing but the stop; the worker alone writes the flag, and its pass stops the player when neither the menu nor the pop-up is open.
