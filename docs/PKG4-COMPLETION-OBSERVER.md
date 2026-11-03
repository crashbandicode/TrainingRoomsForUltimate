# Native capture completion observations

Lead approved this bounded diagnostic slice at 04:15:40Z on 2026-11-03.
It is uncompiled and not runtime-qualified. Do not treat it as a capture fix.
Targeted tests and native stack/allocation qualification belong to the next shared build.

## Reproduced failure and contract
p4c-01 r10 captured two3600-frame tracks and saved the associated native replay,
but its same-boot audit reported truncated=true. The approved36000-frame bound
does not explain that result. Existing release/diag lines omit the completion cause.
Never clear truncation, increase capacity, substitute a shorter test, or claim a
native simulation clock from the opaque serializer counter to make this row pass.

Observe the existing callback branches without changing their decisions:
normal/partial stop, FRAM publication filters, serializer losing its owned recording,
copy of a still-active record, segment continuation, and per-entry state/sample/input
rejection. All values were already owned/read at those callbacks. The invalid-native
FRAM case uses only try_lock on the existing capture lock to post a diagnostic; it reads
no additional native fields and adds no blocking acquisition. Contention skips this
one rejection observation, so a missing line is not proof that publication ran or
that native() accepted. The normal logger is the existing asynchronous fixed
ring through runtime/macros/mod.rs println!, with no callback filesystem or heap.
The helper is non-inlined so its formatting frame can be measured independently.

The new value-only CompletionTrace contains only a record identity and128-bit
seen mask. Reset it at pool reset and every segment-zero start, including Off.
Begin it only for an armed record. Each event/entry pair emits at most once until
the next record. Invalid entries refuse without consuming another bit. Generic
bound is72 notes/record; these call sites use at most29. No per-frame success log.
Non-diag builds compile out all observer state, calls and helper module.

## Diagnostic interpretation
Each ReplayCaptureEnd line includes event, diagnostic record generation/index,
current active key, optional entry, flags, observed counter, owned clock and
pending-entry bitmap. An absent active key is not a new record.
- stop flags bit0 means explicit partial=true. A nonzero pending mask also makes
  the existing stop partial. flags0/pending0 preserves earlier worker seals.
- publish flags bits0/1/2/3 mean native() accepted, native recording, state token
  matches, owner/index matches. Zero indicates native() refused; no additional
  pointer read is attempted. Only the existing filters decide finalization.
- serializer flags bits0/1/2 mean native() accepted, recording, owner/index match.
  This is only emitted in the existing failure branch.
- active_copy identifies the existing forced-partial prefix fallback.
- segment identifies the existing nonzero-segment partial transition.
- state/input identify their existing missing-value seal branch.
- sample flags1 means stable=true, therefore the evaluated live_scene() was false.
  flags0 means the existing short-circuit stable check failed; scene was not read.
  Do not infer which body/key/clock comparison failed from that single flag.

This does not diagnose every worker-side seal (allocation failure, continuity,
limit) directly. Missing callback reasons plus partial output is evidence to inspect
that existing worker path, not permission to mark a capture complete. Earlier
drops retain their existing MacroHandoff diagnostics. Native save association and
all capture/toggle/event semantics remain unchanged.

## Exact integration and tests
PKG4-COMPLETION-OBSERVER.patch applies four source/test files to exact main r10
d9e1cd35ac087d26e1c63122b43763724d7e69a2. Adjacent JSON binds before/after and patch
SHA256. Own live_replay baseline differs from r10 ONLY in the already-existing
Deluxe host_registered facade. The patch retains main's deluxe_row call.
The earlier narrative saying full source byte equality was inaccurate; capture
logic was equal. The first exact-hash editing assertion refused before mutation;
the corrected operation verified that one-line difference.

Append the16 runtime and one module transforms from this commit to the existing
main carry rows; never replace main's manifest/ledger. Own full carry derivation
passes42 files against stage m5q. New helper/test are original, not carried.
Four original tests in tests/capture_completion.rs cover idle/invalid admission,
repeated callbacks and distinct fighters/reasons, new/off record attribution,
and the exact fixed bound. They exercise the helper used by the real callbacks.
Requested next candidate: cargo test --test capture_completion, relevant capture
library regressions, diag native build, exact callback/formatting stack checks,
and one batched native save acceptance run with zero total Unmapped/all allocations.
No separate build or boot was run for this slice. Final full gates remain lead-owned.
