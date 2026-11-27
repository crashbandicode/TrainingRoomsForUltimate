# Guard the Edge and Juggler: Player 1's start

## Measured cause

On release `83eecc32`, Sonic's 26 Guard the Edge releases used the last point of Final Destination's main floor,
x −79.9856. The re-read is
`/home/intpa/smash-training-lobby/artifacts/phase23/edge-juggle/evidence/cp1a-sonic-83ee-1/p1-before-release.txt`.

| Player 1 at the write | Releases | What followed |
| --- | ---: | --- |
| Edge-teeter wait | 19 | All teeter, at x −79.0256 |
| Hanging on the ledge | 2 | Both teeter |
| Landing (0x16) or status 0x50 | 2 | Both teeter |
| Wait inside at x −76.79 | 1 | Teeter |
| Wait inside at x −75.13 | 1 | Fall requested at x −81.9956 two frames into Ready, then ledge hang |
| First attempt, A came through as a jab on the point | 1 | Fall requested one frame into Ready, then ledge hang |

Both falls came after Ready started. What makes the game choose Fall was not recorded: facing and speed are absent.
The Lucas traces in `docs/DECISIONS.md` GJ4 instead support the inference of a Player 1 KO: Standby was requested,
then Fall at (0, 150), and the next release showed Player 1 falling. In Training a KO passes through Standby and the
rebirth platform through Rebirth.

### The teeter range (2026-11-23 rework)

The game's `GroundModule::is_ottotto` checks the floor end the fighter faces at 0.86 + 0.1 + 0.1 = 1.06 units.
Within that range it moves the fighter to 0.96 from the end. In fix-build flight `755d9936`, both hosts wrote
x −78.9856 and rested at −79.025604 (−79.9856 + 0.96), status 126 in Training and 127 in the waiting room.

## Rule

Placement waits while Player 1 is Dead, Standby or Rebirth. A restart wakes the CPU if this placement held it asleep.
Player 1's damage remains unchanged. A Guard the Edge Player point is clamped 1.25 units inside each main-floor end; if
the floor is narrower than 2.5 units, use its midpoint. Juggler's point is unchanged. The wizard and the placement
use the same rule.

## Requirement trace

Clauses are the decided GJ4 of `docs/DECISIONS.md` (2026-11-22); the item was ordered into the fix build on 2026-11-23.

| Requirement clause | Design | Test |
| --- | --- | --- |
| The next attempt waits for a KO'd Player 1 | KO status restarts placement, waking a held CPU | `placement_waits_for_a_knocked_out_player`, `edge_juggle_waits_for_a_knocked_out_player` |
| Player 1 starts where the wizard shows it | Guard the Edge uses the same 1.25-unit inset for cursor and placement, with a midpoint for a narrow floor | `guard_player_point_stops_outside_the_teeter_range_of_each_floor_end`, `a_main_floor_narrower_than_two_insets_keeps_the_player_on_its_middle`, `a_crosshair_below_a_platform_finds_the_floor_under_it` |
| The Preview prints where Player 1 will rest, not (−80, 0) | Player and Preview use the Player point after the cursor's inset | `guard_player_point_stops_outside_the_teeter_range_of_each_floor_end` |

## Limits and proof

- The 1.06 range is read from the game's code; the 1.25 inset is not yet flown. The fix boot's rows must read
  status 0 and the written x at each release on both ends.
- The edge-teeter start is no longer selectable. Juggler's point is not clamped.
- A KO during Ready, after the write, is not handled. Player 1's damage between attempts is left alone.
