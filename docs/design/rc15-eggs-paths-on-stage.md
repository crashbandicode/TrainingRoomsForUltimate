# Eggs-ercise: a moving path stays over the stage

2026-11-22. Report from the card: with Moving paths checked, a target sometimes flies off the stage and stops where the fighter cannot get to it.

## Cause

In view and Whole stage put a new target on one of the stage's floors. A moving path (Axe, Galaxian, Boomerang, Homing Missile) was then fitted into a box that is larger than the stage: the screen for In view, the stage's camera range for Whole stage, each cut only by the blast lines. `Motion::new` moved the start until the path's whole envelope fitted that box, and the flight ran to the envelope's end: up to 180 units sideways (Galaxian) and about 74 units below its start (Axe), where the target then rests. Nothing compared the flight with the floors. An on-stage flight has no outline check either: only an Off stage flight carries a side for it.

So the direction, the scale and the outline check are not the cause. The box is.

First failing position on main `7d962416`, from the red run of `moving_paths_stay_over_the_stage` (a floor from x −50 to 50 at height 0, a platform from −40 to −10 at height 30, the screen 137 units wide, the camera range ±100 and −60 to 90):

```text
left the stage: area 0 target 0 path 0 seed 0 tick 41: (-46.3392, 25.237444), floor from -50 to 50 at 0
```

The flight goes on from there. A probe of the same fixture (first target of seed 0; `probe-main.log` and `probe-fix.log` in `artifacts/phase23/eggs-paths-on-stage/`) has that Axe come to rest at (−62.7, −12.8) on main: 12.7 units past the ledge and 12.8 below the floor line. On Whole stage the Axe rests at (−93.1, −50.2) and the Galaxian at (−90.6, 3.7), 40.6 units past the ledge. With the fix the eight probed flights rest between x −44.7 and 27.1, at heights 4.0 to 8.6.

## Fix

The box of an on-stage moving path is also cut to the stage:

- sideways, to the span from the leftmost to the rightmost end of the floors the spawn could use, a target radius in from each end;
- below, to a target radius above the floor the target appeared over (the higher end of a sloped floor).

The fit is unchanged: the start is moved and the flight is scaled until the whole envelope is inside the box, so a flight that needs more room than the stage has is flown smaller. When no box is left (a spawn range narrower than one target), the target uses Pop up.

Unchanged: Off stage targets (their zone and the outline check), Pop up, how high above its floor a flight may start, and the random draws, so a seed gives the same spawn point and direction as before.

## Limits

- The floors are the game's floor lines without their kind. A target can pass under a platform, and on a main floor with a step it can pass through the higher part at its own floor's height.
- Between two floors that do not touch, a target can be over the gap, at or above its own floor's height.
- When the floor read gave nothing, the spawn range and the run's ground stand in for the stage.
- Not run in Eden. The host tests fly every path on both areas with the disc and with the CPU as the target.

## Requirement trace

| Operator clause | Design | Test |
| --- | --- | --- |
| Moving paths are restricted to the stage | The box a moving path is fitted into is cut to the outer ends of the usable floors and to the floor the target appeared over | `moving_paths_stay_over_the_stage` |
| A target does not end up off the stage where it cannot be reached | The flight is scaled and its start moved to fit that box, as before for the larger box; with no room over the floor the target uses Pop up | `moving_paths_stay_over_the_stage`, `a_moving_path_with_no_room_over_its_floor_pops_up` |
| Find why a flight leaves the stage, and show it with a failing host test first (tr-primary-2, 2026-11-22 14:06:33Z) | The cause above; the test fails on main at the position quoted there | `moving_paths_stay_over_the_stage` |
