# Implementation progress, package 6 worker B (stream `pkg6b`)

Branch `stream/pkg6b` (worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-pkg6b`, from `23cdc6c`), target `/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-pkg6b`,
Eden labels `s6b-NN` (none run). Claude Sonnet 5.5 high (transcript `agent-adabf798deb6144b5.jsonl`, hidden watcher in the temp folder `pkg6b-watch`). Playbook followed: `ae87443`.
Scratch worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-pkg6b-scratch` (branch `tmp/pkg6b-scratch`: pkg6b + `stream/wire` merged locally, target `cargo-targets/trainingrooms-pkg6b-scratch`);
never merged anywhere; delete it with `git worktree remove` and `git branch -D tmp/pkg6b-scratch` when its commits are cherry-picked.

## Host-rule slip (2026-11-02, recorded at the lead's request)
I ran Switch release builds (`cargo skyline build --release`, about six, the first from 06:3x Z, all on the Surface, target `trainingrooms-pkg6b` and later `-scratch`), a full `cargo clippy --all-targets -- -D warnings`
and full `cargo test` on the Surface while the fork's M5q online claim was open or about to open (I checked for a local Eden process and for the ledger file, found neither, and did not read the queue
file's "claims at the next RELEASED line" as an open claim). The 06:48Z build the primary saw was mine. No Eden of mine ever ran. From now: no NRO/Switch builds and no full clippy on any host with an open claim (Surface and Yoga both
have the M5q claim), short host tests only, until the claim has a later RELEASED line.

## Committed on `stream/pkg6b`
- `0d11a89` tranche 1: carry tool (`#[path]` rewrite, `training_mod_sync` / `training_mod_macros` heads), 204 carried files, `runtime::compat::{common,menu,sync}`, typed menu +3 fields, engine areas on the Switch, leaves
  `game_mode`, `training_stage_rotation`, `native_keyboard`, `mii_identity_binding` Switch-compiled. Host `cargo test` 5,632 pass (the one failure is the wire stream's `input_sites::every_row_matches_the_owned_elf`), clippy 0, `--no-default-features` builds, release build `Finished`.
- `4f21d21` carry tool: per-entry `widen` flag (`pub(super)` to `pub(crate)`).

## In the scratch tree, not yet on `stream/pkg6b` (needs `stream/wire` for its natives and notices)
Edits on `tmp/pkg6b-scratch` (uncommitted when this was written; the Switch build of that tree was `Finished` with a 4-item `character_roster` stub and a `pending.rs` stub of packages 2/4 providers, both scratch-only):
manifest: `widen` on `lobby_stage_cycle` / `practice_stage_request_policy`; `FighterId`; recorded edits of `training/lobby.rs` (drop `available_actions` / `available_ledge_options`, `set_native_ui_busy` is the single producer of `Owners::NATIVE_UI_BUSY`,
`cpu::set_lobby(now)`) and `training/practice.rs` (`try_get_object_accessor`); carried pure `mii_menu.rs` (+ `compat::panel_view`) and `resume.rs`; the pure automation helper (`adapter, control, flow, popup, pulse, recording`: 41 host tests pass);
`runtime::compat::{common (typed), consts, hazard_manager, replay_preview, filesystem}`, `lobby/training/mod.rs` facade (notifications over wire's `notices::upsert`, `save_states::stage_id`, `native_menu::reset` via a new `notices::reset_combo_display`),
`engine::lobby::notice_ids` (host test scans the carried files for headers), `runtime/lobby/automation_helper.rs`. Last step before the stop: `compat/filesystem.rs` and `automation_helper.rs` written, not yet compiled.

## Findings
- The Mii game files are `#[path]` children of the roster cycle's `character_roster.rs` and use `use super::*` / `super::{...}`: they compile only with worker A's cycle. Only leaves compile alone.
- Interface needs, by owner: **A / cycle**: `character_roster::{busy, scene_changed(Mode), mii_catalog_request::request, mii_saved_alpha::menu_epoch}` (called by the lobby half and the editor bridge); the cycle directory name `crate::runtime::roster` is assumed by the manifest map (`modules.root.character_roster`, `character_lifecycle`, `cycle_preload*`, `swap_native::scope_gate`).
  **pkg4**: `named_macros::{suspend_waiting, resume_waiting, suspended, forget_stopped_entry, forget_stopped_scene, waiting_scene_changed, selection_revision}`, `replay_store` (catalog queue, `request_mii_catalog`, `import_mii`), `replay_capture::loading`; `resume.rs` (carried here, pkg4 should not carry it again).
  **pkg2**: `tech::forget_hidden_{entry,scene}`, `cpu_rolls::request_all` (a `features::cpu::request_rerolls(bits)`), a checked `is_operation_cpu` (wire's answers false when the natives are not usable).
  **pkg5**: the lobby host tick for `info_waiting` (`TrModPort`) calling `lobby::set_native_ui_busy`; roster grid panes; the Mii list screen fed by `mii_menu::CatalogMenu::view`; a menu call to reopen at an editor (`QUICK_MENU_APP` in `mii_editor_bridge/mod.rs` and `mii_saved_alpha.rs`); one call `runtime::compat::menu::apply(&Settings)` after settings are published (boot and commit).
  **call sites** (the fork's mixed code called them): `lobby::update` per frame, `observe_fighter` from the fighter init hook, `practice::{refresh, entry_departed, scene_departed, retire}`, `runtime::lobby::init` (the `init()` functions of the carried files, order: `training::lobby::init`, `lobby_stage_probe::init` (it starts the rotation), `practice_stage_request::init(base)`, `practice_stage_observer::init`).
- Decisions for the lead: (1) `mii_editor_bridge::event` writes `mii-editor-last.json` through a worker on every editor event in the shipped build (playbook: events belong in diag; gating it is a feature gate, so it is yours to approve), and the carried `write_trace` uses `std::fs::write` (not the R1 transaction); the bridge and `mii_hud_portrait` allocate in callbacks (R5). (2) `automation_helper::plugin` needs the Deluxe SDK (L1): not declared. (3) `compat::common` exists twice (A's `runtime::p1::compat::common`): unify at the merge.
- Superseded, not carried: `layout_registry`, `anchors`, `native_menu` (pkg5 / pkg4 own them), `training/ui/{roster,stage}_grid.rs`.

## Next (restart-safe)
1. When a host has no open claim: build the scratch tree (`B_TREE=TrainingRoomsForUltimate-pkg6b-scratch`, scripts in `/tmp/pkg6b-scratch/{sw,cg,errs}.ps1`, recreate from docs/BUILD.md if /tmp is cleared), fix `filesystem.rs`/`automation_helper.rs`, run `--features diag,automation-results-observer`.
2. Commit the scratch changes in two groups: independent of wire (manifest, mii_menu, resume, automation pure, notice_ids needs only `engine`) cherry-picked onto `stream/pkg6b`; the wire-dependent compat on `tmp/pkg6b-scratch`, offered after the lead merges wire.
3. The declaration commit (`mod` lines of the cycle-dependent files) after A's cycle lands; Eden rows `s6b-NN` only then.
