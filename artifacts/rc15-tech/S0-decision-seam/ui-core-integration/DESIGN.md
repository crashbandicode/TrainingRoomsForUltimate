# UI/core integration design — 2026-11-16

Scope: isolated `rc15/tech-seam`, base `8b43bdf2`. Register the reviewed surface rows in the existing boot capability pipeline and add the P1 UI frame boundary. No UI implementation, observer grading, input calibration, Eden, build, release or card work.

| Operator clause | Design | Test |
| --- | --- | --- |
| "one crate::runtime::features::tech::frame(a0), independent Hitbox" | CPU post-native dispatcher invokes the UI after the adapter; each pass owns its readiness boundary | `ui_tech_frame_is_once_after_adapter_and_independent_of_hitbox` |
| "UI module first gate tech_ready" | Declare the actual first-condition boundary in the unchanged graph audit | `every_boundary_checks_its_gate_first` |
| "boot outcomes/poison/usable/reports" | Feed all 17 surface rows through comparison, diagnostic poison, grant construction and install reports | `surface_rows_are_registered_through_boot` |
| "poisoning tech_cos or state setter leaves CpuCore/CpuCommon/Hitbox installed" | Keep the optional TechPractice closure separate and assert rejection of every surface row | `optional_tech_poison_preserves_existing_features` |
| "boundary and cross-needs exact actual callgraph no test relaxation" | Keep graph-derived exact closure and poisoned-entry tests; add only the real UI readiness boundary | `cross_needs_match_the_call_graph` |

Grant storage is a Vec of deduplicated offsets, so no fixed capacity needs increasing. The 17 rows are called/relied-only and add no code-writing records.

Known blockers: UI `VIEW.lock`, `IDS.set`, and surface `VERIFIED.get_or_init` are reachable from game hooks; these are reported to the primary/UI owner without audit exceptions. The stable launch/adapter/contact API remains unchanged. Native outcome mapping and guard-press calibration remain unbound.
