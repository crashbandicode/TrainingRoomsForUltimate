# stream/stack — thread stacks (2026-11-02)

Lead `0caa72b8`; worker Claude Opus 5.5 high (agent a2594031). Playbook `ae87443`. Branch `stream/stack` from
`work/implementation` 27d84f4, worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-stack`, host target
`cargo-targets/trainingrooms-stack`, Switch target `cargo-targets/trainingrooms-stack-sw`, Eden labels `st-NN`.

## Playbook

Playbook followed: `ae87443` (pulled 2026-11-02 at the start and again ~12:0xZ; read in full at the start: README.md, every
docs/ file, tools/PROVENANCE.md; tools read before use: eden-solo/solo.ps1, eden-solo/gv.py, build/nro_size.py,
check/compliance.py).

### Checklist (docs/CHECKLIST.md)

| Item | Tag | Result | Evidence |
| --- | --- | --- | --- |
| crate-type cdylib only | M | pass (unchanged) | Cargo.toml; compliance.py before READY |
| release profile s/lto/cu1/abort, skyline =0.6.0, toolchain 1.98.1 | M | pass (unchanged) | Cargo.toml, rust-toolchain.toml |
| pure logic in model with host tests; runtime switch-only | M | pass | `model::thread_stacks` + `tests/thread_stacks.rs`, `BoxedSlot` tests; `runtime::threads` |
| gates: test, clippy -D warnings, no-default-features, release, diag (one cargo call each) | M | open | Ally WSL run 1 (60e0ad9): host-test failed (runtime_diag_log harness lacked `threads`, fixed ab6ef6c); rerun pending |
| no heap in hooks (counting allocator, heap_allocations=0) | M | open | BoxedSlot put/take/finish allocation-free (host test); Eden install line pending |
| fixed buffers, a writer thread on a static stack, paths built on the stack | M | open (core item) | every thread from one host-checked size table, measured in diag; the stacks are still allocated by std at spawn (not static arrays): proposal to the lead pending the st-01 numbers |
| root-cause fixes / approved workarounds | M | pass so far | trace writer: by-reference boxed slot (no size bump); no workaround shipped |
| version guard, 64-byte site guards, card scan, hook placement | M | N/A for this stream | no hook site added or moved (the headroom probes are statements inside existing hooks) |
| hook sharing / chaining stand-ins | C | N/A | no new or changed hook site |
| no diag code or strings in the shipped NRO | M | open | `headroom` is an inline no-op without `diag`; compliance.py on the release NRO pending |
| shipped NRO writes only its install line | M | pass (unchanged) | the [ThreadStack]/[HookStack] lines are diag-only |
| online rules (read-only, replay A/B, two consoles) | C | N/A | no online path changed |
| Eden: claim/release, one per host, MemAvailable, never profile a / :61 | M | open | st-01 queued on the Yoga |
| Eden: hard-linked profile copy, copies vs links | M | open | own profile copy for st-NN (gv.py prep pattern) |
| df >= 40 GB before builds and Eden; one target dir per source copy; products deleted with a manifest at the end | M | partial | df 173 GB (11:3xZ); host target `trainingrooms-stack`, Ally target `c1161-targets/TrainingRoomsForUltimate-stack-ally`; cleanup at the end |
| waits gated on a log line / screenshot | M | open | st-01 |
| failed precondition aborted and retried under the next label | M | open | st-01 |
| shortest ruleset | M | N/A for Training rows (Training mode has no match end) | — |
| profile restored, console.log scanned (panic/abort/crash/Unmapped), run.json + memory.peak kept | M | open | st-01 |
| every local command under memory/time guard | M | pass | `systemd-run --user --scope -p MemoryMax=6G /usr/bin/timeout …` (host.ps1, analysis runs) |
| outputs per label | M | open | `eden/runs/st-NN/` |
| README / GIF | M | N/A for this stream | no user-visible change (the lead owns README) |
| dated evidence doc | M | open | `eden/EDEN-STACK.md` after st-01 |
| numbers from tested scripts | C | applies | `tools/stack_depth.py` + `tools/test_stack_depth.py` (static numbers); the st-01 high-water table from the logs by a script (to write, with tests) |
| no operator quotes, no claims beyond tests | M | pass | this file |
| handoff current | M | pass | this file; the lead's TRAININGROOMS-HANDOFF gets F7 text from me |
| release/card | C | not authorized | no tag, release, push or card write |
| model/effort gate and watcher | M | pass | claude-opus-5-5 / high from transcript agent-a2594031d8d5c0534; watcher pid 1204646 (`/tmp/stack-watcher-a2594031`) |

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

## State

| Step | Result |
| --- | --- |
| host test `thread_stacks` | pass (124 incl. new) |
| carry verify pkg6 / pkg4 | 262 / 42 files verified against m5q |
| Switch diag + release builds | pending (Surface claim) |
| Eden st-NN | queued on the Yoga |

## Next

1. Surface free → claim, `systemd-run … pwsh -File /tmp/stack-a2594031/tools/sw-build.ps1 -Diag`, then release build.
2. `python3 -I tools/stack_depth.py <diag .so> table`; refresh `static_bound` from it.
3. Eden st-01: ≥ 8 cold boots, save/load, recording, menu open/close last; read `[ThreadStack]` lines; fill `measured`.
