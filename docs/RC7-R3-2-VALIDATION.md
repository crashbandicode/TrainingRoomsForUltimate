# RC7 R3-2 correction contribution — 2026-11-03

Source/test commit **cab66034975f2b8d87de7cc21c648c35cc43d4ee**, exact RC6
base **719255d578c952160e6ed73c5cefb5ad28b2c968**. Owned branch
`stream/slim-rc7-r3`, worktree `tmp-slim/rc7-r3`. Follow-up a3c19d0 changes
only explanatory comments/provenance; later progress commits are documentation.
Playbook2dee6cc freshly pulled/read completely before correction.

## Boundary

R3-2 is the source-established RC6 freeze violation, not a reproduced crash or
disproved native layout. The runtime hurtbox reader/call and exactly three
exclusive guard/generator rows are removed. Ordinary hitboxes, CATCH/rebound,
effect API, original calls, capsule/tint/rate/capacity and owned-handle cleanup
remain. Historical pure HB1 facts/tests remain counted and dormant, not moved
behind a counting exclusion. HB1/isolated sphere work remains Phase3.

The production attack planner/dispatcher is shared with host probes. Tests spy
exact AttackData reads, all eight slots, inactive/null data, point/capsule/grab
requests, poisoned hurtbox scratch, retention and Off/shield/grab/retirement.
The ordinary shield sentinel survives owned cleanup. Capability probes reject
the removed HB1 dependency while retaining the ordinary feature's guards.
Four explicit AttackReader edges keep source-callgraph coverage rather than
excluding the shared planner's native callbacks. Help/README match this boundary.

## Actually executed: focused host checks only

All four jobs use the SAME frozen cab6603 source/unique Yoga target. Fresh
queue/ledger/process/disk/memory admission,6GiB/swap0/Tasks256, inner540s and
outer600s bounds; all Cargo exits0, no OOM, each released with no compiler left.
No Surface compile or Eden/profile action.

Command prefix: `cargo test --locked --test core06_hitbox --test capabilities`.

| Job | Additional arguments | UTC | Raw log SHA-256 |
|---|---|---|---|
| Default host tests | `-- --nocapture` |14:19:13–14:19:35|7d75b1497cd12f6c41244365dbd629d1610a2daf7dfe93813c23524f7205ea42|
| Diagnostic-feature host tests | `--features diag -- --nocapture` |14:26:15–14:26:32|05e8833bd4d613330ffce3476bc9013468a414588945d4a40006cef38c8e2fc1|
| Default matching listing | `-- --list --format terse` |14:27:45|67631de905e7a33c9e9e173bdb390f68ba36a11a55e5e027c3fe7e8d672ce892|
| Diagnostic matching listing | `--features diag -- --list --format terse` |14:28:29|7bcdbe1777c0025578f3bb0037c21e3f7a5782732a82510b8af0f2cf3a299fe8|

Both executed test commands have literal parent summaries: capabilities
**216 passed /4 ignored**, core06_hitbox **223 passed /0 ignored**. All THREE
new `core06_rc_*` probes pass in each feature closure. SIX owned-ELF fact tests
visibly early-return `SKIPPED` in each core06 run; their parent “ok” is NOT a
native-fact pass. Shared inline model tests repeat between harnesses; no unique
whole-suite total is claimed. The standard evidence.py `cargo-count` explicitly
refuses this targeted input (requires successful all-target jobs); exact refusal
is retained, not bypassed or represented as a passing count audit.

`focused1/release/` means default-feature **host debug tests**, NOT a Switch
release NRO or optimized release build. The matching listings run the exact
same ELFs as their respective test command:

| ELF | Default SHA-256 | Diagnostic-feature SHA-256 |
|---|---|---|
| capabilities |96597dc4e1162643e9466fd2c239fad668c8aa28a2297058fc639803f2dabdb8|7e46f60035ea0b6f57941029660f6e9e1cbea351f8c1c4485b278ae63e0f8e78|
| core06_hitbox |f57e543fdc4b00ccf340f44a48b68b071b2c7cd1cc1cbddec673113f0b77dde4|cde4f0835bd8dd8d18666a7747081e17e544281a67d59f1d77eded65f45a335e|

Evidence root: `/home/intpa/smash-training-lobby/tmp-slim/rc7-r3-evidence/focused1/`.
Every before/after verification matches all1529 source files; content SHA
82f4228a8f509e01d2d4b07ca71241c7f9ff0fc0102eafb5fef46c7640317ce6,
archive SHA-256 1cedc4a7ccce7ccc1c33b13ef14c44bc3cbf7fa1eae1479f2f44d3cb691a45bf.
Local independent audit rehashes all four logs/eight preserved ELF records and
checks test/list binary identity (`independent-audit.json`). Initial no-claim
queue refusals remain in driver logs, not counted as behavioral test failures.
Cache retirement inventory deletes only finished owned target310files/261371917B;
source archive, raw logs, executed ELFs and receipts remain. No foreign cleanup.

## Source checks and outstanding acceptance

- Existing size gate: **134811/136069**, headroom1258,55 fewer than RC6;
  formatter/counter/ceiling/exclusions unchanged. No carried destination or
  carry transform changed; composed carry/size gates remain lead-owned.
- Existing generator reproduces all18 retained guard rows byte-for-byte from
  approved owned main SHA0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2.
  This is static byte comparison, not native execution or all layout tests.
- Own compliance actually exits0/zero machine failures;848 guards all64bytes.
  Thirty manual dispositions are in `tmp-slim/rc7-r3-evidence/RC7-R3-2-COMPLIANCE.md`,
  not blanket acceptance. Static1392 annotations are
  not an executed test count.
- Pkg4 independent source review/lead provisional ACK finds no R3-2 source
  blocker; final receipt review and changed-boundary R3 review remain separate.
- No executed RED is claimed for this source-established original R3 finding.
  Frozen RC6, original SH03 failures, S2 RED and UNKNOWN M5q comparisons remain.

**NOT READY / no runtime acceptance.** The composed RC7 still needs its one
full gate wave, actual Switch release/diag closure and stack checks, zero-heap
execution, independent changed-boundary re-review, and confirming runtime.
Original generic8KiB own-frame failures and historical stack maxima are not
waived. No push, tag, release, Switch/card/DBI/USB action or new agent.
