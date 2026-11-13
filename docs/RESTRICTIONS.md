# Restrictions

Every user-facing refusal a branch adds cites one approved row here with `// restriction: <ID>` on its line or one of
the 3 above it (playbook `docs/CHECKLIST.md`, the "cites an approved restriction" row; `compliance.py --base`).

| ID | Message | Why | Approved |
| --- | --- | --- | --- |
| `mii-class-unavailable` | `"{label} - unavailable"` (`src/editors/roster.rs`) | A greyed Mii tile: the role has no Mii of that class it can load (`docs/MII.md` row 17; the fork's behaviour). | operator 2026-11-05 ("approve all four") |
| `replay-preview-fighter` | "Preview fighter is not available" (`src/model/macro_ui/replay_target.rs`) | The capture's fighter kind is not one the roster can load, so there is nothing to preview or switch to. | operator 2026-11-05 ("approve all four") |
| `replay-mii-metadata` | "Recorded Mii metadata is unavailable" (`src/model/macro_ui/replay_target.rs`) | The capture's Mii could not be resolved, so Switch cannot load that exact Mii; Keep still plays the inputs. | operator 2026-11-05 ("approve all four") |
| `replay-mii-loader-build` | "Replay Mii loading is unavailable in this build" (`src/model/macro_ui/replay_target.rs`) | No current build shows it: the `mii-ordinary-cpu-alpha` switch was removed in rc.10 and every build has the Mii loader. The string stays for a build without it. | operator 2026-11-05 ("approve all four") |
| `combos-training-only` | "Combos work in Training" (retired; the string is gone from `src/`) | superseded by combos-lobby in rc.5. The waiting-scene return is gone. | retired 2026-11-06; was operator 2026-11-05, for rc.5 only |
| `combo-save-wait-load` | "Wait for the practice fighters to finish loading" (`src/runtime/ui/combos.rs`) | The combo list opens only once the practice fighters have loaded (the panel is not up yet). | operator 2026-11-05 ("yes to both") |
| `combo-name-online-default` | "The next combo gets a Combo N name" (`src/runtime/ui/combos.rs`) | An online waiting room does not open the name keyboard; the next save uses Combo N. | operator 2026-11-05 ("A") |
| `preload-wait-memory` | `" ({} waiting for memory)"` (Cycle Preload notice, `cycle_preload_policy.rs` `CacheText`) | An extra group whose prefetch start found too little process headroom is held and retried, not failed; the notice names what it waits for (P-HOT, `LANE-HOT-SLOTS-REPORT.md` §8). A No. of CPUs seat (entry 2 or 3 at one CPU) whose load memory refused counts the same way (primary reading F9-SEAT-NOTICE, 2026-11-08; `hat_slots::with_seats` on `p23/rc6-open-fixes`) | operator 2026-11-05 ("yes to both") |
| `preload-wait-pool` | `" ({} waiting for pool space)"` (Cycle Preload notice, `cycle_preload_policy.rs` `CacheText`) | The same hold when the fighter pool has too few free bodies after its reserve (P-HOT §8) | operator 2026-11-05 ("yes to both") |
| `preload-failed-and-waiting` | `" ({} failed, {} waiting for {})"` (Cycle Preload notice, `cycle_preload_policy.rs` `CacheText`) | Failed and held groups at once, in one bracket (P-HOT §8) | operator 2026-11-05 ("yes to both") |
| `chain-recordings-only` | (none) | a chain inside a chain would need a loop check | operator 2026-11-06, revision 8 Q13 |
| `macro-start-save-missing` | No start save. This macro plays from where the fighters are. | the connected start file was deleted | operator 2026-11-06, revision 3 answer 3 |
| `save-load-off` | "Save States is Off (Saves page)" (`src/model/state/refused.rs`) | A Save or Load chord with Saves › Save States Off; before rc.12 it did nothing, silently, and fired later when the row went On | the primary 2026-11-13 13:29Z ("every refused or ignored Save/Load … shows a readable notice"); for the operator's review |
| `save-load-empty` | "Nothing is saved here yet: Save first, or bind a save" (`refused.rs`) | Load with nothing bound (RECORDING.md: "If none is bound, it does nothing") | the same |
| `save-load-busy` | "A load is already being placed" / "A setup is being placed: save (load) again after it" (`refused.rs`) | A load, setup, preview or Play warp is being placed; the press would cut it | the same |
| `save-load-player-cpu` | "A CPU drives Player 1: take Player 1 back to load" (`refused.rs`) | Load takes Player 1's own body; under the CPU pilot or CPU control no body takes it | the same |
| `save-load-deluxe-page` | "Online Deluxe's open page uses this button: close it to save (load)" (`refused.rs`) | A chord moved onto a D-pad button Online Deluxe's open page acts on (C4, narrowed in rc.12) | the same |
| `save-load-gamecube-triggers` | "Both triggers are Online Deluxe's L + R: hold one of them" (`refused.rs`) | C3: both GameCube triggers are Deluxe's L + R | the same |
| `save-load-not-ready` | "Save states have not started in this match yet" (`refused.rs`) | The save-state step's game values are not resolved yet at the match's start | the same |
