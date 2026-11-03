# Current asset recipe reproduction

2026-11-03. Scoped host reproduction passes for the builder and item inputs of frozen ab13a0f. Final native pixels, archive reversal and whole-RC acceptance remain open.

The lead executed the retained UI19 builder binary in two fresh processes, one Training and one Waiting, without Cargo or a compiler. Its SHA-256 is a14622ca015460da41efddf0b6941b510aca54f99567d83626e07459f3916375. All 51 original frozen inputs were rehashed before and after. Builder source/manifests, pane and recipe models, and all approved item files match r15 exactly. Private stock inputs were verified against the original UI19 receipt before execution; no live profile was read or changed.

The run used a 512 MiB scope, 20% CPU quota and 60-second total timeout. Actual interval: 07:18:41.541001–07:18:48.725486 UTC. Both processes exited zero. Each generated recipe and complete target archive matches the retained UI19 target; each recipe also matches the frozen r15 asset. This is new execution evidence, not a repeat compilation or inferred reproducibility.

| Output | Stock SHA-256 | Recipe SHA-256 | Target SHA-256 |
| --- | --- | --- | --- |
| training | 90a8dc81751efa9d4521f836be9ccf8a134cb24af9fb5b195211e58203f9317c | 06f2c6b44d931d44ca7d2e797f21a8a6d35688664f872d6ee29f31679e2feac0 | 2362939b8a6978cfab4025a9c7fec0a484d9029c2429355c120bc1d49419f1fa |
| waiting | 7dcc45360bd84dadffcfe8b47771f5d6b40e44468c6a54cbd3dbb6b4142eea64 | 014eb1685143d4e66f58681c1d865019ae581f2eeedcaa3cdd30c9079f69ffcb | 02dc5796b6c5210774940add80b684c012ecc2221504f761b5c75561f5de5eb7 |

The approved runtime-portrait and runtime-stage rules replace the old baked roster/stage generator outputs. The current replacement pipeline uses the retained 38 approved item images and clean-room layout recipe builder; it must not run the old fork builder against an upstream archive. The core07.5 progress row therefore maps to this current pipeline, not to an unexecuted old reproduce.py command.

The raw reports deliberately retain untouched_roundtrip_identical=false: Training serialization is451476 versus451604 original bytes; Waiting1283972 versus1284116. Exact target/recipe reproduction does not establish byte-identical reverse serialization of the original SARC. Existing stock member/pane/material/texture preservation evidence is separate, and core07.3 remains open for its complete reversal contract. No failure or warning is suppressed by this reproduction result. The earlier builder dependency future-incompatibility warning also remains disclosed in UI18-UI-BOUNDARIES.md; this binary execution makes no new compiler-warning claim.

Private evidence: ../artifacts/trainingrooms-lead-astra4/main-r15/asset-reproduction/receipt.json, both raw stdout/stderr files and generated private targets. Receipt SHA-256: ce6c44a5d80cf27e692bf92b4d5a47d001b7b626126d1db11635f92d07823789.

## Exact layout reversal, 07:38 UTC

A separate inverse starts from each generated target layout, removes only the added TrMod/tr_ pane subtrees, appended item materials, waiting text material and item texture references, then recomputes the section offsets/counts. It does not copy original layout bytes into its output. Both inverses reproduce the stock BFLYT byte for byte: Training43,204B/SHA25642fc96ea79d148ca4bde0a3f687f1cc4f1056886f4e5054ce152ca95b4308046; Waiting16,136B/6f4ee9b4e2709ab2b894ff691403582f7075f8d93965821e0d25145aa821ce1d.

Neither stock layout has a pane-name collision with the owned prefixes. Mutating an original pane alpha in each target causes the inverse comparison to fail. The173 Training and182 Waiting members other than the modified layout and appended BNTX remain byte-identical to stock. Private scripts, original independent parser and reversed layouts are retained at main-r15/layout-reversal; receipt SHA25695fa7c25314628bcc1d9f4cb38fab3b6debc84bcce589e687c2c0c3429cac684.

This closes the exact layout inverse subcase. Whole BNTX-file reversal and original SARC container padding are not proved, so core07.3 remains open. The pinned SARC writer explicitly guarantees member preservation rather than original serialization; its untouched-roundtrip differences are not a claim of changed stock payloads. No compiler, native build, live profile change or runtime acceptance is associated with this audit.
