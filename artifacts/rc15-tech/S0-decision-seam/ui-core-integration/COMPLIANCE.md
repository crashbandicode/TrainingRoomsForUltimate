# Scoped compliance disposition — 2026-11-16

Candidate source is frozen at `fc266f47`. This report does not establish plugin acceptance.

Required scanner attempted, not completed: `systemd-run --user --scope -p MemoryMax=1G timeout 90 nice -n 19 ionice -c 3 /usr/bin/python3 -I /home/intpa/ssbu-mod-playbook/tools/check/compliance.py /home/intpa/tr-fixes-20261113/rc15-tech-seam --base 8b43bdf2 --trace artifacts/rc15-tech/S0-decision-seam/ui-core-integration/DESIGN.md --out artifacts/rc15-tech/S0-decision-seam/ui-core-integration/COMPLIANCE.raw.md`.

Tool output, verbatim: `Running as unit: run-p1201114-i101789448.scope; invocation ID: 1e39d06d9cba4516b8cfe0b48febfb94`. The containing command returned exit code 1 with no further output after its bounded run. `COMPLIANCE.raw.md` does not exist. No diagnostic text was returned; do not attribute a scanner defect. No retry because the primary's canonical Surface gates started meanwhile and the primary froze this snapshot.

| Requirement | Tag | Result | Evidence |
| --- | --- | --- | --- |
| Playbook/bootstrap/model gate | M | pass | Full playbook read at f0dbc0b, exact-thread Sol medium verification recorded in HANDOFF.md |
| Thread watcher | M | pass | PID 1165452, bound to owner PID 3869311, reaped before worker completion; no changed-pair marker |
| Delegation | C | N/A: bounded worker, further delegation expressly forbidden | Primary briefing |
| Root cause and requirement trace | M | pass | DESIGN.md; absent boot registration fixed rather than bypassed; no UI restrictions introduced |
| Source scope and clean diff | M | pass | fc266f47, git diff --check; SOURCE-CHECK.json contains source hashes and eight true static predicates |
| Native capability poison isolation | M | not authorized | Rust regressions added; execution belongs to primary coordinated gates |
| Test, clippy, no-default build, release, diag | M | not authorized | No local compiler started. Primary Surface unit rc15-tech-seam-gates-fc266f47 now owns canonical test/clippy/no-default evidence |
| Actual CROSS_NEEDS regeneration | M | not authorized | Existing exact graph comparison retained; compiled scanner pending primary gates |
| Hooks do not block | M | fail | Unchanged UI VIEW.lock / IDS.set and surface VERIFIED.get_or_init paths listed in HANDOFF.md; owner notified, no audit exemptions |
| Complete machine compliance report | M | fail | Bounded scanner did not return a report; exact received output above |
| Every local command under memory/time guard | M | fail | Initial read-only bootstrap and git commands used required pwsh but lacked systemd resource wrapper; scanner and source proof were guarded |
| Release profile, toolchain, version and allocator | M | not authorized | Inherited unchanged; machine scanner incomplete; no new build or NRO |
| Site windows and card scan | M | not authorized | Reviewed surface rows inherited from UI commit; no new offset/ABI claims or card access |
| Hook placement | M | N/A: no new hooks | Adds one call to existing CPU post-native dispatcher; surface rows are called/relied only |
| Sharing stand-ins and load orders | C | N/A: no new hooks | No hook/seam ownership changes |
| Heap, NRO identity, diagnostic stripping and release logging | M | not authorized | No built artifact in worker scope |
| Online read-only review and detector pair | C | N/A: worker performs no online runtime | Final integration acceptance remains primary-owned |
| Eden claims, profile copy, instances, memory, resolution, mute, runner, fixtures, waits, shortest match, cleanup, pruning and console scan | M | N/A: no Eden operation in worker scope | Main worktree, profiles and ledger untouched |
| ARCropolis and visual asset verification | C | N/A: no mod or asset runtime claim | No screenshots or visual evidence |
| Per-copy targets and finished build cleanup | M | N/A: no build products created | No cargo/rustc invoked |
| Evidence/GIFs and numerical log analysis | M | N/A: additive source-only handoff | SOURCE-CHECK.json explicitly limited to static source; no gameplay evidence claimed |
| Handoff current | M | pass | HANDOFF.md plus this final disposition; source frozen fc266f47 |
| Git transport and commit identity | M | pass | Local commit only, requested crashbandicode identity and UTF-8 git commit -F file |
| Push, tag, release, card install | C | not authorized | None performed; no mount, unmount, DBI or card operations |

Final acceptance is blocked by compiled closure validation and UI hook locks. The source-only commit is ready for primary review; no build, Eden, native grading or exact-once guard-press claim is made.
