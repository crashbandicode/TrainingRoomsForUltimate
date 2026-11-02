# Implementation progress (package 5: menu rendering and assets)

Stream `pkg5`, branch `stream/pkg5`, worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-pkg5`, target
`/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-pkg5`. Implementer: Claude Sonnet 5.5 (high) under lead `0caa72b8`
(model/effort from the stream transcript `agent-a3d90394d2381c41f.jsonl`: `claude-sonnet-5-5` / `high`; hidden watcher in the temp folder
`pkg5-watch`). Playbook followed: `/home/intpa/ssbu-mod-playbook` `009c9ac`. `work/implementation` `fa8b77d` merged (`af0cd7f`).

## Done (2026-11-02)
- **S5-1** (accepted by the lead): the menu on the plugin's own layout, open with B + D-pad Up, navigation, close. Eden `s5-01..s5-09` on the Yoga.
  Layout archive: `tools/layout-builder` -> `assets/info_training.tra1` recipe over the player's own stock archive (docs/BUILD.md); archive hook at
  `main+0x3774704`; UI anchors `main+0x375efa8` / `0x3784ad4`; input from the controller holder (x3) at the input-mapping callers; menu worker thread;
  `Owners::MENU` produced by the input hook and cleared by the tick after the 15-frame close window.
- **S5-2**: page, popups (list, checklist, delay with range, slider, swap groups, overrides, chords, chord editor), two-line help, hints, Developer page
  (page in the ring when `developer.ini` says so; host-tested), pilot detection (`ssbu_cpu_pilot_state_v1`, Q11), HUD hidden while open.
- **S5-3 (host part)**: direction chips, 5 x 8 / 4 x 4 tile grids (item and stage, catalog-length stage grid), notice panes `TrModDisp0*`.
- **S5-4 (part)**: `hud` toggle rule (`is_ready_go` Called site); hitbox visualizer: not started.

## Not done / blocked
- Run-time asset reads (stage thumbnails, portraits, DI / SDI chips from `ef_common.eff`): SPEC-core-07 U1-U8, V4-V6 unresolved; needs ARCropolis route probes in Eden.
- B3 (a modded `info_training` archive): recipe over the vanilla file needs the same route.
- Menu outbox INI writes: recorded in the diagnostics only; pkg4's `chords::commit_shortcut` and the replay-capture write land with its merge.
- Settings write on close: temp + rename placeholder; r1fix's storage API replaces it.
- Eden batch 2 (grids, pilot absent, Developer page, HUD Off, each mockup compared) waits for the Yoga (fairness rule: 3 minutes after another stream's RELEASED).

## Merge of work/implementation (2026-11-02, d12a09c)
Conflicts: `src/runtime.rs` (both `mod ui;` and `mod scene;`) and `CARRY-OVER.md` (both sections kept). The stage grid takes its labels from package 6's `engine::stages::catalog` (117 stages + All = 118 tiles) through a `StageLabel` function the worker passes in; `menu::grid::STAGE_TILES` stays a model constant (the by-path test crates cannot see `engine`) and a lib test in `src/lib.rs` ties it to `TILE_COUNT`. Host tests all pass after the merge; the Switch build after the merge is not done (the Surface and the Yoga are Eden hosts; build and full clippy wait for a free host).

## Gates at the last commit
`cargo test` all pass, `cargo clippy --all-targets -- -D warnings` clean, `cargo build --no-default-features` ok, Switch release build ok (sha256 `3423fb04...`, resident 450,560 B), diag build ok.