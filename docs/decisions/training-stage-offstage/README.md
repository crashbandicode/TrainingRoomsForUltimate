# How the Training stage's off-stage rule was chosen

This 2026-11-25 review compared the old readers with the proposed rule. For the implemented geometry, see
[Technical notes](../../TECHNICAL.md#stages). The diagrams use the game's stage file and the plugin's readers;
they are not game footage.

## 1. The stage

![The Training stage's collision](training-01-outline.png)

- The Training stage is **three separate solids**: a block with a wall on the left, a Battlefield-shaped island with
  three platforms, and a long floor on the right. The fighters start on the long floor.
- The block's top and the long floor both run out through the side blast lines. Nobody can fall off there.
- Air with nothing under it exists only in the two gaps beside the island (each 90 units wide) and under the island.
- The game's own stage file lists four ledges: one on each side of each gap.

## 2. What the readers saw before the change

![What each reader takes on the Training stage](training-02-readers-today.png)

- Guard the Edge, Juggler and the eggs Off stage zone used the solid under the middle of the camera.
  On Training that was the long floor only.
- Tech traced the solid under Player 1, plus the platforms.
- Placement queried the floor under each point and found all three floors.

## 3. Where the old rule put fighters and targets

![Off-stage placement and target positions before the change](training-03-off-stage-today.png)

- **Eggs, Off stage:** the zone reached from the long floor into the island; about a quarter of the recorded target
  spots fell inside the island's solid.
- **Guard the Edge and Juggler:** the long floor ended past the blast line, so neither drill had a valid stage.
  This finding came from code, not a run.
- **Placement, Off stage:** switching to Off stage left the fighter above its old floor. The crosshair did not use
  the stage's shape.

## 4. The proposed rule

![The proposed rule on the Training stage](training-04-off-stage-proposed.png)

One rule for every stage, with no list of stages:

- A point is **off stage** when it is inside the blast lines, not inside a solid, and there is nothing to stand on
  beneath it down to the bottom blast line.
- **Under a solid** (teal) is kept as its own class, so each feature can take it or leave it.
- A **ledge** is the end of a solid floor that has such open air beside it.
- A **default spot** (the white crosses) is the middle of the open air beside a ledge.

On Training this marks the two gaps and the same four ledges the stage file lists. The block's top, the step at the
wall and the long floor's far end are not ledges.

![Battlefield under the old and proposed rules](battlefield-05-today-and-proposed.png)

On Battlefield both rules marked the same area and the same two ledges.

## 5. Three choices

**Answered 2026-11-25 about 17:05Z:** T1 A; T2 an option in Guard the Edge to start the CPU under the stage (and one for
the CPU's midair jumps already used); T3 was included. See [the decisions](../../DECISIONS.md).

The table preserves the original alternatives, not the final choices.

| | Choice | A (recommended) | B |
| --- | --- | --- | --- |
| **T1** | Placement, when a fighter is switched to Off stage | It goes to the default spot of the nearest ledge with open air. The crosshair then moves freely, as now. | Off stage accepts only off-stage points; the air above a floor becomes a third place. More change to the picker. |
| **T2** | Under a solid (under the island, under Battlefield) | Placement accepts it; Guard the Edge keeps refusing it, as decided on 2026-11-22. | No feature accepts it. |
| **T3** | Guard the Edge, Juggler and the eggs Off stage zone | Move them to the same rule in a later build, so the drills work on Training and the zone stays out of the island. | Placement only; the other three stay as they are. |

## 6. Not shown

- Nothing was run on the emulator or the console for this review. The stage's shape, the ledges the readers find and
  the eggs zone come from earlier recorded runs; the rest is read from the code.
- The original review did not cover moving stages or stages without an open-air ledge.
