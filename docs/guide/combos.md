# Share a combo

## Copy a combo to another card

![A combo file can be shared without the library.](../diagrams/combo-sharing.png)

| Step | What to do |
|---|---|
| 1. Identify it | In **Recording → Combos & Sequences**, note the combo name and recorded fighter. |
| 2. Copy its file | Find the matching JSON under `sd:/ultimate/TrainingRooms/combos/`; copy just that file. |
| 3. Import it | Put it in the other card's `combos/` folder without overwriting a different combo. |
| 4. Practise | Reopen the menu, select the combo and choose **Drill**. No restart needed. |
| Different fighter? | Answer [Fighter Mismatch](recording.md#fighter-mismatch); keeping your fighter still lets you practise. |
| <a id='the-format'></a>Need the file layout? | See [combo storage](../../specs/SPEC-input-07-combo-helper.md) and the [input grading rules](../design/action-model.md). |

### Name the moves in a link

| **Moves** (`from`, `to`) | Accepted keys |
|---|---|
| Jab, dash attack and tilts | `jab`, `dash_attack`, `side_tilt`, `up_tilt`, `down_tilt` |
| Smashes and aerials | `side_smash`, `up_smash`, `down_smash`, `aerial` |
| Specials | `neutral_special`, `side_special`, `up_special`, `down_special` |
| Grabs, throws and getups | `grab`, `throw`, `ledge_attack`, `getup_attack`, `trip_attack` |
| Other moves | `other` |

These are kinds of move, not fighter-specific move names.

### Reading rules

| When sharing | What the reader does |
|---|---|
| Sharing named grab, throw or getup keys | Check [reader compatibility](../RELEASE-NOTES-rc15.md#combo-reader-through-a945b0c1) before sending the file. |
| Older file with `other` and a move description | Still reads; renaming preserves it, cropping writes the named keys. |
| Final Smash | Use `other` with `final_smash` in the description, not directly in `from` or `to`. |
