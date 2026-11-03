# Hitbox constructor outcome observation

Lead assignment 2026-11-03 03:41:48Z, accepted box03:43:39..04:28:39. This is a diagnostic observer for the unresolved r10 ly02 two failed sphere requests. It does not fix or tolerate missing spheres. Only the lead's next integrated candidate may compile/boot this slice. Native callback, stack, coexistence runtime and actual outcome evidence remain pending.

## Factual contract and evidence boundary

Owned game13.0.5 ELF SHA256 `0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2`. No upstream behavioral source or fork disassembly was used. Text windows come from address+0x1000 in this ELF. `slim/hitbox-observer/native-audit.json` under `/home/intpa/smash-training-lobby/artifacts/trainingrooms-nro-archive` records all six full64 windows, complete flow hashes, and the direct-branch scan.

The manager calls constructor356cec0 at3563760, so native entry LR is3563764. Its frame is0xf0. The constructor calls wrappera51a0 at356d2dc; wrappera51b0 tail-branches to helpera4f60, whose frame is0x40 and whose return LR is356d2e0. The helper restores x29/x30 at a50d4 before the observed epilogue. No stack memory or native object is dereferenced by our callbacks.

| Inline offset | Incoming observation | Overwritten16 bytes and continuation |
| --- | --- | --- |
| 356cec0 | x0 opaque object, SP before frame, x30 native manager return | sub SP0xf0; three register-pair stores; continue356ced0 |
| a50d8 | w0 helper result, x30 restored caller, SP=constructor frame-0x40 | two pair loads, post-index pair load restoring SP, native RET through restored x30 |
| 356d970 | x19 opaque constructor object, SP=constructor frame | load q0, compare, cset, store byte; continue356d980 |
| 356cfb8 | w0 constructor result, SP=constructor frame | four pair loads; continue356cfc8 restoring SP then RET |

All four sites are8-byte aligned. The displaced words contain no PC-relative load/address/branch; a50d8 explicitly includes RET. Before accepting a native build, review the actual inline trampoline's context restoration and relocated RET behavior, all four callback symbols and their measured stack budgets. A generic function-entry assumption is insufficient. Native x19 is already restored by356cfb8, so exit correlation deliberately uses only SP and w0. Callbacks do not mutate InlineCtx. This does not imply that the framework preserves NZCV or scratch x16/x17; it does not.

The full constructor range356cec0..356db70 (3248bytes, FNV4d169258c892d75b) and helper/wrapper rangea4f60..a51b4 (596bytes, FNV5cac3e4803f744c9) are checked strictly before installing anything, as are the six full64 seam windows. Both strict checks and the existing seam gate must pass. Rows compile away without diag. No partial admission, foreign chaining or shortened guard is accepted.

The audit scans direct B/BL/B.cond/CBZ/TBZ targets across text and found none in any displaced16-byte interior. It does not claim a general indirect-branch proof. The exact six sites were also scanned against the approved saved16-plugin NRO inventory using playbook `tools/build/scan-offsets.py`; every plugin reported no reference, raw `coexistence.log` retained. This is static coexistence evidence only. Constructor exit full64 guard overlaps the read-only counter guard at356cfe4, but its actual16-byte write ends356cfc8, before that counter window. No counter is patched. The other observer writes are disjoint.

## Correlation and outcomes

`Request` brackets only `GameEffects::sphere_spawn`'s actual `natives::effect_req_follow` call, before RGB/rate operations. It acquires one fixed atomic capture using the native thread identity. Another thread cannot contribute or clear it; nested requests on the owning thread mark it ambiguous. Multiple constructor entries, duplicate helper/return samples, bad entry frame/caller/object or contradictory helper/final samples also remain ambiguous. All native calls still run normally. There is no lock, allocation, filesystem operation or native argument/result change in the observer.

The captured entry SP determines the exact constructor frame; helper return additionally requires the exact native caller and frame+0x40 relation. Final phase additionally matches the opaque constructor object. Missing constructor/return remains Unobserved/NotReturned. A completed false constructor is classified using the actual guarded branches:

- Helper false:356d2e8 tests result bit0 and branches to356db48.
- Helper true without final phase:356d2f0 null handle or356d300 epoch mismatch branches to356db30.
- Final phase reached then constructor false:356da58/356db24..356db2c exhaust the final payload alternatives before356db30.
- No helper and no final phase: BeforeHelper, with no finer attribution.
- Constructor true with outer handle0: ConstructorSucceeded, leaving the later failure unassigned.

Final phase may legitimately follow the helper-skipping356d2b4 path. Final phase plus a recorded false helper is contradictory and cannot be presented as a final payload rejection. These outcomes identify branch families; none proves a specific allocator/resource root.

Only the first eight failed, captured outer calls are logged. Successful calls clear capture without a record. The capture is released before formatting, and Drop clears an unfinished owning request. Existing async diagnostic readiness gates acquisition and output, preventing the startup filesystem fallback. Native callbacks enter HookScope and existing headroom observation. The post-call diagnostic executes inside the existing Hitbox caller scope. Actual zero-allocation and stack evidence must come from the next coherent native artifact/run.

## Tests, integration and remaining acceptance

Five original tests in `tests/hitbox_observation.rs` import the production pure helper directly. They cover branch outcomes including helper-skip, foreign identities/frames/callers/objects, nested/multiple constructors, duplicate/contradictory samples, incomplete/invalid requests and clean reuse. Standalone rustc1.98.1 edition2021 `-Dwarnings`:5PASS,0failed,0ignored; exact source unchanged before/after. Surface CLAIMED/RELEASED04:00Z, no remaining queue/claim/compiler/Eden.

Receipt `slim/hitbox-observer-host/receipt.json`; executed binary SHA256 `6076a9d87392b5f3b4ae501bd9778992669c38d257dca45028f3e896b89911d6`; raw test log SHA256 `680dee98df6e0350fc1f6f58f531e54c4b98a8a955cfe3522b78d18b3da7635a`. This compile does not qualify native adapter wiring or Skyline ABI.

Own stream has no Hitbox feature implementation. Lead03:45:52 authorized a small integration patch against immutable maina2cd571 rather than importing its feature tree. `hitbox-observer-integration.patch` inserts only init under the existing admitted install closure and the exact native-call bracket. Adjacent JSON binds base, path, before/after hashes and patch hash. Lead must review/apply this patch along with the new runtime/pure modules, six seam rows, runtime module declaration and three carry transforms. Full253-file carry verification passes; ledger regenerated. No foreign worktree was edited and no native build/boot was started.

Remaining: independent source/ABI review; integrated targeted tests, clippy/all-target and no-default build; actual native four-callback/stack qualification; failed-call outcome capture with zero total Unmapped and every hook allocation delta0; the resulting root fix and complete Hitbox acceptance; final actual-artifact compliance and the other stream acceptance gates. This document does not mark the observer or slim stream READY.

## Integrated static ABI qualification — 2026-11-03

Lead inspected the actual Yoga Skyline subsdk9, SHA256
9959ecb13eab6db81a07cee04a789b1905fd605409fc84efeb7ec43da7e29280.
All three decoded NSO segment hashes matched their embedded hashes. The compiled
inline handler reserves 0x300 bytes, restores SP/LR before the trampoline, and
does not preserve NZCV or scratch x16/x17. All 16 displaced words fail all four
compiled relocation classifier masks and take literal-copy fallback, including
the helper RET. The phase CMP and CSET remain adjacent. The near form copies one
word then resumes before the original CMP/CSET; the far form copies four words.

Independent F6 review bf5346e qualifies incoming flags/scratch liveness at all
four sites and all discovered direct callers, including constructor caller
0x453210 and shared-helper caller 0x356ffa4. New flags are defined before use;
scratch values are defined before their loop uses. This is bounded static proof,
not a claim about unknown indirect callers or runtime success. Evidence:
tmp-lead-astra4/skyline-inline-copy-words.json and tmp-f6-worker/hitbox-abi/receipt.json
under the project parent; the addressed review is copied alongside this contract.

Integrated source includes dfd693a pure condition cleanup after the earlier
standalone test receipt. New compiled callback stack accounting must include the
0x300-byte framework frame. All four live callback/headroom outcomes and tests
remain pending the coherent candidate; no missing Hitbox sphere is accepted.
