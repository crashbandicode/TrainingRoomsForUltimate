# Implementation progress, stream f6 (ledger F6: the first-boot group-build hitch)

## Astra lead continuation (2026-11-02)

- Found and fixed missing F6 authoritative guards: `CONTROL_CALLBACK` (+0x655f30) and `CONTROL_FUNCTOR` (+0x6574e0) only had 32/12-byte legacy fingerprints and were absent from ASYNC_BUILD_SEAMS. Added both 64-byte Called rows and gated them through the existing authoritative native_body_bank group. Existing relocated vtable slot comparison remains. Manifest transforms own all three carried-file edits, including a regression changing the final instruction word; full 264-file derivation passes (run-p2858297-i15424256). Rust regression remains unrun until a host is admitted.
- Guard bytes use the existing reviewed gen_sites_pkg6.py ELF window reader against owned main SHA256 0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2. This is ordinary ELF byte extraction, not a REA behavioral conclusion. REA skill read; MCP tools absent, doctor reports aligned registration, no setup changes. No new disassembly was necessary. Playbook scan-offsets.py on the retained 16-plugin card copy: no reference to 655f30/6574e0/4f81510 in any plugin (run-p2851609-i15423561). No physical card action.
- Frozen bd5f4a9 is now superseded before compilation by this guard correction. Do not build the old staged candidate. All hosts held by foreign Eden at 14:56Z, so no compile/claim taken. Replace queued source identity with this commit's new freeze before admission.

- Lead owns this dormant stream now; no additional worker slot. Exact live thread 01a19c96-0e78-7293-a76c-6f46a70be468, gpt-6-astra / medium, one owner-bound watcher. Playbook ae87443 read completely; bounded-validation-evidence skill applied for upcoming acceptance.
- Merged fixed integration ebfcb9800fba1f82b2730b19ed26e231c810a2d6 into f6 from eed09c1. Source merge was automatic. Manifest conflict combined the integration stack transform with all F6 transforms in order; regenerated ledger. Full derivation verified all 264 pkg6 files against frozen m5q (scope run-p2718935-i15286256).
- No compile, Eden or ledger claim in this continuation yet. Source integration is not runtime proof. Remaining: inspect/adopt shared harness, freeze source, obtain admitted job, merged host/Switch gates and f6-05/06 with zero total Unmapped. Retain prior failed runs.
- Existing checklist below is historical; open rows stay open until new source-bound evidence. In particular the first-switch runtime criterion and Kirby native baseline remain unproved.
- Fixed the F6 row driver's false-success path: missing prefetch now prevents input, missing bank/native completion retains the failed lap then exits, and native Reload uses its own completion event. It consumes the shared harness route/session through --harness (or TR_HARNESS), rather than the old admission helper. Three Python regressions pass (run-p2759814-i15344525). The old f6_boot.py orchestration is not used for new runs; shared solo.ps1 owns lifecycle/claims.
- Frozen plugin source bd5f4a9: archive a50729285affd75ece9da9a24bc04c654ab7baf4720557c47d89be3127a2290d, content 4dede3a1c39721b809cfc7d6c1adfda67b404dceb6cffe1486668d126aa96854, 1,121 files extracted and verified under artifacts/trainingrooms-f6-astra/bd5f4a9/src. Later driver/test/progress changes do not alter plugin source; disclose their separate identity at runtime.

2026-11-02. Branch `stream/f6` (from `stream/pkg6` `98d6f5b`), worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-f6`, target
`/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-f6` (Surface), Yoga folder `/home/intpa/trainingrooms-build-f6` (own `target/`,
`host-target/`), Eden labels `f6-NN`, own profile `/home/intpa/trainingrooms-eden-f6/profiles/f6` (display `:97`). Model gate: `claude-opus-5-5` /
high from the session transcript (subagent `agent-abaf2af94e284b397`); hidden watcher `/tmp/f6-watcher.py` (model/effort only, exits with the
owner PID 1069629). Stream rules: `/home/intpa/smash-training-lobby/TRAININGROOMS-STREAM-RULES.md`.

**Playbook followed: `ae87443`** (pulled 2026-11-02, `Already up to date`; README.md, docs/{CHECKLIST,EDEN,GIFS,HOOK-SHARING,MACROS,TILE-CURSOR,WORKFLOW}.md,
tools/PROVENANCE.md read completely; tool scripts read: `tools/build/nro_size.py`, `tools/check/compliance.py`, `tools/eden-solo/{gv.py,solo.ps1,step.ps1}`,
`tools/eden-solo/examples/retry-batch.ps1`, `tools/eden-pair/{solo_surface.ps1,examples/queued-run.ps1}`).

## Task
Root-cause and fix the first-boot group-build hitch of the carried Phase B′ build (`runtime::roster::native_body_bank`): the first
asynchronous group build of a boot paid 75-107 ms of `setup_us` on the game thread (later 10-16 ms; Kirby-first groups' gap 91-109 ms),
evidence `R/stages/m5q/eden/{A,S}/RESULT.md`, `R/c1162/group-build-offthread/{RESULT,FINDINGS}.md`.

## State (restart-safe)
- `206e8bb` diag split (WIP), `a3c30f1` gates + harness. Diag NRO `cfe67ff9…` (`cargo-targets/f6-nro/f6-split-diag-cfe67ff9.nro`, Surface),
  release `74bfad23…` (`f6-release-74bfad23.nro`, Yoga; no `[BuildSplit]` strings).
- **Static finding (owned 13.0.5 ELF + owned decompile `research/phase2-native/native_entry_dispatch_6556b0.c`):** native loader state 8 runs
  `0x654dc0` and `0x655320` on the game thread, then queues a task (`0x392e290(0x10,0x90)`, functor vtable `0x4f81510`, callback `0x655f30`,
  `0x3549720(task+0x10, 0x655f30, 2, 1)`, record `E+0x5a68`); the functor's call slot `+0x30` is `0x6574e0: b 0x64f820`
  (`pointer-tables-03.json` relocation addends; `llvm-objdump`). State 9 waits for it, state 10 allocates the bodies. So native runs
  `0x64f820` (control init: AI params `0x17e8890(…, kind, 0x13)` + `0x2c9900` + context vfunc `+0x90` per context) **on the task worker**;
  the carried Phase B′ calls it on the game thread (the synchronous `0x658af0` order, whose only caller is the respawn `0x65be14`). In the
  stock game `0x64f820`'s only direct caller is `0x658af0`. Hypothesis to confirm in Eden: the first-use cost sits in `control_64f820`.
- `e50fea0` **fix (WIP, unproven until `f6-01` confirms)**: 12 recorded edits of `native_body_bank.rs` (manifest reasons `F6 fix …`):
  `start_async_construction` = native state 8 (`0x654dc0`, `0x655320`, then `0x64f820` queued with functor `0x4f81510` / callback
  `0x655f30`), `queue_task` (one task builder for states 8 and 10), `queue_bodies` (state 10), `settle_construction` (state 9 poll of the
  control task, then state 10 outside the bank lock; `[GroupSwap] bodies queued`), `drain_construction` (a destroyer in state 9 waits
  for the control task and runs state 10, then the existing wait), fingerprints of `0x655f30` and `0x6574e0` + the live slot check of
  `0x4f81510+0x30`. Diag: `[BuildSplit] queue` (state 10 tick), `Work::Queue`.
- Plan (queue file, Surface #3): one Eden claim, two launches: `f6-01` with `cfe67ff9` (measure), and only if `control_64f820`
  carries the first-use cost, `f6-02` with the fix diag NRO (proof: first build of the boot, ordinary and Kirby-first, `setup_us` ≤
  plain-lap tick gap). Fix NRO: Yoga compile line, else one incremental Surface compile at the start of the slot.
- **Eden so far (Surface, display :97, muted, own profile f6; claims 11:49-11:55Z and 11:58-12:12Z):**
  - `f6-01` (split `cfe67ff9`): precondition miss before Training (the integrated install line is 8,142 chars and cut before
    `result=`; the Training reference screenshot is FD while profile g's tile 6 is Kongo Jungle). Gates changed to the install line's
    start and `[CharacterRoster] observed owner=0`. No laps.
  - `f6-03` (split `cfe67ff9`): **cause confirmed.** First group build of the boot, the Kirby-first `[6, b, 1f]`: carried `setup_us`
    130,551; split: `control_64f820` 127,566 µs (98 %), Registry log 1,703, plugin 973, articles 589, unprepare 567, models 93,
    contexts 56, memory 38, the three allocations + task queueing about 190 µs. Settlement tick 8.0 ms (verify/ack 3.3 ms, ledger 2.3 ms).
    Gaps: setup tick 147,833 µs; the largest gap 174,665 µs follows a plain poll tick mid-construction (t49 of 108, near Kirby's body
    settling at 1.07 s), not the plugin's setup or settle. Plain poll ticks up to 26.5 ms. 0 panic/abort/crash/Unmapped (12,072 lines),
    memory.peak 7,323,860,992. Table: `python3 -I tools/f6_split_table.py eden/runs/f6-03/TrainingRooms/practice-diagnostics*.log`.
  - `f6-04` (fix `3c9ff2d5`): booted with `async build enabled … control_slot=0x8149a4e0 expected=0x8149a4e0` (the new state-8 slot
    guard passes live), then a boot-time stall before the main menu: plugin `hook_stats` at +23 s (2 calls) and +37 s (844), nothing
    for the next ~40 s; NVDEC (opening movie) at 58.6 s against 31.6 s in `f6-03`. Black screen. No group code ran. Reported to the
    lead (the tree lacked stream/sync; F8 lock livelock plausible, not attributed). Precondition miss → next labels.
- `8c8e7e9` merged `work/implementation` 97648b8 (stream/sync). `carry_pkg6 verify` reports `seam_install.rs` DIFFERS, from
  work/implementation's 73831b8 (hand edit, no manifest entry); reported, not mine.
- Next (Surface queue, end): one incremental diag compile of the merged tree (fix + `[BuildSplit] stall`), then one claim:
  `f6-05` proof (fix), `f6-06` native-Reload Kirby baseline (`eden/settings/f6-native.json`: preload, bank and group swap off).
- Found in passing (not changed, reported): on a native allocation failure mid-loop the carried path returns `Err` with earlier
  bodies' construction tasks still queued and untracked, and the rollback then destroys those bodies (latent, allocation-failure-only).

## Scripts written by this stream (playbook tool check)
| Script | Playbook tool | Why the playbook does not cover it |
| --- | --- | --- |
| `eden/f6_eden.py` | `tools/eden-solo/gv.py` + `solo.ps1` (and `eden-pair/solo_surface.ps1`) | Those are hard-wired to other workspaces (gsp-visualizer `ROOT`, profile `b`/`y`, owner, display `:68`, labels `eNN`, a local ledger on the Yoga; solo_surface drives the Surface from the Yoga for elite-rematch). This is the stream's copy (pkg2 `s2_eden.py` → pkg6 `s6_eden.py`, itself adapted from gv.py + solo.ps1) with the same steps (hard-linked profile copy, set-aside, `eden-run.py` 8 GiB bounded launch, claim/release) plus what the playbook lacks here: the Surface-local run with the Yoga ledger over ssh, `settings.json` in the copy, `core.volume=0` through the CLI before boot (AGENTS.md), and `HH:MM:SS` ledger stamps (fix of a bug that made released claims look open). |
| `eden/route_f6.py` | `tools/eden-solo/to-css.ps1`, MACROS hybrid, TILE-CURSOR | Copy of pkg6's `route_s6.py`: the MACROS "hybrid" (recorded segment macro `tr-main-to-stage-select`, screenshot gates against refs) and tile-cursor steps with the `settled` log gate; to-css.ps1 goes to Quickplay with fixed sleeps, not to Training. |
| `eden/f6_rows.py`, `eden/f6_boot.py` | `tools/eden-pair/examples/queued-run.ps1`, `eden-solo/examples/retry-batch.ps1` (shape) | The row driver of this task (cycle chords gated on `[CharacterRoster] complete`, screenshots) and up→route→rows→collect→scan→down→restore in one bounded call with down/restore in `finally`; the playbook examples are elite-rematch / judge-log specific. |
| `tools/f6_split_table.py` (+ `test_f6_split_table.py`, `tests/f6_split.rs`) | none (WORKFLOW §6 asks for it) | The committed, host-tested script that turns the `[BuildSplit]` lines into the evidence table. |
| `/tmp/f6-watch/{gate,yf6,yhost}.ps1`, `claims.py`, `claim.py`, `free.py` | `solo.ps1` claim logic | Host-local claim + one cargo call + release (also on failure) for compile claims, not in the repo. |

### `eden/f6_eden.py` against the playbook's `tools/eden-solo/gv.py` + `solo.ps1` (for the lead's parameterised version)
Hard-wired values changed (gv.py / solo.ps1 value → f6 value):
- `ROOT` `/home/intpa/gsp-visualizer-work` → `/home/intpa/trainingrooms-eden-f6`; profile `profiles/y` → `profiles/f6`.
- `SOURCE` Yoga profile `b` → on the Surface `ssbu-desync-1029/profiles/g` (Yoga: `b`).
- `OWNER` `Claude 88c0f449… gsp-visualizer` → `Claude 0caa72b8/f6-eden`; display `:68` → `:97`; label pattern `e\d{2,}` → `f6-\d{2}`.
- `OUTPUT` `ultimate/gsp-visualizer` (+ quiet-elite, elite-rematch) → `ultimate/TrainingRooms` (+ tile cursor, cpu pilot).
- `FORCED` `lan_online=true, lan_online_tls=true, airplane_mode=false` → `pauseWhenInBackground=false, volume=0, volume\default=false`, and on the Surface `lan_online=false, airplane_mode=true`.
- Game path: Yoga `~/Games/Roms/Switch/…xci` → Surface `~/eden-assets/games/01006A800016E000/base/…xci`.
- Ledger: local `Add-Content` on the Yoga file → from the Surface, the line is shipped to the Yoga over `scp`/`ssh` (the one ledger lives there).
- Claim check: `(CLAIMED|RELEASED): (\S+ \S+)` per owner → the same plus a 12-hour staleness cut, the host named by `on <host>`, and `HH:MM` or `HH:MM:SS` stamps.
- GPU lock (`flock /tmp/opencode/yoga-gpu-reservation.lock`): kept on the Yoga, skipped on the Surface (no lock there). Launch runtime `5400` → `3600` s.
- Display unit/socket names `gv-<label>-…` → `trf6-<label>-…`.
Added steps (not in the playbook tools): `settings.json` copied into the profile copy before boot; `core.volume=0` set, saved and read back through the control API before `game.load` (AGENTS.md); `collect`, `scan`, `restore`, `reload` subcommands.

## Checklist (docs/CHECKLIST.md at `ae87443`)
| Requirement | Tag | Result | Evidence |
| --- | --- | --- | --- |
| crate-type cdylib only | M | pass (unchanged) | `Cargo.toml` (`compliance.py` row) |
| release profile s / lto / cgu 1 / abort; skyline =0.6.0; toolchain 1.98.1 | M | pass (unchanged) | `Cargo.toml`, `rust-toolchain.toml` |
| pure logic in model with host tests; runtime only for Switch | M | pass | `src/model/build_split.rs` (6 unit tests), `tests/f6_split.rs` (2); `src/runtime/roster/build_split.rs` Switch-only |
| gates: test, clippy -D warnings, no-default-features, skyline release, diag | M | partial | Yoga `a3c30f1` tree: test 63 binaries 8,055 passed 0 failed (owned-ELF facts skipped there: Surface run **open**), clippy 0, no-default-features ok; Switch release `74bfad23` (Yoga), diag `cfe67ff9` (Surface) |
| no heap in hooks (`heap_allocations=0`) | M | open | meter uses fixed statics + stack lines; install line of `f6-01` to read |
| fixed buffers, writer thread, stack paths | M | pass | lines through package 1's `diag_log` ring (no new writer) |
| root cause fixed or workaround approved | M | open | measurement `f6-01` pending; no workaround coded |
| version guard 13.0.5 | M | pass (unchanged) | package 1 boot |
| 64-byte guard on every site / called function / relied instruction | M | open | the fix adds called natives (`0x655f30` callback, vtable `0x4f81510` slot): fingerprint rows to add with the fix |
| card scan before the site is chosen | M | N/A: no new hook site; the fix calls game functions the carried path already calls or the native loader calls (scan-offsets.py run planned for the two new called addresses with the fix) |
| hook sharing / chaining | C | N/A: no hook added or moved |
| hook after prologue / call instead of hook | M | pass | calls only, no hook |
| no diag code or strings in shipped NRO | M | pass | `74bfad23`: 0 `BuildSplit` strings; `compliance.py` row at READY |
| shipped NRO writes only its install line | M | pass (unchanged) | meter is `cfg(feature = "diag")` |
| online rows (read-only review, replay A/B, two consoles) | C | N/A: Training-only code path (the hot swap runs in Training / the lobby practice, never in an online match) |
| Eden claim written and released, also after failure | M | open | `f6_boot.py` releases in `finally`; ledger lines at `f6-01` |
| one Eden per host, MemAvailable >= 10 GiB, not profile a / :61 | M | open | harness asserts (`up`, `launch`) |
| profile copy hard-linked | M | pass | `du -sh` source `g` 6.5G + copy together 791M (game content linked), `PREP-f6-01.json` |
| link identical / copy written files | M | pass | harness copies config/saves/plugins, links `READ_ONLY` trees |
| one target per source copy; df >= 40 GB; build products deleted at the end | M | open | targets `cargo-targets/trainingrooms-f6`, Yoga `trainingrooms-build-f6/{target,host-target}`; df Surface 188 GB, Yoga 253 GB; deletion manifest at the end **open** |
| waits gated on log line / screenshot | M | pass (scripted) | `route_f6.py` (screenshot + tile log gates), `f6_rows.py` (diag line gates with caps) |
| failed precondition aborted, retried under the next label | M | open | route exits 2 on a missed gate; retry as `f6-0N+1` |
| shortest ruleset | M | N/A: Training mode, no match (no stock/time limit applies) |
| profile restored after the run | M | open | `f6_boot.py` runs `restore` in `finally` |
| console.log scanned; run.json, memory.peak kept | M | open | `collect` + `scan` in `f6_boot.py` |
| every local command under memory/time guard | M | partial | cargo under `systemd-run … MemoryMax=6G timeout 1800`; Eden under `eden-run.py` 8 GiB / 3600 s; the profile prep ran unguarded once (file copy, my own root) |
| outputs kept under the label | M | open | `eden/runs/<label>/` |
| README / GIF | M | N/A for this stream: no README change (the package's README is the lead's); evidence is the Eden doc |
| dated evidence doc | M | open | `eden/EDEN-F6.md` after `f6-01` |
| numbers from a tested script | C | pass (tool) | `tools/f6_split_table.py`, `tools/test_f6_split_table.py`, `tests/f6_split.rs` |
| no operator quotes, no claims beyond tests | M | pass | |
| handoff current | M | pass | this file |
| release / tag / card | C | not authorized: no order (nothing pushed, tagged or written) |
| card connect/disconnect by the user only | M | pass | no card access |
| nothing pushed / tagged / written without order | M | pass | |
| model/effort gate and watcher | M | pass | see header |
| spawned workers read the playbook | M | N/A: no workers spawned |

## Handover (2026-11-02 ~14:1xZ, operator order: the lead role moves to the Codex Astra lead)
- Branch `stream/f6`, worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-f6`, head = the commit `WIP: handover to Astra
  lead` (after `c426fe0`); it contains the merge of work/implementation 97648b8 (`8c8e7e9`). **Uncompiled since the merge.**
- Targets: Surface `/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-f6` 302 MB (no incremental dirs); Yoga
  `/home/intpa/trainingrooms-build-f6` 1.5 GB (5.1 GB incremental deleted; manifest `cargo-targets/f6-nro/DELETED-incremental-20261102T1302.txt`).
  Evidence NROs in `/home/intpa/smash-training-lobby/cargo-targets/f6-nro/`: `f6-split-diag-cfe67ff9.nro` (split meter, pre-merge),
  `f6-fix-diag-3c9ff2d5.nro` (fix, pre-merge), `f6-release-74bfad23.nro` (pre-fix release, no meter strings).
- **Proven:** the first-boot hitch is `0x64f820` (control init) run on the game thread. In f6-03, the first build of the boot spent 127,566 of
  130,551 µs `setup_us` there (`eden/EDEN-F6.md`, `eden/runs/f6-03/`,
  `python3 -I tools/f6_split_table.py eden/runs/f6-03/TrainingRooms/practice-diagnostics*.log`). Native runs it on the task worker (state 8
  task, functor `0x4f81510`, slot `0x6574e0: b 0x64f820`, callback `0x655f30`). The fix's new live guard passes
  (f6-04: `control_slot=0x8149a4e0 expected=0x8149a4e0`). Host gates of the pre-merge tree passed on the Yoga (8,055 tests, clippy 0,
  no-default-features).
- **Not proven yet:** the fix on Eden. Done criterion: first switch of a boot with `setup_us` <= the plain-lap tick gap, the Kirby-first group
  `[6, b, 1f]` too, R5 delta 0, no Unmapped. Also not done: the native-Reload baseline of the Kirby gap (174.7 ms after a poll tick
  mid-construction in f6-03; the lead wants the game's own Kirby build measured the same way), and the host and Switch gates of the merged tree.
- **NEXT:**
  1. Surface compile (queued): `pwsh -File /tmp/f6-watch/gate.ps1 -What 'Switch diag build' -Kind switch -CargoArgs 'build --release
     --features diag' -Log /tmp/f6-watch/sw-diag-merged.log`, with `CARGO_INCREMENTAL=0`. The helpers in `/tmp/f6-watch` are `gate.ps1`,
     `sw.ps1`, `cg.ps1`, `claims.py`, `claim.py`, `yf6.ps1` and `yhost.ps1`, and all claims go through `eden/f6_eden.py`. Copy the NRO to
     `cargo-targets/f6-nro/f6-fix-diag-<sha8>.nro`.
  2. One Eden claim (queued; check the imgui-nvn-sol / tm-f8-terra / tm-f8-sol interleave first):
     `cd eden; python3 -u -I f6_boot.py f6-05 --nro <nro> --then f6-06 --nro2 <nro> --settings2 settings/f6-native.json --then-always --p1 14 --cpu 4`.
     f6-05 is the proof; f6-06 is the game's own Kirby Reload with preload, bank and groups off. Read `[BuildSplit] setup/queue/settle/gaps/stall`.
     If f6-04's black-title stall recurs on the merged tree, stop and report it as a P1 boot hang (lead ruling).
  3. Gates with `CARGO_INCREMENTAL=0`: test, clippy `-D warnings`, no-default-features, Switch release + diag, `nro_size.py`,
     `compliance.py --nro <release>`.
  4. EDEN-F6.md rows, this checklist, the lead report.
- **Blockers / defects outside f6 (reported):**
  - `carry_pkg6 verify` fails on work/implementation: `seam_install.rs` was edited by 73831b8 without a manifest edit (integrator-3).
  - The install line is cut at about 8,142 characters (integrator-3).
  - In-hook allocations rise about 220 per 10 s with no swaps (pkg6 R5).
  - f6-04's boot stall on the pre-sync tree.
- Open claims: none (last `RELEASED` 12:12Z, queue line deleted). Watcher stopped. Eden profile `/home/intpa/trainingrooms-eden-f6/profiles/f6`
  restored to source g's plugin set.
