# lr-08 — one complete native cycle; second-cycle gate lost rotated lines

**13 PASS / 1 FAILED / 8 unrun. RC NOT READY.** 2026-11-03.

## Execution

Prepared commit219889c, shared dry-run22rows/57gates/20recordings/0errors.
Four hook boundary and five log-contract tests passed. ProductRC8
`cf4bf44c104ecc96a78d382b4c0276bf33838f18` / diagnostic NRO
`3ca7f0c5c9e5552be4725050949b1ba96266cf4f5875bef660f94ba9b323e9e4`
unchanged. Executed Eden AppImage
`d483c082147eb6a5838ca90b8c0988baec8f8e7bc662234215069a4746215570`.
Playbook403f517a7927533290fdaaa582c519b77c12277c; exact tool/hook/library/
manifest identities inprovenance.json. Every receipt binds rows-executed.json
and the shared immutable manifest snapshot; portable paths separately frozen.

Release-gated caller waited for Pkg4's actual21:12:05.325855Z Yoga release,
tokenc8cc9ced9b1b464f88ba4ecc957899ee. lr08 claimed21:12:31.406342Z;
game loaded21:13:24.138178Z; RELEASED21:16:45.859914Z,
token702ae1eac3174e23867493497db8ea33. Turn stayed under20minutes;
guest900s/runner480s bounds and caller Downfinally were active.

## Observed positives

Bootstrap and first warm Falcon/Gunner build passed. The native Quit hook
verified pause, recorded focus/confirm, Player clean-bank finalization and
empty ledger, entry-linked CPU native retirement, managed-source listener/
reservation/claim cleanup, and native CSS. CPU return took Sonic39→40→Lucas39;
new-generation Steve/Lucas reentry passed. Reviewed actual images match
Steve/Lucas bodies and HUD at0%, and the second scene's Falcon/Gunner bodies
and HUD at0%. This qualifies one complete scene-end/reentry/continuation,
not three cycles, pending-build interruption or CPU detached-bank teardown.

## Failure diagnosis — retained, not promoted to PASS

`cycle2-warm-groups` passed correlated Player/CPU ready predicates and HUD
checks, then failed`lr08-cpu-synchronous`. The plugin did emit the expected
second-cycle evidence: retained`practice-diagnostics.previous.log`#527 has
`bank_build=false`, and#541 has matching epoch3/sequence2 CPU Gunner
ready/released. The file rotates after#559; current`practice-diagnostics.log`
begins#560. Both files are retained verbatim and hashed inprovenance.json.

Shared runner`last_match`(:563–578 in executed runner.py) opens only the
current source, resetting its offset on inode/size change. It does not search
the row's retired generation. Thus the later gate cannot find the already
emitted pair after rotation. This is an evidence-reader failure, not proof
that CPU took a different lifecycle. The FAILED receipt remains unchanged;
all eight later rows remain unrun. A future fix must preserve row freshness
across rotation and reject stale/duplicate/conflicting records; weakening the
predicate or reading the entire previous boot would not establish that.

## Cleanup and harvest

Shared Downexit0/released, full21410-entry restore AND separate verification.
Independent source/profile snapshot equality,252immutablelinks/20436private
files/0violations/noownedgame-displayPID/originalidentity/claim. Expected
terminalgame.stop deadline retained; cleanup_failures[] under currentEDEN
clean-forced-Down policy. Fullconsole826643B/5842lines/0Unmapped/0fatal-pattern
matches; transferSHA`8045251714392dae0a20c6f30f0dd3467a8745359fc49b359bcf9554fd541782`
verified. Memory peak6735220736B<8GiB.

Shared harvest adds0segments+2gates:`lr08-cpu-synchronous` (firstcycle only)
and`lr06-catalog-cycle1`. Native-quit hook internal gates are committed
project evidence, not claimed newly harvested library entries. The later
rotation failure remains applicable to repeated-cycle execution.
Raw tools/console/control/lifecycle/snapshots/audit adapters retained at
`/home/intpa/smash-training-lobby/artifacts/trainingrooms-lead-astra4/rc8/lr08-shared/`.

## Handoff

Explicit Yoga handoff to tracing queued as
`2eacb18d-f0cb-430e-a7cf-b69bc40b52f4` after actualrelease/harvest/audit.
Tracing's installation turn≤10min; no further Lead boot queued. Future RC
execution requires actualinstalledreadback/release and refreshed Eden pins;
this run remains bound to the prior AppImage. Pkg4's next turn returnsSurface
after verified tracingrelease21:05:57Z. No product source/build/release/card/
DBI/USB actions by Lead. Remaining RC clauses and numeric-adapter provenance
stay open; no Phase2/3 composition.

PRIMARY21:15Z HOLD consumed: no lr09 until tracing's actualrelease plus
installed AppImageSHA readback. Future configs must use the exact resolved
immutable release path and SHA, never a `current` alias. The explicit handoff
line is posted at the top ofLEAD-INBOX; hold notification also sent toPkg4.

Watcher audit correction: stopping the lr08 systemd watcher left two earlier
same-session timeout/PowerShell watcher processes1261528/1261529. Their exact
session/owner/argv identities were verified before termination; independent
final process scan found none. Earlier sole-watcher assertions were incomplete.
Failure/stop/final-absence receipts are retained in the raw evidence directory;
this procedural failure is not erased by final cleanup.
