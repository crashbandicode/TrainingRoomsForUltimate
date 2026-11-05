# Restrictions

Every user-facing refusal a branch adds cites one approved row here with `// restriction: <ID>` on its line or one of
the 3 above it (playbook `docs/CHECKLIST.md`, the "cites an approved restriction" row; `compliance.py --base`).

| ID | Message | Why | Approved |
| --- | --- | --- | --- |
| preload-wait-memory | `" ({} waiting for memory)"` (Cycle Preload notice, `cycle_preload_policy.rs` `CacheText`) | An extra group whose prefetch start found too little process headroom is held and retried, not failed; the notice names what it waits for (P-HOT, `LANE-HOT-SLOTS-REPORT.md` §8) | operator 2026-11-05 ("yes to both") |
| preload-wait-pool | `" ({} waiting for pool space)"` (Cycle Preload notice, `cycle_preload_policy.rs` `CacheText`) | The same hold when the fighter pool has too few free bodies after its reserve (P-HOT §8) | operator 2026-11-05 ("yes to both") |
| preload-failed-and-waiting | `" ({} failed, {} waiting for {})"` (Cycle Preload notice, `cycle_preload_policy.rs` `CacheText`) | Failed and held groups at once, in one bracket (P-HOT §8) | operator 2026-11-05 ("yes to both") |
