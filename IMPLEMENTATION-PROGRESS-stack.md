# stream/stack — thread stacks (2026-11-02)

Lead `0caa72b8`; worker Claude Opus 5.5 high (agent a2594031). Playbook `ae87443`. Branch `stream/stack` from
`work/implementation` 27d84f4, worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-stack`, host target
`cargo-targets/trainingrooms-stack`, Switch target `cargo-targets/trainingrooms-stack-sw`, Eden labels `st-NN`.

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
