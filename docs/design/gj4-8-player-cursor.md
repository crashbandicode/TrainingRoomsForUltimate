# Guard the Edge and Juggler: Player cursor

Both [Guard the Edge](../guide/guard-the-edge.md) and [Juggler](../guide/juggler.md) use this floor check on the Player page.

## Original defect, 2026-11-23

| Install 1 observation | What the player saw |
| --- | --- |
| Stick crossed the end of a floor | The cursor moved unseen beyond the last valid Player point. |
| Crosshair, notice and bridge | All three continued to show the last point with a floor below. |
| A, reversal or D-pad movement | A failed. Movement stayed invisible until the cursor returned over the floor. |
| Juggler, Yoga | Successive reads were −64, −78 and −78. |
| Guard the Edge, same boot | Successive reads were −66 and −78.9856. |

The receipts are listed in `artifacts/phase23/fix-round-20261123/gj4/gj4-8/CAUSE.md`.

## Shipped cursor rule

| Floor read | Result |
| --- | --- |
| Floor below the new cursor | Update the last floor position and the Player point. |
| New position misses | Restore the last floor position and read it again. |
| That floor is gone too | A refuses the point. |

The fix kept the existing floor lookup for both drills.

## Requirement trace

| Ruling clause (tr-primary-2 2026-11-23 20:11:55Z) | Design | Test |
| --- | --- | --- |
| a cursor move with no floor under it is not kept | A missed move is undone | `a_juggler_player_cursor_stays_where_its_floor_was_read`, `a_guard_player_cursor_on_a_point_with_no_floor_goes_back_to_its_last_floor` |
| the cursor returns to the last position that had one | The cursor and Player point use the last floor position | `a_dpad_step_reaches_the_last_whole_unit_of_a_floor`, `a_player_point_whose_floor_is_gone_is_refused` |
| no Player point without a floor (the restriction keeps its meaning) | A missing floor at both positions refuses A | `a_player_point_whose_floor_is_gone_is_refused`, `a_player_point_with_no_floor_is_refused_and_the_floor_read_stays_the_card_build_s` |
| after a stick hold past the floor's end the point reads -78; one D-pad Right gives -77 | The cursor returns on each missed move, then answers the D-pad | `a_juggler_player_cursor_stays_where_its_floor_was_read` |

## Limits

- The 2026-11-23 design recorded host tests only for that revision.
- A floor separated sideways by empty space wider than one cursor step can no longer be picked.
- In Juggler the cursor's height above the highest floor is still unseen.
- The original Juggler fixture used 2.0-unit stick steps. A 1.0-unit D-pad step reached its last whole floor unit.
- A Juggler point within 1.06 of an end teeters at release; see [Player 1's start](gj4-player-start.md).
- A floor read that misses at one exact x, where a floor meets a wall, stops a step that lands exactly on it;
  a step that passes over it is kept. This is not measured on a stage.
