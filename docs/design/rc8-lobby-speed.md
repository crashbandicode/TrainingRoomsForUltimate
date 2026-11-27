# rc.8 R2: 2/3 and x1.5 in the waiting-room practice

- 2026-11-11, QCF (`tr-opus-qcf`), branch `p23/rc8-lobby-speed` from `9626926c`.
- Scope: `artifacts/phase23/TASK-rc8.md` "SCOPE REOPENED" R2 and R9.
- Research: `quickplay-cancel-fix-work/RESEARCH-rc8-speed.md` §9 (static RE, Eden pairs `qcf-spd04y` and `qcf-spd05y`).
- The bind itself (the chord, the pop-up, Training) is in the [rc.8 bind design](rc8-binds.md).

- The five-speed design shipped in rc.8.
- By 2026-11-27, [1 Frame](rc15-speed-frame.md) preceded those five speeds in both Training and waiting-room practice.
- See the [Game speed guide](../guide/game-speed.md).

## 1. Why the waiting room used a separate clock

| Observation | Result |
| --- | --- |
| Training at 2/3 or x1.5 | Adds a slow slot and raises the game update rate. |
| Rate words | `[main+0x593b444]`: code 4 or 6. `[main+0x523d004]`: 120 or 180. |
| Side effect | The update rate also clocks Quickplay room timers. |
| `qcf-spd04y` experiment | Ran the whole battle task entry twice per two updates. |
| Host room code | Hit every 40 frames instead of 59. |
| Peer | Received the match 0.62 seconds late. |
| Waiting-room rule | Leave both rate words untouched. |

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

| Speed in rc.8 | Waiting-room implementation |
| --- | --- |
| 1/4, 1/2 | Take slot `{1, N}` on Player 1’s frame. Teardown frees it, as in `9626926c`. |
| 2/3 | `LobbyPlan::TwoThirds` takes a paced slot, initially `{1, 1}`. |
| x1 | Normal update cadence. |
| x1.5 | `LobbyPlan::ExtraPass` adds a world step every other allowed update (`Cadence`). |

**Paced slot**

- After each waiting-area slow step, read N and the sub-counter.
- At sub 0, the slow step latches its target. `paced_divisor` and `LobbySlot::pace` alternate the slot divisor between 2 and 1.
- A one-update period followed by a two-update period gives two logic frames per three updates. A hit’s larger divisor still wins.
- Write only the divisor byte and dirty byte while the original table still holds the practice slot.

**Extra pass**

- After the scheduler pass, `run_pass` checks the entire walk using `plan_phases`, following the unpaused `FUN_0137b3f0` order.
- Read each phase again immediately before ticking it. Use the scheduler’s phase word and skip masked tasks and the registry walker (tick `main+0x1358970`).
- Fighters, items and stage receive the extra step. The room and mode objects stay at x1.
- A complete pass adds both slow-step counter increments, as the native x1 step does.
- `pass_allowed` requires live practice, free UI, unpaused play, N and target both 1, and phase word −1 between passes.
- `Plan::runnable` checks every pointer, caps each phase at 96 tasks and requires exactly one walker. Otherwise nothing ticks and diagnostics record `lobby pass skipped`.
- Stop if a tick invalidates a later phase or teardown clears the live flag between ticks. An incomplete pass adds no counters (R2 review, finding 1).

**Teardown and hooks**

- Player 1’s practice frame arms the live flag. `game_speed::session_clear` clears it and frees the slot on practice P1 destruction, exit or scene change, before native teardown.
- The match uses another class. With the slot freed and flag cleared, it starts at x1 (R9).
- Boot compares ten `lobby_speed` rows from `tools/gen_rows_lobby_speed.py`. Install the hook only if those and the `game_speed` guards pass.
- `lobby_speed_rows_tests` decodes offsets and constants from guard words. The 2026-11-11 rc.7 card scan found no plugin references to these sites.
- The rc.8 inline hook runs after the prologue, uses `try_lock` and allocates no heap memory.
- World tasks can reach Player 1’s frame and take the `game_speed` lock. Release that lock before running them.
- The pass holds only its exclusive static task buffer, keeping that buffer off the world ticks’ stack (finding 2).

## 4. Requirement trace

| Requirement clause | Design | Test |
| --- | --- | --- |
| The speed bind works in the waiting room | The rc.8 list matched Training. | `lobby_offers_the_training_speeds`, `each_lobby_speed_has_its_plan_and_none_raises_the_update_rate` |
| The speed-up is local too | The extra pass leaves room and mode objects at x1. | `the_walk_follows_the_scheduler_and_leaves_out_the_walker_and_masked_groups`, `the_extra_pass_needs_the_live_practice_unpaused_with_no_slow_and_between_passes`, `the_extra_pass_runs_on_every_other_update_and_restarts_after_a_reset`, `the_skipped_tick_is_the_registry_walker`, `the_pass_ticks_every_task_in_the_schedulers_order_and_is_whole`, `each_phase_is_read_again_right_before_its_ticks`, `counting_and_listing_walks_agree_and_a_phase_wider_than_the_buffer_is_not_run` |
| The speed changes ship in rc.8, not in a later release | Paced slot and extra pass shipped together. | `lobby_offers_the_training_speeds`, `two_thirds_paces_periods_of_one_and_two_updates`, `half_and_quarter_are_not_paced_and_two_thirds_yields_to_a_longer_slow` |
| None of it is deferred to rc.9 | R2’s pair also covered R9 teardown at 1/4. | `match_found_at_two_thirds_frees_the_paced_slot_before_the_drop`, `match_found_with_no_busy_edge_frees_the_slot_before_the_drop`, `the_paced_slot_is_kept_through_its_divisors_and_freed_with_the_one_it_holds`, `the_pass_stops_between_ticks_once_the_practice_is_no_longer_live` |

## 5. Original rc.8 test plan and later evidence

| Evidence | Scope |
| --- | --- |
| [Feature ledger, row 82](../FEATURE-LEDGER.md) | Later five-speed timing runs. |
| Original plan | A Quickplay pair (b hosting, a joining), diagnostic build and QCF’s trace points. |

- per cycle (x1, 2/3, x1.5, 1/4): the host's room cadence (`main+0x1564088` hits), the round start and the match
  arrival on both consoles, a clean match (`sync_end … desync_seen=0` at the same frame);
- the fighter's speed: Player 1's frames per second (a trace counter on the plugin's per-fighter frame) against x1;
- match found while holding each speed: T3 (`main+0x1fca5d0`, the match's table has no plugin slot), T5 (destroy before
  create), and the diag lines `[GameSpeed] lobby pass` / `lobby paced` stopping at the session clear;
- R9: the 1/4 match-found teardown row.
