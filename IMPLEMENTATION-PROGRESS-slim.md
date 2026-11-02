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
