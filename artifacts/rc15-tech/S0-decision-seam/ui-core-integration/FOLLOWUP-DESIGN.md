# Follow-up design — 2026-11-16

Base `f84e0de4`; only this isolated seam is writable. The canonical closure receipt reports precisely three missing TechPractice dependencies. Add those declarations, retaining exact callgraph comparison and poison admission. No UI-owned source edits or speculative receipt hardening.

| Operator clause | Design | Test |
| --- | --- | --- |
| "fix exact 3 TechPractice CROSS_NEEDS missing" | Declare camera_limit_range, stage_id_call and utility_get_kind from CLOSURE-f84e0de4.json | `tech_stage_and_kind_dependencies_are_admission_requirements` |
| "No guard/test relaxation" | Existing exact graph closure and poison checks retained | `cross_needs_match_the_call_graph` |
| "after constants ready before P1 hooks, not on_frame" | Deferred pending a proven pre-hook readiness event; current only readiness retry is inside already-installed CPU on_frame | `ui_tech_frame_is_once_after_adapter_and_independent_of_hitbox` |

The last row's test pins the existing dispatch only; it does not prove the unimplemented initializer ordering. This work must not be described as completing initialization.
