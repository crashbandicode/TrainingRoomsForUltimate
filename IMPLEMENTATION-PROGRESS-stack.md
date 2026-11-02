# stream/stack — thread stacks (2026-11-02)

Lead `0caa72b8`; worker Claude Opus 5.5 high (agent a2594031). Playbook `ae87443`. Branch `stream/stack` from
`work/implementation` 27d84f4, worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-stack`, host target
`cargo-targets/trainingrooms-stack`, Switch target `cargo-targets/trainingrooms-stack-sw`, Eden labels `st-NN`.

## Playbook

Playbook followed: `ae87443` (pulled 2026-11-02 at the start and again ~12:0xZ; read in full at the start: README.md, every
docs/ file, tools/PROVENANCE.md; tools read before use: eden-solo/solo.ps1, eden-solo/gv.py, build/nro_size.py,
check/compliance.py).

### Checklist (docs/CHECKLIST.md), final 2026-11-02

| Item | Tag | Result | Evidence |
| --- | --- | --- | --- |
| crate-type cdylib only | M | pass | compliance.py (aa18732 tree, release 2b1c0095) |
| release profile s/lto/cu1/abort, skyline =0.6.0, toolchain 1.98.1 | M | pass | compliance.py |
| pure logic in model with host tests; runtime switch-only | M | pass | `model::thread_stacks`, `BoxedSlot` with host tests (`tests/thread_stacks.rs`, trace_slot unit tests); `runtime::threads` switch-only (host stub only for the diag_log test harness) |
| gates, one cargo call each | M | pass | Ally WSL 1a14480: cargo test 7966 passed 0 failed, clippy -D warnings 0, no-default-features 0, Switch release `2b1c0095` + diag `03ecd495`, 0 warnings (`/tmp/stack-a2594031/b1a14480/GATES.txt`); aa18732 (boot split) gated in integrator-3's merged-tree build (lead decision b) |
| no heap in hooks | M | pass | install line `result=installed heap_allocations=0` in st-02..st-11 `events.log`; `BoxedSlot` put/take/finish/flush allocation-free (host test with the counting allocator) |
| fixed buffers, a writer thread on a static stack, paths built on the stack | M | pass | every plugin thread on a page-aligned static stack + static `ThreadType` through `nn::os::CreateThread/StartThread` (gsp-visualizer's pattern), sized by the measured table (`tests/thread_stacks.rs`); the trace writer's state in boot-made buffers; paths are `FixedPath` |
| root-cause fixes / approved workarounds | M | pass | the fault's cause (11 KB trace moved by value → 59.7 KB frame) removed, not a bigger stack; no workaround shipped |
| version guard, 64-byte site guards, card scan, hook placement | M | N/A | no hook site added or moved (probes are statements inside existing hooks); compliance's "guard sizes [0, 64]" row fails on the integration head too (repository-level, not this stream) |
| hook sharing / chaining stand-ins | C | N/A | no new or changed hook site |
| no diag code or strings in the shipped NRO | M | pass | compliance.py: no diag markers; the diag threads' names and stacks absent from 2b1c0095 (`llvm-strings`: 0 for tr-probe, tr-practice-trace, tr-state-diag, tr-diag-reload, tr-macro-diag) |
| shipped NRO writes only its install line | M | pass | `[ThreadStack]`/`[HookStack]` are diag-only |
| online rules | C | N/A | no online path changed |
| Eden: claim/release, one per host, MemAvailable, never profile a / :61 | M | pass | ledger CLAIMED/RELEASED per run st-02..st-12 (solo.ps1 copy), display :99, MemAvailable checked by solo.ps1 (23 GB at st-02) |
| Eden: hard-linked profile copy | M | pass | `gv.py prep` copy of ssbu-desync-1029/profiles/b at `/home/intpa/trainingrooms-eden-st/profiles/y` (read-only content hard-linked) |
| disk: df, one target per copy, products deleted with a manifest | M | pass | df 173 GB (Surface) / 238 GB (Yoga); targets `trainingrooms-stack` (host), Ally `c1161-targets/TrainingRoomsForUltimate-stack-ally`; cleanup below |
| waits gated on a log line / screenshot | M | pass | `st_run.py`: install line, `hook_stats` beat, diag request lines, screen matches on the route |
| failed precondition retried under the next label | M | pass | st-01 → st-02 (driver), st-10 → st-11 (record row), st-11 → st-12 (stopped at the lead's release order) |
| shortest ruleset | M | N/A | Training mode, no match |
| profile restored, console.log scanned, run.json + memory.peak kept | M | pass | scan 0 in st-02..st-12 (`eden/runs/st-NN/scan.txt`); run.json and memory.peak kept per run (peaks 4.99–6.23 GB); the profile is this stream's own copy |
| every local command under memory/time guard | M | pass | systemd-run scopes for every cargo, analysis and Eden driver run |
| outputs per label | M | pass | `eden/runs/st-NN/` |
| README / GIF | M | N/A | no user-visible change; compliance's README/GIF rows fail repository-wide (lead) |
| dated evidence doc | M | pass | `eden/EDEN-STACK.md` |
| numbers from tested scripts | C | pass | `tools/stack_table.py` (+ `test_stack_table.py`), `tools/stack_depth.py` (+ `test_stack_depth.py`) |
| no operator quotes, no claims beyond tests | M | pass | this file, EDEN-STACK.md |
| handoff current | M | pass | this file; F7 text and numbers sent to the lead (ledger row F7) |
| release/card | C | not authorized | nothing tagged, pushed or written to a card |
| card handling by the user | M | pass | no card access |
| model/effort gate and watcher | M | pass | claude-opus-5-5 / high (transcript agent-a2594031d8d5c0534); watcher `/tmp/stack-watcher-a2594031` |

compliance.py (`/tmp/stack-a2594031/compliance.md`, aa18732 + release 2b1c0095): 4 machine rows fail — guard sizes [0, 64], README GIF, docs/media GIF, `HANDOFF.md` — all four fail identically on work/implementation (repository-level, not introduced here); every other machine row passes; agent rows resolved in this table.

### Scripts written by this stream

| Script | Why the playbook does not cover it |
| --- | --- |
| `tools/stack_depth.py` (+ `tools/test_stack_depth.py`) | static frame / call-graph depth and the thread-table and hook budget checks from the ELF; the playbook has no stack analysis |
| `/tmp/stack-a2594031/tools/host.ps1` | one host cargo call under the playbook's systemd-run guard with this copy's target dir (a wrapper, not a new procedure) |
| `/tmp/stack-a2594031/tools/sw-build.ps1` | unused: replaced by the repository's `eden/ally/ally-build.sh` (Surface held) |
| `/tmp/stack-a2594031/tools/ally-pack.ps1` | the packing step of `eden/ALLY-SETUP.md` §3 as one script |
| `/tmp/stack-a2594031/y-*.ps1` | read the claim ledger (open claims) and append one claim/release line, as `solo.ps1` does for its own owner |
| `/tmp/stack-a2594031/tools/{add_edits,probes}.py` | record clean-room edits as exact-text manifest edits (the carry tools have no add-edit command) |
| `/tmp/stack-a2594031/tools/{find_fault,callers,memcpys}.py` | one-off ELF queries for the fault attribution and the frame analysis |
| `eden/stack/gv.py`, `eden/stack/solo.ps1` | copies of the playbook's `tools/eden-solo/gv.py` / `solo.ps1` (ae87443) with a small diff (below); for the playbook as an operator decision |

### Diff of the eden-solo copies (for the playbook)

`gv.py` (9 lines changed): `ROOT` → `/home/intpa/trainingrooms-eden-st`; `OWNER` → `Claude 0caa72b8/stack-eden`; `OUTPUT` →
`ultimate/TrainingRooms`; `OTHER_OUTPUTS` → the tile cursor only; `FORCED` adds `volume=0`, `volume\default=false`
(operator rule: muted, persisted in the profile); label pattern `st-\d{2}`; set-aside folder name. Why: the playbook
copy hard-codes gsp-visualizer's root, owner, output folders and `eNN` labels; a parameter for each would let it run as is.
`solo.ps1` (14 lines changed): the same root, owner, label pattern; display `:99`; the gpu-lock unit names; the ledger
texts; and before `game.load` a `settings.set core.volume 0` + `settings.save` + `settings.get` through the control API
(operator rule 2026-10-09); the claim check counts only claims whose line names the Yoga (the playbook copy refuses
while any owner holds a claim on any host, which never clears with several streams on three hosts). Why: same
hard-coding, plus the mute step and the per-host claim rule the playbook copy lacks.
`eden/stack/st_run.py` drives the runs: Up/Down through the copies above, pkg4's screen-gated `route4.py` steps and
reference shots for the route to Training, the diag request files for the busy rows, then fetch and console scan.
`tools/stack_table.py` (+ `tools/test_stack_table.py`) turns the `[ThreadStack]`/`[HookStack]` lines into the tables.

## The fault

Eden s4-07 (pkg4, Yoga) and u4b-02 (u4b, every boot) at plugin boot: `Unmapped Write64 @ …81b80`, a 0x20 SDK frame below a
6,240 B SDK frame, return to 0. SP `…9b90`, saved FP `…b3f0`, R28 `…9e38` in both runs (bases differ): a deterministic
depth. Stack top `…92000` (the page above R28), bottom `…82000`: a 64 KiB stack, overrun by 0x480.

Thread: `tr-trace-writer` (`runtime/compat/filesystem.rs`, default feature `mii-editor-alpha`, shipped too). The diag ELF
of 27d84f4 gives its body (`__rust_begin_short_backtrace::<compat::filesystem::init::{closure#0}>`) a 59,712 B frame:
the Mii editor `Trace` (FixedVec<Event,192> + input checks, about 11 KB) is moved by value through
`LatestSlot::take`/`restore` and the `Option`. thread_start 112 + shim 16 + 59,712 = 59,840 B before any call, against
60,432 B from the top to the SDK frame's saved FP: the remaining ~0.6 KB is the Rust frames between them.
(`tools/stack_depth.py <elf> entry …` and `frames`; the callee pattern is not in our NRO.)

## Fix

- `engine::lobby::trace_slot::BoxedSlot<T: Copy>`: two heap buffers made at init (`prepare`, copied from
  `Trace::EMPTY` straight to the heap); `put(&T)` copies in place; the writer swaps buffers and serialises by
  reference (`flush_boxed`). No allocation after `prepare` (`tests/thread_stacks.rs`, counting allocator).
- `model::thread_stacks`: one row per thread (17): name, stack size, static bound, measured high-water, the runs that
  measured it. `required = max(static, measured) * 1.25 + 16 KiB SDK reserve`, whole pages. `tests/thread_stacks.rs`
  checks every row and bans `stack_size(` / `thread::Builder` / `thread::spawn(` outside `runtime::threads`.
- `runtime::threads::spawn(ThreadId, f)`: the only spawn. Diag: `[ThreadStack] spawn name base size top sp kind`
  (svcQueryMemory at SP), the stack painted below SP, `[ThreadStack] high_water name used size free low` on the
  heartbeat's 10 s beat when it grows, `exit` lines for returning threads, the boot thread across `boot::run`.
- `tools/stack_depth.py <elf> table`: each row against the diag ELF (exit 1 on a shortfall); `tools/test_stack_depth.py`.

## Eden st-NN (Yoga, lead-granted slot 2026-11-02 ~11:52Z)

Build: 6541f2d diag `5d21cf2f…` (static nn::os stacks, lock-free diag registry) + tile cursor `b8313ca2…` + CPU pilot
`e3033426…`; profile `/home/intpa/trainingrooms-eden-st/profiles/y` (hard-linked copy of ssbu-desync-1029/profiles/b), muted
(core.volume 0 set and saved through the control API before `game.load`), display :99.

| Label | Kind | Result |
| --- | --- | --- |
| st-01 | cold | not booted: my solo.ps1 claim-check edit read `$Matches` after a second `-match` (fixed); no claim, no Eden |
| st-02 | cold | boot clean (install line, every `[ThreadStack] spawn` + `high_water`, console scan 0); the driver's report gate waited for a second heartbeat line that is only written when the mark grows (fixed: gate on `hook_stats`) |
| st-03..st-05 | cold | pass, console scan 0 |

First numbers (st-02): the boot thread is Skyline's, **64 KiB** (`spawn name=boot … size=0x10000`), and boot used
38,160 B (static 37,520 B: `run_inner` 26,624 B with two 8,040 B `Settings` copies). `tr-trace-writer` peaks at 1,696 B
(was ≥ 61,968 B). `[HookStack]`: the pipeline ticks run on the main thread (1 MiB region, ~1 MB free at entry);
`ui_archive` / `ui_pass_calculate` run on a stack inside a 156 MB heap block (a fiber), where svcQueryMemory cannot
bound the stack: such sites are marked by `kind=` from the next build and are not read as stack room.

## Final report (2026-11-02 ~12:4xZ)

- **Fault:** `tr-trace-writer` (64 KiB) ran off its stack at every boot where the SDK file path under it went deep (s4-07, u4b-02,
  ui-09: identical SP/FP offsets). Cause: the 11 KB Mii editor trace moved by value through the slot → a 59,712 B frame.
  Fix: `BoxedSlot` (two buffers made at init, copy in place, serialise by reference): static depth 61,968 → 2,624 B, measured
  1,696 B.
- **Every thread:** one host-checked table (`model::thread_stacks`), static page-aligned stacks through `nn::os` (no heap for
  stacks or names), diag `[ThreadStack]` spawn/high-water lines (painted stacks, lock-free registry), sizes = max(static,
  measured) × 1.25 + 16 KiB from Eden st-02..st-12. Boot loaders and the captures share one 40 KiB stack. Diag-only threads
  exist only in diag builds (names too).
- **Hooks:** `[HookStack]` entry room per site; main-thread hooks ≥ 1,021,280 B free (native service 1,032,672 B), the
  `other` sites' 896 KiB thread 523,536 B; fiber-hosted sites marked. Budget gate `tools/stack_depth.py budget`
  ((523,536 − 16 KiB) / 1.25 per path, 16 KiB on fibers): 134 roots, 0 over. No stack switch needed (lead).
- **Boot thread** (Skyline's, 64 KiB): measured 38,160 B pre-merge; on the merged tree static 43,776 B → aa18732 keeps boot's
  sections out of its frame; integrator-3 checks `boot::run` ≤ 39,321 B in the merged-tree build (lead decision b).
- **Resident (release):** 2b1c0095 at 1a14480: .bss 413,696 B, resident 3,321,856 B (3,244 KiB); 6541f2d with unmeasured static
  stacks was 4,264 KiB; before static stacks 2,816 KiB plus ~1.15 MB of heap stacks. The merged-tree figure comes from
  integrator-3's build.
- **Commits:** stream/stack up to aa18732 (merge of work/implementation 92f5143 at 1a14480); gated 1a14480; aa18732 gated in
  the merged-tree build.
- **Open (moved to integrator-3's merged-tree Eden boot by the lead):** recording row, menu open/close, the settings-reload
  stall seen in st-11 (stream/sync's livelock class, no fault line).
- **Carried edits:** recorded in the pkg6/pkg4 manifests (verify 261/42) and CARRY-OVER `## stack`.
- **Cleanup:** after integrator-3's build: delete `cargo-targets/trainingrooms-stack`, the Ally target
  `/home/intpa/c1161-targets/TrainingRoomsForUltimate-stack-ally` and `C:\Users\intpa\stack-pack`, with a manifest; NROs, logs and
  receipts stay under `eden/runs/st-NN/` and `/tmp/stack-a2594031/b*/`.
