# HB1 carry ledger — worker C, branch `p23/hb1` (2026-11-04)

What this branch re-implemented, and from where. Nothing was copied as bytes
from another branch: the RC7 R3-2 removal (cab6603 on `stream/slim-rc7-r3`)
took the runtime HB1 reader out of an older architecture, and `work/
implementation` (42b510c) refactored the planner/dispatcher into the model
behind `AttackReader`; the hurtbox side was rebuilt in that architecture.

| Carried fact | Source (commit / doc) | Where it landed |
|---|---|---|
| Draw hurtboxes, yellow; shapes read per fighter per frame on the hitboxes' per-frame entry; cost measured in the diag build; help text | operator decision HB1 (a), 2026-11-02, `docs/DECISIONS.md`, spec update `dcbb2d1` | `src/model/hitbox.rs` (rules + reader + dispatch), `docs/HB1-RENDER-ROUTES.md`, `eden/phase23/rows-C.md` |
| HitModule layout (accessor `+0xb0`; groups `+0x60`, `0x3c`/group, first `+4`, count `+6`; parts `+0x70`, `0x50`/part, status `+0`, offsets `+0x08`/`+0x14`, size `+0x20`, node `+0x28`; total `+0x78`; status 3 off) and the three relied windows (`0x203db10`, `0x469af0`, `0x468490`) | owned ELF, `docs/test-provenance/core-06.md:115-118`; guard bytes as in `stream/slim-phase3-rc6` `sites_hitbox.rs` | layout constants already in `model::hitbox::layout` (historical), re-read by `want_hurtboxes`; the three guard rows are NOT re-added yet — recorded in `docs/HB1-RENDER-ROUTES.md` as required with the wiring |
| Sphere counts per fighter (Steve 19, Luigi 23; hb-03), Ridley's first pass (55 wanted, 53 asked, 2 pool-refused, then `failed=0`; hb-04) | `stream/hitbox` `eden/EDEN-HITBOX.md` (Eden, 2026-11-02) | `core06_ridley_first_pass_completes_in_the_model`, `eden/phase23/rows-C.md` C1/C3 |
| lr02 part geometry (status 0, centres (0.6,0,0)/(2.4,0,0), radius 2.4, bone `0x51a36341b`; group first 0/count 10) | `docs/HB1-LR02-DISPOSITION.md` (stream/slim-phase3-rc6) | test fixtures' part geometry (synthetic tables built from it) |
| Option A negative result and the A/B question going back to the operator | `tmp-slim/phase3-rc6/OPTION-A-DESIGN.md` (2026-11-03) | `docs/HB1-RENDER-ROUTES.md` (no route picked) |
| The RC7 CPU-role lifetime correction the above composition depends on | composition `1769bdd` (ancestor of the baseline), `docs/RC7-R3-1-VALIDATION.md` | reported in `artifacts/phase23/C-REPORT.md`; no code change |

Files this worker changed: `src/model/hitbox.rs`,
`src/runtime/features/hitbox.rs` (comments and the `None` argument only),
`tests/core06_hitbox.rs`, plus new `docs/HB1-RENDER-ROUTES.md`,
`eden/phase23/{rows,trace}-C.md` and this ledger. `sites_hitbox.rs` and
`tests/capabilities.rs` were left untouched (not in this brief's owned list).
