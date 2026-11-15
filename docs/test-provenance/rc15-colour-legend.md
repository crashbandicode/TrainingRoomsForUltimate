# Hitbox colour legend evidence — 2026-11-15

The Hitbox Visualization checklist shows a colour key beside its ticks. The key changes with the current selections.

- Tested merged source: `3526828736909e797fafe5b7e192edac5616e4bb` (rc.15).
- Diagnostic NRO SHA-256: `1660a9e420016760535cd1641d33b6905c7db627bca351a43745f479e1b02d5c`.
- Eden run: `rc15-integration-35268287-remaining1`, Yoga; row `colour-legend-both-schemes`.
- Row: PASS, 2026-11-15 20:46:02–20:46:26 UTC; starts and ends at the Training HUD.
- The row's `requested_source=c1d9d2b6` names the donor implementation; it is not the merged binary identity.

The normal capture has Frame data colours unticked and the other six items ticked. Its thirteen keys show all eight
hitbox colours, magenta grab boxes, yellow fighter/article hurtboxes, deep-blue windboxes and wind areas, and white
direction lines. After Down three times and A, Frame data colours is visibly ticked. Nine keys show strong red and
weak pink hitboxes, purple grabs, white normal hurtboxes, blue unhittable hurtboxes, green no-damage hurtboxes, and the
wind and direction keys. Both complete checklists, swatches and labels were visually reviewed: no overlap or clipping.

The existing per-press Ui trace records Checklist cursor 0 on opening and cursor 3 on the A that changes the scheme.
The runner receipt and its start/end HUD gate captures are retained with the run. Automated collection alone is not
visual acceptance; the separate image review establishes the observations above.

| Original PNG, copied without modification | Dimensions | SHA-256 |
| --- | --- | --- |
| [Normal colours](../media/rc15-colour-legend-normal.png) | 1280 × 674 | `9d0b41973ab7d299a0a5bcdf708f4d25244fb80536c1f5b74a488739c5941a7f` |
| [Frame data colours](../media/rc15-colour-legend-frame-data.png) | 1280 × 674 | `42be49c942d872fb9dce16303c23e1764625f97fa2829f80002d560e2960e589` |

Host coverage: `menu_swatches_match_the_renderer_in_both_schemes` checks actual renderer colours in both schemes and
all 128 selection masks; `hitbox_checklist_colour_key_updates_and_clears_with_the_screen` checks menu updates and cleanup.
The merged XPS gate passed tests, Clippy with warnings denied, no-default-features, release, diag and size checks.
Its normalized production total is 148,194, with 56 lines contributed by the legend. Switch builds retain an inherited
unused-function warning; host Clippy has none. No strict aggregate Cargo test count is claimed.

This documentation was added after that build; it changes no product source. The captures establish the legend's menu
layout and scheme change in Eden, not hardware behavior or every gameplay status. The image named for a final-build
footer did not visibly show the version text and is not used as footer proof. Other integration rows and run closure
have their own receipts; this record makes no whole-run or release-readiness claim.
