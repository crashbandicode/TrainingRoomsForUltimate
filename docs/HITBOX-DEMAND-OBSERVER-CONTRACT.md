# Positive Hitbox emitter-demand rejection observation

Lead 2026-11-03 06:27 approved this bounded replacement for the unqualified
global `93660` draft. The global error stub is not hooked. This observer
collects evidence for a proven unresolved helper-false result; it does not
fix missing spheres, raise capacity or count retries as first-pass success.

## Exact predicate and correlation

Owned 13.0.5 ELF SHA-256
`0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2`.
The additional inline site is **a5020**, before `mov w8,-1` and the original
output-clearing stores. Both incoming paths prove signed w8 > w9:
descriptor demand at `a4fcc`, or fallback demand zero falling through `a501c`
when available is negative. Negative values are preserved as signed i32;
they must not be described as ordinary unsigned capacities.

| Captured register | Meaning at a5020 |
| --- | --- |
| w8 / w9 | signed requested / available emitter counts |
| x1 | constructor object + `1d0`, output pair address |
| x19 | inner VFX manager |
| x30 | exact constructor helper return `356d2e0` |
| SP | captured constructor frame minus `40` |

Admission requires the owning synchronous Request/native thread, exactly one
validated constructor, nonzero distinct manager, checked object+`1d0` and
checked SP ancestry, exact LR, positive signed comparison and no prior helper
or constructor return. A duplicate, stale, conflicting or malformed marker
leaves the reason ambiguous. Foreign threads and inactive requests cannot
contribute. No pointer is dereferenced and no InlineCtx/native data is changed.

The existing helper-return observer at `a50d8` retains x8/x9/x19 before the
displaced load restores caller x19. Only a matching false helper and false
constructor return, plus the same manager, permits `EmitterDemandRejected`.
The original recorded signed values and output/manager/SP/LR remain in the
snapshot. A missing demand marker is **Unattributed**; in particular, x8=-1
alone cannot distinguish demand rejection from initializer cleanup. Existing
early resource-null/disabled evidence still requires x19 equal to the captured
constructor object and no contradictory demand marker. Other initializer or
EmitterSet failures are not assigned a finer reason.

Successful outer handles clear capture without output or spending the unchanged
eight-failure budget. Records are formatted only after the synchronous request
returns, through the existing asynchronous writer. New HookScope/headroom
coverage is `hitbox_observer_helper_demand`; no new thread or native probe is
created beyond this one site.

## Guard, feature and native contract

The diag ROWS contains seven labels: the existing four installed hooks, this
one demand hook, and two relied-on boundaries. The actual runtime gate and
installation macro use those labels/hooks. The former helper_error hook and
its seam row are removed; release ROWS remains empty. Four owned product/test
files compose directly onto immutable main `127f6f6`; no carry transform is
needed. The existing Request integration and feature/module graph remain wired.

All seven full64 windows must match, along with whole constructor, helper/
adapter, initializer, cleanup and no-op error-stub fingerprints. Keeping the
latter two strict read-only fingerprints preserves the relied-on return
register flow; it does not install a global error hook. The complete helper
flow protects both signed incoming paths and the unchanged native stores.

At aligned `a5020` the far four-word window is:

1. `mov w8,-1`
2. `str xzr,[x1]`
3. `adrp x0, nativebase+42c3000`
4. `add x0,x0,f95`

Continuation `a5030` stores the original -1 epoch and branches to the native
no-op error call, then the false-return epilogue. Near mode relocates one word
and resumes at `a5024`, leaving the native ADRP in place. F6 commit 0f1995e independently qualified actual Skyline 9959ecb1 relocation
and the bounded known-caller liveness. Its external ADRP path emits optional
NOP, LDR X0 literal, B over the absolute target page, then the copied ADD.
Eight arithmetic/alignment/ASLR cases and wrong-copy negative controls passed.
Skyline does NOT preserve incoming NZCV/x16/x17; the inspected false-return
path and known callers do not consume them. This is a scoped native review,
not a blanket ABI or computed-caller proof. All 42 receipt files were rehashed
by Slim unchanged; receipt SHA-256
`4e455c43f125bf16d705588068952d0b4689fda7fd54eb80a900eec3129f7a97`.
The shared F6 helper-demand-abi/REPORT.md records addressed raw evidence.

Whole executable direct-target scan finds only explicit `a4fcc -> a5020`
plus the already reviewed fallthrough; no branch targets the overwritten
16-byte interior. No 64-bit site/interior literals occur in allocated
non-executable ELF segments. Frozen main model offsets do not overlap the
new write. All approved saved16NRO hashes remain unchanged and the unchanged
offset scanner reports no references. These are bounded direct/literal scans,
not universal computed-flow or co-installation proof.

## Evidence and remaining gates

Native audit is in shared
`artifacts/trainingrooms-nro-archive/slim/hitbox-demand-observer/`.
Thirteen original pure tests cover previous branch outcomes, positive signed
counts (including negative available and negative demand), rejection of
unsigned/equal/nonrejecting interpretations, thread/request/object/output/
manager/caller/SP errors and overflow, stale/duplicate/conflicting markers,
missing return registers, clean reuse, success contradiction and the exact
eight-failure budget. Fresh Surface queue/ledger/process/resource admission
after UI's released host attempt yielded one rustc1.98.1 edition2021
`-Dwarnings` compile, exit zero, then **13 passed, zero failed/ignored**.
Own CLAIMED/RELEASED 06:33Z; all six recorded source hashes were unchanged.
Executed binary SHA-256
`eb369a6e3662a3d17d8ad8444364826606915e2e94f40dedfc7b9ee56f7705e6`;
test log `8d95888d6c5d3aaeaab738183461cd94737b631e1d14e94edf3b0c9174fee72b`.
The separate `slim/hitbox-demand-host/receipt.json` binds this result. The
preceding eleven-test receipt belongs to the superseded global-stub draft.

No native build or runtime has been performed for this replacement. Required
before acceptance: lead integration, composed diagnostic build and callback/stack gates,
then an admitted first-pass capture with zero hook allocations and zero total
Unmapped. Missing markers preserve the unresolved branch. READY remains unset.
