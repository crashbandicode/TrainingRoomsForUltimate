# PROGRESS — core area specs (spec writer)

Started 2026-11-01. Writer: Claude agent (Fable 5.1), spec-writer role for area `core`.
Sources read: upstream `97cbe9b9` (all files listed in the brief), upstream `077a8e43`
diff (`src/hitbox_visualizer/mod.rs` +4, `src/training/mod.rs` +8/−1), fork stage M5i
mixed ranges per `docs/provenance/provenance-files.md`, fork docs REVISION29–33,
menu-shortcuts.md, native-operations.md, cycle-preload.md, README.md (fork head),
playbook WORKFLOW.md and HOOK-SHARING.md, `specs/TEMPLATE.md`.

| Spec | Status |
| --- | --- |
| SPEC-core-01-boot-and-install.md | written (draft) |
| SPEC-core-02-menu-model.md | written (draft) |
| SPEC-core-03-menu-rendering-and-input.md | written (draft) |
| SPEC-core-04-menu-persistence.md | written (draft) |
| SPEC-core-05-layout-archive-and-hud.md | written (draft) |
| SPEC-core-06-hitbox-visualizer.md | written (draft) |
| SPEC-core-07-assets-inventory.md | written (draft) |
| INDEX-core.md | written |

Completed 2026-11-01: seven specs, `INDEX-core.md` and this file. Every spec follows
`TEMPLATE.md` section by section (purpose, user-visible behaviour, inputs, native
state table with 13.0.5 `main+0x…` offsets, timing, edge cases, persistence, resource
budget, test provenance, open questions). Facts that could not be determined from the
sources are listed at the end of `INDEX-core.md`, not guessed.

No file outside `specs/` was touched. Nothing was built, run or committed.
