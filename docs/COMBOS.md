# Combo files: keep, edit and share combos

The combo helper (Recording page › Combos; [SPEC-input-07](../specs/SPEC-input-07-combo-helper.md)) saves every combo
as a small text file on the SD card. You can copy these files to another card, send them to a friend, or write one by
hand. A combo is not tied to a fighter: the fighter it was recorded on is a note, and the helper grades whatever
fighter Player 1 is.

## Where the files are

```text
sd:/ultimate/TrainingRooms/combos/
    combo-1.json        a combo the plugin saved ("Combo 1")
    nair-loop.json      a combo saved with the name "Nair loop"
    Ryu BnB.json        a file someone copied in
    selected.txt        the id of the target combo (the file name without .json)
```

- **Add a combo:** copy its `.json` file into this folder. It appears in **Saved combos** the next time you open the
  list. No restart is needed.
- **File names:** any name works, up to 64 characters before `.json`, except that it must not:
  - contain `/ \ : * ? " < > |` or control characters;
  - start with a dot (hidden files, such as the `._` files a Mac writes, are never combos);
  - start or end with a space.

  The plugin names its own files from the combo's name (`nair-loop.json`, `nair-loop-2.json`, …).
- **The plugin never replaces a file it did not write unless you act:**
  - *Save last combo* always makes a new file;
  - *Rename* (X in the list) changes only the `name` field of that file;
  - *Delete* (Y) removes it.
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
| `steps` | yes, 0 to 4 | The presses of the follow-up, oldest first. |

Each step:

| Field | Meaning |
| --- | --- |
| `button` | `attack`, `special`, `jump`, `shield` or `grab`. |
| `aim` | The stick when the button went down: `neutral`, `up`, `down` or `side`. Left and right are both `side`, so the facing does not matter. |
| `offset` | When the button went down, in frames from A₁. A negative number means it was buffered. The helper compares each attempt's press with this one: positive is late. |

**Frames and A₁.** All numbers are game frames (1/60 s). They count from **A₁**, the first frame after the first hit
on which Player 1 could act. Every link starts again at its own A₁.

**Moves** (`from`, `to`): `jab`, `dash_attack`, `side_tilt`, `up_tilt`, `down_tilt`, `side_smash`, `up_smash`,
`down_smash`, `aerial`, `neutral_special`, `side_special`, `up_special`, `down_special`, `other`. These are kinds of
move, not one fighter's moves, so a combo recorded on one fighter can be practised on any other. All aerials are
`aerial`; the step's `aim` tells a forward air from a neutral air.

## Reading rules

- **Unknown fields are ignored**, at the top and inside links and steps, so you can add your own notes. A rename keeps
  them.
- **A file that cannot be used** is skipped: the list shows the others. A notice names the first such file and the
  reason, for example `typo.json skipped: malformed file: bad combo field: to` (a misspelt follow-up move), and adds
  "(+N more)" when there are several. Reasons:
  - a misspelt move, button or aim;
  - a missing or empty `links`;
  - more than 16 links or 4 steps;
  - broken JSON;
  - a newer `schema_version`;
  - a file over 64 KiB;
  - a file name that is not allowed.
- **Order:** the list is sorted by name (ignoring case), then by file name.
- **Versions:** files written by 0.2.0-rc.4 and later all have `schema_version` 1. Later fields are optional, so an
  older build reads a newer file and ignores what it does not know.

## Sharing a combo

1. Open the list (Recording › Combos › Saved combos) to check the name.
2. Take the card out (or use FTP), and copy `sd:/ultimate/TrainingRooms/combos/<file>.json`.
3. On the other card, put the file in the same folder and open Saved combos. Choose it with A to make it the target.
   Then do the combo on any fighter.

## Practising on the recorded fighter

When you choose a combo (A in Saved combos) whose `recorded_on` is not Player 1's fighter, the plugin asks the same
question a named macro recorded on another fighter asks:

- *Keep current fighter* (the default): the combo becomes the target, and you practise it on the fighter you have.
- *Switch to recorded fighter*: the combo becomes the target, and Player 1 changes to that fighter when you close the
  menu. This works with **Player Swapping** (Fighters & Stages) On or Off. Player Swapping stays as it is, and with it
  Off the plugin still never changes Player 1 on its own.
- B: nothing changes.

The question is skipped in these cases:

- **No question at all** when the combo has no `recorded_on` or when the fighter already matches.
- **The current fighter is kept, without asking:**
  - when **Ask before switching fighter** (Recording › Named macros) is Off;
  - when `recorded_on` is not a fighter name the plugin knows; the notice says so.
