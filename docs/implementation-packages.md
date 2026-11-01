# Implementation packages (phase 3 plan, draft 1)

2026-11-01, project lead. Implementers work from `specs/` only and never open
the upstream trees; the spec writers who read upstream do not implement. Every
package ships with host tests, Eden rows, a resident-size measurement
(`tools/build/nro_size.py` from the playbook) before and after, and its row in
the provenance matrix (phase 4). No package starts before its specs are
reviewed; the menu packages also wait for the operator's approval of
`docs/menu-redesign.md`.

## Package 1 — skeleton and three small features

Goal: a plugin that installs on 13.0.5 under the playbook's rules, loads the
fork's settings file, and ships three upstream-origin features whose hooks are
small enough to measure cleanly. Everything later plugs into this.

| Item | Spec | Carry-over input | Gate |
| --- | --- | --- | --- |
| Crate layout: `cdylib` only, `model` (host) / `runtime` (`target_os = "switch"`), `diag` feature, release profile per playbook | SPEC-core-01 | the fork's `src/model.rs` version guard (display version + GNU build-id, `model::verdict`) is ours and is reused as-is | `cargo test`, clippy `-D warnings`, `--no-default-features`, `cargo skyline build --release`, diag build |
| Boot: version guard, site table with 64-byte guards, one install line to `sd:/ultimate/TrainingRooms/events.log`, size-capped diagnostics ring (the fork's `diag_ring`/`logging` ring is ours) | SPEC-core-01 | `src/diag_ring.rs`, the ring part of `src/logging.rs`, `training_mod_consts/src/native_seams/` (seam guards) | install line shows `result=installed heap_allocations=0`; card scan of every site |
| Settings persistence: load/save the fork's `training_modpack.json` and `save_states.json` shapes, defaults, unknown-field tolerance | SPEC-core-04 | `training_mod_consts/src/menu_json.rs` and its fixtures (ours) | host tests on the ported-JSON fixtures |
| Menu **model** (pure, host-tested): pages, sections, dropdown, toggle, checklist, slider, editor rows, sub-screen stack, reset/save-defaults semantics | SPEC-core-02 + `docs/menu-redesign.md` | the fork's grids/lists/panels are carried over later as editor rows | host tests only; no rendering yet |
| Input Delay | SPEC-input-03 | – | Eden: frame-counted delay check |
| Stage Hazards | SPEC-state-07 | `src/hazard_manager/policy.rs` (ours) | Eden: hazard on/off on a hazard stage |
| L+R+A Reset | SPEC-state-06 | – | Eden: reset observed; `lra_reset=Off` ignored |

Measured before/after: resident bytes of the package build versus the same
features isolated in the fork (C116.1 build with every other feature flag off is
not available; the package reports its own absolute size and the fork's
`docs/resource-costs.md` row for each feature).

## Package 2 — CPU behaviour

SPEC-cpu-01…07. The largest player-facing block and the one with the most
per-frame work. Ships with the per-frame hook counter in the diag build so the
budget is measured, not estimated. Depends on package 1 (settings, menu model,
install line, the `common` NRO hooks for shield, DI and status change).

## Package 3 — save states

SPEC-state-01…05. Depends on package 1; the fork's setup library and JSON saves
(ours) are integrated here, which is the first carry-over of a fork feature
that is interleaved with upstream code (`save_states.rs` had 533 fork-only
lines): the spec writer's triage of those ranges is the integration list.

## Package 4 — recording, input display, chords, notifications

SPEC-input-01, 02, 04, 05. Carries over the fork's named-macro, replay and
record-slot layer (ours) on the clean-room recording core; the input display is
built to the fork's optimised behaviour from the start.

## Package 5 — menu rendering and assets

SPEC-core-03, 05, 07 after the operator approves `docs/menu-redesign.md`.
Our own layout archive and icons; the dropdown and section panes; the HUD
toggle; hitbox visualizer (SPEC-core-06) rides along because it shares the
effects/draw hooks.

## Package 6 — carry-over of the fork's engines

Hot swap, Mii, cycle preload, CPU-as-P1, practice stages, lobby practice,
Kirby hats, character items, seam guards (all ours). Mostly file moves plus
re-pointing at the clean-room interfaces named in the specs' "interface"
notes; the Eden matrix from the C116.1 campaign (`…/parked-reuse175/refactor/`)
is re-run on the new build.

## Package 7 — parity and release (phase 5)

Online and desync rows, size and heap statement, compliance report, then the
operator's go for the Forgejo push and the Switch install.

## What runs in parallel

Packages 2, 3 and 4 are independent once package 1 lands and can run as three
implementer streams; package 5 waits for the menu approval; package 6 waits for
2–4. Build hosts: the Surface when no Eden runs here, the Ally WSL runner
otherwise; one `CARGO_TARGET_DIR` per source copy; `df` at least 40 GB.
