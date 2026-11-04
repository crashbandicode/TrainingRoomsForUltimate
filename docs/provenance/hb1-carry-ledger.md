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

## C2 — runtime reader, census, restored rows (session tr-glm-hb1-reader, 2026-11-04)

| Carried fact | Source (commit / doc) | Where it landed |
|---|---|---|
| The three relied `hurtbox_*` guard rows, bytes exactly as before the RC7 removal | removed by `cab6603` ("exclude deferred HB1 from RC hitbox dispatch and guards"); bytes recovered from its parent `719255d`, where they had been generated from the owned 13.0.5 ELF by `tools/gen_hitbox_sites.py` (the generator's three rows restored too) | `src/model/sites_hitbox.rs` (`SITES` 18→21; consts `HITBOX_HURT_{MODULE,GROUPS,PART}_OFFSET`), re-checked against the owned ELF by `core06_rows_match_the_owned_elf` |
| The checked-pointer live reader: type-check every pointer (null/misaligned refused, 0) because Eden answers 0 for an unmapped read | playbook EDEN "Read the results"; the runtime's raw-read pattern (`GameAttacks`) | `model::hitbox::GameHurt` (reads bound to one `load`, host-probed over a fake layout at exact addresses), runtime `stats::mem_load` |
| The windowed diag-line pattern (accumulate, close a window, one formatted line) | `model::hook_stats::Observer` + `Display` (review R5 precedent) | `model::hitbox::HurtCensus`/`HurtWindow`, runtime `stats::CensusBook`/`hurt_census` (diag only), line `[Hitbox] hurt census kind=… parts=… parts_max=… first_pass=… frames=… ticks_min/avg/max=…` |

Not carries but this session's decisions, for the record: `WANT_SCRATCH`
(hit + hurt spheres together, `model::hitbox`); Hitbox's `CROSS_NEEDS` gains
`utility_get_kind` — the census names each line with the fighter's kind
through the already-guarded `utility::get_kind` row (diag-only caller; the
release build calls no native and reads no HitModule; reverting is one
identifier in the census line if the lead prefers an accessor-keyed line).

## C3 — route B wired (session tr-glm-hb1-routeb, 2026-11-04)

| Carried fact | Source (commit / doc) | Where it landed |
|---|---|---|
| Route B: hitbox, grab-box and hurtbox spheres drawn the fork's way, shared-pool `sys_shield`; hurtboxes yellow; refusals counted and logged | operator decision 2026-11-04 ~06:5xZ, `docs/DECISIONS.md` HB1-route, `review/menu-proposal` commit `5ca5e3b` (reverses the 2026-11-03 option A default; route A stays later research) | the runtime dispatch (`frame` constructs `GameHurt` behind `enabled`, both builds), spec help-text line, rows-C C2/C3 |
| The persistent-sphere HandleSet semantics the wiring reuses unchanged: one request per want per pass, refusals counted (`Counts::failed`), a refused want asked again on a later pass once the pool has room, never a respawn loop | `stream/hitbox` `src/model/hitbox.rs` `HandleSet` (proven Eden hb-03; refusals recorded hb-04: 53 asked, 2 refused, `failed=0` after) | already ported in C1 (`model::hitbox`); the Ridley test now pins one attempt per refused want per pass and the recovery |
| The shipped build reads the HitModule only while the setting is On | the C2 release-silence gate, re-formed for the wired route | test `core06_release_reads_the_hit_module_only_while_the_setting_is_on` (source scan) + the built-NRO string check at the gates |
| Help text "On: hitboxes, grab boxes and hurtboxes are drawn."; §8 cost row pending until the Eden census | the brief (task 1), rows-C C4 | `specs/SPEC-core-06-hitbox-visualizer.md` (help-text line, §8 note) |

Not a carry: the runtime read (`mem_load`) moved from the diag `stats` module
to the module level — the same raw read C2 host-probed, now reachable in the
release build behind the same reader type-checks. No new native, no new site;
the three `hurtbox_*` rows stay relied on, never written.
