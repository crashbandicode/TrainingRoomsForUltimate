# RC7 UI correction contribution — 2026-11-03

Returned for composition; not whole-RC/native acceptance. Pkg4 independently
reviewed the source and all four final focused host receipts with no blocking
contribution finding. Composition must include pkg6's authoritative scene/P1
producer `185e4c8` exactly once; this UI branch deliberately does not edit its
`practice.rs` or `training/mod.rs` wiring.

## Exact source

- Base: `719255d578c952160e6ed73c5cefb5ad28b2c968`, branch `stream/ui-rc7-r3`.
- Observer/input ownership: `f9860f266502a4ca140e62613718750cef04d5d5`.
- Pre-write renderer receipt/editor installation: `6545f1d38cb42e9ab6cc6bdf23c4fc29ab024571`.
- Final protocol/tests: `f2a6127e31ea3b3783e2d73b07cf7171151d8e68`.
- Pinned policy file AND complete carry entry equal pkg6 `185e4c8`; file SHA256
  `9c69bd57bf9ba8dc9419b724d66ded157f275e96bf4334c6abe4aca055c64fcc`.
- Full 247 carry destinations re-derived successfully; no original stage hash,
  counter, budget, exclusion, dependency, native offset or guard policy changed.
- Normalized implementation: **135092 / 136069**, headroom **977**.

Native pause no longer depends on injected panes/worker/renderer availability.
Boot's authoritative pre-write guard outcomes and actual observer-install receipt
admit/report the renderer without rereading its own hooks. Failed guards still
refuse. The editor's fixed-unavailable RC path performs no editor preflight,
native-default, trace, connected/lifecycle/LDN, keyboard or resume initialization;
retained Training-stage and saved-catalog readiness remain separate.

The six actual callback/InputAdapter probes include no-lease/no-CPU-frame
departure, queued-input and paused variants, held-lease release ordering, reentry
before a new callback, stale epoch rejection and same-address registration. Four
installation probes include stock guard → own hook write → reread refusal negative
→ pre-write receipt positive. Five editor probes execute the real install
dispatcher and bind its native call grouping. Native layout and unrelated mode
classification use disclosed host shims, not native ABI qualification.

## Final focused execution

Evidence root: `../tmp-ui-screens/rc7-r3/green3/` relative to the worktree.
Frozen **1534 files**, all before/after checks match on all four jobs:

- Content: `c4716e3819654bee795e1878528bdb6d831f06c5734d057d619b5e6e354b82b1`.
- Archive: `3a3c7657dd41b1459e27b981f710ee256f800e08e75835c5ce67a7a313738901`.

Each job executes one Cargo command after fresh Yoga queue/full no-expiry ledger,
process, disk ≥40GB and memory ≥10GiB admission; MemoryMax6GiB, swap0, Tasks256,
1800-second timeout, incremental0. Each claim released; no Eden/compiler overlap.

```text
cargo test --locked --test ui_native_pause --test ui_install_boundary --test mii_install_boundary
```

Diag adds `--features diag`; matching lists add `-- --list --format terse`.

| Job | UTC | Actual exit | Memory peak B | Raw log SHA256 |
| --- | --- | --- | --- | --- |
| default test | 14:43:08–16 | 0 | 840396800 | `2681fe2c4a67a17254ffb6b3278ce041aadb0bd50dd0de6266dce446869013b9` |
| diag test | 14:46:59–14:47:02 | 0 | 867782656 | `defa349c74d8c8c19da9c5af8d2ec0ecde4cd15658be11dacec2585861f364ea` |
| default list | 14:48:22 | 0 | 79872000 | `2e2c160d9e55d37dab8d02fba6620d6b2bd4ad240397ff4dcbee0fe12557306f` |
| diag list | 14:49:42 | 0 | 66134016 | `7d82266b34603c5ae54d026a34adba82704873627c124f9dd86e524f5c2f660e` |

Literal test parents EACH closure: `mii_install_boundary` **5 passed**,
`ui_install_boundary` **207 passed**, `ui_native_pause` **212 passed**;
zero failed/ignored/warnings. All 15 named original probes pass and are listed.
Counts include duplicated inline models and three dependency-policy cases;
there is no unique/full-suite aggregate. Standard `evidence.py cargo-count`
refused the focused jobs with exit1 (all-target job required); unchanged helper
and refusal retained. No native-fact PASS inferred.

All 12 execution records and six unique ELFs are preserved in `binaries/` by
their full SHA256, with original paths/identities in `*-binaries.json`.
Each default/diag test/list pair uses identical three ELFs. Raw jobs, complete
source checks, admission/release state, runner/capture/helper source and hashes
remain alongside them; `collection-audit.json` independently rehashes the bundle.

## Preserved failures and limits

`red1` is a harness import compile failure, NOT a behavior reproduction.
`red2` executes frozen RC6's real pane walker/tick/InputAdapter: expected pause
owner16, actual0; 0 passed/1 failed/203 filtered, Cargo101. Original raw source,
log and executed ELF remain. `green1`'s pass does not waive its later source
blockers. Corrected `green2` independently passes four commands at `6545f1d`;
only final `green3` covers the shared-consumer follow-up. Admission refusals are
not behavioral reds. Five finished targets are retired by exact hashed file
inventories after preserving executed ELFs; see each `target-retired.json` and
`target-retirement-manifest.json`, plus `../retirement-audit.json`.

Playbooks `2dee6cc` then `03e2321`; revised tracepoint rule read and added to the
checklist. Final source compliance and all 31 manual dispositions are in
`../tmp-ui-screens/rc7-r3/COMPLIANCE.md`; initial unbounded bookkeeping commands
remain a procedural FAIL. Native/full gates, compiled closure/stacks, zero heap,
runtime, replay/online, and new media remain unverified/lead-owned. No new native
prints/trace boot, menu-open-quirk investigation, profile/helper/frozen-RC6
mutation, push/tag/release/card/DBI/USB or subdelegation occurred. Phase 2 parked.
