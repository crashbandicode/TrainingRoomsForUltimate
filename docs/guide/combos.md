# Combo files: keep, edit and share combos

The combo helper (Recording › Combos & Sequences, on the Recording page and in the pop-up; see the
[Recording guide](recording.md#combos)) saves every combo as a small text file on the SD card. You can copy these files to another card, send them to a friend, or write one by
hand. A combo is not tied to a fighter: the fighter it was recorded on is a note, and the helper grades whatever
fighter Player 1 is.

## Where the files are

```text
sd:/ultimate/TrainingRooms/combos/
    combo-1.json        a combo the plugin saved ("Combo 1")
    nair-loop.json      a combo saved with the name "Nair loop"
    Ryu BnB.json        a file someone copied in
```

The drill, the one combo or sequence the helper grades, is kept in `sd:/ultimate/TrainingRooms/recording-index.json`
with the rest of the recording index, not in this folder.

- **Add a combo:** copy its `.json` file into this folder. It is listed under COMBOS the next time the menu opens,
  because the menu reads the card at every open. No restart is needed.
- **File names:** any name works, up to 64 characters before `.json`, except that it must not:
  - contain `/ \ : * ? " < > |` or control characters;
  - start with a dot (hidden files, such as the `._` files a Mac writes, are never combos);
  - start or end with a space.

  The plugin names its own files from the combo's name (`nair-loop.json`, `nair-loop-2.json`, …).
- **The plugin never replaces a file it did not write unless you act:**
  - *+ Save last combo…*, the first row of COMBOS, always makes a new file;
  - *Rename*, on the combo's own screen, changes only the `name` field of that file;
  - *Delete*, on the combo's own screen, removes it.
- **Writes are atomic:** a `.tmp` file is written, then renamed over the old one. A card pulled out mid-write keeps the
  old file.

## Sharing a combo

![A combo file copied from one card to the same folder of another, then listed under COMBOS](../diagrams/combo-sharing.png)

1. Open Recording › Combos & Sequences to check the name: COMBOS lists each file's name, with "Recorded on" and the
   fighter under it.
2. Take the card out (or use FTP), and copy `sd:/ultimate/TrainingRooms/combos/<file>.json`.
3. On the other card, put the file in the same folder and open the menu. The combo is listed under COMBOS. Drill it
   (A in the pop-up, or Drill on its own screen), then do the combo on any fighter. A shared file keeps its own file
   name, and it stays the drill like any other combo.

## Practising on the recorded fighter

When you drill a combo whose `recorded_on` is not Player 1's fighter, the plugin asks Fighter Mismatch, the same
question a macro recorded on another fighter asks ([Recording guide](recording.md#fighter-mismatch)):

- *Keep current fighter* (the default): the combo becomes the drill, and you practise it on the fighter you have.
- *Switch to recorded fighter*: the combo becomes the drill, and Player 1 changes to that fighter when you close the
  menu. This works with **Player Swapping** (Fighters & Stages) On or Off. Player Swapping stays as it is, and with it
  Off the plugin still never changes Player 1 on its own.
- B: nothing changes.

The question is skipped when the combo has no `recorded_on`, when the fighter already matches, or when
**Ask before switching fighter** (Recording › Replays) is Off. Off keeps the fighter you have, as Keep does.

## Writing a file by hand

The rest of this page is for writing or fixing a combo file yourself.

### The format

One JSON object, at most 64 KiB:

```json
{
  "schema_version": 1,
  "name": "Pikachu nair loop",
  "recorded_on": "Pikachu",
  "links": [
    {
      "from": "aerial",
      "to": "aerial",
      "verdict": {"kind": "true", "link": -2},
      "window": [0, 4],
      "steps": [
        {"button": "jump", "aim": "neutral", "offset": 2},
        {"button": "attack", "aim": "neutral", "offset": 5}
      ]
    }
  ]
}
```

| Field | Required | Meaning |
| --- | --- | --- |
| `schema_version` | no (1 when left out) | The format's version, 1 to 4 (see Versions under Reading rules). A file with a larger number was made by a newer TrainingRooms and is skipped with a notice. |
| `name` | no | Shown in the list, 1 to 48 characters. Left out or blank: the file name is used. A longer name is shortened in the list; the file is not changed. |
| `recorded_on` | no | The fighter Player 1 was when the combo was saved, for example `"Ryu"`, written as the menu names fighters (case does not matter). The list shows "Recorded on Ryu". When you choose the combo while P1 is another fighter, the plugin can ask whether to switch P1 to it (see below). The helper grades any fighter either way. |
| `links` | yes, 1 to 16 | The combo, link by link, in order. |

Each link:

| Field | Required | Meaning |
| --- | --- | --- |
| `from` | yes | The move that hit first. |
| `to` | yes | The follow-up move. The helper's attempt starts when its first link's `from` lands, and each link must use the same follow-up move. |
| `verdict` | yes | What the frame strip measured when the combo was recorded: `{"kind": "true", "link": N}` for a true combo, where N ≤ 0 is the margin in frames (`null` when unknown), or `{"kind": "drop", "link": N}` where N ≥ 1 is how many frames the CPU was free. Shown for comparison only: an attempt is graded on its own verdict. |
| `window` | no (`null`) | The combo window, `[first, last]`: the frames on which the follow-up's press would still have made a true combo. |
| `steps` | yes when `events` is absent, else the press subset | The presses of the follow-up, oldest first, at most 4. A file with no `events` is graded on these presses. A newer file still writes them so an older build can open it. |
| `events` | no | The input events of the follow-up, oldest first, at most 12: presses, releases, the C-stick, stick flicks and special directions. When this field is present, the combo is graded on it. |
| `from_shown`, `to_shown` | no | Description objects for both moves. Both must be present to describe the link. Members: `of` for the continued move; `angle` for `up` or `down`; `variant` for a number from 0 to 31; `move`, only `final_smash` beside `other` in `from` or `to` (an older file may hold `grab`, `throw`, `ledge_attack`, `getup_attack` or `trip_attack` there). An empty object is valid. |
| `spot` | no | The spot the follow-up's hit landed: `"sweet"`, `"sour"` or `"late"`. A link with no `spot`, or any other word, has no spot and always matches. It is compared only when Recording › Options › [Match sweet/sour spot](../menu/recording.md#match-sweet-sour-spot) is On. |
| `drift` | no | The held direction between the events, facing-relative. Shown only. It is not graded. |

Each step:

| Field | Meaning |
| --- | --- |
| `button` | `attack`, `special`, `jump`, `shield` or `grab`. |
| `aim` | The stick when the button went down: `neutral`, `up`, `down` or `side`. Left and right are both `side`. This is how a presses-only file is graded. |
| `offset` | When the button went down, in frames from A₁. A negative number means it was buffered. The helper compares each attempt's press with this one: positive is late. |

Each event, when `events` is present:

| Field | Meaning |
| --- | --- |
| `kind` | `press`, `release`, `cstick`, `flick` or `aim` (the latter in versions 3 and 4). |
| `button` | For a press or a release: `attack`, `special`, `jump`, `shield` or `grab`. |
| `flick` | For a flick: `fast_fall`, `dash` or `tap_jump`. |
| `part` | For an aim: 1 or 2. |
| `hold` | For a release whose press is in the same link or drill: how many frames the button was held. Optional; a whole number of 0 or more. The offsets decide the grade, not this value. |
| `reacted` | This build writes `true` or `false` on each input-drill release: `true` grades its hold; `false` does not. When absent in an older file, an owned hold under eight frames is not graded and a hold of eight or more is graded in its band. Re-record the drill to grade releases by the game's reaction. |
| `dir` | Facing-relative: `neutral`, `forward`, `forward_up`, `up`, `back_up`, `back`, `back_down`, `down` or `forward_down`. The same combo facing the other way is the same direction. |
| `offset` | The frame of the event, counted from A₁ the same way a press is. |

**Frames and A₁.** All numbers are game frames (1/60 s). They count from **A₁**, the first frame after the first hit
on which Player 1 could act. Every link starts again at its own A₁.

**Moves** (`from`, `to`): `jab`, `dash_attack`, `side_tilt`, `up_tilt`, `down_tilt`, `side_smash`, `up_smash`,
`down_smash`, `aerial`, `neutral_special`, `side_special`, `up_special`, `down_special`, `grab`, `throw`,
`ledge_attack`, `getup_attack`, `trip_attack`, `other`. These are kinds of move, not one fighter's moves, so a combo
recorded on one fighter can be practised on any other. All aerials are `aerial`; a newer file's event direction tells
a forward air from a neutral air, relative to the facing. An older file's `aim` still uses `side` for both left and right.

`grab`, `throw`, `ledge_attack`, `getup_attack` and `trip_attack` are written by name in `from` and `to`
(2026-11-25). This is not backward compatible: install 4 (`a945b0c1`) and earlier shipped installs skip a file that
has one of the five there, with `bad combo field: from` or `to`. Files without them are not affected.

An older file has `other` in `from` or `to` and the move in the description's `move`. It still reads the same.
A rename keeps it as it is; a crop writes the names. A Final Smash keeps the older form: `other`, with
`"move": "final_smash"` in the description. A file with `final_smash` straight in `from` or `to` is skipped,
and so is one whose `from` or `to` names a move and whose description also has `move`.

### Reading rules

- **Unknown fields are ignored**, at the top and inside links, steps, and events, so you can add your own notes. A rename keeps
  them.
- **A file that cannot be used** is skipped: the list shows the others. A notice names the first such file and the
  reason, for example `typo.json skipped: malformed file: bad combo field: to` (a misspelt follow-up move), and adds
  "(+N more)" when there are several. Reasons:
  - a misspelt move, button, aim, event or direction;
  - a `hold`, `reacted`, `from_shown` or `to_shown` with the wrong shape or invalid members;
  - a missing or empty `links`;
  - more than 16 links, more than 4 presses in `steps`, or more than 12 events;
  - broken JSON;
  - a newer `schema_version`;
  - a file over 64 KiB;
  - a file name that is not allowed.
- **Order:** the list is sorted by name (ignoring case), then by file name.
- **Versions:** a hit combo is written as version 1, or as 3 when it has special directions (`aim` events); an input
  drill as 2, or as 4 with them. Fields added to a version since are optional: a build that reads that version
  ignores the fields it does not know. A build skips a version it does not know.

![Recording > Combos & Sequences: the COMBOS list with Jab jab, Luigi jabs (Recorded on Luigi), Jab nair and Nair nair](../media/rc13-combo-library.png)<br>
*COMBOS after combo files were copied into `combos/`: each lists its name and "Recorded on" fighter, and the one made on Luigi says so. Eden, rc.13 (untested on hardware).*

## Input-drill files (rc.15)

Input drills share this folder and the combo id namespace, with default names Drill N. They are listed separately under
COMBOS > DRILLS. A drill has an explicit kind and input steps: version 2 without aim events, or version 4 with them.

```json
{"schema_version":2,"kind":"input_drill","name":"Drill 1","input_steps":[{"events":[{"kind":"press","button":"jump","dir":"neutral","offset":0},{"kind":"release","button":"jump","dir":"neutral","offset":2,"reacted":false}]}]}
```

Each step has 1–12 events. There are at most 16 steps, sharing the combo capacity. Event fields use the same kinds and
facing-relative directions as link events. Offsets are nonnegative frames from the take's first event, which is offset
0; they stay global across steps. A press, C-stick input or flick after a gap greater than 20 frames starts a step;
a release or first aim does not split on that gap. A second aim starts a step, and a full 12-event step continues in
another. On load, a step beginning with a release or first aim joins its predecessor if both fit in 12 events; the
file is not changed by loading. A whole-step crop rebases the retained events from the new first event. An optional
`from_shown` and `to_shown` on each step store the first and last move's `move` and optional `of`, `angle` and
`variant` for display; a single input uses `to_shown`. A later special direction keeps the continued special in
`from_shown` and any last input of its own in `to_shown`. Grading still checks inputs.
No hit, spot, window or TRUE verdict is stored or graded. A reacted release is required when its press is in the drill. Optional
`recorded_on` and `made_from` have the same meaning as on hit combos; direct recordings have no `made_from`.
Renaming preserves the kind and unknown metadata. Schema-1 files remain readable.
