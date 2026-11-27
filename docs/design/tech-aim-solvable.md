# Tech practice aim: a target is its line and a parameter, and one draw without a start does not end the session

2026-11-27, lane preview-vis-sol. Worktree `/home/intpa/tr-fixes-20261113/tech-aim`, branch `fix/tech-aim-solvable`
from `rc-next` `0a63c101`. Playbook `c8d9959`. Orders: tr-primary-2, inbox 2026-11-27 05:40:04Z (the defect and the
behaviour), 05:48:17Z (the bound of eight and the text), 05:50:07Z (part 1 revised after rv-hit-streak's finding of
05:49:37Z) and 05:57:16Z (go, with the remembered spot's field). Finding: docs-tutorials 05:39:25Z. Reviewer: rv-hit-streak.

## What was wrong (measured on the host, `gate/repro1`, `gate/repro2`, `gate/red1`)

With factory settings and Surface = Floor on a flat stage, 1,330 of 12,844 draws had no start, and 99 of 1,000 attempts
as played had no plan, the first at attempt 5. Every refusal was one check: the solver took a target only when the
line's snap of it equalled it bit for bit (`solver.rs:65`). The picker computes the point from a parameter; a line's
snap is not its own fixed point in f32 (of 65,536 positions a snap is moved by the next snap 127 times on a flat line
and 4,626 times on a sloped one), so snapping once is no fix, and the Chosen point, itself a snap, had the same fault.
One refused draw then ended the session (`tech.rs`, `planning_failed`).

## Design

1. **A spot is its line, a parameter `t` and one flag.** `picker::Spot { line, t, far }`: `t` is 0 at the end it is
   measured from and 1 at the other; `far` says it is measured from the line's second end.
   - One world-point function: `Spot::point` (`Line::at`).
   - One membership function: `Spot::on_segment` (`t` from 0 to 1 on a valid line; no projection in it). The solver
     calls it in place of the snap-and-compare, the planner asks it first whatever the start mode, the wizard asks it
     before it takes a spot, and saving and recovering a remembered spot go through it (the reviewer's correction,
     rv-hit-streak 2026-11-27 06:38:56Z). No position is compared, and there is no tolerance. A parameter outside 0 to
     1, or not finite, is refused.
   - Producers: the random spot (its drawn `t`), the cursor's nearest point and the picker's restore (the projection's
     clamped parameter, `Line::project`, `Surfaces::nearest`), the remembered spot.
   - The remembered spot stores its line's ends in the order `t` is measured in, and `t`. Rebinding keeps the fresh
     line as read and sets `far` when its ends come back reversed, so the world point is the same arithmetic and the
     same bits, and the retained launch's target still equals it.
2. **An attempt draws up to eight candidates in all** from the same settings, in the same frame
   (`planner::draw_attempt`, `CANDIDATES`). The bound: values of the player's own can leave no draw a start, and with
   every value fixed each draw is the same, so the loop must end; each candidate is at most 73 passes of the solver on
   the game frame.
3. **After the eighth the drill stops** with "No launch fits these Tech practice values", restriction
   `tech-no-launch-fits` (`docs/RESTRICTIONS.md`). Factory settings never reach it.
4. **The planner says why** (`planner::Refused`: values, aim, start); only the diagnostic log shows it.

## Limits

- Host tests and builds only. The flat and sloped stages of the tests are solid blocks of my own, not a stage's
  measured lines.
- With every value fixed, the eight candidates are the same draw: the stop comes after eight tries of it.
- A "Good to know" row for the stop is the docs lane's on the joined docs; `docs/guide/tech-practice.md`'s caption still
  quotes the old line of an old film.

## Requirement trace

| Order clause | Design | Test |
| --- | --- | --- |
| "one unsolvable draw must NOT end the session" (05:40:04Z) | 2 | `an_unsolvable_draw_is_drawn_again_and_the_stop_is_a_listed_restriction` |
| "Factory settings must never reach the 'no solution' stop" (05:40:04Z) | 1 | `factory_draws_on_a_flat_floor_never_need_a_second_candidate` |
| "If the player's own values make EVERY draw unsolvable, the drill stops with a clear message ... a documented restriction" (05:40:04Z) | 3 | `an_unsolvable_draw_is_drawn_again_and_the_stop_is_a_listed_restriction` |
| "Bound the number of tries per attempt and say why that bound" (05:40:04Z); "8 TOTAL candidates, same attempt and frame" (05:50:07Z) | 2 | `an_attempt_draws_eight_candidates_in_all` |
| "carry the target as its line plus its parameter ... ONE membership function shared by every producer and consumer" (05:50:07Z) | 1 | `every_position_of_every_producer_is_a_target_the_solver_takes` |
| "Off-segment targets are still refused" (05:50:07Z) | 1 | `a_target_off_its_segment_is_refused` |
| the same, for the wizard and for a Set or Random start (rv-hit-streak 06:38:56Z) | 1 | `off_segment_targets_never_replace_the_wizard_pick_or_get_a_manual_start` |
| "Red first against a one-snap attempt using rv-hit-streak's two draws" (05:50:07Z) | 1 | `the_two_reviewed_draws_have_a_start_and_one_snap_is_not_why` |
| "cover the Chosen/restore path" (05:50:07Z) | 1 | `every_position_of_every_producer_is_a_target_the_solver_takes` |
| "zero redraws on the factory fixture" (05:50:07Z) | 1, 2 | `factory_draws_on_a_flat_floor_never_need_a_second_candidate` |
| the remembered spot on a reversed line: "pin exact world point and retained launch target" (rv-hit-streak 05:55:51Z, accepted 05:57:16Z) | 1 | `a_remembered_spot_returns_to_its_exact_point_on_a_reversed_line` |
