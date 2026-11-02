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

## Astra continuation — 2026-11-02

Thread 01a19c9f-18c1-7832-9236-a14dcebafa3b; live rollout confirms gpt-6-astra / medium.
No subdelegation (worker brief forbids it). Owner-bound watcher PID 2608818, owner Codex PID 2589455,
script /tmp/slim-watch-01a19c9f-18c1-7832-9236-a14dcebafa3b.py; model/effort only.
Playbook followed: ae87443cbb801472e474b3f0c20b5f9c3f5f1f77 (ff-only pull already current).
Read README, all seven docs files, tools/PROVENANCE.md personally; tool reads recorded as used.
Starting HEAD 2e09317ab57a0488785bbabea5d524e97067fa63, stream/slim, this worktree.
Authorized one-time merge of ebfcb9800fba1f82b2730b19ed26e231c810a2d6:
only conflict was generated pkg4 ledger; regenerated using merged manifest.
tools/carry_pkg4.py verify --dest-only: 42 verified. No source conflict resolution.
No claims, builds, Eden runs, profile changes, targets, pushes or card writes in this continuation yet.
Next: finish relevant spec reads, host admission, golden blessing + inspect + unblessed repeat,
full host gates, Switch release/diag, then approved probe retirement and follow-up gates.
Roster/Mii refactors require pkg6 baselines and integrated fixes first; empty KNOWN still required.

### Current M/C acceptance checklist

Every mandatory and conditional bullet from the followed playbook is listed below.
Unverified is an explicit pending state; before READY each must have the prescribed final result and evidence.

| Requirement | Tag | Result | Evidence |
| --- | --- | --- | --- |
| `crate-type = ["cdylib"]` only. An `rlib` alongside makes Cargo skip LTO for the crate without saying so, and its code stays in the resident image the game's heap pays for. Shaderdb-probe went from 1,081,344 to 618,496 bytes resident with the same exports. Evidence: `Cargo.toml` and `tools/build/nro_size.py` on the shipped NRO. | M | unverified | Acceptance pending; no pass claimed. |
| Release profile `opt-level = "s"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`; `skyline = "=0.6.0"`; toolchain `1.98.1`. | M | unverified | Acceptance pending; no pass claimed. |
| Pure logic in `model.rs` with host tests; `runtime.rs` only under `#[cfg(target_os = "switch")]`. | M | unverified | Acceptance pending; no pass claimed. |
| Gates pass, one cargo call each: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo build --no-default-features`, `cargo skyline build --release`, and the diag build on its own. Evidence: the outputs. | M | unverified | Acceptance pending; no pass claimed. |
| No heap in hooks: a counting allocator and `heap_allocations=0` in the install line. | M | unverified | Acceptance pending; no pass claimed. |
| Fixed buffers, a writer thread on a static stack, files opened from stack-built paths. **E** gsp-visualizer, ssbu-gnw-judge-log. | M | unverified | Acceptance pending; no pass claimed. |
| Every fix solves the root cause. Otherwise the workaround (guard, skip, restriction, tolerance, lint `allow`, feature gate, setting off) went to the operator with its root cause, cost and a recommendation, and was approved before it shipped. The report lists each approved workaround with its date ([WORKFLOW](WORKFLOW.md), "Root cause, or the operator's approval"). Defects that share one cause were checked for a single abstraction, and the report says which approach was chosen and why. | M | unverified | Acceptance pending; no pass claimed. |
| Version guard: the game reads `13.0.5` or nothing is installed and the log says `result=unsupported_version`. | M | unverified | Acceptance pending; no pass claimed. |
| 64-byte guard on every hooked site, every called function and every relied-on instruction; a mismatch leaves that site alone and the log names it. | M | unverified | Acceptance pending; no pass claimed. |
| Card scan before the site is chosen: `tools/build/scan-offsets.py` over the card copy, for the site and, for a virtual function, its vtable slot. Evidence: the scan's output and the copy's date. | M | unverified | Acceptance pending; no pass claimed. |
| (another plugin hooks or may hook the site) Sharing v1 or the chain rules ([HOOK-SHARING](HOOK-SHARING.md)); the owner election runs before the byte guard; both stand-in modes (`foreign=inline`, `foreign=far`) and both load orders tested in Eden. N/A only when the scan showed nobody at the site, with the scan cited. | C | unverified | Acceptance pending; no pass claimed. |
| Hook after the prologue, or call the game function instead of hooking it; hook the most specific instruction. | M | unverified | Acceptance pending; no pass claimed. |
| No diag code or strings in the shipped NRO (`diag` feature off; no `diag.txt` or `standin` strings). Evidence: `compliance.py`. | M | unverified | Acceptance pending; no pass claimed. |
| The shipped NRO writes its install line and nothing per frame. **C** (a logging plugin whose product is the log) its data files too, but no per-frame diagnostics. | M | unverified | Acceptance pending; no pass claimed. |
| Read-only rules reviewed in the code, hook by hook: no checked random number drawn, no native code that draws one skipped or reordered, no simulated state written (ssbu-desync-detector `docs/WHAT-IS-CHECKED.md`). Evidence: the review in the evidence doc. | C | unverified | Acceptance pending; no pass claimed. |
| Replay A/B with the desync detector: the same replay with and without the plugin, identical check values and positions. | C | unverified | Acceptance pending; no pass claimed. |
| Online-only paths: two consoles, the detector on both, the changed path played to its end so the session after it is checked; `desync_seen=0` at the same `frame=` on both. | C | unverified | Acceptance pending; no pass claimed. |
| Claim written before `Up` and released after `Down`, also after a failed run. Evidence: the coordination file's lines. | M | unverified | Acceptance pending; no pass claimed. |
| One Eden per host, MemAvailable at least 10 GiB, never profile `a`, display `:61` or another agent's runs. | M | unverified | Acceptance pending; no pass claimed. |
| The profile is a copy with the game content hard-linked (`gv.py prep`, or `cp -al`); a full copy per profile fills the filesystem. Only the copy is changed. Byte-identical files are links, and files that differ or that the run writes in place (saves, rewritten INI/config, shader caches, logs) are real copies ([EDEN](EDEN.md), "Link what is identical, copy what is not"). Evidence: `du -sh` against `du -sh --apparent-size` on the copy. | M | unverified | Acceptance pending; no pass claimed. |
| One `CARGO_TARGET_DIR` per source copy, never shared between copies of the same workspace, and the cargo home shared per host and toolchain; sources refreshed with new file times; `df -h /` checked before every build or Eden launch, stopping below 40 GB free; finished work's build products deleted, with a manifest, while NROs, receipts and logs are kept ([WORKFLOW](WORKFLOW.md), "Build caches and disk"). | M | unverified | Acceptance pending; no pass claimed. |
| Every wait gated on a log line or a screenshot check, between macro segments too ([MACROS](MACROS.md), "Hybrid"); a fixed sleep only where neither exists, and then rounded up. | M | unverified | Acceptance pending; no pass claimed. |
| A run whose precondition line never comes is aborted and retried under the next label ([EDEN](EDEN.md), "Queue runs"). | M | unverified | Acceptance pending; no pass claimed. |
| The ruleset is the shortest that covers the path under test: 1 stock, a 1:00 limit, items, hazards and FS meter off ([EDEN](EDEN.md), "Keep matches short"). A longer match only when the test needs it, with the reason in the handoff. Evidence: the ruleset screenshot or the match length in the log. | M | unverified | Acceptance pending; no pass claimed. |
| Profile restored after the run: set-aside NROs back, saves and replays back, temp files removed, the host released. | M | unverified | Acceptance pending; no pass claimed. |
| `console.log` searched for `panic`, `abort`, `crash` and `Unmapped`; an `Unmapped Read`/`Unmapped Write` below `0x1000` or inside the plugin's activity window fails the lane, because Eden tolerates the null read the console faults on ([EDEN](EDEN.md), "Read the results"). `run.json` and `memory.peak` kept with the run. | M | unverified | Acceptance pending; no pass claimed. |
| Every local command under the memory and time guard; anything with `${var}` in a script file. | M | unverified | Acceptance pending; no pass claimed. |
| Each run's outputs kept under its label. | M | unverified | Acceptance pending; no pass claimed. |
| README: one paragraph, a full-screen GIF (1280 wide, under 10 s, caption with date and build), the install path, the build commands, a license line and the AI-assistance disclaimer. | M | unverified | Acceptance pending; no pass claimed. |
| A dated evidence doc: run labels, the build (commit or NRO sha256), what was done, what the log showed. | M | unverified | Acceptance pending; no pass claimed. |
| (the evidence cites numbers computed from logs) every such number comes from a committed script over the logged files, with host tests, and the command is named next to the numbers. | C | unverified | Acceptance pending; no pass claimed. |
| No operator quotes, no claims beyond the tests. | M | unverified | Acceptance pending; no pass claimed. |
| The handoff file current at each checkpoint: branch, commit, evidence, live state, next commands; no secrets. | M | unverified | Acceptance pending; no pass claimed. |
| Minor version bumped, annotated tag, Forgejo release with the NRO, `manifest.json` and `SHA256SUMS.txt`; every asset verified by anonymous download. | C | unverified | Acceptance pending; no pass claimed. |
| A rebuild of the tagged commit is byte-identical to the released NRO. | C | unverified | Acceptance pending; no pass claimed. |
| Install over DBI MTP (or the mounted card with `upgrade2-ums.py`; a swap of one plugin for another with `swap-ums.py`; Skyline's `exefs/subsdk9` with `replace-exefs.py`): preflight (card listing, local copies, `old` sha256 matches), a rollback copy on the card, readback of the new NRO, every other plugin and the protected files; receipts kept; a phase with an existing receipt is not re-run. | C | unverified | Acceptance pending; no pass claimed. |
| The user connects and disconnects the Switch. The agent never mounted, unmounted, ejected or remounted the card and never started or stopped DBI or hekate UMS; it read and wrote the SD only through what the user exposed (MTP, or the mount point), and its report told the user when the card could be disconnected. `compliance.py` fails scripts that carry a mount, unmount, eject or power-off command. | M | unverified | Acceptance pending; no pass claimed. |
| Without the order: nothing pushed, tagged or written to the card, and the report says so. | M | unverified | Acceptance pending; no pass claimed. |
| Forgejo, never GitHub; push only on the operator's order; commit messages from a UTF-8 file. | M | unverified | Acceptance pending; no pass claimed. |
| The model and effort gate and the per-thread watcher follow the routing authority (`/home/intpa/.codex/DELEGATION_ROUTING.md`); they are not restated here. | M | unverified | Acceptance pending; no pass claimed. |
| Every spawned worker read this playbook itself (`README.md`, all of `docs/`, `tools/PROVENANCE.md`) before changing code, built its own checklist and ran `tools/check/compliance.py` before READY; the brief that spawned it said so and recorded the playbook commit it followed. | M | unverified | Acceptance pending; no pass claimed. |

### Restart checkpoint — first compile, 2026-11-02 14:26Z

HEAD e237f0f on stream/slim follows authorized merge dae0787. Fixes are UNCOMPILED.
First cargo test failed with exit=101 before any tests ran:
error[E0609]: no field `runs` on type `&ThreadStack`
  --> tests/hook_locks.rs:69:16
warning: unused import: `crc32` at src/model/cpu/hash.rs:98.
Exact raw output: /tmp/slim/astra-host-first.log. No golden fixture generated.

e237f0f preserves the three previous diag exclusions using stable ThreadId values;
the sole crc32 test caller now imports its canonical owner directly.
No added lint suppression, lock exemption, or behavior change intended.

Operational state: CLAIMED 14:25Z / RELEASED 14:26Z on Surface, Codex 01a19c9f/slim,
in the Yoga ledger. Queue line removed. No held claims, compiler, Eden or profile changes.
No push, tag, release or card access. Target trainingrooms-slim exists and is retained for continuing work.
Prebuild disk 245G free, aggregate targets 16G. Watcher 2608818 stopped/exited at handoff.
Shared harness ledger.py and params.py read completely and reused read-only from
/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-harness/eden/harness.
No harness merge and no new admission tool.

Next:
1. Verify exact thread model/effort and rearm an owner-bound watcher.
2. Finish applicable spec/index reads before further reductions. Read so far: brief, operating rules,
handover, complete playbook README/docs/PROVENANCE, BUILD, DECISIONS, MENU-DRAFT via ranges,
implementation-packages, SIZE-PLAN, IMPLEMENTATION-PROGRESS, CARRY-OVER via ranges
(recheck stack/pkg6 C7 range if uncertain), TEMPLATE, SPEC-core-01, PACKAGE-6-PLAN,
both carry tools and model/runtime module roots.
3. Requeue Surface for one cargo test. Honor 3-minute release fairness and parallel owners.
Use TR_HOST=surface, TR_OWNER="Codex 01a19c9f/slim", TR_PREFIX=sl, TR_STREAM=slim.
Bounded helper: /usr/bin/python3 -I <harness>/ledger.py free, queue, claim, release.
Check disk, target totals and live processes before claiming.
4. SMD_GOLDEN_BLESS=1; systemd-run --user --scope -p MemoryMax=6G /usr/bin/timeout 1800
/usr/bin/pwsh -NoLogo -NoProfile -NonInteractive -File /tmp/slim/cg-inner.ps1
-Args1 "test" -Log /tmp/slim/astra-host-second.log
Release immediately and remove queue row even on failure.
cg-inner.ps1 records exit=N but does not propagate cargo exit: inspect log.
sw-inner.ps1 can copy a stale NRO on failure: never accept without successful compile evidence.
Both existing wrappers were read completely.
5. Inspect blessed fixtures; separately admitted unblessed repeat, then clippy, no-default,
SMD/hos-lock tests, Switch release/diag, size and seam Eden gates.
6. Only after prior edits are gated, retire approved probes through manifests and regenerate ledgers.
Roster/Mii refactors require pkg6 baseline rows and integrated fixes.
READY remains absent. All final acceptance criteria remain open; no operator blocker identified.

