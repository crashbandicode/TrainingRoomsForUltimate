# Guard the Edge and Juggler player start

## Measured cause

On installed card build `51d19bbd` (NRO `280f3a0d`), the Player 1 start write found a hit knockback status or hit freeze in 91 releases; 21 slid after the write. None of 70 releases slid when the write found Player 1 otherwise. In `fg2-g1-yoga-33`, the trace shows Player 1 on the point one update later in Wait, then moving off it. Trace SHA-256: `4c8b475c29ed8e94fddb93d9c01114237a6f2f6c7d04b299818d1fc5c49131cb`. Review passed at 05:47:51Z.

## Placement and bound

Before the start write, watch only if Player 1 is in hit freeze or not in Wait. Between that watched write and Ready, write the same start again while Player 1 is in hit freeze or is in Wait off its point. Leave any other status alone. The count permits at most one existing ready period of rewrites: 60 for Guard the Edge or 45 for Juggler, from `Drill::ready_frames()`. The limit and rule were approved in the tr-primary-2 ledger at 05:45:52Z. No rewrite is held through Ready.

An initial write that finds Player 1 idle follows the existing path. No hook site or native site is added; the natives used are already granted rows.

## Requirement trace

| Operator clause | Design | Test |
| --- | --- | --- |
| "the rewriting is armed only when the start write found P1 in hit freeze or not in Wait" | Observe the status before the start write and arm `settle` only for a watched start. | `hit_player_start_is_rewritten_until_it_holds`, `edge_juggle_settles_a_hit_player_before_ready` |
| "write P1's start again each frame while P1 is in the game's hit freeze, or in Wait and off its point; any other P1 status is left alone" | `PlacedPlayer::rewrite` decides whether `settle_player` repeats the write. | `hit_player_start_is_rewritten_until_it_holds`, `edge_juggle_settles_a_hit_player_before_ready` |
| "no hold through Ready; at most one ready period of rewrites (the existing 60 Guard / 45 Juggler constants, no new constant), then today's behaviour" | Count rewrites against `Drill::ready_frames()` before the Ready transition. | `hit_player_start_is_rewritten_until_it_holds`, `edge_juggle_settles_a_hit_player_before_ready` |

## Limits and proof

The rewrite pinning Player 1 remains unproven until an Eden proof on the built head. A first slide step past the ledge or into a teeter leaves Wait and is not rewritten: `fcd2-guard-1` attempts 62 and 38 each moved to Fall off the left ledge in one update, by 6.4 and 7.9 units respectively. The speed term that survives the original write is not identified.
