# Input-log text backing candidate

2026-11-03. UI21-02 on r16 showed light frame counts and status text losing
contrast against the Training grid. UI22 adds one opaque PANEL picture behind
each existing frame-count and status text box, 30 pictures total. Icons and
empty row space remain transparent. Existing anchors, text, row pitch, TTL,
fade, and producer semantics are unchanged.

The builder emits each backing immediately before its text. The runtime
resolves it from that text's preceding sibling, checking the parent, sentinel,
reciprocal link and exact name. A mismatch follows the existing display-disable
path. No additional tree search or pointer cache is introduced.

The host-tested source is exact main `b6b2f46` plus the integration patch in
`../tmp-ui-screens/ui22-implementation`, not the standalone worker branch.
Main retains its long-notice pane: old indices 0–587 remain unchanged, new
indices are 588–617. MENU_COUNT is unchanged. No carried source changes or
manifest transforms are needed.

One fresh-target builder compile passed in 21.48 seconds. Actual executions:
five traversal fixtures, fourteen builder/model/private-stock tests, and one
generated-tree traversal test: 20 passed, none failed or ignored. The existing
binrw 0.13.4 future-incompatibility warning remains; this is not a full Cargo
or native gate. Raw logs, executed binaries, 1,338 source hashes, cgroup peak
787,230,720 bytes and zero OOM events are in `builder-evidence/receipt.json`.

The committed verifier's command is:

```text
python3 -I tools/check_ui22_layout.py STOCK_DIR PRIOR_ASSETS GENERATED_DIR
```

`builder-evidence/preservation.json` and `generated-traversal.json` bind its
results and emitted full-tree fixtures to the executed production traversal.
The verifier rejects a planted mutation to an existing text pane's alpha.

| Layout | Old archive | New archive | Panes | Preserved panes |
| --- | ---: | ---: | ---: | ---: |
| Training | 742,292 | 746,388 | 795 | 765 |
| Waiting | 1,570,692 | 1,574,788 | 681 | 651 |

Each archive grows 4,096 bytes. Both boot buffers now require 1,574,788 bytes
each, an 8,192-byte total increase, within the unchanged 9 MiB recipe limit.
All prior pane bytes/parents/order, materials/fonts/texture sections, and 174
Training / 183 waiting non-layout members remain byte-identical. The 30 new
backings match their text's transform, dimensions and pane origin, start hidden,
and retain inherited alpha. No new material, font or texture is defined.

Native contrast/blending/clipping, total picture/material/renderer allocation,
compiled HUD path stack cost, final all-hook zero allocations and zero Unmapped
remain open for the next lead-qualified candidate. The declared 7,440-byte
Picture object floor is not a total resident allocation measurement. Numeric
kind:status naming is unchanged. No standalone Switch build or runtime lane was
run for this slice; no release or device operation was performed.

The initial separate generated-tree execution lacked a memory cgroup and is
retained as noncompliant .unscoped evidence. The same binary and fixtures
passed a replacement one-test execution under 1 GiB, zero swap and 60 seconds
(peak 10,158,080 bytes, no OOM). Acceptance uses the 19 original scoped tests
plus this repaired test. integration-repair.json binds both receipts.

Lead review independently rehashed all 1,338 frozen source files, the nine-file
patch and three executed binaries, then reconstructed both actual recipes and
verified every prior pane and non-layout member. Evidence:
artifacts/trainingrooms-lead-astra4/main-r18/lead-ui22-layout-audit.json in
the parent workspace. This integration is a future candidate; frozen r18 does
not contain UI22.
