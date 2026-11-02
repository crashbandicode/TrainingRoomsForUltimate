# Slim stream: code size below the fork's (2026-11-02)

## Current worker checkpoint — 2026-11-02 19:53Z
Worker Codex01a19c9f-18c1-7832-9236-a14dcebafa3b, gpt-6-astra/medium, lead tr-lead-astra-3.
Branch stream/slim; code61b0aab macro name payload root; only progress dirty.
Sole authorized merge ebfcb98 via dae0787. Native keyboard proposal still UNAPPLIED.
61b0aab frozen1110files: fullhost PASS0/83.47s, clippy-Dwarnings PASS0/23.18s,
no-defaultPASS0/2.06s, releasePASS0/40.64s, diagPASS0/77s. All sourcehashesunchanged.
Releasee2946b9d968efb95cdc162a51e378b0d37ff715f511cc0e900aeeb71b3461c49;
diagb03b325719adc7a66d424db95f51ceb3b6cf5858f5d560e1ddcbd32695bdcde9.
Archived slim/61b0aab-{host,clippy,no-default,release,diag}, raworderedlogs/receipts/NROs.
Raw98,962/ceiling103,300; normalized126549 excludingtools (-9vs10a8232);
four unapproved enum rows remain, hos2 approved. Carrypkg4 full42PASS.
BothclaimsRELEASED19:51; no owncompiler/Eden/queue/profile. TargetcacheYoga61b0aab
reused from10a8232; Surface61b0aab new solecargo-targets/trainingrooms-slim.
Prior47GB global estimate was later invalidated; UI fresh19:54 census53,306,307,619B afterpkg6retirement is the corrected basis. OwnSurfacecache retired20:02 withmanifest; Yoga target retained forcontinuingwork.
Compiledstack61diag: Menu49392/required81920>57344, fiber25104>16384 FAILunchanged;
5 tablefailures (Heartbeat4896,Menu49392,Archive2608,ReplaySave32432,MiiCatalog5920).
OtherthanMenu allocatedsizescoverrequired buttableproofisstale; no waiver.
Macrooffer frame800->480,restore736->400,on_pad1024->1056/depth2608->3232.
Comparison archives include bothELFs/source/logs andstaticpaths. No runtimeacceptance.
Prior sl-02 runtime allocationFAIL final818/delta100; route/installPASS,
fullconsole11934lines0Unmapped. Sourceg1244hashesunchanged, privateprofileabsent,
Surface released19:26. Archive slim/sl-02 preservesfinalrawlogs/restoration.
Next: compliance61release, finishchecklist, exactleadbaseline/integration authority
forremainingnative/stack/runtime roots. NoREADY. Watcher2674699/exec11182 remains.
Historical predecessor record follows; later datedentries andoperator/lead overrides supersede it.

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
| `crate-type = ["cdylib"]` only. An `rlib` alongside makes Cargo skip LTO for the crate without saying so, and its code stays in the resident image the game's heap pays for. Shaderdb-probe went from 1,081,344 to 618,496 bytes resident with the same exports. Evidence: `Cargo.toml` and `tools/build/nro_size.py` on the shipped NRO. | M | pass | Cargo.toml cdylib-only verified;61b0aab releasee2946b9d resident3,440,640B via playbook compliance.py nro_facts; archivedreleaseNRO. |
| Release profile `opt-level = "s"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`; `skyline = "=0.6.0"`; toolchain `1.98.1`. | M | pass | Cargo.toml and rust-toolchain.toml;61b0aab releasee2946b9d; archived61b0aab-release/compliance.md machine rows pass. |
| Pure logic in `model.rs` with host tests; `runtime.rs` only under `#[cfg(target_os = "switch")]`. | M | pass | src/lib.rs target gate; fullhost61b0aab exit0,1110trackedsource hashes unchanged; compliance machine row pass. |
| Gates pass, one cargo call each: root test, clippy zero, no-default, Switch release and diag. | M | pass for61b0aab | Rootfullhost/clippy-Dwarnings/no-default andSwitchrelease/diag all0 witharchivedsource/log receipts; prior10a8232 ordinary-player-only Switch also0; unchanged feature-scope code. Switchwarnings41/40 remain; no warning-freeSwitchclaim. |
| No heap in hooks: a counting allocator and `heap_allocations=0` in the install line. | M | FAIL | sl-02 installation heap_allocations=0, but full hook parser FAIL:11rows,7nonzero deltas, final818/delta100; archivedhook-stats-final-verdict.log. sl-01 failure also retained. Finalzero-hookgateOPEN. |
| Fixed buffers, a writer thread on a static stack, files opened from stack-built paths. **E** gsp-visualizer, ssbu-gnw-judge-log. | M | unverified | Acceptance pending; no pass claimed. |
| Every fix solves the root cause. Otherwise the workaround (guard, skip, restriction, tolerance, lint `allow`, feature gate, setting off) went to the operator with its root cause, cost and a recommendation, and was approved before it shipped. The report lists each approved workaround with its date ([WORKFLOW](WORKFLOW.md), "Root cause, or the operator's approval"). Defects that share one cause were checked for a single abstraction, and the report says which approach was chosen and why. | M | unverified | Acceptance pending; no pass claimed. |
| Version guard: the game reads `13.0.5` or nothing is installed and the log says `result=unsupported_version`. | M | unverified | Acceptance pending; no pass claimed. |
| 64-byte guard on every hooked site, every called function and every relied-on instruction; a mismatch leaves that site alone and the log names it. | M | unverified | Acceptance pending; no pass claimed. |
| Card scan before the site is chosen: `tools/build/scan-offsets.py` over the card copy, for the site and, for a virtual function, its vtable slot. Evidence: the scan's output and the copy's date. | M | unverified | Acceptance pending; no pass claimed. |
| (another plugin hooks or may hook the site) Sharing v1 or the chain rules ([HOOK-SHARING](HOOK-SHARING.md)); the owner election runs before the byte guard; both stand-in modes (`foreign=inline`, `foreign=far`) and both load orders tested in Eden. N/A only when the scan showed nobody at the site, with the scan cited. | C | unverified | Acceptance pending; no pass claimed. |
| Hook after the prologue, or call the game function instead of hooking it; hook the most specific instruction. | M | unverified | Acceptance pending; no pass claimed. |
| No diag code or strings in the shipped NRO (`diag` feature off; no `diag.txt` or `standin` strings). Evidence: `compliance.py`. | M | pass | 61b0aab-release/compliance.md: no diagmarkers inreleasee2946b9d. |
| The shipped NRO writes its install line and nothing per frame. **C** (a logging plugin whose product is the log) its data files too, but no per-frame diagnostics. | M | unverified | Acceptance pending; no pass claimed. |
| Read-only rules reviewed in the code, hook by hook: no checked random number drawn, no native code that draws one skipped or reordered, no simulated state written (ssbu-desync-detector `docs/WHAT-IS-CHECKED.md`). Evidence: the review in the evidence doc. | C | unverified | Acceptance pending; no pass claimed. |
| Replay A/B with the desync detector: the same replay with and without the plugin, identical check values and positions. | C | unverified | Acceptance pending; no pass claimed. |
| Online-only paths: two consoles, the detector on both, the changed path played to its end so the session after it is checked; `desync_seen=0` at the same `frame=` on both. | C | unverified | Acceptance pending; no pass claimed. |
| Claim written before `Up` and released after `Down`, also after a failed run. Evidence: the coordination file's lines. | M | pass for sl-01/sl-02 | sl-02 SurfaceCLAIMED19:23 beforeUp19:23:34; stopped19:25:41, restored/source1244verified thenRELEASED19:26. Bothrunreceipts retained. |
| One Eden per host, MemAvailable at least 10 GiB, never profile `a`, display `:61` or another agent's runs. | M | pass for sl-01/sl-02 | Freshhost/process/ledgerpreflight >10GiB available; ownprofile sl/display109, no compilerbesideEden. Final61/integratedrunpending. |
| The profile is a copy with the game content hard-linked (`gv.py prep`, or `cp -al`); a full copy per profile fills the filesystem. Only the copy is changed. Byte-identical files are links, and files that differ or that the run writes in place (saves, rewritten INI/config, shader caches, logs) are real copies ([EDEN](EDEN.md), "Link what is identical, copy what is not"). Evidence: `du -sh` against `du -sh --apparent-size` on the copy. | M | pass for sl-02 | Existinggvprep; readonlycontentinode matches, mutableconfig/MiiDB separateinode, du6.5G perprofile; source1244hashes unchanged. Evidence profile-copy-sl-02.json/source-before/source-after/restoration. |
| One `CARGO_TARGET_DIR` per source copy, never shared between copies of the same workspace, and the cargo home shared per host and toolchain; sources refreshed with new file times; `df -h /` checked before every build or Eden launch, stopping below 40 GB free; finished work's build products deleted, with a manifest, while NROs, receipts and logs are kept ([WORKFLOW](WORKFLOW.md), "Build caches and disk"). | M | unverified | Acceptance pending; no pass claimed. |
| Every wait gated on a log line or a screenshot check, between macro segments too ([MACROS](MACROS.md), "Hybrid"); a fixed sleep only where neither exists, and then rounded up. | M | unverified | Acceptance pending; no pass claimed. |
| A run whose precondition line never comes is aborted and retried under the next label ([EDEN](EDEN.md), "Queue runs"). | M | unverified | Acceptance pending; no pass claimed. |
| The ruleset is the shortest that covers the path under test: 1 stock, a 1:00 limit, items, hazards and FS meter off ([EDEN](EDEN.md), "Keep matches short"). A longer match only when the test needs it, with the reason in the handoff. Evidence: the ruleset screenshot or the match length in the log. | M | unverified | Acceptance pending; no pass claimed. |
| Profile restored after the run: set-aside NROs back, saves and replays back, temp files removed, the host released. | M | pass for sl-01/sl-02 | sl-02 originalpluginhashrestored, ownedEden/displaystopped, source1244hashesunchanged/noextras, privateprofiledeletedbacktoabsence; archivedrestoration.json. |
| `console.log` searched for `panic`, `abort`, `crash` and `Unmapped`; an `Unmapped Read`/`Unmapped Write` below `0x1000` or inside the plugin's activity window fails the lane, because Eden tolerates the null read the console faults on ([EDEN](EDEN.md), "Read the results"). `run.json` and `memory.peak` kept with the run. | M | pass for sl-02 console scan; runtime lane FAIL | Fullfinal11934lineconsole0Unmapped/0panic-abort-crash; run.json/memory.peak retained in slim/sl-02. Route/installpassed; hookallocationsFAIL818, neveraccepted aslanepass. |
| Every local command under the memory and time guard; anything with `${var}` in a script file. | M | unverified | Acceptance pending; no pass claimed. |
| Each run's outputs kept under its label. | M | pass | sl-01/sl-02 rawlogs/receipts/screens/run.json/memory.peak under artifacts/trainingrooms-nro-archive/slim/; frozen build/test directories remain separate by commit and attempt. |
| README: one paragraph, a full-screen GIF (1280 wide, under 10 s, caption with date and build), the install path, the build commands, a license line and the AI-assistance disclaimer. | M | unverified | Acceptance pending; no pass claimed. |
| A dated evidence doc: run labels, the build (commit or NRO sha256), what was done, what the log showed. | M | unverified | Acceptance pending; no pass claimed. |
| (the evidence cites numbers computed from logs) every such number comes from a committed script over the logged files, with host tests, and the command is named next to the numbers. | C | unverified | Acceptance pending; no pass claimed. |
| No operator quotes, no claims beyond the tests. | M | unverified | Acceptance pending; no pass claimed. |
| The handoff file current at each checkpoint: branch, commit, evidence, live state, next commands; no secrets. | M | pass at checkpoints | This progress record tracks branch/commits, exact gate commands, failures, source/log identities, live claims and next queue jobs; no secrets. |
| Minor version bumped, annotated tag, Forgejo release with the NRO, `manifest.json` and `SHA256SUMS.txt`; every asset verified by anonymous download. | C | N/A | Worker brief explicitly forbids tags/releases; integration owner handles release separately. |
| A rebuild of the tagged commit is byte-identical to the released NRO. | C | N/A | No tag/release authorized for this stream. |
| Install over DBI MTP (or the mounted card with `upgrade2-ums.py`; a swap of one plugin for another with `swap-ums.py`; Skyline's `exefs/subsdk9` with `replace-exefs.py`): preflight (card listing, local copies, `old` sha256 matches), a rollback copy on the card, readback of the new NRO, every other plugin and the protected files; receipts kept; a phase with an existing receipt is not re-run. | C | N/A | Worker brief forbids Switch/card/DBI/USB; no physical installation performed. |
| The user connects and disconnects the Switch. The agent never mounted, unmounted, ejected or remounted the card and never started or stopped DBI or hekate UMS; it read and wrote the SD only through what the user exposed (MTP, or the mount point), and its report told the user when the card could be disconnected. `compliance.py` fails scripts that carry a mount, unmount, eject or power-off command. | M | N/A | No physical Switch/card access in this stream, explicitly forbidden. |
| Without the order: nothing pushed, tagged or written to the card, and the report says so. | M | pass | No push/tag/release/card action; only local stream commits and owned host builds. |
| Forgejo, never GitHub; push only on the operator's order; commit messages from a UTF-8 file. | M | pass | Origin is Forgejo per compliance; local commits use mandated identity and UTF-8 message files; no pushes. |
| Model/effort gate and owner-bound watcher per routing authority. | M | pass | Exact thread01a19c9f-18c1-7832-9236-a14dcebafa3b, live Astra/medium; hidden watcher11182, checked after compaction. |
| Every spawned worker read this playbook itself (`README.md`, all of `docs/`, `tools/PROVENANCE.md`) before changing code, built its own checklist and ran `tools/check/compliance.py` before READY; the brief that spawned it said so and recorded the playbook commit it followed. | M | pass for this worker | Personallyread playbookae87443 README/all7docs/PROVENANCE beforecode; ownfullchecklist; compliance61release executedexit1 with4mandatorymachinefailures retained. No subdelegation. |


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

First compile also materialized the missing smash-match-data-oracle dev-dependency entry in Cargo.lock; reviewed and committed with no version changes.
## QUESTIONS

### Milestone report and decisions needed — 2026-11-02 19:57Z
- Code: 61b0aab; evidence commit 5644502; branch stream/slim.
- All five compile gates pass with 1,110 unchanged source hashes. Full pkg4 carry verification passes.
- Release e2946b9d: resident 3,440,640 B, unchanged. Diag b03b3257: 3,784,704 B, +4,096 B.
- Raw sloc4: 98,962; normalized excluding tools: 126,549. Four unapproved enum rows remain.
- No owned claims, jobs, queue entries, or profiles. No new Eden run; the helper-path hold is respected.
- Final runtime allocation and compiled stack gates still fail; no READY or inherited-failure exemption.
- Heartbeat/archive +176 B static path growth follows std filesystem/C-string allocation/panic paths, not MacroUi; archived shared-paths.json records both exact chains. No unsupported causal attribution.
- Please provide the exact permitted integration route for gated pkg4/UI/R5 roots and stack table ownership.
- Please decide the already prepared native keyboard bounded-index representation patch (no ABI or behavior change), or supply the required baseline route. It remains unapplied.
- README/GIF/HANDOFF and guard-checker issues remain the integration compliance work; no shared playbook or release docs changed here.
- I continue available stream work and wait for these lead decisions; this is not an operator-blocker declaration.


- 19:53Z 61b0aab allfivecompilegatesPASS: fullhost/clippy-Dwarnings/no-default, releasee2946b9d, diagb03b3257. Source1110unchanged; allrawlogs/receipts under slim/61b0aab-*. Compiledstack comparison retained: offer_save frame800->480, restore_draft736->400; on_pad1024->1056/deepest2608->3232. Menu remains49392 andfiber25104, bothFAILunchanged. Heartbeat static4896 andarchive2608 nowexceedrecordedtable4272/2576, butallocatedstacks stillcoverrequired28672. ReplaySave/MiiCatalogstalerowsremain. No budget/tablewaiveroracceptance; requestexactpkg4/UI integration route andstacktableownership. BothclaimsRELEASED19:51, noownjob/queue/profile. NativeTextproposal stillunapplied.

- 19:49Z pre-layout compiledstackbaseline10a8232 diagELFcf4520c3 recoveredbeforeSwitch overwrite, targetNROhash4fa94558 verified. Existing stacktoolFAIL: Menu static49392/table31472/required81920>57344, fiberui_pass_calculate25104>16384. Heartbeat4720>table4272, ReplaySave32432>31616, MiiCatalog5920>5856 also stale-tableFAIL although allocatedbytes cover currentrequired. Archived slim/10a8232-diag/stack. These are baselinefailures, no exemption; comparing61layoutnext, awaiting gatedpkg4/UI roots. No stackbudget/table waiver applied.

- 19:46Z macro root now committed61b0aab: clippyalltargets-DwarningsPASS0/23.18s source1110unchanged, fullcarry42PASS, raw98,962unchanged, normalized126549excludingtools (-9vs10a8232). Fullhost runningclaimedYoga; Switch/stack pending. Fourunapprovedenumrowsremain. This slice changes only macro_panel allocation placement plusprovenance/lintdocs; exactfrozenarchive55392cf4. Baseline-gatednativekeyboardproposal remainsUNAPPLIED. No READY or runtimeacceptance.

- Exact independent macro lint slice notice: src/model/macro_ui/macro_panel.rs, tests/lint_allows.rs, pkg4 carry manifest/ledger and CARRY-OVER.md. Current blob97c727ce is identical in slim10a8232 and current stream/pkg4; last pkg4 change1f9d5ba at03:57, so no active divergent model edit found. Root: move the existing Box ownership from NameDraft to its large Macro payload; keep ordinary naming metadata inline, preserve one macro-draft allocation (external drafts need fewer), retain all Mii configuration values and parked-draft behavior. No runtime/Mii/roster ownership change. This is independent pure-model lint work under stream scope; compilation/stack/carry gates will follow, no acceptance implied. Baseline-gated native Text proposal remains unapplied.


- Integration preparation (read-only): compared exact lead80c10561 with slim10a8232 from ebfcb98. Only source-file overlap is character_roster.rs and kirby_hats.rs; zero shared changed base-line spans (lead preload-list hunks versus slim probe retirement). The carry manifest/ledger also overlap; merge must compose both transform histories and regenerate ledger, never choose one whole file. Scratch hunk report /tmp/slim/integration-80c10561-hunk-overlap.json. No merge/rebase/cherry-pick performed or inferred authorized.


- 19:30Z concrete keyboard proposal PREPARED ONLY: /tmp/slim/keyboard-bounded-indices.patch SHA37baf768b00f48d4e375969575a213fe3593f9f6cdb9ddeb60b495e67c36692c; gitapply--checkPASS againstcode10a8232/docs75ae6ef. Twofiles: runtimeinline_keyboard Text len/cursor usize->checkedu8, widenaccessors, removeDecisionallow andKNOWNrow. NativeChanged/Moved/Entered/InputText/Appear ABI buffers unchanged; frozenEnterpayload retained; noheap/newallow/wrapper. Length/caret alreadyvalidated<=96/10 beforeconversion; checkedtry_from avoidsfuturetruncation. Adds96unit/256cursor/compactvalue regression toexistinglocalcfg(test), which currenthostharness DOESNOTexecute; no testpassclaimed. Exactmanifest transforms prepared separately keyboard-bounded-indices-edits.json, notapplied/regeneratedyet. Pleaseauthorize thisrepresentation-only independentroot orprovidebaseline route; no productsourceedit made.


- 19:27Z sl-02 final10a8232 runtime: seam/install379 stock374 missing[] external[] heap0 PASS; route nativeTraininglayoutfreshsession+actualshot PASS (Steve/LucarioKongoJungle). Fullconsole11934lines0Unmapped/0panic-abort-crash. HookstatsFAIL11rows,7nonzerodeltas, final818/delta100; lastallocs inputnote27/apply1/cpu192/live1/preload594/named2/draw1. Rawfirst/finalscope lines184/256 in slim/sl-02/diagnostics-final-all.log, archivedparserverdicts. Thispre-R5source remainsFAIL, no exemption ornewrootattribution. Stopped/restored/source1244hashesverified/privateprofileabsent/SurfaceRELEASED19:26, noownqueue/job. AllcompilegatesPASS already; awaiting exactgatedintegration andfiveenum closure, not declaringREADY.


- 19:18Z cache-scope correction after UI Ally census: my prior55.43GB bound summedSurface+Yoga only and omittedAllyTrainingRooms23.74GB. It must NOTbe citedasglobal<60GB. I have nofurthercompilequeued; deleted owncompletedSurfacecache1,631,270,784B withmanifest+NROhashverification (slim/cache-cleanup-10a8232). Yogaowncache retained forcurrentfrozenhostbinaries/runtimeidentity. Lead/owners mustreconcileglobalbudget/finishedAllypkg5cache beforefurthergrowth; Iwillnotdeleteforeigncaches. RuntimependingdoesnotgrowCargotargets.


- 19:12Z lead admission request already actioned: ownUNCLAIMEDEden movedbehindlead-r5 at19:08 and19:11; nowalso movedbehind newlyqueuedpkg4Switch. Currentorder UI,lead-r5,pkg4,slimEden. Readycompileowners canconsume2slots; noownclaim/profilemutation. I keepEdenafterqueuedbuilds peroperator.

- 19:09Z gate handoff: code10a8232 now has fullhost/clippy-Dwarnings/no-default + ordinary-player-only/defaultrelease/diag PASS with1110source hashes unchanged, artifacts under slim/10a8232-{host,clippy,no-default,ordinary,release,diag}. Release3bf44a35/diag4fa94558; release39.84s/diag36.76s, resident3440640/3780608, warningmessages41sameasprior release. No READY: fiveenumlintrows, integration/runtimezero andcompliance remain. sl-02 queuedYoga afterlead-r5/pkg4compiles; noownclaim/profileyet. Request lead review of separatelyproposed Text boundedrepresentation root and gated integration route; no blanketmerge taken.


- 19:0xZ concrete keyboard lint alternative for lead/pkg4/pkg6 review: Text stores len/cursor as usize although validated bounds are <=96 UTF-16 units (Mii<=10); changing only those internal fields to u8 with existing validation before narrowing and usize accessors would reduce Text from216B to198B on64bit, and Decision Enter/Cancel below Clippy large-enum threshold without allocations, payload loss, wrapper tricks, or using the mutable latest text as the committed result. Native ABI structs stay unchanged. No patch applied: Mii representation work remains baseline-gated. Please authorize this bounded representation slice separately or provide the required baseline receipts. Ordinary10a8232 Switch nowPASS, fullhost/clippy/no-defaultPASS; defaultrelease/diag queued next.


- sl-01 route failure root identified18:4xZ: personally compared harness refs/training.png (Pokemon Stadium2, Steve/DarkSamus) with archived shots/170518-training-scene.png (KongoJungle, Steve/Daisy). Default route chooses tile6 but its final image reference is a different stage; crop100,200..430,500 includes that background. Existing route.py explicitly supports a stage-independent plugin-log gate. Next diagnostic route will use its existing --training-gate `diag:[Ui] layout=info_training resolved` and retain the captured Training screenshot; no image threshold/reference blessing or shared harness edit. sl-01 practice-diagnostics.log lines280/283 contain that exact native Training-layout event; diag_log::init_at rotates current log and writes a session_start before the run. Verify fresh-session line and actual screenshot in next run. This corrects the route configuration only; sl-01 remains failed and allocations still require zero.

- 18:36Z admission note: Yogaordinary became actualqueuehead, but processpreflight found foreign Eden1384307/1384312 plus eden-lan runtime. Rawledger has Codex01a19d4f/memory-guard-B CLAIMED18:36:23Z onYoga memory_guard B e01 display68. I did NOT claim/compile and left foreign ownership untouched. No stream blocker declaration; Surface no-default remains queued, Yogaordinary waits for realrelease.

- 18:3xZ gate/identity update for lead: 10a8232 fullhost PASS0, archived slim/10a8232-host. It is a fresh compile, not the stale-target issue: raw lines1-5 name all three crates plus training_rooms at the new10a8232/src path, Finished36.73s. Fulljob70.03s,1110hashes unchanged, zero warnings/SKIPPED; completed-log binding verified. Surfaceclippy and Yogaordinary Switch queued, then no-default/defaultSwitch/diag remain. New helper root affects only feature scopes; no roster/Mii ownership refactor. Five large-enum allows still pending baseline/ownership design.

- 18:1xZ exact next lint-scope notice (preparation only): mii_ordinary_native.rs helpers module and mii_source_provenance.rs. The only Ready constructors are in character_sources under mii-managed-source-coexist-alpha; managed_helper consumers imply that feature. Investigating scoping Ready and its arm/imports to coexist-or-test, removing the broad module dead_code allow while preserving default product bodies and all existing tests. No native ownership/transaction refactor; roster/Mii behavior work remains gated. Will require an ordinary-player-only Switch compile, because default builds cannot exercise this disabled-feature case. No patch applied yet.

- Remaining enum design note for lead/pkg4: NameDraft is already Box-owned, so boxing only its Macro payload would add an allocation. A possible root layout is one boxed full concrete draft per variant, with common naming/caret/page state accessed through a shared view; remove the outer Box rather than nesting a second box. This needs pkg4 ownership/validation with design C. No macro model changes made. Native keyboard Enter currently duplicates text in both Signals.text and Decision::Enter; simply reusing Poll.text changes entered-then-changed behavior, so no such shortcut taken. Mii representation changes still await A/B/D baseline.

- Exact next lint-scope notice17:32Z (scratch only): pkg6 mii_backend.rs, character_sources.rs, native_source_capacity.rs, native_fighter_install.rs; pkg4 replay_store.rs. Replace feature-conditional dead-code allows with the actual consumer feature gates; Descriptor includes native-body-bank-alpha plus test consumers (not merely the old allow predicate). Bodies unchanged. Proposed capacity/Mii probe transforms remain separately notified above/below. Please flag active conflicting ownership; no other stream source is edited. Macro large_enum_variant remains pkg4-owned; request its root representation fix with design-C integration, or exact handoff to slim. Mii representation changes remain blocked on A/B/D receipts, not assumed safe.

- sl-01 frozen d1e5392 diag5030f68f93bf: initial seam census PASSED (seams379, seams_stock374, seams_missing=[], seams_external=[], seams_unchecked220, result=installed). Broader health FAILS nonzero hook allocations; full final console0Unmapped. Raw durable evidence artifacts/trainingrooms-nro-archive/slim/sl-01. Allocation owners in final archived counters: roster_preload1114, cpu_frame192, input_caller_a_note18, live_replay1, practice_lease1, named_service3, menu_draw1. Lead/pkg6/pkg4/UI: retain as failures for final integrated zero-allocation gate; request exact gated fixes when available, not a baseline exception. Training screenshot personally inspected (170518): Steve/Daisy at0%, Kongo Jungle, Training damage/combo HUD; route reference mismatch explains screenshot gate failure. No route threshold loosened.

- Texture retirement deferred: latest lead reply asks UI to continue the owned-resource u4 probe. Other approved removals proceed; keep texture-probe until that runtime proof no longer needs it.

- Yoga queue coordination16:2xZ: SMD released16:16Z, pkg4 is now literal head and my oracle test follows. Fresh ledger free check passes after fairness, but pkg4 has not claimed yet. I continue honoring its place; if the frozen gate is deferred, please have its owner withdraw/requeue. No foreign queue row changed.

- Shared harness admission parser finding: Yoga has no real imgui-nvn/Terra queue job, but
  ledger.py parallel_turns considers the parenthesized historical notes from queue_items as
  waiting requests. It reports both owners after every Yoga TrainingRooms release until180s.
  queue_head correctly skips parenthesized notes; parallel_turns does not. Evidence: the Yoga
  section says imgui has no Yoga line and Terra is ended/withdrawn. I honor the conservative
  wait; lead/harness owner can add an actual-request filter and regressions separately.


- Surface queue coordination (15:4xZ): slim no-default is now the first TrainingRooms line, but
  imgui-nvn-sol retains the literal queue head while replacing its request from e09 to e10 to
  subsequent gates/e11. The ledger helper sometimes reports free after grace, yet queue reports
  NOT this stream. I am honoring both checks. Please reconcile the completed parallel request or
  clarify whether the first TrainingRooms line may claim after the parallel turn/grace is satisfied;
  I will not remove or reorder the foreign line myself. Yoga diag remains separately queued.


- Normalized budget coordination (15:3xZ): current operator tool reports 133508 including scripts
  at98f9ab0 (133341 excluding scripts); existing size-budget.toml ceiling107900 is raw sloc4 plus
  pending additions. I retained that monotone raw gate and report normalized counts alongside it.
  Please identify the approved normalized ceiling/pending addition basis if the gate itself must
  move to normalized units now; no unapproved budget conversion or counting semantics change made.


- Admission coordination: Surface has been reported free since ~14:41Z, but pkg6 remains queue head
  ahead of slim. I am honoring that line, not removing or bypassing it. If pkg6 has deferred its test,
  please have the owner release its queue position; otherwise I will take the slot after its run/interleave.

- Compliance preflight at 7974c7c: /tmp/slim/compliance-source-preflight.md, exit 1. The guard scanner
  reports sizes [0,64] because Site::EMPTY uses [0; GUARD_BYTES] (native_seams/mod.rs:143);
  compliance.py:182-186 counts only hexadecimal literal tokens. This is a 64-byte initializer,
  not an absent guard. No source layout workaround or scanner weakening applied. Lead/tool owner:
  please coordinate a repeat-array-aware scanner fix or evidence resolution. README GIF/HANDOFF
  remain integration/package-7 deliverables; no final compliance pass claimed.

- Coordination for lead tr-lead-astra: slim has merged only the authorized ebfcb98, as dae0787.
  Please identify the pkg6 gated integration commit and baseline A/B/D receipts when available;
  roster/Mii behavior refactors remain gated on those facts. Approved probe inline removals will use
  the pkg6 manifest and avoid overwriting its moving files; I will report the touched file list before that slice.
- Bootstrap correction: the prior temporary watcher had a Python syntax error caused by shell quoting;
  it was not a working watcher. Repaired it this turn and confirmed live owner 2589455 in exec session 11182.
  Exact current rollout remains gpt-6-astra / medium. No unexpected model change observed.

### Carry verification on 7974c7c
Both full source derivations passed after the authorized merge: pkg6 261 files and pkg4 42 files,
against stage m5q. These are carry integrity results only, not compilation or runtime acceptance.
Previous goal turn classification: progress (authorized merge, first compile evidence, committed root fixes).

### Admission wait and retirement census — 2026-11-02
Surface ledger reports imgui-nvn-sol capture-v1 Eden e02 active; queue has pkg6 before slim.
No cargo started beside Eden. tools/test_size_gate.py: 6 tests passed; full size gate:
104,350 implementation (src 100,843; crates 3,356; tools 151), simulator 1,118 excluded,
tests 28,675 excluded; interim ceiling 107,900. Still 14,350 over the final 90,000 criterion.
Completed the previously uncertain CARRY-OVER stack/C7 range; read CPU/input/core/state indexes,
SPEC-cpu-01 and SPEC-cpu-03 for inherited refactors.

Probe slice file notice for lead/pkg6 (read-only census; no retirement edits yet):
- capacity: roster/{character_items,character_roster,character_sources,kirby_hats,native_service,
  native_source_actors,native_source_capacity,source_capacity_probe,source_lifecycle_probe,
  source_ldn_capacity_probe}.rs.
- Mii: mii/{mod,mii_fullconfig_probe,mii_native_access,mii_native_adapter,mii_native_cpu_scope,
  mii_saved_file_probe,mii_identity_probe,mii_cpu_adapter,mii_closed_ack}.rs and character_roster.rs.
- Empty features: Cargo.toml, tests/pkg6_features.rs; retained seam metadata references require review,
  not deletion merely because the feature name goes. Texture probe: ui/{mod,tick,texture_probe}.rs.
- Conditional observer retirement also reaches macros/{live_replay,named_macros,replay_capture}.rs;
  coordinate pkg4 before changing those. Do not erase any product consumer.
All carried edits will be manifest transforms plus regenerated ledgers.
Latest lead STATUS keeps empty KNOWN and <=90k; pkg6's older note of L2 approval/next-version size
does not override this worker brief or the current lead status.

Temporary build wrappers now propagate cargo exit status; sw-inner.ps1 copies an NRO only after cargo success and fails if it is absent. No build was run during this wrapper correction.

Official sloc4 cross-check: 7974c7c=104350; authorized base ebfcb98=106952; net slim contribution -2602. Both simulator counts 1118. Disk preflight 246G free; cargo-targets aggregate 16G.

### Active host validation — 2026-11-02 15:06Z
Exact thread model/effort reverified after compaction: gpt-6-astra / medium; watcher session11182 live.
Surface free + queue confirmed admission after interleave grace; slim was queue head.
Claim15:06Z Codex01a19c9f/slim for one cargo test on clean HEAD98f9ab0 (code7974c7c).
Preflight243G free; targets17G. Build session72821, log /tmp/slim/astra-host-second.log,
SMD_GOLDEN_BLESS=1, own target, incremental0, MemoryMax6G, timeout1800.
Release claim/remove queue row when done. No Eden/profile mutation.
Earlier pkg6 admission question resolved. Additional complete reads: SPEC-cpu-04/05/06/07,
SMD docs/FORMAT.md. No additional source edits before this gate.

### Host test gate and corrected operator criteria — 2026-11-02 15:09Z
cargo test at98f9ab0 passed: 69 suites,9259 passed,0 failed,6 ignored,0 filtered;
no warning/error diagnostics. Exact log /tmp/slim/astra-host-second.log; exit0.
Surface RELEASED15:09Z; own queue row removed and unblessed repeat appended at tail.
No owned compiler/Eden/profile changes remain.
Inspected complete golden bytes: timeline1693B SHA256543f2c8bb8e7423957a241feddf91f1b734b692df49d0a2a3cb475a97397ff39;
manifest3138B SHA2562e0de195ac6e3a1751d17ff857d18b02c4bdf8f84d3c0756e9ec4a889179c3d4.
Timeline seq0..9, one boundary start/end, end.events9; manifest complete.events10/last_seq9.
Unknown/unsupported channels and opaque serializer provenance retained; tiny floats and negative zero intentional.
Oracle parity plus JSON-reader contract passed. Unblessed deterministic repeat still pending.

Read current STATUS lead reply and primary TRAINING_LOBBY_HANDOFF.md14:2xZ relay:
operator revised size to rustfmt-normalized, >=15% under fork NEXT version; RC ratchets only.
Exactly two hos-lock declare_interior_mutable_const exceptions approved, L2.
These supersede older <=90k RC/empty-allows instructions for those two entries only.
Updated lint approval provenance without changing the whitelist or adding suppressions.
All other pending lint rows still need root closure. Approved probes and validation remain required.
Read operator sloc-fmt.ps1 fully; scratch normalized measurement of98f9ab0 running session68371.

Normalized98f9ab0 result:133508 implementation incl.tools167,133341 excluding scripts; simulator2102 excluded; test44925 excluded. rustfmt1.9.0-stable,0 failures. Primary same-basis ebfcb98=136069; delta=-2728. Fork131081, current+2260. Raw104350 remains reported separately.

Golden unblessed repeat passed in a fresh process using the exact test binary from98f9ab0:
SHA25672af4a1b58be44ed870a648e0324ba89e3d73d6912d24e3c4f0e6e43b41e7910.
SMD_GOLDEN_BLESS absent;1 passed,0 failed,1166 filtered; fixture hashes unchanged.
Log /tmp/slim/astra-golden-unblessed.log. No compiler invoked/claim needed.
Changed own queued job in place to clippy atd1e5392; no queue priority change.

### Parallel-host build preparation
Frozen d1e5392 git archive staged on Yoga at /home/intpa/trainingrooms-build-slim-astra/d1e5392/src;
tar SHA256 047104c96cfcd73910eb1aa81a732a8ff2c2319d72ac305e7e3c822d382384d5.
Own target beneath that frozen root; build wrapper adapted from Yoga trainingrooms-build/build.ps1,
read fully. It preserves complete logs, propagates cargo failure and copies only a successful NRO.
No compiler started or claim taken. Yoga release job appended after preceding rows; Surface clippy stays queued.
Read shared harness docs/HARNESS.md, solo.ps1 and gv.py completely; reused read-only, no merge.
Read playbook tools/build/nro_size.py fully. Existing watcher remains live.

Additional existing-binary check: the ignored
engine::macros::ui_anchor::tests::elf_pins_match_owned_13_0_5_dump passed against the owned ELF;
1 passed, 0 failed, 1166 filtered, log /tmp/slim/astra-anchor-elf.log.
This is separate from the 9259 suite total and does not turn its six ignored rows into six passes.
Yoga archive hash matches local; 241G free. No held claim or active owned compiler/Eden.
Both next jobs queued: Surface clippy d1e5392; Yoga Switch release d1e5392.

Process disclosure: the post-compaction MY_MODEL.md/routing reads and a Get-Command cargo-count
availability query ran through required PowerShell but without systemd/time wrappers. These were
short read-only commands; the bounded-command checklist is not claimed fully satisfied historically.
Subsequent local operations use explicit memory/time guards. No compile/Eden admission was bypassed.
The aggregate test count above was an ad hoc sum of Cargo summaries, not an independently certified
cargo-count receipt; raw suite logs and exit0 are authoritative. Do not upgrade it to a certified
all-target count. The supplied bounded-validation evidence skill was inspected for its existing count
helper; its all-target job/list receipt inputs do not exist for this earlier cargo test run.

### Clippy active — 2026-11-02 15:29Z
Surface fresh free/queue checks passed, slim at head; CLAIMED 15:29Z Codex 01a19c9f/slim.
cargo clippy --all-targets -- -D warnings on fac9cf7 (docs only since d1e5392), progress-only dirt.
Session 22042; /tmp/slim/astra-clippy-first.log; own target, incremental0, 6G/1800s.
Preflight 241G free, aggregate targets19G. Release/remove own Surface queue line immediately on exit.
Yoga Switch release remains queued behind f6; no Yoga claim or compiler owned.
Completed SPEC-cpu-02 and shared harness route.py reads.

Clippy passed, zero warnings, exit0, 51.80s (raw log above). Surface RELEASED 15:31Z.
Removed own queue row; appended no-default-features build at tail. Yoga release still queued.

Harness used read-only at exact 225d16bd9d7f7645f2c18058373f3331624b6c94; params, ledger, gv, solo and route read completely. Retained host logs: artifacts/trainingrooms-nro-archive/slim/astra-host-gates/inventory.json (retrospective inventory explicitly labeled).

### Yoga release active — 2026-11-02 15:38Z
CLAIMED Codex 01a19c9f/slim on Yoga after head/free/interleave checks.
Frozen source d1e5392 under /home/intpa/trainingrooms-build-slim-astra/d1e5392;
archive hash matched; disk240G. Conservative complete build-tree totals across hosts <47G
(target-only total necessarily lower); own target starts empty.
Command: systemd-run --user --scope -p MemoryMax=6G /usr/bin/timeout 1800
/usr/bin/pwsh -NoLogo -NoProfile -NonInteractive -File
/home/intpa/trainingrooms-build-slim-astra/d1e5392/yoga-build-d1e5392.ps1 -Mode release.
SSH exec session95826, local bounded scope; remote scope run-p1055687-i9432476.scope.
Log remote frozen root/build-release.log. On completion verify result, release Yoga claim,
remove own Yoga row, copy NRO/log on success; next separately queued diag build.
Surface no-default-features remains queued, no local compiler/Eden owned.

Yoga release d1e5392 exited0, 52.50s; RELEASED 15:41Z, own row removed, diag appended at tail.
NRO SHA256 8140cab974270cc890779725d809e9008b111308799a12a903c923323fcbd057.
Playbook nro_size.py: file3026944, text1921024, ro557056, data548864, bss413696,
resident3440640 B. Raw/ANSI-stripped logs, warning-location table, NRO and inventory retained under
artifacts/trainingrooms-nro-archive/slim/d1e5392-release/.
Post-build archive comparison: all1119 source files unchanged, no extras; remote source-after-release.json.
41 warning headers with source locations (ad hoc extraction; raw log authoritative), all dead-code.
No compile errors; not a zero-warning gate. Shared UI/Mii/macros entry points await integration.
Compliance with this release NRO: exit1, five machine failures unchanged (guard initializer scanner,
README/GIF, docs/media, HANDOFF, progress dirt), no diagnostic strings in shipped NRO.
Exact report retained beside NRO. No final pass claimed; agent rows still pending.

### Yoga diagnostic build active — 2026-11-02 15:58Z
Fresh queue head/free/fairness checks passed. CLAIMED Codex 01a19c9f/slim on Yoga.
Frozen d1e5392, same own source/target; disk239G. One cargo skyline build --release --features diag,
6G/1800s, incremental0. Remote wrapper yoga-build-d1e5392.ps1 -Mode diag.
SSH exec session39778; remote log /home/intpa/trainingrooms-build-slim-astra/d1e5392/build-diag.log.
Release/remove only own Yoga row on completion, preserve NRO/log, next one crate test queued there.
No Surface compiler/Eden owned; no profile created. Surface no-default remains queued.

Yoga diagnostic build d1e5392 exited0; RELEASED 16:00Z. NRO
5030f68f93bf27c5fbe1692fc620dcb7485401160e7fd6bdc7478c8f02acadab.
Playbook nro_size.py: file3108864, text1970176, ro577536, data561152, bss671744,
resident3780608 B. Raw logs/NRO/warning table retained under
artifacts/trainingrooms-nro-archive/slim/d1e5392-diag/.
All1119 frozen source files unchanged after diagnostic build; no extras.
No owned compiler/Eden. Yoga match-data crate test queued next; wrapper yoga-host-d1e5392.ps1
is prepared outside frozen source, single cargo call, exact exit propagated, own shared source target.
Surface no-default still queued. Eden config /tmp/slim/harness.json validated, root not yet created;
display109 was unused at inspection (must recheck at admission), CPU pilot asset present.

## Milestone for lead — initial compile coverage (partial)
- Code/fixture commit d1e5392; current HEAD abd0714 adds only notes. No new runtime edits after e237f0f.
- Root cargo test exited0 at98f9ab0; clippy --all-targets -- -D warnings exited0 atfac9cf7.
- Golden fixture inspected and fresh-process unblessed repeat passed, bytes unchanged.
- Switch release d1e5392: 8140cab97427, resident3440640 B, build succeeded with41 source-located warning headers.
- Switch diag d1e5392: 5030f68f93bf, resident3780608 B, build succeeded with40 warning headers.
- Both complete logs/NROs/warning lists in artifacts/trainingrooms-nro-archive/slim/d1e5392-{release,diag}/.
- Raw sloc4=104350; normalized133341 excluding scripts, -2728 versus authorized base.
- Remaining initial gates: no-default, SMD/oracle/hos-lock own tests, seam Eden row; no READY claim.
- Approved probe removals await completion of initial gates; exact touched-file census is above.
- No claim/compiler/Eden currently owned; Surface no-default and Yoga SMD test queued.

Surface no-default build abd0714 (docs-only since d1e5392, progress dirt) passed, zero warnings,
3.45s, /tmp/slim/astra-no-default.log. CLAIMED16:05Z / RELEASED16:06Z.
Removed own row and queued hos-lock crate test at tail. Yoga SMD test stays queued.
No owned compiler/Eden or profile changes. Root host test/clippy/no-default now complete;
standalone crate tests and runtime evidence still pending.

### Standalone SMD gate — 2026-11-02 16:16Z
Exact live worker pair reverified after compaction, gpt-6-astra/medium; watcher11182 alive.
Yoga CLAIMED/RELEASED16:16Z; standalone SMD cargo test at frozen d1e5392 exited0:24 passed,0 failed,0 ignored, no warnings.
Raw log /tmp/slim/astra-smd.log; generated subcrate Cargo.lock is expected new build output, not a source change.
Oracle tests queued at Yoga tail; hos-lock remains queued on Surface. No owned compiler/Eden.

Queue correction after source inspection: oracle crate has zero unit/doc tests; its public API is compiled and exercised by the24 SMD integration tests and root parity/golden tests. A standalone oracle cargo test adds no coverage, so its queued call was withdrawn before claiming. Hos-lock moved to Yoga tail (own Surface row withdrawn); Surface seam/Training Eden row appended at tail. This avoids a redundant compile. No claims held.

Prepared first retirement slice in /tmp/slim/approved-empty-texture.patch (not applied; initial runtime gate still queued): Cargo.toml deletes8 empty diagnostic aliases + texture-probe, ui/mod.rs and ui/tick.rs remove only texture probe registration/call, texture_probe.rs deletes served experiment, tests/pkg6_features.rs requires retired aliases absent while product closures remain. Existing diagnostic >=30 guard still holds after these9 retirements. No carried UI file in pkg4/pkg6 manifests; seam-row metadata preserved. This is the exact file notice for lead/UI; remaining approved capacity/Mii manifest slice follows separately.

### Initial host gates complete — 2026-11-02 16:38Z
Hos-lock standalone tests at frozen d1e5392 passed20/0, no warnings, exit0; Yoga CLAIMED/RELEASED16:38Z.
Raw log archived in artifacts/trainingrooms-nro-archive/slim/astra-host-gates/astra-hos-lock.log.
Post-build verification: all1119 archive files unchanged; exactly two generated lockfiles (SMD and hos-lock) extra.
No owned claim/compiler/Eden. Surface sl-01 seam/install-line and idle Training row queued; current code d1e5392, HEADeb9331f plus progress-only dirt.
Prepared scratch capacity-retirement-edits.json covers native_service.rs, character_items.rs, native_source_actors.rs with exact old/new/count/reason and source hashes, not applied. Shared managed-source branches retained. No new runtime edit before initial Eden gate.

Runtime admission moved to Yoga tail; own Surface Eden request withdrawn. Shared harness225d16b deployed with its existing deploy.py (read fully) to /home/intpa/trainingrooms-eden-slim-astra, no profile or Eden created yet. Added reference images manually because deploy.py copies macros but not refs; own deployed config macros points at deployed root/macros. Source profile b, display109 unused at preflight, Yoga tile NRO present. Diag hash5030f68f93bf verified after transfer; pilot e30334261370 verified. Local authoritative queue/ledger claim will precede remote solo Up -NoClaim; Down -KeepClaim -Restore then local release after cleanup.

Preparation checkpoint: /tmp/slim/capacity-retirement-edits.json now covers4 source files (23 exact edits in native_source_capacity.rs plus6 across the first3 files). This is a scratch plan, not applied source and not a gate result. The capacity plan removes only probe-gated callbacks, legacy wrappers and event diagnostics; shared managed source branch bodies, guards, native calls and ownership cleanup stay intact. Source hashes bind every plan entry; remaining character_roster/character_sources/kirby_hats/source_lifecycle inline sites still need preparation. Initial Eden sl-01 waits at Yoga tail behind pkg6/ui-screens. No claims or profiles held.

### ACTIVE Eden sl-01 — 2026-11-02 16:57Z
Yoga CLAIMED Codex01a19c9f/slim after fresh queue/free checks. Frozen d1e5392 diag5030f68f93bf, pilot e30334261370 plus tile; own root /home/intpa/trainingrooms-eden-slim-astra, source b, profile sl, display109, shared harness225d16b. All7 deployed script hashes equal source. Preflight21GiB available/254GB free, no Eden/display; own profile initially absent. Source976 mutable files hashed before boot in source-before-sl-01.json.
Up -NoClaim executing via bounded SSH session60359; local ledger holds the authoritative claim. Collect raw outputs BEFORE Down -KeepClaim -Restore; then verify source hashes and remove freshly created own profile (restores initial absence) before local RELEASED/own queue-row removal. No compiler allowed on Yoga during this claim.

### sl-01 runtime failure — 2026-11-02 17:07Z
Stopped/restored and RELEASED Yoga; own queue line removed. Source976 hashes unchanged; fresh private profile deleted back to initial absence. Raw evidence remote trainingrooms-eden-slim-astra/evidence/sl-01. Full final console5689 lines:0 Unmapped,0 panic/abort/crash. Installation result=installed and pilot_present=true, but route ended GATE FAIL training scene best_dist=52.1 limit=30. Hook counters failed: allocations1120 delta100, roster_preload904, cpu_frame192, input_caller_a_note18 plus other startup sites. No runtime pass claimed. Initial runtime gate remains open; approved scratch retirements still unapplied. Lead coordination needed for shared runtime root fixes; continue independent audit/reductions.

### Next approved slice17:10Z
Initial previously-uncompiled code now has host/Switch and seam census evidence; broader shared runtime failure remains explicit. Proceeding under lead instruction to continue approved probe deletions. Exact capacity manifest slice: native_service.rs, character_items.rs, native_source_actors.rs, native_source_capacity.rs, character_roster.rs, character_sources.rs, kirby_hats.rs; remove source_capacity_probe.rs/source_ldn_capacity_probe.rs/source_lifecycle_probe.rs. Conditional lifecycle audit found only probe attribution, no product/test consumer; keep actual ownership release calls and seam metadata. Scratch source hashes/exact replacements verified before application. Cargo/test feature closure updates accompany retirement.

### Capacity retirement applied17:2xZ (ungated)
Manifest transforms regenerated258 carried files; removed three retired probes; both carry verifications pass (pkg6258/pkg442). Removed19features (8empty+10capacity+1lifecycle); explicit20diagnostic inventory replaces arbitrary>=30testfloor. Texture remains. Rawsloc4=101224, reduction3126 thisslice, total5728 versus authorizedebfcb98; simulator1118 unchanged. Budget cut105600 retains4318pending. No fresh compile yet; runtime failure stays open.

### Frozen capacity gate / Mii preparation17:2xZ
HEAD1c0104e, only this progress note changes. Surface root test queued; Yoga release queued. Yoga archive1116files staged trainingrooms-build-slim-astra/1c0104e/src; tarSHA d7b4bc33caafffb7b1bd37da26b4f0cf084e1f5e4a47959a0e3155bb4a40520a matcheslocal. Wrapper same bounded recipe, no compile/claim yet; one own target. Carry258/42 and six size-tool tests pass; sizegate101224/105600.
Next Mii slice only scratch /tmp/slim/mii-retirement-edits.json, not applied: character_roster.rs, mii_native_access.rs, mii_native_cpu_scope.rs, mii/mod.rs; retire six Mii experiment files (identity/saved_file/fullconfig/native_adapter/cpu_adapter/closed_ack). Shared saved/ordinary helper checks stay; preserve existing write_tokens exact-write-set regression in shared primitive module. This is exact manifest/file notice to lead/pkg6; no behavior refactor. Remaining seven Cargo features/testinventory to update at application.

Final archived hook parser correction:43stats rows, last allocations1330 delta110 (earlier progress cited an intermediate read1120/100). Existing eden/hook_stats.py exited1;13nonzero delta rows, first197. Raw verdict archived sl-01/hook-stats-verdict.log. Last per-hook allocation totals: input_caller_a_note:18,cpu_frame:192,live_replay:1,roster_preload:1114,practice_lease:1,named_service:3,menu_draw:1. All remain failures; no exemption.

Normalized1c0104e via operator sloc-fmt.ps1:129580incltools167 =129413excltools; simulator2102excluded; tests44925excluded. Rustfmt1.9.0-stable48a229cea,0failures. Delta thisslice-3928, versusauthorizedbase-6656. Sourceunchanged (scratch normalization only).

Mii scratch preview verified: all exact replacements match; no cfg/cfg! use of the seven retired features remains after simulated deletion. No source writes. Planned product module renames: lobby_stage_probe.rs -> lobby_stage.rs and practice_stage_observer.rs -> practice_stage.rs, module map and declarations plus notice_ids.rs test path; historical seam group strings unchanged. These are still preparation, not shipped/gated edits.

### ACTIVE capacity release gate17:27Z
Yoga claim admitted after queue head/live ledger/interleave grace/process checks. Frozen1c0104e one Switch release, own src/target incremental0,6G/1800s; SSH exec48304. Remote log trainingrooms-build-slim-astra/1c0104e/build-release.log. Release immediately on exit, preserve log/NRO/source identity then queue diag at tail. Surface root test still queued; no local compiler/Eden/profile owned.

Capacity release1c0104e PASS exit0; RELEASED17:28Z, own row removed and diag appended at tail. NRO8823fc87e988a87820f7b73b8f8dd22b8a1410f25da896a7e43a1f8e62acd9bd; raw log preserved. Source verification/warning census pending; no compiler/Eden/profile owned.

Release1c0104e identity check: all1116 frozen files unchanged, no extras; 52.62s exit0. ANSI-stripped source warning census41 (same as prior release; raw ANSI grep falsely found0, corrected before any zero-warning claim). Complete log/NRO/source manifests archived artifacts/trainingrooms-nro-archive/slim/1c0104e-release/.

### Batch decision17:4xZ
No claim/job owned. Withdrew own pending1c0104e host and diagnostic queue rows before execution; release gate remains valid. Next host/Switch gates will cover approved Mii retirement and independent feature-helper lint fixes together, requeuedEND with new committed identity. This avoids redundant intermediate gates; no unrun gate is claimed passed. Expanded exact lint notice: runtime/p1/cpu_pilot.rs plus model/p1/{mod,config,move_observer}.rs (remove unused untested draft encoder/walkoff; test-only helpers retain tests), runtime/mii/native_keyboard/inline_keyboard.rs (four Mii-only helper/result fields, native sdk_calc still called in either feature configuration). Large-enum and shared behavior refactors remain deferred.

### Mii/helper batch applied17:4xZ (ungated)
Six served Mii probe files/sevenfeatures retired through manifest edits; existing write_tokens exact-write-set test retained in shared nativeaccess. Both stage service modules renamed through module map, seam groups untouched. Twelve dead-code allows removed across seven ratchet rows by actual caller/test scope, including native-body-bank Descriptor consumers; no new allows. Six pending rows remain (five large enums and one helpers module), plus approvedhos2. No behavior refactor or hook allocation exemption. Raw98,956 (-2,268from1c0104e; -7,996fromebfcb98); ceiling103,300 retains4,318pending. Carry252/42 verified before final comment/EOF cleanup; full repeat and gates next. Old1c host/diag requests withdrawn, not passed.

### Frozen next gate f07bda2
Committed Mii/helper/stage-name batch f07bda2; all code now frozen, only progress may change. Carry252/42, size98,956/103,300, diffcheck pass; stage service bodies byte-identical except qualified module paths. No new compiler gate yet. Surface root test and Yoga Switch release queuedEND. Yoga1110-file archive d131fdc97ed543cf986a8acbbc521b94b52b4a1980ef9724483d5f9b89b72ec4 verified on both hosts, own root trainingrooms-build-slim-astra/f07bda2; wrapper6G/1800s. Host wrapper /tmp/slim/cargo-host-ordered.py uses OS-level merged FD, source manifests and completed-log SHA receipt (replaces PowerShell reorder-prone capture); one cargo call. No claim/compiler/Eden/profile owned.

Normalizedf07bda2 via operator sloc-fmt.ps1:126710incltools167 =126543excltools; simulator2102excluded, tests44339excluded; rustfmt1.9.0-stable48a229cea,0failures. Reduction2870from1c0104e,9526fromauthorizedbase136069. The next-version15%target remains unmet; current RC criterion is monotone ratchet.

Read-only post-retirement seam census /tmp/slim/f07bda2-seam-census.json: source_lifecycle2 and mii_identity4 now have no local non-test consumer; hitbox3 also absent locally but belongs to pending hitbox integration. All14fullconfig rows still have consumers by offsets in shared native/Mii code and stay. No additional row deletion applied; cross-stream consumer proof required first.

### ACTIVE f07bda2 release17:53Z
Yoga CLAIMED after actual queue head, imgui17:50:37 release/interleave, remaining fairness, full recent ledger/process/disk/memory and1110sourcehash checks. One bounded release6G/1800s, incremental0; SSH exec74813, remote frozenroot/build-release.log. Remote roots30GiB + local19GiB below60GB,252GBfree/21GiBavailable. Release immediately on wrapper exit; archive log/NRO/sourceafter, remove own Yoga row and requeue diag at tail. Surface test still queued; no owned Eden/profile.

Releasef07bda2 PASS exit0 NRO07fd793ddb1d3f5b74485a934888280cf94661298c0cefb260f820d23d0192ed; Yoga RELEASED17:55Z, own release row removed. Surface root test request withdrawn before execution and requeued at Yoga tail, SAMEfrozen1110 source/target for host+Switch; avoids held Surface and no extra target copy. Diag queues after that one host job; no owned claim/compiler/Eden/profile. Sourceafter/warning census/archive next.

Releasef07bda2 postcheck: all1110 source hashes unchanged/no extras;57.33s;41source-located warnings with no new warning messages versus1c0104e. NRO resident3,440,640B (unchanged). Raw/ANSI-stripped log, warning list, NRO and before/after manifests archived artifacts/trainingrooms-nro-archive/slim/f07bda2-release/. Host runner prepared at remote frozenroot/host-f07bda2.py; exact source manifest checked before and after, completed log bound by SHA, inherited RUSTUP_TOOLCHAIN removed so pinned source toolchain applies. Command under6G/1800s: python3 -I host-f07bda2.py <root>/host-test.log test --no-fail-fast. No current claim/job; Yoga queue has pkg4 ahead.

Standalone gate continuity: git diff d1e5392..f07bda2 for crates/hos-lock and crates/smash-match-data (including tests/oracle) is empty; subtree identities match. Earlier20hos-lock and24SMD passing tests apply to identical crate sources. No redundant standalone rerun planned; fresh root test/clippy/no-default and Switchdiag still required for changed root source.

### ACTIVE f07bda2 full host gate18:10Z
Exact worker model/effort reverified after compaction; watcher11182 alive/no alert. Operator priority override observed in shared queue: TrainingRooms first, fork after two runs, imgui last; ledger safety unchanged. Actual Yoga queue head/free and full raw ledger checked,1110 source hashes unchanged, process census clear,232GiB free/21GiB available. CLAIMED18:10Z, one cargo test --no-fail-fast via ordered FD runner,6G/1800s incremental0, SSH exec19428. Release immediately on exit; log frozenroot/host-test.log. No owned Eden/profile.

### f07 first host attempt failed18:12Z — fixture setup root
Released Yoga/removed own row immediately on wrapper exit. Cargo101,69.99s; completed ordered log SHA6ce96061d3034b894f49d07768c2078f8849309be3d819fb5d63755d083c6b2c archived with job/before/after manifests at artifacts/trainingrooms-nro-archive/slim/f07bda2-host-first/. All13 failed targets report missing owned ELF or common NRO; no compiler warning/error and carry/size/lint suites passed. Exact representative errors:
```text
owned 13.0.5 ELF missing at /home/intpa/trainingrooms-build-slim-astra/f07bda2/src/../research/game-13.0.5/main-13.0.5.elf: link ../research, or set SKIP_OWNED_ELF_FACT=1 to skip
owned 13.0.5 common NRO missing at /home/intpa/trainingrooms-build-slim-astra/f07bda2/src/../research/game-13.0.5/nro/lua2cpp_common.nro: extract it (IMPLEMENTATION-PROGRESS-pkg2.md), or set SKIP_OWNED_ELF_FACT=1 to skip
```
Root correction: staged both existing owned binary fixtures outside frozen src, copied from pkg6 verified research artifacts; ELF0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2, common43876f961a80e413370629d32c23af9de6e4cc26b448bab88d5433714a566097. NoSKIP/no source edits. All1110 hashes unchanged; six extra generated __pycache__ .pyc files explicitly inventoried in host-first-inventory.json. Requeued fullhost repeat at Yoga END behind pkg4/UI under new priority. No held claim/compiler/Eden/profile.

Helper feature-scope slice applied in working tree after exact file notice; frozenf07 source is unchanged. Eight manifest transforms across mii_ordinary_native.rs/mii_source_provenance.rs; removed remaining module-wide dead_code ratchet row. Ready construction/decision branches and parked argument now share coexist-or-test scope; no dummy reads, underscore suppression or new allow. Five pending large-enum rows plus approvedhos2 remain. This slice is UNGATED; next validation must include ordinary-player-only Switch build, plus enabled/default and retained host tests.

Helper lint root committed10a8232. Carry252 fullverify and rawsize98,962/103,300 pass; enabled/test arms unchanged, compiler gates still pending. Withdrew queued f07 repeat BEFORE execution and requeued10a8232 fullhost at END, to validate fixture repair plus final helper scope in one necessary fullgate. f07 first failure remains preserved; no unrun repeat claimed passed. Next Switch diag/release/ordinary-player-only and clippy/no-default use10a8232. No claim/job/profile owned.

10a8232 staging complete:1110 files, archive5d6063126e0c822a628c2c2ae5e074c12e7eb2d0cbe160d131f2a5504e778b50, remote trainingrooms-build-slim-astra/10a8232/src exact inventory verified. Existing owned f07 target MOVED to10a8232/target with no job running; old source/logs/dist retained, no duplicate target. Both owned fixtures copied/hash-preserved outside src, runner prechecks them and unsetsSKIP. Switch wrapper has release/diag/ordinary modes (ordinary=no-default+mii-ordinary-player-alpha); one Cargo call each. No gate started. Normalized10a8232=126725incltools167=126558excltools, +15 versusf07, -9511 versusauthorizedbase;0rustfmtfailures. Raw98,962 and ceiling103,300 unchanged.

### ACTIVE10a8232 fullhost18:25Z
Yoga CLAIMED at actualqueuehead after fullrawledger free, no compiler/Eden,1110 hashes checked,231GiBfree21GiBavailable; buildroots35.4GB + local19GiB below60GB. New operatorpriority applied through existing TR_PARALLEL_GRACE=0 (only obsoleteinterleave disabled); actualqueue has NO fork request onYoga, own3minfairness remains180. No sharedharness edit. SSHexec45169 runs one fullhost orderedFD6G/1800s pinned1.98.1 incremental0 noSKIP with precheckedmain/common fixtures. Release onexit; log10a8232/host-test.log, nextgate ordinary-player Switch at queueEND to exercise disabled-coexist lint case.

10a8232 fullhost PASS exit0,70.03s, orderedlogSHA29bdbab26b52007b9d7b18551270085aace7b19b1f06d6e7d79acf57c27d37f9.1110 source hashes unchanged. YogaRELEASED18:27Z, own row replaced at END with ordinary-player-only Switch gate after own3minfairness. No current compiler/Eden/profile; complete inventory/log archive review next. No certified exact test count claimed.

Host10a8232 postcheck complete: rawlog/job/manifests/runner/fullinventory archived artifacts/trainingrooms-nro-archive/slim/10a8232-host; completed log SHA matches jobreceipt. Zero warning headers, zero SKIPPED lines; all1110 files unchanged, only six explicitly inventoried Python cache outputs. Carry252/42, size/lint and unblessed golden regression passed in this fulljob. Runtime mii_source_provenance's local cfg(test) cases are retained but are not reached by this host harness; no execution claim for those cases. Ordinary-player-only Switch gate remains necessary.

Surfaceclippy prepared/queuedEND while Yogaordinary gate waits: fresh exact10a8232 archive1110files at /home/intpa/trainingrooms-build-slim-host/10a8232/src, source-before.json; both owned binaryfixture hashes verified via ownresearch symlink. Orderedrunner host-10a8232.py reuses existing solelocal cargo-targets/trainingrooms-slim (no newtargetcopy). One cargo clippy --all-targets -- -D warnings under6G/1800s incremental0. No claim/job on eitherhost yet.

Mtime preflight correction18:3xZ: independently read lead stale-target finding; our10a8232 host rawlog proves fresh36.73s compile at newpath, not stale acceptance. Explicitly refreshed all1110 staged file times on both own host copies before future gates, with bytehash verification unchanged. No current owned job while touching; no target sharing between active source copies.

### ACTIVE Surfaceclippy18:33Z
CLAIMED after fresh actualhead/fullledger+rawtail/process checks;234GiBfree15GiBavailable, localtargets19.87GB + remote35.4GB below60. Existing harness used with obsoleteparallelgrace0 peroperatoroverride; no actualfork request, ownfairness180 retained. One cargo clippy --all-targets -- -D warnings on frozen10a8232/src via orderedrunner6G/1800s incremental0; exec13533, log /home/intpa/trainingrooms-build-slim-host/10a8232/host-clippy.log. Release immediately onexit thenremove own Surface row/requeue nodefaultEND. Yogaordinary still queued behindpkg4; no Eden/profile.

Surfaceclippy10a8232 PASS0 zero warnings,48.56s; RELEASED18:34Z. Full1110-file inventory identical/noextras; log SHA d30fdd405e36b8da2fc2b290567affba349f5ed86554b7bd618f5169c7d6cfc8 verified/archive slim/10a8232-clippy. OwnSurface row removed and no-default queuedEND afterown3min. No claim/job currently; Yogaordinary remains queued.

Yogaordinary launch refused by process preflight before claim18:36Z: foreign memory-guard-B Eden active (ledger18:36:23Z). Exact census eden1384307, Eden.AppImage1384312, eden-lan-runtim1385315;1110sourcehashes stillunchanged. No owned claim/compiler/Eden.

### Host gates complete for10a8232 —18:47Z
Surface no-default PASS0,5.49s, zero warnings; CLAIMED/RELEASED18:47Z. All1110 files unchanged/noextras. Log SHAaca1b750c2263a05ab63a57be8eefccb9a2f59119578c69ab818a28115b6bab4 bound/archived slim/10a8232-no-default. Fullhost and clippy already pass above. Removed completedSurface row AND unrunYogaordinary row; moved ordinary Switch check to SurfaceEND afterown18:47+3min because foreignmemory-guard-B nowholdsYoga e03. Samefrozen1110src/solelocaltarget, prepared surface-build-10a8232.ps1 uses documented campaign RUSTUP_HOME/CARGO_HOME and cargo-skyline with no RUSTUP_TOOLCHAIN override. No current owned claim/compiler/Eden/profile.

### Restart-safe checkpoint18:5xZ (work continues)
Branchstream/slim HEADc637c8b (docs-only after code10a8232); onlyprogressdoc changes currently. ModelAstra/medium watcher11182 alive. Host10a8232 test/clippy/no-default allPASS0 with exactsource/log receipts archived; raw98,962, normalized126558excludingtools, budget103,300 unchanged. Switch ordinary/release/diag for10a8232 remainUNRUN; f07release07fd793d remains prior evidence only. Surfaceordinary request nowEND afterpkg6targeted row; Yoga requestwithdrawn because foreignmemory-guard-B Eden(e01..e03) continued. No owned claim/compiler/Eden/profile.
Next: fresh Surface queue/rawledger/process/disk/source checks then existing ledger claim, bounded /usr/bin/pwsh -NoLogo -NoProfile -NonInteractive -File /home/intpa/trainingrooms-build-slim-host/10a8232/surface-build-10a8232.ps1 -Mode ordinary. Inspect actual disabled-coexist warnings/errors, preserve rawartifact/source identity; release immediately and requeue defaultrelease/diag. No newsource until necessarygate evidence. Five large-enum lint rows stillpending (hos2approved), baseline-gated roster/Mii refactors have no newauthorization. sl-01runtime allocationFAIL retained; next diagnosticroute uses existing nativeTraininglayout loggate as recordedQUESTIONS, then screenshot. Finalzero-hook/0Unmapped/compliance stillrequired beforeREADY.

### ACTIVE Surfaceordinary Switch18:52Z
CLAIMED after fullrawledger/actualhead/priority+noactualfork request/processidle preflight;1110 sourcehashes unchanged, freshmtimes,233GiBfree14GiBavailable targetupper19.89+35.4GB. Bounded6G/1800s surface-build-10a8232.ps1 -Mode ordinary usescampaign skyline-v3, incremental0, one cargo skyline build --release --no-default-features --features mii-ordinary-player-alpha. Exec92334; rawlog /home/intpa/trainingrooms-build-slim-host/10a8232/build-ordinary.log. Release immediately onexit; inspect compile result/warnings and source identity before claiming helper root closed. No ownedEden/profile.

### Ordinary-player feature gate10a8232 PASS18:54Z
Surface wrapper92334 exited0,71s; RELEASED18:54Z verified in rawledger. NROac944fc22ddbfecc3b72fb13b2225dded3cffc2fb31f72760be6e2e99f0c49cb. All1110source hashes unchanged; full inventory/rawlog/ANSI-strippedlog/wrapper/NRO archived slim/10a8232-ordinary. There are86 warningheaders in this restricted-feature build; none in mii_source_provenance, so removing its broadallow exposed no hidden dead members. Existing helper_snapshot_for unused warning in mii_ordinary_native remains outside the changed decision body; no zero-warning claim for this restricted build. Hostclippy remains zero. Defaultrelease queued at SurfaceEND, diag follows; no ownedjob/Eden/profile. Queue now also records operator throughput override: two independent compiles when no Eden, second only with>=10GiBavailable and>=40GBfree; noTR3min gap. Foreign Eden/claim safety unchanged.

### ACTIVE Yoga defaultrelease19:02Z
Lead19:01 canonicalreconciliation of foreignmemoryguard e05 verified inrawledger; noforeignclaim erased byslim. Moved unrunSurface request toYogaEND, thenpkg4claimedfirst. Followingoperator two-compile override, ownclaim appended via existing ledger.format_line/append after freshmanualnext-unclaimed-head/fullledger checks (old ledger.claim rejects anysecondcompile). Fresh1110hashesunchanged, noEden, MemAvailable23,823,306,752B, disk247,383,904,256B; remote buildrootsupper35,214,312,849B+localtargets20,217,969,173B<60GB. One releasevia existingyoga-build-10a8232.ps1 under6G/1800s, SSHexec81573; ownsource/targetdistinctfrompkg4. Releaseimmediatelyonexit, archiveidentity/warnings, requeuediagEND. NoownedEden/profile.

Release10a8232 PASS0/39.84s; YogaRELEASED19:03Z. NRO3bf44a35270cc95992ffce5e1b2ff6e1f9d593f40801a5c854aa0b536e65fecb resident3,440,640B unchanged; rawlogSHA3a0c2bfa04d0345969f867095261cf4257011bd0665d3d10507173c565aba381. All1110source hashes unchanged, onlysix previouslyrecordedhostPythoncacheextras;41warnings exactlysame warningmessages asf07release. Archive slim/10a8232-release includes raw/cleanlog, sourcebefore/after/inventory, wrapper andNRO.

### ACTIVE Yoga diagnostic19:04Z
RequeuedEND afterrelease thenactualhead/free; all1110sourcehashes and idleprocessfreshchecked, MemAvailable23,435,136KiB, disk247,380,656,128B. Existing ledger.claim usedwith fair_seconds0/parallel_grace0 peroperator/lead19:01 override. One diagvia existingwrapper6G/1800s incremental0; SSHexec96322. Releaseonexit; archiveartifact/source/warnings before runtime. NoownedEden/profile.

Diag10a8232 PASS0/36.76s, YogaRELEASED19:04Z; NRO4fa94558f055532ce884f3d6d5521c23da40afbeab07e8cfac8b2960cbdd261b.40warnings retained,1110source hashes unchanged, onlysixhostpyccaches; archive slim/10a8232-diag raw/cleanlog/wrapper/manifests/NRO. Completeddiag queue removed; nojob/Eden/profile now. All required compilegates for10a8232 nowpassed; runtime/lint debt remains.
Compliance on clean dcf88f3 (docs-only after10a8232), release3bf44a35: exit1, fourmachinefailures unchanged: guardscanner counts repeat initializer as0, README GIF/build/license combinedrow, missingGIF, missingHANDOFF.md exactname. No diagmarkers inrelease; clean-tree checkPASS. Fullreport archived10a8232-release/compliance.md; agent checklist stillneedsruntime/currentintegration. No READY.

### sl-02 prelaunch checkpoint19:0xZ
Allcompilegates for10a8232 pass; dcf88f3HEAD docs-only, progressdirty. Queued Yoga sl-02 after allbuilds; actualhead/free/nootherbuildrow and processidlechecked. Private root trainingrooms-eden-slim-astra/profiles/sl initiallyabsent, sourceb mutableinventory19988files saved source-before-sl-02.json (broader thanprior976manifest), no profilemutationyet. Prestaged diagnostic4fa94558 inownnro dir, pilot e3033426/tileb8313ca2 verified. Next existingharness solo.ps1 Up sl-02 with thisNRO installedaslibtraining_rooms.nro, Tile/Pilot, envTR_FAIR_SECONDS0/TR_PARALLEL_GRACE0 peroverride. Batch install/seam, nativeTraininglayoutgate+actualshot, allocation/fullconsole scan. Onanyfailure DownRestore, independentlyverify restoration/sourceunchanged, archiveallrawreceipts/logs/memory.peak, deleteonlynewownprofiletoinitialabsence, release/removeownqueue. Noforeignprofiletouch.

Prelaunch correction: freshqueue included lead-r5 targeted integration gate behindownEden; initialkeywordassert failedtoclassify its wording ascompile. Human/modelreview caughtit BEFORE Up/claim/profilemutation. MovedownEdenrowEND behindlead-r5; no readycompile bypass. Noownedclaim/process. Admission must classify actualrowmeaning, notonlycargo/Switch/compile keywords.

Completed Surface cache cleanup 2026-11-02T19:17:52.550191+00:00: allthree10a8232NROarchive hashes reverified, noCargo/rustc active locally. Deleted ONLY own cargo-targets/trainingrooms-slim (1848files,1,631,270,784B) after perfilepath/size/mtime manifest archived slim/cache-cleanup-10a8232/surface-target-manifest.json. Sources/logs/NROs/receipts unchanged; Yoga10a8232target and fullhostbinaries retained forcontinuation. FutureSurfacebuild will regenerateowncache; nootherstreamtargettouched.

### Surface sl-02 prelaunch 2026-11-02T19:23:19.295529+00:00
YogaEden requestWITHDRAWNunrun whilebuildscontinue; Surfacefree/noqueuedbuild. Prepared own /home/intpa/trainingrooms-eden-slim-surface harnesscopyof225d16b, configuredsourceg/privateprofile sl/display109/offlineforced; profileinitiallyabsent,1244mutable sourcefilehashes savedsource-before-sl-02.json. Sameverifieddiag4fa94558/pilote3033426/tileb8313ca2 stagedownnro. Freshqueueonlyownrow/fullrawledgerfree/processidle+gvpreflightPASS. NextUpclaims beforeprofilecopy; route usesnativeTraininglayoutgate+screenshots. Afterrun DownRestore, independentplugin/sourcehashverify, archivefulloutputs, deleteonlycreatedownprofilebacktoabsence. ExistingYogaprestage untouched/noownedclaimthere.

### ACTIVE sl-02 Surface19:23Z
CLAIMED19:23 beforeprivateprep; Upcompleted19:23:34. OwnedEdenPID4030437/display109 at trainingrooms-eden-slim-surface/runs/sl-02. Appd483c082/runtimeee3c2e95; plugin/pilot/tilePREPhashesverified. Mute set/save/get canonical0 BEFOREgame.load. Sourceg1244hashes retained; privateprofilecreated, sourceuntouched. Routeexec17104 runs existingroute.py --transport local --training-gate diag:[Ui]layout=info_training resolved (actualargv retains spaces), orderedroute-sl-02.log/receipt. Nextinspectroute/session/install/hookstats, collectthenDownRestore andfullidentityrestoration, releasequeue. DoNOTcompileonSurfacewhileclaimactive.

### 2026-11-02 19:41Z — macro payload root applied, gates pending
Own model gate after compaction: exact thread gpt-6-astra/medium; existing hidden
watcher2674699 alive/no alert. No subdelegation.
Applied the announced pure-model macro_panel change: NameBody::Macro owns one
Box<MacroNameBody>; Screen::Save and held_save hold NameDraft inline. This relocates
the existing allocation rather than adding a second box; external naming drops
its outer allocation. Exact payload data and existing transitions are preserved.
18 exact carry transforms recorded; pkg4 full verify --stage m5q PASS42 and ledger
regenerated. Removed only macro_panel's KNOWN row. Four unapproved enum rows remain.
Existing tests cover native edit validation, placement parking, interruption,
failed-save restoration, and exact Mii naming/save-as; compilation/layout checks
still pending, no acceptance inferred. No runtime/profile/claim currently owned.

### 2026-11-02 19:45Z — 61b0aab validation underway
Code61b0aab frozen1110files, archive55392cf481afa5dc770b3f6c19cb7bcd74bc2f25e12959c01e0b76939ea6e1ba.
Yoga isolated61b0aab/src hashverified and mtimesrefreshed; own10a8232 target moved
while idle, oldsource/logs/dist retained. Research points to prior own verified
fixtures; runner prechecks both SHA and clears SKIP. CARGO_INCREMENTAL=0.
Clippyalltargets-Dwarnings PASS0/23.18s,1110unchanged, rawSHA
ca208187d3ca41590d26a2ea0ae62c2dfcf6e0de55f38967a8e178424e628d4d.
Claim19:43 released19:44; archive slim/61b0aab-clippy includes rawlog/receipt/source.
Used two-compile override beside UI afterfresh23.4GB available/245GBfree andfullledger,
nextunclaimedqueuehead; old single-claim helper not used to override its stale policy.
Fullhost ONEcargo test --no-fail-fast claimed19:44 actualhead/processidle, SSH70029
running6G1800; release and archive immediately oncompletion. No Eden/profile mutation.
Rawsize98,962 unchanged; fullcarry42PASS. Normalizedmeasurement running28295.
NoREADY; fourunapprovedenumrows, nativebaseline/integration/runtimezero/compliance remain.

### 2026-11-02 19:53Z — final macro-slice compiler receipts
61b0aab source1110 unchanged before/after allfivegates.
Fullhost raw7ced10b37e7f45591ef52b2e123c7fa6389c0d7866d9431915cbf23a8aaa82ea:
69 successful result lines, rawsum9259pass/0fail/6ignored, not certifiedunique total
because nestedprocess suites may appear. Zero warnings; relevant exactMii,
nativecaret, validation, placement, interruption andretry tests namedPASS.
No-default freshrootcompile2.01s actualcargo0.
Surface diag77s/40warnings raw70ec54b1cd704afddc0d00d3c5c73757334a490066b7fc84866fe681e8ebda87,
ELFc036adffd79fc1ab6bcb13a3f41df4be6a3f4720a8fdd434e150da7685ccd81a.
Yoga release40.64s/41warnings rawffb029c971a4620d77309a64ed8812bcad59a446bac510ec5c1a0bbb175246f2.
No newwarning suppression; no claimofwarningfreeSwitch. No ordinary-onlyrepeat
requiredfor this puremodel change; previous10a8232 feature-scopeordinarygate retained.
Compiledstacktools readfully; newandbaseline reports preserve failures, no tableedit.

### 2026-11-02 19:54Z — compliance repeated
Clean docsHEAD5644502/code61b0aab, releasee2946b9d: complianceexit1,4mandatorymachinefails unchanged (guardscanner sees repeat-initializer0, READMEbuild/GIFrow, noGIF, exactHANDOFF.mdmissing),29agentrows. Resident3440640B, nodiagmarkers, cleantreepass. Report archived slim/61b0aab-release/compliance.md. CurrentM/C table refreshed withactual61gates andsl02operationalproof; runtime/stack/lintremaininggates stillFAIL/pending, noREADY.

19:57Z stack path audit: baseline/new Heartbeat and Archive shim frames unchanged2432/144. The +176B selected deepestpath enters std::sys::fs::switch::cstr112 + Vec::spec_extend48 + reserve16 beforepanic. Reports retain actualELF identities; no claimthisis caused by macrostate layout. Residentdiag+4096 is observed, cause not independentlyattributed.

### 2026-11-02 19:58Z — keyboard test-surface audit (read only)
Read the complete native inline adapter. Its existing cfg(test) module contains
the 10-unit Mii buffer and 48-scalar/96-unit macro/caret tests, but no root host
target includes that runtime module. Direct inclusion also requires the native
skyline current-thread call, SDK imports and logging boundary; it is not a
self-contained existing host test surface. No native-call stubs, blanket allows,
source-extraction harness or duplicate Text implementation were introduced.
A proper follow-up would move the pure bounded Text/Decision validation into an
existing engine-owned model surface, retain the runtime ABI/callback adapter and
test frozen Enter independently of subsequent Changed events. That is larger than
the prepared two-file representation patch and needs the lead's scoped ownership
decision; no source change or test pass is claimed for the unapplied patch.

### 2026-11-02 20:02Z — cache correction and completed Surface retirement
Read latest lead INBOX: UI's earlier52.06GB/lead47GB estimates omitted deeper
target-host directories; corrected pre-retirement total was73.16GB. Pkg6 owner
retired21,901,351,012B, then UI fresh19:54:55 census found53,306,307,619B globally.
My19:49/19:51 claims cited the then-accepted47GB figure; that global-bound claim is
invalid in retrospect. Those jobs ended19:51 before I read the newer correction.
No further compiler claim or cache growth has been made after learning it.
Deleted ONLY own completed Surface target at20:02:26 after checking compiler
cwd/environ and verifying every61diag archivedfilehash;622files/322,548,096B.
Manifest artifacts/trainingrooms-nro-archive/slim/cache-cleanup-61b0aab/
surface-target-manifest.json saysdeleted. Source, NRO, ELF, rawlogs, receipts and
stack reports retained. OwnYoga61target remains for continuing source work; a
future claim requires a fresh global census, not the superseded47GB figure.

### 2026-11-02 20:04Z — exact guard checker limitation
Reproduced compliance.py's guard regex over every src Rust file. Of849matches,
the sole non-64 literal count is src/model/seams/native_seams/mod.rs:143,
Site::EMPTY's guard: [0; GUARD_BYTES]. GUARD_BYTES is re-exported from model/guard.rs,
whose line13 defines64; Site.guard and Seam.guard are typed[u8;GUARD_BYTES].
This is a valid64-byte repeat initializer, not a zero-length guard. The report's
sizes[0,64] is the literal-hex regex limitation. No initializer expansion, test
waiver or shared checker edit was made merely to turn the scanner green.
61host raw log630/633/850 namesPASS for wp_a2/wp_a3 guard tests and
engine::seams::native_seams::tests::compared_rows_match_the_owned_elf, using the
hash-verified owned ELF. This resolves the machine size finding manually only;
it does not certify every install/ownership/runtime row or change the retained
compliance exit1. Broad guard acceptance remains separately evidenced/pending.

### 2026-11-02 20:08Z — coordination dependency remains open
No new lead integration/native authorization found; lead STATUS/checkpoint still
records its bounded turn closed19:38Z. Requested asynchronously that the primary
resume tr-lead-astra-3 to resolve the existing top QUESTIONS. This is not a grant
of authorization and not a READY/operator-blocker declaration. The goal remains
active, with no owned claims/compiler/Eden/profile/queue. NativeText proposal is
still scratch-only. Do not re-run old-source Eden to fill this wait.
