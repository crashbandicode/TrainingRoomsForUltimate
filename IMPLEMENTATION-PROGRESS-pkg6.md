# Implementation progress, package 6 (stream `pkg6`)

Branch `stream/pkg6`, worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-pkg6`, base `473da26`, merged
`work/implementation` `fa8b77d` (`5f98056`). Target `/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-pkg6`,
Yoga folder `/home/intpa/trainingrooms-build-pkg6`, Eden labels `s6-NN`. Claude Sonnet 5.5 high (hidden watcher in the
native temp dir, bound to the harness PID; model gate from the session transcript). Playbook followed: `009c9ac`.
Stream rules: `/home/intpa/smash-training-lobby/TRAININGROOMS-STREAM-RULES.md`.

## Done
- S6-0: `docs/PACKAGE-6-PLAN.md`; generated inventory (`docs/provenance/pkg6-inventory.{json,md}`), dependency census
  (`pkg6-deps*.json`, `pkg6-edges.json`), carry layers (`pkg6-layers.md`) by `tools/pkg6_inventory.py` and
  `tools/pkg6_deps.py` (`python3 -I tools/pkg6_inventory.py . m5o`, then `pkg6_deps.py . m5o`).

## Running / blocked
- Nothing running. Decisions D1-D10 of the plan wait for the lead; C0/C1 can start on D2/D3 (module layout and
  dependencies).

## Next
- S6-1: `tools/carry_pkg6.py` + manifest, `model::compat`, seam guards (C1), pure halves of cycle preload / Kirby hats /
  character items (C2); then C3-C5.
## S6-1 tranche 1 (2026-11-02)
- Merged `work/implementation` `fa8b77d` (`5f98056`). Lead rulings D1-D11 recorded in the plan section 12.
- `tools/carry_pkg6.py` (carry / verify / ledger) and manifest; 22 files carried (seam table + unlink guards, source lifetime and load,
  roster grid, preload grace and notice queue) under `src/model/{seams,swap,preload}`, `cfg(test)`-only until their consumers land.
- Cargo features: the fork's `[features]` table (lead-authorised read of lines 82-122 and 124-240 (line 123 not printed), disclosed in CARRY-OVER.md), `default` = the M5o release list.
- Gates (host, Surface, target `cargo-targets/trainingrooms-pkg6`): `cargo test` all binaries pass (the carried unit tests run in every by-path test binary),
  `cargo clippy --all-targets -- -D warnings` clean, `cargo build --no-default-features` builds. Switch builds: not run for this tranche (no runtime change; resident
  size unchanged at the package-2 value because the carried areas are test-only).
- Not blocked. Next: C2 remainder, C3 (D3 dependency checks first: serde derive, byteflags), then C4/C5; `src/runtime/scene.rs` (D4) before C7.

## Layer gates after tranche 1 (2026-11-02)
- **C3a** `943e2a5`: Mii crate (23 files, 7 integration tests), body_bank, swap_model + fixtures, trio, ordered_roster, character_forms, identity_view, capacity_creation, mii_cycle, preload/kirby/lobby policies. D3: serde derive added (release NRO byte-identical, 233,472 B resident), serde_json pinned 1.0.151.
- **C4** `b48ac10`: stage models (lobby_stages, stage_forms, stage_menu, lobby_stage_cycle, practice_stage_request_policy, training_stage_rotation_policy, waiting-hazards policy).
- **D4** `cdd9438` (cherry-picked by the lead as `5fe6bbf`): `model::scene` + `runtime::scene`.
- **C2b** `6e3f0d0`: character items model.
- Gates at `6e3f0d0` (+ tool change): host `cargo test` 2,434 pass, clippy 0, `--no-default-features` builds, Switch release builds (NRO unchanged, 233,472 B resident, sha `d2722f4d`; carried areas are `cfg(test)`-only so there is no resident change yet).
- Disclosures recorded in CARRY-OVER.md: fork `Cargo.toml` ranges 82-122 and 124-240 (123 not printed); an `rg` that named two mixed tui files (3 lib.rs lines printed).

## In progress / next (restart-safe)
- **cycle_preload_config.rs** (consts, 2.7k lines): not carried yet; the attempt was backed out to keep the tree green. Needs, in order: (1) `byteflags` dep
  (`byteflags = { git = "https://github.com/asimon-1/byteflags.git", rev = "e6ac815a9d0dc4665b39fe7849e2baec8bf443cb" }`; resolves `rand` git rev `8e87ec2a` and
  `getrandom` Raytwo `23a14191` through the fork's own manifests, no `[patch]` needed; pin `zerocopy`/`zerocopy-derive` to 0.8.62 as in the m5o lock with
  `cargo update -p zerocopy@0.8.63 --precise 0.8.62` after a networked `cargo fetch`); (2) `engine::compat::menu` grows the typed fields `cycle_preload_{slots,sources,
  bank_members,ready_fighters,group_swap,ic_pair_bank,original_bank,solo_popo_groups,ic_pair_groups,native_groups,pair_groups,mii_groups,measurements}` (types are the file's own
  byteflags groups and `OnOff`), `OnOff::ALL_NAMES`, serde for `OnOff`; (3) manifest edits removing the fork's menu macros (`mod bank_submenu` / `mod ready_submenu`
  with `impl_submenutrait!`) and the UI tests that call `create_app`; (4) `TRAINING_MODPACK_ROOT` -> `model::storage::ROOT` (the tool already rewrites the
  `sd:/ultimate/TrainingModpack` string); (5) later, the R1/R4 adaptation of its INI writer/parser as recorded manifest edits.
- **C7 cluster** (character_roster, cycle_preload, kirby_hats, character_items, practice_stage_{request,observer}, training/{lobby,practice}, character_lifecycle,
  swap_native/presentation, native_unlink_install, native_service, training/ui/anchors): needs `compat::menu` for `lobby_practice`, `trio_practice`, `player_roster`...,
  package 2's fighter/accessor natives, package 3/4 stubs (`replay_preview::occupied`, `named_macros::*`, `save_states::is_loading`) as interface modules, `skyline_smash`
  (D3: MIT, rev 01bf583), and the `Seam` install API (`swap_native/seam_install.rs`) carried first.
- Remaining by-path Mii tests (`quickplay_backend`, `ldn_backend`, `source_provenance`, `heap_metrics`, `ui_archive_name`, `replay185_headgear`) wait for the root files they include.

## Successor worker, step 1 (2026-11-02): cycle_preload_config carried (C2a)
- `cycle_preload_config.rs` carried (`engine::preload`, 2,719 lines, 37 carried tests; 4 manifest edits, see CARRY-OVER.md C2a). **No `byteflags` dependency**: `model::compat::choice` is a clean-room `byteflags!`/`count!` (the crate's macro forces `non_snake_case` allows and three git deps); the tool rewrites the carried `use byteflags::...`. Lead to be told (deviation from the accepted dependency).
- `compat::menu`: 13 typed `cycle_preload_*` fields + Solo Popo, serde attributes as the fork, `from_snapshot` / `rows` (write-back), known default divergences pinned (`solo_popo_original`, `cycle_preload_measurements`: fork On, factory Off; lead decision wanted for the cycle_preload INI first-boot value). `compat::menu_json::parse_menu_file` stand-in.
- Clean-room INI transaction tests (9) pass against the unchanged carried writer/reader. Gates at this commit: `cargo test` 2,480 pass, clippy `-D warnings` 0, `--no-default-features` builds. Switch builds not run (no runtime change; areas still `cfg(test)`).
- Next: C6 (CPU-as-P1: needs `ssbu-cpu-pilot-core` dep check + Q11 fallback rows), then the C7 cluster (needs `skyline_smash`, `seam_install`, package 2/3/4 interfaces; list of exact needs goes to the lead).
- Lead question answered in the report: lobby/rotation catalog = `training_mod_consts/src/lobby_stages.rs` `STAGES` (117 entries, id + label), carried as `engine::stages::lobby_stages::STAGES`.