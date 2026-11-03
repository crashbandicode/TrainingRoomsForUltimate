# lr-07 — native Quit reached; CPU bank-finalize predicate failed

**10 PASS / 1 FAILED / 11 unrun. RC NOT READY.**

Product RC8 `cf4bf44c104ecc96a78d382b4c0276bf33838f18`, diagnostic NRO
`3ca7f0c5c9e5552be4725050949b1ba96266cf4f5875bef660f94ba9b323e9e4` unchanged.
Playbook `403f517a7927533290fdaaa582c519b77c12277c`; exact executed tools,
library, hook and manifest identities are in `provenance.json`.

## Actual result and diagnosis

Bootstrap and first warm Falcon/Gunner row passed. Native Quit hook passed
stable HUD, fresh pause ownership, native pause, recorded Quit focus/confirm,
and Player clean group finalization. Reviewed Quit-focus image shows the
native menu with Falcon/Gunner. The CPU `lr06-clean-finalize-1` gate timed out;
the row remains FAILED, with no accepted CSS or reentry checkpoint.

`practice-diagnostics.log` establishes why the CPU predicate was inappropriate:

- #287: CPU saved-Gunner staged `bank_build=false`; #301 matching ready/released.
- #380–388: Player attached retirement and detached-group teardown,
  `map_after=0 skipped=0 alias_ok=true`, followed by empty ledger reconciliation.
- #395–400: CPU native attached plan `routes_stale=0`, no Kirby mismatches,
  `skipped=0`; route released; saved-Mii destroyed `cleanup_writes=false`;
  owner1 entry invalidated; active Gunner and empty pending trio at destruction.
- #412–421: original-source actor/entry cleanup, listener removal,
  reservation cleared and claim released.

Source explanation: `src/runtime/roster/native_body_bank.rs:4077–4085` calls
attached retirement before detached retirement at finalize; `retire_groups`
returns without a summary when no matching bank or no groups/prefetch exists
(:3622–3623). Native CPU retirement is not evidence of a CPU detached bank.
lr08 will check this actual native path explicitly while retaining Player
clean-bank checks. No pending-build interruption or pointed Mii identity is
qualified by this diagnosis. All three scene-end/reentry cycles remain open.

## Closure

Game loaded20:52:22.867981Z; actual ledger RELEASED20:55:16.429921Z,
token86bfddce4e694f2b9bba1a534067551c. Even process-start20:52:09 to release
is under20minutes. Caller Down exit0, expected terminal game.stop deadline
retained in `down.log`; forced cleanup succeeded, cleanup_failures empty.
Full restore and separate verification plus independent audit:21410 entries,
source/profile equal,252 immutable links/20436 private files/0violations,
no owned claim/game/display PID or original process identity remains.
Full console823834B/5824lines,0Unmapped/0fatal-pattern matches; transferred
SHA `65b4528a6f51cb8ae77901a6e0c049a3d01df67df4d17bbe00a63ec16e1bb849`
verified. Peak6365687808B below8GiB.

All11 receipts bind byte-exact `rows-executed.json` and shared manifest
snapshot. `rows.json` freezes portable dependency paths separately. Four
offline hook boundary probes passed; they do not replace native acceptance.
Shared harvest adds0segments/0gates: qualified dependencies already exist;
hook-internal calls and the failed row are not promoted by that tool.
Raw tools/console/control/snapshots retained at
`/home/intpa/smash-training-lobby/artifacts/trainingrooms-lead-astra4/rc8/lr07-shared/`.

## Scheduling and remaining scope

Operator20:54Z order: Yoga alternates Lead→Pkg4→Lead, each Up→RELEASED≤20min;
tracing retains Surface/Ally. Pkg4 receives the next Yoga turn; harvest/release
notification775adf92-ba3d-4309-8947-056901b7248d queued. lr08 stays offline
until pkg4's actual Yoga RELEASED. Surface release returns pkg4 to Surface.
This supersedes the previous tracing Yoga-package reservation.
Remaining13 top-level clauses, extra R3 positives and numeric-adapter
provenance remain open. No product changes, push/tag/release or device action.
