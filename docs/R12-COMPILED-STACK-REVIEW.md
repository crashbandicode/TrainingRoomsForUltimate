# r12 compiled callback and stack review

Completed 2026-11-03T05:08:51.080995+00:00; bounded review began05:01Z. Frozen6047101,
ELF b2c0a5c32a3bc3e11a7378bc54dcfeec826ece535c9d00fbafd07a1d4482e0d7.
Build job exit0/source_verified_after=true. Independently verified all1307 frozen
source hashes, archive SHA and ELF SHA. Evidence directory: /home/intpa/smash-training-lobby/artifacts/trainingrooms-nro-archive/slim/r12-stack-review.
No compiler, live controller, profile or product edits in this review.

## Blocking table results

| Worker | Current owned depth | Recorded bound | Allocation / policy required |
| --- | ---: | ---: | ---: |
| Heartbeat | 5136 | 5056 | 28672 / 28672 |
| INI writer | 3216 | 3184 | 28672 / 28672 |
| Menu | 34624 | 32432 | 57344 / 61440 |

These are addressed compiled bounds, not observed runtime overflow. Heartbeat
and INI writer need current static metadata; their historical measured9072/9344
still dominates the unchanged quarter-margin+16384-reserve policy. No allocation
increase is needed for those two. Their fresh runtime measurements remain owed.
Menu needs a root correction before its next compiled gate; lead/UI own it.

Comparison with exact r10 ELFa779278f57b2fef0b94a31b83992abaff015a9e832919a756bdf786c43023b96:
Heartbeat5136 versus5056; new shim1dda0 frame2768 versus2656. INI writer3216
versus2864 (the old table3184 was already conservative), new shim21a64 frame848
versus640. Both new deepest paths continue via threads::diag::start e4cac304,
diagnostic8bd80688, Locked::with9558192, report_failure8c1c4208, File::open224918128,
format_inner1ce46480 and retained panic/runtime tail. comparison.json preserves
every address/frame and old/new path, including changed deepest branches. Thus
80/32 are growth above table bounds, NOT both deltas from the r10 binary.
Heartbeat source adds two HookIds and larger snapshot arrays; writer source
itself is unchanged, but its inlined/transitive publication code changed. The
compiled prologues, rather than an inferred source-size formula, bind these bounds.

Menu34624 versus r1031168. Its worker frame grew6624 to10080; the remainder of
the selected browser/capture/deserialize path has identical per-function frames.
The r12 worker directly zeros4096 bytes at sp+fb0 (ab6d8..ab6e4), then passes that
same buffer and4096 size to File::read atab844..ab854, after sync_panels returns.
Inlining keeps this buffer in the persistent frame beneath unrelated deep menu
paths. UI dcb70b8 isolates the whole Worker::step turn; its next ELF must prove
both the reduced menu path and separate reload path. No padding/algorithm change
is justified by this report.

Probe: owned3264 <=table3440, qualified external143376, historical measured151920,
allocation208896 (204KiB), unchanged policy required208896: static policy PASS.
This does not replace a fresh completed-path measurement.

## Changed callbacks

| Callback | Owned frame / direct depth | Known additional prefix |
| --- | ---: | --- |
| Hitbox constructor | 48 / 192 | Skyline768; hook before native prologue |
| Hitbox helper_return | 48 / 192 | Skyline768 + native ctor240 + helper64 |
| Hitbox final_phase | 48 / 192 | Skyline768 + native ctor240 |
| Hitbox constructor_return | 48 / 192 | Skyline768 + native ctor240 |
| Q15 constructor_context | 304 / 2368 | Skyline768; entry before native prologue |
| count_readiness | 5728 / 7088 | Skyline768 + original native272 |
| debug_set_float | 48 / 912 | replacement hook, no inline768 frame |
| Capture::completion_note | 256 / 2320 | called beneath existing capture callbacks |
| hud_draw::draw | 6880 / 8992 | registered indirect HUD callback |

Known local totals are Hitbox960/1264/1200/1200 and count8128. They are not whole
native-thread ancestry or SDK bounds. Actual native prefixes come from ownedELF
0789 and retained F6/native-flow evidence; complete UI start-to-hook instruction
scans confirm only160/48 native stack reservation for pass/register paths.

The generic direct scanner misses the HUD edge. Actual tick a1cac has frame2928;
BLR a1dc8 loads registered DRAWS at400510 (source registration + symbol verified).
UIpass32 +tick2928 +HUD8992 +Skyline768 +native160 =12880, under the existing
16384 fiber budget by3504. Registration substitutes native48 ->12768. The HUD's
native text setter goes through pane::set_text a1a98 frame704 and an indirect
call; original setter/SDK internals and recursive pane walks remain outside the
direct scanner. This explicit known-path accounting is NOT whole-callgraph or
native allocation acceptance. New live headroom/zeroallocation and full notices
must still pass; old UI measurements cannot qualify this changed path.

Developer compiled callback preserves x0/s0/w1 through x19/s8/w20 and calls its
original exactly once at1b2d74 BEFORE installed/row/scene/kind tests. Subsequent
indirect calls are fighter_kind1b2e0c and backshield1b2e3c after nestedHookScope.
The original native callee and SDK paths are not included in direct912. Existing
lead actual-Skyline replacement/trampoline proof remains a separate prerequisite.

completion_note callers include stop(frame256/depth2576), copy_record1824/4144,
publish_record_frames80/2656, serialize_frame320/2496, start_recording400/2848,
and cpu::on_frame9536/79488. The latter's deepest path is another existing CPU
path, not a claim the formatter itself uses79488. Generic owned-hook budgets
have no failure, but indirect/native costs and live allocation remain open.

Raw reports: receipt.json, table.txt, comparison.json, detail.json,
native-prefix.json and addressed assembly. Scanner limits remain explicit:
direct calls only, recursive edges cut, SDK/import/indirect native bodies omitted.
An incidental Python bytecode cache created by the first analyzer import was
removed; subsequent imports disable bytecode. All1307 product/source bytes remain
unchanged. Initial boundary wrapper asserted on generator truthiness before any
layout claim/build; it was corrected before that separate successful layout job.

No READY: r12 fails Menu policy and two metadata bounds. Lead owns correction;
next source/ELF must be independently remeasured before runtime admission.
