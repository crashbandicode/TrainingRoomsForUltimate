# Minimal Hitbox pool observation proposal

2026-11-03; lead 07:50 disposition, within the existing 08:10 UTC box. Initial design-only state; lead 07:58:54 subsequently authorized the bounded implementation and one targeted host gate. Native build/runtime remain lead-composed and pending.

The existing qualified points can expose the actual manager capacity and available count **after** one owned helper call. They cannot supply a qualified before count for successful calls. Therefore this proposal can close the live-total gap, but cannot by itself establish per-request allocation attribution. It must not be presented as a complete occupancy census or a root fix.

## Existing points and provenance

| Existing hook | What is available | Limitation |
| --- | --- | --- |
| Constructor entry 356cec0 | x0 outer effect object, x3 file entry, SP, LR3563764 | No proved emitter-manager register. The native manager load occurs later at 356d2bc from global5334ad8. Reading that global here would introduce a separate early-lifetime assumption. |
| Demand rejection a5020 | Qualified x19 manager, w8 requested, w9 available before the rejection; x1 object+1d0 | Only rejected calls execute this point; it cannot provide successful-call before counts. |
| Helper return a50d8 | x0 result, x19 manager on qualified normal paths; before displaced restore of caller x19 | Early null/disabled paths retain constructor-object x19. Reject these before any dereference. |
| Final phase 356d970 | Outer object/thread/frame | Already after helper allocation; not a before observation. |
| Constructor return 356cfb8 | Result/frame; x19 already restored at356cfb0 | No newly qualified manager. Do not reuse a pointer from a previous request or dereference caller x19 as the manager. |

Exact caller 356d2d8 sets w6=0. Wrapper a51a4 computes w7=w6&1, a51b0 tail-enters a4f60. Thus this caller takes the nonlocking success path a50f4..a519c; it cannot take a5148, where the other success path repurposes x19 as an emitter-set pointer. This distinction is essential: a50d8 x19 is not universally a manager for all native callers.

At a4f7c/a4f84, null/disabled resource returns branch to a50d0 **before** a4f90 assigns x19=x0. These paths retain the outer constructor object from356cf44. A qualified request whose x19 equals its captured object is unavailable for manager reads. For this exact caller, normal success and later failures execute a4f90 and preserve manager x19 through the return site. Existing full constructor/helper/wrapper fingerprints cover that path.

## Proposed read permit

Before either load, a pure model predicate must require all of:

- An active plugin-owned synchronous Request, matching current thread, asynchronous writer ready, and a reserved observation ordinal.
- Exactly one constructor, nonzero captured object, expected constructor LR, checked frame calculation, no nesting/conflict/ambiguity.
- Helper LR356d2e0 and helper SP+0x40 equal the recorded constructor frame; no earlier helper return and no completed constructor return.
- Result exactly0 or1, x19 nonzero and distinct from the captured outer object, four-byte alignment, and checked x19+0x84 address arithmetic.
- Installation gates for the exact existing hooks and relied native flow. Add strict relied evidence for the +0x70 capacity field (a3ec4/a3ec8) and its +0x80 initialization (a404c/a4070); the already fingerprinted helper reads +0x80 at a4fc4. These are additional field-contract guards, not new hooks.

Alignment/nonzero/overflow checks alone are **not** validity proof. The proof is the exact qualified native path that has a live manager in callee-saved x19 and has not destroyed that manager before this hook. Early-resource and foreign-call paths are excluded before dereferencing.

The runtime would perform only two aligned 32-bit observations, at manager+0x70 and manager+0x80. No resource-table walk, slot-array census, heap scan, pointer retention across callbacks, query-memory syscall, new lock or native write. The manager layout extends beyond +0x98 in the same qualified native helper; the initialization proof supplies the field meanings. Retain raw signed/unsigned bit patterns. Negative or available>total values mean inconsistent/unavailable classification, not fabricated zero or a reason to clamp/modify native state.

These two reads are separately timed observations, **not an atomic snapshot**. Plugin Capture ownership does not exclude another native manager consumer. No concurrency, stability or sole-writer claim is inferred from the observer's own thread guard.

## Correlation, budget and output

Keep the original eight-failure evidence and its classification unchanged. Proposed supplemental window: **first 80 plugin-owned requests per process**, enough to include the original74 attempted spheres and observed two retries if the same route is reproduced. This is an observation budget, not a rendering limit. Record the reserved ordinal even for unavailable observations; do not silently replace misses with later successful samples. If unrelated earlier owned requests consume the window, report an incomplete reproduction.

For each eligible request, capture at most one helper sample: request ordinal, manager identity token, total/available raw words, stage=AfterHelper, eligibility/status and original handle/outcome. Fixed scalar atomics in the existing Capture; no per-request Vec, String, queue allocation or retained native pointer. The failure demand sample remains a distinct stage=DemandRejectBeforeReturn, never a successful-call before value.

Emit at most one short supplementary record per reserved request from Request::finish, **after releasing capture ownership**, through the existing asynchronous diagnostic sink. No formatting or logging from the inline hook. Preserve missing/ambiguous/helper-not-reached/budget-exhausted categories; logger loss remains visible and prevents a complete accounting claim. A compact numeric schema should be bounded to256 bytes including normal prefix/reserve before implementation acceptance. Eight original failure pairs are separate and retain their existing bounds.

Consecutive after counts may be reported as an **inter-request observed difference** with both ordinals and manager identities. They must not be relabelled as allocation by the later request: cleanup, other native effects and frames can occur between them. Demand-before-return versus after-return is meaningful only for the already rejected call and cannot prove successful-request consumption.

## Validation required if the lead authorizes implementation

Original policy regressions must cover: early null/disabled path whose x19 is the outer object (no read), success on exact w7=0 path, other-caller/w7=1 path rejection, foreign thread, wrong LR/SP/object, nested/duplicate constructor/helper, zero/misaligned/overflow pointer, missing return, all raw count anomalies, first80 window/exhaustion and original eight-failure budget. Use an injected read spy to prove denied cases execute zero memory reads; do not test by dereferencing fake addresses.

No new compiled stack number is claimed. Adding scalar capture and formatting can change callback and formatter frames even without allocation. The next coherent native build must recheck the modified helper-return callback, existing constructor callbacks, Request::finish formatter path, strict guard/collision/Skyline qualification and all thread tables. A bounded actual first-pass run must show every new sample, zero allocations for every hook, measured headroom, complete logging and full-console health, preserving original failures and cleanup.

## Exact unresolved boundary

There is no qualified before-success manager observation at the five existing points. Reading a cached manager across requests, using the global earlier than native does, subtracting the resource's nominal11 from a later count, or treating a previous after sample as the next before would manufacture attribution.

To obtain an actual per-success delta, a separately qualified pre-helper boundary or a demonstrated manager lifetime/serialization contract at the existing constructor entry is required. Neither is authorized or established here. The narrow after-only design is reviewable for live total/available evidence; whether that evidence alone is sufficient for the next root decision is for the lead to dispose. No global observer, new hook or product behavior change is proposed implicitly.

## Implemented source and targeted gate

The four-file implementation adds PoolBudget with exactly80 reserved owned requests and fixed scalar Capture fields. The existing a50d8 callback invokes a checked injected-reader path before the existing helper observation; only the qualified path reads two volatile u32 fields. Exact caller provenance is enforced by the guarded native flow and LR, not an unqualified reading of live w7 at return. Two Relied full64 rows cover a3ec4 and a404c; all nine guards match the owned13.0.5 ELF, no other model rows overlap the additions, and the five installed hook offsets are unchanged.

Output is [HitboxPool] n=... stage=AfterHelper status=... mgr=... total=... free=... handle=... outcome=.... Status0 means MissingHelper,1 Denied,2 AfterHelper,3 Inconsistent,4 Ambiguous. Stage describes the intended sampling point even when status says no sample exists; zero fields for missing/denied are not pool measurements. An absent ordinal or sink-loss report is incomplete capture, never zero allocation. handle=None identifies Request drop without normal completion. No request beyond ordinal80 substitutes for a missing sample. The original eight failure pairs remain independent. Manager is an identity integer only after capture, never a retained pointer for later dereference.

Standalone rustc1.98.1 edition2021 -Dwarnings compiled the frozen model and original regression binary; 19/19 tests PASS, zero ignored, source hashes unchanged before/after. Six added regressions cover permitted reads, early-object exclusion, foreign/context/arithmetic denial, ambiguity/nesting/duplicates, raw anomalies, all80 missing reservations, independent eight-failure limit and maximum-field record plus53-byte sink-prefix reserve <=256B. Existing13 tests remain passing. Runtime callback code is frozen with the evidence but is NOT compiled by this standalone host gate.

First attempt failed compilation with E0428 duplicate test helper sample; raw compile and frozen input remain in slim/hitbox-pool-host. Renamed only the new helper pool_sample; fresh normal-queue admission and evidence are in slim/hitbox-pool-host-v2. Both claims were released and queue entries removed. No native build or runtime was started by this slice.

Pending before runtime: composed native build, modified callback/formatter compiled stack and allocation review, live strict qualification, actual first-pass ordinal/loss audit and measured thread headroom. This is after-only instrumentation, not a capacity change, occupancy attribution or Hitbox fix. READY remains unset.
