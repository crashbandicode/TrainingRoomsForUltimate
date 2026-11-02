# Test provenance, package 5 (menu rendering and assets)

| Requirement (spec) | Host test | Eden row | Result |
| --- | --- | --- | --- |
| UI site rows match the owned ELF; anchor pins; manager slot decode (SPEC-core-05 §9) | `tests/ui_sites.rs` (4), `model::sites_ui::tests` | install line `ui_*=stock/near`, `[Ui] anchors installed` (s5-02..09) | pass |
| Archive recipe rebuilds the target, refuses a changed stock archive and any partial or corrupted recipe | `model::archive_recipe::tests` | `[LayoutArchive] handed out ... bytes=467860 stock=451604` | pass |
| SARC directory lookup (carried) and archive retirement (carried) | `model::archive_name::tests`, `model::archive_retire::tests` | handed out line | pass |
| Pane table: unique names, indexes, linear colour | `model::panes::tests` | `resolved ... missing=0` | pass |
| Page frame: groups, rows, values, help (SPEC-core-03 §2.2, §2.5) | `model::frame::tests` | s5-07/09 screenshots | pass |
| Sub-screens (list, checklist, delay incl. grey reason, slider, swap, overrides, chords, chord editor), grids by console mode | `model::screens::tests` | s5-09 p02-p05; s5-10 every sub-screen and both grid layouts (docked 5 x 8, handheld 4 x 4 by flipping the Eden operation mode live) | pass; mockup differences listed in `eden/EDEN-PACKAGE-5.md` |
| Input adapter: chord edge rule, stick repeat, controller mapping (SPEC-core-03 §2.1, §3) | `model::adapter::tests` | s5-07 open / navigate / close | pass |
| Close fade curve | `model::frame::tests::the_fade_is_logarithmic...` | close screenshot | pass |
| P1 output empty while open and 15 frames after (owner bit) | `model::input_delay` poll tests (package 1) | s5-07: P1 did not act | partial: no adapter-level lane for the producer yet |
| HUD hidden while the menu is open | - | s5-08 / s5-09 screenshots | pass |
| `hud` Off in a running match | `Snapshot::hud` default | s5-10 `hud-on-vs-off.png`: percentages, names and cursors hidden, the Training panel stays | pass |
| Pilot absent: the two fallback rows; Developer page in the ring only with `developer.ini` | `model::menu` tests | s5-11 (`20-developer.png`, `cpu-p1-fallback-rows.png`) | pass |
| Run-time file route (SPEC-core-07 U1-U3) | - | s5-12 `[Probe]` lines: `arcrop_load_file` from a plugin thread returns the mod's file; `mods:/` serves only mod files; `arc:/` the vanilla view | answered |