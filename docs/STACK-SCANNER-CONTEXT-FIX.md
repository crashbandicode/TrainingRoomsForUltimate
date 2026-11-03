# Context-correct direct stack path analysis

Completed 2026-11-03T05:26:06.792280+00:00; lead25min assignment received05:18Z. Own scope only
tools/stack_depth.py and original regressions in tools/test_stack_depth.py.
No native code, allocations, thread sizes, graph-edge omissions or compiler jobs.

The old DFS cached a node's suffix even when an ancestor caused a cycle cut.
Reusing that partial suffix under another caller silently omitted a reachable
longer path. Sorting alone cannot repair that context error.

The correction finds strongly connected components in O(V+E), then memoizes
by (node, visited-bitmask-within-that-component). A path leaving a component
cannot return, so its other-component ancestors cannot restrict the suffix.
Repeated nodes still end the path. Every possible simple modeled direct path
is considered; no cycle cap or sum of mutually exclusive branches is substituted.
Sorting by function name/address chooses a stable equal-depth witness only.

DAG state/edge work is linear before path-copy/sorting costs. A cyclic component
of m nodes can have O(m*2**m) states; longest simple path is not polynomial in
general. Returned path tuples also consume storage proportional to their lengths.
R13 has4938 functions and22 cyclic components, largest31 nodes. The complete
bounded rerun took 2.643s under1GiB/180s, including
all17 thread rows,144 hook roots, extras and reversed-input checks. No timeout or
partial-result fallback. This remains a direct-call LOWER-BOUND contract, with
the existing instruction/frame model: recursive repetitions, external/native,
indirect calls, SDK internals and dynamic control feasibility are not proved.

Original regression: R(1)->A(10),B(100); A->C(1000); B->C; C->A. Visiting A first
used to memoize C without A, returning1101 instead of the valid1111 R-B-C-A path.
The old production function fails that exact regression (AssertionError1101!=1111).
The new implementation passes all48 address/order permutations. A second test
checks multiple cycles/shared suffix against independent exhaustive path enumeration.
Own suite7PASS; main-compatible composed suite8PASS (preserves the lead's
external-frame policy test). Exactly2 new tests, not8 new tests.

Exact main-compatible patch docs/stack-scanner-integration.patch:
94d7238229e9c8701312fd52addea9374f0ad0f2ed99fcc8daf581befe3881f3
AdjacentJSON binds before/after SHA for both files against immutable5d92df4.
Initial direct worker diff failed gitapplycheck because this older worker lacks
the lead's external-frame parser/test changes. It made no mutation; the returned
patch composes ONLY algorithm/new regressions, preserving those main changes.
Actual scratch apply and all8 composed tests pass; both source files unchanged
after validation. own-diff.patch and raw logs remain private evidence.

R13 ELF2080ba7a, corrected stable results (ascending/reversed node and edge
orders have identical depth AND witness names):

| Worker | Owned direct bound | Current table | Allocated / policy required |
| --- | ---: | ---: | ---: |
| Heartbeat | 5168 | 5136 | 28672 / 28672 |
| INI writer | 3248 | 3216 | 28672 / 28672 |
| Menu | 31280 | 32432 | 57344 / 57344 |
| Probe | 3296 | 3440 + external143376 | 208896 / 208896 |

Only Heartbeat/INI metadata fail; other present rows pass, PracticeTrace is off.
Do not discard the newly found5168/3248 paths. Existing measured9072/9344 dominate
both allocations; no stack increase. All144 generic hook roots fit their existing
limits, with the scanner's explicit missing indirect/native boundaries.

Changed callbacks remeasured: Hitboxfour192 each; Q15constructor2400;
count7088; developer912; completion_note2352. Isolated reload step6368 plus
worker6736+shim16 yields13120, below wholeMenu31280. Registered HUD indirect
accounting is32+tick2928+draw9024+Skyline768+native160=12912 (register12800), below
fiber16384. These corrected maxima supersede the order-dependent lower figures
in R12/R13 reports. Native prefix proofs/source identities remain unchanged.
No recursive/indirect completeness or live headroom/allocation acceptance claim.

Evidence: /home/intpa/smash-training-lobby/artifacts/trainingrooms-nro-archive/slim/stack-scanner-fix/receipt.json; table.txt; old-regressions.log; new-tests.log;
composed tools; exact patch. Lead owns independent review, integration, measured
metadata and runtime admission. No READY/full-gate/native/runtime PASS yet.

## Lead integration qualification, 2026-11-03

Main-compatible patch independently checked against both before/after hashes and applied. Eight Python tests pass; a fresh disassembly of exact r13 ELF2080ba7a with the corrected scanner passes all16 present thread rows (PracticeTrace feature off) and144 generic callback budgets. Logs and source/tool/NRO identities are bound in parent artifacts/trainingrooms-lead-astra4/main-r13/lead-stack-qualification.json.

Only recorded static bounds change: Heartbeat5136->5168 and IniWriter3216->3248. All allocation bytes, measured values, external bounds and SDK/margin policy stay unchanged. These fields are validation metadata: source-wide static_bound/required usage is within the model; runtime threads use allocation bytes, names and stack painting. The exact existing r13 binary was compiled from5d92df4; it was NOT rebuilt from this tool/metadata documentation commit. This qualifies that binary for bounded development observation against the corrected evidence, not a fresh final-source build or full RC gate. A coherent final release/diag build and all runtime/regression/compliance gates remain mandatory. Known explicit HUD12912/reload13120 and native/SDK/recursive limits above remain in force.