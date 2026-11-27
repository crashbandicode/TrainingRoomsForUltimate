# Asset recipe reproduction at ab13a0f

On 2026-11-03 the host checks reproduced the recipes at `ab13a0f`; they did not verify native pixels or the whole release. For the current recipes, see [Building TrainingRooms](BUILD.md#layout-archive-recipe-package-5-host-tool).

- The lead executed the retained UI19 builder binary in two fresh processes, one Training and one Waiting, without Cargo or a compiler.
- Its SHA-256 is a14622ca015460da41efddf0b6941b510aca54f99567d83626e07459f3916375.
- All 51 original frozen inputs were rehashed before and after.
- Builder source/manifests, pane and recipe models, and all approved item files match r15 exactly.
- Private stock inputs were verified against the original UI19 receipt before execution; no live profile was read or changed.

The run used a 512 MiB scope, 20% CPU quota and 60-second total timeout. Actual interval: 07:18:41.541001–07:18:48.725486 UTC. Both processes exited zero. Each generated recipe and complete target archive matches the retained UI19 target; each recipe also matches the frozen r15 asset. This is new execution evidence, not a repeat compilation or inferred reproducibility.

| Output | Stock SHA-256 | Recipe SHA-256 | Target SHA-256 |
| --- | --- | --- | --- |
| training | 90a8dc81751efa9d4521f836be9ccf8a134cb24af9fb5b195211e58203f9317c | 06f2c6b44d931d44ca7d2e797f21a8a6d35688664f872d6ee29f31679e2feac0 | 2362939b8a6978cfab4025a9c7fec0a484d9029c2429355c120bc1d49419f1fa |
| waiting | 7dcc45360bd84dadffcfe8b47771f5d6b40e44468c6a54cbd3dbb6b4142eea64 | 014eb1685143d4e66f58681c1d865019ae581f2eeedcaa3cdd30c9079f69ffcb | 02dc5796b6c5210774940add80b684c012ecc2221504f761b5c75561f5de5eb7 |

At that revision the clean-room builder used 38 item images; portraits and stage thumbnails came from the game at runtime. This was the pipeline verified for core07.5, not the old `reproduce.py` command.

- The raw reports deliberately retain untouched_roundtrip_identical=false: Training serialization is451476 versus451604 original bytes; Waiting1283972 versus1284116.
- Exact target/recipe reproduction does not establish byte-identical reverse serialization of the original SARC.
- Existing stock member/pane/material/texture preservation evidence is separate, and core07.3 was still open at 07:18 UTC for its complete reversal contract.
- No failure or warning is suppressed by this reproduction result.
- The earlier builder dependency future-incompatibility warning also remains disclosed in UI18-UI-BOUNDARIES.md (removed after v0.2.0-rc.1; see that tag); this binary execution makes no new compiler-warning claim.

Private evidence: ../artifacts/trainingrooms-lead-astra4/main-r15/asset-reproduction/receipt.json, both raw stdout/stderr files and generated private targets. Receipt SHA-256: ce6c44a5d80cf27e692bf92b4d5a47d001b7b626126d1db11635f92d07823789.

## Exact layout reversal, 07:38 UTC

| Stock BFLYT reproduced byte for byte | Bytes | Inverse SHA-256 |
| --- | ---: | --- |
| Training | 43,204 | `42fc96ea79d148ca4bde0a3f687f1cc4f1056886f4e5054ce152ca95b4308046` |
| Waiting | 16,136 | `6f4ee9b4e2709ab2b894ff691403582f7075f8d93965821e0d25145aa821ce1d` |

The inverse removed only the added `TrMod`/`tr_` pane subtrees, item materials, waiting text material and item texture
references from each generated layout. It recomputed section offsets and counts without copying original layout bytes.

Neither stock layout has a pane-name collision with the owned prefixes. Mutating an original pane alpha in each target causes the inverse comparison to fail. The173 Training and182 Waiting members other than the modified layout and appended BNTX remain byte-identical to stock. Private scripts, original independent parser and reversed layouts are retained at main-r15/layout-reversal; receipt SHA25695fa7c25314628bcc1d9f4cb38fab3b6debc84bcce589e687c2c0c3429cac684.

- At 07:38, exact layout reversal passed. Whole BNTX-file reversal and original SARC padding were unproved, so core07.3 stayed open.
- The pinned SARC writer explicitly guarantees member preservation rather than original serialization; its untouched-roundtrip differences are not a claim of changed stock payloads. No compiler, native build, live profile change or runtime acceptance is associated with this audit.

## Required preservation contract reconciled, 08:08 UTC

| Preservation check | 08:08 UTC result |
| --- | --- |
| Stock textures | 43 Training and 91 Waiting textures preserved |
| Layouts | Both stock layouts recovered byte for byte |
| Original SARC padding | Not required by the contract |

- SPEC-core-07 section4.2 requires stock texture metadata, pixels, mip offset arrays and4KiB pool alignment to survive.
- Section4.4 specifies byte-for-byte reversal of the original layout.
- It does not require reproducing original SARC padding or an identical whole BNTX container after removing additions.
- The earlier complete-container inverse hold was broader than this contract.
- Its raw serialization differences remain recorded; they are not a failure of stock-member preservation.
- No requirement or runtime failure was waived.

- A fresh independent raw-byte audit verifies all43Training and91Waiting stock textures: metadata spans, every pixel byte and every relative mip offset match.
- Each target adds exactly38owned item textures.
- All173/182untouched SARC members match byte-for-byte; the previously proved inverse restores both stock BFLYT files exactly.
- BNTX relocations, each texture alignment,4KiB pool size and relocation/data-section boundaries pass.
- A mutated stock pixel is rejected by the comparison.
- Both combined BNTX members also start on4KiB archive offsets.
- The small stock bushvt member is not4KiB aligned; an initial extra assertion incorrectly applied the texture-pool requirement to every shader member and was removed.
- Its unchanged bytes remain verified.

Private evidence: main-r15/stock-texture-preservation.json, SHA256
8f558fcd3c5c117cdf48a7eac097d50f47161af145bd007c11f5b985d9f126fb;
verifier tmp-lead-astra4/audit-stock-textures.py. This supplies the missing
host evidence for core07.3. Re-run the scoped checks on the next generated
assets and final candidate; no final RC row is closed by this development
result. No compiler, native build or live profile was used.
