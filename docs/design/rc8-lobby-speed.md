# rc.8 R2: 2/3 and x1.5 in the waiting-room practice

2026-11-11, QCF (`tr-opus-qcf`), branch `p23/rc8-lobby-speed` from `9626926c`. Scope: `artifacts/phase23/TASK-rc8.md`
"SCOPE REOPENED" R2 and R9. Research: `quickplay-cancel-fix-work/RESEARCH-rc8-speed.md` §9 (static RE, Eden pairs
`qcf-spd04y` and `qcf-spd05y`). The bind itself (the chord, the pop-up, Training) is `docs/design/rc8-binds.md`.

## 1. Why not Training's way

Training's 2/3 and x1.5 raise the game's update rate (`[main+0x593b444]` code 4 or 6, `[main+0x523d004]` 120 or 180)
and add a slow slot. The update rate drives every task, the Quickplay room's timers too, so the peer would see the host's
room run fast. `qcf-spd04y` ran the whole battle task entry twice per two updates: the host's room code hit every 40
frames instead of 59 and the peer got the match 0.62 s late. **The lobby never writes those two words.**

## 2. One update in the waiting area (13.0.5)

```text
FUN_0135ca00 (the battle task entry, every update, main thread)
 |-- slow step         main+0x52c5184: frames +0, N +4, target +5, sub +6, second counter +8
 |                     N = 1: a logic frame on every update; N > 1: one per N updates (the period ends when sub -> 0)
 |-- FUN_0137b3f0(S)   the battle scheduler: phases 0..9 of tasks
 |      phase 1: ... 0x5061910 BattleObjectManager/Fighter/Item ...  0x5061b98 REGISTRY WALKER ... 0x5061c28 Fighter/Stage
 |      phase 3: ... 0x5061cb8 BattleObjectSlow/Fighter/Item ...
 |                     the walker ticks the registry objects: the Quickplay ROOM (0x506fec0) and the mode object
 |-- class update      vtable +0x50: the waiting area's FUN_0151b150
        +-- .151b170   <== lobby_speed.class_update (our inline hook): 2/3's pace, x1.5's extra pass
        +-- FUN_014f8940 (the battle flow check), the scene messages
```

## 3. Design

1. **The list:** the lobby offers Training's five, 1/4, 1/2, 2/3, x1, x1.5, in the same order (`LOBBY_SPEEDS`).
2. **1/2 and 1/4:** unchanged from `9626926c`: a slot `{1, N}` taken on Player 1's frame, freed by the practice's
   teardown.
3. **2/3, a paced slot (`LobbyPlan::TwoThirds`):** the same slot, taken as `{1, 1}`. On every waiting-area update, after
   the slow step, the hook reads N and the sub-counter; at a period's end (sub 0) it rewrites the slot's divisor to 2
   after a period of 1 and to 1 after a period of 2 (`paced_divisor`, `LobbySlot::pace`). The slow step latches the
   target only at a period's end, so the periods alternate 1, 2: two logic frames per three updates. Only the slot's
   divisor byte and the dirty byte are written, in the table the slot was taken from, while it still holds ours.
   - **Picture (updates →):** `| 1 | 2 2 | 1 | 2 2 | 1 |` with one logic frame per period: 2 per 3 updates.
   - A hit's slow (a larger divisor) wins over the paced slot, as natively.
4. **x1.5, an extra world step (`LobbyPlan::ExtraPass`):** on every other allowed update (`Cadence`), after this
   update's scheduler pass, the hook runs `run_pass`: it checks the whole walk first, as `FUN_0137b3f0` walks it
   unpaused (`plan_phases`), then reads each phase again right before ticking it, as the scheduler does, and ticks
   every unmasked task except the registry walker (the task whose tick is `main+0x1358970`), in the scheduler's order
   and with its phase word. A whole pass then adds the slow step's own two counter increments for that step (what the
   native step does at x1). **The room object and the mode object stay at x1**, every world task (fighters, items, stage) runs 1.5
   times per update.
   - **Allowed only** (`pass_allowed`) while the practice is live, its UI is free, the game isn't paused, no slow of
     any kind is active (`N` and the target both 1), and the scheduler is between passes (phase word -1).
   - **Skipped** (nothing ticked; diag `lobby pass skipped`) unless the walk is whole: every pointer checked, at most
     96 tasks in any phase, the walker found exactly once (`Plan::runnable`).
   - **Stopped**, not whole (no counters), when a phase's second read isn't clean (a tick changed a later phase) or the
     live flag drops between two ticks (the practice's teardown began inside one). From the docs lane's R2 review,
     finding 1.
5. **Teardown (R9 too):** the practice's live flag is armed by Player 1's frame in the practice and cleared by
   `game_speed::session_clear`, the same teardown that frees the slot in `9626926c` (the practice P1's destroy, the
   exit, a scene change). The pace and the extra pass stop there, before the native teardown. A match found starts at
   x1: no slot (freed), no extra pass (the match is another class, and the flag is clear).
6. **Guards:** ten `lobby_speed` rows (`tools/gen_rows_lobby_speed.py`, compared at boot; the hook is installed only
   when they and `game_speed`'s rows pass). The host tests decode every offset and constant from the guard words
   (`lobby_speed_rows_tests`). Card scan 2026-11-11 over the rc.7 card copy: no plugin refers to any of the sites.
7. **Hook rules:** one inline hook, mid-function after the prologue; no lock but `try_lock`. No `game_speed` lock is
   held while the world's tasks run (Player 1's frame inside them takes it), only the pass's own task buffer, a static
   that nothing else takes (finding 2: no buffer on the stack under the world's ticks); no heap.

## 4. Requirement trace

| Operator clause | Design | Test |
| --- | --- | --- |
| "and the lobby it will just work" | §3.1–3.5: every speed of the bind works in the waiting-room practice, with the same list as Training | `lobby_offers_the_training_speeds`, `each_lobby_speed_has_its_plan_and_none_raises_the_update_rate` |
| "can you make the speed up local too?" | §3.4: x1.5 is an extra world step that leaves the room and mode objects at x1 | `the_walk_follows_the_scheduler_and_leaves_out_the_walker_and_masked_groups`, `the_extra_pass_needs_the_live_practice_unpaused_with_no_slow_and_between_passes`, `the_extra_pass_runs_on_every_other_update_and_restarts_after_a_reset`, `the_skipped_tick_is_the_registry_walker`, `the_pass_ticks_every_task_in_the_schedulers_order_and_is_whole`, `each_phase_is_read_again_right_before_its_ticks`, `counting_and_listing_walks_agree_and_a_phase_wider_than_the_buffer_is_not_run` |
| "i said I want the speed changes etc in rc8 not another release after that" | §3.1, 3.3, 3.4: 2/3 and x1.5 in the lobby in this release | `lobby_offers_the_training_speeds`, `two_thirds_paces_periods_of_one_and_two_updates`, `half_and_quarter_are_not_paced_and_two_thirds_yields_to_a_longer_slow` |
| "why are you deferring to rc9? I never approved that." | §3.5: the teardown row R9 (1/4 match found) is proven with R2's pair, not deferred | `match_found_at_two_thirds_frees_the_paced_slot_before_the_drop`, `match_found_with_no_busy_edge_frees_the_slot_before_the_drop`, `the_paced_slot_is_kept_through_its_divisors_and_freed_with_the_one_it_holds`, `the_pass_stops_between_ticks_once_the_practice_is_no_longer_live` |

## 5. Eden proof (planned)

A Quickplay pair (b hosts and waits; a joins), the diag build, QCF's trace points:

- per cycle (x1, 2/3, x1.5, 1/4): the host's room cadence (`main+0x1564088` hits), the round start and the match
  arrival on both consoles, a clean match (`sync_end … desync_seen=0` at the same frame);
- the fighter's speed: Player 1's frames per second (a trace counter on the plugin's per-fighter frame) against x1;
- match found while holding each speed: T3 (`main+0x1fca5d0`, the match's table has no plugin slot), T5 (destroy before
  create), and the diag lines `[GameSpeed] lobby pass` / `lobby paced` stopping at the session clear;
- R9: the 1/4 match-found teardown row.
