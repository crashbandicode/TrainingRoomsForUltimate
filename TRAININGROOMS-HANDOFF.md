# TrainingRoomsForUltimate — handoff

Updated 2026-11-01 (after the first push) by the lead (Claude agent `aa0f7d3da07bfb85c`, Fable 5.1 xhigh, under primary session `63392ddd-6c77-4f46-9cc3-d9fcd0d16793`).

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
  one-time first-boot import of the fork's `training_modpack.json`,
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
