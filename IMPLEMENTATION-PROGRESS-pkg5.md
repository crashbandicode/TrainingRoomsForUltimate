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
## Successor session (2026-11-02, Sonnet 5.5 high; watcher `/tmp/pkg5-watch2`)
- Builds (Ally WSL, `eden/ally/ally-build.sh`): `f05365e` release `162d04ef` (495,616 B resident), diag `824fe986` (602,112 B); `00cfb44` release `31b25337` (495,616 B), diag `adf4e987` (606,208 B); host tests 3,305 pass, clippy 0, `--no-default-features` ok, 0 Switch warnings. Clippy fix for Rust 1.98.1: `Text::string_lossy` (Copy type, by-value rule), `is_multiple_of`. `work/implementation` has its own lint fix of the same two sites (`6c9e530`, `Label::to_string_lossy`): take one of the two at the merge.
- Eden batch 2 done on the Ally (s5-10, s5-11, s5-12): `eden/EDEN-PACKAGE-5.md`.
- Fixed from the mockup comparison: direction chips overflowed the help panel (size 184 / pitch 206, recipe rebuilt `225fdfd0`); docked tile labels cut mid-word and ambiguous (now 12 characters plus two dots).
- Open from the comparison (not fixed: design work, each listed in `eden/EDEN-PACKAGE-5.md`): DI / SDI chips are the clean room's arrows (run-time chips wait for U4), no "Selected (weight)" side panel on the direction grids, the delay popup is the checklist only (no range control yet), overrides screen is one column without "Use Mash set", item names in the docked item grid keep the old order of the list, Button chords row shows no "6 chords" value.
- Host-aware ledger helper `eden/ally/ledger.py` and `s5_eden.py` (pkg6 `9360dfa` rule): a claim line names only the host it claims.
## Gate 2026-11-02 (second): R2-2, R2-7, range control, overrides, B3, menu_app
- Merges: work/implementation c5863bd, 915fda1 and a44b7ce (`f7a99fa`, `d44b573`, `54f2589`); the one conflict each time is recorded in the merge commit.
- Gates at `8bb6e5a`+`menu_app` fix (Ally WSL): host tests 6,361 pass, clippy 0, `--no-default-features` ok, 0 Switch warnings; release `7c7e5c8d` 798,720 B resident, diag `89b42e30` 954,368 B.
- Done and Eden-proven (Ally `s5-14`..`s5-17`, `eden/EDEN-PACKAGE-5.md`, `eden/runs/s5-1x/`): B3 vanilla-base injection; Selected (weight) panel; delay popup (checklist grid over the min-max range control, mutual exclusion, Y clears, grey reason); overrides in three columns of ten (the model has 30 actions; mockup 06 drew 13 in two); R2-2 (forced archive refusal leaves input free); R2-7 (UI hooks counted, per-load allocations apart, steady delta 0 in menu use).
- Open: SPEC-core-06 hitbox visualizer not started (site rows exist as seams `rows_a3.rs`; offsets resolved in the owned ELF: AttackModule is_attack `0x1fd01c0`, attack_data `0x1fd0520`, EffectModule set_rgb_partial_last `0x2017c80`, set_rate_last `0x2017ce0`, MotionAnimcmdModule set_sleep_effect `0x2069140`; the AttackData field offsets still have to be derived from the ELF, they are not in the spec); U4 texture binding (stream/u4); package 6 pieces: `menu_app` holder done (`src/runtime/ui/menu_app.rs`), the `info_waiting` / `TrModPort` host tick, the roster-grid panes and the Mii list screen are not built.