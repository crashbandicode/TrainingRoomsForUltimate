# P0 trace results: edge-juggle-p0-d6393cb0-yoga-1 (2026-11-20, Yoga)

- **Build:** Eden 2f315fc4 (muted, GDB stub off, trace.configure + points before game.load), TR release d6393cb0 (build ID
  checked in r02, exact-release.json), tile cursor 66f00442, nro_hook ca7aec03.
- **Run:** Up 01:08:42Z, Down 01:17:55Z. 7/7 rows PASS. Down: `cleanup_failures=[]`, `released=True`; forced stop
  (SIGKILL) after game.stop, not a crash. Ledger CLAIMED 01:08:45Z, RELEASED 01:17:55Z. console.log 4,256 lines and
  eden_log.txt 4,262 lines: 0 matches for panic/abort/crash/unmapped.
- **Trace:** trace/edge-juggle-p0-d6393cb0.trace.jsonl, sha256 6aaf9c57082158903169fbd7db6c04cf0aaff9cd185cc843d5c6ee4d9cd700b6.
  Point 1 (AI frame) 21,143 hits at the counters call, 0 dropped; status 1,289; hits 145.
- **Numbers:** `/usr/bin/python3 -I evidence/tools/analyze_p0.py <trace>` → attempts.txt;
  `aggregate_p0.py attempts.txt` → aggregate.txt.
- **Setup:** P1 Mario on Final Destination at x=-45, firing a right-stick up smash every 0.75 s. CPU Mario, Lv 9: rank 100
  on every record. Training CPU Behavior was CPU (still-003), and every TR CPU override was off. 20 `tr-load` presses
  with a random slot, plus autoload on KO. Starts came from save_states.json.

| CPU start (x, y), 80 % | Loads | First action | Ledge grabbed | Up special early | Air dodge at release | Goal seen |
| --- | --- | --- | --- | --- | --- | --- |
| (-100, -20) left, close | 5 | ESCAPE_AIR 4, JUMP_AERIAL 1 | 5/5 | 0 | 5/5 | (-80, 0) |
| (-115, -50) left, low | 1 | JUMP_AERIAL | 1/1 | 0 | 0 | (-80, 0) |
| (105, -25) right, close | 2 | JUMP_AERIAL | 0/2 (landed on the stage edge) | 0 | 0 | (80, 0) |
| (120, -45) right, low | 3 | JUMP_AERIAL (frame 1) | 3/3 | 0 | 0 | (80, 0) |
| (-40, 70) juggle start, 60 % | 1 | JUMP_AERIAL, then ATTACK_AIR | n/a | 0 | 0 | (-80, 0) |

Findings:
- **R1, confirmed:** the return goal (ai+0x270) is always the nearer ledge point, (±80, 0) on Final Destination. No
  attempt chose a high route or the far side. From (105, -25) the double jump carried Mario over the edge, and he landed
  on the stage beside the ledge. That was not a deliberate high route.
- **Predictable, confirmed:** for the same start, the paths repeat. The three (120, -45) loads pressed jump at identical
  positions (120,-45) → (116,-34) → (112,-28) → … in 2 distinct paths over 3 loads.
- **R2, corrected:** from below the ledge, the CPU jumps on its first actionable frame, not at a fixed lower height.
  Jump timing is immediate and identical every time. The fixed-height thresholds in `r_jump_timing_check` were not the
  deciding factor in these starts.
- **R3, confirmed:** the air dodge toward the ledge is used only from the close start (20 units out), 5/5 at release. It
  was never used from 35+ units out; there the double jump was used.
- **Up special:** no first-approach up special was needed from these starts. One late up special happened, at
  (-85, -51) after a hit (attempt 8). The far/low starts that would force up special, and the danger-avoidance delay,
  are not covered.
- **J1, supported (n = 1 juggle start):** after release at (-40, 70) the CPU double jumped at once and attacked (aerial).
  Its air stick pointed toward P1 on 68 of 70 off-centre frames.
- Incidental hits on stage showed DAMAGE_FLY followed by ESCAPE_AIR (attempts 3 and 9).
- The juggle sample is too small for rates. A dedicated juggle boot (slot 4 only) is the next P0 row.
