# Combo files: keep, edit and share combos

The combo helper (Recording › Combos & Sequences, on the Recording page and in the pop-up;
[SPEC-input-07](../../specs/SPEC-input-07-combo-helper.md); [RECORDING.md](recording.md), Combos) saves every combo
as a small text file on the SD card. You can copy these files to another card, send them to a friend, or write one by
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
with the rest of the recording index (`src/model/library/hot.rs:1`), not in this folder.

- **Add a combo:** copy its `.json` file into this folder. It is listed under COMBOS the next time the menu opens; the
  menu reads the card at every open (`src/model/recording_menu.rs:2152`). No restart is needed.
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

## The format

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
| `schema_version` | no (1 when left out) | The format's version. This document is version 1. A file with a larger number was made by a newer TrainingRooms and is skipped with a notice. |
| `name` | no | Shown in the list, 1 to 48 characters. Left out or blank: the file name is used. A longer name is shortened in the list; the file is not changed. |
| `recorded_on` | no | The fighter Player 1 was when the combo was saved, for example `"Ryu"`, written as the menu names fighters (case does not matter). The list shows "recorded on Ryu". When you choose the combo while P1 is another fighter, the plugin can ask whether to switch P1 to it (see below). The helper grades any fighter either way. |
| `links` | yes, 1 to 16 | The combo, link by link, in order. |

Each link:

| Field | Required | Meaning |
| --- | --- | --- |
| `from` | yes | The move that hit first. |
| `to` | yes | The follow-up move. The helper's attempt starts when its first link's `from` lands, and each link must use the same follow-up move. |
| `verdict` | yes | What the frame strip measured when the combo was recorded: `{"kind": "true", "link": N}` for a true combo, where N ≤ 0 is the margin in frames (`null` when unknown), or `{"kind": "drop", "link": N}` where N ≥ 1 is how many frames the CPU was free. Shown for comparison only: an attempt is graded on its own verdict. |
| `window` | no (`null`) | The combo window, `[first, last]`: the frames on which the follow-up's press would still have made a true combo. |
| `steps` | yes when `events` is absent, else the press subset | The presses of the follow-up, oldest first, at most 4. A file with no `events` is graded on these presses. A newer file still writes them so an older build can open it. |
| `events` | no | The input events of the follow-up, oldest first, at most 12: presses, releases, the C-stick, and stick flicks. When this field is present, the combo is graded on it. |
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
| `kind` | `press`, `release`, `cstick` or `flick`. |
| `button` | For a press or a release: `attack`, `special`, `jump`, `shield` or `grab`. |
| `flick` | For a flick: `fast_fall`, `dash` or `tap_jump`. |
| `dir` | Facing-relative: `neutral`, `forward`, `forward_up`, `up`, `back_up`, `back`, `back_down`, `down` or `forward_down`. The same combo facing the other way is the same direction. |
| `offset` | The frame of the event, counted from A₁ the same way a press is. |

**Frames and A₁.** All numbers are game frames (1/60 s). They count from **A₁**, the first frame after the first hit
on which Player 1 could act. Every link starts again at its own A₁.

**Moves** (`from`, `to`): `jab`, `dash_attack`, `side_tilt`, `up_tilt`, `down_tilt`, `side_smash`, `up_smash`,
`down_smash`, `aerial`, `neutral_special`, `side_special`, `up_special`, `down_special`, `other`. These are kinds of
move, not one fighter's moves, so a combo recorded on one fighter can be practised on any other. All aerials are
`aerial`; a newer file's event direction tells a forward air from a neutral air, relative to the facing. An older file's `aim` still uses `side` for both left and right.

## Reading rules

- **Unknown fields are ignored**, at the top and inside links, steps, and events, so you can add your own notes. A rename keeps
  them.
- **A file that cannot be used** is skipped: the list shows the others. A notice names the first such file and the
  reason, for example `typo.json skipped: malformed file: bad combo field: to` (a misspelt follow-up move), and adds
  "(+N more)" when there are several. Reasons:
  - a misspelt move, button, aim, event or direction;
  - a missing or empty `links`;
  - more than 16 links, more than 4 presses in `steps`, or more than 12 events;
  - broken JSON;
  - a newer `schema_version`;
  - a file over 64 KiB;
  - a file name that is not allowed.
- **Order:** the list is sorted by name (ignoring case), then by file name.
- **Versions:** files written by 0.2.0-rc.4 and later all have `schema_version` 1. Later fields are optional, so an
  older build reads a newer file and ignores what it does not know.

## Sharing a combo

1. Open Recording › Combos & Sequences to check the name: COMBOS lists each file's name, with "Recorded on" and the
   fighter under it.
2. Take the card out (or use FTP), and copy `sd:/ultimate/TrainingRooms/combos/<file>.json`.
3. On the other card, put the file in the same folder and open the menu. The combo is listed under COMBOS. Drill it
   (A in the pop-up, or Drill on its own screen), then do the combo on any fighter. A shared file keeps its own file
   name, and it stays the drill like any other combo.

## Practising on the recorded fighter

When you drill a combo whose `recorded_on` is not Player 1's fighter, the plugin asks Fighter Mismatch, the same
question a macro recorded on another fighter asks ([RECORDING.md](recording.md), Fighter Mismatch):

- *Keep current fighter* (the default): the combo becomes the drill, and you practise it on the fighter you have.
- *Switch to recorded fighter*: the combo becomes the drill, and Player 1 changes to that fighter when you close the
  menu. This works with **Player Swapping** (Fighters & Stages) On or Off. Player Swapping stays as it is, and with it
  Off the plugin still never changes Player 1 on its own.
- B: nothing changes.

The question is skipped in these cases:

- **No question at all** when the combo has no `recorded_on` or when the fighter already matches.
- **The current fighter is kept, without asking:**
  - when **Ask before switching fighter** (Recording › Replays) is Off;
  - when `recorded_on` is not a fighter name the plugin knows; the notice says so.
