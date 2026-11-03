# lr-05 — checked return CPU and fresh Training reentry

**16/16 rows PASS; stopped, restored, separately verified and released.
RC NOT READY.** This is scoped runtime evidence, not full Phase1 acceptance.

## Observed behavior

- Shared Up loaded Yoga at **2026-11-03T20:19:38.638576Z**, meeting PRIMARY20:15's
  20:30 deadline. All sixteen rows finished without manual input or row retry.
  Row01 used two bounded, predeclared intro-cancel recoveries; both failed
  intermediate main-menu gate checks remain in the full output/actions.
- Delay2 pause/resume08/09: native images reviewed, `queued_at_begin=2`,
  `owned_polls=126 owned_non_neutral=0`, `resume_checked delay=2 leaked=0`.
  The screenshots also show transient Load State/Input Record helper overlays;
  the native pause menu closes and Steve/Lucas remain visible.
- Native quit10 and13: original-fighter retirement predicates passed and
  returned to CSS. These are not warm GroupSwap or interrupted teardown tests.
- Return hooks11 and14 both observed fresh CPU `39→40` (Sonic), verified the
  held-Sonic image, stepped `40→39` (Lucas), and placed it. Reviewed images
  show Steve/Lucas and **Ready to Fight** after both returns. Fresh tile edges
  and hook results are retained in receipts/actions/tile-final.log.
- Reentry12 passed fresh original Steve/Lucas observations, catalog
  generations `[3,3]`/opening4, both owner qualifications, and native unpaused
  Training. `reentry-diagnostics.log` sequences398/399/405/406/407 correlate
  the new entry. This short run did not itself exercise log rotation; it used
  the rotation-fixed shared runner.

The Roy26→27 branch was **not exercised live in lr05**. Five offline hook
contract tests pass, including Roy correction, crossed-edge rejection,
missing fresh-token rejection and L3 release after input failure. lr04's
separate Roy correction evidence remains historical; it is not a live lr05
hook pass. This run qualifies the two observed Sonic return convergences.

## Cleanup and integrity

Shared Down ran in the caller's `finally`; row/down exits both0. Graceful
`game.stop` **FAILED** with a terminal deadline; forced cleanup succeeded.
Unit result is timeout/SIGKILL, not OOM. Peak6740557824B was below the
8589934592B limit. Actual release: **20:22:47.472444Z**.

Both shared restore/separate verify and an independent audit cover21410
entries. Source and restored profile equal the pristine snapshot. The audit
found252 immutable links,20436 private mutable files,0 topology violations,
0 owned open claims,0 owned game/display PIDs and0 surviving original process
identities. Complete pristine/post-run profiles remain on Yoga.

Full console822233B/5814lines:0 Unmapped and0 panic/abort/crash review matches.
Transferred raw bytes verify SHA256
`c94dc64edba557e32fd9a53bf84c9f7c2d3a74b4b699726e5f9c46449f2aa3c0`.
This bounded scan does not establish every resource/Developer acceptance clause.

Accepted product remains RC8`cf4bf44c104ecc96a78d382b4c0276bf33838f18`;
diagnostic NRO`3ca7f0c5c9e5552be4725050949b1ba96266cf4f5875bef660f94ba9b323e9e4`.
No product source changes. Playbook read/pin`d8f0d9f9451df75874435c706ffddca9573921b2`;
exact runner/lifecycle/library/hook hashes are in `provenance.json`.
All16 receipts bind the byte-identical retained `rows-executed.json`, and
staged/executed/archive hook bytes match. `rows.json` freezes library values
with portable paths; its different hash is explicit. Shared engines are
retained externally as evidence, not copied into this product repository.

## Reuse

`steps.py:return_cpu_to_lucas` is a project step hook using shared context
operations. It requires the row's ready CSS state and its named segments/gates.
It accepts only fresh observed edges26→27 or39→40, verifies Roy/Sonic, moves
toLucas39 and places the CPU. It does not establish arbitrary starting positions.

Shared structural dry-run:16rows/48gates/20recordings/0missing/0errors READY.
Shared `harvest.py --label lr-05` added **0 segments/0 gates**: all top-level
dependencies were already present. The harvester does not traverse project
hook calls, so the return hook and its internal dependencies are retained here,
not claimed as newly registered shared-library entries. lr04's harvest remains
6segments/11gates.

```text
python3 -B -I ~/ssbu-mod-playbook/tools/eden-runner/runner.py dry-run eden/lr05-shared/rows.json --out /tmp/opencode/lr05-dry-run
python3 -B -I eden/lr05-shared/test_steps.py -v
```

Raw full output, exact tools, lifecycle/control logs, inventories, original
console and compliance dispositions:
`/home/intpa/smash-training-lobby/artifacts/trainingrooms-lead-astra4/rc8/lr05-shared/`.
Remote preserved state: `memento-claw-yoga-ubuntu:/home/intpa/tr-lr05-yoga-state/`.
Original lr03/lr04 failures remain closed and unchanged.

## Remaining acceptance

Interrupted teardown×3, pending-build ordering, forms/pairs/hats, pointed
saved-Mii identities, pilot, Developer/resource clauses, completeR3 including
catch/shield positives, independent pause producer, frame-advantage lifecycle
and active attacks, full UI exact-return, coexistence and remaining online
acceptance stay open. The fresh solo return/reentry evidence narrows that gap;
it does not close whole-RC contracts or authorize Phase2/3 composition.
No new boot, agent, push, tag, release, card, DBI or USB action is part of this closure.
