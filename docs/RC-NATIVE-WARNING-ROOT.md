# RC native warning-root return — 2026-11-03

Owned branch `scratch/slim-rc5-native-root`, exact base
`5e3fe5ef07ed2b31666410e8ec0e637fd5d976e8`. Lead 12:57/12:58Z assignment:
remove only orphan APIs; no Cargo, claim, freeze or Eden. Playbook pulled
ff-only/up-to-date at `2dee6cc1fcabb379ac67db4bfd6a3aa453dd3303`; README,
all seven docs and tools/PROVENANCE fully reread before changing code.

## Contract and evidence

Actual RC5 release build.log63–96 emits five dead-code warnings despite Cargo0.
The original receipt's ANSI detection bug stays recorded: RC5 native FAIL.
Searching every tracked Rust file at the exact base, without cfg filtering,
finds SIX occurrences: five definitions and `fighter_ready -> cycle_ready`.
There are no test or feature consumers. These Rust functions in private runtime
modules are not external exports. Removed only these five APIs and their own
docs/cfg branches; no reachable state transition, native call or fallback changes.

The source-only audit `/home/intpa/smash-training-lobby/tmp-slim/sol-tmp/audit-rc5-warning-removal.py`
rejects the original six occurrences and passes current source absence. It also
confirms that EVERY other byte of all four runtime files is unchanged. Thus
`original_kind`, progress, live registry, native transaction paths and state
remain intact. This is not execution of their native behavior.

Only four existing carry transforms/destination hashes changed; all 247 stage
source pins and M5q stage remain unchanged. Existing full carry verification
passes **247 + 41** files, not destination-only. Existing ledger regenerated.
Existing size gate passes **134866 / 136069**, **1203** headroom, unchanged
counting/budget/exclusions. Lead integration omits the proposed string-presence
test because it mirrors these deletions without exercising behavior; existing
`tests/pkg6_features.rs` tests remain unchanged. Lead owns the complete
all/list/clippy/no-default/native gates for combined RC6; these are still pending.

## Independent pane review

`de92c4a`: pinned nnsdk0.6.2 ui2d.rs62–88 says Pane.name is
`[libc::c_char;25]`; libc-nnsdk0.5.0 lib.rs18 defines c_char=u8. Root Cargo
pins that nnsdk. Redundant cast removal preserves unsigned comparisons. Shim
width25 and NUL precondition match the binding. Fifteen Safety contracts cover
live traversal links/lifetime/UI thread, writable subtype fields, UTF16 buffer,
setter ABI/executability and synchronous stack copy. No blocking source finding;
not native ABI qualification. Two new declared pane regressions UNRUN by Slim.

## Preliminary compiled review: FAILED RC5 only

Actual preserved release ELF `d6e93e82d74329f84088927f85dcf98013adf48d8fef75559ebb713e92c8935e`
and diag `dc736f08a96c36d95ad4ec9619325041e147b3f880ce5007aa365df256a400aa`:
logs/NROs/ELFs independently match original job receipts and before/after
1528-file source reports. No binary/source substitution. Existing stack_depth:

- Table: release11 present/6 feature-off, diag16 present/1 feature-off, zero row
  failures across retained17-row policy. No missing-feature runtime coverage inferred.
- `budget --all`:135/139 matching hook/callback roots, zero over budget. Direct
  service233504/405721; UI fiber8992/9024 under16384. Not all indirect callbacks.
- `hooks --deepest20`: **exit1**, existing8192-byte own-frame inventory flags50
  frames in each ELF, max111824. Preserved as inventory failure, not silently
  promoted to budget PASS; separate measured-entry budget is unchanged.
- Boot: release run_inner32256 (wrapper not separately emitted), diagrun32336
  below39321. SDK/indirect frames are not completely modeled.
- Body::copied frame368/depth1120; Queue::post416/1248; recorder
  notices::post_phase384/1632 both builds. Queue upsert/tick/mail drain inline
  in hud_draw::draw, frame6928/6736, depth9072/8736. Source paths and compiled
  callers reviewed, not representation-size-only proof.
- Known indirect Heartbeat TICK->report_late:32+2896=2928 release,
  2768+2304=5072 diag, below retained5696/measured11184. Registration boot.rs60
  and call diag_log.rs339 retained. Generic scanner omits this edge.
- StateWriter40960/measured17664 and Probe208896/external143376/measured151920
  retained; no historical maximum, bound, allocation or margin changes.

Existing analyzer adapter and addressed paths/callers:
`tmp-slim/sol-tmp/review-rc5-native.py`, `rc5-release-stack-review.txt`,
`rc5-diag-stack-review.txt`. Indirect UI callback coverage and final native
measurements remain coordinator-owned. Final RC6 ELFs need fresh review.

## Checklist and limitations

| Requirement | Tag | Result | Evidence |
| --- | --- | --- | --- |
| cdylib/profile/toolchain/layout/version/64-byte guards/allocator/docs/Forgejo | M | pass | compliance raw report, source unchanged outside deletions |
| Root cause, no suppression/workaround | M | pass | exact deletion + six-reference source audit; no new allow/gate/visibility change |
| Final Cargo/native zero-warning gates | M | not authorized | no worker Cargo; final Rust/new regressions UNRUN; RC5 native FAIL |
| Hook/card scan/placement changes | M | N/A: no site/call/guard changed | four runtime files differ only by dead API deletion |
| Sharing/load-order tests | C | N/A: no hook change | no site added/modified |
| Online read-only/replay/two-console | C | not authorized | applies to plugin; final shared batch OPEN, not waived |
| Eden claim/host/profile/waits/retry/ruleset/health/cleanup/output | M | N/A: no launch | no worker claim, Eden or profile mutation |
| Targets/cache admission/retirement | M | N/A: no build | no target/cache created/deleted;40GB rule retained |
| Command guards | M | fail | first read-only date and playbook-pull calls had tool timeouts but lacked inner memory guard; subsequent commands guarded |
| Current model/watcher | M | pass | exact Sol/high checked13:02; sole owner-bound ext5 PID369122; stop before yield |
| Evidence/handoff/accurate claims | M | pass | this record; source versus compiled versus runtime scopes separated |
| Existing GIF | M | N/A: no visible behavior change | existing development GIF unchanged; no new footage claim |
| Numbers from existing tools | C | pass | size_gate/carry/stack_depth; source audit only, no invented runtime count |
| Release/tag/rebuild/card installation | C | not authorized | none performed |
| No card/DBI/USB/mount/unmount/push/tag/release | M | pass | no such operation |
| Delegated workers read playbook | M | N/A: no subagents | work stayed in existing assigned stream |

Compliance initial run exit1 was precommit dirty-tree row only; final report
must be rerun after commit. No shipped NRO exists for this source change yet.
Failures retained: grouped manifest context patches refused before write;
split exact patches applied and full carry passed. Two guessed receipt.json
reads failed; actual filename job.json located via directory inventory.
Owned watcher runtime-extension failed `Cannot set property RuntimeMaxUSec, or unknown property.`;
stopped ext4 before starting sole ext5. No duplicate watcher or hidden source
waiver. **No RC READY, native zero-warning, runtime or final compliance PASS.**
