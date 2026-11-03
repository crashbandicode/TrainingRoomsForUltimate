# RC7 R3-1 CPU-role lifetime — 2026-11-03 (focused evidence, NOT RC READY)

## Source and root

- Exact base: `719255d578c952160e6ed73c5cefb5ad28b2c968`.
- Product changes: `39656a5269f8530c73e48ab55662690371b121d6`, then shared producer/carry correction `185e4c8e47225caf606e69aec9f871b57018a2d4`.
- Tested head: **`4e2e73c52e6179820ceab0a233178524bb0a9edf`** (test engine-module import repair only after185e4c8). Branch `stream/pkg6-rc7-r3`.
- Phase2 WIP `c3717f1` is separate and absent. No fork modification, new game hook/native address, dependency, game RNG, guard relaxation, size-counter/budget change or new lint allowance.

The source-established exposure was an accessor-only role table surviving native
entry destruction/scene departure, with P1 frame advantage querying the retained
CPU accessor before any replacement CPU frame. No hardware crash/desync or
executed baseline failure is claimed.

The existing RoleTable now owns scene/entry generations and frame receipts,
captured before native original execution and checked afterward. Authoritative
pre-destruction revokes both paired owner keys before nested body-bank cleanup.
Every accessor-role consumer requires current practice and a current registration.
Cross-fighter measurement resolves the current manager/active primary first,
requires the matching registration, then queries natives outside the CPU lock.

The shared `practice_policy::depart_session(retire_lease: impl FnOnce(),
clear_consumers: impl FnOnce())` protocol always retires the operation lease and
then clears consumers; the lease lock is released before cleanup. Actual scene
departure and P1 pre-destruction use it without a lease condition. Numeric owner
and entry-address stop receipt remain distinct. CPU/state/recorder/UI own their
cleanup; UI's exact-protocol cross-consumer regression is separately reviewed.

## Frozen identity

Evidence root: `../rc7-r3-validation/shared2/` relative to the worktree.

| Field | Value |
|---|---|
| Files / dirt at freeze | 1532 / zero |
| Content SHA256 | `ae5b4fb45d886723aa688519632b35e29e18c8438af762783954ff802128f1cc` |
| Archive SHA256 | `2baf0b93d041f6e835615f030ac0ee143a66b514c96423e6ca0e0f1b3e8252ff` |
| Manifest SHA256 | `68afc590ca85385b00bf947c016866012e33b0779fc2513382fe14dd7985b642` |

Own integrity audit verifies every archive member against its manifest and pinned
Git blob, completed log/runner/capture hashes, before/after source inventory and
actually executed preserved ELF hashes. All six test/list ELF pairs match.
`integrity-audit.json` covers BOTH completed default and diag pairs, including
all13namedprobes in each actual test/list. No whole-suite count is computed.

## Actual focused jobs

Default test command:

```text
cargo test --locked --test cpu_role_lifetime --test cpu_core --test capabilities --no-fail-fast -- --nocapture
```

Matching listing replaces the trailing arguments with `-- --list --format terse`.
Diag uses the same selection plus `--features diag`. Each is a separate normal
Yoga queue/ledger/resource admission, owned source/target, incremental0, jobs2,
actual6GiB/swap0/Tasks256 cgroup,540s child/600s outer timeout. No Surface compiler,
Eden/profile/new boot/card/USB/DBI/push/tag/release action.

| Output label | UTC | Actual outcome |
|---|---|---|
| shared1/test at185e4c8 | 14:31:57–14:32:09 | Cargo101 compile error E0433: host probe incorrectly imported `model::lobby`; no executed ELF or behavioral result |
| shared2/test | 14:33:58–14:34:45 | Cargo101 solely inherited ownedELF input missing; all13newR3probes alreadyPASS; all3ELFs preserved |
| shared2/test2 | 14:37:21–14:37:49 | Cargo0, same source/assertions/ELFs; existing visible missingELF SKIP contract; all13newR3probesPASS |
| shared2/list | 14:42:04–14:42:05 | Cargo0; all13namedprobeslisted; all3ELFs identical to test2 |
| shared2/diag | 14:44:57–14:45:36 | Cargo0; all13newR3probesPASS; same literal summaries and one visible missingELF SKIP as default |
| shared2/diag-list | 14:47:15 | Cargo0; all13namedprobeslisted; all3ELFs identical to diag |
| shared2/clippy | 14:48:37–14:48:46 | Cargo0, selected3harnesses, `-D warnings`, no warning/error diagnostic; no new allowance |

All completed claims released after confirming no compiler remained. Initial
queue-head refusals hold no claim and are not behavioral failures. First two
failed jobs/sources/runners remain preserved; no rerun overwrote them.

The absent approved game ELF is **one inherited fact test visibly SKIPPED**, not
native qualification. `SKIP_OWNED_ELF_FACT=1` is its pre-existing explicit contract;
no assertion/guard/source change was used to turn the failed job green. The
failed and positive shared2 commands executed identical test binaries.

Literal parent summaries in the successful default test:

| Harness | Passed | Failed | Ignored | Qualification |
|---|---:|---:|---:|---|
| capabilities | 216 | 0 | 4 | Actual capability/callgraph probes, no scanner exclusions changed |
| cpu_core | 230 | 0 | 0 | Existing role assertions preserved and adapted to receipts |
| cpu_role_lifetime | 1284 | 0 | 2 | Includes13newnamedR3probes and one visible missingELF early-return |

The diag test has identical literal summaries; no warnings/errors in either
successful test command. One inherited fact is visibly SKIPPED in EACH closure.

These harnesses duplicate inline model tests. **No unique/full-suite sum is
claimed.** Existing `evidence.py cargo-count` actually refuses this focused job:
`requires a successful all-target Cargo job bound to this exact log` (exit1,
`cargo-count-refusal.txt`). The helper contract is not weakened or substituted.

## Artifact hashes (default pair)

| Artifact | SHA256 |
|---|---|
| test2 log | `3c057c08a0cc9247bbb8e537391180a6f72f9df610cb7b8734bac86773b40879` |
| list log | `27853ae8487c2a02b0e5892b9f1e04c5195a80accf84b9d35a701e6a2072a6ee` |
| capabilities ELF | `1100113ecbdf3e8bad51d2a20e0c358ab8e780600e97e7f44793a2ebdff50722` |
| cpu_core ELF | `6d47a297cb4476cff882f9f8fbcd21c717ca69bd1a2744a2e5688bb897ac7055` |
| cpu_role_lifetime ELF | `363954027ac2cb80ad5962795caaf6e27b39594ff43823be06ac5f518fa842bf` |
| diag test log | `b8716e2ace908cb7e48176274d8b8191872aaa8cd1fd6a38133f46fc1a05caf5` |
| diag list log | `e24aca037b8971540db42052c3be6d74a809ee192ab4da692a0c8bf09b850fed` |
| diag capabilities ELF | `b8b75b563c54e328f9fef3dff14d673693bedd645ac9ca031ba3eff1d0057236` |
| diag cpu_core ELF | `dadef7e44f0e466af78b3b7bebaed252125567d26be6ccf0c80e3bc30c959c4d` |
| diag cpu_role_lifetime ELF | `69c363ee00f7480e46cb31ddb8a0f6ce3a1f964f99a942a1d3a8087dd8bc62c0` |

Test2 peak229261312B; listing217899008B; original missinginput job1895915520B.
Diag peak1742503936B; log`b8716e2ace908cb7e48176274d8b8191872aaa8cd1fd6a38133f46fc1a05caf5`.
Diag listing peak223121408B.
Focusedclippy command: `cargo clippy --locked --test cpu_role_lifetime --test cpu_core --test capabilities -- -D warnings`.
Clippylog`9d6d101cddec837466963b5c3ce0032a037f1ef4e79ac67f39ede02145dbb234`,
peak1091911680B; completedsource-before/afterMATCH. Ownintegrity-audit alsoverifies
thisjob/log/runner/capture/command andabsenceofwarning/errordiagnostics.
All actual6GiB/swap0/Tasks256, OOM0, source-before/afterMATCH. Each output contains
job.json/log,source verification,resources,preservedELFs and exactrunner/capture/
coordinator. test2runner matches bound SHA3fe5a1c3; list runnerb0e46a48.

## Static/review evidence and open gates

- Full existing `carry_pkg6.py verify` and `--dest-only`: **247** transforms/hashes pass. Three changed carried destinations have append-only exact transforms; original stage hashes unchanged; generated ledger consistent. Independent pkg4 repeats pass.
- Existing normalized size gate at4e2e73c: **135097/136069, +972** headroom. Same rustfmt1.9.0, counter, budget and exclusions. Isolated contribution only, not composedRC7 size.
- Raw source compliance185e4c8: exit0/zero mandatory machine failures,851guards64B;1392 static model test annotations are not execution. All30manualdispositions explicit in sibling validation/COMPLIANCE.md.
- Updatedplaybook03e2321 read/pulled: new guest-tracepoint M row added (31manualdispositions at final). No worker nativefactcapture/Eden/extra trace boot or new diagnosticprint. HOSTdiagtests are featureclosure validation, not nativeinstrumentation. No menu-openquirk investigation.
- Independent pkg4 structurally reviewed185e4c8sharedroot/carry, and default test/log/archive/allGitblobs/actualELFs. UI exactf2a6127sharedprotocol/InputAdapter test-design source reviewed; intended composition includes this authoritative native producer. Cross-consumer executed UI evidence and final contribution verdict remain independently owned/pending. Allfourpkg6focusedtest/listreceipts are now available for review.
- Bothownedtargets nowretiredwithexacthashedmanifests: shared1=301files/206602928B; shared2=466files/449877207B. Allsourcearchives/logs/receipts/actuallyexecutedELFs/exactrunners retained; extra cacheELFlinks explicitlyarenotexecutionevidence. Noownjob/claim/queue/target/Eden/profileheld; noforeign/activetargettouched.
- Thisdocument and finalprogress updates are documentation-only AFTER frozen4e2e73c; product/testsources andcarrytransforms remain byte-identical to the testedrevision. No new behavioral fix is hidden in the handoff.

No Switch release/diag build, compiled native closure/stack/headroom/resident
qualification, positive zero-allocation runtime, native teardown/reentry/paired
body gameplay, coexistence, replay/online, new media or whole coherent RC7 gate
acceptance follows from these host probes. Those are lead-owned/open. No RC READY.
