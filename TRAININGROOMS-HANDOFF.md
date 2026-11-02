# TrainingRoomsForUltimate — handoff

Updated 2026-11-01 (after the first push) by the lead (Claude agent `aa0f7d3da07bfb85c`, Fable 5.1 xhigh, under primary session `63392ddd-6c77-4f46-9cc3-d9fcd0d16793`).

## Operator's answers recorded; phase 3 started — 2026-11-02 ~00:2xZ (lead `0caa72b8`)

- Lead: Claude session `0caa72b8-b70b-4e6c-b4f4-ccb596f8a6e3` under primary `b81ec740`, `claude-opus-5-5` / high
  (transcript check; the operator chose Opus 5.5 high + Sonnet 5.5 high). Watcher `/tmp/trlead-0caa72b8-watch.py`
  PID 2795950 (status `/tmp/trlead-0caa72b8-watch-status.json`), not a repo artifact. Playbook `53fd1f24`.
- `docs/DECISIONS.md`: Decision column + *Decided* lines; new section N1–N10. Open: Q5, B11; to confirm: N3 slot
  sets, N5 stage grid.
- Workers (native Sonnet 5.5 high): renderer (SDI streaks, new screens 18–20, updates 02/03/05/11), docs (draft 9,
  v10, specs), package-1 implementer on `work/implementation` (worktree
  `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-work`, base `9fabada`, target
  `/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-work`; carry-over source stage `m5o`).
- Status for the primary: `/home/intpa/smash-training-lobby/TRAININGROOMS-STATUS.md`.
- Docs worker (Sonnet 5.5 high): `MENU-DRAFT.md` draft 9 (Counts recounted from its own tables: §1 has 111
  visible rows, not 103; new layout 91 with the pilot, 92 without, + 3 hidden Developer rows), proposal v10,
  19 specs + 4 indexes with *Decided* lines and 53 new §9 rows; `SPEC-cpu-02` §5 (no per-hitlag re-roll, Q18 A)
  and `SPEC-cpu-05` (Shield Tilt In/Out opponent-relative, B7) changed to match the answers. Open details it left
  in the specs: dock/undock while the item grid is open, INI rewrite rules for shortcuts (`SPEC-input-04` Q6),
  the item-on-load conversion map (`SPEC-state-04` Q5/Q6), proposed ids `replay_capture`, `respawn_invincibility`.
- Renderer worker (Sonnet 5.5 high): new screens 18 (item grid, handheld), 19 (delay popup), 20 (Developer page,
  three rows from `SPEC-core-01`; no raygun row, the spec does not describe one), 21 (SDI stretch comparison);
  updated 02, 03, 04, 05, 11, 14, 15. SDI chips: `tools/render_sdi_stretch.py` re-runs W9's renderer with the burst
  emitter's scale-Y × 2.0 along the SDI vector (×1.17 from the emitter's own key table was invisible at chip size;
  burst tip at 0.39 of the chip vs 0.36 captured Up); chips on 01, 16, 17 and decisions 17, 18. Remaining gap: the
  render's burst is narrower than the captured arch.
- Next: merge `review/menu-proposal` into `work/implementation`; Switch build of package 1 (campaign toolchain on
  this Surface, `/home/intpa/smash-training-lobby/toolchain/`, when no Eden runs here, or the Yoga / Ally).

### Package 1 checkpoint and a fact correction — 2026-11-02

- `work/implementation` (local only): `39fd9e0` M1–M3 (crate skeleton per the playbook, carried version guard /
  diag ring / seam-guard code from stage `m5o`, `settings.json`, Input Delay, L+R+A, hazard scans), `a3cebdc` M4
  runtime (not yet Switch-compiled), `4138419` merge of `5e34c82`, `d7c6169` import plan. Host gates: `cargo test`
  88 passed, clippy zero warnings, `--no-default-features` OK. Evidence: `CARRY-OVER.md`,
  `IMPLEMENTATION-PROGRESS.md`, `docs/test-provenance/package-1.md` in that worktree.
- Correction (implementer's finding, checked against the card listing `switch-c1162-firstboot/list-TrainingModpack.txt`):
  the fork's settings live in `training_modpack_menu.conf` (41,649 B); `training_modpack.json` is the updater record
  (72 B) and is not imported. S1's decision is unchanged in meaning; specs, plan and the S1 picture corrected.
- Lead decision: unknown settings keys are kept on rewrite (`SPEC-core-04`); retired keys dropped.
- Then `7c596f3` (playbook `model.rs` / `runtime.rs` roots), `57fc1ed` (Training reset hub on the fighter on-init
  wrapper; B8 clear), `81cea59` (cast fix, build record). Switch builds on this Surface with the campaign launcher
  (`RUSTUP_TOOLCHAIN=nightly-2026-02-14`, `toolchain/tools` homes; the 1.98.1 pin fails in cargo-skyline with
  "the option 'Z' is only accepted on the nightly compiler" — open, with the primary): release `6d644384…`
  172,032 B resident, diag `82f60e24…` 270,336 B resident (`nro_size.py`). Card-site scan of the 2026-11-01 MTP
  preflight copy: only the fork's NRO touches our hook sites. `compliance.py`: 4 rows read only `src/*.rs`
  (playbook tool limit, reported), README GIF / build line / HANDOFF rows wait for the RC.
- Eden (Yoga, tr101–tr110): all package-1 rows pass after root fixes D1–D4 (diagnostics file, Switch tick,
  install-line semantics, diag `hub_run`) and the input hook's move to caller-side sites (`e20416a`: note/apply at
  `0x17546e0`/`0x1754720` and `0x175523c`/`0x1755254`; the epilogue `0x1751ce8` is the CPU pilot's `fim_exit`).
  Pilot v0.2.1 coexists in both load orders (tr108/tr109). Current NROs: release `26a5fef0…`, diag `4792c91a…`.
- READY (`e8dbcc9`): push request with the summary picture sent to the primary (`TRAININGROOMS-STATUS.md`). Not
  pushed. Next: packages 2–4 in parallel streams after the order or meanwhile on the same local branch.

### 2026-11-02 ~05:0xZ — autonomous build-out (operator order via the primary)

- Order: build every remaining package end to end (all designed and agreed features, N1–N10, carried fork features,
  fix-ledger rows as the fork lands them), gated and pushed fast-forward; online rows on the Surface + Yoga pair;
  an RC (gates + size statement + provenance matrix + illustrated summary) → add `## RC READY` (RC sha, NRO hashes,
  evidence paths) to TRAININGROOMS-STATUS.md (the primary's watcher fires on it and on a non-empty `## BLOCKED`); the
  review is Codex Astra (high) (the operator replaced Fable); its findings are fixed at the root before the card swap.
  Astra's early package-1 review is running; findings come via the primary.
  Blockers go to `TRAININGROOMS-STATUS.md` "## BLOCKED"; non-root workarounds are parked as operator decisions.
- Streams (Sonnet 5.5 high), rules `/home/intpa/smash-training-lobby/TRAININGROOMS-STREAM-RULES.md`: pkg2 CPU,
  pkg3 save states, pkg4 recording/input, pkg5 menu rendering/layout/assets, pkg6 carry-over (plan first). Each on
  `stream/<name>` in `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-<name>`, target
  `cargo-targets/trainingrooms-<name>`, Yoga folder `/home/intpa/trainingrooms-build-<name>`, Eden labels `s<N>-NN`.
  The lead merges into `work/implementation` at each gate (integration worktree `TrainingRoomsForUltimate-work`,
  lead only) and deletes stream branches at the end.
### 2026-11-02 ~06:xxZ — stream heads (checkpoint)

- `work/implementation` (integration, local, not pushed past `473da26`): `fa8b77d` pkg2 core → `4895ea3` shared Store
  → `5fe6bbf` scene wrapper (cherry-pick). Streams: pkg2 `299bb62` (P2-1…P2-8 host; Eden pending, Surface), pkg3
  `58f6486`+ (S3-1; S3-2/3/5 next; import runtime assigned), pkg4 `5a5d42e` (S4-1…S4-3; S4-4 after pkg6's Mii crate;
  Eden s4 on the Yoga), pkg5 `47d3d61`+ (menu renders in game; S5-2…), pkg6 `8acbbe6` (109 files carried; successor
  at cycle_preload_config / C6 / C7), r1fix (Astra R1–R5). Stream rules add a 3-minute host-fairness wait.
- Docs pushed this phase: spec corrections `b2d1d30` (replay capture default off; charge nulls), card-swap answers
  `7970d94` (`.smd-N.tmp` skipped, exact `developer.ini`, undo U1 open for the operator).
### 2026-11-02 ~04:4xZ — standing push order; package 1 complete on origin

- Operator (via the primary): general push permission until the first version ships to the card — fast-forward
  pushes of gated work only, no force, no tags, no releases, nothing on the card (the card swap keeps its own order).
  Each push is logged with its sha in `/home/intpa/smash-training-lobby/TRAININGROOMS-STATUS.md`.
- Pushed by the lead: `origin/work/implementation` `e8dbcc9..473da26` (fast-forward) = package 1 complete.
### 2026-11-02 — first code push (operator's order, executed by the primary)

- `origin/work/implementation` = `e8dbcc9` (verified with `git ls-remote`): package 1 without the menu model —
  crate, carried code, settings, Input Delay (caller-side sites, coexists with the CPU pilot), Stage Hazards,
  L+R+A + reset hub, Eden tr101–tr110, compliance report.
- Local only (8 commits, `e8dbcc9..473da26`): the review-branch merge, the menu model (`b462d2f`), `docs/BUILD.md`
  (`b895414`, `68a072b`), Eden tr111–tr113 (`473da26`), plus package 2 work after it. Every further code push needs
  its own operator order, asked through the primary with an illustrated summary.
### 2026-11-02 ~04:1xZ — package 1 complete locally; package 2 started

- `work/implementation` HEAD `68a072b` = package 1 complete: menu model `b462d2f` (5 pages + hidden Developer page,
  96 rows, 68 host tests), `docs/BUILD.md` (`b895414`/`68a072b`: the playbook recipe — pin 1.98.1 for host gates,
  cargo-skyline's `skyline-v3` for Switch builds; the Surface's default `~/.rustup` lacks `skyline-v3`; a clean
  checkout reproduced `26a5fef0` byte-identical). Final NROs built on the Yoga (operator 2026-11-02: Surface, Yoga
  and Ally available; never compile beside an Eden): release `434d8176…` 180,224 B, diag `3192a97b…` 282,624 B.
  Yoga build copy `/home/intpa/trainingrooms-build/` (src from `git archive`, own target, `build.ps1`).
- Eden regression tr111/tr112 (Yoga, with the CPU pilot) running; then the push request for `68a072b`.
- Package 2 (CPU, SPEC-cpu-01…07) implementer started on the same branch: P2-1 CPU core → P2-8 frame advantage.
- Storage 2026-11-02: Surface df 253 GB, work target 1.0 GB; Yoga 283 GB, build dir 0.2 GB. A probe target
  (219 MB) deleted with manifest `/tmp/tr-pin-probe-deleted.json`.

### Addendum 2026-11-02 (operator order via the primary): card swap and fixes in both trees

- **Done** = a release candidate with the full gate, batched Eden rows, the playbook's online rows, compliance and an
  illustrated install summary; then the primary swaps the fork for the clean room on the card. Plan and data
  migration: `docs/CARD-SWAP-PLAN.md` (also lists what the clean room needs from the card).
- **Carry-over rule:** a carried file is taken from the newest fork stage that has every ledger fix to it; each
  carried file records its stage and sha256 in `CARRY-OVER.md` on `work/implementation` (package 1 uses `m5o`,
  which is C116.2 as shipped). Before each package closes, its carried files are diffed against the newest stage.

### Two-way fix ledger (fork ⇄ clean room)

| # | Found in | Defect | Fork fix (stage / commit) | Carried file(s) | Clean room | Offered back to the fork |
| --- | --- | --- | --- | --- | --- | --- |
| F1 | fork, C116.2 console (W19, `c1162/c1162-console-triage/TRIAGE.md` Q1) | "Paused 2/3": the notice maps Waiting (unstarted, nothing owed) to Paused when blocked, and its `blocked` counts the preloader's own speculative-helper / hat INSTALL tokens; the 3rd item is the CPU's retired bank | operator 2026-11-02: name the role ("Loaded 2/3 · swap the CPU to load 1 more"); Paused only when a load is truly deferred; the notice's blocked input excludes speculative-helper / hat INSTALL tokens. W19 patching the fork (stage pending) | `src/cycle_preload.rs`, `src/cycle_preload_policy.rs`, `src/character_roster.rs`, `src/character_sources.rs`, `src/native_install.rs`, `src/native_body_bank.rs`, `training_mod_consts/src/body_bank.rs` (package 6; none carried yet) | carry from W19's patched stage when the primary points to it | n/a |
| F2 | fork, C116.2 console (TRIAGE Q2) | hat cycle used the ~3 s canonical Reload: hats were not preloaded first while P1 is Kirby | operator 2026-11-02: hats first while P1 is Kirby (`cycle_preload_policy::merge`); W19 patching the fork | `src/cycle_preload_policy.rs`, `src/cycle_preload.rs` (package 6) | carry from W19's patched stage | n/a |
| F3 | fork, C116.2 console (TRIAGE Q3) | ~1 s whole-game freeze on the first switch into a prefetched, unbuilt group (synchronous build on the game thread, `native_body_bank.rs` group build) | operator 2026-11-02: investigate a background / at-a-pause build; W20 investigating, no change yet | `src/native_body_bank.rs`, `src/cycle_preload*.rs` (package 6) | waits for W20 | n/a |
| F4 | decision B11 (operator 2026-11-02), both trees | ledge options (and throw options) ignore a menu change while the CPU already hangs on the ledge / holds a grab | W19, fork next build M5p (pending) | – (ledge/throw are upstream-origin: clean-room spec path, not carry-over) | `SPEC-cpu-03` / `SPEC-cpu-06` re-roll on change; menu model emits the re-roll event (package 1), CPU package 2 consumes it | n/a (fork implements it itself) |
| R1 | clean room, Astra review of package 1 (`artifacts/trainingrooms-reviews/R1-astra-pkg1/REVIEW.md`) | settings recovery renamed the active file and never wrote the recovered settings back (boot 2 loses them) | fork: different corruption policy (upstream deletes); not applicable | – (clean-room code) | fix stream `r1fix`: boot-time storage transaction + two-boot fs test incl. write failure | n/a |
| R2 | clean room, Astra | native pause never reached the input-delay owner mask | to check: whether the fork's input delay keeps queued input across the native pause (primary) | – | `r1fix`: guarded native pause site → owner mask; pause lane | offered: report to the primary if the fork shows it |
| R3 | clean room, Astra | delay 0 emitted one stale queued record | to check in the fork's input delay (primary) | – | `r1fix`: clear + pass through; SPEC-input-03 reconciled | offered: as R2 |
| R4 | clean room, Astra | developer.ini accepted duplicate keys / oversized / partial reads | n/a (fork has no developer.ini) | – | `r1fix`: fail-closed validation | n/a |
| R5 | clean room, Astra | heap_allocations observed only at boot | n/a | – | `r1fix`: diag post-exercise counter + hook counts; Eden lanes gated on delta 0 | n/a || C1 | clean room (decisions) | the first-boot import must ignore a pre-installed `developer.ini` (only `migration.json` marks it done) | n/a (fork has no import) | – | `SPEC-core-08` requirement + host test | n/a |

Rule: every fork fix to a carried file gets a row and lands in the clean room when that file is carried (or at
once if it is already carried); every clean-room fix to a carried file gets a row and is offered to the fork
through the primary.

## Decisions page refresh (Q18, new B11) — 2026-11-01

- Worker: Claude session `c0296db6-c2d5-4ad1-adec-70d4f39242d8` under primary `b81ec740`. Model gate
  from its own transcript: `claude-sonnet-5-5` / `high` (66/66 assistant records at the check). No
  watcher (single bounded docs turn); no Eden, no build.
- **Q18** now records the fork's answer: stage M5o applies a changed SDI Direction / Shield Tilt at the
  next CPU frame (DI was already read live per hit; evidence `…/refactor/c1162/di-sdi-apply/RESULT.md`,
  `PATCH.patch`). The open question for the clean room is *follow the fork?* Recommendation is now
  **A, follow the fork**; the per-hit SDI roll is kept as option B (it was the recommendation before;
  `SPEC-cpu-02` question 9, `MENU-DRAFT.md` question 18 and `docs/MENU-PROPOSAL.md` v9 still describe
  the pre-M5o fork and were not edited). Picture 18 keeps the timeline; lane labels, the green outline
  and the example rolls changed; one evidence line added under it.
- **B11 (new):** ledge options changed while the CPU is already on the ledge (W11 finding, fork
  unchanged; `SPEC-cpu-03` currently says changes apply on the next grab). Options: (a) re-roll on menu
  change like SDI (recommended), (b) next ledge grab (current). Marked as relevant to both the fork and
  the clean room. Picture `B11-ledge-change.png` (menu change → CPU on ledge → each option).
- `tools/render_decisions.py`: `d_applychange` edited, `d_ledgechange` added; 30 decisions, 30
  pictures. Changed PNGs 18 and B11 were both viewed; all other PNGs are byte-identical.
- Next: the operator answers `docs/DECISIONS.md`; fold Q18 / B11 into `MENU-DRAFT.md` §4 and the
  specs (`SPEC-cpu-02` Q9, `SPEC-cpu-03` §settings change) once decided.

## Illustrated decisions page — 2026-11-01

- Worker: Claude session `62132d68-754d-4266-9bfe-9759abe5e159` under primary `b81ec740`.
  Model gate from its own transcript: `claude-opus-5-5` / `high` (109/109 assistant records before
  the commit). No watcher was armed; this was a single bounded docs turn.
- `docs/DECISIONS.md`: 29 open operator decisions, each with options, a recommendation, a cost and
  one picture (`docs/decisions/*.png`, drawn by the new `tools/render_decisions.py`, which reuses
  `render_menu_mockup.py`). Menu Q1–Q12 and Q14–Q18 (Q18 is recorded as *fork: being fixed in its
  next build; clean room: same rule?*, recommendation: apply at once plus a new SDI roll per
  hitlag), S1 (the five `SPEC-core-08` import details), S2 (corrupt file), B1–B10 (plugin name,
  panic, layout mods, AI-driven P1, playback vs settings, per-fighter state, In/Out convention,
  a batch of 13 behaviour fixes plus 6 kept behaviours, dev-only code, README media). The
  "Already answered" table records where each closed item was answered.
- Pictures reuse only the review crops/renders already in `docs/menu-mockup/captures/` (no new
  game-derived file). All 29 PNGs were viewed. Links: `docs/MENU-PROPOSAL.md` (top and *Decisions
  for you*), `README.md` Status.
- Next: the operator answers `docs/DECISIONS.md`. Fold the answers into `MENU-DRAFT.md` §4 and the
  specs' §10, then re-render.

## Menu proposal v9 — 2026-11-01 (real SDI chips; DI / SDI chips from W9 renders)

- Worker: Claude session `d1ae5b05-ff6e-4452-b66e-8d8ba639b068` under primary `b81ec740`.
  Model gate from its own transcript: `claude-opus-5-5` / `high` (18/18 assistant records at
  the start, 192/192 before the commit). Watcher: scratchpad `/tmp/claude-d1ae5b05/watcher.ps1`,
  PID 2287338, bound to harness PID 2282691, log `ok claude-opus-5-5/high`; not a repo artifact.
- Evidence read (cited, not copied; `C1162` as in v8): `C1162/captures-ally-sdi/RESULT.md` (W8)
  and `C1162/di-sdi-extract/RESULT.md` (W9, sha256 `5145b560…`; it appeared during this task).
- Chips: **DI and SDI chips draw W9's game-derived renders** (one Up still per effect, byte
  copies in `captures/renders/`, W9 manifest `e6ac05dd…`, turned per direction). **Captures
  are the check** on 16: the v8 DI streak crop, plus two new crops in `captures/sdi/` cut from
  W8 `sdiup_f+12` (`34084656…`; centre (595,262), axis 90°) and `sdiin_f+12` (`c4305933…`;
  (463,352), 180°), pixels unchanged, alpha = edge feather × whiteness key (200, 245).
  Boxes are in `index.md`. No W9 `.eff` / texture / mesh / parameter file is in the repo.
- Screens: 01 (SDI row previews Up + In; SDI Strength shown High), 09 (render chips, legend),
  16 (re-laid out: DI and SDI render rows + capture check column, smaller arrow row), new 17
  (SDI grid, layout of 09). 02–08 and 10–15 and the 102 crops, 38 icons and DI crop are
  byte-identical. All four changed PNGs viewed; 280 SVG image refs resolve; no origin words.
- Specs: `SPEC-core-07` §4.6 (W8 SDI look and timing, run-B root cause, DI rotation in motion,
  W9 renders; V7 answered; Q7). `SPEC-cpu-02` question 9 (a changed SDI Direction applies only at
  the next Training reset in the fork; recommendation, not decided). `MENU-DRAFT.md` draft 8
  (question 17 status, new question 18), `docs/MENU-PROPOSAL.md` v9, `PROVENANCE.md` §C note.
- Not done: W9's own statement is "never commit `re/` and `out/`"; per the primary's brief, only
  the two render PNGs are committed as review material. The primary should confirm this.
- No Eden run, no build. Pushed to `origin/review/menu-proposal` (docs-only standing order).

## Menu proposal v8 — 2026-11-01 (DI / SDI visuals identified; DI chips from a real crop)

- Worker: Claude session `97acad8a-5569-4f1e-bd76-027160168778` under primary
  `b81ec740`. Model gate from its own transcript: `claude-opus-5-5` / `high`
  (Documentation row of the routing table). Watcher: a background PowerShell
  loop in this session, polling that transcript's model/effort, bound to the
  harness PID 2227236; not a repository artifact.
- Evidence read (cited, not copied; `C1162` = `artifacts/…/parked-reuse175/refactor/c1162`):
  `C1162/di-sdi-trace/RESULT.md` (static trace, 13.0.5) and
  `C1162/captures-ally/RESULT.md` (Ally Eden, M5l). No `re/` file was copied.
- `specs/SPEC-core-07` §4.6 rewritten: V1 DI `sys_vector` → `P_CmnVector`,
  SDI `sys_hitstop_spark` → `P_CmnHitStopSpark` (two visuals,
  `effect/system/common/ef_common.eff`); V2 one effect rotated to the
  DI-corrected launch angle / SDI vector; V3 emitter colours over greyscale
  masks; run-time steps for a chip; open V4–V7 (parse the `.eff` and emitter
  colours, cost, resident copy, SDI capture). §4.5 U5: the FD mod replaces
  `stage_1`…`stage_4` at stock sizes; the fork's baked grid tile is identical
  with and without it. §3 inputs and §10 Q5 / Q7 updated; `SPEC-cpu-02` §2
  pointer updated.
- Mockup: `tools/crop_menu_captures.py` cuts `captures/di/di-streak.png`
  (137×44) from run A frame +13 (sha256 `4c7efe01…`; centre (738,332), axis
  44.8°, along −45…92, ±22, alpha feather only). `tools/render_menu_mockup.py`
  turns it per direction (`image_rotated`, SVG `rotate` transform). DI chips on
  01 (row preview), 09 and 16; SDI chips are the placeholder ("pending");
  Airdodge / Shield Tilt arrows unchanged. Re-run of the crop tool left the
  102 crops and 38 icons byte-identical. Changed: 01, 09, 16 (PNG + SVG),
  `index.md` (v8 text, DI crop table), `crops.json`. All three PNGs viewed;
  255 SVG image references resolve; no `upstream` / `ours` / `origin` in any SVG.
- Docs: `MENU-DRAFT.md` draft 7 (question 17 (a) unchanged, (b) status),
  `docs/MENU-PROPOSAL.md` v8, `PROVENANCE.md` §C note on the DI crop.
- Next: an SDI capture (V7: check the CPU's head per hitlag frame); then one
  development probe for §4.5 U1–U4 and §4.6 V4–V6.
- No Eden run, no build. Push: docs / mockup / renderer / crop files only,
  pushed to `origin/review/menu-proposal` under the standing order; confirm
  with `git status -sb`.

## Menu proposal v7 — 2026-11-01 (DI / SDI chips from the game's own visual)

- Worker: Claude session `b426fba6-7e69-4490-a135-49457a87f961` under primary
  `b81ec740`. Model gate from its own transcript: `claude-opus-5-5` / `high`
  (all assistant records at the check). No watcher was armed for this
  single-turn, docs-only task.
- Request: the DI and SDI direction chips show the game's own DI / SDI
  direction visual (a blue streak and a white arc), read at run time.
- Asset: **not identified.** Fork M5i and upstream `077a8e43` request no DI /
  SDI effect (all `sys_*` hashes in `src/` checked; `directional_influence.rs`
  and `sdi.rs` only steer the stick). Upstream `di_state.svg` / `sdi_state.svg`
  are black glyphs, not the visual. Candidate names only, from
  `artifacts/enhancements/stage-assets/ParamLabels.csv` (sha256 `ba4a818c…`):
  `sys_direction`, `sys_direction2`, `sys_vector`, `sys_hitstop_spark`.
- Capture: **none usable.** No recorded run set DI / SDI. The upstream
  `photos/mash_trigger_tumble.gif` (640×360; every 6th frame 0–186 and
  every frame 144–175 viewed) shows attack effects only. Per the brief, the screens were **not** changed or
  re-rendered, and no crops were added. `PROVENANCE.md`, `index.md`, both
  tools and all PNG/SVG are untouched.
- Docs: `MENU-DRAFT.md` draft 6 (*Direction chips*, question 17: DI / SDI only,
  Airdodge and Shield Tilt keep arrows), `docs/MENU-PROPOSAL.md` v7,
  `specs/SPEC-core-07` §4.6 (rule, findings, V1–V5, test rows E5/E6, Q7),
  `specs/SPEC-cpu-02` §2 pointer.
- Next: the primary schedules the three-run Eden capture in SPEC-core-07 §4.6
  (DI Up; SDI Up High; DI Out, with Hitbox Visualization off). Then crop under
  `docs/menu-mockup/captures/`, pin it in `index.md`, add a PROVENANCE note and
  re-render 01, 09 and 16. Optional: an effect-hash probe for V1 (a build).
- Push: docs-only, pushed to `origin/review/menu-proposal` under the standing
  order. No Eden run, no build.

## Menu proposal v6 — 2026-11-01 (origin tags off the screens)

- Worker: Claude session `4156eb7d-f07f-4a1a-83c7-2bf3e9e95661` under primary
  `b81ec740`. Model gate from its own transcript: `claude-sonnet-5-5` / `high`
  (28 of 28 assistant records). No watcher was armed for this single-turn,
  docs-only task.
- Change: `tools/render_menu_mockup.py` `render_page` no longer draws the small
  `upstream` / `ours` / `upstream+ours` tag after each row label; the unused `UP`
  colour is gone. The fourth field of each row stays in the data as the internal
  record and is never drawn. Sub-screens, grids and the chip sheet never drew it.
- Re-rendered all 16 screens. Changed: 01, 02, 03, 04, 05, 14, 15 (41 tags: 6,
  11, 4, 5, 6, 5, 4), PNG and SVG. 06–13 and 16 are byte-identical (git shows no
  change). Evidence: a whole-word search finds no `upstream` / `ours` /
  `origin` in any SVG; for each changed screen, deleting the tag `<text>` lines
  from the v5 SVG gives the v6 SVG exactly, and every differing PNG pixel lies
  inside an old tag box. PNGs 01, 04, 06 and 16 were viewed.
- Docs: `docs/MENU-PROPOSAL.md` is v6 (one line that the screens carry no origin
  tags and the *Visual sources* table is the internal record; the legend row for
  the tag removed; the table itself unchanged). `MENU-DRAFT.md` only had its
  pointer line updated; its Origin column is unchanged. `docs/menu-mockup/index.md`
  is regenerated (title v6, one sentence). `PROVENANCE.md` is untouched.
- Push: the commit is pushed to `origin/review/menu-proposal` under the standing
  order for docs-only draft commits on this branch (relayed by primary `b81ec740`);
  confirm with `git status -sb`. No Eden run, no build.

## Menu proposal v5 — 2026-11-01 (operator's review of v4)

- Worker: Claude session `b124dad3-7453-40aa-8f6f-b448f4448014` under primary
  `b81ec740`. Model gate from its own transcript: `claude-opus-5-5` / `high`
  (10 of 10 assistant records). No watcher was armed for this single-turn,
  docs-only task.
- Item grid: `captures/items/icon-00..37.png` are byte copies of the fork's
  `scripts/item-icons/assets/` from the frozen M5k stage. Manifest
  `sources.json` `8fbed7ab…` and all 38 icon hashes are identical in the stage
  and the worktree, and match the manifest. `tools/crop_menu_captures.py`
  re-checks them on every run. The stand-in glyph code (`item_glyph`, `star`,
  `INK`) was removed from the renderer.
- Stage grid: `MENU-DRAFT.md` draft 5 (*Grid pictures*), `docs/MENU-PROPOSAL.md`
  v5 and `specs/SPEC-core-07` §4.5 now say thumbnails are read at run time
  from `ui/replace[_patch]/stage/stage_1/stage_1_<name_id>.bntx` through the
  mod-aware view, never through `arc:/` (vanilla) and never baked. Fork
  evidence: `stage_grid.rs` only toggles the 115 baked `stage_icon_NNN`
  panes; `stage_asset_probe.rs` (feature-gated) lists the 115 `stage_1`
  paths; the ARCropolis 4.1.1 audit is `artifacts/enhancements/arc-stage-assets/REPORT.md`;
  the probe log `plan4-native-08` has `stage_1` at 20,584 B and `stage_2` /
  `stage_3` at 135,272 B. Open: U1–U8 in §4.5 (load route and lock rules,
  `mods:/` semantics, the game-loader entry, binding a BNTX to a pane, which
  slot mods replace, forms, cost, added stages). Question 13 is answered;
  question 16 (fighter portraits and mods) is added and left open.
- Re-rendered: only 11 and 12 (PNG and SVG) changed. The 102 v4 crops and
  screens 01–10 and 13–16 are byte-identical. Both changed PNGs were viewed;
  all 232 SVG image references resolve. No Eden run, no build.
- Not checked: the file list of the operator's Final Destination mod (U5).
  `SPEC-core-07` §1 and the §4.4 roster row still say the portraits are
  decoded from the game's files; the v4 manifest finding (SmashWiki copies)
  contradicts that. Left unchanged as outside this request.

## Menu proposal v4 — 2026-11-01 (operator's review of v3)

- Worker: Claude session `e5bac3f4-4a52-47b9-923f-8a968f5c5505` under primary
  `b81ec740`; model gate from its transcript: `claude-opus-5-5` / `high`
  (6 of 6 assistant records); watcher in the session scratchpad, bound to
  the harness PID, not a repository artifact.
- Feedback handled: the Kirby Hats, Solo Popo and character-items pictograms
  are replaced by one placeholder chip (`chip_placeholder`). Their final
  icons are open until the functionality is settled. The portrait and stage
  grids keep the v3 layout with the real icons and thumbnails, cropped by the
  new `tools/crop_menu_captures.py` from four pinned captures of the fork's
  own grids into `docs/menu-mockup/captures/` (102 crops, `crops.json`).
  Every crop box is in `docs/menu-mockup/index.md`.
- Sources (sha256): m5f `072638-gsp9-p1grid.png` `c0a2e5b4…`, m5h
  `121311-s40-cpugrid.png` `50869bfc…` (Fox and Sora: clean there), fork
  docs `kirby-grid.png` `dda1ba4b…` (ALL cell only; its Ike carries the
  original-CPU tint) and `stage-rotation.png` `84dbcd0e…` (byte-identical in
  the fork worktree and the m5k stage).
- Re-rendered 04 and 10–16 (PNG and SVG); 01–09 are byte-identical. Each
  changed PNG was viewed, and all 200 SVG image references resolve. Text
  that ran past the panel edge was shortened on 10, 13 and 16 (10 and 13
  already overflowed in v3).
- Docs: `MENU-DRAFT.md` draft 4, `docs/MENU-PROPOSAL.md` v4, `PROVENANCE.md`
  §C review-mockup note. Question 13 is restated: the fork commits its grid
  pictures (89 roster, 115 stage, 38 item) and does not build them from the
  player's files, as draft 3 said.
- Not done: the item grid still uses stand-in glyphs (outside this request);
  no push (needs the operator's order).

## Menu proposal v3 — 2026-11-01 (redraft on the operator's request)

- Operator feedback on v2: the mockup lacked the fork's grid and item / stage
  visuals; DI / SDI should be a directional grid of chips; Kirby Hats, Solo Popo
  and character items get pictogram chips drawn after the operator's sketches
  (not pasted).
- Done (this commit, a fork worker of the coordinator session): `MENU-DRAFT.md`
  is draft 3 (new control types `direction grid`, `portrait grid`, `item grid`,
  `stage grid`, `chip`; Airdodge / DI / SDI / Shield Tilt rows; grid rows; item
  count corrected 45 → 38; questions 12–15 added). `docs/MENU-PROPOSAL.md` is
  v3 with a *Visual sources* section (what carries over from the fork with
  file:line, and that the 42 upstream icon SVGs incl. `di_state` / `sdi_state`
  / `air_dodge_dir` / `shield_tilt` / `character_item` are redrawn, never
  copied). `tools/render_menu_mockup.py` now draws PNG and SVG from one
  primitive list; screens 01–16 in `docs/menu-mockup/` (new: 09 direction grid,
  10 character grid, 11 item grid, 12 stage grid, 13 Kirby hats grid, 14 / 15
  the other Fighters & Stages groups, 16 chip review sheet).
- Not done / waiting: the push of this commit (operator's order needed); the
  operator's answers to questions 1–15; `specs/SPEC-cpu-02` and
  `SPEC-core-03` still describe the direction rows as a multi-select list and
  must be amended to the direction grid once the operator approves v3.

## Objective and scope

Clean-room, MIT-licensed reimplementation of the **upstream-origin** features of
the Training Modpack, carrying over as-is everything the fork wrote itself.
Plan of record: `/home/intpa/smash-training-lobby/TRAININGROOMS-PLAN.md`
(phases 0–5). Brief: `/home/intpa/smash-training-lobby/TRAININGROOMS-BRIEF.md`.
Current work: phase 0 (provenance) done; phase 1 (inventory, specs) done in
draft; the phase-2 menu proposal is with the operator; phase 3 package 1 starts
on the operator's menu decisions.

## Repository state (2026-11-01, after the operator-ordered first push)

- Remote `origin` = `https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate.git`
  (private Forgejo repo, created and pushed by the coordinator on the
  operator's order).
- `main` = `3a8117f` "TrainingRoomsForUltimate: license and README"
  (LICENSE + README only), tracking `origin/main`.
- `review/menu-proposal` = `a86e8f1` "Phase 0 for review: menu proposal,
  provenance, specs, mockup screens" (68 files: the whole phase-0/1 tree incl.
  `specs/SPEC-core-08`, the decision notes, `docs/MENU-PROPOSAL.md`,
  `docs/menu-mockup/` and `tools/render_menu_mockup.py`), tracking
  `origin/review/menu-proposal`. The local checkout is on this branch.
- Policy (relayed by the coordinator): local commits are allowed on review and
  feature branches; **no push without the operator's explicit order** (the
  coordinator relays it); no tag, no Switch write. Standing order, relayed
  2026-11-01 by primary `b81ec740`: docs-only draft commits on
  `review/menu-proposal` may be pushed; anything else still needs an order.
- Next branch: `pkg1/skeleton` for implementation package 1, after the
  operator's menu decisions.

## Decisions so far
- MIT copyright line reads `Copyright (c) 2026 intpa` — **decided 2026-11-01:
  the handle.**
- **Storage root decided 2026-11-01:** `sd:/ultimate/TrainingRooms/`; a
  one-time first-boot import of the fork's `training_modpack_menu.conf`,
  `save_states.json`, `setups/`, `mii-presets.json`, `macros/` by copying,
  originals untouched, marker `migration.json`; not migrated: legacy
  TOML/`.conf`, the two INIs (recommended to add), `cycle_preload.ini`
  (regenerated), `replay_captures/`, logs, dev files. Policy and its five open
  points: `specs/SPEC-core-08-storage-root-and-migration.md` (decides
  SPEC-core-01 Q2, SPEC-core-04 Q3, SPEC-input-01 #15, SPEC-state-01 #9). The
  coordinator delivers the menu to the operator as a markdown review doc with
  rendered PNG screens in `docs/menu-mockup/`; `MENU-DRAFT.md` stays the
  source of truth and the operator's answers are folded into it as they come.
- Comparison baseline: the fork's final tree is stage M5i
  (`…/parked-reuse175/refactor/stages/m5i/stage`, content sha `dd766f46…`,
  NRO `17708c2c…`, installed on the Switch 2026-11-01 13:31Z). Byte-identical
  to the worktree `/home/intpa/smash-training-lobby/UltimateTrainingModpack-lobby-parity`
  except Cargo.lock files and HANDOFF.md. The worktree is never edited.
- Upstream baseline: merge base `97cbe9b9c03150c91595ee6f5af4beaaa41f2481`
  (2026-10-07, "Once Per Frame Upgrades (#780)"); upstream HEAD on 2026-11-01
  `077a8e43b11d1f61ce57169156fb82e92e784c79` (2026-10-11, "Add more training
  mode checks (#781)"), differing only in `src/hitbox_visualizer/mod.rs` (+4)
  and `src/training/mod.rs` (+8/−1).
- Playbook followed: `/home/intpa/ssbu-mod-playbook` at `f94a1dd9`.
- Spec writers may read upstream; implementers (later, separate workers) read
  specs only. Both roles are named in every worker brief.
- Workers run on Fable 5.1 (native Agent tool); no OpenCode in this project.

## State

- Upstream clones (read-only, untrusted, never built or run):
  `/home/intpa/smash-training-lobby/upstream-ref/UltimateTrainingModpack-upstream`
  (`main` = `077a8e43`) and `…/UltimateTrainingModpack-upstream-97cbe9b`
  (detached worktree at the merge base).
- `PROVENANCE.md` — per-file / per-feature table, dispositions, cost where
  measured, open questions. Counts: 889 files; 754 carry over as-is, 71
  clean-room (47 mixed + 24 identical), 54 replace (assets), 2 drop, 8 new.
- `docs/provenance/provenance-files.{json,md}` (scan), `feature-groups.{json,md}`
  (grouping), generated by `tools/provenance_scan.py` and `tools/feature_groups.py`.
- `docs/menu-redesign.md` — draft 1 of the five-page menu with dropdowns,
  grouping table (every current row → new page/section/control), variants and
  operator questions. Needs the operator's approval before any menu is built.
- `specs/TEMPLATE.md` — the spec format. Spec writers (each left
  `specs/PROGRESS-<area>.md` and `specs/INDEX-<area>.md`): `cpu` 7 specs
  **done**, `state` 7 **done**, `input` 5 **done**, `core` 7 in progress
  (01–06 on disk, 07 assets inventory and the index pending at this
  checkpoint). Every finished spec has the template's ten sections, no copied
  upstream code, 13.0.5 `main+0x…` sites, fork fixes folded in and named in
  the index, planned host tests and Eden rows, numbered open questions
  (cpu/state/input: about 110 in total, each with a recommendation).
- `docs/resource-costs.md` — **done** (research worker): Table A whole-image
  trail (30 builds, `nro_size.py` on recorded NROs), Table B per
  upstream-origin feature with file:line sources and confidence words,
  Table C fork budget context, gaps and measurement methods, and a list of
  contradictions between sources. Key rows are folded into `PROVENANCE.md` §B.
  Three of its receipts were spot-checked by the lead and matched.
- `docs/provenance/provenance-files.json` regenerated with
  `fork_only_ranges_full` (the table truncates after 12 ranges; the JSON no
  longer does).
- Model gate: transcript `agent-aa0f7d3da07bfb85c.jsonl` shows
  `claude-fable-5-1` / `xhigh`. Watcher: `trainingrooms-watcher.ps1` in the
  session scratchpad
  (`/tmp/claude-1000/-home-intpa-smash-training-lobby-UltimateTrainingModpack-lobby-parity/63392ddd-6c77-4f46-9cc3-d9fcd0d16793/scratchpad/`),
  watcher PID 1724155, bound to harness PID 1237879, log
  `trainingrooms-watcher.log` (first `seen claude-fable-5-1/xhigh` at
  13:52:30Z). Not a repository artifact.
- `docs/implementation-packages.md` — draft 1 of the phase-3 package order
  (package 1 = skeleton + Input Delay, Stage Hazards, L+R+A Reset).
- `MENU-DRAFT.md` — **the menu design authority for the operator's green
  light** (requested by the coordinator): §1 today's inventory (103 visible
  rows + 4 hidden + 3 outside the menu, with control, options, default,
  origin), §2 the proposed five pages in `page | group | item | control |
  options/range | default | origin | note`, §3 rationale and drop/merge list,
  §4 eleven operator questions, §5 reconciliation with the coordinator's
  mockup v1 (adopted items, grouping differences, the option lists v1 got
  wrong, missing rows, per-upstream-item cost). The coordinator renders
  PNG/SVG images of the in-game look from it (the operator wants images, not
  a web page; the interactive v1 page is deleted). Operator update
  2026-11-01: the standalone CPU pilot plugin works online, is on the card and
  has its own menu page — the P1 CPU Control / Level rows left the normal
  layout and are a hidden fallback group shown only when the pilot is absent
  (`MENU-DRAFT.md` §2/§3, open question 11; notes in `specs/INDEX-cpu.md` and
  `specs/INDEX-core.md`). Still waiting on the operator: menu decisions,
  license holder, storage root.
- All four spec areas are **done** (26 specs, 4 indexes, 4 progress files):
  `core` finished SPEC-core-05/06/07 and INDEX-core. Notable: `check.svg`
  carries a Font Awesome Pro commercial-license notice and `m_retnormal.svg`
  is most likely a game texture (both replaced, never copied); upstream
  `077a8e43` gates the L+R+A reset check on `!is_training_mode()` (looks
  inverted; cpu, state and core all flag it); the Infinite-shield parameter
  rewrite lives in upstream's hitbox ATTACK hook (ownership question).

## Not done / not authorized

- No commits, tags, pushes, Switch writes or builds. `df` was 285 GB free.
- Nothing in the fork worktree, the stage or the upstream clones was edited.

## Next commands

1. Operator decisions: `MENU-DRAFT.md` §4 (menu), `PROVENANCE.md` §Open
   questions (license holder, dev-only code, fork media), and the
   "decide first" items each `specs/INDEX-*.md` lists (RNG and per-fighter
   state for the CPU, save-state load latch and trigger-once, storage root
   `TrainingRooms/` vs `TrainingModpack/`, panic policy, settings-file
   corruption policy, chord edge rule).
2. On the operator's order only: first commit of this tree (message from a
   UTF-8 file, `git commit -F`); no remote until the Forgejo order.
3. Phase 3, package 1 (`docs/implementation-packages.md`): implementers who
   read `specs/` only, never `upstream-ref/`; one `CARGO_TARGET_DIR` per
   source copy; `df` ≥ 40 GB; playbook gates one cargo call each.
4. Keep this file current at every checkpoint; the lead's scratchpad watcher
   (PID 1724155) dies with the harness process and is not part of the repo.
