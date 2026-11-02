# Swap replay fixtures (WORKPLAN §1.4, WP2)

One JSONL file per recorded trace; each line is one event the simulator
(`training_mod_consts::swap_model::native_model`) applies. Lines starting with `#` and blank lines are
ignored. Every event carries `"ev"` and may carry `"note"` (the log line it transcribes). Kinds are
decimal in JSON (`0x5a` → `90`); a cell `-1` means empty. `role` is the bank role: `0` = Player
(owner 0), `1` = CPU (owners 1 and 4).

| `ev` | fields | log line it comes from | model effect |
|---|---|---|---|
| `flags` | `role`, `r48`, `r7d`, `r7f` | not logged (D-SG1 §7 inference; `[Ledger] reconcile … r48= r7d= r7f=` once WP2 lands) | `R+0x48`, `R+0x7d`, `R+0x7f` |
| `missing_keys` | `role`, `keys[]` | the `map=` arithmetic (`0x17e2df0` inserts no node for an absent file) | keys that never create a node |
| `window` | `role`, `cells[1..3]`, `actors`, `active` | `[TrioBoundary] reload … incoming=` + `[BodyBank] ready` / `[MiiOrdinary*] ready` of a completed load | window constructed, tails ran, map empty, `active` cell activated |
| `prefetch` | `role`, `group`, `kinds[]` (incl. the object kind) | `[GroupSwap] plan` + `prefetch` + `prefetched … map= tree=` | `0x17e64d0` per kind (generic registration) |
| `build` | `role`, `group` | `[GroupSwap] build` + `built ok=true` + `released object_kind=` | Phase B exchange: group kinds construct into the window (tails), native object release, the bank's post-release |
| `attach` | `role`, `group` | `[GroupSwap] request … state=Built` + `attach` | warm swap window ↔ Built group |
| `activate` | `role`, `cell` | `[BodyBank] reuse owner= from= to=` | in-window member switch (a Kirby becoming live triggers the D-KIRBY1 key return under the recipe) |
| `copy_acquired` | `role`, `kind` | `[KirbyHats] confirmed role= kind=` | live Kirby copies `kind` on its own R |
| `publish_rejected` | `role` | `[MiiOrdinary*] refused … prequeue refused; native epilogue selected` | the next Reload's prequeue refuses |
| `reload` | `role`, `cells[1..3]`, `actors`, `bank` | `[MiiOrdinary*] submit` → `[GroupSwap] teardown` → `[TrioBoundary] reload` (→ `native_return … loader=7` when refused) | teardown, states 1–3 (destroy, tree clear), state 6 (free check, publication, native + supplemental requests) |
| `ready` | `role` | `published` → `ready` / `PreloadMetrics … outcome=ready` | construction tails (pair rule), activation (routes re-pointed), loader 14, INSTALL released |
| `tick` | `frames` | `HW.Memory Unmapped Read64 @ 0x0` / `Read8 @ 0x38` lines | the pad thread's route tick |
| `expect` | `role`, optional `policy` (`old` / `recipe`), `map`, `tree`, `loader`, `install`, `pair_refs` | the `map=`/`tree=`/`loader=`/`INSTALL_active=` values of the cited line | calibration: a mismatch is `Fault::Expectation` |
| `finalize` | `role` | `destroyed … INSTALL_released=true` / `teardown … reason=finalize` | entry destruction (a stranded INSTALL is reported first) |

Policies: `old` = the code before each fix (B123-pre for D-SG1/D-KIRBY1/D-PTA1/D-HANG1, C114 for
SOLO-STALL, B119 for IC); `recipe` = the C116.1 baseline point fixes (D-SG1 orphan key first, D-KIRBY1
defer/return/guard, D-PTA1b `NativeContinue`, D-PTA1c route clearing, C122 `R+0x50 = count`,
SOLO supplemental from cell 2); `recipe+7a` = the recipe plus WP1's `FormClass::Kirby → CellRule::FirstOnly`
(a Kirby is never built as an upcoming member of a window; WP-K `RE-KIRBY-UNLOAD.md`).

| Fixture | Source | old policy must reproduce | recipe | recipe+7a |
|---|---|---|---|---|
| `d-sg1-aegis-orphan` | Surface B123-pre lap 1 `153134-stall79-eden.log` L7760–L8069 (D-SG1-RCA §1) | `map` 7, 15, 9, 1 then `LoaderRefused7 {0x1338}` | none | none |
| `solo-stall-double-link` | `lobby-compat-c114 … 222200-repro.log` #82–#171 (SOLO-STALL-RCA §1–§2) | `LoaderRefused7 {0x7e, 0x84}` at the original Reload | none | none |
| `ic-b119-state11` | IC119-RUNTIME / C122-IC-FIX §1 | `State11Stall` with `R+0x50 = 2` | none (`R+0x50 = 3`) | none |
| `d-kirby1-k1` | Ally B123-pre S2 L7087–L7099 (D-KIRBY1-ROB1-RCA §1) | `Spin { MirrorReleaseUnderLiveCopy }` | `Spin { KirbyCopyListAtNonZeroCell }`: the keys are returned, but the recorded window keeps Kirby at cell 1 (the model's prediction for E4b as recorded; K-R1 decides) | n/a (7a never builds that window) |
| `d-kirby1-k1-7a` | the K1 shape as 7a builds it (Kirby the target at cell 0) | `Spin { MirrorReleaseUnderLiveCopy }` | none | none |
| `d-pta1c-pad-route` | PTA116T s1 #569–#661; D-PTA1BC-FIX §2 | `DanglingRoute` during the Reload, none after `ready` | none | none |
| `d-pta1b-refusal` | Ally S4 L8957 burst; D-PTA1-RCA §3 | `StrandedInstall { loader 6 }` + a permanent `DanglingRoute` | none (`NativeContinue`) | none |
| `d-hang1-s-b`, `d-hang1-r` | `d-hang1-sep` e3/e1 (D-HANG1-SEPARATION §1) | `Spin { KirbyCopyListAtNonZeroCell }` | the same (the baseline does not fix D-HANG1) | none |
| `d-hang1-s-a`, `d-hang1-s-k` | `d-hang1-sep` e2/e6 | none | none | none |
| `defect2-lobby-cpu-aegis-prefetch` | Yoga `b123pre-lobby` 18:11–18:12:52Z (DEFECT #2; raw logs pending) | `LoaderRefused7 {0x1338}` on the CPU owner | none | none |

`keys-13.0.5.json` is the `0x65dd70` table dumped from the owned ELF (`0789f0a4…`) by
`wpB-keys-dump.py` (index `kind-5` at `0x44be510`, 0x71 entries); `ledger::TABLE_BIT1_KINDS` is checked
against it. WP0's `tools/swap_trace.py` is expected to emit this vocabulary from Eden/diag logs; the
hand-transcribed fixtures here cite their lines in `note` and stay as the regression floor.
