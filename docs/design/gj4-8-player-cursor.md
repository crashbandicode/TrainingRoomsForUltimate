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
| Reject a move without a floor | A missed move is undone | `a_juggler_player_cursor_stays_where_its_floor_was_read`, `a_guard_player_cursor_on_a_point_with_no_floor_goes_back_to_its_last_floor` |
| Restore the last position with a floor | The cursor and Player point use the last floor position | `a_dpad_step_reaches_the_last_whole_unit_of_a_floor`, `a_player_point_whose_floor_is_gone_is_refused` |
| Require a floor for the Player point | A missing floor at both positions refuses A | `a_player_point_whose_floor_is_gone_is_refused`, `a_player_point_with_no_floor_is_refused_and_the_floor_read_stays_the_card_build_s` |
| After a stick hold reads −78, one D-pad Right gives −77 | The cursor returns on each missed move, then answers the D-pad | `a_juggler_player_cursor_stays_where_its_floor_was_read` |

## Limits

| Case | Limit |
| --- | --- |
| Evidence at the original revision | The 2026-11-23 design recorded host tests. |
| Gap wider than one cursor step | The cursor cannot cross it to select the next floor. |
| Juggler cursor above the highest floor | Its height remains unseen. |
| Juggler fixture, stick and D-pad | Two-unit stick steps stopped at −78. A one-unit D-pad step reached −79. |
| Juggler release near an end | Within the facing-end teeter range (1.06), see [Player 1’s start](gj4-player-start.md). |
| Missing floor at a wall joint | A step landing on that x stops. A step crossing it succeeds. |
| Wall joint evidence | Host fixture only. The original design recorded no stage measurement. |
