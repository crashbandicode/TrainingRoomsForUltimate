# 0.2.0-rc.6

Notes for players. This is the build after `0.2.0-rc.5`. How to use recording is [RECORDING.md](RECORDING.md). The
pictures for those pages are the list in [MEDIA.md](MEDIA.md).

## Sequence drilling

rc.5 let you build and edit sequences, but not drill them. rc.6 drills them.

- A sequence is an ordered list of combos. Drill sequence, or A on a sequence in the pop-up, makes it the one drill.
  Drilling a sequence replaces a drilled combo, and drilling a combo replaces a drilled sequence.
- The bar under the frame strip is the combo helper's bar with a sequence header and a row for the sequence's combos:
  SEQUENCE, the name, combo k of n, the current combo, its links and the tolerance, and the streak and best. Done
  combos are green with a check, the current one is white, and the later ones are grey.
- Combo 2 follows combo 1 inside the same exchange. The link that joins them is shown on the strip and is not graded,
  but the move it lands must be combo 2's first move. The CPU may act for up to 20 frames between the combos. Each
  combo is graded the same way as a single drilled combo: the input events, the direction, the order, the tolerance
  and the move. TRUE and drift are shown, not graded.
- A finished sequence adds one to the streak, and best is the highest streak. A drop sends the sequence back to combo
  1 and the streak to 0, and best is kept. A different move after the joining link, the exchange ending before the
  next combo, and a save-state load or scene change in the middle of a sequence are drops too.
- The streak and best are saved in the sequence file. The menu shows the same numbers as the bar, and they stay after
  you leave Training.
- A numbered row inside a sequence drills that combo alone, with that combo's own streak. The sequence file stays.
- A deleted combo is skipped. An empty sequence drills nothing. Every combo of a sequence is drilled, however many it
  has.

![Drill a sequence: combo 1, then combo 2, then a drop back at combo 1](media/rc6-sequence-drill.gif)

### Saved now

- Edit combos on a sequence and Edit chain on a chain are written to the card. In rc.5 the new order lasted only
  until the menu reloaded.
- Delete on a macro, a chain, a combo or a sequence removes the file. In rc.5 only a save state's Delete reached the
  card; the others came back the next time the menu read the card.

### The notice moves down

The sequence bar is taller than the combo bar, so notices sit 56 layout units (about 37 pixels at 1280×720) lower
than in rc.5, under either bar.
