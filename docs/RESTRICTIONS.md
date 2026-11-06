# Restrictions

Every user-facing refusal a branch adds cites one approved row here with `// restriction: <ID>` on its line or one of
the 3 above it (playbook `docs/CHECKLIST.md`, the "cites an approved restriction" row; `compliance.py --base`).

| ID | Message | Why | Approved |
| --- | --- | --- | --- |
| `mii-class-unavailable` | `"{label} - unavailable"` (`src/editors/roster.rs`) | A greyed Mii tile: the role has no Mii of that class it can load (`docs/MII.md` row 17; the fork's behaviour). | operator 2026-11-05 ("approve all four") |
| `replay-preview-fighter` | "Preview fighter is not available" (`src/model/macro_ui/replay_target.rs`) | The capture's fighter kind is not one the roster can load, so there is nothing to preview or switch to. | operator 2026-11-05 ("approve all four") |
| `replay-mii-metadata` | "Recorded Mii metadata is unavailable" (`src/model/macro_ui/replay_target.rs`) | The capture's Mii could not be resolved, so Switch cannot load that exact Mii; Keep still plays the inputs. | operator 2026-11-05 ("approve all four") |
| `replay-mii-loader-build` | "Replay Mii loading is unavailable in this build" (`src/model/macro_ui/replay_target.rs`) | Only builds without the Mii loader (`mii-ordinary-cpu-alpha` off); release builds have it. | operator 2026-11-05 ("approve all four") |
| `combos-training-only` | "Combos work in Training" (retired; the string is gone from `src/`) | superseded by combos-lobby in rc.5. The waiting-scene return is gone. | retired 2026-11-06; was operator 2026-11-05, for rc.5 only |
| `combo-save-wait-load` | "Wait for the practice fighters to finish loading" (`src/runtime/ui/combos.rs`) | The combo list opens only once the practice fighters have loaded (the panel is not up yet). | operator 2026-11-05 ("yes to both") |
| `combo-name-online-default` | "The next combo gets a Combo N name" (`src/runtime/ui/combos.rs`) | An online waiting room does not open the name keyboard; the next save uses Combo N. | operator 2026-11-05 ("A") |
| `preload-wait-memory` | `" ({} waiting for memory)"` (Cycle Preload notice, `cycle_preload_policy.rs` `CacheText`) | An extra group whose prefetch start found too little process headroom is held and retried, not failed; the notice names what it waits for (P-HOT, `LANE-HOT-SLOTS-REPORT.md` §8) | operator 2026-11-05 ("yes to both") |
| `preload-wait-pool` | `" ({} waiting for pool space)"` (Cycle Preload notice, `cycle_preload_policy.rs` `CacheText`) | The same hold when the fighter pool has too few free bodies after its reserve (P-HOT §8) | operator 2026-11-05 ("yes to both") |
| `preload-failed-and-waiting` | `" ({} failed, {} waiting for {})"` (Cycle Preload notice, `cycle_preload_policy.rs` `CacheText`) | Failed and held groups at once, in one bracket (P-HOT §8) | operator 2026-11-05 ("yes to both") |
