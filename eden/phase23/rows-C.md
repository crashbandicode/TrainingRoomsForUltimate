# Phase 23 Eden rows — worker C, HB1 (prepare only; the primary composes the batch)

Build: the integrated Phase 3 candidate's **diag** build (the cost and pool
numbers are diag lines; the shipped build writes only its install line).
Fighters: Steve (P1, tile-cursor pick) and the CPU fighters named per row.
Ruleset: 1 stock, 1:00, items and hazards off. Evidence per row: the
`[Hitbox] stats` line(s), the named screenshots, `events.log`, `console.log`
(scanned for `panic`/`abort`/`crash`/`Unmapped`).

| # | Row | Setup | Check | Evidence |
|---|---|---|---|---|
| C1 | Part counts per fighter | Training, P1 Steve, CPU Luigi; `hitbox_vis` On from settings fixture | `want_hurt` per 600 passes matches the recorded sphere counts (Steve 19, Luigi 23; hb-03) within the pass's own accounting | diag `[Hitbox] stats` `want_hurt=`, one idle window |
| C2 | Hurtboxes drawn in yellow | Same fight, idle | Every fighter shows its hurtbox capsule set in yellow next to any red hit sphere; colours read against the sphere rule (yellow `1,1,0`) | screenshot after HUD settles; the stats line's `want_hurt` > 0 |
| C3 | Ridley first pass | Training, P1 Steve, CPU **Ridley** (hb-04 route), idle from the first settled frame | Ridley's hurtboxes all drawn on his first pass; `failed` allowed on the first spawn pass only if the refused spheres appear once the pool gets room (hb-04 recorded 53 asked, 2 refused, then `failed=0`); no sphere ever missing while idle afterwards | diag `[Hitbox] stats` line #1 and the following lines; idle screenshot |
| C4 | Per-frame cost | Same boot, idle + one move | `pass_ticks_avg`/`pass_ticks_max` and `want_hit`/`want_hurt` per 600 passes recorded for spec §8 (compare hb-03's 225–231 ticks idle) | the diag stats lines; recorded into SPEC-core-06 §8 by the lead |
| C5 | Shields unchanged | Same boot, shield with ZR then L | While shielding: the plugin's spheres go, the red shield bubble alone shows; no `sys_shield` kill by kind anywhere (the install line and the code keep `sphere_effect_kill` handle-only) | shield screenshots; stats lines show `killed` == our handles only |
| C6 | Grab boxes and Off | Same boot, grab (R) in slow motion; then setting Off | Grab boxes visible at the tether and never piling up; Off restores script effects and requests nothing | grab screenshot; the Off window's stats lines show no requests |

Notes: rows C1–C6 chain on one boot (one fight, state carried between rows).
`failed>0` on any pass after the first stops the row and reports (lead
condition 3). The part-count trace (trace-C.md), if run, feeds C3's expected
count; without it, C3 is judged on the recorded hb-04 shape (55 spheres).
