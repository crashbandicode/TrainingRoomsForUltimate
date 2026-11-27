# Tech practice aim and retry limits

The 2026-11-27 fix shipped with the retry and stop behavior described in the [Tech practice guide](../guide/tech-practice.md).

| Design record | Detail |
| --- | --- |
| Lane / reviewer | preview-vis-sol / rv-hit-streak |
| Worktree | `/home/intpa/tr-fixes-20261113/tech-aim` |
| Branch / base | `fix/tech-aim-solvable` from `rc-next` at `0a63c101` |
| Playbook | `c8d9959` |
| Initial finding | docs-tutorials, 05:39:25Z |
| Inbox decisions | tr-primary-2, 05:40:04Z and 05:48:17Z |
| Review correction | rv-hit-streak, 05:49:37Z. Part 1 revised at 05:50:07Z. |
| Implementation approval | 05:57:16Z, including the remembered spot field |

## Original host reproduction

| Before the fix | Measurement |
| --- | --- |
| Runs | `gate/repro1`, `gate/repro2`, `gate/red1` |
| Factory settings, Floor, flat stage | 1,330 of 12,844 draws had no start. |
| Attempts played | 99 of 1,000 lacked a plan. The first failure was attempt 5. |
| Repeated snap, 65,536 positions | The second snap moved 127 flat-line and 4,626 sloped-line points. |
| Solver at the time | `solver.rs:65` required bit equality between the target and its snap. |
| Affected targets | Both parameter-generated and already-snapped Chosen points could fail that test. |
| Consequence | One rejected draw called `planning_failed` in `tech.rs` and ended practice. |

## Shipped design

| Operation | Rule |
| --- | --- |
| Represent a target | `picker::Spot` stores a line, parameter `t` and direction flag `far`. |
| Interpret the parameter | Zero marks the starting endpoint, one the opposite endpoint. |
| Reverse direction | With `far` set, measure from the line’s second endpoint. |
| Compute the world point | `Spot::point` delegates to `Line::at`. |
| Test membership | `Spot::on_segment` requires a valid line and a finite parameter in [0, 1]. |
| Membership consumers | Solver, planner (all start modes), wizard, and remembered-spot save and recovery. |
| Membership review | rv-hit-streak, 06:38:56Z. Membership uses neither projection nor a position tolerance. |
| Produce a spot | Random selection supplies its drawn parameter. |
| Cursor and picker restore | `Surfaces::nearest` uses the clamped parameter from `Line::project`. |
| Remember | Store the endpoints in measurement order with their parameter. |
| Recover | Keep the fresh line. Set `far` when endpoint order reverses. |
| Recovery result | Identical arithmetic reproduces the retained launch target bit for bit. |
| Draw an attempt | `planner::draw_attempt` tries up to eight candidates with unchanged settings in one frame. |
| Bound the work | Each candidate can cost 73 solver passes. Fixed values can repeat identical failures. |
| Exhaust all eight | Stop with “No launch fits these Tech practice values”. |
| Restriction | [tech-no-launch-fits](../RESTRICTIONS.md) applies when the player’s values leave no launch. |
| Diagnostics | `planner::Refused` distinguishes values, aim and start in the diagnostic log. |

## Limits

| Evidence boundary | Detail |
| --- | --- |
| Stage geometry | The original host tests used synthetic flat and sloped stages. |
| Fixed choices | A chosen point with fixed launch values can repeat one rejected candidate. |
| Factory settings | Factory draws avoid the no-launch stop. |

## Requirement trace

| Requirement clause and decision time (UTC) | Regression test |
| --- | --- |
| Retry a draw without a start (05:40:04) | `an_unsolvable_draw_is_drawn_again_and_the_stop_is_a_listed_restriction` |
| Avoid the factory-settings stop (05:40:04) | `factory_draws_on_a_flat_floor_never_need_a_second_candidate` |
| Explain the stop for impossible player values (05:40:04) | `an_unsolvable_draw_is_drawn_again_and_the_stop_is_a_listed_restriction` |
| Count the initial draw toward the eight-candidate bound (05:50:07) | `an_attempt_draws_eight_candidates_in_all` |
| Share line-parameter membership across producers and consumers (05:50:07) | `every_position_of_every_producer_is_a_target_the_solver_takes` |
| Reject off-segment targets (05:50:07) | `a_target_off_its_segment_is_refused` |
| Apply membership to wizard, Set and Random starts (06:38:56) | `off_segment_targets_never_replace_the_wizard_pick_or_get_a_manual_start` |
| Reproduce the two failed draws against a single snap (05:50:07) | `the_two_reviewed_draws_have_a_start_and_one_snap_is_not_why` |
| Cover Chosen-point restore (05:50:07) | `every_position_of_every_producer_is_a_target_the_solver_takes` |
| Require zero redraws on the factory fixture (05:50:07) | `factory_draws_on_a_flat_floor_never_need_a_second_candidate` |
| Preserve the target on reversed lines (05:55:51, accepted 05:57:16) | `a_remembered_spot_returns_to_its_exact_point_on_a_reversed_line` |
