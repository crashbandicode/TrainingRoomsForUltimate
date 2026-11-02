# Slim stream: code size below the fork's (2026-11-02)

Lead `0caa72b8`; slim = Claude Opus 5.5 high (`claude-opus-5-5`, effort `high` from this agent's own transcript
`…/0caa72b8-…/subagents/agent-acd076f24796f9c6b.jsonl`; one hidden watcher `/tmp/slim-model-watcher.ps1`, model/effort
only, exits with the owner process 1069629). Work tree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-slim`,
branch `stream/slim` from `7bb86fd`, target `cargo-targets/trainingrooms-slim`. Nothing pushed, tagged or written to a card.

Playbook followed: `ae87443` (`git -C ~/ssbu-mod-playbook pull --ff-only`: already up to date; README.md, every file in
docs/, tools/PROVENANCE.md and `tools/check/compliance.py` read in full).

## Phase A

### 1. Size gate

- `tools/sloc.py`: the operator's counter `/home/intpa/smash-training-lobby/tools/sloc3.py` (sha256
  `b569131d2f25ec901f7f71ff1ed920a6fc9147161f28b5c8e82a6ba6780de6b4`), counting rules unchanged; `third_party/` excluded
  and reported on its own line; `tree` mode (tracked + untracked-not-ignored files of the working tree); depth 0 = per
  file; importable. Check: `python3 -I tools/sloc.py git . 7bb86fd 1` → 106,983, the operator's number.
- `tools/size_gate.py` + `size-budget.toml`: append-only `[[budget]]` entries; the first ceiling of a scope and every
  raise need `approved_by`; a cut needs a reason; exit 1 over, 2 on a rule break. `tools/test_size_gate.py` (5 tests).
- `tests/size_budget.rs`: runs both under `cargo test`. Choice: shell out to `/usr/bin/python3` (as `tests/eden_scripts.rs`
  does) so the count is the operator's counter itself, not a second Rust implementation that could drift; no skip
  switch (a skippable gate is not a gate); a missing Python 3.11+ fails the test.
- Start budget 114,100 = 106,983 measured at `7bb86fd` + 7,061 pending (each unmerged stream: `tools/sloc.py` tree count
  of its work tree minus its merge base, ~12:10Z) rounded up to the next hundred.
- docs/BUILD.md lists the size gate with the host gates.

### 2. Lint allows (lead, 2026-11-02 ~12:0xZ)

Removed at the root: pkg2's six `too_many_arguments` (`model::cpu::mash::Step` bundles the six things every body rule
step works with; `hide_frame` and `fast_fall_frame` read `FrameFacts`; `ledge::TermQuery` is the transition-term hook's
question; the redundant `playback_running` argument of the fast fall is the step context's playback), pkg5's
`too_many_arguments` on `panes::text` (the pane box `[x, y, w, h]` is one argument of `rect` and `text`), pkg3's
`derivable_impls` on `Listing` (the hand impl stays: a derive would add `S: Default`; recorded edit (7) in CARRY-OVER.md),
`boot.rs` `unused_mut` (only the diag build mutates `state_found`: a cfg-gated rebinding). `tests/lint_allows.rs` fails on
any new suppression and on a stale row; its `KNOWN` table holds the 13 remaining carried rows (pkg4/pkg6 manifests, D9),
none with an operator approval; they go to the lead (pkg6's files: ask before touching).

### 3. Reduction plan

`docs/SIZE-PLAN.md` (five read-only audit workers, one per area, clean-room paths only; their findings checked
against the tree where the plan relies on them: the never-compiled files with `/tmp/slim_orphans.py`, the test-file
miscount by the `#[cfg(test)] mod` lines).

### 4. Safe wins

Streams' touched files (each stream branch's diff from its merge base, plus its work tree's uncommitted files,
12:0xZ): none of the files changed here is touched by a running stream, apart from `src/runtime/boot.rs` (hitbox,
u4b, deluxe, stack, sync edit other hunks; the change here is two lines at the state-table compare) and
`CARRY-OVER.md` (one row of pkg3's table). Commits: `f4617c7` size gate, `149cbee` lint allows, `b0140f2` one
crc32/hash40 and one constant table; rebased onto `566780a` at the lead's request (clean), then the hos-lock rows
(stream/sync's two `declare_interior_mutable_const` allows: lock_api's traits require `const INIT`) added to
`tests/lint_allows.rs`, and the budget ratcheted to 114,000 (107,341 at the stream head).

### 5. Lead rulings (2026-11-02 ~12:3xZ) and what followed

- Never-compiled carried files: `e79019b` deletes `automation_helper/native.rs`, `automation_helper/plugin.rs`,
  `roster/effect_hook_chain.rs` (556 lines; pkg6 manifest `not_carried`, three ledger rows, CARRY-OVER.md "slim");
  pkg6 asked to keep the three Mii adapter files (its `mii_fullconfig_probe` reaches them); deluxe deletes
  `capture_plugin.rs` itself (`166d3cd`). `carry_pkg6.py verify --dest-only`: verified 258 files. A mistaken
  relative-path write while editing the ledger created `docs/provenance/pkg6-carry-ledger.md` (1 byte, untracked) in
  the main work tree `TrainingRoomsForUltimate`; found at once by `git status` there and removed (status clean again).
- Layout-only reductions ruled out; no fmt gate now. The tree is not rustfmt-formatted (444 of 508 files).
- Three numbers per milestone (`4ad43c8`) at `e79019b`: 106,785 as measured / 103,047 test-fixed / 131,781
  rustfmt-normalized + test-fixed. Superseded the same day: `tools/sloc.py` follows the primary's sloc4.py (sha256
  `6dcb208c…`), `--normalize` dropped; sloc4 counts `8b73694` 106,099, stream/slim `eb8e034` 103,497; budget
  re-based to 103,500.
- hos-lock's two allows: pending the operator (kept in KNOWN). The practice_stage_request double init goes to pkg6.

## Gates

(open: queued on the Surface; the lead's order: integrator-3 → pkg6 rows → stack → harness → deluxe → f6 → slim)

## Checklist (docs/CHECKLIST.md)

(filled at READY)

## Handover (2026-11-02 ~14:1xZ, operator order: lead role to a Codex GPT-6 Astra lead)

- **Where:** branch `stream/slim`, worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-slim`, head = the
  "WIP: handover to Astra lead" commit (on `cf6004d`, rebased onto `work/implementation` `8b73694`). Target
  `/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-slim` was never created (no build ran), so there is
  nothing to delete. No ledger claim held (none was ever written); the Surface queue line is deleted.
- **Counts (sloc4, `python3 -I tools/sloc.py git . <rev> 1`):** base `8b73694` 106,099; head 103,497 (−2,602);
  host-only simulator 1,118 (not counted). Fork M5q by sloc4: 106,629 (primary). Target ≤ 90,000 at RC, stretch ≤ 85,300.
- **Budget:** `size-budget.toml` last entry 107,900 = head + pending line items (deluxe e4432ad +83, hitbox eca0f81
  +1,285, ui-screens 237b857 +1,062 + ~1,500 estimated, pkg4 2d25cdf +219, pkg6 d517dab +169); integrator-3
  lowers it to the actual count after each merge.
- **Nothing in Phase A or B is compiled or gated.** The Surface queue never reached slim. Every commit since
  `7bb86fd` is uncompiled. The changes most likely to need a fix at the first build are: `model::cpu::mash::Step`;
  `panes` `[x, y, w, h]` const-fn parameters; `game_constants!` (cpu/state ids); `state_boot` `far!` macro and `boot.rs`
  rebinding (Switch-only: needs the Switch release and diag builds); `threads.rs` `slots!` arms (Switch-only);
  `native_seams` re-export of `guard`; the smash-match-data oracle crate (`crates/smash-match-data/tests/oracle`, a
  dev-dependency; new `Cargo.lock` entry); the golden test `the_published_capture_matches_the_golden_files_and_a_plain_reader`
  (its files `tests/fixtures/smd-golden/` do not exist yet: run it once with `SMD_GOLDEN_BLESS=1`, check the two files
  are deterministic with a second run without it, commit them).
- **Proven:** carry tools: `python3 -I tools/carry_pkg6.py verify` and `python3 -I tools/carry_pkg4.py verify` re-derive every
  carried file from its source (261 / 42 files); `tools/test_size_gate.py` 6 tests pass; `tools/seam_census.py` evidence
  in docs/SIZE-PLAN.md "Seam rows". No Eden row ran; the seam install-line row is owed.
- **Next commands** (Surface, after a claim at the head of the queue; `CARGO_INCREMENTAL=0`, own target, each under
  `systemd-run --user --scope -p MemoryMax=6G /usr/bin/timeout 1800`; helper scripts `/tmp/slim/cg-inner.ps1`,
  `/tmp/slim/sw-inner.ps1`): `cargo test` (with `SMD_GOLDEN_BLESS=1` once, then without), `cargo clippy --all-targets -- -D
  warnings`, `cargo build --no-default-features`, `cargo test --manifest-path crates/smash-match-data/Cargo.toml`,
  `cargo test --manifest-path crates/hos-lock/Cargo.toml`, `cargo skyline build --release` and `--features diag`
  (docs/BUILD.md), `nro_size.py`; then one Eden boot row for the install line (`seams=`, `seams_unchecked=`,
  `result=installed`, nothing missing) per the playbook harness; then `compliance.py` and this file's checklist.
- **Approved, not done:** pkg6's probe list (lead 2026-11-02): delete the 8 empty features (console-map-probe,
  archive-poison-probe, kirby-resource-probe, lobby-stage-probe, practice-stage-observer, stage-asset-probe,
  mii-preview-observer, mii-quit-observer; some have only seam-row `feature:` strings and `tests/pkg6_features.rs` lines),
  the source-capacity family (10 features, `source_capacity_probe.rs`, `source_ldn_capacity_probe.rs`, ~280 inline
  sites), the Mii probe family (7 features: identity, saved-file, fullconfig, native-/cpu-adapter with
  `mii_native_adapter.rs`/`mii_cpu_adapter.rs`/`mii_closed_ack.rs`, identical-reload, special-change; 91 inline sites) and
  `texture-probe` (`runtime/ui/texture_probe.rs`, `ui/mod.rs`, `ui/tick.rs`). Each gets a "not carried: probe served
  (pkg6 confirmation, 2026-11-02)" ledger/CARRY-OVER row; the features leave Cargo.toml and `tools/feature_matrix.py`.
  pkg6-A chose option (1): the slim owner removes the inline sites itself through pkg6 manifest edits; pkg6's
  committed work touches `character_roster.rs`, `character_sources.rs`, `kirby_hats.rs`, `cycle_preload*.rs`,
  `native_source.rs`, `mii_cycle.rs` (not `native_source_capacity.rs`, `native_source_actors.rs` or the Mii
  ordinary/native files). Keep: plugin-heap-meter, native-unlink-probe, hang-trace, console-practice-trace,
  console-swap-trace, seam-drill, simulator. Observers (mii-pool, mii-origin, source-owner, source-lifecycle,
  native-body-bank-probe, named-input-trace): delete if nothing shipped or tested needs them. Rename `lobby_stage_probe.rs`
  and `practice_stage_observer.rs` (they ship) as recorded edits.
- **Other open items:** hos-lock's two `declare_interior_mutable_const` allows wait for the operator (KNOWN rows);
  `src/runtime/roster.rs:13` comment still names `effect_hook_chain`; `practice_stage_request::init` double init and its
  false `ui_wired=true` are pkg6's; the rest of docs/SIZE-PLAN.md §2 (roster/Mii slices after pkg6's fixes merge).
- **Watcher:** the model/effort watcher (`/tmp/slim-model-watcher.ps1`) is stopped with this handover; last check
  claude-opus-5-5 / high.
