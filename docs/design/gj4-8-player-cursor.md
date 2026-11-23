# Guard the Edge and Juggler: Player cursor

## Cause

On the Player page, the stick moves the cursor, but the crosshair, notice and bridge show the last Player point
with a floor below it. Past a floor's end the cursor keeps moving unseen, so A is refused and reversing or D-pad
movement shows nothing until the cursor returns. In install 1's Yoga run Juggler read `-64`, `-78`, `-78`;
Guard the Edge in the same boot read `-66`, `-78.9856`.
See `artifacts/phase23/fix-round-20261123/gj4/gj4-8/CAUSE.md` and the receipts named there.

## Rule

The Player page reads the floor below the cursor. On a miss it returns the cursor to its last floor position and
reads that position again. If that floor is gone too, the point is refused on A. A successful read updates the
cursor's last floor position and the Player point. Both drills use this rule; the floor read itself is unchanged.

## Requirement trace

| Ruling clause (tr-primary-2 2026-11-23 20:11:55Z) | Design | Test |
| --- | --- | --- |
| a cursor move with no floor under it is not kept | A missed move is undone | `a_juggler_player_cursor_stays_where_its_floor_was_read`, `a_guard_player_cursor_on_a_point_with_no_floor_goes_back_to_its_last_floor` |
| the cursor returns to the last position that had one | The cursor and Player point use the last floor position | `a_dpad_step_reaches_the_last_whole_unit_of_a_floor`, `a_player_point_whose_floor_is_gone_is_refused` |
| no Player point without a floor (the restriction keeps its meaning) | A missing floor at both positions refuses A | `a_player_point_whose_floor_is_gone_is_refused`, `a_player_point_with_no_floor_is_refused_and_the_floor_read_stays_the_card_build_s` |
| after a stick hold past the floor's end the point reads -78; one D-pad Right gives -77 | The cursor returns on each missed move, then answers the D-pad | `a_juggler_player_cursor_stays_where_its_floor_was_read` |

## Limits

- Not flown at this head.
- A floor separated sideways by empty space wider than one cursor step can no longer be picked.
- In Juggler the cursor's height above the highest floor is still unseen.
- The stick still cannot place the point closer than 2.0 to a floor's end; the D-pad's 1.0 step reaches the last whole unit.
- A Juggler point within 1.06 of an end teeters at release; see [Player 1's start](gj4-player-start.md).
- A floor read that misses at one exact x, where a floor meets a wall, stops a step that lands exactly on it;
  a step that passes over it is kept. This is not measured on a stage.
